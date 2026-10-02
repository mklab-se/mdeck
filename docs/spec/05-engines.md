# 05. Engines

What an engine is, what it is responsible for, and what it is not.

## Today

### The definition, as written

The docs say an engine is "what a theme does beyond colours and type: the layer it paints under
the slides, and what it plays for the countdown and the end" (`crates/mdeck/doc/engines.md`). The
format spec (9.6) adds a third job: "how text slides are laid out". The design rule is sound:
"the core decides what a slide wants to show; an engine decides how it looks". Engines never parse
markdown or read files.

### The machinery

**`EngineKind`.** An enum of 12 engines (`src/engines/mod.rs:59`):

- `plain`, `particles`, `led`, `splitflap`, `laser`, `blocks`, `thermal`
- `blueprint`, `sketch`, `chalkboard`, `watercolour`, `darkroom`

Each is a cargo feature, and all are on by default.

**`EngineDef`.** Each engine's registration holds:

- its `capabilities`;
- `create`, which builds the runtime;
- for art engines, a `medium`;
- for board engines, a static `render_slide`;
- optionally a `problems` hook for `--check`.

**The `Engine` trait.** Three hooks:

- `prepare` (optional);
- `update(stage)`;
- `paint(stage)`, which draws *under* the slide.

**`Stage`.** Everything the engine receives each frame:

- the moment: slide, countdown digit, burst or end;
- the slide and its reveal step;
- the resolved picture (`figure`, `art`) and the story;
- **hints**: geometry that renderers publish (bars, paths, circles, frames, heading glyphs), so an
  engine can react to a chart or keep dark behind an image.

**`Host`.** The engine-neutral half, owned by the app and export. It keeps the clock, builds the
glyph masks, resolves the picture, collects hints and builds the `Stage`.

**Choosing.** `--engine` beats `@engine`, which beats the theme's `engine:`.

### The 11 capabilities

| Capability | Effect | Engines |
|---|---|---|
| `paints` | the engine layer is painted under the slide | all but plain |
| `editorial` | copy slides use the Ember editorial layouts and chrome | particles, led, laser, blocks, thermal, art engines |
| `board` | the engine renders the whole slide; transitions off | splitflap |
| `illustrations` | `@illustration` is resolved and handed to the engine | as editorial |
| `stories` | story beats add steps; the story is staged | particles |
| `countdown` | the engine draws the countdown | all but plain |
| `end_act` | the engine replaces "The End" | all but plain |
| `art` | generated artworks are resolved and handed to the engine | five art engines |
| `numbers_slides` | suppresses the editorial counter | blueprint |
| `cold_open` | headings form in the field before the copy appears | thermal |
| `heat_trace` | pen annotations drawn as a heat trace (implemented in the app) | thermal |

### The engines

| Engine | What it does | Showcase theme |
|---|---|---|
| plain | nothing; the generic layouts draw everything | dark, light, nord, spring, summer |
| particles | 900 glowing particles that migrate between shapes: digits, words, illustrations, story casts, chart-reactive embers, layout scenes on rotating backdrops | ember, autumn, winter |
| led | an LED wall; pictures light up, marquee border on titles | marquee |
| splitflap | an airport departures board: draws every slide as text on a 32×12 flap grid, owns transitions | departures |
| laser | a beam etches pictures and traces charts, with sparks and smoke | etch |
| blocks | pictures fall as Tetris pieces | stack |
| thermal | a heat field; cold open; heat signatures; heat trace | thermal |
| blueprint | inks line art with construction lines on its own sheet | blueprint |
| sketch | pencil outlines and shading on the theme's paper | sketchbook |
| chalkboard | chalk line art on the theme's slate | chalkboard |
| watercolour | tonal pictures bloom on paper | watercolour |
| darkroom | prints develop under a safelight | darkroom |

### Size

The engine area is substantial:

- about 9,000 lines in `src/engines`;
- about 3,100 in `render/particles`;
- plus `render/art`, `render/strokes` and `render/story`;
- 11 cargo features with a CI matrix and a `cfg(all_engines)` arrangement for dead-code checks;
- per-engine documentation in five places.

## Assessment

1. **"Engine" is four concepts in one setting:**
   - (a) a living layer under the slide;
   - (b) a family of text layouts (`editorial`);
   - (c) a whole-slide renderer (`board`);
   - (d) a consumer of generated artworks.

   Picking `led` silently changes how bullets are arranged. Picking `plain` silently loses the
   editorial design. Nobody can predict this from the word "engine".
2. **Capabilities that serve one engine are names in disguise.** `numbers_slides`, `cold_open`,
   `heat_trace` and `board` each belong to exactly one engine. Two of them are implemented
   outside the engine (`heat_trace` in `app/overlays.rs`, `cold_open` in `render/ember`). The rule
   "branch on capabilities, never on names" is kept in letter, not in spirit.
3. **Ember naming leaks everywhere.** The editorial layouts every picture engine uses live in
   `render::ember`. The hint store is `"ember-hints"`. The particle tints are read by non-particle
   engines.
4. **Features reachable through one engine only.** Stories are particles-only. Backdrops and
   formations are particles internals, yet `CLAUDE.md` describes them as a top-level pattern.
5. **Pictures appear on fewer slides than "supports illustrations" suggests.** Only slides the
   editorial design handles show a picture. A slide with an image, code, table or chart never
   shows one, and `--check` reports only the capability, not this.
6. **The guide's example engine cannot be selected.** It is compiled for tests only.
7. **Probable defects found by reading** (not reproduced at runtime):
   - hint collection is never switched off after leaving a painting engine, so hints accumulate;
   - story beat ticks may show on engines that cannot play stories;
   - `--engine particles` on the default theme gets no countdown.

