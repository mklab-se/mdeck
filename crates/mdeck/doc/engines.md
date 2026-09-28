# Writing an engine

An **engine** is what a theme does beyond colours and type: the layer it
paints under the slides, and what it plays for the countdown and the end.
MDeck ships `plain`, `particles` (the Ember theme), `led` (the Marquee
theme) and `splitflap` (the Departures theme). This guide is for adding one. Read spec section 9.6 first for what users see.

The rule that makes engines safe to add: **the core decides what a slide
wants to show; an engine decides how it looks.** An engine never parses
markdown, resolves an illustration or reads the deck file. It gets a `Stage`
and paints it.

## The pieces

All of it lives in `crates/mdeck/src/engines/`.

| Piece | File | What it is |
|---|---|---|
| `EngineKind` | `mod.rs` | The value a theme carries (`engine: led`). The registry: name, cargo feature, capabilities, constructor. |
| `Capabilities` | `mod.rs` | What the engine can show. The core uses it for fallbacks and `--check` for warnings. |
| `Engine` | `mod.rs` | The trait your runtime implements: `update`, then `paint`. |
| `Stage`, `Moment`, `Figure`, `Place` | `stage.rs` | What the slide wants to show this frame. |
| `FrameCx` | `stage.rs` | The frame: rect, scale, opacity, `dt`, `still`, theme. |
| `Host` | `host.rs` | The engine-neutral half, owned by the app and the export. You get it for free. |

### What the host gives you

Every frame the host builds a `Stage`:

