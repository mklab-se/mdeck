# Sample decks

Every deck here presents with `mdeck <file>` and exports with `mdeck export <file>`. They double as
test decks while working on mdeck: each one shows a feature in isolation, so a change can be
checked on the deck that exercises it.

## Start here

| Deck | What it shows |
|---|---|
| [introducing-mdeck.md](introducing-mdeck.md) | The tour: designs, steps, notes, charts, diagrams, math, pictures, presenting and export, on `ember` |
| [tutorial/talk.md](tutorial/talk.md) | The deck the [tutorial](../docs/tutorial.md) builds |
| [showcase/launch.md](showcase/launch.md) | The slides behind the README pictures; try it with `--theme` and `--engine` |
| [gallery.md](gallery.md) | The deck behind the [gallery](../GALLERY.md) |

## Designs

[`layouts/`](layouts) has one deck per slide design: `title`, `section`, `statement`, `points`,
`split`, `media`, `gallery`, `quote`, `code`, `visual`, `columns`, `table` and `content`, plus
[all-designs.md](layouts/all-designs.md) with every design in one deck and
[image-generation.md](layouts/image-generation.md) for `![prompt](generate:)` placeholders.
[features/designs.md](features/designs.md) shows every design with a deck-local theme that
overrides arrangements.

## Visuals

[`visualizations/`](visualizations) has one deck per chart and diagram kind, and
[visualizations/all.md](visualizations/all.md) has every kind. Thermal images are in
[features/thermal.md](features/thermal.md).

## Features

| Deck | What it shows |
|---|---|
| [features/settings.md](features/settings.md) | Slide settings in HTML comments |
| [features/notes.md](features/notes.md) | Speaker notes in ```` ```@notes ```` blocks |
| [features/math.md](features/math.md) | LaTeX math, inline and display |
| [features/backgrounds.md](features/backgrounds.md) | Background images |
| [features/symbols.md](features/symbols.md) and [features/cjk.md](features/cjk.md) | Symbols and Chinese, Japanese and Korean text |
| [`transitions/`](transitions) | One deck per transition: `fade`, `slide`, `spatial`, `none` |

## Themes and engines

| Deck | What it shows |
|---|---|
| [ember.md](ember.md) and [`ember/`](ember) | The `ember` theme on the particles engine: plain text, images, visuals, point clouds |
| [`engines/`](engines) | One deck per engine and its showcase theme: `led`, `splitflap`, `blocks`, `thermal`, `blueprint` and `chalkboard` (the line engine), `sketch`, `watercolour`, `darkroom` |
| [themes/seasons.md](themes/seasons.md) | The seasonal variants |
| [themes/minimal-theme.md](themes/minimal-theme.md) | A custom theme in a few lines of YAML |
| [themes/custom-theme.md](themes/custom-theme.md) | A theme converted from the design system in [`design-systems/`](design-systems) |
| [themes/logo.md](themes/logo.md) | A logo on every slide, without a custom theme |

## Longer decks

[poker-night.md](poker-night.md), [saloon-workshop.md](saloon-workshop.md) and
[continents.md](continents.md) are complete talks. `continents.md` and
`layouts/image-generation.md` use image placeholders, so `mdeck --check` reports them as not
generated until `mdeck ai images` has run; `features/math.md` and `features/thermal.md` contain
deliberate mistakes that `--check` reports.
