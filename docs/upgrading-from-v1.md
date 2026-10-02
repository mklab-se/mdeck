<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Upgrading from v1

mdeck 2 breaks cleanly with the v1 syntax: v1 constructs are not honoured, and there is no
migration tool. A plain markdown file presents exactly as before; only mdeck's own additions
changed, and every change makes a deck a cleaner markdown document (valid YAML frontmatter,
settings invisible on GitHub).

Two things make the move quick:

1. **`mdeck talk.md --check` names the v2 form of every v1 construct it finds**, on its line:

   ```text
   slide 4 (line 31): [settings] "@layout: quote" is v1 syntax; write <!-- design: quote -->
   slide 6 (line 52): [settings] `???` is v1 syntax; put speaker notes in a ```@notes block
   slide 7 (line 60): [visual] `@barchart` is v1 syntax; write ```@bar
   ```

2. **This page is complete enough to hand to an AI.** Give your coding agent this file and the
   format reference (`mdeck spec`), and ask it to convert the deck (a prompt is at the
   [end](#converting-with-an-ai)).

## Deck settings (frontmatter)

v1 keys started with `@`, which made the frontmatter invalid YAML. v2 keys are plain YAML keys,
and `--check` reports unknown keys and invalid values.

| v1 | v2 |
|---|---|
| `@theme: ember` | `theme: ember` |
| `@engine: led` | `engine: led` |
| `@transition: slide` | `transition: slide` |
| `@slide-level: 2` | `slide-level: 2` |
| `@countdown: true` / `false` | `countdown: on` / `off` (both directions now work) |
| `@footer: text` | `footer: text` (now also drawn in export) |
| `@logo`, `@logo-position`, `@logo-opacity`, `@logo-height` | `logo`, `logo-position`, `logo-opacity`, `logo-height` |
| `@background`, `@background-opacity` | `background`, `background-opacity` |
| `@palette: iron` | `palette: iron` |
| `@art: <the deck's world>` | `art-world: <the deck's world>` |
| `@image-style`, `@icon-style` | `image-style`, `icon-style` |
| `@story` | removed (stories are gone) |
| `@aspect`, `@code-theme`, `date` | removed (they never had an effect) |
| `title`, `author` | unchanged |

New in v2: `reveal: none` (no steps), `requires: [pack, extension]`.

## Slide settings

v1 slide directives were visible `@key: value` lines. In v2 they are `key: value` lines in an HTML
comment anywhere in the slide, invisible on GitHub, and they apply to the slide they are in, never
the next one. Several settings can share one comment, one per line.

| v1 | v2 |
|---|---|
| `@layout: bullet` | `<!-- design: points -->` |
| `@layout: two-column` | `<!-- design: columns -->` |
| `@layout: image` | `<!-- design: media -->` (or `split` for an image beside text) |
| `@layout: diagram`, `@layout: visualization` | `<!-- design: visual -->` |
| `@layout: title`, `section`, `quote`, `code`, `gallery`, `content` | `<!-- design: title -->` and so on, same names |
| `@illustration: rocket` | `<!-- picture: rocket -->` |
| `@art: none` | `<!-- picture: none -->` |
| `@art: <a scene>` (on a slide) | `<!-- picture-prompt: <a scene> -->` |
| `@zoom: Hotspot` | `<!-- zoom-to: Hotspot -->` |
| `@logo: none`, `@logo: partner.svg` | `<!-- logo: none -->`, `<!-- logo: partner.svg -->` |
| `@background: ...`, `@background-opacity: ...` | `<!-- background: ... -->`, `<!-- background-opacity: ... -->` |
| `@thermal-window: 25..90 °C` | `<!-- thermal-window: 25..90 °C -->` |
| `@class: ...` | removed |

New in v2: a slide may set its own `transition` and `reveal`. Deck-only settings (`theme`,
`engine`, `slide-level`, ...) written in a slide are reported. Most slides no longer need a
`design` at all: v2 recognises thirteen designs, including the new `statement` (a heading and a
sentence) and `table`, and `mdeck --check -v` prints which one each slide got. Remove a v1
`@layout` when the recognised design is the same.

Example:

```markdown
## Our platform
@layout: bullet
@illustration: rocket
```

becomes

```markdown
## Our platform
<!--
design: points
picture: rocket
-->
```

## Slides, steps and notes

| v1 | v2 |
|---|---|
| Three blank lines split a slide | Blank lines never split; use a heading or `---` (with blank lines around it) |
| Setext headings (`Title` over `===`) did not split | They split like `#` headings |
| `???` followed by notes, to the end of the slide or the next heading | A ```` ```@notes ```` fenced block of markdown, anywhere in the slide |
| `*` items revealed together with the previous `+` item | `*` is an ordinary static bullet, like `-`; nest items under a `+` item to reveal them with it |
| Steps numbered per list | Steps count across the whole slide in reading order, visuals included |

Notes example:

```markdown
## Key decision

- Microservices for team autonomy

???
Stress team autonomy, not scale.
```

becomes

````markdown
## Key decision

- Microservices for team autonomy

```@notes
Stress team autonomy, not scale.
```
````

## Visuals

Fence tags match exactly, with one name per kind:

| v1 | v2 |
|---|---|
| ```` ```@barchart ```` | ```` ```@bar ```` |
| ```` ```@linechart ```` | ```` ```@line ```` |
| ```` ```@piechart ```` | ```` ```@pie ```` |
| ```` ```@donutchart ```` | ```` ```@donut ```` |
| ```` ```@story ````, ```` ```@scene ```` | removed |

All other tags are unchanged. Inside every visual there is now one grammar:

| v1 | v2 |
|---|---|
| `# x-label: Year` (a setting written as a comment) | `x-label: Year`, before the first item (`#` now starts a comment) |
| Item lines without a list marker | Every item is a list line, `- ` (shown) or `+ ` (a step) |
| `@orgchart` with `(parent: X)` | `- Manager -> Report` |
| `@kpi` with `(trend: up, change: +12%)` | `- Revenue: $4.2M (trend: +12%)` |
| `@venn` with `Set: items` | `- Name (size: N)` sets and `- A & B: label` overlaps |
| `@flower` `centre` | `center` |

## Images and generated assets

| v1 | v2 |
|---|---|
| `![Photo @width:80%](p.jpg)` | `![Photo @width: 80%](p.jpg)` (the space is optional, so v1 still works) |
| `@height:VAL`, `@fill` | `@height: VAL`, `@fill` |
| `@fit`, `@left`, `@right`, `@center` | removed: placement belongs to the slide's design |
| `![prompt](image-generation)` | `![prompt](generate:)` |
| `(icon: generate-image, prompt: "...")` | `(icon: generate:, prompt: "...")` |
| `<deck>.art.yaml` and `art/` | `<deck>.assets/` with one `manifest.yaml` for every generated asset |

v2 never rewrites the deck: placeholders stay in the source and are matched to their files
through the manifest. Run `mdeck ai talk.md` once to regenerate what is missing in the new layout.

## Themes

| v1 | v2 |
|---|---|
| Default theme `light`, default transition `slide` | Default theme `dark` (plain, still), default transition `fade` |
| `engine: blueprint` | `engine: { name: line, surface: sheet }` |
| `engine: chalkboard` | `engine: { name: line, surface: slate }` |
| Top-level `particles:`, `heat:`, `art:`, `surface:` | Inside the `engine:` block (theme building says where each key went) |
| `countdown: none`, `plain`, `burst` | `countdown: off` or `on` (the engine decides how it looks) |
| The editorial look came with the engine | `designs: editorial` or `standard`, chosen by the theme |
| Theme `etch` and the `laser` engine | removed |

## Commands

| v1 | v2 |
|---|---|
| `mdeck ai create` | `mdeck ai deck` |
| `mdeck ai art` | `mdeck ai pictures` |
| `mdeck ai generate` | `mdeck ai images` and `mdeck ai icons` (or `mdeck ai talk.md` for everything) |
| `mdeck ai generate-image` | `mdeck ai images --prompt "..." --output file.png` |
| `mdeck ai story` | removed |
| `mdeck illustration generate` | `mdeck ai point-cloud` |
| `mdeck illustration import`, `list`, `show`, `contribute` | `mdeck point-cloud import`, `list`, `show`, `contribute` |
| `--check` category `illustration` | `point-cloud` |
| `mdeck theme new --from <dir>` | `mdeck ai theme <name> --from <dir>` |
| `MDECK_EXPORT_AT`, `MDECK_EXPORT_MOMENT` | `mdeck export --at <seconds>`, `--moment <moment>` |
| cargo features `blueprint`, `chalkboard` | `line` |

## Converting with an AI

Paste this into your coding agent, with this page and the format reference at hand:

```text
Convert the mdeck deck <file.md> from the v1 format to mdeck 2.

Use docs/upgrading-from-v1.md (the v1 to v2 map) and the output of `mdeck spec` (the v2 format
reference) as the only sources of truth. Change only mdeck syntax; keep every word of content.
- Frontmatter: drop the `@` from every key; map keys as the table says.
- Slide directives (`@key: value` lines): move them into one HTML comment per slide,
  `<!-- key: value -->`, mapping names and values as the tables say. Drop a `design` that the
  slide would be recognised as anyway.
- `???` notes: move the notes into a ```@notes fenced block in the same slide.
- Fence tags: @barchart -> @bar, @linechart -> @line, @piechart -> @pie, @donutchart -> @donut;
  inside visuals, `# key: value` settings become `key: value`.
- Image placeholders: (image-generation) -> (generate:), icon: generate-image -> icon: generate:.
- `*` list items that were meant to appear with the previous `+` item: nest them under it.
- Slides separated only by blank lines: add `---` between them.
Then run `mdeck <file.md> --check` and fix every warning until it reports no issues.
```

Then look at it: `mdeck talk.md --check -v` lists each slide's design, and `mdeck export talk.md`
shows every slide.
