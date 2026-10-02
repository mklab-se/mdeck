# v2 implementation plan

How v2 is built from v1.19. The requirements are in the other documents of this folder; this file
records the architecture decisions taken to implement them, the order of work and the status. It
is updated at the end of every phase.

Work happens directly on `main`. Every commit keeps `cargo fmt --all -- --check && cargo clippy
--workspace -- -D warnings && cargo test --workspace` green.

## Architecture decisions

### D1. One language table

`crates/mdeck/src/language/` holds one table of every setting (`Setting { name, scope, kind,
values, default, summary, since }`, scope `Deck`, `Slide` or `Both`) and the list of mdeck fences.
The parser validates against it, `--check` reports against it, and the format reference sections
and `mdeck spec --short` that list settings are generated from it (LANG-04). The AI skill is
produced from the format reference, so it follows automatically.

### D2. Deck settings

Frontmatter is parsed as plain YAML with plain keys (`theme: ember`). A v1 `@key` is not honoured;
`--check` reports it with its v2 form (LANG-07). Unknown keys and invalid values are reported.

### D3. Slide settings

An HTML comment whose first non-blank line is `key: value` with a known slide-setting key is a
settings comment; every line in it is `key: value`. It applies to the slide it is in. Other
comments are ordinary comments; `--check` warns when one looks like a misspelt setting. Visible
`@key: value` lines are plain text, and `--check` reports them as v1 syntax.

### D4. Splitting

ATX and setext headings at or above the slide level split slides. `---` with blank lines on both
sides is the explicit break. Three blank lines no longer split. No setting ever moves between
slides. Slide level inference is unchanged (MD-01, MD-03).

### D5. Notes

```` ```@notes ```` fenced blocks hold markdown notes, anywhere in the slide, joined in order. The
model keeps the notes source; the presenter view and PDF notes parse and render it as markdown.

### D6. Steps

`+` items are steps; `-` and `*` are static. Children of a `+` item reveal with it. Steps are
numbered across the slide in reading order, over lists and visuals. Hidden content reserves its
space. Deck or slide setting `reveal: none` turns steps off.

### D7. Visual kinds

Visual fences are matched by exact tag against the visual registry. Tags: `@bar`, `@line`, `@pie`,
`@donut`, `@scatter`, `@stackedbar`, `@funnel`, `@radar`, `@progress`, `@kpi`, `@wordcloud`,
`@timeline`, `@gantt`, `@architecture`, `@orgchart`, `@gitgraph`, `@flower`, `@artifactflow`,
`@venn`, `@thermal`. `@notes` is the one other mdeck fence. Inside fences: settings are
`key: value` lines before the first item, items are list lines with `(key: value)` attributes,
relations are `A -> B: label`, and `#` starts a comment (VIZ-03).

### D8. Designs and arrangements

Designs: `title`, `section`, `statement`, `points`, `split`, `media`, `gallery`, `quote`, `code`,
`visual`, `columns`, `table`, `content`. One recogniser (a documented ordered table) assigns them.
Each design has one Rust renderer that is parameterised entirely by an `Arrangement` value. The
design sets `standard` and `editorial` are YAML files embedded in the binary
(`crates/mdeck/designs/*.yaml`); a theme picks one with `designs:` and overrides any arrangement
key with `arrangements:`. Editorial's eyebrow, column, pillow and stagger are arrangement values,
so a design set never falls back to another one. No renderer drops content.

### D9. Themes

The v2 theme file:

- adds `designs`, `arrangements`, `transition`, a spacing scale and `countdown: on|off`;
- replaces `engine: name` with an `engine:` block whose keys the engine validates;
- moves `particles`, `heat` and `art` into that block.

The default theme is `dark`: plain, bright foreground, `designs: standard`, `engine: plain`,
`transition: fade`, no countdown. A theme without `extends` inherits `dark`. Variants (`autumn`,
`winter`, `spring`, `summer`) carry `variant-of:` so lists and Shift+T tier them.

