# 07. The authoring language

Everything mdeck adds on top of markdown, why today's syntax falls short, and the proposed v2
language.

## Today

mdeck layers seven kinds of extra syntax on markdown:

| Syntax | Where | Example |
|---|---|---|
| `@key: value` in the frontmatter | deck settings | `@theme: ember` |
| `@key: value` lines in the body | slide settings | `@layout: quote`, `@illustration: rocket` |
| `@tag` fence info strings | visuals | ```` ```@barchart ```` |
| `@word` / `@word:value` in image alt text | image options | `![Team @width:60%](team.png)` |
| `+` / `*` list markers | reveal steps | `+ appears on click` |
| `+++` line | column break | |
| `???` line | speaker notes | |

### Frontmatter

- **Recognised keys** (`parser/frontmatter.rs:102`):
  - plain keys: `title`, `author`, `date` (`date` is parsed and never used);
  - `@` keys: `@theme`, `@engine`, `@transition`, `@slide-level`, `@countdown`, `@story`,
    `@image-style`, `@icon-style`, `@palette`, `@art`, `@logo`, `@logo-position`, `@logo-opacity`,
    `@logo-height`, `@background`, `@background-opacity`;
  - `@aspect`, `@code-theme` and `@footer`, which are reserved or half-implemented.
- **`@` is not valid at the start of a YAML key.** mdeck pre-quotes these lines before parsing.
  Every other tool sees invalid YAML.
- **Unknown keys are dropped silently.** `theme: dark` without the `@` is ignored, and so is
  `@thme: nord`. Invalid values (`@transition: wipe`) are not reported either.

### Slide directives (`parser/directives.rs`)

- **Known names.** `SLIDE_DIRECTIVES`:
  - `layout`, `illustration`, `logo`, `art`, `background`, `background-opacity`;
  - `thermal-window`, `zoom`;
  - `class` (reserved, unused).
- **How a line is taken:**
  - At the start of a slide, any `@name: value` line is taken, known or not, and unknown ones
    vanish.
  - After that, only known names are taken, and only at column 0 outside fences.
  - Unknown names after the start stay visible as text.
- **Last wins** when a name repeats.
- **Lines move across slides.** A run of directive lines just before a splitting heading moves to
  the next slide ([02](02-markdown-and-slides.md)).
- **Deck settings inside a slide.** Global names written in a slide are removed and ignored, and
  `--check` reports them.
- **Layout values.** An unknown `@layout` value becomes `content` without a warning.

### Visual fences

- There are 20 tags, matched by prefix.
- `@story` and `@scene` are authoring fences that are never rendered.
- Ordinary code fences take a language and line highlights: ```` ```rust {3,5-7} ````.

### Other

- **Image hints.** No space after the colon (`@width:80%`). Unknown hints vanish from the alt text.
- **AI placeholders.** `![prompt](image-generation)` and `(icon: generate-image, prompt: "...")` are
  rewritten in place by `mdeck ai generate`.

### Precedents

| Tool | Deck settings | Slide settings | Invisible on GitHub? |
|---|---|---|---|
| Marp | plain YAML keys (`theme:`) | `<!-- _class: lead -->` HTML comments | yes |
| Slidev | plain YAML keys | per-slide YAML block after `---` | partly |
| reveal.js (markdown) | n/a | `<!-- .slide: data-background=... -->` | yes |
| Deckset | `theme:` lines at the top | `[.background-color: #000]` | no |
| Pandoc/Quarto | YAML (`format:`) | `{.class key=value}` attributes, `::: columns` fenced divs | no |
| mdeck v1 | `@key:` YAML (invalid) | `@key: value` lines | no |

## Assessment

1. **The `@` in the frontmatter is the most expensive choice in the language, and it buys
   nothing.** It makes nearly every deck's line 2 invalid YAML for every tool except mdeck,
   breaking VIS-03 and mdeck's own "graceful degradation" principle. Frontmatter keys are already
   namespaced by position. Meanwhile, the natural thing (`theme: dark`) is silently ignored.
