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
| 2 | Workspace, SDK, registries, paint; engines v2 (D10-D13), laser removed, line merged, D24/D26 fixed | in progress: `mdeck-sdk` crate exists (not wired), laser removed, line merged, D24/D26 fixed, visual grammar done |
| 3 | Designs and themes v2 (D8, D9), default theme, layout defects | todo |
| 4 | Presenter view, per-slide transitions, slide jump, `--theme`, `export --at`; generated assets and `mdeck ai` (D14) | done |
| 5 | Extensibility tooling: `mdeck build`, packs, external visual programs, `mdeck sdk new/preview`, SDK docs and tutorials | todo |
| 6 | Documentation, README, gallery, format reference, CHANGELOG, release workflow (publish `mdeck-sdk`), v2.0.0 | todo |

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
  `Slide::illustration`, `picture: none` and `picture-prompt` fill `Slide::art`, and a slide that
  names a point cloud without a `picture-prompt` takes no generated art (`render::art::wants_art`);
  phase 2 makes the picture one source (D13). `art-world` is `PresentationMeta::art_world`.
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

## Deferrals

Any requirement deferred to 2.x is listed here and in the release notes.

- **Phase 2:** wiring `mdeck-sdk` into mdeck: registries instead of `EngineKind`/`Chart`/
  `Layout` (D11), built-in engines on `paint` only, `mdeck::run(registry)`; engine settings in
  the theme's `engine:` block (`surface:` and `particles`/`heat`/`art` move there); image options
  in the settings grammar and their validation (LANG-12, VIZ-11); `picture:` resolving to
  artworks and image paths as one source (PIC-02, D13).
- **Phase 3:** designs and themes v2 (D8, D9) as planned; `design:` still maps onto the v1
  `Layout` enum.
- **Phase 6:** `docs/*.md`, the README and the AI supplement were converted mechanically to the
  v2 syntax but not rewritten; the gallery and tutorial screenshots were not regenerated; the
  format reference still describes v1 layouts and engines outside the sections phase 1 changed.