### D10. Workspace and SDK

- **`crates/mdeck-sdk`** holds the public, stable interfaces:
  - `Engine` and `EngineDef`, `Visual`, `DesignSet`, `Transition`;
  - the content model (`Slide`, `Block`, `Inline`);
  - theme tokens, the `Stage`, point clouds and published geometry;
  - the `Registry`;
  - the drawing interface `paint` (own `Color`, `Pos2`, `Vec2`, `Rect`, `Stroke`, `Mesh`,
    `Painter`, text and textures);
  - `testing`.

  It depends on egui privately and exposes none of its types (EXT-24, EXT-25).
- **`crates/mdeck`** is the library and the binary: parser, designs, themes, app, export,
  commands, and the built-in engines, visuals and design sets. The built-ins register themselves
  through the same `Registry` an extension uses (EXT-06).
- **Raw egui** is reachable through the `unstable-egui` feature, outside the promise (EXT-26).
  Built-in visuals may use it in 2.0, with that deferral noted; built-in engines use `paint` only.

### D11. Registries

`EngineKind`, `Chart` and `Layout` enums give way to name-keyed registries filled at startup.
`mdeck::run(registry)` is the library entry point; the `mdeck` binary calls it with the built-ins.

### D12. Engines

The ten engines are:

- `plain`, `particles`, `led`, `splitflap`, `blocks`, `thermal`;
- `line` (with `surface: sheet|slate`);
- `sketch`, `watercolour`, `darkroom`.

Capabilities: `picture`, `countdown`, `ending`, `board`, `transition`, `medium`. Laser, stories,
`editorial`, `numbers_slides`, `cold_open` and `heat_trace` (as capabilities) are removed.

### D13. Pictures

The `picture:` setting resolves, in order:

1. a current generated artwork for the slide;
2. a point cloud of that name;
3. an image path.

`picture: none` keeps the slide empty. `picture-prompt` (slide) and `art-world` (deck) feed only
`mdeck ai`.

### D14. Generated assets

One folder (`<deck-stem>.assets/`) and one manifest (`manifest.yaml`) for every generated asset,
with current, stale and pinned states. `![prompt](generate:)` placeholders stay in the source.

### D15. Custom builds

An extension crate exposes `pub fn register(r: &mut mdeck_sdk::Registry)`. `mdeck build --with
<path|crate[@version]>...` generates a cargo project that depends on `mdeck` and the extensions,
calls `mdeck::run` with the built-ins plus each `register`, builds it in release mode and installs
the binary where `--out` says.

## Phases

| # | Phase | Status |
|---|---|---|
| 1 | Language and content model: D1-D7, story removal from the format, samples converted to v2 syntax | done |
| 2 | Workspace, SDK, registries, paint; engines v2 (D10-D13), laser removed, line merged, D24/D26 fixed | done (see Phase 2b notes and deferrals) |
| 3 | Designs and themes v2 (D8, D9), default theme, layout defects | done (see Phase 3 notes and deferrals) |
| 4 | Presenter view, per-slide transitions, slide jump, `--theme`, `export --at`; generated assets and `mdeck ai` (D14) | done |
| 5 | Extensibility tooling: `mdeck build`, packs, external visual programs, `mdeck sdk new/preview`, SDK docs and tutorials | done; packs read every EXT-09 folder (`themes/`, `designs/`, `point-clouds/`, `styles/`, `fonts/`). Deferred: there is no `mdeck sdk preview` (an extension is tried with `mdeck build`); built-in visuals, transitions and the two built-in design sets are not yet registered through the SDK (see CHANGELOG "Deferred to 2.x"); the point cloud folders are still named `illustrations/` |
| 6 | Documentation, README, gallery, format reference, CHANGELOG, release workflow (publish `mdeck-sdk`), v2.0.0 | in progress: release workflow, packaging and CI done (see Release readiness); v2 docs merged; product gaps (packs, `mdeck point-cloud`, CLI help) closed; v2.0.0 not yet tagged |