- `moment`: `Slide`, `Countdown { digit, mask, progress }`, `Burst { progress }`
  (the countdown's exit) or `End { elapsed, words }`. `mask` and `words` are
  point masks of the glyphs in the theme's display face (points in the unit
  square, plus the shape's width over height).
- `figure`: the slide's `@illustration`, resolved through the deck, user and
  built-in libraries, with `place` (a box in slide fractions) already chosen:
  on the right beside the copy, or large and centred behind a title
  (`backdrop`). The cloud's points are in importance order: the first sixty
  already sketch the subject.
- `hints` and `hints_key`: the geometry the slide's renderers drew last frame
  (`Bar`, `Path`, `Circle`, `Point`, and `Frame` for boxes to keep out of),
  so an engine can serve a chart instead of decorating around it.
- `index`, `reveal`, `title`, and the `slide` itself for engines that lay out
  text (`story` only when the engine plays stories).

The host also keeps the clock, times the end slide, swaps the runtime when
the theme's engine changes, and decides when the caption appears on the end
slide (`EngineKind::end_caption_delay`).

## The frame

```text
host.frame(shot) ─┬─ build Stage (moment, figure + place, hints)
                  ├─ engine.update(&FrameCx, &Stage, &mut Library)   advance by cx.dt
                  └─ engine.paint(&Ui, &FrameCx, &Stage)             under the slide
```

`update` moves your state toward what the stage asks for; `paint` draws it.
Keep them separate: a rehearsal (below) runs `update` many times and `paint`
once.

## The rules

1. **Stills are finished.** When `cx.still` is set (PNG and PDF export),
   settle at once and paint the final look: no half-lit LEDs, no beam, no
   smoke that depends on when the capture happened. Two exports of the same
   deck must be byte-identical.
2. **No wall-clock randomness.** Seed everything from the slide index and
   per-element hashes (see `hash01` in `led.rs`), never from time.
3. **Scale everything.** Multiply every size in pixels by `cx.scale`
   (`min(w/1920, h/1080)`), and paint only inside `cx.rect`: the rect moves
   during transitions and is a tile of a bigger canvas in export.
4. **Colours come from the theme.** Use `accent`, `accent-soft`, `secondary`,
   `particles.light` and `particles.cool`, and check `theme.is_light()`: light
   that adds up on black vanishes on a light page. Never branch on a theme's
   or an engine's name.
5. **Keep out of the copy.** On editorial slides the copy sits in the left
   column and the figure's `place` is on the right; keep the left calm. Stay
   dark inside `Hint::Frame` boxes: that is where the chart or diagram is.
6. **Repaint only while moving.** Call `ui.ctx().request_repaint()` while an
   animation runs, not forever (the particles engine is the exception: its
   field never stops).
7. **Draw with egui meshes.** One `egui::Mesh` with a small sprite texture
   draws tens of thousands of quads per frame. Premultiplied colours with
   zero alpha add light (glow). Use a GL paint callback only for blending
   egui cannot do, and test it in export, which runs on the glow renderer.

## Capabilities and fallbacks

| Flag | Meaning when true |
|---|---|
| `paints` | The engine paints a layer (plain does not). |
| `editorial` | Copy slides use the editorial layouts: a copy column on the left, a stage on the right, display headings and the counter chrome. |
| `board` | The engine draws every slide itself, text included, and owns the transitions between slides (split-flap). `render_slide` then hands the slide to the engine's static renderer for thumbnails (`SlideContext::engine_drew` says whether the live engine drew it already), the app skips its transitions and scrolling, and the engine prints its own labels. |
| `illustrations` | Shows `@illustration`. |
| `stories` | Plays story beats (and they add reveal steps). |
| `countdown` | Draws the opening countdown itself (`countdown: burst`). |
| `end_act` | Plays an act of its own on the end slide. |

What an engine cannot show is reported, never silently dropped:
`engines::unsupported` names it per slide, `mdeck --check` lists it under the
`engine` category, and presenting and exporting print one summary line. If
your engine cannot show a kind of content, add its message there.

## Adding one: the checklist

1. `engines/<name>.rs`: a type implementing `Engine`, with unit tests for
   its geometry and its still.
2. `engines/mod.rs`: a variant in `EngineKind`, its name in `name()` and
   `ALL`, `available()` behind `cfg!(feature = "<name>")`, its
   `Capabilities`, `create()`, and `end_caption_delay()` if it plays an end
   act.
3. `crates/mdeck/Cargo.toml`: a feature, on by default.
4. A showcase theme: `crates/mdeck/themes/<theme>.yaml` with `engine: <name>`,
   listed in `theme::lookup::BUILTIN` behind the same feature.
5. `samples/engines/<name>.md`, a deck that shows what the engine is good at.
6. Docs: spec sections 9.1 (the theme) and 9.6 (the engine), the `@engine`
   and `@theme` lists in the spec and `spec --short`, README, GALLERY
   (stills as JPEG), CHANGELOG.
7. Look at it (below), then run the golden check so no other engine moved.

## A minimal engine

This engine draws the slide's illustration as dots that fade in. It is
compiled and tested as `engines/example.rs`, so it is always current:

```rust
/// Dots: the slide's illustration as accent-coloured dots that fade in.
pub struct Dots {
    /// The slide the fade belongs to, and how far it has come (0..1).
    slide: Option<usize>,
    shown: f32,
}

impl Dots {
    pub fn new() -> Self {
        Self {
            slide: None,
            shown: 0.0,
        }
    }
}

impl Engine for Dots {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, _lib: &mut Library) {
        // A new slide starts the fade over; export (`still`) shows it done.
        if self.slide != Some(stage.index) {
            self.slide = Some(stage.index);
            self.shown = 0.0;
        }
        self.shown = if cx.still {
            1.0
        } else {
            (self.shown + cx.dt / 0.6).min(1.0)
        };
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, stage: &Stage) {
        let Some(figure) = &stage.figure else {
            return;
        };
        let rect = cx.rect;
        let place = figure.place;
        let alpha = self.shown * cx.opacity * if figure.backdrop { 0.35 } else { 1.0 };
        let color = cx.theme.accent.gamma_multiply(alpha);
        for p in figure.cloud.points.iter() {
            let pos = egui::pos2(
                rect.left() + (place.u + p[0] * place.w) * rect.width(),
                rect.top() + (place.v + p[1] * place.h) * rect.height(),
            );
            ui.painter().circle_filled(pos, 2.5 * cx.scale, color);
        }
        if self.shown < 1.0 {
            ui.ctx().request_repaint();
        }
    }
}
```

Registered as described above, `@engine: dots` would show every
illustration as a constellation of accent dots.

## Looking at motion

Export captures stills, but two environment variables let it capture the
motion too:

```bash
# 0.3 s into the slide, simulated at 60 frames a second from a cold start
MDECK_EXPORT_AT=0.3 mdeck export deck.md --slide 3 --output-dir /tmp/frames

# the countdown digit 3, the burst, or the end act (with MDECK_EXPORT_AT for timing)
MDECK_EXPORT_MOMENT=3 MDECK_EXPORT_AT=0.6 mdeck export deck.md --slide 1 --output-dir /tmp/cd
MDECK_EXPORT_MOMENT=end MDECK_EXPORT_AT=2.0 mdeck export deck.md --slide 1 --output-dir /tmp/end
```

Export a few moments of an animation and look at them side by side before
calling it done. Then present the deck and step through it: timing is felt,
not seen in stills.

## Not breaking the others

Exports are deterministic, so a change that should not move other engines
can prove it. Build the last release and your branch, then:

```bash
scripts/engine-golden.sh old/mdeck target/release/mdeck
```

It exports the sample decks with both binaries and names every image that
differs. (`samples/ember/with-images.md` can differ from run to run with
image loading; everything else must be identical.)
