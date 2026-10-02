# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- Chart reactions no longer leak onto the next slide: LED peak markers and other engine reactions
  to a chart stay on the chart's slide instead of showing on the slides after it.
- The thermal engine's end slide exports with its glowing "THE END" instead of an empty frame.

## [1.19.0] - 2026-10-02

### Added

- `@flower` visualization ([#3](https://github.com/mklab-se/mdeck/issues/3)): a platform in the
  centre and the teams around it. Each `petal` is a bulb in its own colour whose outline runs out
  of the `center`, around the bulb and back in with an arrow; `A -> B` lines link petals with
  curves that keep clear of the centre. Petals are spaced evenly for any count, the flower sizes
  itself for its text, and `+` makes a petal grow out of the centre. Sample:
  `samples/visualizations/flower.md`; spec section 14.18.
- `@artifactflow` visualization ([#4](https://github.com/mklab-se/mdeck/issues/4)): artifacts
  moving from `producer`s through `service`s to `consumer`s. Producers and consumers sit in
  titled panels (`# producers: Title | subtitle`), services are larger cards in the middle with
  optional indented bullet items, and every `A -> B: artifact (icon: package)` edge is a smooth
  arrow with its label at its quieter end. Without edges, everything flows through the services.
  Sample: `samples/visualizations/artifactflow.md`; spec section 14.19.
- Three more built-in icons: `team`, `package` and `code`, for architecture diagrams too.

- Thermal images and a thermal look for infrared talks ([#20](https://github.com/mklab-se/mdeck/issues/20)).
  - **`@thermal` blocks** show one thermal image and help explain it. A grayscale white-hot
    export is coloured with a palette (`iron`, `white-hot`, `black-hot`, `rainbow`, `arctic`,
    `lava`; `palette:` per block, `@palette` for the deck, `C` / `Shift+C` live), with a legend.
    Steps reveal it: `lens X% Y% R%` over a registered `visible:` photo (it glides, then stays
    put), `reveal` opens it to the full frame, `above 85%` colours only what is hotter, and
    `spot Name X% Y%` marks points. Ordinary images are never treated as thermal.
  - **What a picture may claim.** A plain export gets a *relative intensity* legend and author
    spot text marked `†`. With `mapping: linear 18..92 °C` or a `data:` file (16-bit PNG and a
    `.yaml` sidecar with unit, scale, offset, and codes for missing and clipped pixels), the
    legend shows values, spots are measured (`≈` for a mapping), thresholds can be in degrees,
    `window:` sets level and span, and clipped or missing pixels are hatched. Two blocks on a
    slide with `@thermal-window: 25..90 °C` share one scale, converted from their own mappings.
  - **Colour exports** are detected from their pixels and shown as they are, with a diagnostic
    when presenting, exporting and in `--check` (new `thermal` category). Steps a source cannot
    show are left out, so no click does nothing; the lens still works on colour exports.
  - **`@zoom: Spot`** enters a slide by zooming into that spot of the previous slide's image.
- The **`thermal` engine** and **`thermal` theme**: a heat field under the slides in the theme's
  `heat.palette`, drawn in contour bands. Title and section headings form in heat (the cold
  opening: readable within a second, then the crisp type rises into a settling contour halo),
  illustrations glow as heat signatures, the countdown and end words heat up and cool, the
  field stays dark around charts and images, `heat: { drift: true }` adds drifting embers, and
  pen strokes become a heat trace that cools away. A board engine shows `@thermal` blocks,
  annotations included, in its image panel.
- `--reduced-motion` (and `mdeck config set defaults.reduced_motion true`): every slide and
  reveal step in its settled state, with no transitions, entry or reveal animations, countdown
  or engine motion; steps still arrive one click at a time.
- Engines can implement `Engine::prepare` to read what needs the UI before each update.
- Samples: `samples/engines/thermal.md`, `samples/features/thermal.md` (every option), with
  synthetic example images in `samples/images/thermal/`; spec section 14.20.

## [1.18.0] - 2026-10-02

### Added

- Background images ([#19](https://github.com/mklab-se/mdeck/issues/19)). `@background: file`
  in the frontmatter puts an image behind every slide; the same directive under a slide's heading
  replaces it there, and `@background: none` turns it off. `@background-opacity` (0 to 1, or a
  percentage, default 30%) tones it down at either level; a slide that sets only the opacity
  reuses the deck's image. The image covers the slide on the theme's background colour, under
  the engine layer and the content, sits on the sheet on page themes, moves with its slide in
  transitions, fades in once loaded, and is identical in PNG and PDF export. PNG, JPEG, WebP and
  SVG work. `mdeck --check` reports missing or unreadable files and bad opacities with their
  line. Sample: `samples/features/backgrounds.md`; spec section 9.8.

### Fixed

- SVG images in slides (`![Diagram](diagram.svg)`) are drawn; they showed only a placeholder.

## [1.17.0] - 2026-09-28

### Added

- `mdeck --check` names the line in the file for each warning: `slide 5 (line 37): [directive]
  ...`. Directive, illustration, art and diagram warnings point at their own line; the rest at
  the slide's first line.

### Fixed

- A directive written with a space before the colon (`@layout : title`) is treated as a
  directive when slides are split, as it already was when the slide is read; `@:` is not.
- The diagram debug overlay (`R`) shows the routes the slide and `--check` compute (measured
  lane capacities and the configured routing weights), not a default configuration's.
- A theme's `page.surface` given with alpha (`#80808080`) is painted in the colour written; it
  came out darker.

### Changed

- The rest of the refactor: code only some engines use is left out of builds without them
  (instead of a crate-wide `allow(dead_code)`), the app's state is grouped, no production
  function is longer than 100 lines, errors carry `anyhow` context instead of plain strings, the
  story and art sidecars share one reader and writer, the five engines with a countdown share
  one notion of what a moment shows, and the renderer no longer reads the config file itself.
  Rendered output is unchanged, pixel for pixel.

## [1.16.0] - 2026-09-28

### Fixed

- Frontmatter is read as YAML again. The `@` keys (`@theme: dark`) are not valid plain YAML
  keys, so every real deck fell back to splitting lines at the first colon: a trailing
  `# comment` or single quotes ended up in the value. Lines that are not valid YAML (a title
  with a second colon) are still read line by line.
  A multi-line `@story: >` hint now reaches `mdeck ai story` in full; it used to arrive as
  just `>`. Stories written against the old hint show as stale once (they still play); run
  `mdeck ai story --stale` to refresh them.
- `mdeck ai create` reads visualization opportunities with a JSON parser, so `\u00e5`-style
  escapes and quotes inside values come through intact.
- Error messages show their cause: "AI request failed" now says why it failed.
- Diagram nodes with a space after `prompt:` (`prompt: 'a server rack'`) no longer leave the
  closing quote in the rest of the node's metadata.
- Diagram routing falls back to three lanes per corridor when no node could be measured, instead
  of treating the corridor as unlimited.
- A line like `1 . x` (a number, a space, then a dot) no longer hangs the parser; it reads as
  plain text.
- `mdeck export` reports problems in the story sidecar, as presenting does.

### Changed

- YAML goes through `serde_norway`, the maintained fork of the archived `serde_yaml`.
- The engine cargo features now leave their engines out of the build: `cargo install mdeck
  --no-default-features --features led` builds MDeck with only the LED engine (and `plain`).
- Generated art loads on a small pool of worker threads, the slide on screen first, instead of
  one thread per slide.
- The window title and the header of `config.yaml` no longer use an em-dash.
- A structural refactor of the whole code base: presenting and export draw slides through one
  shared `Deck`, drawing code takes context structs instead of long argument lists, and the
  largest files (the app, text, Ember layouts, diagrams, charts, parser, themes, engines) are
  split into focused modules. Rendered output is unchanged, pixel for pixel.

## [1.15.2] - 2026-09-28

### Fixed

- AI images work with MAI image models such as MAI-Image-2.6-Flash
  ([#18](https://github.com/mklab-se/mdeck/issues/18)): ailloy is updated to 2.2.1, and
  `mdeck ai art` gives up what a model does not take (the gpt-image quality, JPEG output and
  compression first, then the style reference images) instead of failing, remembering it for
  the rest of the run. Pictures are re-encoded to compact JPEG either way.

## [1.15.1] - 2026-09-28

### Fixed

- On the editorial layouts, the slide's copy no longer animates in again by itself a few
  seconds after a picture has finished drawing, and revealing the next bullet no longer
  replays every bullet. A slide counted as re-entered when it had not been painted for a
  quarter second, so an idle window (the art engines stop repainting once their picture is
  done) replayed the entrance on its next frame. It now counts as re-entered only when it was
  not drawn in the previous frame.
- Stepping back through bullets only removes the last one. The bullet that became the last
  one used to rise in again when Back came within a second of Next, so going back and forth
  looked like every bullet being rewritten; and nine seconds after entering a slide, bullets
  stopped animating at all. Now only Next animates, and always the same way. Revealed chart
  and diagram steps no longer animate again on Back either.

## [1.15.0] - 2026-09-28

### Added

- The **watercolour engine** and its `watercolour` theme
  ([#17](https://github.com/mklab-se/mdeck/issues/17)): cold-press paper on a table, and a
  slide's generated watercolour blooms onto it, a pale first wash and then the colour spreading
  from where the paint is heaviest, the dark accents last. Without art the slide's
  `@illustration`, the countdown and the end words are drawn in ink with loose washes. Sample
  `samples/engines/watercolour.md`.
- The **darkroom engine** and its `darkroom` theme: a red safelight over the bench, and a
  slide's generated black-and-white photograph develops as a print on white fibre paper,
  shadows first, before the white light comes on and the print shows its true greys. Without
  art the slide's `@illustration` becomes a photogram. Sample `samples/engines/darkroom.md`.
- With these, all five media proposed in #17 are in: blueprint, sketch, chalkboard,
  watercolour and darkroom.

## [1.14.0] - 2026-09-28

### Added

- The **chalkboard engine** and its `chalkboard` theme, the third art engine
  ([#17](https://github.com/mklab-se/mdeck/issues/17)): a green slate in a wooden frame with
  the ghosts of earlier drawings wiped off it. A slide's generated line art is drawn in chalk
  along its strokes, breaking up in clumps on the slate, with a stick of chalk at the point and
  dust falling from it. It shares line art with the blueprint, so a deck switches between the
  two without new pictures. Without art, the slide's `@illustration`, the countdown and the end
  words are drawn in chalk. Sample `samples/engines/chalkboard.md`.
- `Reveal::grain` lets a medium break up on its surface (engine guide).

## [1.13.0] - 2026-09-28

### Added

- The **sketch engine** and its `sketchbook` theme, the second art engine
  ([#17](https://github.com/mklab-se/mdeck/issues/17)): every slide is a sheet of drawing
  paper on a desk, and the slide's generated drawing, graphite and ink in the MKLab house style
  (old craft meeting modern technology), is drawn in by a pencil you can see: the outlines
  first, traced along the lines, then the shading laid in stroke by stroke in bands that sweep
  across the picture. Without art, the slide's `@illustration`, the countdown and the end
  words are drawn in pencil. Sample `samples/engines/sketch.md`.

### Changed

- The art engines share their slide, countdown and end bookkeeping (`engines::art::Canvas`),
  so a new medium is only its drawing.

## [1.12.0] - 2026-09-28

### Added

- **Art engines** ([#17](https://github.com/mklab-se/mdeck/issues/17)): engines that draw a
  picture generated for each slide with the configured image model, drawn in as the slide
  opens. The first is the **blueprint engine** and its `blueprint` theme: every slide is a
  Prussian blue drawing sheet on a drafting table, with a grid, a ruled border and a title
  block carrying the deck's title and the sheet number. The slide's line art is inked the way
  a draughtsman works: construction lines run ahead, the ink follows stroke by stroke under a
  drafting machine's crosshair, and dimension lines are ruled around the finished drawing.
  Without art, the slide's `@illustration` is drawn with a technical pen, as are the countdown
  and the end words. Sample `samples/engines/blueprint.md`.
- `mdeck ai art talk.md` draws the pictures (`--slide`, `--stale`, `--force`, `--dry-run`,
  `--engine`, `--node`), four at a time with retries. `@art:` in the frontmatter sets the
  deck's world, under a slide's heading its scene (otherwise the chat model writes one from
  the copy and notes), and `@art: none` skips a slide. Pictures go in `art/` next to the deck
  and `talk.art.yaml` records them by slide hash and style, with `pinned: true` to keep one.
  Presenting never calls the AI. `S` while presenting draws the current slide's picture in
  the background.
- `mdeck --check` names slides without a picture and stale pictures (`art` category), and
  presenting prints one line when a deck on an art engine lacks art. An `@art` scene on an
  engine that draws no art is reported under `engine`.
- Themes gain a `page:` block (the slide as a sheet on a surface, with shadow and paper grain,
  on any engine) and an `art:` block (the kind, style prompt and style swatches of generated
  art).

### Changed

- `S` does AI for the current slide on either kind of engine: a story on the particles
  engine, a picture on an art engine.

## [1.11.1] - 2026-09-28

### Added

- A test keeps engines inside their boundary: every file under `src/engines/` may use the
  stage, the theme, the parsed slide and the shared render helpers, never the app, the
  commands, the config or the CLI ([#16](https://github.com/mklab-se/mdeck/issues/16)). The
  engine guide records why engines stay modules rather than crates.
- `samples/introducing-mdeck.md` has a slide on the six engines.

### Changed

- README, GALLERY and the spec count twelve built-in themes and show every engine; the Ember
  and Winter gallery stills are re-exported with the editorial text colours fixed in 1.9.0.

## [1.11.0] - 2026-09-28

### Added

- The laser engine and its `etch` theme ([#16](https://github.com/mklab-se/mdeck/issues/16)):
  a beam from in front of the screen etches each illustration onto the slide in about two
  seconds, along a drawing path through the cloud's points (untangled and smoothed, the pen
  lifting on long jumps). Fresh marks burn white-hot and cool through yellow and orange to a
  pale engraving; sparks fly off the tip and smoke drifts up. On charts and diagrams the beam
  traces lines, edges, circles and bar tops. The countdown's digits are etched and burned
  away; the end words are etched and fade. Sample `samples/engines/laser.md`.
- The blocks engine and its `stack` theme: illustrations cut into bevelled blocks, grouped
  into pieces that drop from above, bottom row first, land with a bounce and settle; the next
  slide flashes and clears the stack like a completed line. The countdown's digits fall in
  blocks and burst apart. Sample `samples/engines/blocks.md`.

### Changed

- The split-flap sample's picture slide shows a station photograph in the board's image panel
  (`samples/engines/images/station.jpg`).

## [1.10.0] - 2026-09-28

### Added

- The split-flap engine and its `departures` theme
  ([#16](https://github.com/mklab-se/mdeck/issues/16)): the slide is a departure board. All
  of its text sits on a fixed grid of 32 by 12 flaps in capitals: headings in timetable
  yellow, lists behind coloured markers, tables as timetables with numbers right-aligned,
  `@kpi` as label and value, `@progress` as bars of solid flaps, and the slide's first image
  in a panel on the board. Going to the next slide every flap turns forward through its wheel
  of characters until it shows the new one, cells starting a moment apart; `+` items flip in
  on their step. The countdown is drawn in solid flaps before the board scrambles awake, and
  the end clears flap by flap. What the board cannot show (overflowing text, code, charts,
  formulas, characters the flaps do not carry) is listed by `--check`, never typeset outside
  the board. `@engine: splitflap` puts any theme on it. Spec sections 9.1 and 9.6; sample
  `samples/engines/splitflap.md`.
- A `board` capability for engines that draw every slide themselves, text and transitions
  included (see the engine guide).

## [1.9.1] - 2026-09-28

### Added

- A guide to writing an engine, `crates/mdeck/doc/engines.md`
  ([#16](https://github.com/mklab-se/mdeck/issues/16)): the interface, what the host
  provides, the rules (finished stills, no wall-clock randomness, theme colours, keep out of
  the copy), capabilities and fallbacks, a checklist, and a minimal engine that is compiled
  and tested with the code so the guide stays current. Linked from the README and
  CONTRIBUTING.
- `scripts/engine-golden.sh` exports the sample decks with two builds and names every image
  that differs, to prove a change did not move an engine.
- Export can capture motion for looking at animations: `MDECK_EXPORT_AT=<seconds>` rehearses
  the engine from a cold start, `MDECK_EXPORT_MOMENT=3|2|1|burst|end` shows the countdown and
  the end act.

## [1.9.0] - 2026-09-28

### Added

- The LED engine and its `marquee` theme ([#16](https://github.com/mklab-se/mdeck/issues/16)):
  a wall of RGB LEDs behind every slide, unlit lenses just visible. Illustrations power on
  from their centre, each LED flickering as it strikes, and shimmer between the theme's
  colours; brightness follows the point cloud's density so a picture keeps its structure.
  Title slides get a marquee border of chasing bulbs, other slides a faint aurora away from
  the copy, a reveal sends a band of light across the wall, and charts get a peak marker
  over every bar, trails along lines and a halo around pies. The countdown is lit digit by
  digit before a white-hot ring runs out over the wall; the end words die out LED by LED.
  `@engine: led` puts any theme on it. Spec sections 9.1 and 9.6; sample
  `samples/engines/led.md`.

### Fixed

- The editorial layouts (particles and LED engines) drew all their text white: the fade-in
  replaced every run's colour. Text now keeps the theme's colours, so Ember shows its intended
  tokens (the ember numeral in the eyebrow, `*emphasis*` in ember, a quieter lead), and a
  light theme on an editorial engine is readable instead of white on white.
- On a light theme the particle field blended additively and vanished into the page; it now
  draws like ink.

## [1.8.0] - 2026-09-28

### Added

- `@engine` in the frontmatter runs a deck on another engine than its theme's, with the
  theme's colours, fonts and logo ([#16](https://github.com/mklab-se/mdeck/issues/16)).
  `--engine <name>` does the same when presenting, exporting and checking, without editing
  the deck. Precedence: `--engine`, then `@engine`, then the theme's `engine:`. Spec section
  9.6.
- `mdeck --check` has an `engine` category: content the deck's engine does not show (an
  `@illustration` or a story on the plain engine) and an unknown `@engine`. Presenting and
  exporting print one summary line when a deck has any.

### Changed

- Engines have a documented interface inside MDeck (`src/engines/`): the core hands each
  engine a `Stage` (the moment, the slide's figure and where it goes, the geometry the
  renderers drew) and the engine paints it; each engine declares its `Capabilities`. Plain
  and particles moved behind it without a visible change: exports of every Ember, theme and
  visualization sample are byte-identical before and after.

## [1.7.0] - 2026-09-28

### Added

- Per-slide logos ([#15](https://github.com/mklab-se/mdeck/issues/15)): `@logo: none` under a
  slide's heading hides the logo on that slide, and `@logo: file.svg` shows another logo there
  (or adds one to a deck without), in the deck's position, size and opacity. `@logo: none` in
  the frontmatter still hides a theme's logo for the whole deck. Spec section 9.5; sample
  `samples/themes/logo.md`.
- `mdeck --check` reports directive problems under a new `directive` category: unknown names
  with a "did you mean" (`@ilustration`), known directives that were not applied because they
  sit in a list, quote or indented block, deck directives such as `@theme` inside a slide, and
  a slide directive written twice.

### Fixed

- Slide directives written under the slide's heading are applied instead of being shown as
  text ([#12](https://github.com/mklab-se/mdeck/issues/12)). A known slide directive
  (`@layout`, `@illustration`, `@logo`) on its own line now works anywhere at the top level
  of its slide, so heading-split decks no longer need `---` or the line above the heading.
  Existing placements keep working; unknown names further down a slide stay text. The spec,
  README and samples now show directives under the heading. Sample
  `samples/features/directives.md`.
- A `##` section directly under the `#` title is no longer merged into the title slide
  ([#13](https://github.com/mklab-se/mdeck/issues/13)). An H2 right after the H1 is a
  subtitle only when it has no content of its own; with paragraphs, lists or other blocks it
  is a section and gets its own slide, so a README-shaped file gets a title slide and one slide
  per section. Decks that relied on the merge now show the section on its own slide. Sample
  `samples/layouts/sections.md`.

## [1.6.0] - 2026-09-27

### Added

- Custom themes ([#14](https://github.com/mklab-se/mdeck/issues/14)): a theme is a YAML file,
  `themes/<name>.yaml` (or `themes/<name>/theme.yaml` with its fonts and logo) next to the
  deck or in the user folder, selected with `@theme: <name>` like a built-in one. It sets
  colours, the chart and diagram series, fonts by role (a bundled face or a TTF/OTF file in
  the theme folder), sizes, line height, chart fill opacity, the syntax theme (bundled or a
  `.tmTheme` file), the opening countdown, a logo and the engine. `extends` inherits every
  unset key from another theme, so a brand theme can be ten lines. Custom themes work when
  presenting, in `Shift+T` cycling, in PNG and PDF export and in `--check`. Themes are data
  only: a deck or theme folder can never make MDeck run code, and theme file paths cannot
  leave the theme folder. Spec section 9.4; samples in `samples/themes/`.
- Engines: what a theme does beyond colours and type. `plain` draws slides on a flat
  background; `particles` is Ember's living particle field, editorial copy layouts, story
  beats and illustrations, now in any theme's own colours.
- Four seasonal built-in themes, written as theme files like every built-in: `spring` and
  `summer` (light, plain engine), `autumn` and `winter` (dark, particles engine).
- Logos (spec section 9.5): a PNG with transparency or an SVG in a corner of every slide,
  when presenting and in PNG and PDF export. A theme carries one in its `logo:` block, and any
  deck can add, replace or hide one with `@logo`, `@logo-position`, `@logo-opacity` and
  `@logo-height`, no custom theme needed.
- `mdeck theme`: `list` (every theme visible from here), `check` (errors, fallbacks, hard to
  read colours), `new` (a commented starter theme) and `preview` (a sampler deck as PNGs).
  `mdeck theme new <name> --from <folder>` turns a design system (a Claude Design export, CSS
  tokens, W3C design tokens, a Tailwind config) into a theme with AI, copying its fonts and
  logos into the theme folder. The spec documents the mapping from design-system roles to
  slide roles, so any AI agent can do the conversion too. Sample input:
  `samples/design-systems/mdeck-co`.
- `mdeck export --theme <name>` renders a deck in any theme without editing it.
- `mdeck --check` reports theme problems under a new `theme` category: an unknown name, an
  invalid theme file (unknown keys are errors, so typos surface), a font or logo that fell
  back, and text with too little contrast against its background.

### Changed

- The built-in themes are defined as embedded theme files instead of code. They render as
  before, except that Nord and Ember now use the dark themes' pen and arrow annotation colours
  (they used the light theme's).
- An unknown `@theme` name is reported when presenting, exporting and checking; it used to
  fall back to `light` silently. `mdeck config set defaults.theme` accepts user themes.
- `mdeck export` now honours the configured `defaults.theme`, as presenting always did.
- The cargo feature `particles` (on by default) holds the particles engine, the themes that
  use it and the built-in illustrations; `--no-default-features` builds without them.

## [1.5.0] - 2026-09-25

### Added

- PDF export ([#10](https://github.com/mklab-se/mdeck/issues/10)): `mdeck export deck.md
  --format pdf` writes `export/deck.pdf` with one page per slide, each exactly as presented
  (final reveal step, Ember field frozen), at PowerPoint's widescreen page size with a
  bookmark per slide and the deck's title and author as PDF metadata. `--notes` writes
  `deck-notes.pdf` instead: printable notes pages with the slide on top and its `???` speaker
  notes below, dark on white in every theme, in A4 proportions; long notes continue on the
  next page. `--slide`, `--range`, `--width`/`--height` and `--debug` (a page per reveal step)
  work with PDF too. Pages are images, so PDF text is not selectable.

### Changed

- Exports and slide thumbnails no longer show Ember's "Space to begin" hint on the title
  slide; it only appears while presenting.

## [1.4.0] - 2026-09-25

### Added

- LaTeX math ([#11](https://github.com/mklab-se/mdeck/issues/11)): `$...$` inline and
  `$$...$$` display formulas in paragraphs, lists, headings, quotes and table cells. Layout is
  done by [RaTeX](https://github.com/erweixin/RaTeX) (KaTeX syntax: fractions, roots, sums,
  integrals, matrices, cases, accents, braces, `\mathbb`, `\text`, ...) and drawn by mdeck with
  the bundled KaTeX fonts, so formulas are sharp at any resolution and in PNG export, in the
  slide's text colour in every theme. Inline formulas sit on the text baseline; display
  formulas are centred on their own line, and a formula wider than its column shrinks to fit.
  Dollar amounts stay text (`$5 and $10`, `($K) and ($M)`), and `\$` is a literal dollar.
  A formula that does not parse shows as its source and `mdeck --check` reports it under a new
  `math` category. Sample: `samples/features/math.md`.

### Changed

- The binary is about 4 MB larger (RaTeX and the KaTeX fonts).

### Fixed

- A list item whose first line is taller than usual (an inline fraction or sum) now keeps its
  bullet on the text baseline instead of at the top of the line.

## [1.3.1] - 2026-09-25

### Fixed

- Chinese, Japanese and Korean text drew as boxes in every theme
  ([#11](https://github.com/mklab-se/mdeck/issues/11)). The bundled faces carry no CJK glyphs,
  and a face that does is too large to ship, so mdeck now borrows one from the system at
  startup: PingFang, Hiragino or Arial Unicode on macOS, Microsoft YaHei, Yu Gothic or Malgun
  Gothic on Windows, Noto Sans CJK, WenQuanYi or Droid Sans Fallback on Linux. The borrowed
  faces close every font family's fallback chain, shifted onto the family's own baseline, so
  mixed lines such as "English 和 中文" sit level. `MDECK_CJK_FONT=/path/to/font.ttc` names a
  font file explicitly. When no font covers a script a deck uses, `mdeck --check` warns (new
  `fonts` category) and presenting or exporting prints the same warning.
  `samples/features/cjk.md` covers Chinese, Japanese and Korean in headings, bullets, code and
  tables.

## [1.3.0] - 2026-09-22

### Changed

- Dependencies: ailloy 2.1.2 → 2.2.0, which brings `reqwest` 0.13 in transitively (mdeck itself has
  no direct `reqwest` dependency — it uses `ureq`). No API or behavior change for mdeck's own
  surface.
- Building from source on Windows now needs [NASM](https://www.nasm.us/) and
  [CMake](https://cmake.org/) on `PATH` to compile `aws-lc-rs`'s optimized assembly routines; macOS
  and Linux need nothing extra. `cargo binstall` and Homebrew are unaffected (pre-built binaries).
  The release workflow's Windows build installs NASM via `ilammy/setup-nasm@v1`.

## [1.2.5] - 2026-09-22

### Changed

- Dependencies: ailloy 2.1.1 → 2.1.2 (lockfile refresh + CI/release-process migration only, no
  API change). eframe stays on 0.36 (0.36.2, no newer stable major). Every other manifest
  requirement was already at its latest stable line; `cargo update` refreshed transitive patch
  versions only.

## [1.2.4] - 2026-09-16

### Added

- Three built-in illustrations: `account` (a classical bank building, for finance topics),
  `lightbulb` (ideas) and `question` (a question mark), bringing the set to thirty-eight.

## [1.2.3] - 2026-09-16

### Changed

- Dependencies: ailloy 2.1.0 → 2.1.1 (lockfile only; no API change). Every manifest requirement
  was already at its latest stable line. `cargo audit bin` reports no vulnerabilities, only two
  unmaintained-crate notices (bincode 1.3 via syntect, ttf-parser 0.25 via pdf-extract).
- MSRV corrected to Rust 1.95: eframe 0.36 (adopted in 0.18.0) has required it all along, and the
  declared 1.88 was stale.
- CI and release workflows lint with `cargo clippy --workspace --all-targets`, matching the
  template; the release skill watches the workflow and verifies the outputs before declaring
  success. README gains a "Releasing" section.

## [1.2.2] - 2026-09-15

### Added

- Fourteen built-in illustrations: `man`, `woman`, `thermographer`, `presenter-up`,
  `presenter-down`, `agent-friendly`, `agent-evil`, `ai`, `camera`, `gauge`, `glasses`, `eyes`,
  `flag` and `blackhole`. The set is now thirty-five.
- Story figures: `man`, `woman`, `thermographer`, `presenter-up` and `presenter-down` are cast
  as people, sized and labelled like `person`.

## [1.2.1] - 2026-09-15

### Fixed

- Circled numbers (①②③), circled letters, check marks, arrows and geometric shapes drew as
  boxes in every theme ([#7](https://github.com/mklab-se/mdeck/issues/7)). Noto Sans Symbols
  and DejaVu Sans are now bundled as the last fallback of every font family.
- Code blocks taller than the slide were cut off in PNG export
  ([#8](https://github.com/mklab-se/mdeck/issues/8)). Code now shrinks to fit the slide, in
  height and in line width, down to 40% of the theme's code size (about 65 lines on a 16:9
  slide); only past that does the slide scroll. Long lines shrink instead of wrapping.
- Ember bullet slides: list items now step in from the copy edge with the dot in the gutter,
  nested items a step further, and items are set in the regular face in a brighter ink than
  the light paragraphs around them ([#9](https://github.com/mklab-se/mdeck/issues/9)).

## [1.2.0] - 2026-09-15

### Added

- Built-in illustration `punchcard`.
- `mdeck illustration contribute <name>` offers a deck or user illustration to the built-in set:
  it writes a `.mdpc.json` copy GitHub accepts as an attachment and opens a prefilled issue
  (name, description, prompt, braille sketch); `--no-open` prints the link instead.
- Built-in illustrations are registered by a build script from `crates/mdeck/illustrations/`,
  so adding one is a single file; the `/include-illustrations` skill does the bookkeeping.

## [1.1.1] - 2026-09-15

### Added

- **Space backdrops.** On every Ember slide but the title, the dark behind the scene is one of
  four backdrops chosen by slide number, so neighbouring slides never share one: a star field
  drifting slowly forward with parallax (a new `Forward` drift), the original dust, a galaxy of
  two spiral arms turning about the centre, and a nebula of large out-of-focus clouds.
- **Formations.** A bullet slide's item clusters take a different formation on each slide (arc,
  lazy S, ring, diagonal, scatter, column), so a run of three-bullet slides no longer looks like
  the same picture three times.

## [1.1.0] - 2026-09-15

### Added

- **Point cloud illustrations.** `@illustration: server` at the top of a slide makes the Ember
  field settle into a server beside the copy (behind it, faded, on a title slide). Twenty
  illustrations are built in: `person`, `hooded`, `box`, `orb`, `doc`, `docs`, `inbox`, `db`,
  `cloud`, `laptop`, `folder`, `mail`, `gate`, `server`, `robot`, `phone`, `globe`, `lock`,
  `gear`, `rocket`. A name resolves through the deck's `illustrations/` folder, then
  `~/.config/mdeck/illustrations/`, then the built-in set, so decks and users can shadow
  built-ins.
- **`mdeck illustration`.** `generate --name <n> --description "..."` asks the image provider
  for a sparse constellation of glowing particles and reduces it to an `.mdpc` file; `import
  <image> --name <n>` converts any image of light strokes on a dark ground; `list` shows every
  visible name and what shadows what; `show <n>` renders a preview. `--user` writes to the user
  library, `--force` overwrites.
- **The `.mdpc` format.** JSON with a name, description, prompt, aspect and up to 1500 points
  in importance order (greedy farthest-point sampling weighted toward the silhouette), so a
  prefix of any length is a spread-out sketch of the whole subject.
- `mdeck --check` warns on illustrations that do not resolve, on layouts that cannot show one,
  on slides where a story shadows the illustration, and on story casts with unknown kinds.

### Changed

- **Story casts are point clouds.** A cast member's `kind` is now an illustration name
  resolved through the same library, so `kind: server` works once `server.mdpc` exists and
  `mdeck ai story` offers the model the names the deck can resolve. The hand-drawn cast
  silhouettes are gone; the thirteen former kinds are built-in clouds under the same names.
- The field assigns mask points in order (a group of *n* particles takes the first *n*), so a
  small cast member is a sketch of its subject rather than a random speckle; countdown digits
  and the end words are shuffled once so they still fill evenly.

## [1.0.0] - 2026-09-08

MDeck 1.0: the Ember theme, a living particle field, stories, and the countdown.

### Added

- **The `ember` theme.** MKLab's brand as a theme: near-black ink, one ember accent, bundled
  Spectral, Hanken Grotesk and JetBrains Mono, an editorial copy column on the left, and a
  living field of glowing particles behind every slide, drawn additively in a dedicated OpenGL
  pass with the short trails the MKLab site has. Select it with `@theme: ember`, or cycle with
  `Shift+T`. Showcase in `samples/ember.md`; four more decks in `samples/ember/`.
- **A field that follows the content.** Without any authoring, every layout gets a scene:
  a constellation on the title, one cluster per bullet that lights with its reveal step, a
  warm mass on section dividers, a candle behind quotes, rain behind code. On charts and
  diagrams the renderers publish their geometry and the field serves it: embers rise off bar
  tops, runners follow line series, routed edges and timelines, sparks circle pies and radars,
  glints sit on scatter points, and dust keeps to the margins around images and tables. Every
  chart, diagram and label uses the theme's faces.
- **Stories.** A story script names a cast of people and props (person, hooded, laptop,
  inbox, orb, gate, …) in stage cells, flows of light between them, and beats the presenter
  releases with Space, each with a spoken line. Scripts live in a sidecar next to the deck
  (`deck.scenes.yaml` or `.yml`), written by `mdeck ai story deck.md` from an English
  ```` ```@story ```` hint on the slide, or from the copy and notes when there is none, with a
  frontmatter `@story:` for deck-wide direction. Entries are keyed by content hash and go
  stale when the slide changes (`--check` warns, `--stale` refreshes); `pinned: true` marks a
  hand-written entry that is never regenerated. `--slide`, `--range`, `--force` and
  `--dry-run` narrow or preview a run; generated scripts are rejected when labels would
  overlap. `S` writes the current slide's story from inside the presentation, `H` shows the
  beat's line. Stories play only on slides with a stage (bullet, content, quote, section) and
  their beats count as reveal steps only under Ember.
- **The countdown.** Ember and Nord decks open with 3, 2, 1. Ember forms the digits out of
  particles in the serif face, morphs between them and bursts the 1 into black before the
  first slide assembles; Nord fades plain numerals. Any key or click cancels it, starting on a
  chosen slide skips it, `@countdown: false` turns it off.
- **The end.** Under Ember the deck ends with the particles spelling THE END, letting go into a
  swirl, and bursting into black before a quiet caption.
- **`mdeck export --slide N` and `--range A-B`** export one slide or a range with the deck's
  numbering kept in the file names; with `--debug` the quick way for a person or an agent to
  check one slide's reveal steps.

### Changed

- Headings, body text, code and every visualization label take their font families from the
  theme, so themes can bundle typefaces.
- Chart fills are tuned per theme (`Theme::fill_opacity`).
- Idle particles wander a little more, on two incommensurate frequencies, so the field never
  looks still.

## [0.19.0] - 2026-09-06

### Added

- **Supply-chain transparency for release builds** — binaries are built with `cargo auditable`
  (dependency list embedded in the executable, readable with `cargo audit bin` or `syft`), and
  a per-target CycloneDX 1.5 SBOM (`mdeck-vX.Y.Z-<target>.cdx.json`) is attached to every
  GitHub release. See the README's "Software bill of materials" section.

### Changed

- **Dependencies upgraded to current versions** — Ailloy 2.0 → 2.1, `clap`/`clap_complete` 4.5 → 4.6,
  `colored` 3 → 3.1, `dirs` 6 → 7, `regex` 1.11 → 1.13, `rayon` 1.10 → 1.12, `tokio` 1 → 1.53,
  `ureq` 3 → 3.4, `zip` 8.3 → 8.6, plus `cargo update` across the lockfile.
- **GitHub Actions on Node 24** — `actions/checkout@v7`, `actions/upload-artifact@v7`,
  `actions/download-artifact@v8`, `softprops/action-gh-release@v3`.

## [0.18.0] - 2026-09-03

### Added

- **Clicker-friendly keys** — PageDown and Enter advance, PageUp and Backspace go back, so presentation remotes work out of the box. `B` is an alias for `.` (blackout). Keys pressed during a transition are queued instead of dropped.
- **Shared shortcut table** — the HUD and `mdeck spec --short` are generated from one table in `app/keys.rs`; the quick reference no longer lists a stale `D` theme key and now includes Home/End, PageUp/PageDown, blackout, and the debug overlay.
- **`mdeck <file> --check -v`** prints one line per slide (layout, block count, reveal steps, title).
- **Config defaults are honoured** — `mdeck config set defaults.theme|transition` now applies when the frontmatter does not set them (frontmatter > config > built-in). `mdeck config show` prints every key, including image and icon styles and the remembered monitor position.
- **Git graph in the gallery** — `samples/gallery.md` and `GALLERY.md` now include the `@gitgraph` visualization.
- **`BACKLOG.md`** — a roadmap of larger ideas and open decisions collected during a full review of the product.
- **Edge-case sample slides** in `samples/layouts/` and `samples/visualizations/` (wrapped titles, long quotes, overflowing lists, wide tables, long labels, star-shaped radar, thousands separators, legend overflow) for visual regression checks.

### Changed

- **Pixel-exact export** — `mdeck export` now produces images of exactly the requested size (1920x1080 by default) on every display. Previously HiDPI screens doubled the output and windows were clamped to the screen, so `--width 3840` could yield neither 3840 nor 1920 pixels. Slides larger than the display are rendered in tiles and stitched. Export also waits for images to finish loading.
- **Images load in the background** — decoding happens on a worker thread, and the next two slides' images are preloaded, so large photos no longer stall a transition.
- **Pie and donut charts** are drawn as single meshes instead of hundreds of thin polygons, removing the visible striping inside slices.
- **Charts pick round axis limits** — bar, line and stacked-bar axes now end on a round number above the data (the tallest bar no longer touches the top of the chart), and axis labels never print `-0`.
- **Word clouds fill the slide** — the layout is scaled up to use the available area instead of floating small in the centre, and no word is drawn below the readable floor (half the body size); words that cannot fit are dropped rather than shrunk to illegibility.
- **KPI cards** are sized to their content with centred text; **Gantt** rows get more room when there are few tasks; **bar charts** use gaps proportional to bar width.
- **Venn diagrams** with three sets overlap properly and place pairwise labels inside their lens instead of on top of each other; labels wrap.
- **Charts accept decorated numbers** — `$4,200`, `12%`, `1_000`, `40 users` all parse; `inf`/`nan` are rejected instead of hanging the renderer. Comma-separated series such as `1,000, 2,000` are read correctly.
- **Labels fit** — category labels, legend entries, progress-bar labels, KPI values and donut centre text shrink to a shared size and truncate with an ellipsis instead of overflowing; crowded line-chart and Gantt axis labels are thinned. Legend entries keep their percentage when truncated.
- **Radar charts** fill concave (star-shaped) series correctly; axis labels are anchored by angle so they stay clear of the rings.
- **Stacked bars** work without a `# categories:` line (numbered 1..n) and round only the top segment.
- **Gantt** dependency arrows point at the right task when an earlier task could not be resolved.
- **Overflow detection is accurate** — bullet, content and two-column slides are measured at the width they are drawn at, with wrapped list items and table rows counted, so long slides scroll instead of being cut off and short two-column slides no longer show a scroll indicator.
- **Wrapped titles and quotes** are centred on their real height; a fill-image heading band grows to fit an H1.
- **Tables** size columns to their content, shrink the font for wide tables, clamp extra cells, and get a subtle header background and zebra rows.
- **Bold text is visible** (rendered in the heading colour); links and inline code follow the theme and fade with transitions.
- **Images** upscale to their reference size so decks look the same at every resolution; `@width:300px` scales with resolution; photos above 4096 px are downscaled and mipmapped so they stay crisp in the grid overview.
- **Syntax highlighting** is cached per code block instead of recomputed every frame, and multi-line constructs (block comments) highlight correctly.
- **Transitions** use cubic easing; heading-to-body spacing is consistent across layouts.
- **README** rewritten with a sharper introduction, a sixty-second start, and complete presenting and command references.
- **Format spec** documents the nord theme, the spatial transition, and marks directives that are accepted but not yet applied (`@background`, `@footer`, `@class`, `@code-theme`, `@aspect`, per-slide `@theme`/`@transition`) as reserved instead of implemented.

### Fixed

- **`Q` no longer quits on a single stray keypress** — it needs a double tap within a second, like Esc and Ctrl+C.
- **Hot reload survives atomic saves** (vim, emacs, JetBrains) on Linux by watching the directory instead of the file's inode; reloading keeps the current slide's reveal state and can no longer panic mid overview animation.
- **Overflowed slides no longer jump to the top before a transition**; the scroll position is reset when the transition completes. Revealing an item below the fold scrolls it into view.
- **Grid overview animation** honours the grid's scroll offset for slides in lower rows.
- **Mouse release outside the window** no longer fires a stray "next slide" or commits a half-drawn stroke.
- **Spurious "time_jump" incidents** after Cmd-Tab or display sleep are gone; a jump is only recorded when an animation was actually in flight.
- **`M` gives feedback** with a toast when the window is not fullscreen instead of silently doing nothing.
- **FPS overlay** is only shown together with the HUD, never to the audience.
- **Export** pads file names to the deck size (three digits from 100 slides) and exits non-zero when a PNG cannot be written.
- **`mdeck ai create -i` and `ai style add -i`** end cleanly on EOF (Ctrl-D or piped input) instead of looping forever; long prompts with non-ASCII text (Swedish, em dashes, emoji) no longer panic; temp files use the platform temp directory (fixes Windows); an explicit `--output` path is respected instead of being replaced by an AI-suggested name; `--style` no longer applies an image style name to icons.
- **Removed a dead retry loop** that printed five bogus "Restarting presentation" messages after a display error.
- **Parser hangs and panics** — a line such as `#hashtag` or `#include <stdio.h>`, or a malformed image like `![alt] text`, made the parser loop forever; a line consisting of a single emoji or accented character panicked; a highlight range such as `{1-99999999999}` allocated unbounded memory. All fixed with regression tests.
- **CRLF files** (Windows line endings) corrupted the frontmatter and leaked the closing `---` into the first slide.
- **Separators inside code blocks** — a `---` line or three blank lines inside a fenced code block no longer splits the slide (`samples/introducing-mdeck.md` renders its "How Slides Work" example on one slide again).
- **Wrapped list items** — continuation lines now stay in their bullet instead of breaking the list into list, paragraph, list.
- **Title slides** — `# Title` directly followed by `## Subtitle` is one title slide, as the spec always said.
- **Ordinary markdown that rendered literally** — setext headings (`Title` over `===`), closing hashes (`## Head ##`), HTML comments, `_italic_`/`__bold__`, `***bold italic***`, backslash escapes, double-backtick code spans, escaped pipes in tables, image titles (`![a](x.png "Title")`) and code info strings with extra words (` ```rust title=x`) are now handled. `5 * 3 * 2` is no longer italicised. A lone `| text |` line is text instead of vanishing.
- **Frontmatter numbers** — `date: 2026` displays as `2026` instead of `Number(2026)`.
- `mdeck config set` accepts `defaults.image_style` and `defaults.icon_style`; incident log files are unique per process and second.
- `mdeck ai create` no longer exits the whole process to show help, and its tests no longer read the real stdin (which could hang in CI).

### Dependencies

- Upgraded eframe/egui 0.33 → 0.36 (glow renderer), ailloy 1.0 → 2.0, colored 2 → 3, inquire 0.7 → 0.9, base64 0.22 → 0.23, pdf-extract 0.10 → 0.12, plus a full `cargo update`. `cargo audit` reports no known vulnerabilities (previously six advisories in lopdf, quick-xml, quinn-proto, webbrowser, crossbeam-epoch). MSRV is now Rust 1.88.

## [0.17.3] - 2026-07-07

### Changed

- Upgraded to ailloy 1.0: Azure OpenAI / Microsoft Foundry requests now use
  the unified `/openai/v1/` surface, models that reject sampling parameters
  are retried automatically, and current default models (gpt-5.4-mini,
  claude-sonnet-5) replace retiring ones.

## [0.17.2] - 2026-05-18

### Fixed

- **AI config parse error** — `mdeck ai` no longer fails with `Failed to parse config from ~/.config/ailloy/config.yaml` when the config contains embedding nodes (`capabilities: [embedding]`) or a `defaults.embedding:` key. Caused by ailloy's embedding capability being absent in the 0.6 line and re-added in 0.7; configs written by newer ailloy CLIs were unreadable.

### Changed

- **Bump `ailloy` 0.6 → 0.8** — adopts embedding support re-introduced in ailloy 0.7 and the auto-detected embedding dimensions in 0.7.3. No behavior changes for mdeck (mdeck does not use embeddings); chat and image APIs are unchanged.
- **Refresh transitive dependencies** via `cargo update`.
- **CI: bump `actions/checkout@v4` → `@v5`** across CI and release workflows; release workflow upload step bumped to `actions/upload-artifact@v5`.
- **Internal: satisfy newer clippy lints** (`collapsible_match`, redundant `.max(0)` on unsigned arithmetic) surfaced by Rust 1.95. No behavior changes.

## [0.17.1] - 2026-03-29

### Changed

- **Improved AI presentation generation quality** — AI-created presentations now use varied layouts (two-column, quotes, section breaks, image splits, tables), include atmospheric images on title and product slides, apply visual rhythm (alternating dense/sparse), and follow presentation archetypes (product comparison, tutorial, pitch, etc.). The interactive chat also asks about visual mood to inform styling.
- **Refactored large source files into modules** — Split `app.rs` (2,574 lines) into `app/` module (drawing, input, helpers), `commands/create.rs` (1,606 lines) into `create/` module (prompts, interactive, extractors, opportunities), and `render/diagram/mod.rs` (3,569 lines) into submodules (types, parsing, layout, edges, icons). No behavior changes.

## [0.17.0] - 2026-03-24

### Added

- **Grid view shows final reveal step** — pressing G now shows each slide fully revealed, making it easy to identify slides by their content. Fixes #5.
- **Move fullscreen to next monitor** — press M to cycle the presentation between monitors. The last used monitor is remembered in config and used on next launch. Fixes #6.

## [0.16.0] - 2026-03-24

### Added

- **Atlassian-style `@gitgraph` visualization** — complete rewrite with new visual model: dotted gray lanes for declared branches, solid colored segments for active branches, proper S-curves for forks (bowing left) and merges (bowing right), vertical lines for simultaneous `*` events, tag boxes with arrows, and pill-shaped merge labels on S-curve midpoints. New syntax: `lane`, `branch A -> B`, `merge A -> B`, `tag`, `commit`.
- **`--debug` flag for export** — `mdeck export --debug` exports every progressive reveal step as a separate PNG (e.g., `slide-01-step-00.png`), enabling systematic visual QA at full resolution.
- **`test-visualization` skill** — reusable testing methodology for visual QA of any mdeck visualization, committed to `.claude/skills/`.

### Changed

- Branch labels now left-aligned at a consistent margin, appearing only when the branch first becomes active.
- S-curves use proper cubic bezier control points with real horizontal distance — forks connect to the target's next event, merges connect from the source's last event.
- Fork endpoint positions are stable across progressive reveal steps (computed from all events, not just visible ones).

### Fixed

- Solid lines now connect fork endpoint dots to subsequent events on the same branch.
- No more double dots at branch/merge events.
- Merge labels positioned on the S-curve midpoint instead of floating.
- Vertical lines for `*` merges draw dots on both source and target lanes.

## [0.15.0] - 2026-03-23

### Added

- **Git graph visualization** (`@gitgraph`) — new visualization type rendering precise git branching diagrams from text. Branches as horizontal lanes, commits as dots, forks and merges as S-curves. Color-coded per branch with pill-shaped labels. Supports Git Flow and any branching strategy. Progressive reveal builds the graph step by step.
- **AI-driven interactive presentation creation** — `mdeck ai create -i` now features a true AI conversation (not fixed questions) that gathers context naturally, suggests a descriptive filename, and shows a confirmation before generating.
- **Visualization opportunity logging** — when AI identifies missing visualization types, detailed GitHub-issue-ready feature requests are logged to `visualization-opportunities.md` with data models, rendering specs, ASCII mockups, and proposed syntax.

### Changed

- **AI create improvements:** true AI chat for interactive mode, animated spinners during generation, no JSON output shown to user, smart filename suggestions, approval step before generation, auto-image generation as part of the pipeline.
- **Speaker notes in AI-generated presentations** are now detailed enough for inexperienced presenters — include core message, talking points, delivery approach, background context, and transitions.
- **AI image generation policy:** only decorative/mood images are generated. Precision diagrams (flowcharts, branch histories) are never AI-generated — visualization opportunities are logged instead.
- Visualization opportunities file appends new entries instead of overwriting, with deduplication by name.

### Fixed

- Unicode arrows (→, ←, ⇒) and symbols (✓, ✗) rendering as □ — AI now avoids these characters.
- `mdeck ai create` without arguments shows help (same as `--help`).
- `mdeck ai create -i` without `--input` prompts for input instead of showing help.
- `[READY]` marker no longer visible in AI chat output.
- `mdeck ai generate` respects quiet flag and shows progress indicators.

## [0.14.0] - 2026-03-22

### Added

- **AI presentation creation** (`mdeck ai create`) — create complete presentations from any content source. Supports text prompts, PDF files, DOCX files, markdown, plain text, and piped stdin input. AI analyzes the content, identifies key points, and generates a structured presentation with speaker notes, visualizations, and image generation markers. Includes interactive mode (`-i`) for guided creation with audience/purpose context, and custom prompt support (`--prompt`) for tailored presentations.
- **Speaker notes** (`???` separator) — add presenter-only notes to any slide. Notes are parsed and stored but never rendered in the presentation. Supports full markdown formatting. Designed to help presenters understand slide intent, especially valuable in AI-generated presentations where notes explain delivery guidance and talking points.
- **Git graph visualization** (`@gitgraph`) — precise, data-driven branch diagrams showing branches as horizontal lanes with commits, forks, and merges. Supports Git Flow and any branching strategy. Progressive reveal builds the graph step by step.

### Changed

- Upgraded ailloy dependency from 0.5 to 0.6.

### Dependencies

- Added `pdf-extract` for PDF text extraction.
- Added `zip` for DOCX text extraction.

## [0.13.0] - 2026-03-20

### Added

- **AI agent skill command** (`mdeck ai skill`) — setup guide and skill file emitter for AI agents like Claude Code. `--emit` outputs a ready-to-save skill file, `--reference` outputs the full format spec and AI reference documentation at runtime.
- **Explicit `mdeck ai status` subcommand** — alias for running `mdeck ai` without arguments.
- **AI reference supplement** (`ai-reference-supplement.md`) — comprehensive CLI and AI image generation reference bundled into the binary for AI agent consumption.

## [0.12.3] - 2026-03-19

### Added

- **Interactive AI config wizard** (`mdeck ai config`) — guided setup for AI providers and models, replacing the previous "open in editor" approach. Powered by ailloy's `config-tui` module.
- **Interactive style creation** (`mdeck ai style add -i`) — AI-assisted style crafting with interactive prompts. `set` is now an alias for `add`.
- **Color-coded edge labels** — architecture diagram edge labels now use the edge's color as background, making it easy to see which label belongs to which connection.

### Changed

- Upgraded ailloy dependency from 0.4 to 0.5 with `config-tui` feature for shared AI status/enable/disable logic.
- AI status, enable, and disable commands now delegate to ailloy's `config_tui` module for consistent behavior across ailloy-based tools.
- Reorganized sample presentations from `sample-presentations/` to `samples/` with subdirectories (`visualizations/`, `layouts/`, `transitions/`).
- Edge label horizontal padding increased for better readability.

## [0.12.2] - 2026-03-11

### Added

- **Nord theme** — an arctic, blue-gray theme inspired by the polar landscape. Calm, muted, and professional. Theme cycling is now dark → light → nord → dark (press `D`).
- **Standardized visualization design tokens** — all 15 visualization types now share centralized constants for font sizes, stroke widths, corner radii, opacities, and swatch sizes, ensuring visual consistency within each theme.
- **Theme-aware trend colors** — KPI cards now use theme-appropriate green/red instead of hardcoded values, ensuring readability across all three themes.

### Changed

- Synchronized font sizes, stroke widths, corner radii, and legend styling across bar charts, stacked bars, line charts, scatter plots, pie/donut charts, radar charts, Venn diagrams, funnel charts, KPI cards, org charts, gantt charts, progress bars, timelines, and word clouds.
- Stacked bar charts now have rounded corners matching regular bar charts.
- Radar chart axis labels reduced from 0.75 to 0.65 for consistency with other visualizations.
- Timeline date/description fonts adjusted for better readability at distance.

## [0.12.1] - 2026-03-11

### Changed

- **Improved README** — rewritten "What is MDeck?" section emphasizing presentation quality, built-in visualizations, and AI-native workflow. Removed minor features from the hero section.
- **Updated gallery images** — refreshed AI-generated visuals in GALLERY.md.

## [0.12.0] - 2026-03-11

### Added

- **AI image generation (`mdeck ai generate`):** Scan a presentation for `![prompt](image-generation)` markers and diagram nodes with `icon: generate-image`, then generate all images in one command. Automatically detects orientation (horizontal for full-slide, vertical for side-panel layouts, square for icons) and rewrites the markdown with actual file paths.
- **Style management (`mdeck ai style`):** Define named image and icon styles in config, set defaults, and override per-presentation via `@image-style` / `@icon-style` frontmatter directives. Hardcoded defaults ensure good results out of the box.
- **Ad-hoc image generation (`mdeck ai generate-image`):** Generate a single image from a prompt with `--prompt`, `--style`, `--output`, and `--icon` flags.
- **Diagram prompt metadata:** Diagram nodes now support `prompt: "..."` in parenthetical metadata for AI icon generation (e.g., `Gateway (icon: generate-image, prompt: "An API gateway")`).
- **Diagram icon aspect ratio preservation:** Non-square icon images are now rendered with correct aspect ratio instead of being stretched.
- **Ungenerated image warning:** Launching a presentation with `image-generation` markers prints a terminal warning suggesting `mdeck ai generate`.
- **Enhanced `mdeck ai test`:** Image generation test now lets you choose between normal image and icon, using the configured default styles.
- **Smart heading-level slide splitting:** Files with a single H1 heading (the common "title + H2 sections" pattern) now automatically split on both H1 and H2 headings. Files with multiple H1s keep the original behavior (only H1 splits). This makes standard markdown files work as presentations without needing explicit `---` separators.
- **`@slide-level` frontmatter directive:** Explicitly control which heading level triggers slide breaks (e.g., `@slide-level: 2` means H1 and H2 both split). Overrides the automatic inference when set.
- **Visual gallery (`GALLERY.md`):** Comprehensive showcase of all layouts, diagrams, and visualizations with exported slide screenshots. Linked from README.
- **Revamped `README.md`:** Restructured with feature overview, visualization table, AI documentation, gallery preview images, and navigation links.

## [0.11.2] - 2026-03-10

### Added

- **Image-aware layouts for Bullet, Code, and Quote slides:** Adding a single image to a bullet, code, or quote slide now renders the content on the left (55%) with the image as a side panel on the right (40%), instead of falling through to the generic Content layout. The Content (fallback) layout also gains the same image-split behavior.

## [0.11.1] - 2026-03-10

### Added

- **Gantt chart visualization (`@gantt`):** Project timelines with tasks, durations, dependencies, and automatic time scaling. Supports absolute dates (`YYYY-MM-DD`), calendar days (`Nd`), working days (`Nwd`), weeks (`Nw`), months (`Nm`), and dependency chains (`after Task`, `after Task + 3d`). Timeline auto-scales between days, weeks, and months based on project span.
- **Gantt weekend shading:** Non-working days (Saturday/Sunday) are shown as subtle gray columns when the timeline is at day-level scale.
- **Gantt labels inside bars (`# labels: inside`):** Option to render task names inside their bars instead of in a left column, giving the full width to the timeline.

### Removed

- **`architecture-diagrams.md`:** Removed redundant standalone diagram documentation. All specifications are now consolidated in `mdeck-spec.md`.

## [0.11.0] - 2026-03-10

### Added

- **Ten new visualization types:** Donut chart (`@donutchart`), line chart (`@linechart`), scatter plot (`@scatter`), stacked bar (`@stackedbar`), funnel chart (`@funnel`), KPI cards (`@kpi`), progress bars (`@progress`), radar chart (`@radar`), Venn diagram (`@venn`), org chart (`@orgchart`)
- **Chart axis labels:** `# x-label:` and `# y-label:` directives for bar chart, line chart, scatter plot, and stacked bar
- **Word cloud improvements:** Elliptical cloud shape, non-linear font size contrast (`t^1.5`), rotation restricted to smallest words only
- **Format specification command:** `mdeck spec` prints the full format spec, `mdeck spec --short` prints a quick reference card
- **Per-visualization test files:** Individual test presentations for each visualization type
- **MDeck intro presentation:** `introducing-mdeck.md` — a real presentation about MDeck itself

### Changed

- **Reorganized sample presentations:** Removed redundant files, added comprehensive `test-all-visualizations.md`

## [0.10.0] - 2026-03-10

### Added

- **Four new visualization types:** Word cloud (`@wordcloud`), timeline (`@timeline`), pie chart (`@piechart`), and bar chart (`@barchart`) — all using the same code-block DSL as diagrams with `@` language tags
- **Reveal step support for visualizations:** All new visualization types support `-` (static), `+` (next step), and `*` (with previous) reveal markers for progressive disclosure
- **Bar and pie chart reveal animations:** Bars grow from zero height/width and pie slices sweep from zero angle when revealed, with smooth ease-in-out easing over 0.4 seconds
- **Mixed content slides:** Visualization layout supports heading + text blocks + visualization on the same slide
- **Bar chart orientations:** Vertical (default) and horizontal via `# orientation: horizontal` directive
- **Bar chart grid labels:** Nice-number algorithm for clean axis labels (20, 40, 60 instead of 23.3, 46.7)
- **Word cloud layout:** Dense spiral placement with area-proportional font sizing, cached for stable positions across frames
- Sample presentation `test-visualizations.md` covering all visualization types

### Changed

- **Larger fonts across all visualizations and diagrams** for better readability in presentation settings: diagram node labels (0.55x → 0.8x), diagram edge labels (0.45x → 0.65x), timeline dates (0.55x → 0.85x), timeline descriptions (0.45x → 0.7x), pie chart legend (0.45x → 0.65x), bar chart labels (0.4x → 0.6x)

## [0.9.1] - 2026-03-10

### Fixed

- **Diagram reveal ordering:** Interleaved nodes and edges now reveal in file order instead of all nodes first then all edges. This fixes diagrams like "Pipeline Growth" where `+ Source -> Build` should appear between `+ Build` and `+ Test`, not after all nodes.
- **False time-jump warnings on Linux:** Raised the time-jump detection threshold from 200ms to 2000ms. The Linux repaint keepalive (500ms) was triggering spurious "power-state gap" incidents every frame cycle, flooding the incident log.

## [0.9.0] - 2026-03-06

### Changed

- **AI integration rewrite:** Migrated from custom AI provider system (direct OpenAI/Gemini API calls via `ureq`) to the [`ailloy`](https://github.com/mklab-se/ailloy) crate for unified AI access with async support
- New AI subcommands: `ai test`, `ai enable`, `ai disable`, `ai config` replace the old `ai init`, `ai status`, `ai remove`
- `ai` (no subcommand) now shows status directly
- `ai test` supports interactive testing of both chat completion and image generation with inline terminal image display (iTerm2, Kitty)
- `ai config` opens the ailloy configuration file in your editor

### Removed

- `generate-icons` command (AI icon generation now handled via ailloy)
- Custom `AiConfig`, `AiProvider`, and `ImageGenProvider` types from config (replaced by ailloy's config system)
- `ureq` and `serde_json` dependencies (replaced by `ailloy` and `tokio`)

## [0.8.1] - 2026-03-04

### Added

- **Power-state resilience (Linux):** More aggressive repaint keepalive (500ms vs 4s) prevents GPU context instability when presenting on battery or while screen-sharing
- **Time-jump detection:** Frame deltas >200ms are detected and all in-flight animation timestamps (transitions, overview zoom, pen strokes, arrows, toasts, reveal steps) are shifted forward so animations resume smoothly instead of snapping to completion
- Time-jump incidents are logged to the incident log for diagnostics
- Incident log header now includes `XDG_CURRENT_DESKTOP` and `DESKTOP_SESSION` environment variables for better desktop environment diagnostics

## [0.8.0] - 2026-03-03

### Added

- **Incident logging:** Lightweight `IncidentLog` module records all recovered and fatal errors (display errors, file watcher errors, reload failures) to `~/.config/mdeck/logs/incident-YYYY-MM-DD-HHMMSS.log` for diagnostics
- Log files are created lazily — no file is written during normal operation
- At session end, if any incidents occurred, the log file path is printed to stderr
- Log header includes version, presentation file, OS/arch, and display-related environment variables (DISPLAY, WAYLAND_DISPLAY, XDG_SESSION_TYPE) for Linux debugging
- File watcher errors are now logged (previously silently ignored)
- File reload errors are now logged in addition to the existing toast notification

## [0.7.1] - 2026-03-02

### Removed

- Debug frame profiling that wrote `/tmp/mdeck-profile.log` on every exit

## [0.7.0] - 2026-03-02

### Added

- **"The End" slide:** Virtual end slide shown when navigating past the last slide, with centered "The End" title and MDeck logo/attribution in the bottom-right corner
- **Blackout mode:** Press `.` (period) to toggle screen to solid black for audience attention; press `.` again to resume
- **`--check` CLI flag:** Validate presentations without launching the GUI — reports diagram routing warnings with exit code 1 on problems, 0 on success
- Structured warning system (`CheckReport`, `CheckWarning`, `CheckCategory`) for extensible presentation validation
- Diagram route warnings collected once during background precache instead of per-frame `eprintln!` spam
- Brief one-liner warning summary printed to stderr in GUI mode when routing issues are found

### Changed

- Replaced `precache_all_diagrams_background` with `precache_all_diagrams_with_report` that returns a `CheckReport` via channel
- Removed noisy per-frame `eprintln!("ROUTE WARNING: ...")` from `draw_diagram_sized`; fallback drawing logic preserved
- HUD (press H) now shows `.` blackout shortcut

## [0.6.0] - 2026-03-02

### Added

- Background pre-caching of diagram routes: all diagrams are pre-computed on a background thread at startup and after file reload, making transitions to diagram slides instant
- Diagram scale-to-fit: large diagrams (3+ rows) that overflow the slide area are automatically scaled down to fit
- `# scale:` directive in diagram blocks: `fit` (default), `scroll`, or a numeric factor (e.g. `0.7`)

### Changed

- Diagram route cache upgraded from thread-local `RefCell` to global `Mutex`, enabling cross-thread cache sharing between background precache and render threads
- Removed per-frame adjacent-slide precaching in favor of whole-presentation background precaching

## [0.5.0] - 2026-03-02

### Added

- Live file watching: presentation auto-reloads when the markdown file is saved, with slide position preservation
- Configurable routing cost weights (`routing.length`, `routing.turn`, `routing.lane_change`, `routing.crossing`) in config
- Crossing-aware edge routing: A* search now penalizes routes that cross existing edges
- Crossing detection at junctions and empty cell centers for perpendicular and pass-through segments
- Turn-conflict detection for lanes adjacent to turning routes
- 37 new unit tests for crossing avoidance, routing weights, and file watcher

### Changed

- Edge routing engine uses weighted cost function (length + turns + lane changes + crossings) instead of simple path length

## [0.4.0] - 2026-03-02

### Added

- Diagram rendering overhaul: proper grid layout, auto-layout, much larger nodes
- Diagram parser: skip comment lines, parse `icon:` and `pos:` metadata, detect all 5 arrow types (`->`, `<-`, `<->`, `--`, `-->`)
- Geometric fallback icons for 15+ node types (user, server, database, cloud, lock, api, cache, etc.)
- AI-generated diagram icons via `mdeck generate-icons <file.md>` command
- Icon images loaded from `media/diagram-icons/{name}.png` when available
- OpenAI DALL-E 3 and Google Gemini Imagen API support for icon generation
- `image_generation` config section for API provider and key
- Orthogonal edge routing engine with A* pathfinding and lane allocation
- Edge rendering with rounded corners, proper arrowheads, and lane-aligned connections
- Dashed lines for `--` and `-->` arrow types
- Edge label pills with semi-transparent backgrounds
- Diagram debug overlay (press R) showing routing details
- Gallery layout for image-heavy slides
- 244 unit tests covering parsing, routing, and rendering

### Changed

- Diagram nodes now render as rounded rectangles with icon + label (was: tiny pills in a single row)
- Diagram layout uses grid positioning or auto-layout (was: single horizontal row)

### Fixed

- Arrow port offsets now derived from lane assignments, eliminating diagonal "lane-switching" segments
- Entry face computation corrected with `.opposite()` to match routing direction
- Edge labels moved to 20% along polyline to prevent overlap on opposing edges (A->B and B->A)
- Debug overlay route format now shows lane labels between coordinates per routing spec

## [0.3.0] - 2026-02-28

### Changed

- Renamed project from `presemd` to `mdeck` across the entire codebase
- Binary name changed from `presemd` to `mdeck`
- Config directory changed from `~/.config/presemd/` to `~/.config/mdeck/`
- Crate name changed from `presemd` to `mdeck` on crates.io
- Homebrew formula changed from `presemd` to `mdeck`
- Repository URL changed from `mklab-se/presemd` to `mklab-se/mdeck`

## [0.2.0] - 2026-02-28

### Added

- Full CLI with clap: `mdeck <file.md>` to launch presentations
- Subcommands: `ai init/status/remove`, `config show/set`, `completion`, `export`, `spec`, `version`
- Shell completions for bash, zsh, fish, and powershell (static and dynamic)
- AI provider configuration with auto-detection (Claude, Codex, Copilot, Ollama)
- YAML-based configuration at `~/.config/mdeck/config.yaml`
- Configurable defaults: theme, transition, aspect ratio, start mode
- Global flags: `--verbose`, `--quiet`, `--no-color`, `--windowed`
- `--slide <N>` flag to start on a specific slide (1-indexed)
- `--overview` flag to start in grid overview mode
- `defaults.start_mode` config setting (`first`, `overview`, or slide number)
- Grid overview: mouse hover highlight, click to select slide, mouse wheel scrolling
- Grid overview: fade gradients at screen edges when content overflows
- Grid overview: presentation title shown instead of generic "Slide Overview"
- Freehand pen annotations (left-drag) with outline/glow effect
- Arrow annotations (right-drag) with large arrowhead and drop shadow
- Distinct colors: pen strokes in cyan/blue, arrows in yellow-orange/red
- ESC clears drawings on current slide before double-tap-to-quit
- Mouse input: left-click forward, right-click backward, scroll wheel for content
- PNG export via `mdeck export` with configurable resolution
- Format specification via `mdeck spec` (full and `--short` quick reference)
- Sample presentations for testing (`samples/`)

## [0.1.1] - 2026-02-28

### Added

- Initial implementation with hardcoded demo slides
- Slide transitions: fade and horizontal slide with easing
- Keyboard navigation with arrow keys
- FPS overlay
- `--version` flag support
