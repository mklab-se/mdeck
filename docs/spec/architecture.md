# Architecture

The architecture decisions behind mdeck 2, as built. The requirements are in the area documents
of this folder; this file records how the code meets them. What is not done yet is in the
"Deferred to 2.x" list in the [README](README.md#deferred-to-2x).

## D1. One language table

`crates/mdeck/src/language/` holds one table of every setting (name, scope `Deck`, `Slide` or
`Both`, kind, values, default, summary) and the list of mdeck fences. The parser validates
against it, `--check` reports against it, and the settings sections of the format reference
(`<!-- generated: settings -->` in `mdeck-spec.md`) and `mdeck spec --short` are generated from
it (LANG-04). The same module knows the v1 form of every construct, so `--check` can name its v2
replacement (LANG-07), and suggests the nearest name for a misspelt one.

## D2. Deck settings

Frontmatter is plain YAML with plain keys (`theme: ember`). A v1 `@key` is not honoured;
`--check` reports it with its v2 form. Unknown keys and invalid values are reported.

## D3. Slide settings

An HTML comment whose first non-blank line is `key: value` with a known slide-setting key is a
settings comment; every line in it is `key: value`. It applies to the slide it is in. Other
comments are ordinary comments; `--check` warns when one looks like a misspelt setting. Visible
`@key: value` lines are plain text, and `--check` reports them as v1 syntax.

## D4. Splitting

ATX and setext headings at or above the slide level split slides. `---` with blank lines on both
sides is the explicit break. Three blank lines do not split. No setting ever moves between
slides. The slide level is inferred (0 or 1 H1 gives level 2, otherwise 1) or set with
`slide-level`.

## D5. Notes

```` ```@notes ```` fenced blocks hold markdown notes, anywhere in the slide, joined in order.
The model keeps the notes source; the presenter view and PDF notes pages parse and render it as
markdown (`app::presenter::notes_blocks`).

## D6. Steps

`+` items are steps; `-` and `*` are static. Children of a `+` item reveal with it. Steps are
numbered across the slide in reading order, over lists and visuals (`parser::steps`); `Deck`
renumbers with thermal step counts from the image library. Hidden content reserves its space.
The deck or slide setting `reveal: none` turns steps off.

## D7. Visual kinds

Visual fences are matched by exact tag against the visual registry, with no aliases. Built-in
tags: `@bar`, `@line`, `@pie`, `@donut`, `@scatter`, `@stackedbar`, `@funnel`, `@radar`,
`@progress`, `@kpi`, `@wordcloud`, `@timeline`, `@gantt`, `@architecture`, `@orgchart`,
`@gitgraph`, `@flower`, `@artifactflow`, `@venn`, `@thermal`. `@notes` is the one other mdeck
fence. One grammar (`render/visualizations/grammar.rs`) reads every visual: settings are
`key: value` lines before the first item, items are list lines with `(key: value)` attributes,
relations are `A -> B: label`, and `#` starts a comment (VIZ-03).

## D8. Designs and arrangements

Designs: `title`, `section`, `statement`, `points`, `split`, `media`, `gallery`, `quote`, `code`,
`visual`, `columns`, `table`, `content`. One recogniser, a documented ordered table
(`parser::design::RULES`, printed in the format reference and by `--check -v`), assigns them; a
slide may choose one with `design:`. One renderer (`render::designs`) draws every design,
parameterised entirely by an `Arrangement` value, and never drops content. The design sets
`standard` and `editorial` are YAML files embedded in the binary (`crates/mdeck/designs/*.yaml`);
a pack may add more. A theme picks one with `designs:` and overrides any arrangement key with
`arrangements:` (`all:` applies to every design). Editorial's eyebrow, column, pillow and stagger
are arrangement values, so a design set never falls back to another one. A `designs:` name that no
YAML set has may name a code design set an extension registered (`mdeck_sdk::design::DesignSet`,
EXT-05), which then draws every slide through `render::board`, the path a board engine's set
takes; a name that is neither falls back to `standard` with a warning.

## D9. Themes

A theme file has a look (colours, fonts, sizes, a spacing scale, code theme), `designs`,
`arrangements`, `transition`, `countdown: on|off`, chrome (logo, page) and an `engine:` block
whose keys the engine validates (`engine: name` is shorthand). Engine-specific sections such as
the particle tints, the heat palette, the line engine's `surface` and the art style live in that
block. The default theme is `dark`: plain, bright foreground, `designs: standard`,
`engine: plain`, fade transitions, no countdown. A theme without `extends` inherits `dark`.
Variants (`autumn` and `winter` of `ember`, `spring` and `summer` of `light`) carry
`variant-of:` so lists and Shift+T tier them.

## D10. Workspace and SDK

- **`crates/mdeck-sdk`** holds the public, stable interfaces: the `Engine`/`EngineDef`, `Visual`,
  `DesignSet` and `Transition` traits; the content model (`Slide`, `Block`, `Inline`); theme
  tokens, the `Stage`, point clouds and published geometry (`Hint`); the `Registry`; problems
  for `--check`; the drawing interface `paint` (its own `Color`, `Pos2`, `Vec2`, `Rect`,
  `Stroke`, `Mesh`, `Painter`, text and textures); and `testing` for headless extension tests.
  It depends on egui privately and exposes none of its types (EXT-24, EXT-25). It is versioned in
  lockstep with mdeck and published to crates.io next to it.
- **`crates/mdeck`** is the library and the binary: parser, designs, themes, app, export,
  commands, and the built-in engines, visuals and design sets.
- **Raw egui** is reachable through the SDK's `unstable-egui` feature, outside the promise
  (EXT-26). Built-in engines use `paint` only (a test keeps egui out of every engine module);
  built-in visuals still draw with egui through a bridge (deferred).

## D11. Registries

Engines, visual kinds and embedded themes and point clouds are name-keyed registries filled at
startup. `mdeck::builtins(&mut Registry)` registers the built-ins, `mdeck::run(registry)` (and
`run_with_args`) is the library entry point, and `src/main.rs` is the two of them.
`crate::registry::get()` is the registry in use, so an extension's engines, visuals, themes and
point clouds are found exactly like the built-ins (EXT-06), and its transitions and code design
sets by name next to the built-in ones (`render::transition::TransitionKind::Extension`,
`Theme::code_design_set`; EXT-05). `mdeck extensions list` shows each
registration and where it came from.

## D12. Engines

Ten engines, each a cargo feature on by default:

- `plain` (always built), `particles`, `led`, `splitflap`, `blocks`, `thermal`;
- `line` (with `surface: sheet|slate`, used by the `blueprint` and `chalkboard` themes);
- the art engines `sketch`, `watercolour` and `darkroom`.

Capabilities: `picture`, `countdown`, `ending`, `board`, `transition` and `medium`. What used to
be one-engine capabilities are generic hooks on the `Engine` trait (`copy_hold`,
`numbers_slides`). The engine host (`engines/host/`) is the core's half: it converts the parsed
slide to the SDK content model, the theme to tokens, published geometry to `Hint`s, resolves the
picture, and builds the `Stage`, `Frame` and `Painter` each frame. Board engines draw every slide
through `render::board` with their own `DesignSet`. Laser, stories and the `editorial`,
`numbers_slides`, `cold_open` and `heat_trace` capabilities are gone.

## D13. Pictures

The `picture:` setting resolves, in order:

1. a current generated artwork for the slide (on art engines);
2. a point cloud of that name;
3. an image path.

`picture: none` keeps the slide's stage empty. `picture-prompt` (slide) and `art-world` (deck)
feed only `mdeck ai`.

## D14. Generated assets

One folder next to the deck (`<deck-stem>.assets/`) and one manifest (`manifest.yaml`) for every
generated asset (images, icons, artworks, external visual output), with current, stale and
pinned states (`crate::assets`). `![prompt](generate:)` and `(icon: generate:, prompt: "...")`
placeholders stay in the source. Everything that calls an AI is under `mdeck ai`; presenting and
export only read the folder.

## D15. Custom builds

An extension crate exposes `pub fn register(r: &mut mdeck_sdk::Registry)`. `mdeck build --with
<path|crate[@version]|git source>...` generates a cargo project that depends on `mdeck` and the extensions, calls
`mdeck::run` with the built-ins plus each `register`, builds it in release mode and installs the
binary where `--out` says. `mdeck sdk new` scaffolds an extension crate from templates embedded
in the SDK, and `mdeck sdk preview` exports a preview deck with a chosen engine and theme.

## D16. Packs and external visual programs

- **Packs** are data extensions that need no compiler: a folder (or zip) with an
  `mdeck-pack.yaml` manifest and any of `themes/`, `designs/`, `point-clouds/`, `styles/` and
  `fonts/`. They install into the user config folder's `packs/` or a deck's own `packs/`, and
  lookups take them after the deck's and the user's own folders and before the built-ins
  (`extensions::packs`).
- **External visual programs** map a fence tag to a command in the user config. mdeck sends the
  block as JSON on stdin and caches the PNG it prints in `<stem>.assets/visuals/`, keyed by the
  command and the request, so a program runs at most once per block, when the deck opens
  (`extensions::external`).
