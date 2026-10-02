# Design sets and board engines

A **design** is a kind of slide (`title`, `points`, `split`, `quote`, ...) with named roles; mdeck
recognises it from the content or the author chooses it (`<!-- design: statement -->`). A
**design set** arranges every design. mdeck's `standard` and `editorial` sets are data, and a
theme can restyle them without code. Write a design set in code when data cannot express the look:
a poster wall, a kiosk layout, or the whole-slide medium of a board engine.

Start with the scaffold:

```bash
mdeck sdk new design-set poster
```

## The `DesignSet` trait

```rust
pub trait DesignSet: Send + Sync {
    fn name(&self) -> &str;                                          // `designs: poster`
    fn render(&self, cx: &mut DesignCx, slide: &Slide, rect: Rect);  // draw the slide
    fn measure(&self, cx: &mut DesignCx, slide: &Slide, rect: Rect) -> f32 { rect.height() }
    fn unsupported(&self, slide: &Slide) -> Vec<Problem> { Vec::new() }
}
```

Register it, and a theme selects it:

```rust
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.design_set(Box::new(Poster))?;
    r.theme("poster", include_str!("../theme.yaml"))
}
```

```yaml
name: poster
extends: dark
designs: poster
```

### `render`

`render` draws the slide into `rect` at the reveal step `cx.step()`. It receives the parsed
content as `mdeck_sdk::content::Slide`: blocks (headings, paragraphs, lists, quotes, code, tables,
images, visuals), the slide's design name in `slide.design`, its settings and its notes. A design
set never parses markdown itself.

- `slide.title()` and `content::plain_text` give text without formatting.
- `ListMarker::NextStep` marks a list item revealed on the next step; show it when `cx.step()` has
  reached it.
- `cx.tokens()` gives the theme's colours, `cx.scale()` the scale to multiply every size by,
  `cx.animate()` whether to animate (false for stills and reduced motion).
- `cx.publish(Hint::Frame(rect))` tells the engine where the copy is, so it stays calm there.
  Publish the copy's box and every image or card.

The content model is a first draft in SDK 2.0 and may gain fields in minor versions (never lose
them); match on it with a wildcard arm (`_ => {}`) so new block kinds do not break you.

### `measure`

The height `slide` needs at `rect`'s width. When it is taller than the slide, mdeck scrolls the
slide smoothly. The default says everything fits.

### `unsupported`: say what you do not show

A design set may not show every block kind. That is fine, but silently dropping content is not:
`unsupported` returns one problem per thing it leaves out, and `--check` reports them.

```rust
fn unsupported(&self, slide: &Slide) -> Vec<Problem> {
    slide
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Image { .. } => Some("images"),
            Block::Visual { .. } => Some("visuals"),
            _ => None,
        })
        .map(|what| Problem::new("design", format!("the poster design set does not show {what}")).at(slide.line))
        .collect()
}
```

## Board engines

A **board engine** draws every slide itself, text included, in its own medium: mdeck's split-flap
departures board is one. It is an engine whose definition carries a design set:

```rust
static BOARD: FlapBoard = FlapBoard; // implements DesignSet

pub static DEF: EngineDef = EngineDef {
    name: "flaps",
    summary: "Every slide on a split-flap board.",
    capabilities: Capabilities {
        board: true,       // the core draws slides through `board`, not the theme's design set
        transition: true,  // and leaves the change between slides to the engine
        countdown: true,
        ending: true,
        ..Capabilities::NONE
    },
    settings: &[],
    needs: Needs { page: false },
    ending_caption_delay: 3.0,
    create: |_| Box::new(Flaps::default()),
    board: Some(&BOARD),
};
```

- With `board: true`, the core arranges every slide with `DEF.board` instead of the theme's design
  set. The engine's `update` and `paint` still run every frame under it, for the board's surface
  and its motion.
- With `transition: true`, the core does not slide or fade between slides. The engine sees
  `stage.index` change and animates the change itself (the flaps flip from the old text to the
  new). A board must own its transitions: sliding a board would break the illusion.
- `unsupported` matters more for a board than for any other design set: a split-flap board cannot
  show images or charts, and `--check` must say so for every slide that has them.

## Testing

`Headless::render_design` draws a slide with a design set and returns the image, the published
hints and the measured height:

```rust
let out = Headless::new(480, 270).render_design(&Poster, &slide, 0, &Tokens::default());
assert_eq!(out.hints.len(), 1);
assert_golden(path, &out.image, GOLDEN_TOLERANCE);
```

Build test slides directly from `content` types (see the scaffold's `tests/golden.rs`), and test
`unsupported` with one slide per block kind you leave out.
