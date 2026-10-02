# 02. Markdown and slides

How a markdown file becomes a sequence of slides, which markdown mdeck understands, and how
content is revealed step by step.

## Today

### Pipeline

`parser::parse` (`crates/mdeck/src/parser/mod.rs:23`) runs, in order:

1. frontmatter split
2. `splitter::split` into raw slides
3. empty slides dropped
4. per slide (`parse_slide`):
   1. notes extracted (`???`)
   2. directives extracted
   3. blocks parsed
   4. `@story` / `@scene` fences removed
   5. `@thermal-window` appended to thermal blocks
   6. the layout classified once and stored

### Slide splitting (`parser/splitter.rs`)

There are three mechanisms, applied in phases:

1. **Dash separator:** a line of three or more `-` with a blank line (or file start) before it and
   a blank line (or end) after it, outside fences.
2. **Blank gap:** three or more consecutive blank lines.
3. **Heading split:** an ATX heading (`#`) at or above the *slide level* starts a new slide when
   the current one has content.
   - The level is `@slide-level: N`, or else inferred: level 2 when the file has zero or one H1,
     level 1 when it has two or more.
   - With an inferred level, an H2 directly after a lone H1 with no body becomes its subtitle
     instead of a new slide.

Things that do **not** split:

- **Setext headings** (`Title` underlined with `===` or `---`) never split, although the block
  parser does treat them as headings.
- A `---` without blank lines around it is either a setext H2 underline or literal text.

**Directive migration.** A run of directive lines at the end of a slide, just before a splitting
heading, is moved to the next slide (`strip_trailing_directives`, `splitter.rs:296`). This is why
`@layout: quote` on the line above `# Heading` works. The check ignores indentation, so an indented
directive inside the previous slide's last list item also migrates: a verified bug.

**The owner's experience.** Adding a slide setting "only worked with `---` between slides". Whatever
the exact failing case, that is the symptom of the problem: whether a `@` line applies to a slide
depends on subtle rules, none of which are visible to the author:

- start of slide versus elsewhere;
- known name versus unknown name;
- column 0 versus indented;
- before versus after the heading;
- whether the next heading is at the splitting level.

### Supported markdown (`parser/blocks/mod.rs:31`)

**Supported:**

- ATX headings, paragraphs, emphasis (bold, italic, strikethrough), inline code
- links (shown styled, not clickable; `url` is unused)
- unordered and ordered lists with nesting
- images on their own line
- fenced code (backticks or tildes, with line highlighting `{1,3-5}`)
- blockquotes (flattened to one paragraph)
- pipe tables
- `***` / `___` rules
- multi-line HTML comments (skipped)
- math (`$`, `$$`)

**Shown as literal junk:**