## Requirements

### What an engine is

- **ENG-01** MUST `change`: An **engine** is the code that brings a slide to life around its
  content. It is responsible for exactly these:
  1. **the living layer:** the animated (or still) ground under and around the slide content;
  2. **the picture:** rendering the slide's picture ([06](06-visuals-and-pictures.md)) in the
     engine's medium, when the engine supports pictures;
  3. **the opening and ending:** the countdown and the end act, when the engine supports them;
  4. **reactions to content:** responding to what is on the slide (charts, images, headings)
     through published geometry, without drawing over it;
  5. **its own transition between slides**, when it declares one (a board must).
- **ENG-02** MUST `change`: An engine does **not** arrange slide content. Arranging content is the
  design set's job ([03](03-slide-designs.md)), chosen by the theme. The `editorial` capability
  disappears; the editorial look becomes a design set any theme can use with any engine.
- **ENG-03** MUST `change`: A **board engine** is the one exception. It renders the whole slide in
  its medium. It does so by providing a design set and a transition, and it declares which
  designs and block kinds it cannot show, so `--check` reports them.
- **ENG-04** MUST `change`: Capabilities describe what the *core* must do differently for an
  engine, never what an engine does internally:
  - Kept: `picture`, `countdown`, `ending`, `board`, `transition`, `artwork medium`.
  - Removed as capabilities: `numbers_slides`, `cold_open`, `heat_trace`. These become engine
    internals behind generic hooks (for example a hook for drawing annotations, and a chrome
    setting for the counter).
- **ENG-05** MUST `keep`: An engine never parses markdown, never reads the deck file, never calls
  the network, and receives everything through the stage.
- **ENG-06** MUST `keep`: The engine is chosen by `--engine`, then the deck setting, then the
  theme.

### Contract with the core

- **ENG-07** MUST `keep`: Stills are finished and deterministic. An export, at any moment,
  produces the same image every time, with no wall-clock randomness.
- **ENG-08** MUST `keep`: The engine scales with the slide, paints only inside the slide (or the
  page), takes colours from theme tokens, and keeps content readable. It stays dark or calm
  inside frames that renderers publish (images, visuals) and behind the copy.
- **ENG-09** MUST `keep`: Under reduced motion, the engine shows its settled state.
- **ENG-10** MUST `new`: The engine has a performance budget. It holds 60 fps at 4K on a
  2020-class integrated GPU, and requests repaints only while something moves. An engine that
  always moves (particles) declares that it does.
- **ENG-11** MUST `new`: The engine's settings are typed and validated by the engine itself, from
  the theme's `engine:` block (THM-11). Unknown or inert settings are reported.
- **ENG-12** MUST `new`: The engine declares what it needs from the theme (for example a page) and
  what it provides itself (THM-12).
- **ENG-13** MUST `new`: The engine interface is public and versioned (see
  [08](08-extensibility.md)). mdeck's own engines implement it exactly as an external engine
  would, and the guide's example engine is a real, selectable engine built from that interface.

### Pictures and decoration

- **ENG-14** MUST `change`: Which slides can show a picture is decided by the design (the
  arrangement has a stage), not by the engine. Any design whose arrangement has a stage shows the
  picture on any picture-capable engine. `--check` reports a picture set on a slide whose design
  has no stage.
- **ENG-15** MUST `keep`: Decoration (backdrops, formations, layout-inferred scenes, reactions to
  visuals) belongs to the engine. It is never authored, never part of the format reference, and
  may change between versions. It rotates by slide number so neighbours differ and exports stay
  reproducible.
- **ENG-16** MUST `keep`: An engine falls back gracefully. Without an artwork, an art engine
  draws the point cloud in its medium; without a picture, it shows its ground. A missing optional
  asset never produces an empty or broken slide.

### Built-ins

- **ENG-17** MUST `new`: Each built-in engine has at least one showcase theme and one sample deck,
  and must justify its maintenance cost by being visibly distinct.
- **ENG-17a** MUST `change`: v2 ships **ten** engines (decided after the side-by-side review of
  all showcases):

  | Engine | Showcase themes | Status |
  |---|---|---|
  | `plain` | `dark` (default), `light`, `nord` | keep |
  | `particles` | `ember` | keep |
  | `led` | `marquee` | keep |
  | `splitflap` | `departures` | keep |
  | `blocks` | `stack` | keep |
  | `thermal` | `thermal` | keep |
  | `line` | `blueprint`, `chalkboard` | `change`: the blueprint and chalkboard engines merged |
  | `sketch` | `sketchbook` | keep |
  | `watercolour` | `watercolour` | keep |
  | `darkroom` | `darkroom` | keep |

  - **`line`** draws line art (generated artworks in the shared line style, or point clouds) on
    a surface chosen by an engine setting: `surface: sheet` (today's Blueprint: Prussian-blue
    sheet, grid, construction lines, drafting crosshair, dimension lines, title block) or
    `surface: slate` (today's Chalkboard: chalk grain and falling dust on the theme's slate). Both
    themes stay and select their surface in their `engine:` block (THM-11).
  - **`laser` is removed** (`remove`), together with its showcase theme `etch`. It was spectacular
    in motion but its settled state was a faint outline that overlapped with the line-drawing
    engines.
- **ENG-17b** SHOULD `new`: The fallback that art engines draw when a slide has no generated
  artwork looks finished. Today the line, sketch and watercolour engines draw a point cloud as a
  thin squiggle, the weakest image in the review; darkroom's photogram shows the standard to
  reach.
- **ENG-18** SHOULD `keep`: Engines remain cargo features, so a minimal build is possible. In v2
  that matters more for third-party builds that want only their own engine (see
  [08](08-extensibility.md)).
