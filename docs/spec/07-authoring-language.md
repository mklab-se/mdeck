# 07. The authoring language

Everything mdeck adds on top of markdown. The language is small on purpose:

| Syntax | Where | Example |
|---|---|---|
| `key: value` in the frontmatter | deck settings | `theme: ember` |
| `key: value` lines in an HTML comment | slide settings | `<!-- design: quote -->` |
| `@tag` fence info strings | visuals and notes | ```` ```@bar ````, ```` ```@notes ```` |
| `@option` in image alt text | image options | `![Team @width: 60%](team.png)` |
| `+` list marker | steps | `+ appears on click` |
| `+++` line | column break | |
| `generate:` link target | AI placeholders | `![a rocket at dawn](generate:)` |

Every addition is invisible on a standard markdown renderer (frontmatter, comments, alt text) or
reads as meaningful markdown there (a fenced block of chart data, a bullet, a notes block). The
choices follow Marp's precedent: plain YAML keys for the deck, HTML comments for slides.

## Requirements

### Principles

- **LANG-01** MUST `implemented`: The language has exactly **one way to write a setting**:
  `key: value`, with the same key names wherever they appear.
  - A key in the frontmatter is the **deck value**.
  - The same key in a slide's settings is **that slide's value**, overriding the deck's.
  - Keys that only make sense deck-wide (for example `theme`, `engine`, `slide-level`) are
    reported by `--check` when written in a slide, and slide-only keys when written in the
    frontmatter.
- **LANG-02** MUST `implemented`: Every mdeck addition is either **invisible** on a standard
  markdown renderer (frontmatter, HTML comments, alt text) or **reads as meaningful markdown**
  there (a fenced code block of chart data, a `+` bullet, a notes block). Nothing renders as stray
  mdeck syntax.
- **LANG-03** MUST `implemented`: Every name and value in the language is validated. Unknown keys,
  unknown values, unknown visual tags and unknown image options are reported by `--check`, with a
  "did you mean" suggestion when one is close. Nothing the author writes is silently ignored
  (VIS-13).
- **LANG-04** MUST `deferred to 2.x`: The language is one table in the code (`crate::language`: name,
  scope, kind, allowed values, default, summary, since-version, and the list of mdeck fences). The
  parser reads settings by these names, `--check` validates against it, and the settings
  sections of the format reference (`mdeck spec`), `mdeck spec --short` and the AI skill (made
  from the format reference) are generated from it, so they cannot drift. *Deferred:* editor
  completion generated from the table does not exist yet; the rest is built.
- **LANG-05** MUST `implemented`: Reserved-but-unimplemented names are not accepted. A name exists
  when it works. v1's `aspect`, `code-theme`, `class` and `date` are reported as removed.

### Deck settings (frontmatter)

- **LANG-06** MUST `implemented`: The frontmatter is plain, valid YAML with plain top-level keys
  (Marp and Slidev style), not nested under an `mdeck:` map:

  ```yaml
  ---
  title: Launching Orbit
  author: Kristofer Liljeblad
  theme: ember
  transition: slide
  reveal: steps
  ---
  ```

  A v1 `@key` is not honoured; `--check` reports it with its plain form.
- **LANG-07** MUST `implemented`: v2.0 breaks cleanly with the v1 syntax. There is no migration
  tool, and v2 does not honour v1 syntax. Instead:
  - the v2 format is completely specified in the format reference (`mdeck spec`), which is
    precise enough for a user to hand to their AI harness with "convert this deck to the v2
    format";
  - the format reference (section "Upgrading from v1"), `docs/upgrading-from-v1.md` and the
    release notes map every v1 construct to its v2 form;
  - `--check` recognises v1 constructs (`@` frontmatter keys, visible `@key:` slide lines, `???`
    notes, renamed fence tags, image options and an `illustrations/` folder) and names the v2 form
    in its message, for example
    `"@layout: quote" is v1 syntax; write <!-- design: quote -->`. This is a validation message
    (LANG-03), not a converter. The removed `@story` and `@scene` fences are reported as unknown
    fences that show as code.

### Slide settings

- **LANG-08** MUST `implemented`: Slide settings are written in an **HTML comment** inside the
  slide. The comment holds one or more `key: value` lines:

  ```markdown
  ## Why now

  <!-- design: statement -->

  The market moved, and we are the only ones ready.
  ```

  ```markdown
  ## Our platform
  <!--
  design: split
  picture: rocket
  -->

  - Fast
  - Private
  ```

  - A comment is a settings comment when its first line is `key: value` with a known setting
    key; every line in it is then a setting. There is no explicit marker: the validation in
    LANG-03 catches typos.
  - Any other comment is an ordinary comment. `--check` warns when it looks like a setting with a
    misspelt key.