2. **Slide directives are visible on every other renderer.** On GitHub, `@illustration: rocket`
   is a stray paragraph, and in issues and PRs `@layout` may even render as a user mention.
3. **Directive extraction rules cannot be explained in one sentence.** The
   start/known/column-0/migration rules are exactly what made the owner believe slide settings
   only work with `---`.
4. **Five micro-grammars for "key: value":**
   - `@k: v`;
   - `@k:v` in alt text;
   - `# k: v` in charts;
   - `k: v` in thermal;
   - `(k: v)` on items.
5. **Silence on mistakes everywhere.** There are no checks for:
   - frontmatter keys or values;
   - layout names;
   - fence tags;
   - image hints;
   - content that will show as raw syntax.
6. **Reserved names look supported.** `@aspect`, `@code-theme`, `@class` and `date` are accepted
   and do nothing. `@footer` works in the window but not in export. `@countdown: true` cannot turn
   a countdown on.

## Requirements: the v2 language

### Principles

- **LANG-01** MUST `new`: The language has exactly **one way to write a setting**: `key: value`,
  with the same key names wherever they appear.
  - A key in the frontmatter is the **deck value**.
  - The same key in a slide's settings is **that slide's value**, overriding the deck's.
  - Keys that only make sense deck-wide (for example `theme`, `engine`, `slide-level`) are errors
    in a slide.
- **LANG-02** MUST `new`: Every mdeck addition is either **invisible** on a standard markdown
  renderer (frontmatter, HTML comments, alt text) or **reads as meaningful markdown** there (a
  fenced code block of chart data, a `+` bullet, a notes block). Nothing renders as stray mdeck
  syntax.
- **LANG-03** MUST `new`: Every name and value in the language is validated. Unknown keys, unknown
  values, unknown visual tags and unknown image hints are `--check` errors, with a "did you mean"
  suggestion. Nothing the author writes is silently ignored (VIS-13).
- **LANG-04** MUST `new`: The language is generated from one table in the code (name, scope, type,
  allowed values, description, since-version). The format reference, `mdeck spec --short`, the
  `--check` validation, editor completion and the AI skill are all produced from that table, so
  they cannot drift.
- **LANG-05** MUST `remove`: Reserved-but-unimplemented names are not accepted. A name exists when
  it works.

### Deck settings (frontmatter)

- **LANG-06** MUST `change`: The frontmatter is plain, valid YAML with plain keys:

  ```yaml
  ---
  title: Launching Orbit
  author: Kristofer Liljeblad
  theme: ember
  transition: slide
  reveal: steps
  ---
  ```

  - **Decided:** plain top-level keys (Marp and Slidev style), not nested under an `mdeck:` map.
    Decks are written for mdeck, and simplicity wins over avoiding collisions with other tools'
    keys.
- **LANG-07** MUST `new`: v2.0 breaks cleanly with the v1 syntax. There is no migration tool, and
  v2 does not honour v1 syntax. Instead:
  - the v2 format is completely specified in the format reference (`mdeck spec`), which is
    precise enough for a user to hand to their AI harness with "convert this deck to the v2
    format";
  - the v2.0 release notes and the format reference have an "Upgrading from v1" section that maps
    every v1 construct to its v2 form (the "Replaces" columns below);
  - `--check` recognises v1 constructs (`@` frontmatter keys, `@key:` slide lines, renamed tags)
    and names the v2 form in its message, for example
    `slide 4: "@layout: quote" is v1 syntax; write <!-- design: quote -->`. This is a validation
    message (LANG-03), not a converter.

### Slide settings

