<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Writing slides

Any markdown file is a deck. mdeck splits it into slides, recognises what kind of slide each one
is, and handles overflow, so you write content, not layout. Everything on this page is optional:
a file with no mdeck syntax at all is a complete deck.

Charts and diagrams have their own page, [Visualizations](visualizations.md). The complete
reference is the [format specification](../crates/mdeck/doc/mdeck-spec.md) (also `mdeck spec`,
and `mdeck spec --short` for a one-page card). Coming from mdeck 1? See
[Upgrading from v1](upgrading-from-v1.md).

## Slides

Two things start a new slide, and nothing else does:

1. **A heading at the slide level**, written `#` style or underlined (setext). With at most one
   `#` heading in the file, both `#` and `##` start slides; with several `#` headings, only `#`
   does. Set `slide-level: 2` (any of 1 to 6) in the frontmatter to choose. A `##` written
   directly under the lone `#` with nothing of its own is that title's subtitle.
2. **A `---` line** with a blank line above and below, for a slide without a heading (a
   full-slide photo, a quote).

Blank lines never split a slide, and nothing inside a fenced block or an HTML comment does.

## Designs

Every slide has one of thirteen **designs**, recognised from what is on it. You do not choose;
you write, and the first row of this table that matches wins:

| # | Design | The slide has |
|---|---|---|
| 1 | `columns` | a column separator, `+++` |
| 2 | `title` | the first slide, a lone H1 |
| 3 | `title` | an H1 + one short line, an H2 or a paragraph |
| 4 | `section` | a lone heading |
| 5 | `section` | a heading + a deeper heading, its kicker |
| 6 | `statement` | at most a heading + 1 or 2 short paragraphs |
| 7 | `points` | a heading + one list, optionally a lead paragraph before it |
| 8 | `quote` | one quote, optionally a heading and an attribution after it |
| 9 | `media` | one image, optionally a heading, a lead before it and a caption after it |
| 10 | `gallery` | two or more images, optionally a heading |
| 11 | `split` | one image + text: paragraphs or one list |
| 12 | `code` | one code block, optionally a heading and a short paragraph |
| 13 | `visual` | one chart or diagram, optionally a heading and a short paragraph |
| 14 | `table` | one table, optionally a heading and a short paragraph |
| 15 | `content` | anything else |

A short line is at most 120 characters; a statement has at most 2 paragraphs of at most 240
characters; the lead of a code, visual or table slide is at most 240 characters. Horizontal rules
do not count. `mdeck talk.md --check -v` prints every slide's design and the rule that matched.

Some consequences worth knowing:

- The most common slide in a talk, **a heading and a sentence, is a `statement`**: the sentence
  in large type.
- A heading with a list and a picture beside it is `split`; a single photo is `media`, and
  `![alt @fill](photo.jpg)` makes it cover the whole slide.
- **No design drops anything.** A diagram with bullets, two charts, code with an image: that is
  `content`, which shows everything in reading order, and whatever a design has no place for
  shows in its body.