- **LANG-09** MUST `implemented`: A settings comment applies to the slide it is in, wherever in the
  slide it is written (MD-07). It never moves to another slide.
- **LANG-10** MUST `implemented`: Visible `@key: value` lines are not settings; they are plain
  text. `--check` points at each one and names its v2 form (LANG-07).

### Slide settings

| Key | Scope | Values |
|---|---|---|
| `design` | slide | a design name ([03](03-slide-designs.md)) |
| `picture` | slide | point cloud name, image path, or `none` |
| `picture-prompt` | slide | text for `mdeck ai pictures` |
| `background` | slide + deck | image path or `none` |
| `background-opacity` | slide + deck | 0 to 1, or a percentage |
| `logo` | slide + deck | path or `none` |
| `transition` | slide + deck | `fade`, `slide`, `spatial`, `zoom` (slide only, with `zoom-to`), `none`, or a transition an extension registers |
| `zoom-to` | slide | a thermal spot name on the slide before |
| `thermal-window` | slide | `25..90 °C` |
| `reveal` | slide + deck | `steps`, `none` (MD-21) |

### Deck settings

| Key | Values |
|---|---|
| `title`, `author` | text |
| `theme` | a theme name or path (default `dark`) |
| `engine` | an engine name (default: the theme's) |
| `transition` | as above (default: the theme's, else the config's, else `fade`) |
| `slide-level` | 1 to 6 (default: inferred) |
| `countdown` | `on`, `off` (both directions work) |
| `reveal` | `steps`, `none` |
| `footer` | text, drawn in the window and in export |
| `logo`, `logo-position`, `logo-opacity`, `logo-height` | path; corner; opacity; pixels |
| `background`, `background-opacity` | as above |
| `palette` | thermal palette name (it names thermal colour maps only) |
| `requires` | packs and extensions the deck expects; `--check` names missing ones |
| `art-world` | text for `mdeck ai pictures` |
| `image-style`, `icon-style` | style name for `mdeck ai images` and `mdeck ai icons` |

The generated reference (`mdeck spec`, section 7.3) is the authoritative list with defaults.

### Visuals, images, steps, columns, notes

- **LANG-11** MUST `implemented`: mdeck's fences keep the `@` prefix (```` ```@bar ````,
  ```` ```@notes ````). It is the one place where `@` earns its keep: it namespaces mdeck's kinds
  against real programming languages, and on GitHub the block reads as data. Tags match exactly,
  with one name per kind and no aliases (VIZ-02), and one grammar inside (VIZ-03).
- **LANG-12** MUST `implemented`: Image options stay in the alt text, which is invisible when the
  image renders. They use the settings grammar: `![Team @width: 60% @fill](team.png)`, with an
  optional space after the colon. The options are `@width`, `@height` and `@fill`; unknown
  options are reported.
  - Image placement that is really a design choice (side, full-bleed) belongs to the design and
    its arrangement, not to image options; v1's `@fit`, `@left`, `@right` and `@center` are
    reported as removed.
- **LANG-13** MUST `implemented`: `+` list items are steps ([02](02-markdown-and-slides.md)). `*`
  has no special meaning.
- **LANG-14** MUST `implemented`: `+++` on its own line separates columns (the `columns` design).
  It is short and memorable, and Hugo's TOML conflict only applies at the very top of a file.
- **LANG-15** MUST `implemented`: Speaker notes are a ```` ```@notes ```` fenced block holding
  markdown (MD-14). There is no `???` separator.
- **LANG-16** MUST `implemented`: AI placeholders stay in the source: `![prompt](generate:)` for
  an image and `(icon: generate:, prompt: "...")` for a diagram icon. `mdeck ai` resolves them
  through the asset manifest and never rewrites the deck ([10](10-generated-assets-and-ai.md)).

### The whole language on one card

~~~~markdown
---
title: Launching Orbit          # deck settings: plain YAML
theme: ember
---

# Launching Orbit               # first H1: the title slide
## Private analytics, finally

## Why now                      # each heading at the slide level: a new slide
<!-- design: statement -->      # slide settings: an HTML comment

The market moved.

## What it does
<!-- picture: rocket -->

+ Collects nothing personal     # + : one step per item
+ Answers in milliseconds

```@notes
Pause here. **Ask** who has been fined under GDPR.
```

## Growth                       # a visual follows
```@bar
- Q1: 12
- Q2: 19
```
~~~~

(Comments in the card are explanations for this document; they are not mdeck syntax.)
