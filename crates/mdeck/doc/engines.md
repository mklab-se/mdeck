# Writing a built-in engine

An **engine** brings a slide to life around its content: the living layer
under the slide, the slide's picture in the engine's medium, the countdown
and the end act, reactions to what the slide shows, and (for a board) its own
transitions (ENG-01). MDeck ships `plain`, `particles` (Ember), `led`
(Marquee), `splitflap` (Departures), `blocks` (Stack), `thermal` (Thermal),
and the art engines `line` (Blueprint, Chalkboard), `sketch` (Sketchbook),
`watercolour` and `darkroom`.

**Built-in engines are written exactly like an extension's** (EXT-06,
ENG-13): against the public SDK, `crates/mdeck-sdk`, and nothing else of
mdeck. So the guide for writing one is the SDK documentation:

- [`docs/sdk/getting-started.md`](../../../docs/sdk/getting-started.md): from
  nothing to an engine running in a custom mdeck;
- [`docs/sdk/concepts.md`](../../../docs/sdk/concepts.md): the model, the frame
  lifecycle, the stage and the contract every engine keeps;
- the three tutorials (`docs/sdk/tutorial-1-ambience.md` and on), each a
  tested engine under `examples/`;
- the API reference: `cargo doc -p mdeck-sdk --open`.

This page covers only what is different about an engine that ships inside
mdeck. How an engine gets there (build it as an extension, propose it with
exports, then promote it) and what the pull request must contain are in
[Contributing an engine](../../../CONTRIBUTING.md#contributing-an-engine).

## Where it lives

All of it lives in `crates/mdeck/src/engines/`:

| Piece | Where | What it is |
|---|---|---|
| your engine | `engines/<name>.rs` or `engines/<name>/` | A type implementing `mdeck_sdk::engine::Engine` and its `pub static DEF: EngineDef`. |
| registration | `engines::register` in `engines/mod.rs` | `r.engine(&<name>::DEF)?`, behind the engine's cargo feature, like an extension's `register`. |
| shared helpers | `engines/art/` (art engines), `engines/rng.rs`, `engines::hash01`, `engines/heat_palette.rs` | Engine-only code several engines (or the core) use. |
| the host | `engines/host/` | The core's half, not an engine: it converts mdeck's slide, theme and published geometry to the SDK's types, resolves the picture, builds the `Stage`, `Frame` and `Painter` each frame and calls your engine. `EngineId` (in `mod.rs`) is the handle a theme carries. |

**The boundary.** Every file under `engines/` except `engines/host/` may use
only `mdeck_sdk`, std and other engine helpers under `crate::engines::`; no
egui, no `crate::render`, `crate::theme` or `crate::parser`. A test
(`engines::tests::engines_stay_inside_their_boundary`) enforces it. If your
engine needs something the SDK does not offer, add it to the SDK (a small,
documented, additive method with a doctest) rather than reaching into mdeck.

## What the host does for you

- **The stage.** The slide as the SDK content model (`stage.slide`, its
  `design` is the slide's design name), the picture (`stage.picture`: on an art
  engine the slide's generated artwork, else the point cloud it names, else
  the image file it names, placed by the design), the moment (slide,
  countdown digit with its glyph mask, burst, end with the end words), and
  the geometry the slide's visuals drew last frame (`stage.geometry`; a
  heading is `Hint::Text`, one per glyph where the layout put it).
- **The frame.** Rect, scale, opacity, `dt`, `still` (export), reduced motion,
  the theme's colour tokens, and your engine settings.
- **Settings.** Declare `SettingSpec`s in `DEF.settings` and read them in
  `create`. The host builds them from the theme with
  `theme::engine_settings`; `--check` and `mdeck theme check` report values
  of the wrong type and keys your engine never reads.
- **Hooks.** `annotate` draws the presenter's pen strokes your way (the
  thermal heat trace), `copy_hold` holds title and section copy back while
  the engine forms the heading (the thermal cold opening), `numbers_slides`
  tells the core to leave out its counter (the line engine's sheet), and
  `animating` tells the host when to stop repainting.
- **A board** (`Capabilities.board`) gives `DEF.board` a `DesignSet` that draws
  every slide (`render::board` calls it, lending deck images and visuals
  through `DesignCx::image` and `DesignCx::visual`); its `unsupported` feeds
  `--check`. With `Capabilities.transition` the core does no slide motion.
- **An art engine** sets `Capabilities.medium`; the core generates and
  prepares pictures in that medium (`render::art`, `mdeck ai pictures`) and
  hands them over as `PictureSource::Artwork`. `engines/art/` holds what the
  art engines share: the `Canvas` that follows the stage, `Drawing` (the CPU
  reveal into a texture), and `fallback_strokes` for a point cloud.

## Adding one: the checklist

1. `engines/<name>.rs`: the engine and its `DEF`, with unit tests for its
   geometry and its still (`mdeck_sdk::testing::Headless` renders frames
   headlessly).
2. `engines::register`: the module and the registration behind
   `#[cfg(feature = "<name>")]`.
3. `crates/mdeck/Cargo.toml`: a feature, on by default (an art engine's
   feature turns on `art` too); its name in `ENGINES` in `crates/mdeck/build.rs`
   (for the `all_engines` dead-code check) and in the `features` matrix of
   `.github/workflows/ci.yml`, which builds with it alone and with no engines.
4. A showcase theme: `crates/mdeck/themes/<theme>.yaml` naming the engine,
   listed in `theme::lookup::BUILTIN` behind the same feature.
5. `samples/engines/<name>.md`, a deck that shows what the engine is good at.
6. Docs: the spec's theme and engine sections, the lists in the spec and
   `spec --short`, README, GALLERY, CHANGELOG.
7. Look at it (below), then run the golden check so no other engine moved.

## Looking at motion

Export captures stills, but two flags let it capture the motion too:

```bash
# 0.3 s into the slide, simulated at 60 frames a second from a cold start
mdeck export deck.md --slide 3 --at 0.3 --output-dir /tmp/frames

# the countdown (its 3; or 2, 1, burst), or the end act, with --at for timing
mdeck export deck.md --slide 1 --moment countdown --at 0.6 --output-dir /tmp/cd
mdeck export deck.md --slide 1 --moment end --at 2.0 --output-dir /tmp/end
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

## Modules, not crates

The built-in engines stay modules of the `mdeck` crate, behind cargo
features, rather than crates of their own (decided in #16): they would gain
nothing a module boundary does not already give, and every release would
publish more crates. Since v2 the boundary they keep is the public SDK
itself, so moving an engine out into its own crate, or writing a new one
outside this repository, is the same code with a different `register` call.