- **LANG-08** MUST `change`: Slide settings are written in an **HTML comment** inside the slide.
  The comment holds one or more `key: value` lines:

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

  - A comment is treated as mdeck settings when its first line is `key: value` with a known slide
    setting key.
  - Any other comment is an ordinary comment. `--check` warns when it looks like a setting with a
    misspelt key.
  - **Decided:** no explicit marker. Marp's convention has shown that plain keys work, and the
    validation in LANG-03 catches typos.
- **LANG-09** MUST `change`: A settings comment applies to the slide it is in, wherever in the slide
  it is written (MD-07). It never moves to another slide.
- **LANG-10** MUST `remove`: Visible `@key: value` lines are no longer settings. `--check` points
  at each one and names its v2 form (LANG-07).

### Slide settings in v2

| Key | Scope | Values | Replaces |
|---|---|---|---|
| `design` | slide | a design name ([03](03-slide-designs.md)) | `@layout` |
| `picture` | slide + deck default | point cloud name, image path, or `none` | `@illustration`, `@art: none` |
| `picture-prompt` | slide | text for `mdeck ai art` | `@art:` (slide) |
| `background` | slide + deck | image path or `none` | same |
| `background-opacity` | slide + deck | 0-1 or % | same |
| `logo` | slide + deck | path or `none` | same |
| `transition` | slide + deck | `slide`, `fade`, `spatial`, `zoom`, `none` | deck `@transition`; slide `@zoom` becomes `transition: zoom` + `zoom-to:` |
| `zoom-to` | slide | a thermal spot name | `@zoom` |
| `thermal-window` | slide | `25..90 °C` | same |
| `reveal` | slide + deck | `steps`, `none` | new (MD-21) |

### Deck settings in v2

| Key | Values | Replaces |
|---|---|---|
| `title`, `author` | text | same (`date` is removed until it is used) |
| `theme` | a theme name or path | `@theme` |
| `engine` | an engine name | `@engine` |
| `transition` | as above | `@transition` |
| `slide-level` | 1-6 | `@slide-level` |
| `countdown` | `on`, `off` | `@countdown` (now both directions work) |
| `reveal` | `steps`, `none` | new |
| `footer` | text | `@footer` (implemented in the window *and* export, or removed) |
| `logo`, `logo-position`, `logo-opacity`, `logo-height` | as today | `@logo*` |
| `background`, `background-opacity` | as today | `@background*` |
| `palette` | thermal palette name | `@palette` (it names thermal colour maps only) |
| `art-world` | text for `mdeck ai art` | `@art` (deck) |
| `image-style`, `icon-style` | style name | `@image-style`, `@icon-style` |

### Visuals, images, steps, columns, notes

- **LANG-11** MUST `change`: mdeck's fences keep the `@` prefix (```` ```@bar ````,
  ```` ```@notes ````). It is the one place where `@` earns its keep: it namespaces mdeck's kinds
  against real programming languages, and on GitHub the block reads as data. Tags match exactly,
  with one name per kind and no aliases (VIZ-02), and one grammar inside (VIZ-03).
- **LANG-12** MUST `change`: Image options stay in the alt text, which is invisible when the image
  renders. They use the settings grammar: `![Team @width: 60% @fill](team.png)`, with an optional
  space after the colon. Unknown options are errors.
  - Image placement that is really a design choice (side, full-bleed) belongs to the design and
    its arrangement, not to image options.
- **LANG-13** MUST `keep`: `+` list items are steps ([02](02-markdown-and-slides.md)). `*` loses its
  special meaning.
- **LANG-14** MUST `keep`: `+++` on its own line separates columns (the `columns` design).
  - **Decided:** keep `+++`. It is short and memorable, and Hugo's TOML conflict only applies at
    the very top of a file.
- **LANG-15** MUST `change`: Speaker notes are a ```` ```@notes ```` fenced block holding markdown
  (MD-14). `???` is removed.
- **LANG-16** MUST `keep`: AI placeholders (`![prompt](image-generation)`, generated icons) remain,
  and are documented under [10](10-generated-assets-and-ai.md).

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