- raw HTML (mdeck's own README title slide is a wall of `<p align="center">`)
- indented code blocks (become prose)
- task lists `[ ]` / `[x]`
- GitHub alerts `> [!NOTE]`
- footnotes, reference-style links and their definitions
- autolinks `<https://...>`
- nested blockquotes
- `<details>`
- badge rows (become link text)

**Silently wrong:**

- ordered list start numbers (always 1)
- table column alignment (ignored)

`--check` reports none of this.

### Notes

- A line of three or more `?` starts the notes; everything after it on that slide is notes.
- Notes are extracted *after* splitting, so a heading or `---` inside the notes starts a new,
  visible slide.
- Notes are never shown while presenting: there is no presenter view. They are used by
  `export --format pdf --notes`, by `--check`, and as context for AI prompts.

### Reveal (steps)

In lists:

| Marker | Effect |
|---|---|
| `-` | static |
| `+` | the next step |
| `*` | revealed together with the previous `+` (or static if no `+` came before) |
| ordered lists | static |

Charts and diagrams use the same markers on their items.

How steps behave:

- A slide's step count is the **maximum** over its blocks, not the sum, so two `+` lists on one
  slide reveal in parallel. The spec does not say this.
- Hidden list items do not reserve space, so content below them moves down as they appear, and
  they pop in without animation (charts and diagrams do animate).
- On editorial engines nested items inherit their parent's step, so a nested `+` counts as a step
  that reveals nothing.

## Assessment

1. **Slide boundaries have three mechanisms and hidden rules.** A heading split is the natural one:
   it is how people already structure markdown. `---` is a markdown *thematic break* (and a
   setext underline), so reusing it as "new slide" fights the format. Three blank lines are
   invisible. Setext headings, which many people use, do not split at all.
2. **Settings attach to slides by position rules nobody can see.** This must become one simple,
   checkable rule.
3. **Plain markdown in the wild breaks visibly.** That violates P4. The fix is cheap for most
   constructs (strip HTML tags and keep the text, render task lists as checkboxes, drop reference
   definitions, render alerts as callouts).
4. **Notes can leak onto the screen**, and there is nowhere to read them while presenting.
5. **`*` meaning "with previous" is a trap** for imported markdown, where `*` is simply another
   bullet character. Parallel reveal across lists is surprising. Layout jumping as items appear
   is not polished.

## Requirements

### Splitting

- **MD-01** MUST `keep`: Headings split slides. The slide level is inferred as today (0 or 1 H1
  gives level 2, otherwise level 1) and can be set by the deck.
- **MD-02** MUST `change`: Setext headings split exactly like the equivalent ATX headings.
- **MD-03** MUST `keep`: An H2 directly under a lone H1 with no body of its own is the H1's
  subtitle, not a new slide.
- **MD-04** SHOULD `change`: An explicit slide break exists for slides without a heading (a
  full-screen image, a quote). It is optional; no deck needs it.
  - **Decided:** the explicit break is `---` with blank lines around it, as in Marp and Slidev.
    A heading never *requires* it.
- **MD-05** MUST `remove`: Three blank lines no longer split slides. Invisible syntax is not
  syntax.
- **MD-06** MUST `new`: A `---` that is not a slide break (for example directly under a paragraph,
  making a setext heading) behaves exactly as standard markdown says.

### Attaching settings to slides

- **MD-07** MUST `change`: A slide setting applies to the slide it is written in. The slide
  begins at its heading (or at an explicit break) and ends where the next one begins. A setting
  written directly under the heading always applies to that slide, whatever separates the slides.
- **MD-08** MUST `remove`: Settings never migrate from one slide to the next. The current
  backward migration (`strip_trailing_directives`) and its indentation bug go away. A setting
  above a slide's heading belongs to the previous slide, and `--check` warns when it was
  probably meant for the next one.
- **MD-09** MUST `new`: `mdeck --check -v` lists, per slide, the settings that apply to it. A
  setting that applies nowhere, has an unknown name or an invalid value is always reported.
- **MD-10** MUST `new`: The syntax for slide settings is one rule, with no "known name versus
  unknown name" or "start of slide versus elsewhere" exceptions. See [07](07-authoring-language.md).

### Markdown coverage

- **MD-11** MUST `keep`: CommonMark plus the GitHub extensions that people use (tables,
  strikethrough, task lists, autolinks, fenced code) is the input language.
- **MD-12** MUST `new`: Every CommonMark/GFM construct is either rendered well or degrades
  invisibly. Concretely:
  - **Raw HTML:** tags are removed and their text content kept; `<img>` becomes an image.
  - **Task lists:** render as checked and unchecked boxes.
  - **Links:** reference-style links resolve, and their definition lines are never shown.
  - **Autolinks:** render as links.
  - **Footnotes:** markers are hidden, and footnote text goes to the notes.
  - **GitHub alerts:** render as callouts.
  - **Indented code:** renders as code.
  - **Nested quotes:** render as nested quotes.
  - **Ordered lists:** honour their start number.
  - **Tables:** honour column alignment.
- **MD-13** MUST `new`: When mdeck cannot present something it parsed, `--check` names it
  (category `content`).

### Notes

- **MD-14** MUST `change`: Speaker notes are written in a fenced block tagged `@notes`:

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
  - This replaces `???` (`remove`). The `???` separator could not work once notes contain
    markdown: a heading inside the notes would start a new slide.
- **MD-15** MUST `new`: Notes are markdown. The presenter sees them rendered (headings, emphasis,
  lists, code, math) while presenting (RUN-03), and in the PDF notes pages. Today they exist only
  in PDF export, as plain text.

### Steps

- **MD-16** MUST `keep`: `+` list items reveal one step at a time. Plain `-` and `*` items are
  static.
- **MD-17** MUST `change`: `*` has no special meaning. Grouping several items in one step is
  expressed by nesting them under a `+` item, which reveals its children with it. (This removes
  the "with previous" marker; see [removal candidates](removal-candidates.md).)
- **MD-18** MUST `change`: Steps are counted across the whole slide in reading order. The second
  `+` list on a slide continues after the first one; it no longer reveals in parallel.
- **MD-19** MUST `new`: Hidden content reserves its space. Revealing never moves content already
  on screen; revealed items animate in.
- **MD-20** MUST `keep`: Visuals reveal their items with the same markers and count in the same
  step sequence.
- **MD-21** SHOULD `new`: A deck setting turns all reveal off (`reveal: none`), so imported
  markdown that happens to use `+` bullets presents without clicks.