## Phase 1 notes

What later phases build on:

- **`crate::language`** is the language table (D1): `SETTINGS` (`SettingDef`), `FENCES`,
  `invalid_value`, `suggestion`, `v1_replacement`, `v1_fence`, `settings_reference` (fills the
  `<!-- generated: settings -->` marker in `mdeck-spec.md` via `commands::spec::full_reference`)
  and `settings_card` (`spec --short`). Phase 6 generates the rest of the format reference from it.
- **Parser model:** `Slide::settings` (as written, with file lines), `Slide::problems`,
  `Slide::reveal`, `Slide::steps`; `ListItem::step` and `checked`; `Block::List { start }`,
  `Block::Table { align }`, `Block::BlockQuote { blocks }`, `Block::Callout`, and `step_base` on
  `Block::Chart` / `Block::Diagram` (draw visuals with `BlockCx::after_steps(step_base)`).
  `parser::steps::number` numbers a slide; `Deck` renumbers with thermal counts from the library.
- **Interim mappings to replace:** `design:` maps onto the v1 `Layout` enum
  (`parser::layout::design_layout`; phase 3 replaces it with designs). `picture: <name>` fills
  `Slide::illustration`, `picture: none` and `picture-prompt` fill `Slide::art`; a slide that
  names a point cloud still takes generated art (`render::art::wants_art`), which comes first
  (D13). `art-world` is `PresentationMeta::art_world`.
- **Per-slide transitions:** a slide's `transition` sets how it is entered (and left going back)
  in the window (`app::look::slide_transition`); `transition: zoom` works with `zoom-to`. Phase 4
  adds the rest of RUN's transition work.
- **`*` is static everywhere**, visuals included (`VizReveal::WithPrev` and
  `DiagramReveal::WithPrev` are gone); the git graph's straight `*` merge line went with it.
- **`--check`:** new categories `settings`, `visual` and `content`; `--check -v` prints the
  settings that apply per slide.

## Integration notes (phases 2 and 4, parallel branches)

Five branches were merged onto phase 1 (October 2026):

- **Engines:** laser and its `etch` theme are removed; blueprint and chalkboard are one `line`
  engine with the interim top-level theme key `surface: sheet|slate` (`theme::Surface`), which
  moves into the theme's `engine:` block with D9. `Theme::numbers_slides()` replaces the engine
  capability. D24 (hints leaking to the next slide) and D26 (thermal end still) are fixed.
- **SDK:** `crates/mdeck-sdk` holds the public interfaces (D10: `paint::Painter` and its own
  geometry types, tokens, content, stage, `Engine`, `Visual`, `DesignSet`, `Transition`,
  `Registry`, `testing`). It is a workspace member with its own tests but **not yet used by
  mdeck**: no registry is filled, no built-in goes through it, `mdeck::run` does not exist (D11).
- **Presenter and run:** presenter view (`V`, `--presenter`, `app/presenter/`, `app/cockpit.rs`;
  markdown notes via `presenter::notes_blocks`, also used by PDF notes), slide jump, `--theme`
  for presenting, transition precedence deck > theme `transition:` > config > `fade`, per-slide
  `transition` (`app::look::slide_transition`; `zoom` with `zoom-to`), `countdown: on|off` in
  themes and decks, `Deck::draw_chrome` in window and export, `export --at/--moment` and the
  hidden `export --presenter-view`.
- **Generated assets (D14):** `crate::assets` (`<stem>.assets/manifest.yaml`, placeholders
  `![prompt](generate:)` and `(icon: generate:, prompt: "...")`), everything AI under `mdeck ai`,
  check category `assets`. Artworks key on `art-world` and the slide source.
- **Visual grammar (VIZ-03, VIZ-04, VIZ-08):** `render/visualizations/grammar.rs` reads every
  visual; `*` is static there too, and `count_viz_steps` counts `+` items through the grammar so
  the parser's slide numbering and the visual agree. Tags are exact v2 tags only.

