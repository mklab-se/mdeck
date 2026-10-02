# 02. Markdown and slides

How a markdown file becomes a sequence of slides, which markdown mdeck understands, and how
content is revealed step by step.

In short: headings split slides, `---` with blank lines around it is an optional explicit break,
a slide's settings live in an HTML comment inside that slide and never move, speaker notes are
```` ```@notes ```` blocks, and `+` items are the steps.

## Requirements

### Splitting

- **MD-01** MUST `implemented`: Headings split slides. The slide level is inferred (zero or one H1
  gives level 2, otherwise level 1) and can be set with the deck setting `slide-level`.
- **MD-02** MUST `implemented`: Setext headings split exactly like the equivalent ATX headings.
- **MD-03** MUST `implemented`: An H2 directly under a lone H1 with no body of its own is the H1's
  subtitle, not a new slide.
- **MD-04** SHOULD `implemented`: An explicit slide break exists for slides without a heading (a
  full-screen image, a quote): `---` with blank lines (or the start or end of the file) on both
  sides, as in Marp and Slidev. It is optional; a heading never requires it.
- **MD-05** MUST `implemented`: Blank lines never split slides, however many there are. Invisible
  syntax is not syntax.
- **MD-06** MUST `implemented`: A `---` that is not a slide break (for example directly under a
  paragraph, making a setext heading) behaves exactly as standard markdown says.

### Attaching settings to slides

- **MD-07** MUST `implemented`: A slide setting applies to the slide it is written in. The slide
  begins at its heading (or at an explicit break) and ends where the next one begins. A setting
  written directly under the heading always applies to that slide, whatever separates the slides.
- **MD-08** MUST `implemented`: Settings never migrate from one slide to the next. A setting above
  a slide's heading belongs to the previous slide, and `--check` warns when it was probably meant
  for the next one (a settings comment that ends a slide, not directly under that slide's own
  heading).
- **MD-09** MUST `implemented`: `mdeck --check -v` lists, per slide, the settings that apply to it
  (marking those that come from the deck). A setting that has an unknown name, an invalid value or
  the wrong scope is always reported.
- **MD-10** MUST `implemented`: The syntax for slide settings is one rule, with no "known name
  versus unknown name" or "start of slide versus elsewhere" exceptions. See
  [07](07-authoring-language.md).

### Markdown coverage

- **MD-11** MUST `implemented`: CommonMark plus the GitHub extensions that people use (tables,
  strikethrough, task lists, autolinks, fenced code) is the input language.
- **MD-12** MUST `implemented`: Every CommonMark/GFM construct is either rendered well or degrades
  invisibly. Concretely:
  - **Raw HTML:** tags are removed and their text content kept; `<img>` becomes an image and
    `<h1>` to `<h6>` a heading.
  - **Task lists:** render as checked and unchecked boxes.
  - **Links:** reference-style links resolve, and their definition lines are never shown.
  - **Autolinks:** render as links.
  - **Footnotes:** markers are hidden, and footnote text goes to the notes.
  - **GitHub alerts:** render as callouts.
  - **Indented code:** renders as code.
  - **Nested quotes:** render as nested quotes, each set in from its parent.
  - **Ordered lists:** honour their start number.
  - **Tables:** honour column alignment.
- **MD-13** MUST `implemented`: When mdeck cannot present something it parsed, `--check` names it
  (category `content`, or the category of the feature involved: `visual`, `point-cloud`,
  `engine`, `math`, `fonts`).

### Notes

- **MD-14** MUST `implemented`: Speaker notes are written in a fenced block tagged `@notes`:

  ````markdown
  ## Why now

  The market moved.

  ```@notes
  Pause here. **Ask the room** who has been fined under GDPR.

  ## If there is time
  - the Meta fine
  - the Amazon fine
  ```
  ````

  - The fence makes the notes' boundary explicit. Notes can contain anything markdown can,
    including headings, lists, `---` and code (with a longer outer fence). None of it ever
    splits the slide or appears on it.
  - A `@notes` block can sit anywhere in the slide. Several blocks on one slide are joined in
    order.
  - On GitHub and in editors, the notes show as a code block under the slide's content: visible
    and readable, clearly set apart.
  - There is no other notes syntax. A v1 `???` separator is reported by `--check` with this form.
- **MD-15** MUST `implemented`: Notes are markdown. The presenter sees them rendered (headings,
  emphasis, lists, code, math) in the presenter view (RUN-03), and in the PDF notes pages.

### Steps

- **MD-16** MUST `implemented`: `+` list items reveal one step at a time. Plain `-` and `*` items
  are static.
- **MD-17** MUST `implemented`: `*` has no special meaning. Grouping several items in one step is
  expressed by nesting them under a `+` item, which reveals its children with it (a nested `+`
  item is a step of its own).
- **MD-18** MUST `implemented`: Steps are counted across the whole slide in reading order. The
  second `+` list on a slide continues after the first one, and a visual's steps follow the
  steps written before it.
- **MD-19** MUST `implemented`: Hidden content reserves its space. Revealing never moves content
  already on screen; revealed items animate in.
- **MD-20** MUST `implemented`: Visuals reveal their items with the same markers and count in the
  same step sequence.
- **MD-21** SHOULD `implemented`: The setting `reveal: none` (deck or slide) turns steps off, so
  imported markdown that happens to use `+` bullets presents without clicks. A slide's
  `reveal: steps` turns them back on for that slide.
