# 05. Engines

What an engine is, what it is responsible for, and what it is not.

An engine brings a slide to life around its content: the ground under the slide, the slide's
picture drawn in the engine's medium, the countdown and the end act, and reactions to what is on
the slide. It does not arrange the slide's text; that is the design set's job
([03](03-slide-designs.md)), chosen by the theme. The one exception is a **board** engine
(split-flap), which draws the whole slide in its own medium through a design set of its own.

Every engine, built-in or not, is registered as an `EngineDef` through the `mdeck-sdk` registry
([08](08-extensibility.md)): a name, a one-line summary, its capabilities, the settings it reads
from the theme's `engine:` block, what it needs from the theme, and a `create` function for the
runtime that implements the `Engine` trait (`update`, then `paint`, under the slide). The core
hands it everything through the **stage**: the moment (slide, countdown digit, burst or end), the
slide as the SDK content model and its step, the resolved picture, the theme's tokens, and the
geometry renderers publish (bars, paths, frames, heading glyphs, the copy box) so the engine can
react to content and keep clear of it.

## Requirements

### What an engine is

- **ENG-01** MUST `implemented`: An **engine** is the code that brings a slide to life around its
  content. It is responsible for exactly these:
  1. **the living layer:** the animated (or still) ground under and around the slide content;
  2. **the picture:** rendering the slide's picture ([06](06-visuals-and-pictures.md)) in the
     engine's medium, when the engine supports pictures;
  3. **the opening and ending:** the countdown and the end act, when the engine supports them;
  4. **reactions to content:** responding to what is on the slide (charts, images, headings)
     through published geometry, without drawing over it;
  5. **its own transition between slides**, when it declares one (a board must).
- **ENG-02** MUST `implemented`: An engine does **not** arrange slide content. Arranging content
  is the design set's job ([03](03-slide-designs.md)), chosen by the theme with `designs:`. There
  is no `editorial` capability; the editorial look is a design set any theme can use with any
  engine.
- **ENG-03** MUST `implemented`: A **board engine** is the one exception. It renders the whole
  slide in its medium by providing a design set (`EngineDef::board`) and owning the transition,
  and it declares which designs and block kinds it cannot show, so `--check` reports them
  (category `engine`, also with `--check --theme departures`).
- **ENG-04** MUST `implemented`: Capabilities describe what the *core* must do differently for an
  engine, never what an engine does internally. The capabilities are `picture`, `countdown`,
  `ending`, `board`, `transition` and `medium` (the artwork medium of an art engine). What used
  to be the one-engine capabilities `numbers_slides`, `cold_open` and `heat_trace` are generic
  `Engine` hooks: `numbers_slides` (the engine prints the slide number, so the core leaves out
  its counter), `copy_hold` (seconds the core holds back the copy of a title or section slide
  while the engine forms the heading) and `annotate` (the engine draws the presenter's pen
  strokes its own way).
- **ENG-05** MUST `implemented`: An engine never parses markdown, never reads the deck file,
  never calls the network, and receives everything through the stage. Engine code outside the
  host uses only `mdeck_sdk` and engine helpers, never egui (a test checks).
- **ENG-06** MUST `implemented`: The engine is chosen by `--engine`, then the deck's `engine`
  setting, then the theme's `engine:`.

### Contract with the core

- **ENG-07** MUST `implemented`: Stills are finished and deterministic. An export, at any moment
  (`export --at`, `--moment`), produces the same image every time, with no wall-clock
  randomness.
- **ENG-08** MUST `implemented`: The engine scales with the slide, paints only inside the slide
  (or the page), takes colours from theme tokens, and keeps content readable. It stays dark or
  calm inside frames that renderers publish (images, visuals) and behind the copy (`Hint::Copy`).
- **ENG-09** MUST `implemented`: Under reduced motion, the engine shows its settled state.
- **ENG-10** MUST `implemented`: The engine requests repaints only while something moves: the
  host stops repainting a settled slide when `Engine::animating` is false, and an engine that
  always moves (particles) keeps it true. The target is 60 fps at 4K on a 2020-class integrated
  GPU; no automated benchmark enforces it.