## Phase 2b notes (SDK wiring)

What later phases build on:

- **Library and registry.** `mdeck::builtins(&mut Registry)` and `mdeck::run(registry)` (and
  `run_with_args`) are the entry points; `src/main.rs` is the two of them. `crate::registry::get()`
  is the registry in use (the installed one, else the built-ins). Engines (`EngineId`, a handle on
  a registered `EngineDef`), visuals (`parser::Chart` is a fence tag), embedded themes
  (`theme::lookup`) and embedded point clouds (`render::illustration`) are looked up there, so an
  extension's are used exactly like the built-ins. `crates/mdeck/tests/extension_engine.rs`
  registers `examples/engine-ambience` next to the built-ins and exports a slide with it and its
  `dusk` theme (EXT-14 end to end); `mdeck build`'s ignored test builds a real custom mdeck.
- **The engine host** (`engines/host/`) is the core's half: it converts the parsed slide to the
  SDK content model (`host::convert`),
  the theme to `Tokens` and font roles, published hints to SDK `Hint`s (a heading galley becomes
  `Hint::Text`), resolves the picture (D13: artwork on art engines, else the named point cloud,
  else the named image file, the last two only where `render::design_has_stage`), and builds
  `Stage`, `Frame` and `Painter` each frame. Every file under `engines/` outside `host/` uses only
  `mdeck_sdk` and engine helpers (a test checks; no egui).
- **Seams with phase 3:** `theme::engine_settings(theme)` hands the engine its `engine:` block
  minus `theme::CORE_ENGINE_KEYS` (the particle tints and art keys the core reads);
  `theme::validate::engine_keys` is the engine def's `settings` plus those core keys.
  `render::design_has_stage` decides where clouds and image pictures show. The SDK slide's
  `design` is the v2 design name, and the particle scenes and the board key on it, so no engine
  reads `parser::Layout` any more (the core still does in a few places). The core learns what an
  engine needs from a runtime made with the theme's settings in `Theme::set_engine` (`copy_hold`,
  `numbers_slides`), so changing the engine goes through `set_engine`.
