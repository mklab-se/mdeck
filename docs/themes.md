<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Themes

A theme is everything about how a deck looks: colours, the chart palette,
fonts, sizes, a logo, the countdown and its engine. Pick a built-in one with a
line of frontmatter, or write your own in a few lines of YAML.

## Built-in themes and transitions

The built-in themes come in two tiers. `mdeck theme list` and `Shift+T` show
the themes first, then the variants.

**Themes:**

| Theme | Engine | Designs | What it is |
|---|---|---|---|
| **dark** | plain | standard | The default: plain dark, a bright foreground, fades, no countdown |
| **light** | plain | standard | Dark text on white |
| **nord** | plain | standard | The Nord palette |
| **ember** | particles | editorial | Monochrome luxury on near-black with a living particle field |
| **thermal** | thermal | editorial | A thermal instrument: headings form in heat (see [Thermal](engines.md#thermal-the-deck-through-a-thermal-camera)) |
| **marquee** | led | editorial | A wall of LEDs |
| **departures** | splitflap | board | A departure board |
| **stack** | blocks | editorial | Falling blocks |
| **blueprint** | line | editorial | Generated line art inked on a drafting sheet |
| **chalkboard** | line | editorial | Generated line art in chalk on a slate |
| **sketchbook** | sketch | editorial | Generated drawings, pencilled in |
| **watercolour** | watercolour | editorial | Generated paintings that bloom |
| **darkroom** | darkroom | editorial | Generated photographs that develop |

**Variants** (recolourings of a theme): **spring** and **summer** of light,
**autumn** and **winter** of ember.

A deck without a `theme` gets `dark` (or your configured default). Transitions
are **fade** (the default), **slide**, **spatial**, and **none**. Set them in
the frontmatter (a theme can set its own with `transition:`) or cycle them live
with `Shift+T` and `T`. Every theme draws symbols (①, ✓, →) from bundled
fallback faces, and Chinese, Japanese and Korean from a font on your system
(`mdeck --check` tells you if none was found):

```yaml
---
theme: winter
transition: spatial
---
```

<p align="center">
  <img src="../media/gallery/theme-spring.png" width="45%">&nbsp;&nbsp;
  <img src="../media/gallery/theme-summer.png" width="45%">
</p>
<p align="center">
  <img src="../media/gallery/theme-autumn.png" width="45%">&nbsp;&nbsp;
  <img src="../media/gallery/theme-winter.png" width="45%">
</p>

## Your own theme

**Your own themes are YAML files**, and the built-in ones are written the same
way. Put `themes/acme.yaml` next to a deck (or in your user themes folder:
`~/Library/Application Support/mdeck/themes/` on macOS, `~/.config/mdeck/themes/`
on Linux) and set `theme: acme`; everything you leave out comes from the theme it
extends, and a theme without `extends` extends `dark`. Themes are looked up in
the deck's `themes/`, then your user folder, then installed packs, then the
built-ins:

```yaml
name: Acme
extends: light
colors:
  background: "#fbfaf7"
  text: "#2b2d42"
  heading: "#14213d"
  accent: "#e85d04"
  series: ["#e85d04", "#14213d", "#2a9d8f", "#e9c46a"]
fonts:
  display: spectral-light      # a bundled face, or a .ttf/.otf in the theme folder
engine: plain                  # or particles, for Ember's living field in your colours
```

A theme sets colours, the chart palette, fonts by role, sizes, spacing and
corner radius, the syntax theme, the countdown, the default transition, the
design set and its arrangements, the engine and its settings, and a **logo**.
Themes are data only, so a deck can never run code. Custom themes work
everywhere a built-in one does: presenting, `Shift+T`, PNG and PDF export, and
`--check`. A recolouring of another theme says so with `variant-of: <theme>`,
and is then listed with the variants.

**Spacing and corners.** `spacing:` is the scale the slide designs use for
their gaps (`xs`, `sm`, `md`, `lg`, `xl`, px on a 1920x1080 slide; 8, 16, 24,
40 and 64 by default), and `radius:` the corner radius of code blocks, tables
and callouts (8 by default). Change them to make every slide airier or tighter
in one place:

```yaml
spacing: { md: 32, lg: 56 }
radius: 4
```

## The engine and its settings

The engine is what a theme does beyond colours and type: the particle field,
the LED wall, the departure board, the falling blocks, or the generated
pictures of a blueprint or a chalkboard (both the line engine), a sketchbook, a
watercolour or a darkroom. See [Engines](engines.md). `engine: plain` names
one; a block names it and gives its settings:

```yaml
engine:
  name: thermal
  palette: iron          # iron, white-hot, black-hot, rainbow, arctic, lava
  drift: false           # embers drifting on ordinary slides
```

| Setting | Read by | What it does |
|---|---|---|
| `light`, `cool` | particles, led, blocks, line, sketch | the brightest tint and a cool tint the engine draws in |
| `palette`, `drift` | thermal | the heat field's palette, embers drifting |
| `surface` | line | `sheet` (a draughtsman's blue sheet) or `slate` (chalk) |
| `kind`, `style`, `references` | line, sketch, watercolour, darkroom | the house style of generated pictures (below) |

A theme that extends another and names the same engine (or none) inherits its
settings key by key; one that names a different engine starts from its own
settings only. When a deck or `--engine` runs another engine, the theme's
settings for its own engine are ignored. In v1 these lived in top-level
`particles:`, `heat:`, `art:` and `surface:` sections; a theme that still has
them gets an error saying where each key went.

## Pages and art

A `page:` block lays every slide on a sheet with a surface around it: paper
on a desk, a blueprint on a drafting table. It works on every engine.

```yaml
page:
  surface: "#0a1b33"     # around the sheet
  margin: 26             # px on a 1920x1080 slide
  shadow: 0.7            # 0 to 1
  grain: 0.25            # paper fibre, 0 to 1
  radius: 2
```

On an art engine, the engine block sets the house style of the generated
pictures, so a brand's illustration style is theme data like its colours:

```yaml
engine:
  name: sketch
  kind: line             # line (ink the engine draws) or tonal (a finished picture)
  style: "detailed graphite and ink, cross-hatching, old craft meets modern technology"
  references: [refs/teacup.jpg, refs/street.jpg]   # your own style swatches
```

The sketch, watercolour and line engines draw on the page; `mdeck theme check`
warns when a theme picks one of them without a `page:`.

## Logos

**Logos.** A PNG (with transparency) or SVG in a corner of every slide, in
presenting and in export. A theme carries its brand's logo (`logo:` with
`file`, `position`, `height` and `opacity`), and any deck can add or replace
one without a custom theme:

```yaml
---
theme: dark
logo: brand/logo-white.svg
logo-position: bottom-right   # default top-right
logo-opacity: 40%             # default 60%
---
```

`logo: none` in the frontmatter hides a theme's logo for the whole deck. Under a
slide's heading, `logo: none` hides it on that slide and `logo: partner.svg`
shows another logo there.

## Background images

An image behind the slides, on any theme, without a custom theme. Set it once
in the frontmatter, and override it on a slide under its heading:

```markdown
---
background: images/texture.jpg   # relative to the deck
background-opacity: 25%          # default 30%
---

# Welcome
<!--
background: images/stage.jpg
background-opacity: 60%
-->

# The code
<!-- background: none -->
```

The image covers the slide (scaled, centred, cropped, never stretched) and
sits on the theme's background colour under everything else, so a low
opacity keeps text readable on light and dark themes. A slide that sets only
`background-opacity` shows the deck's image with that opacity. PNG, JPEG,
WebP and SVG work; `mdeck --check` reports files that are missing or
unreadable. See `samples/features/backgrounds.md` and spec section 9.8.

## From a design system

**From a design system.** If your brand already lives in a design system (a
Claude Design export, CSS tokens, a Tailwind config, W3C design tokens),
`mdeck ai theme acme --from path/to/design-system` reads its rules and tokens,
copies its fonts and logos into the theme folder, and writes the theme with AI.
Then look at it and adjust:

```bash
mdeck theme new acme                       # a commented starter theme in themes/
mdeck ai theme acme --from ./brand         # convert a design system (AI)
mdeck theme check acme                     # errors, fallbacks, contrast, keys that do nothing
mdeck theme preview acme -o /tmp/acme      # one slide per design, as PNGs
mdeck theme list                           # every theme visible from here, themes then variants
mdeck export talk.md --theme acme          # any deck in any theme
```

`mdeck theme new` writes every key, commented, generated from the theme
format itself. `mdeck theme check` reports errors and fallbacks, contrast
below WCAG AA for every text colour the theme draws (body text, headings,
bold, links, muted captions and eyebrows, code, each slide design's roles at
the size and opacity they are drawn), and keys that do nothing: an engine
setting the theme's engine does not read, or `fonts.lead` when no design in the
theme's design set uses the lead face.

Section 9.4 of the spec (`mdeck spec`) documents every key and the mapping
from design-system roles to slide roles, so an AI agent can do the conversion
too. `samples/themes/` has a converted design system and a hand-written theme.