- **ENG-11** MUST `deferred to 2.x`: The engine's settings are typed and declared by the engine
  (`EngineDef::settings`) and validated from the theme's `engine:` block (THM-11) by `theme
  check` and `--check`: an unknown key or a value of the wrong type is reported. Saying that a
  theme's engine settings are ignored when the deck or `--engine` runs another engine is
  deferred to 2.x (THM-11).
- **ENG-12** MUST `implemented`: The engine declares what it needs from the theme
  (`EngineDef::needs`, today a page) and `theme check` warns when it is missing (THM-12).
- **ENG-13** MUST `implemented`: The engine interface is public and versioned in `mdeck-sdk`
  ([08](08-extensibility.md)). mdeck's ten engines implement it exactly as an external engine
  would and register through the same registry, and the guide's example engines (the SDK
  tutorials under `examples/`) are real engines that run in a custom mdeck built with
  `mdeck build`.

### Pictures and decoration

- **ENG-14** MUST `deferred to 2.x`: Which slides can show a picture is decided by the design (the
  arrangement has a stage), not by the engine. Any design whose arrangement has a stage shows the
  picture on any picture-capable engine; the `standard` design set has no stage. `--check`
  reports a picture set on a slide whose design has no stage. One exception remains in 2.0: a
  generated artwork shows on any slide the art pipeline resolves one for, not only where the
  design has a stage; deferred to 2.x.
- **ENG-15** MUST `implemented`: Decoration (backdrops, formations, layout-inferred scenes,
  reactions to visuals) belongs to the engine. It is never authored, the docs describe it only
  under the engine that draws it, and it may change between versions. It rotates by slide number
  so neighbours differ and exports stay reproducible.
- **ENG-16** MUST `implemented`: An engine falls back gracefully. Without an artwork, an art
  engine draws the point cloud in its medium; without a picture, it shows its ground. A missing
  optional asset never produces an empty or broken slide.

### Built-ins

- **ENG-17** MUST `implemented`: Each built-in engine has at least one showcase theme and one
  sample deck (`samples/engines/`, `samples/ember/` for particles), and is visibly distinct.
- **ENG-17a** MUST `implemented`: mdeck 2 ships **ten** engines:

  | Engine | Showcase themes | Notes |
  |---|---|---|
  | `plain` | `dark` (default), `light`, `nord`, `spring`, `summer` | no layer, no countdown, no pictures |
  | `particles` | `ember`, `autumn`, `winter` | |
  | `led` | `marquee` | |
  | `splitflap` | `departures` | board engine |
  | `blocks` | `stack` | |
  | `thermal` | `thermal` | |
  | `line` | `blueprint`, `chalkboard` | one engine for line art; `surface: sheet` or `surface: slate` |
  | `sketch` | `sketchbook` | |
  | `watercolour` | `watercolour` | |
  | `darkroom` | `darkroom` | |

  - **`line`** draws line art (generated artworks in the shared line style, or point clouds) on
    a surface chosen by an engine setting: `surface: sheet` (Prussian-blue sheet, grid,
    construction lines, drafting crosshair, dimension lines, title block) or `surface: slate`
    (chalk grain and falling dust on the theme's slate). The `blueprint` and `chalkboard` themes
    select their surface in their `engine:` block (THM-11).
  - There is no `laser` engine and no `etch` theme: the laser's settled state was a faint
    outline, and the line, sketch and particle engines cover drawn pictures.
- **ENG-17b** SHOULD `implemented`: The fallback that art engines draw when a slide has no
  generated artwork looks finished: the line engine traces the point cloud's lines clean with
  section hatching, sketch adds hatching and cross-hatching, watercolour lays washes over the
  shape, and darkroom makes a photogram.
- **ENG-18** SHOULD `implemented`: Engines are cargo features (all on by default), so a minimal
  build is possible; CI builds with none and with each one alone. A custom build can carry only
  its own engines ([08](08-extensibility.md)).