- **New SDK surface** used by the core: `Engine::copy_hold` and `Engine::numbers_slides`
  (generic hooks replacing the `cold_open` and `numbers_slides` capabilities), `DesignCx::image`,
  `DesignCx::visual`, `DesignCx::engine_live`, `DesignCx::deck_title`, `DesignCx::count` (what a
  board's design set needs), `Registry::registrations` (origins for `mdeck extensions list`),
  `content::ListItem::step` and `content::Block::Visual::step_base`, `mdeck_sdk::templates`
  (the scaffolds, embedded in the SDK so `mdeck` packages on its own). Engine ports added what
  they needed to `mdeck_sdk::paint` (see the commits).
- **Boards** draw through `render::board::render` with the engine's `DesignSet`; `--check` takes
  its `unsupported` messages.
- **External visual programs** are wired: `extensions::external::configure` (from `run`) makes
  their tags fence tags, `Deck::open` runs the missing ones and records the images in the image
  cache, the block renderer draws them (or the source when there is none).
- **Exports after the port** (every `samples/engines`, `samples/ember`, `samples/themes` deck,
  the launch showcase, `layouts/bullet`, `visualizations/all`, plus `--at`, countdown and end
  stills; 340 images against the 1.19 binary): 275 identical, 65 differ, all invisibly. Art
  engines: at most one colour level on a few edge pixels of textured quads (the SDK mesh splits a
  rect along the other diagonal). LED countdown: two pixels by one level. Thermal (3 slides,
  under 0.06%): contour band edges move about a pixel, because a heading now reaches the engine
  as one `Hint::Text` per glyph instead of a galley. Split-flap (9 images, 0.02%): the labels
  under the board, drawn per character without letter spacing (the SDK text API has none).
  After merging phase 3, the same decks exported with main (phase 3) and with this branch
  differ in exactly the same 65 images and nothing else.
- **SDK additions from the ports:** `Painter::glyph_ink` (ink pixels of a text at their layout
  positions, for the thermal cold opening), `Painter::glyph_mesh` (glyph quads on the font atlas,
  for the split flaps; `Texture` can refer to the atlas). Note: the SDK's `Vec2::rot90` turns the
  other way from egui's (`engines::art::across` compensates).
- **Image options** use the settings grammar (`@width: 60%`, `@height`, `@fill`); `@fit`, `@left`,
  `@right`, `@center` and unknown options are `content` problems in `--check`.
## Phase 3 notes

What later phases build on:

- **Designs.** `parser::design` is the catalogue (`Design`) and the one recognition table
  (`RULES`, thresholds as named constants; `rules_reference` fills
  `<!-- generated: designs -->` in the format reference, a test keeps `docs/writing-slides.md`
  in sync). `Slide::design` and `Slide::recognition` replace the layout as the renderer's input;
  `--check -v` prints the design and the rule, `--check` (category `settings`) reports a chosen
  design with a rest or without its core block (which then falls back to `content`).
  `parser::design_layout` is gone.
- **`Slide::layout` stays as a derived view** (`parser::Layout::of(design, blocks)`); after the
  phase 2b merge no engine reads it (they key on the SDK slide's design name). Designs without an editorial
  stage map to quiet kinds (table, columns, wide content: `Visualization`; split: `Image`).
  Engines should move to `slide.design` and `render::design_stage`, then the enum goes.
- **Arrangements** (`theme/arrangement.rs`) are typed and `deny_unknown_fields`: a set file is a
  `base` plus per-design keys, theme `arrangements:` overrides merge as YAML values key by key
  (through `extends` in `ThemeFile::over`, then over the set), `all:` applies to every design.
  `Theme::arrangement(design)` is what the renderer reads; `theme::spacing` holds the spacing
  scale and `radius`.
- **One renderer** (`render::designs`): `parts::of` (roles), `plan` (copy `Stack`, plate,
  footer, columns; `below` or `beside`), `layout` (fit: code to 40%, then prose to 80%),
  `measure`, `revealed_bottom` and `render` share the plan. `render/layouts` and the editorial
  renderer in `render/ember` are gone; `render::ember` keeps the chrome and the compatibility
  questions engines ask (`is_title` is now `design == Title`, `handles` is the editorial stage).
- **Seams with phase 2b, final:** `theme::uses_editorial(theme)` (the theme's design set),
  `theme::engine_settings(theme)` (the theme's `engine:` block, `Theme::engine_block`), and
  `render::design_has_stage(slide, theme)` / `render::design_stage` (the arrangement's stage; a
  `content` slide with a wide block gives it up).
- **Themes v2:** `engine:` block (string shorthand kept), `variant-of:`, default theme `dark`,
  pack theme folders in the lookup, `theme/schema.rs` key table for the starter, inert-key and
  missing-page warnings, contrast over every rendered text pair, `theme preview` one slide per
  design, chart grids on `rule`/`muted`.

## Release readiness

- **Packaging.** `cargo package -p mdeck-sdk -p mdeck` packages both crates and verifies mdeck
  against the packaged SDK through a temporary local registry (mdeck-sdk is not on crates.io
  yet; the release workflow publishes it first). CI runs it as the `Package` job and the release
  skill as a pre-flight check. The SDK templates store their manifests as `Cargo.toml.tmpl`.
  Every `include_str!`/`include_bytes!` the build reads lives inside its crate; the ones that
  reach outside (`docs/`, `samples/`, `examples/`) are in `#[cfg(test)]` code only, so the
  published crate builds but its unit tests need the repository.
- **Version.** mdeck and mdeck-sdk take `version.workspace = true`; the workspace's exact
  `mdeck-sdk` pin is the one other place the number lives. The release skill bumps both, and a
  missed pin fails the build at once (cargo cannot resolve `=OLD` against the new path crate).
- **CI.** fmt, clippy (`--all-targets`), `cargo test --workspace` (examples included; GitHub
  sets `CI`, so a missing golden image fails instead of being recorded), the engine feature
  matrix (none, then each of particles, led, splitflap, blocks, line, sketch, watercolour,
  darkroom, thermal alone) and the package check.
- **`--check` over `samples/`.** Every deck checks clean except these deliberate warnings:
  `continents.md` and `layouts/image-generation.md` (images and icons not generated yet: the
  generated assets stay out of git), `features/math.md` (a broken formula that shows its source),
  `features/thermal.md` (an author-supplied spot value and a colour image shown as it is),
  `themes/custom-theme.md` and `themes/minimal-theme.md` (contrast advice on sample themes) and
  `design-systems/mdeck-co/SKILL.md` (not a deck: the input for `mdeck theme new --from`).

## Deferrals

Any requirement deferred to 2.x is listed here and in the release notes.

- **Phase 2 (to 2.x):**
  - Built-in visuals still draw with egui: each is a registered `Visual` whose `draw` reaches the
    slide's `VizCtx` through a bridge (`render::visualizations::builtin`), using the SDK's
    `unstable-egui` feature. Moving them onto `mdeck_sdk::paint` is a 2.x task. The `@thermal`
    and `@architecture` visuals are registered (tags, check, steps) but drawn by their own
    renderers, which need the deck's images.
  - Transitions (`slide`, `fade`, `spatial`, `none`, `zoom`) are still the app's own and not
    registered through the SDK `Transition` trait; `mdeck extensions list` lists them as built in.
  - Design sets: the standard and editorial designs are phase 3's; only a board's code design set
    goes through the SDK today.
  - The SDK content model is converted from the parser's each time a slide is first shown
    (quotes and callouts flatten to one run of text); the parser does not produce it directly.
  - A generated artwork shows on any slide the art pipeline resolves one for (as in 1.x), not
    only where `design_has_stage` says; point clouds and image pictures follow the seam.
  - Split-flap: the SDK content model has no list `start`, so numbered lists on the board count
    from 1; a panel image that is still loading shows the empty panel colour; the board drawn
    without a live engine (grid thumbnails, overview) has no golden test yet
    (`mdeck_sdk::testing::Headless::render_design` is documented but not implemented).
  - `Hint::Text` carries no letter spacing or wrapping, so the host publishes a heading one glyph
    at a time; the glyph's own font section is not reachable through egui, the first section's
    font is used.
- **Phase 3:**
  - DES-14 (a code extension providing a whole design set) beyond the SDK trait: only a board
    engine's design set goes through the SDK (`render::board`); registering design sets by name
    is phase 5 work.
  - THM-11's last sentence: `--check -v` does not yet say that the theme's engine settings are
    ignored when the deck or `--engine` runs another engine.
  - THM-12 is partial: `theme check` warns when an engine that declares `needs.page` runs
    without a `page:`; nothing else is declared yet.
  - Inline images in a copy column are boxed at a fixed height (60% of the column width, at most
    400 px) so measurement does not depend on the decoded image; a portrait image is letterboxed
    in that box.
  - Italic display text (editorial quotes) is egui's synthetic slant; no italic faces are
    bundled.
- **Release readiness (to 2.x):**
  - `--moment` exports the moment once per slide in range (each over that slide's backdrop);
    pass `--slide 1` for one image.
  - The presenter view (`--presenter`) opens a window and is not covered by an export path; it
    was not exercised in the release-readiness pass.
  - mdeck's tests that read `docs/`, `samples/` or `examples/` run from the repository only,
    not from the unpacked crate (the SDK's tests run from either).
- **Phase 6:** `docs/*.md`, the README and the AI supplement were converted mechanically to the
  v2 syntax but not rewritten; the gallery and tutorial screenshots were not regenerated; the
  format reference still describes v1 layouts and engines outside the sections phase 1 changed.