How each design looks comes from the theme's design set: the classic centred `standard` set or
the magazine-style `editorial` set ([Themes](themes.md#designs-and-arrangements)). A slide is the
same design in every theme. Content that does not fit first shrinks (code down to 40% of its
size, then prose and lists down to 80%), then scrolls with Up and Down, with a fade at the edge.

## Settings

**Deck settings** are plain YAML keys in the frontmatter:

```yaml
---
title: Launching Orbit
author: Ada Lovelace
theme: ember
transition: slide
---
```

**Slide settings** are `key: value` lines in an HTML comment anywhere in the slide. The comment is
invisible on GitHub and in every markdown preview, and it applies to the slide it is in, never to
another one:

```markdown
## Why now
<!-- design: statement -->

The market moved, and we are the only ones ready.

## Our platform
<!--
design: split
picture: rocket
-->

- Fast
- Private
```

A comment counts as settings when its first line is `key: value` with a known key; any other
comment is an ordinary comment. A key set on a slide overrides the deck's value for that slide.

| Setting | Where | Values |
|---|---|---|
| `title`, `author` | deck | text |
| `theme` | deck | a theme name or a path to a theme file (default `dark`) |
| `engine` | deck | an engine name, instead of the theme's ([Engines](engines.md)) |
| `slide-level` | deck | 1 to 6 (default inferred) |
| `countdown` | deck | `on`, `off`: the opening countdown (default the theme's) |
| `footer` | deck | text at the foot of every slide |
| `logo-position`, `logo-opacity`, `logo-height` | deck | the logo's corner, strength and size ([Themes](themes.md#logos)) |
| `palette` | deck | the palette of `@thermal` images that name none |
| `requires` | deck | packs and extensions the deck expects, `[acme-brand, glow]` |
| `art-world`, `image-style`, `icon-style` | deck | inputs for `mdeck ai` ([AI](ai.md)) |
| `transition` | deck and slide | `fade`, `slide`, `spatial`, `none`; on a slide, how it is entered |
| `reveal` | deck and slide | `steps` (default) or `none` |
| `logo` | deck and slide | an image file, or `none` |
| `background`, `background-opacity` | deck and slide | an image behind the slide ([Themes](themes.md#background-images)) |
| `design` | slide | a design name, instead of the recognised one |
| `picture` | slide | a point cloud name or an image path for the slide's stage, or `none` ([Engines](engines.md#pictures)) |
| `picture-prompt` | slide | what `mdeck ai pictures` draws for this slide |
| `zoom-to` | slide | enter this slide by zooming into a named thermal spot on the slide before |
| `thermal-window` | slide | one temperature scale for the slide's `@thermal` images |

Choose a design with `<!-- design: name -->` when recognition is not what you want. A chosen
design that has no place for some content shows the rest in its body; one that lacks its core
block (`design: quote` without a quote) draws the slide as `content`. `mdeck --check` reports
both, along with unknown keys and values (with a "did you mean"), deck settings written in a
slide, and v1 syntax with its v2 form.

## Steps

A list item that starts with `+` is a **step**: it appears on the next key press, with its
nested items. `-` and `*` items are always shown.

```markdown
## The plan

- Where we are
+ What we change
  - and the detail that comes with it
+ What it costs
```

Steps count across the whole slide in reading order: a second `+` list, and the `+` items inside
a chart or diagram, continue after the first. Hidden items keep their space, so nothing moves when
one appears. `reveal: none` (deck or slide) shows everything at once.

## Speaker notes

A fenced block tagged `@notes`, anywhere in the slide, holds your notes in markdown. It never shows
on the slides, and nothing in it (headings, `---`, lists) starts a slide. Several notes blocks on
one slide are joined in order. Footnotes (`text[^1]`) go to the notes too.

````markdown
## Key decision

- We chose microservices for team autonomy

```@notes
Stress that this was about **teams shipping independently**, not scale.
```
````

The presenter view (`V`) shows them rendered ([Presenting](presenting.md#presenter-view)), and
`mdeck export talk.md --format pdf --notes` prints them under each slide. On GitHub a notes block
reads as a code block under the slide: visible, and clearly set apart.

## Columns

`+++` on its own line splits a slide into columns, side by side. A heading before the first column
spans them all:

```markdown
## Before and after

**Before**

- Manual deploys
- Weekly releases

+++

**After**

- One-click deploys
- Daily releases
```

## Images

Standard markdown images work. One image is a `media` slide, two or more a `gallery` (alt texts
become captions), and an image with text a `split`. Where an image goes is up to the design;
three options in the alt text say how big it is:

- `@fill`: the image covers the whole slide (on a `split`, its whole panel);
- `@width: 60%`: the width as a share of the space the image has, or pixels on a 1920x1080
  slide (`@width: 800px`);
- `@height: 400px`: the same for the height.

```markdown
![Our team @fill](team.jpg)
![The new dashboard @width: 70%](dashboard.png)
```

The space after the colon is optional. The alt text minus its options is what a gallery shows as
the caption. Any other `@` word is reported by `--check`; the v1 options `@fit`, `@left`,
`@right` and `@center` are gone, because placement belongs to the design.

PNG, JPEG, WebP and SVG work, and large photos decode in the background so they never stall a
transition. `![A harbour at dawn](generate:)` is a placeholder for an image `mdeck ai images` makes
([AI](ai.md#images-and-icons)); until then it shows as a quiet card with its prompt.

## Math

Formulas are LaTeX between dollar signs, typeset with KaTeX's fonts, sharp at any resolution:
`$E = mc^2$` inline, `$$...$$` centred on its own line.

```markdown
The roots of $ax^2 + bx + c = 0$ are

$$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$$
```

Dollar amounts such as `$5 and $10` stay text; write `\$` for a literal dollar anywhere else. A
formula that does not parse shows as its source, and `--check` says why.

## Markdown coverage

Everything common in READMEs, notes and docs presents cleanly:

- **Text:** bold, italic, strikethrough, inline code, links, backslash escapes. Links show but are
  not clickable while presenting.
- **Lists:** nested, ordered lists keep their start number, task lists show their box.
- **Quotes:** several paragraphs and nesting are kept; on a quote slide a short last paragraph is
  the attribution. GitHub alerts (`> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`,
  `[!CAUTION]`) are callouts.
- **Code:** fenced blocks are syntax highlighted (```` ```rust {3,5-7} ```` highlights lines);
  indented code is code too.
- **Tables:** column alignment from the separator row is kept.
- **Rules:** `***` or `___` draw a rule inside a slide (`---` is the slide break).
- **Reference links and autolinks** resolve, and their definition lines never show.
- **Raw HTML** keeps its text: `<img>` becomes an image, `<h1>` to `<h6>` a heading, `<br>` a line
  break; other tags vanish.
- **Symbols and scripts:** arrows, check marks and circled numbers come from bundled faces;
  Chinese, Japanese and Korean from a font on your system.

What cannot be presented (an image inside running text, a remote image, an HTML `<video>`) is
reported by `mdeck --check` in the `content` category instead of showing as raw syntax.

## Check your deck

```bash
mdeck talk.md --check        # problems, each on its line in the file
mdeck talk.md --check -v     # also every slide's design, steps and settings
```

`--check` exits with status 1 when it finds something, so it fits in CI. Each warning names its
category: `settings`, `content`, `visual`, `architecture`, `thermal`, `math`, `fonts`, `theme`,
`background`, `point-cloud` (pictures), `assets` (generated assets), `engine` (what the chosen
engine does not show) and `extensions` (what `requires` names but is not installed).
