# mdeck format reference

This reference describes **mdeck 2**: everything a markdown file can say to mdeck, and how mdeck
presents it. It is printed by `mdeck spec` (the quick card is `mdeck spec --short`) and is precise
enough for an AI agent to write, check or convert a deck from it. Decks written for mdeck 1 need a
few changes; section 19 maps every v1 construct to its v2 form.

mdeck is a markdown-based presentation tool. Authors write standard markdown; mdeck recognises each
slide's design from its content and renders it as a presentation.

---

## 1. Design Principles

1. **Readability over expressiveness.** A MDeck document should read as a natural markdown document. Someone reading the raw source should understand the content without knowing MDeck exists.

2. **Inference over configuration.** MDeck recognises each slide's design from its content. Authors should almost never need to choose one explicitly.

3. **Standard markdown first.** Every feature uses standard CommonMark markdown when possible. What mdeck adds is either invisible on other renderers (frontmatter, HTML comments, alt text) or reads as meaningful markdown there (a fenced block of chart data, a `+` bullet, a notes block).

4. **Graceful degradation.** When rendered in a standard markdown viewer, a MDeck document should still be readable. Settings are HTML comments and vanish; slide breaks render as horizontal rules; notes show as a code block under the slide.

---

## 2. Document Structure

A MDeck document has two parts:

```
[frontmatter]       (optional, YAML deck settings)
[slides]            (content split at headings and explicit breaks)
```

### 2.1 Deck settings (frontmatter)

Plain YAML frontmatter, delimited by `---` on the first line and `---` on a subsequent line. Must be the very first content in the file (no preceding blank lines). Keys are plain YAML keys.

```yaml
---
title: "Building Resilient Systems"
author: "Jane Doe"
theme: dark
transition: slide
---
```

Every key is a **setting** (section 7). A key in the frontmatter is the deck's value; the same key in a slide's settings comment overrides it for that slide. Keys that only make sense for the whole deck (such as `theme`) are errors in a slide. Unknown keys, invalid values and v1 keys (`@theme`) are reported by `mdeck --check` with a "did you mean" or the v2 form; they are never honoured silently.

#### Transitions

| Transition | Effect |
|------------|--------|
| `slide`    | The next slide pushes the current one horizontally |
| `fade`     | Cross-fade between slides (default) |
| `spatial`  | Slides pan in the direction they sit in the grid overview, so `G` and navigation feel like one continuous space |
| `zoom`     | On a slide with `zoom-to`: enter it by zooming into a thermal spot of the slide before (section 14.20) |
| `none`     | Instant switch |

A custom build (section 18) may add transitions: `transition:` then names
one an extension registers, exactly like a built-in, and `T` cycles through
it after the built-ins.

All transitions use smooth easing and the built-ins last about a third of a
second. Cycle
the deck's transition while presenting with `T`. A slide's own `transition`
says how that slide is entered (and left again going back). The deck's
transition comes from its `transition` setting, then the theme's
`transition:` (section 9.4), then the user config (`defaults.transition`),
then the built-in `fade`; a blank or unknown value passes to the next in
line. Reduced motion shows no transitions, and a board engine (section 9.6)
draws its own: `mdeck --check` reports a slide's `transition` or `zoom-to` there,
since it has no effect.

**Parser rule:** If the document starts with a line that is exactly `---`, begin parsing YAML until a closing `---` line. If no closing `---` is found, there is no frontmatter and the whole file is slides. If the YAML is invalid, each `key: value` line is still read on its own.

---

## 3. Slide Separation

Two things start a slide: a heading at the slide level, and an explicit break. Nothing else does. Lines inside fenced code blocks (including notes blocks) and HTML comments never start a slide.

### 3.1 Headings

Headings start new slides when the current slide already has content. ATX headings (`# Title`) and setext headings (`Title` underlined with `===` or `---`) split alike. Which heading levels split depends on the **slide level**:

1. **Explicit:** Set `slide-level: N` in the frontmatter. Headings at level 1 through N all split.
2. **Inferred:** If `slide-level` is not set:
   - **Single H1 (or no H1):** slide level 2: both `#` and `##` split. This handles "proper" markdown files where H1 is the title and H2s are sections. An H2 that directly follows an H1 (nothing but blank lines and comments between them) and has no content of its own (the next thing after it is another heading, a break, or the end of the file) stays on the same slide and becomes its subtitle, giving a title slide. An H2 followed by its own paragraphs, lists or other blocks is a section and starts its own slide, so a README shaped `# Title` + `## Section` + content gets a title slide and one slide per section.
   - **Multiple H1s:** slide level 1: only `#` splits.

```markdown
# Title Slide

A subtitle

## First Topic

Content here: this is a separate slide because there's only one H1.

## Second Topic

More content: also a separate slide.
```

### 3.2 Explicit break: `---`

A line of three or more dashes with a blank line (or the start or end of the file) above and below is an explicit slide break. It is optional: use it for slides without a heading, such as a full-screen image or a quote. A heading never requires it.

```markdown
![Our team @fill](team.jpg)

---

> The best way to predict the future is to invent it.
```

A `---` directly under a line of text is a setext heading underline, as in standard markdown, never a break. Blank lines never split a slide, however many there are.

### 3.3 Settings belong to their slide

A slide begins at its heading (or at an explicit break) and ends where the next one begins. A setting written in a slide applies to that slide, and never moves to another one: a settings comment above a slide's heading belongs to the slide before it (section 7). Frontmatter `---` delimiters are never slide breaks.

### 3.4 Speaker Notes

Speaker notes are a fenced block tagged `@notes`, holding markdown. It can stand anywhere in the slide; several notes blocks on one slide are joined in order. Nothing inside a notes block ever splits the slide or shows on it, so notes may hold headings, lists, `---` and code (with a longer outer fence).

`````markdown
# Key Architecture Decisions

- Microservices over monolith
- Event-driven communication
+ gRPC for internal APIs

````@notes
This slide sets the stage for the technical deep-dive. Emphasize that
the microservices decision was driven by **team autonomy**, not scale.

## If there is time
- Ask: "How many of you have migrated from a monolith?"

```sh
kubectl get pods
```
````

# Next Slide
`````

Footnotes (`text[^1]` with `[^1]: the note` anywhere in the deck) are notes too: the marker is hidden on the slide and the footnote's text is added to that slide's notes.

**Printing notes:** `mdeck export deck.md --format pdf --notes` writes `deck-notes.pdf` with one notes page per slide: the slide on top and its notes below, dark text on white in any theme, in A4 proportions. Notes are rendered as markdown: headings, paragraphs, emphasis, lists, code, quotes, tables and math are printed; charts, diagrams and images in notes are left out. Notes that do not fit continue on the next page. Without `--notes`, `--format pdf` writes `deck.pdf` with one page per slide.

**Notes are markdown.** While presenting, the presenter view (`V`, `--presenter`, section 15) shows them rendered: headings, emphasis, lists, code, quotes, tables and math.

**Graceful degradation:** In a standard markdown viewer, a notes block renders as a code block under the slide's content: visible and readable, clearly set apart.

---

## 4. Slide Designs

Every slide has a **design**: what kind of slide it is (a title, a list, a quote, a chart).
MDeck recognises the design from the slide's content, the same way in every theme; the theme
decides how each design looks (section 9.9). No design ever drops content: whatever a design has
no place for shows in its body, in reading order.

### 4.1 The designs

<!-- generated: designs -->

`mdeck --check -v` prints each slide's design and the rule that matched, for example
`slide 4: statement (at most a heading + 1 or 2 short paragraphs)`.

What each design shows:

- **title**: the H1 large, the H2 or paragraph as its subtitle. On the first slide the deck's
  `author` is the byline (standard) or part of the eyebrow (editorial).
- **section**: the heading as a divider, a deeper heading or short line as its kicker.
- **statement**: one idea in large type, its heading small above it. The common
  `## Heading` + a sentence slide.
- **points**: the heading, an optional lead paragraph and the list. Every nesting level is
  drawn, ordered lists show their numbers.
- **split**: the text in one column, the image in the other.
- **media**: the image large, a paragraph before it as its lead, one after it as its caption.
  `![alt @fill](path)` covers the whole slide, with the heading in a band at the bottom.
- **gallery**: the images in a grid (two side by side, three as two over one, four as 2x2, more
  in rows of three), each with its alt text as a caption.
- **quote**: the quotation large; the paragraph after it, or the quote's own short last
  paragraph, is the attribution (a leading `--` or `---` is dropped).
- **code**: the code block with its heading and short lead. Short code grows toward the body
  size and its box hugs its lines; code that does not fit shrinks (down to 40% of its size)
  before the slide scrolls; long lines shrink instead of wrapping.
- **visual**: the chart or diagram fills the space under its heading and lead; text written
  after it stays after it.
- **columns**: a leading H1 or H2 spans the columns; every `+++` starts the next column.
- **table**: the table with its heading and lead.
- **content**: everything in reading order. Two charts, a diagram with bullets, code with an
  image: all shown.

### 4.2 Choosing a design

When recognition gives the wrong design, choose one with the `design` setting:

```markdown
# Comparison
<!-- design: columns -->

Left column content...

+++

Right column content...
```

A chosen design that has no place for some of the slide's content shows the rest in its body, in
reading order; a chosen design that lacks its core block (`design: quote` on a slide without a
quote) draws the slide as `content`. `mdeck --check` reports both. An unknown name keeps the
recognised design and is reported too.

### 4.3 Overflow

Content that does not fit first shrinks: visuals and images in the copy (two charts on one
slide) share the height, down to 35% of their usual height, then code down to 40% of its size,
then prose and lists down to 80%. Past that the slide scrolls smoothly (Up and Down), with fade cues at the edges. The
height that decides scrolling is measured with the same layout that draws the slide.

---

## 5. Content Types

### 5.1 Headings

Standard ATX headings. Levels 1-3 are meaningful for design recognition; levels 4-6 are rendered as body-weight text.

```markdown
# Level 1: slide title or section
## Level 2: subtitle or subsection
### Level 3: minor heading within a slide
```

Optional closing hashes (`## Heading ##`) are stripped. Setext headings are
supported too: a one-line paragraph underlined with `===` is a level-1 heading
and one underlined with `---` is a level-2 heading. A `#` that is not followed
by a space (`#hashtag`, `#include`) is ordinary text, as in CommonMark.

### 5.2 Paragraphs and inline formatting

| Syntax                          | Result            |
|---------------------------------|-------------------|
| `**bold**` or `__bold__`        | **bold**          |
| `*italic*` or `_italic_`        | *italic*          |
| `***bold italic***`             | ***bold italic*** |
| `~~strikethrough~~`             | ~~strikethrough~~ |
| `` `inline code` ``             | `inline code`     |
| ``` ``code with ` inside`` ```  | code span with a backtick |
| `[text](url)`                   | hyperlink         |
| `$E = mc^2$`                    | inline math (see 5.9) |
| `$$\frac{a}{b}$$`               | display math (see 5.9) |
| `\*literal\*`, `\$5`            | backslash escapes |

Emphasis follows CommonMark flanking rules: `snake_case_name` and `5 * 3 * 2`
stay literal. Links are rendered visually but are not clickable during
presentation. Lines that continue a list item (indented or not) belong to that item.

Everything else from CommonMark and GitHub markdown either renders well or
degrades invisibly:

- **Reference links** (`[text][label]`, `[text][]`, `[label]` with
  `[label]: url` anywhere in the deck) resolve, and their definition lines are
  never shown.
- **Autolinks** (`<https://example.com>`, `<name@example.com>`) are links.
- **Footnotes**: markers are hidden and the footnote text goes to the slide's
  notes (section 3.4).
- **Raw HTML**: tags are removed and their text kept. `<img src>` becomes an
  image, `<h1>`..`<h6>` a heading, `<br>` a line break, and `<details>` /
  `<summary>` keep their text. HTML comments that are not settings
  (section 7) are skipped.
- **Indented code** (four spaces or a tab) is a code block.

`mdeck --check` names anything it cannot present (category `content`), such
as an image inside running text, a remote image or an HTML `<video>`.

### 5.3 Lists

Both ordered and unordered lists with nesting up to 3 levels. The `+` marker makes an item a step (see [section 6](#6-steps-incremental-reveal)). An ordered list counts from its first number, and task list items show their box.

```markdown
- First item
  - Nested item
    - Deep nested
- Second item

3. Third step
4. Fourth step
   1. Sub-step

- [x] Parser
- [ ] Presenter view
```

### 5.4 Images

Standard markdown image syntax:

```markdown
![Alt text](path/to/image.png)
```

By default an image fits the space its design gives it and keeps its aspect ratio. Size options
go in the alt text, in the settings grammar (`@key: value`; the space after the colon is
optional):

```markdown
![Architecture @width: 80%](arch.png)
![Logo @height: 100px](logo.png)
![Photo @fill](photo.jpg)
```

| Option          | Description                                     |
|-----------------|-------------------------------------------------|
| `@width: VAL`   | Width: a percentage of the image's space, or pixels at the 1920x1080 reference size (`400` or `400px`, scaled on other resolutions) |
| `@height: VAL`  | Height, in the same units; with both, the image keeps its aspect and fits both |
| `@fill`         | Cover the space, cropping, never stretching. On a `media` slide that is the whole slide, with the heading in a band at the bottom |

The options are taken out of the alt text, so `![Our team @fill](team.jpg)` has the alt text
"Our team"; on a standard markdown renderer the alt text only shows when the image is missing.
Where an image goes (beside the text, full bleed, in a grid) is up to the slide's design
(section 4), not an option. Any other `@` word in an image's alt text, including v1's `@fit`,
`@left`, `@right` and `@center`, is reported by `mdeck --check` (category `content`).

#### AI Image Generation

Use `generate:` as the image path to ask for a generated image:

```markdown
![A futuristic cityscape at sunset](generate:)
```

The alt text is the prompt. Leave it empty to have the chat model write one from the slide:

```markdown
![](generate:)
```

Run `mdeck ai images <file.md>` (or `mdeck ai <file.md>` for every kind of asset). The command:
- Picks the orientation from where the image goes (landscape for a slide of its own, portrait beside text)
- Applies the image style (`--style`, then `image-style`, then the config default, then the built-in one)
- Writes the image to `<deck>.assets/images/` and records it in `<deck>.assets/manifest.yaml`
  (section 9.7, "Generated assets"). The deck is never rewritten: the placeholder stays as it is
  and is resolved through the manifest when the deck opens, so it can be regenerated at any time.

Until it is generated, a placeholder shows as a quiet card with its prompt, and `mdeck --check`
reports it (category `assets`).

### 5.5 Code blocks

Standard fenced code blocks with optional language and line highlighting:

````markdown
```rust
fn main() {
    println!("Hello, world!");
}
```
````

Line highlighting uses `{lines}` notation after the language:

````markdown
```rust {3,5-7}
fn main() {
    let pool = Pool::new(10);
    pool.connect();           // highlighted
    let result = pool
        .query("SELECT")     // highlighted
        .fetch()              // highlighted
        .unwrap();            // highlighted
    println!("{:?}", result);
}
```
````

The `{...}` is parsed as comma-separated line numbers and ranges (e.g., `3`, `5-7`). Highlighted lines receive a distinct background. Code blocks without a language identifier render as plain monospace text with no highlighting.

Code shrinks to fit. When a slide's code blocks are taller than the slide, or a line is wider than the column, the code font is reduced until everything fits, down to 40% of the theme's code size (about 65 lines on a 16:9 slide). Only past that does the slide scroll. Long lines therefore shrink rather than wrap, and a PNG export shows the whole block. Short code grows instead, toward 90% of the body size, while its lines and the slide have room. Code shows its characters as written: the monospace font's ligatures are off, so `---`, `->` and `<!--` stay literal.

### 5.6 Blockquotes

Standard markdown blockquotes:

```markdown
> This is a quotation that will be
> rendered prominently on the slide.
```

A quote keeps its paragraphs, each starting a new line inside the quote's
accent bar. A list inside a quote is drawn as a list, and a nested quote
(`> > text`) as a quote within the quote: indented, with a bar of its own.
A quote with a list or a nested quote is set flush left. A quote of
several paragraphs whose last paragraph is short (80 characters or fewer)
ends in its attribution, set apart under the quotation:

```markdown
> The negative is the score, the print is the performance.
>
> Ansel Adams
```

**GitHub alerts** render as callouts: a tinted panel with the alert's label.

```markdown
> [!WARNING]
> Back up the database before migrating.
```

The alerts are `[!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]` and `[!CAUTION]`.

### 5.7 Tables

Standard pipe-delimited tables:

```markdown
| Feature   | Status  |
|-----------|---------|
| Parsing   | Done    |
| Rendering | WIP     |
```

Tables are rendered with theme-appropriate styling. A heading with one table (and at most a short paragraph) is a `table` slide; anywhere else a table is a block in the slide's design. The second line must be a separator row (`|---|`); its colons set each column's alignment (`|:--|` left, `|:-:|` centre, `|--:|` right). Escape a pipe inside a cell as `\|`; pipes inside inline code are kept as text. A lone `| text |` line without a separator row is a paragraph.

### 5.8 Horizontal rules within slides

Since `---` is reserved for slide breaks, use `***` or `___` for a visual rule within a slide:

```markdown
# Timeline

Phase 1: Research

***

Phase 2: Implementation
```

### 5.9 Math (LaTeX)

Formulas use LaTeX syntax between dollar signs, anywhere text goes: paragraphs,
list items, headings, blockquotes and table cells.

```markdown
The roots of $ax^2 + bx + c = 0$ are

$$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$$

- Einstein: $E = mc^2$
```

- `$...$` is inline math, typeset in text style on the line's baseline.
- `$$...$$` is display math: centred on a line of its own, in display style
  (larger fractions, limits above and below sums). It may span several lines of
  the paragraph, but not a blank line.
- The supported LaTeX is KaTeX's: fractions, roots, sub- and superscripts, sums,
  integrals and limits, Greek letters, `\mathbb`, `\mathcal` and other fonts,
  matrices (`pmatrix`, `bmatrix`, ...), `cases`, `aligned`, accents (`\vec`,
  `\hat`, `\overrightarrow`), braces (`\overbrace`), `\left`/`\right`
  delimiters and `\text{...}`.
- Formulas are drawn with the bundled KaTeX fonts in the slide's text colour,
  sharp at any resolution. A formula wider than its column shrinks to fit.
- Dollar amounts stay text: `$` followed by a space never opens a formula, and
  `$` followed by a letter or digit never closes one, so `$5 and $10` and
  `Revenue ($K) and cost ($M)` render as written. Write `\$` for a literal
  dollar sign anywhere else.
- A formula that does not parse is shown as its source, and `mdeck --check`
  reports it (`math` category) with the parser's reason.
- Math is not rendered inside code, charts or diagrams.

---

## 6. Steps (incremental reveal)

A slide can reveal its content one press at a time. The list marker says what is a step:

| Marker      | Behavior                                                    |
|-------------|-------------------------------------------------------------|
| `-` or `*`  | Static: visible when the slide appears                      |
| `+`         | A step: appears on the next forward press, with its children |

### 6.1 In slide lists

```markdown
# Key Points

- Always visible context
+ First reveal
+ Second reveal
  - shown with the second reveal
  - and so is this
+ Third reveal
```

Presentation behavior:
1. Slide appears with "Always visible context" shown
2. Forward press: "First reveal" appears
3. Forward press: "Second reveal" and its two children appear together
4. Forward press: "Third reveal" appears
5. Forward press: advance to next slide

To reveal several items in one step, nest them under a `+` item. A nested `+` item takes a step of its own.

### 6.2 In visuals

The same markers control the items of charts and diagrams (section 8, section 14).

### 6.3 Rules

- Steps are counted across the whole slide in reading order. The second `+` list on a slide continues after the first, and a chart's steps follow the steps before it.
- On a slide with steps, pressing forward reveals the next step rather than advancing to the next slide. Only after all steps have been revealed does forward advance to the next slide.
- Hidden items keep their space: revealing never moves what is already on screen, and revealed items slide and fade in.
- Ordered list items (`1.`, `2.`, etc.) are always static.
- `reveal: none` in the frontmatter turns steps off for the deck (a slide's own `reveal: steps` turns them back on there), so imported markdown that happens to use `+` bullets presents without clicks.

---

## 7. Settings

Everything mdeck adds on top of markdown is a **setting**, written `key: value`
with the same key names wherever it appears:

- in the frontmatter, a setting is the **deck's value** (section 2.1);
- in a slide's **settings comment**, it is **that slide's value**, overriding the deck's.

### 7.1 Slide settings

Slide settings are written in an HTML comment inside the slide, invisible on
any markdown renderer. The comment holds one or more `key: value` lines:

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

A comment whose first line is `key: value` with a known setting key is a
settings comment, and every line in it is a setting. Any other comment is an
ordinary comment. A settings comment applies to the slide it is in, wherever
in the slide it is written, and never moves to another slide (section 3.3).
If a setting is written twice on one slide, the last one wins.

`mdeck --check` reports unknown keys (with a "did you mean"), invalid values,
deck settings written in a slide (and slide settings in the frontmatter),
duplicates, lines in a settings comment that are not `key: value`, and
ordinary comments that look like a misspelt setting. It also recognises v1
syntax and names the v2 form, for example
`slide 4 (line 37): [settings] "@layout: quote" is v1 syntax; write <!-- design: quote -->`.
Each warning names the slide and the line in the file. `mdeck --check -v`
lists, per slide, the settings that apply to it.

### 7.2 Fences

mdeck's own fenced blocks carry an `@` tag on the info string: the visuals
(sections 8 and 14) and notes (section 3.4). Tags match exactly; each kind
has exactly one name.

````markdown
```@architecture
# diagram content (section 8)
```
````

An `@` tag that names no mdeck fence shows as a code block, and `mdeck --check`
reports it (category `visual`) with a suggestion.

### 7.3 Setting reference

These tables are generated from mdeck's language table, so they always match
what the parser accepts.

<!-- generated: settings -->

---

## 8. Diagram Syntax

Architecture diagrams use a fenced block tagged `@architecture`. Like every visual, the block follows the visual grammar of section 14.1: settings first, then list items, `#` for comments.

### 8.1 Basic form

In the simplest form, just write relationships; components are inferred:

````markdown
```@architecture
- User -> Server: Sends request
- Server -> Database: Queries data
- Server -> User: Returns response
```
````

### 8.2 Full form

For explicit layout, icons, and stepped reveal:

````markdown
```@architecture
# Components
- User        (icon: user,      pos: 1,1)
- Server      (icon: server,    pos: 2,1)
- Database    (icon: database,  pos: 2,2)
- Log Service (icon: logs,      pos: 3,1)

# Relationships
- Server -- Log Service: sends logs to
+ User -> Server: Sends request
+ Server -> Database: Queries data
+ Database -> Server: Returns results
```
````

In this example:
- All four components and the logging relationship are visible from the start (`-`)
- Each forward press reveals the next `+` relationship

### 8.3 Components

```
- Name (key: value, key: value)
- Name: Display label (key: value)
```

The text after `:` is the label drawn on the component (the name is used when there is none); relationships refer to the component by its name.

| Key      | Values                          | Default       | Description           |
|----------|---------------------------------|---------------|-----------------------|
| `icon`   | icon name (section 8.8)         | `box`         | Visual icon           |
| `pos`    | `x,y` (integer grid coords)     | auto-layout   | Position hint         |
| `prompt` | quoted string                   | none          | AI icon generation prompt |

Use `icon: generate:` to ask for a generated icon; `prompt` says what to draw (the node's label when
it is left out):

```
- Gateway (icon: generate:, prompt: "An API gateway router icon", pos: 1,2)
```

Run `mdeck ai icons <file.md>` to generate them into `<deck>.assets/icons/`, recorded in the deck's
manifest. The line stays as written; until its icon exists the node shows the generic icon and
`mdeck --check` reports it.

If no components are explicitly declared, they are inferred from relationship lines. Each unique name becomes a component with default icon and auto-positioned layout.

### 8.4 Relationships

```
- Source -> Target: Label
```

Arrow types (written with a space on each side):

| Arrow   | Meaning                    |
|---------|----------------------------|
| `->`    | Solid arrow (directed)     |
| `<-`    | Reverse solid arrow        |
| `<->`   | Bidirectional solid arrow  |
| `--`    | Dashed line (undirected)   |
| `-->`   | Dashed arrow (directed)    |

The text after `:` is the label. If no `:` is present, the relationship has no label.

### 8.5 Settings and comments

| Setting | Values | Default | Description |
|---------|--------|---------|-------------|
| `scale` | `fit`, `scroll`, or a factor such as `0.7` (0.1 to 2) | `fit` | `fit` scales the diagram to the slide; a factor draws it at that size; `scroll` keeps it at full size and scrolls when it is taller than the slide |

Lines starting with `#` are comments. They are ignored but help organize the source, as `# Components` does above.

### 8.6 Layout algorithm

The `pos: x,y` values are relative grid coordinates:
- `1,1` is the top-left of the diagram area
- Higher x moves right; higher y moves down
- The grid auto-scales to fill available space
- Without any `pos`, components are placed in source order: in one row up to five, then in a near-square grid
- When some components have a `pos`, the others fill the free grid cells row by row

### 8.7 Diagram type qualifier

There is none: `@architecture` draws component diagrams only. Other diagram kinds are their own visuals (section 14).

### 8.8 Built-in icons

The built-in themes provide these icon names:

`user`, `server`, `database`, `cloud`, `browser`, `mobile`, `api`, `queue`, `cache`, `storage`, `function`, `container`, `network`, `lock`, `key`, `mail`, `logs`, `monitor`, `team`, `package`, `code`, `box`

An unrecognized icon name falls back to `box`. Icons are simple and clear line drawings, designed to be recognizable at presentation scale.

---

## 9. Theme System

### 9.1 Built-in themes

The built-in themes come in two tiers, in this order in `mdeck theme list` and
`Shift+T`:

- **Themes:** `dark` (the default: plain, dark, a bright foreground, standard
  designs, fades, no countdown), `light`, `nord`, `ember`, `thermal`,
  `marquee`, `departures`, `stack`, `blueprint`, `chalkboard`, `sketchbook`,
  `watercolour`, `darkroom`.
- **Variants** (recolourings, `variant-of:`): `spring` and `summer` of
  `light`, `autumn` and `winter` of `ember`.

A deck that names no theme (and no `defaults.theme` in the config) gets `dark`.

**`light`**

| Property        | Value           |
|-----------------|-----------------|
| Background      | `#FFFFFF`       |
| Primary text    | `#1A1A2E`       |
| Heading text    | `#16213E`       |
| Accent          | `#0F3460`       |
| Code background | `#F5F5F5`       |
| Quote border    | accent color    |

**`dark`**

| Property        | Value           |
|-----------------|-----------------|
| Background      | `#1E1E1E`       |
| Primary text    | `#C8C8C8`       |
| Heading text    | `#FFFFFF`       |
| Accent          | `#5294E2`       |
| Code background | `#2D2D2D`       |
| Quote border    | accent color    |

**`nord`**

| Property        | Value           |
|-----------------|-----------------|
| Background      | `#2E3440`       |
| Primary text    | `#D8DEE9`       |
| Heading text    | `#ECEFF4`       |
| Accent          | `#81A1C1`       |
| Code background | `#3B4252`       |
| Quote border    | accent color    |

**`spring`**, **`summer`**, **`autumn`** and **`winter`**

Four seasonal themes, written as plain theme files (section 9.4) like every
built-in:

| Theme | Feel | Engine | Background | Accent |
|---|---|---|---|---|
| `spring` | cherry blossom and new leaves on a pale morning | plain | `#F7FAF2` | `#C73E6B` |
| `summer` | sun on sand and a deep blue sea | plain | `#FFFAF0` | `#0072B1` |
| `autumn` | dark bark and maple amber, the particle field glowing in amber, rust and gold | particles | `#17110D` | `#E0782A` |
| `winter` | a clear night over snow, the particle field glowing ice-blue and white | particles | `#0A1220` | `#7CC4FA` |

**`ember`**

| Property        | Value           |
|-----------------|-----------------|
| Background      | `#050505`       |
| Primary text    | `#B4B4BC`       |
| Heading text    | `#ECECEF`       |
| Accent          | `#FF4D1C`       |
| Code background | `#101012`       |
| Quote border    | accent color    |

Ember is MKLab's brand theme and goes further than a palette: it runs on the
**particles engine** (section 9.6), which any custom theme can use too. It bundles its
own typefaces (Spectral for headings, Hanken Grotesk for copy, JetBrains Mono
for labels), lays text slides out as a copy column on the left, and draws a
living field of glowing particles behind every slide. The field morphs from
slide to slide and follows the content: a title slide opens on a constellation
(after the particles assemble into the logo), a points slide lights one
cluster per item as the items reveal, a quote slide burns like a candle, a
code slide rains. Ember uses the editorial design set (section 9.9), which
arranges every design, code, tables, charts and images included, with an
eyebrow, display type and a staggered entry.

**`marquee`**

A wall of RGB LEDs in a dark room, on the **LED engine** (section 9.6):
near-black with the faint grid of unlit lenses, hot pink and cyan light,
warm amber bulbs chasing around the title slide, and Hanken Grotesk for the
copy. Point clouds, the countdown digits and the end words appear by
lighting LEDs.

| Property        | Value           |
|-----------------|-----------------|
| Background      | `#06070B`       |
| Primary text    | `#B9BCCB`       |
| Heading text    | `#F6F6FB`       |
| Accent          | `#FF2D78`       |
| LED gradient    | `#FF2D78` → `#FF9CC2` → `#35D6FF` |
| Marquee bulbs   | `#FFC043`       |

**`departures`**

A split-flap departure board in a station hall, on the **split-flap engine**
(section 9.6): charcoal flaps with warm white capitals, headings in the
yellow of a timetable, `**bold**` in pale yellow, `` `code` `` in blue, and
the board as the whole slide.

| Property        | Value           |
|-----------------|-----------------|
| Background      | `#0E0F11`       |
| Flaps           | `#1D1E22`       |
| Characters      | `#F2EDE0`       |
| Headings        | `#FFC21A`       |

**`stack`**

A theme for the blocks engine (section 9.6):

| Theme | Feel | Engine | Background | Accent |
|---|---|---|---|---|
| `stack` | a night-blue playfield where pictures are built from bright bevelled blocks in pink, yellow, cyan, orange and violet | blocks | `#0D0E1A` | `#FF4F8B` |

**`blueprint`**

A draftsman's sheet for the line engine (`engine: { name: line, surface: sheet }`, sections 9.6 and 9.7): a
Prussian blue sheet with a fine grid, a ruled border and a title block, laid
on a dark drafting table (`page:`), with generated line art inked in
blue-white, construction lines first.

| Property        | Value           |
|-----------------|-----------------|
| Sheet           | `#123A6E`       |
| Drafting table  | `#0A1B33`       |
| Ink (headings, drawings) | `#F4F8FF` |
| Grid and construction lines | `#8FB4E3` |
| Accent (a yellow pencil mark) | `#FFD166` |

**`sketchbook`**

A sheet of drawing paper on a grey desk for the sketch engine: generated
graphite and ink drawings in the MKLab house style (old craft meeting modern
technology), drawn in with a pencil; an editorial serif for headings.

| Property        | Value           |
|-----------------|-----------------|
| Paper           | `#F5F0E6`       |
| Desk            | `#44464B`       |
| Headings (graphite) | `#1F1D1A`   |
| Accent (a red pencil) | `#A8472A` |
| Secondary (a blue pencil) | `#35657F` |
| The pencil (`engine.cool`) | `#2F5D50` |

**`chalkboard`**

A green slate in a wooden frame for the line engine (`engine: { name: line, surface: slate }`), with the ghosts
of earlier drawings wiped off it; generated line art is drawn in chalk.

| Property        | Value           |
|-----------------|-----------------|
| Slate           | `#26352F`       |
| Frame           | `#6E4A2C`       |
| White chalk (headings, drawings) | `#F6F6EF` |
| Yellow, pink and blue chalk (`accent`, `accent-soft`, `secondary`) | `#F3D46B`, `#F2A7B4`, `#9FD3E6` |

**`watercolour`** and **`darkroom`**

| Theme | Feel | Engine | Background | Accent |
|---|---|---|---|---|
| `watercolour` | cold-press paper on a pale table (`page:`), sepia-grey ink, an editorial serif; generated watercolours bloom onto the paper | watercolour | `#FBF8F1` | `#C8553D` |
| `darkroom` | a darkroom under a red safelight (`accent`); generated black-and-white photographs develop as prints | darkroom | `#141011` | `#FF4B3A` |

**`thermal`**

| Theme | Feel | Engine | Background | Accent |
|---|---|---|---|---|
| `thermal` | the deck seen through a thermal instrument: cold indigo-black, white-hot headings, iron orange; headings form in heat on title and section slides | thermal | `#05030D` | `#F37A0C` |

Thermal extends Ember's editorial design set and type. It pairs with `@thermal`
blocks for thermal images (section 14.20); the engine is in section 9.6.

Every theme can draw the symbols text faces usually lack: circled numbers and letters (①②③, ⓐ), check marks, arrows, geometric shapes and stars. Noto Sans Symbols and DejaVu Sans are bundled as the last fallback of every font family, so such characters never render as boxes.

Chinese, Japanese and Korean text draws with a font borrowed from the system, since a CJK face is too large to bundle: PingFang, Hiragino or Arial Unicode on macOS, Microsoft YaHei, Yu Gothic or Malgun Gothic on Windows, Noto Sans CJK, WenQuanYi or Droid Sans Fallback on Linux (`fonts-noto-cjk` on Debian and Ubuntu). It is added after the symbol faces in every family, on the family's own baseline. To use a specific file, set `MDECK_CJK_FONT=/path/to/font.ttc`. When no font on the machine covers a script the deck uses, `mdeck --check` warns (`fonts` category) and presenting or exporting prints the same warning; the text then draws as boxes.

On the particles engine, every slide but the title has space behind it.
Slides walk through four backdrops by number, so neighbours never share one:
a star field drifting slowly forward, soft dust, a galaxy of two spiral arms
turning about the centre, and a nebula of large clouds out of focus. A points
slide's item clusters likewise take a different formation on each slide. On
the editorial design set every slide gets a tracked eyebrow with its Roman
numeral and the deck title, and the chrome is a counter and a progress
hairline.

#### The countdown

Every built-in theme except `dark`, `light`, `spring` and `summer` opens
with a three-second countdown before the first slide. Any theme can, with
`countdown: on` (section 9.4), and a deck's `countdown: on|off` wins over the
theme's. The engine decides how it looks: an engine with a countdown of its
own draws it (section 9.6), any other shows plain numerals that fade. In
Ember the particles form the digits 3, 2 and 1 in the display face, morph
from one to the next, and the 1 bursts outward into black before the first
slide's scene assembles. Any key or click cancels it, starting on a chosen
slide (`--slide`, `--overview`) skips it, and reduced motion leaves it out.

#### Pictures

A slide's **picture** is what the engine draws on the design's stage. Ask for
one with the `picture` setting. Its value resolves in this order:

1. on an art engine (section 9.7), the slide's generated artwork, when there
   is one;
2. a **point cloud** of that name: a named file of points that the particles
   settle into, the LEDs light up, the blocks build or the pencil traces;
3. an image file at that path, relative to the deck (`.png`, `.jpg`, `.jpeg`,
   `.webp` or `.svg`, as in `<!-- picture: images/team.jpg -->`). mdeck draws
   it on the stage itself, framed beside the copy or large and dim behind a
   title, so it shows on every engine but the split-flap board (which draws
   the whole slide); the engine sees where it is and keeps clear of it.

```markdown
## Our new server
<!-- picture: server -->

- 5 TB of RAM
- 100 cores
```

Where it goes is the design's stage (section 9.9). In the editorial set,
statement, points, quote, section and text-only content slides show it on
the right, beside the copy, warm and lit from the first step. Title slides
put it behind the centred copy, large, dim and slow: a backdrop rather than
a picture. Split, media, gallery, code, visual, columns and table slides
never show one. The standard set has no stage until a slide sets a
picture: then statement, points, quote, section and text-only content
slides open one on the right and move their copy into the left column, and
a title puts it behind the copy (the arrangement's `with-picture`). Every
engine except `plain` and `splitflap` draws point clouds and artworks, each
in its own medium (section 9.6); on `plain`, mdeck draws a point cloud
itself as a stipple of dots in the theme's accent. An image file shows on
every engine but `splitflap`. `mdeck --check` warns when a slide asks for a picture its
design or engine cannot show, or one that does not exist. `picture: none`
keeps a slide's stage empty.

A point cloud name resolves through these places, first match wins:

1. the deck's generated point clouds, `<deck>.assets/point-clouds/` (made by
   `mdeck ai point-cloud`, section 9.7);
2. `point-clouds/<name>.mdpc` next to the deck;
3. the user library: `point-clouds/` in the user folder
   (`~/.config/mdeck/` on Linux and macOS, or `$XDG_CONFIG_HOME/mdeck/` when
   set, `%APPDATA%\mdeck\` on Windows);
4. the `point-clouds/` folders of installed packs (section 18);
5. the set built into mdeck.

So a deck can carry its own clouds, a user can keep favourites across decks,
and either can shadow a built-in by using the same name. Names are lowercase letters, digits
and hyphens. The built-in set: `person`, `hooded`, `man`, `woman`,
`thermographer`, `presenter-up`, `presenter-down`, `box`, `orb`, `doc`,
`docs`, `inbox`, `db`, `cloud`, `laptop`, `folder`, `mail`, `gate`,
`server`, `robot`, `agent-friendly`, `agent-evil`, `ai`, `phone`, `globe`,
`lock`, `gear`, `rocket`, `punchcard`, `camera`, `gauge`, `glasses`, `eyes`,
`flag`, `blackhole`, `account` (a bank building, for finance topics),
`lightbulb` (ideas), `question` (a question mark).

New clouds come from the image model or from any image of light strokes on a
dark ground:

```bash
mdeck ai point-cloud talk.md     # every picture name the deck uses that resolves nowhere
mdeck ai point-cloud --name server --description "A server rack in a datacenter"
mdeck point-cloud import sketch.png --name sketch
mdeck point-cloud list          # every name visible from here, and what shadows what
mdeck point-cloud show server   # a preview image
```

`mdeck ai point-cloud` asks the configured image provider for a sparse
constellation of glowing particles forming the subject, then reduces the
image to a cloud. Given a deck, it writes every missing name into
`<deck>.assets/point-clouds/`; with `--name`, and `mdeck point-cloud import`,
it writes `./point-clouds/<name>.mdpc` (`--user` writes to the user library
instead, and `--force` overwrites an import). A cloud is JSON: a name, a
description, the prompt that made it, the bounding box's height over width,
and up to 1500 points in the unit square, stored in **importance order** so
that the first sixty points already sketch the whole subject and the first
six hundred fill it in. The field takes as many as it has particles to
spend, which is why the same file serves a small stage picture and a
full-frame backdrop, and why a point cloud hints at its subject rather than
copying it.

A cloud worth sharing can be offered to the built-in set without installing
anything: `mdeck point-cloud contribute <name>` writes a `.json` copy GitHub
accepts as an attachment and opens a new issue on the MDeck repository with
the name, description, prompt and a braille sketch filled in. Drag the file
onto the issue and submit; if it is accepted it ships as a built-in in the
next release.

All themes meet WCAG AA contrast requirements. Cycle themes during a
presentation with `Shift+T`.

### 9.2 Theme properties

A theme is data: colours, type roles, sizes and a syntax theme, plus the
**engine** that draws it (section 9.6). The built-in themes are written in
exactly the format of section 9.4, which lists every key.

### 9.3 One theme per deck

The theme is a deck setting: `theme` in a slide is reported by `--check` and
ignored. Use `theme` in the frontmatter, or `Shift+T` while presenting. To
change a slide's look, give it its own background image (section 9.8).

### 9.4 Custom themes

A custom theme is a YAML file. Put it next to a deck or in your user folder,
name it in the frontmatter, and it works like a built-in one: when presenting,
in `Shift+T` cycling, in PNG and PDF export, and in `--check`. Themes are data
only; a theme can never make MDeck run code.

```markdown
---
theme: acme
---
```

**Where themes live.** A theme is `<name>.yaml` or `<name>/theme.yaml` (a
folder, so it can carry font files). MDeck looks in this order and the first
match wins:

1. `themes/` next to the deck
2. the user folder: `~/.config/mdeck/themes/` on Linux and macOS (or
   `$XDG_CONFIG_HOME/mdeck/themes/` when set), `%APPDATA%\mdeck\themes\` on Windows
3. the `themes/` folders of installed packs (`mdeck pack`)
4. the built-in themes (section 9.1)

`theme` may also be a path to a file (`theme: brand/acme.yaml`), relative
to the deck. A user or deck theme may reuse a built-in name to replace it.
Theme names are lowercase letters, digits, `-` and `_`. `defaults.theme` in
the config accepts built-in and user themes. An unknown name or an invalid
theme file is reported (by `--check`, and as a warning when presenting or
exporting) and the deck falls back to `dark`.

**The format.** Every key is optional. Unset keys come from the theme named by
`extends`, which is `dark` when the file does not say. So a brand theme can be
ten lines:

```yaml
# themes/acme.yaml
name: Acme                 # display name (default: the file name)
extends: dark              # any built-in or custom theme
colors:
  background: "#0b1020"
  text: "#c9d1e3"
  heading: "#ffffff"
  accent: "#ffb400"
```

The full set of keys:

```yaml
name: Acme
extends: dark
variant-of: dark           # this theme recolours another: listed with the variants
engine:                    # the engine and its settings (section 9.6); `engine: plain` names one alone
  name: thermal            # plain | particles | led | splitflap | blocks | line | sketch | watercolour | darkroom | thermal
  palette: iron            # thermal: iron | white-hot | black-hot | rainbow | arctic | lava
  drift: false             # thermal: true lets embers drift through the dark on ordinary slides
  light: "#d7d7e1"         # particles, led, blocks, line, sketch: the brightest tint
  cool: "#afc3f0"          # particles, led, blocks, line, sketch: a cool tint besides the accents
  surface: sheet           # line: sheet | slate
  kind: line               # art engines: line | tonal (section 9.7)
  style: "graphite and ink, cross-hatching"   # art engines: the style prompt
  references: [refs/teacup.jpg]               # art engines: style swatches in the theme folder
designs: standard          # standard | editorial | a set in designs/ | a code design set: how the slide designs look (section 9.9)
arrangements: {}           # per-design overrides of the design set (section 9.9)
countdown: off             # on | off: the 3-2-1 opener (the engine decides its look)
transition: fade           # slide | fade | spatial | none | a registered one (a deck's `transition` wins)
spacing:                   # the gaps the designs use, px on a 1920x1080 slide
  xs: 8
  sm: 16
  md: 24
  lg: 40
  xl: 64
radius: 8                  # corner radius of code blocks, tables and callouts, px
colors:                    # #rgb, #rrggbb or #rrggbbaa
  background: "#0b1020"    # slide background
  text: "#c9d1e3"          # body text
  heading: "#ffffff"       # headings
  muted: "#7d869c"         # captions, eyebrows, slide numbers (default: text toward background)
  strong: "#ffffff"        # **bold** text (default: heading, or accent on light themes)
  rule: "#232a3d"          # hairlines and inactive markers (default: derived)
  accent: "#ffb400"        # links, quote bars, highlights, the one thing to look at
  accent-soft: "#ffd166"   # lighter accent for emphasis and glows (default: derived)
  secondary: "#ff7a59"     # a second, rarer highlight (default: accent-soft)
  code-background: "#141a2e"
  code-text: "#d6dcf0"
  positive: "#3ecf8e"      # KPI trend up
  negative: "#ff5c5c"      # KPI trend down
  series:                  # charts and diagrams, in order; 1 to 8 colours, cycled
    - "#ffb400"
    - "#4ea8ff"
    - "#3ecf8e"
annotations:               # the presenter's pen (drag) and arrow (right drag) tools
  pen: "#50c8ff"
  pen-outline: "#1e82b4"
  arrow: "#ffc832"
  arrow-outline: "#c88c00"
fonts:                     # a bundled face, or a .ttf/.otf file in the theme folder (or a pack's fonts/)
  display: fonts/Acme-Display.ttf   # headings
  body: hanken-regular     # body text and list items
  lead: hanken-light       # lead paragraphs where the design set uses it (editorial; default: body)
  strong: hanken-medium    # **bold** runs (default: body)
  mono: jetbrains-mono     # code, labels, eyebrows
sizes:                     # px on a 1920x1080 slide; everything scales with the window
  h1: 96
  h2: 72
  h3: 52
  body: 44
  code: 30
text:
  line-height: 1.45        # multiple of the font size (default: the face's own)
charts:
  fill-opacity: 0.85       # opacity of filled chart shapes, 0 to 1
code:
  syntax: base16-ocean.dark   # a bundled syntax theme, or a .tmTheme file in the theme folder
logo:                      # a logo in a corner of every slide (section 9.5)
  file: logo.svg           # PNG (with transparency) or SVG, in the theme folder
  position: top-right      # top-left | top-right | bottom-left | bottom-right
  height: 56               # px on a 1920x1080 slide
  opacity: 0.6             # 0 to 1
page:                      # lay every slide on a sheet with a surface around it
  surface: "#0a1b33"       # what is around the sheet (a desk, a drafting table)
  margin: 26               # px around the sheet on a 1920x1080 slide (0 to 300)
  shadow: 0.7              # the sheet's shadow on the surface, 0 to 1 (default 0.5)
  grain: 0.25              # paper fibre on the sheet, 0 to 1 (default 0.5)
  radius: 2                # corner radius in px (0 to 60, default 6)
```

Unknown keys are errors, so a typo never goes unnoticed. The v1 top-level
`particles:`, `heat:`, `art:` and `surface:` sections moved into the engine
block; a theme that still has one gets an error saying where it went.

**The engine block.** `engine:` is the engine's name, or a block with `name`
and the engine's settings. Settings merge key by key through `extends` while
the child names the same engine (or none); a child that names a different
engine starts from its own settings only. When a deck's `engine` or
`--engine` runs another engine, the theme's settings are ignored.
`mdeck theme check` warns about a setting the theme's engine does not read.

**Variants.** `variant-of: <theme>` marks a theme as a recolouring of
another (it normally also `extends` it). Variants are listed after the
themes in `mdeck theme list` and `Shift+T`. It is the theme's own key: a theme
extending a variant is not a variant unless it says so.

**Spacing and radius.** `spacing` is the scale of gaps the slide designs
name (`xs` to `xl`, growing), and `radius` the corner radius of cards. Change
them to make every design airier or tighter at once.

**Checking a theme.** `mdeck theme check <name>` reports errors, fallbacks,
contrast below WCAG AA (4.5:1 for small text, 3:1 for large text of 24 px or
more) for every text colour the theme draws (body text, headings, bold,
links, muted captions and eyebrows, code on its background, the soft accent
where the design set draws emphasis in it, and every role of every design at
its size and opacity), and keys that do nothing: an engine setting the engine
does not read, `fonts.lead` when no design uses the lead face, and an engine
that draws on a page (line, sketch, watercolour) without a `page:`. It exits
non-zero on an error or a contrast failure, so a theme can be checked in CI.

**The page.** With a `page:` block the slide is a sheet (the theme's
`background`) lying on `surface`, with a soft shadow and a fine paper grain.
Everything on the slide, the engine's layer included, draws on the sheet.
`surface` is required for a page; the other keys have defaults. It works on
every engine.

**Art.** On an art engine (section 9.7) the engine block sets the house style
of the deck's generated pictures. `kind` overrides the medium's own kind,
`style` replaces the medium's style prompt (the part that carries the look),
and `references` replaces the medium's bundled style swatches with your own
images (paths inside the theme folder; sent to image models that accept
reference images). Pictures made in one style are never shown in another.

**Engines.** The engine decides what a theme does beyond colours and type
(section 9.6). A theme picks one with `engine:` (a name or a block with its
settings); a deck can run on another with `engine`.

**Fonts.** Fonts are named by *role*, not weight, because a slide draws each
role with one face. A value is either a bundled face or a TTF/OTF file inside
the theme's folder (WOFF and WOFF2 web fonts do not work; download the TTF or
OTF from the type foundry or Google Fonts). The bundled faces:

| Name | Face |
|---|---|
| `sans` | egui's default sans (Ubuntu Light) |
| `mono` | egui's default monospace (Hack) |
| `spectral-light` | Spectral Light, an editorial serif |
| `hanken-light` | Hanken Grotesk Light |
| `hanken-regular` | Hanken Grotesk Regular |
| `hanken-medium` | Hanken Grotesk Medium |
| `jetbrains-mono` | JetBrains Mono Regular |

A font file that is missing or outside the theme folder falls back to the
inherited face with a warning. A file that is there but is not a usable
TTF/OTF font falls back to the role's default with a warning: the `body` face
for `display`, `lead` and `strong`, `sans` for `body`, `mono` for `mono`.

**Syntax themes.** `code.syntax` names one of the bundled syntax themes
(`base16-ocean.dark`, `base16-eighties.dark`, `base16-mocha.dark`,
`base16-ocean.light`, `InspiredGitHub`, `Solarized (dark)`,
`Solarized (light)`) or a `.tmTheme` file in the theme folder.

#### From a design system to a theme

Design systems (a Claude Design export, CSS custom properties, a Tailwind
config, W3C design tokens, Figma variables) describe web interfaces, not
slides; none of them says which colour is body text on a slide or in what
order chart series come. Map their roles like this:

| Design system role | Theme key |
|---|---|
| page or base surface (`surface.base`, `--bg`, `background`) | `colors.background` |
| primary text (`text.primary`, `foreground`) | `colors.heading` |
| a step quieter than primary text (`text.secondary`) | `colors.text` |
| tertiary or muted text, captions | `colors.muted` |
| hairline or subtle border | `colors.rule` |
| primary accent, brand colour, primary action | `colors.accent` |
| lighter accent tint (the 300 step) | `colors.accent-soft` |
| secondary brand colour | `colors.secondary` |
| raised surface or card | `colors.code-background` |
| text on that surface | `colors.code-text` |
| success / positive status | `colors.positive` |
| error / critical status | `colors.negative` |
| data-visualisation or categorical palette (accent first) | `colors.series` |
| display or heading family | `fonts.display` |
| body or UI family (regular weight) | `fonts.body` |
| body family, light weight | `fonts.lead` |
| body family, medium or semibold weight | `fonts.strong` |
| monospace family | `fonts.mono` |

Then decide what the web system cannot tell you:

- **Body text is quieter than headings.** On a slide the body copy is one step
  below the brightest text, so it does not compete with the title.
- **Sizes are for a room, not a page.** Keep the built-in slide sizes
  (`extends` gives them to you) unless the brand has a reason; web type scales
  (15 px body) are far too small.
- **Resolve every alias.** `var(--ink-950)` becomes the hex it points to;
  `rgba(...)` becomes `#rrggbbaa`.
- **Fonts become files or bundled faces.** Use a bundled face when the design
  system names the same family (Spectral, Hanken Grotesk, JetBrains Mono);
  otherwise put TTF/OTF files in the theme folder.
- **Pick the engine.** A dark, atmospheric brand can use `engine: particles`
  (glowing particles) or `engine: led` (an LED wall) with `countdown: on`;
  most brands want `plain`.
- **Check contrast.** `mdeck theme check` flags text that is hard to read.

Tools for the loop of converting, looking and adjusting:

| Command | What it does |
|---|---|
| `mdeck theme list` | every theme visible from here, and where it comes from |
| `mdeck theme new <name>` | writes a commented starter theme to `themes/<name>.yaml` |
| `mdeck ai theme <name> --from <dir>` | reads a design system folder (`SKILL.md`, `readme.md`, CSS tokens, `*.tokens.json`, Tailwind config) and writes the theme with AI (see `mdeck ai`); font files and logos (PNG or SVG files with "logo" in their path) found there are copied into the theme folder |
| `mdeck theme check <name>` | reports errors, fallbacks and weak contrast |
| `mdeck theme preview <name> --output-dir <dir>` | exports a sampler deck in the theme, one slide per design, as PNGs to look at |

`new` takes `--user` to write to the user folder and `--force` to overwrite.
`mdeck export <deck> --theme <name>` renders any deck in a theme without
editing it.

**For AI agents converting a design system:** read the design system's rules
and tokens, write `themes/<name>.yaml` with the mapping above, run
`mdeck theme check <name>`, then `mdeck theme preview <name> --output-dir
<dir>` and look at the PNGs; adjust and repeat until the slides look like the
brand.

### 9.5 Logos

A logo sits in a corner of every slide, drawn the same way when presenting
and in PNG and PDF export. It stays put while slides transition under it,
and it is hidden during the opening countdown and on the end slide. PNG
(with a transparent background) and SVG files work; an SVG is rasterised
sharply at any size (convert text in it to paths first).

A **theme** carries its brand's logo in its `logo:` block (section 9.4), with
the file in the theme folder. A **deck** can add a logo without any custom
theme, or replace or hide the theme's:

```markdown
---
theme: dark
logo: brand/logo-white.svg     # relative to the deck
logo-position: bottom-right    # top-left | top-right (default) | bottom-left | bottom-right
logo-opacity: 40%              # 0 to 1, or a percentage (default 0.6)
logo-height: 48                # px on a 1920x1080 slide (default 56)
---
```

The deck's keys override the theme's one by one, so `logo-opacity` alone
tones down a theme's logo. `logo: none` hides it for the whole deck. Use a
light logo on dark themes and a dark one on light themes. A missing or
unreadable file is reported by `--check` and the slides show no logo.

A **slide** can override the deck with its own `logo` under its heading:
`none` hides the logo on that slide, and a file shows that logo there instead
(or adds one to a deck that has none), in the deck's position, size and
opacity:

```markdown
# Our partners
<!-- logo: brand/partner.svg -->

# A full-bleed photo
<!-- logo: none -->
```

### 9.6 Engines

The **engine** decides what a theme does beyond colours and type: the layer
it paints under the slides, how text slides are laid out, and what it plays
for the countdown and the end. Colours, fonts, sizes and the logo always come
from the theme, so every theme looks like itself on every engine.

mdeck has ten engines. Each has a showcase theme, and any theme can name any
engine.

| Engine | What it shows | Pictures | Countdown and end act |
|---|---|---|---|
| `plain` | slides on a flat background (themes `dark`, `light`, `nord`, `spring`, `summer`) | no | plain numerals |
| `particles` | a living field of glowing particles that morphs from slide to slide and follows the content (themes `ember`, `autumn`, `winter`) | yes | particle digits that burst; the words, a swirl and a bang |
| `led` | a fixed wall of RGB LEDs behind every slide (theme `marquee`) | yes | LED digits, then a white-hot ring runs out over the wall; the words, then every LED dies out |
| `splitflap` | the slide is a departure board: all its text on a grid of split flaps (theme `departures`) | no | digits in solid flaps, then the board scrambles awake; the words, then the board clears |
| `blocks` | pictures built from falling blocks (theme `stack`) | yes | digits in falling blocks that burst apart; the words, then a line clear |
| `thermal` | a heat field under the slides in the theme's heat palette; title and section headings form in heat (theme `thermal`) | yes, as a heat signature | digits that heat up and cool off; the words glow, then cool |
| `line` | generated line art drawn stroke by stroke on a surface: a draftsman's `sheet`, construction lines first (theme `blueprint`), or a `slate`, in chalk (theme `chalkboard`; section 9.7) | yes, as technical pen lines or in chalk when a slide has no art | digits drawn with the pen or the chalk; the words, then they fade |
| `sketch` | a sketchbook page: generated graphite drawings drawn in with a pencil, outlines first, then the shading (theme `sketchbook`, section 9.7) | yes, in pencil when a slide has no art | digits drawn in pencil; the words, then they fade |
| `watercolour` | cold-press paper: generated watercolours bloom onto it, a pale wash first, then the colour spreading (theme `watercolour`, section 9.7) | yes, in ink with a loose wash when a slide has no art | digits in ink and wash; the words, then they fade |
| `darkroom` | a darkroom under a red safelight: generated photographs develop as prints, then the white light comes on (theme `darkroom`, section 9.7) | yes, as a photogram when a slide has no art | digits glowing white like a photogram; the words, then they fade |

The engine and the design set are independent: the showcase themes other
than `departures` use the editorial design set (section 9.9), the plain ones
the standard set. The split-flap engine draws every slide itself, as a board.

**The LED engine.** The slide sits on a wall of LEDs, a few pixels apart,
their unlit lenses just visible. Nothing ever moves: pictures appear by
lighting LEDs. A `picture` powers on from its centre outward, each LED
flickering as it strikes, and then shimmers slowly between the theme's
`accent`, `accent-soft` and `engine.cool`; the hottest cores whiten toward
`engine.light`. Brightness follows the point cloud's density, so strokes
stay brighter than fills and the picture keeps its structure. Title slides get
a marquee border of chasing bulbs in `secondary`; slides without a
picture get a slow, faint aurora on the side away from the copy. A reveal
sends one band of light across the wall. On charts and diagrams the wall
serves the content: a peak marker floats over each bar like a level meter's,
lines and routed edges leave a soft trail, pies and donuts get a halo ring,
and nothing lights inside the chart itself. On a light theme the LEDs read as
a printed dot matrix.

**The split-flap engine.** The slide is a departure board: a fixed grid of 32
columns by 12 rows of flaps, the same on every slide, with the deck's title
and the slide number printed under it. Every piece of text goes on the board,
in capitals:

| Markdown | On the board |
|---|---|
| Heading | the top row(s), in the highlight colour (`accent`) |
| Paragraph | wrapped word by word to the board's width |
| List | a row per item behind a coloured marker; wrapped lines hang under the text; `+` items flip in on their step |
| Table | a timetable: columns on flap boundaries, numbers right-aligned, the header dim |
| `**bold**` | `accent-soft`; `` `code` `` is `secondary` (a flap cannot change weight) |
| Link | its text |
| Quote | between quote marks |
| `@kpi` | label on the left, value on the right |
| `@progress` | label, a bar of solid flaps and the value |
| Image | the slide's first image sits in a panel on the right of the board |

Going to the next slide never slides or fades: every flap turns forward
through its wheel (a blank, A to Z, Å Ä Ö Æ Ø Ü É, digits and punctuation)
until it shows its new character, the cells starting a moment apart, left to
right and down, so a whole board changes in about a second and a half. Flaps
that keep their character stay still. A title slide is centred on the board.
The countdown draws its digits in solid yellow flaps, then the board
scrambles awake into the first slide; the end shows THE END and then clears
flap by flap.

A board never scrolls. What it cannot show is reported by `mdeck --check`,
never typeset outside the board: text that needs more rows than the board has
(the last row then ends in `…`), code blocks, charts and diagrams,
formulas, a second image, `picture`, table cells cut to fit, and
characters the flaps do not carry (Chinese, Japanese and Korean, emoji and
most symbols show as blank flaps). The board says what a timetable says:
agendas, schedules, status and numbers read best.

**The blocks engine.** A `picture` is cut into a grid of blocks,
grouped into pieces of two to four, and the pieces drop from above the slide,
bottom row first, land with a small bounce and settle into the picture. The
blocks are bevelled, in `accent`, `secondary`, `engine.cool`,
`accent-soft` and the fifth `series` colour. Leaving a slide, the stack
flashes and clears row by row, like a completed line. Slides without a
picture stay calm. Exports show the settled stack.

**The line engine.** Line art drawn on a surface the theme chooses with
`surface:` (`sheet`, the default, or `slate`). Line art is the same on both,
so a deck switches between the `blueprint` and `chalkboard` themes without
new pictures. `surface` is a setting in the theme's engine block
(`engine: { name: line, surface: slate }`).

*The sheet* (`surface: sheet`, theme `blueprint`). Every slide is a Prussian
blue drawing sheet: a
fine grid in `rule` with heavier lines every fifth square, a ruled double
border with zone ticks, and a title block in the bottom-right corner with the
deck's title and the sheet number (the sheet numbers itself,
so the editorial counter is left out). A slide's generated line art (section
9.7) is inked in `heading` colour the way a draughtsman works: faint
construction lines in `rule` run ahead, the ink follows stroke by stroke,
large shapes first and details after, under the crosshair of a drafting
machine, and dimension lines are ruled under and beside the finished drawing.
On a title slide the drawing sits large and dim behind the title. Without
art, the slide's `picture` is drawn as technical pen lines, and the
countdown and the end words are drawn the same way. The `blueprint` theme
lays the sheet on a drafting table (`page:` in section 9.4). Exports show the
finished sheet.

**The sketch engine.** A sketchbook: with the `sketchbook` theme every
slide is a sheet of drawing paper on a desk (`page:` in section 9.4). A
slide's generated drawing (section 9.7; graphite and ink by default) is
drawn in with a pencil you can see: first the outlines, traced along the
lines, then the shading, laid in stroke by stroke in bands that sweep across
the picture while the pencil zigzags along them. The pencil's body is
the engine block's `cool`; its lead and line art are the `heading` colour. Line art
(`kind: line` in a theme's engine block) is drawn with a faint underdrawing first.
On a title slide the drawing sits large and faint behind the title. Without
art, the slide's `picture` is drawn in pencil, and so are the
countdown and the end words. Exports show the finished drawing.

*The slate* (`surface: slate`, theme `chalkboard`). Every slide is a slate (`background`) with soft
clouds where it was wiped and the faint ghosts of earlier drawings in white
and the coloured chalks (`accent`, `accent-soft`, `secondary`); the
`chalkboard` theme puts it in a wooden frame (`page:`). A slide's line art
(section 9.7) is drawn in white chalk (`heading`) along its strokes, the
chalk breaking up in clumps on the slate, a stick of chalk at the point and
dust falling from it. On a title slide the drawing sits large and faint
behind the title. Without art, the slide's `picture` is drawn in
chalk, and so are the countdown and the end words.

**The watercolour engine.** With the `watercolour` theme every slide is a
sheet of cold-press paper on a table. A slide's generated watercolour
(section 9.7) blooms onto the paper: a pale first wash over the whole
picture, then the colour spreading outward from where the paint is heaviest,
wet edges arriving softly, the dark accents dropped in last. Without art the
slide's `picture` is drawn in ink (`heading`) with loose washes of
`accent`, `secondary` and `accent-soft` laid along it a moment later, and so
are the countdown and the end words. Line art is drawn as an ink drawing.

**The darkroom engine.** The slides are in a darkroom, a red safelight
(`accent`) glowing above. A slide's generated photograph (section 9.7) is a
print on white fibre paper with a border and a shadow; it develops in place,
the shadows first and the highlights last, everything red under the
safelight, and when it is done the white light comes on and the print shows
its true greys. On a title slide the photograph sits dim behind the title.
Without art the slide's `picture` becomes a photogram: its shape left
white on a black print. The countdown and the end words glow the same way.

**The thermal engine.** The deck is seen through a thermal instrument: a
heat field lies under the slides, drawn in the theme's heat palette
(`engine: { name: thermal, palette: iron }`) in contour bands and transparent where it is
cold. Its motion is kept for the moments that matter:

- *The cold opening.* On title and section slides the heading forms in heat:
  points of heat appear inside the letters, spread and join into contours,
  and the words are readable in under a second. Then the crisp type rises
  into it (the copy waits about 1.5 seconds) and the heat settles into a
  faint contour halo that stays. Going back to the slide plays it again.
- *Heat signatures.* A `picture` glows like a warm body; the
  countdown digits heat up and cool off; the end words glow and fade.
- *Calm evidence.* Where a slide shows a chart, a diagram, an image or a
  `@thermal` block, the field stays dark. With `drift: true` in the engine block a few
  embers drift through the dark on ordinary slides, cooling as they rise;
  by default the background is still.
- *Cooling between slides.* Nothing is cleared on a slide change: the old
  slide's heat cools while the new one's builds.
- *The heat trace.* Pen strokes (left drag) arrive white-hot, cool through
  the palette and fade after about four seconds.

Exports and reduced motion (section 15) show the settled field: the finished
title with its faint halo.

**Choosing one.** A theme names its engine (`engine:` in section 9.4). A deck
can run on another engine without touching the theme:

```markdown
---
theme: winter
engine: plain
---
```

`mdeck deck.md --engine <name>`, `mdeck export deck.md --engine <name>` and
`mdeck deck.md --check --engine <name>` try an engine without editing the deck.
Precedence: `--engine`, then `engine`, then the theme's `engine:`. An unknown
`engine` is reported and the theme's engine is used; an unknown `--engine`
stops with the list of engines. Whether there is a countdown comes from the
deck's `countdown`, then the theme's; an engine without a countdown of its
own shows plain numerals.

**Content an engine does not show.** Engines differ in what they can show:
the plain engine draws no artwork (mdeck stipples point clouds for it), and
the split-flap board shows text only. `mdeck --check`
lists every such slide under the `engine` category (for example
`slide 4 (line 31): [engine] picture: server is not shown by the splitflap engine`),
and presenting or exporting prints one summary line when a deck has any.

Engines are part of MDeck and each one is a cargo feature, on by default.
Building MDeck with `--no-default-features` leaves them out (and with them
the themes that run on them: `ember`, `autumn` and `winter` on particles,
`marquee` on led, `departures` on splitflap, `stack` on blocks, `blueprint`
and `chalkboard` on line, `sketchbook` on sketch, `watercolour` on
watercolour, `darkroom` on darkroom, `thermal` on thermal, and the built-in
point clouds); themes and
decks that ask for one then use `plain`, with a warning.

### 9.7 Generated art

**Art engines** draw a picture made for each slide, generated with the AI
image model you configured, and draw it in as the slide opens. The picture
is made once, by an explicit command, and cached next to the deck: MDeck
never calls the AI while presenting, so there is no waiting, no surprise
cost, and it works offline.

| Engine | Picture | How it is drawn in |
|---|---|---|
| `line` | line art (black ink on white, drawn by the engine in the theme's colours) | on the sheet: construction lines, then ink along the strokes, then dimension lines; on the slate: chalk along the strokes, grainy, with a stick of chalk shedding dust |
| `sketch` | graphite and ink drawings in the MKLab house style (tonal) | outlines along the lines, then shading in sweeping bands, with a pencil |
| `watercolour` | loose watercolours on white paper (tonal) | a pale wash, then the colour blooming from where the paint is heaviest |
| `darkroom` | black-and-white documentary photographs (tonal) | developing as a print, shadows first, under a red safelight |

**Which slides get a picture.** The slides whose design has a stage in
the theme's design set (section 9.9): in the editorial set title, section,
statement, points, quote and text-only content slides. `picture: none` on a slide
leaves it without.

**Scenes.** `picture-prompt:` in a slide's settings says what its picture
shows. Without it, the chat model writes a scene from the slide's copy and
speaker notes: one concrete visual metaphor with a single focal subject.
`art-world:` in the frontmatter is the deck's world (setting, era, recurring characters),
and every scene keeps to it. Pictures never contain text; the slide's own
words stay typeset.

```markdown
---
title: The Harbour Bridge
theme: blueprint
art-world: A Victorian harbour town where a small team builds modern machines.
---

# The Harbour Bridge
<!-- picture-prompt: A great iron suspension bridge under construction across a harbour. -->

# Every slide gets its own drawing

- The chat model writes this slide's scene from its copy

# Numbers stay typeset
<!-- picture: none -->

- No picture here
```

**Making the art.** `mdeck ai pictures talk.md` draws a picture for every slide
that takes one and has none that is current, four at a time with retries
(about 20 seconds each). `--slide N` redraws one slide, `--stale` only the
slides whose picture has gone stale, `--force` all of them, `--dry-run` lists
what would be drawn, `--engine` draws for another art engine than the deck's,
and `--node` uses another image node than the default. Pressing `S` while
presenting draws the current slide's picture in the background.

**Where it is kept.** The pictures go in `talk.assets/artworks/` and are
recorded in the deck's asset manifest (see "Generated assets" below): per
picture the slide number, a hash of the slide's source and the deck's world,
the style it was made in, the file and the scene. Editing a slide makes its
picture **stale**: it is still shown, and `mdeck --check` says so. Set
`state: pinned` on an entry to keep a picture for its slide number whatever
the slide says (it is never stale and never redrawn); `file:` may then point
at any PNG or JPEG of your own. Line art is shared by every line medium, so
switching a deck between line media costs nothing.

**Generated assets.** Everything `mdeck ai` makes for a deck lives in one
folder next to it, `talk.assets/` for `talk.md`: `artworks/`, `images/`
(`![prompt](generate:)`), `icons/` (`icon: generate:`) and `point-clouds/`
(`picture` names that resolve nowhere else). One file,
`talk.assets/manifest.yaml`, records every asset:

```yaml
version: 2
assets:
- kind: artwork            # artwork | image | icon | point-cloud
  file: artworks/talk-02-line-765658.jpg   # relative to talk.assets/
  slide: 2                 # the slide it was made for
  hash: 4a189c3aa2f2a217   # the slide's source when it was made (artworks)
  style: line-9b4b0041     # the style it was made in
  prompt: A navigator holds one large, unmarked map
  generated: 2026-09-28 12:32
  state: current           # current | stale | pinned
- kind: image
  file: images/rocket-at-dawn.png
  placeholder: a rocket at dawn   # the prompt as written in the deck
  slide: 3
  style: default-1a2b3c4d
  state: current
```

An asset is **current** (made from its source as it reads now, in the style
in use), **stale** (its slide or style changed since: still shown, reported
by `--check`) or **pinned** (kept whatever its source says, never
regenerated). `mdeck ai` refreshes `current` and `stale` whenever it writes
the manifest; `pinned` is yours to set. A file replaced by hand is used as it
is. Every `mdeck ai` generation command takes `--slide N`, `--stale`,
`--force` and `--dry-run`: by default it makes what is missing or stale.

**Style.** Each medium has a style card: a style prompt and two small
neutral style swatches (a still life and a street), sent as reference images
to models that accept them. A theme's engine block (`style`, `references`, section 9.4) replaces
either.

**Without art.** An art engine never needs the AI to present: a slide with
no picture shows its `picture` in the medium, or just the page and its
typography. `mdeck --check` and the line printed when presenting name the
slides without a picture and the command that draws them.

---

### 9.8 Background images

A background image sits behind a slide's content: a brand texture or a photo
behind every slide, or one striking picture behind a key slide. Set it once
in the frontmatter for the whole deck:

```yaml
---
background: images/texture.jpg   # relative to the deck
background-opacity: 25%          # 0 to 1, or a percentage (default 0.3)
---
```

A **slide** can change it under its heading:

```markdown
# Welcome
<!--
background: images/stage.jpg
background-opacity: 60%
-->

# The code
<!-- background: none -->

# The quiet one
<!-- background-opacity: 10% -->
```

`Welcome` shows its own image at its own opacity, `The code` shows no
background, and `The quiet one` shows the deck's image, fainter.

- One image per slide. A slide's `background` replaces the deck's (or adds
  one to a deck without a default); nothing stacks.
- `background-opacity` works the same at both levels. A slide's own image
  keeps the deck's opacity unless the slide sets one; a slide that sets only
  the opacity shows the deck's image with it.
- The image **covers** the slide: scaled to fill, centred, cropped on the
  long side, never stretched.
- It is painted on the theme's background colour and under everything else
  (the engine's layer, the content, the logo), so a low opacity blends it
  toward the theme's own colour and text stays readable on light and dark
  themes alike. The default of 0.3 suits photos behind text; raise it for a
  slide where the picture leads.
- On a theme with a page (section 9.4) it goes on the sheet. Engines draw
  above it as usual; a board engine (`splitflap`) covers most of it with its
  board.
- It moves with its slide in transitions and fades in when its image has
  loaded. It is not shown during the opening countdown or on the end slide.
- PNG and PDF export show exactly what the window shows.
- PNG, JPEG, WebP and SVG files work. `--check` reports a file that is
  missing, is not an image or cannot be read, an opacity that does not
  parse, and a slide `background-opacity` with no image to apply to, each
  with its line.

An image with `@fill` (section 5.4) is different: it is content and takes the
slide over. A background stays behind the heading and the text.

### 9.9 Designs and arrangements

A theme decides how every design (section 4) looks, with data only.

**Design sets.** `designs: standard` (the default) is the classic slide:
content centred, the heading on top, no ornament and no entry motion.
`designs: editorial` is the magazine spread: a copy column on the left, an
eyebrow (the slide's Roman numeral and the deck title), display type, a
soft pillow behind the copy, a staggered entry and a stage on the right
where the engine draws the slide's picture. Both sets arrange every design
and recognise slides the same way. The set is independent of the engine:
`designs: editorial` with `engine: plain` is Ember's look on a still
screen; `designs: standard` with `engine: particles` puts centred slides
over the particle field.

`designs:` may also name a set file, `designs/<name>.yaml`, found in the
deck's `designs/` folder, then the `designs/` folder of the user folder, then
installed packs' `designs/` (section 18), then the built-ins. A set file has
`base` (keys for every design) and `designs` (keys per design), merged over
the set it `extends` (default `standard`). A name that no set file or
built-in has may name a design set an extension registers in code (a custom
build, section 18), which then draws every slide; `--check` lists what it does
not show. Any other name falls back to `standard`, with a warning. A set
file:

```yaml
# designs/roomy.yaml
extends: editorial
base:
  roles:
    body: { color: accent }
```

**Arrangements.** `arrangements:` overrides any key of any design's
arrangement; `all:` applies to every design. Overrides are partial and
merge key by key through `extends`.

```yaml
designs: editorial
arrangements:
  all:
    ornaments: { bullet: "◆" }
  quote:
    copy: { region: [0.12, 0.25, 0.76, 0.5], align: center }
    roles:
      quote: { size: h2, scale: 0.9, font: display }
      attribution: { color: accent }
    ornaments: { quote-bar: none, quote-marks: true }
  title:
    roles:
      title: { case: upper, tracking: 0.04 }
    entry: { kind: fade, duration-ms: 900 }
```

An arrangement's keys:

| Key | Values |
|---|---|
| `copy` | `{ region: [x, y, width, height], align: left\|center\|right, valign: top\|middle\|bottom }`: where the text goes, in fractions of the slide |
| `wide` | like `copy`: where a `content` slide's copy goes when it holds an image, code, a table or a visual (its stage is given up) |
| `plate` | `{ region, place: below\|beside, align, valign, gap, rule }`: where the image, gallery, code, table, visual or columns go; `below` puts it under the copy, `beside` in its own region; `rule` draws a hairline over each column |
| `stage` | `none`, `right`, `left` or `backdrop`: where the engine may draw the slide's picture |
| `with-picture` | `{ stage, copy }`: the stage, and where the copy goes instead (like `copy`), on a slide that sets a picture and holds no image, code, table or visual; the standard set uses it to open a stage only when a slide asks for a picture |
| `eyebrow` | `none`, `numeral` (Roman numeral and deck title) or `deck` (author and deck title) |
| `byline` | `true`: a title page shows the deck's author |
| `entry` | `{ kind: none\|fade\|rise\|stagger, step-ms, duration-ms, rise, reveal: slide\|rise\|fade, reveal-ms }`: how copy comes in, and how a `+` item revealed with Next comes in |
| `roles.<role>` | `{ font: display\|body\|lead\|strong\|mono, size, scale, color, opacity, case: none\|upper\|lower, tracking, line-height, gap, align, italic }` for each role: `eyebrow`, `title`, `subtitle`, `kicker`, `byline`, `heading`, `statement`, `lead`, `body`, `list`, `nested`, `quote`, `attribution`, `caption` |
| `ornaments` | `bullet` (a glyph or `dot`), `bullet-color`, `indent`, `nested-indent`, `item-gap`, `numbering`, `quote-marks`, `quote-bar: none\|left`, `bar-color`, `bar-width`, `attribution-dash`, `title-rule`, `emphasis: italic\|accent`, `pillow`, `begin-hint` |

Sizes are theme size tokens (`h1`, `h2`, `h3`, `body`, `code`, or `level`
for a heading's own level) or px at 1920x1080. Colours are theme colour
roles (`text`, `heading`, `muted`, `strong`, `accent`, `accent-soft`,
`secondary`, `bright`, `rule`). Gaps are spacing tokens (`xs`, `sm`, `md`,
`lg`, `xl`, from the theme's `spacing:`), px, or a multiple of the
element's own size (`0.5em`). The built-in sets are
`crates/mdeck/designs/standard.yaml` and `editorial.yaml`; every key is
listed there.

**Invariants** (not themeable): a `@fill` media image covers the whole
slide with its copy in a band at the bottom; gallery grids (two side by
side, three as two over one, four as 2x2, rows of three after); several
visuals on one plate sit side by side when the plate is wide, else
stacked; the fit floors (code 40%, prose 80%).

## 10. Columns

A `+++` line splits a slide into two columns (the `columns` design); it is recognised without a setting:

```markdown
# Comparison

**Before:**

Old approach with manual config.

+++

**After:**

New approach with auto-discovery.
```

Content before `+++` is the left column; content after is the right column. More `+++` lines give more columns. A leading H1 or H2 spans the columns.

The `+++` separator was chosen because it is visually distinct from `---` (slide break) and is not a standard markdown construct.

---

## 11. Edge Cases

### Content overflow
Text is never truncated silently. Content that does not fit first shrinks (visuals in the copy to 35% of their height, code to 40% of its size, then prose and lists to 80%), then scrolls with fade cues at the edges (section 4.3).

### Empty slides
A slide with no content renders as a blank slide with the theme's background. This is intentional, not an error.

### Adjacent separators
Several `---` breaks in a row produce no empty slides: a slide needs content.

### Frontmatter parse failures
If YAML in the frontmatter is malformed, each `key: value` line is still read on its own.

### Missing images
If an image path cannot be resolved, a placeholder box with the alt text is rendered, and a warning is emitted.

### Code blocks without language
Rendered as plain monospace text with no syntax highlighting.

### `+` markers inside code blocks
List markers are never interpreted inside fenced code blocks. This is standard markdown behavior: fenced block content is literal.

### `*` items
`*` is an ordinary bullet, the same as `-`: always visible.

---

## 12. Complete Example

```markdown
---
title: "Scaling Our Platform"
author: "Jane Doe"
theme: dark
transition: slide
---

# Scaling Our Platform

Engineering deep-dive, February 2026

---

# The Problem

+ 10x traffic growth in 6 months
+ P99 latency spiked from 50ms to 800ms
+ Database connection pool exhausted daily

---

# Architecture Before

![Old architecture @width:80%](old-arch.png)

A monolith struggling under load.

---

# The New Architecture

```@architecture
# Components
- User      (icon: user,      pos: 1,1)
- Gateway   (icon: api,       pos: 2,1)
- Service A (icon: container,  pos: 3,1)
- Service B (icon: container,  pos: 3,2)
- Database  (icon: database,   pos: 4,1)
- Cache     (icon: cache,     pos: 4,2)

# Flow
+ User -> Gateway: Request
+ Gateway -> Service A: Route
+ Gateway -> Service B: Route
+ Service A -> Cache: Check cache
+ Service A -> Database: Query
```

---

# Key Code Change

```rust {3-5}
pub async fn handle_request(req: Request) -> Response {
    let key = req.cache_key();
    if let Some(cached) = cache.get(&key).await {
        return cached;
    }
    let result = db.query(req.query()).await?;
    cache.set(&key, &result, TTL).await;
    result
}
```

---

# Results

+ P99 latency: 800ms to 45ms
+ Connection pool usage: 95% to 12%
+ Zero downtime during the migration

---

> The best optimization is the one you don't have to make.

-- Our team's new motto

---

# Before and After
<!-- design: columns -->

**Before:**

- Monolith
- Single database
- No caching
- Manual scaling

+++

**After:**

- Microservices
- Sharded database
- Redis cache layer
- Auto-scaling

---

# Questions?

Thank you for listening.
```

This example shows: frontmatter, a title slide, a points slide with steps, a media slide with a caption, a diagram with static and stepped links (a visual slide), a code slide with line highlighting, a quote with its attribution, the columns design, and a closing title slide. The `---` breaks are optional between slides that start with a heading; the quote needs one, since it has no heading.

---

## 13. Parser Grammar Summary

This section provides a condensed reference for implementation.

### 13.1 Phase 1: Split document into slides

```
Document     = Frontmatter? Slide (SlideSep Slide)*

Frontmatter  = "---\n" YAML_CONTENT "---\n"
               (only valid at document start, line 1)

SlideSep     = RuleSep | HeadingSep

RuleSep      = /\n\n-{3,}\n\n/
               (--- with blank lines on both sides)

HeadingSep   = a heading (ATX or setext) at or above the slide level,
               when the current slide already has content
               (never inside a fenced block or an HTML comment)
```

Reference link definitions (`[label]: url`) and footnote definitions
(`[^id]: text`) are taken out of the whole document before it is split.

### 13.2 Phase 2: Parse each slide into blocks

```
Slide        = (Block | SettingsComment | NotesBlock)*

SettingsComment = "<!--" SettingLine+ "-->"
               (the first line's key is a known setting; section 7)
SettingLine  = /^\s*[a-z][a-z0-9-]*\s*:\s*.*$/

NotesBlock   = /^`{3,}@notes/ MARKDOWN /^`{3,}$/

Block        = Heading | Paragraph | List | Image | CodeBlock
             | BlockQuote | Callout | VisualBlock | Table | HRule

Heading      = /^#{1,6}\s+.+$/ | TEXT_LINE /^(={3,}|-{3,})$/

Image        = /^!\[([^\]]*)\]\(([^)]+)\)$/ | /^<img\s[^>]*src=.../

CodeBlock    = /^`{3,}(\w+)?(\s*\{[^}]+\})?\n/ CONTENT /\n`{3,}$/
             | INDENTED_LINES (four spaces or a tab)

VisualBlock  = /^`{3,}@TAG\n/ CONTENT /\n`{3,}$/
               (TAG is exactly one of the visual kinds, section 14)

BlockQuote   = /^>/ BLOCKS   (one or more consecutive lines)
Callout      = /^>\s*\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]$/ BLOCKS

HRule        = /^(\*{3,}|_{3,})$/

List         = ListItem+
ListItem     = /^[-+*]\s+(\[[ xX]\]\s+)?/ CONTENT   (unordered, task box)
             | /^\d+\.\s+/ CONTENT        (ordered; the first number counts)
```
### 13.3 Phase 3: Parse visual blocks

Every visual block (charts, diagrams, `@thermal`) is read with one grammar:

```
BlockLine    = Blank | Comment | Setting | Item

Comment      = /^\s*#.*/              (a whole line)
TrailComment = /\s#(\s.*)?$/          (cut from any line, outside "quotes")

Setting      = KEY ":" VALUE          (only before the first Item)

Item         = INDENT MARKER TEXT Attrs?

MARKER       = "-" | "+" | "*"        (followed by whitespace; `+` is a step,
                                       `-` and `*` are static)

Attrs        = "(" KEY ":" VALUE ("," KEY ":" VALUE)* ")"
               (at the end of the item, after a space; a value may hold
               commas and "quoted text"; a trailing "(...)" that is not
               key: value pairs is part of TEXT)

Relation     = NAME " " ARROW " " NAME (":" LABEL)?    (an Item's TEXT)

ARROW        = "->" | "<-" | "<->" | "--" | "-->"

KEY          = /[A-Za-z][A-Za-z0-9_-]*/
```

A visual's verbs (`petal`, `lens`, `commit`) are the first word of an item's
TEXT. Any other line is a problem that `--check` reports (category
`visual`); it is never drawn.

### 13.4 Phase 4: Recognise the design

```
design(slide) -> Design:
    if slide sets design: and has the design's core block:  that design
    else: the first rule of the recognition table (section 4.1) that matches
```

---

## 14. Visualization Syntax

MDeck draws charts, diagrams and thermal images from fenced code blocks whose info string is a visual's tag (`@bar`, `@architecture`, ...). Every visual reads its block with the same grammar.

### 14.1 The visual grammar

**Settings** are `key: value` lines before the first item. Each visual lists the settings it takes; any other key is a problem.

**Items** are list lines. `-` shows the item from the start and `+` reveals it on the next step (section 6). An item may end with attributes in parentheses, `(key: value, key: value)`; a value may hold commas (`pos: 1,2`) and `"quoted text"`. Parentheses that are not `key: value` pairs, as in `- Revenue (USD): 40`, are part of the item's text.

**Verbs.** A visual whose items come in kinds names the kind with the item's first word: `- petal Payments`, `+ lens 76% 43% 16%`, `- commit main`.

**Relations** are items of the form `- A -> B: label`, with a space on each side of the arrow. Visuals that draw links take `->`; `@architecture` takes all five arrows (section 8.4).

**Comments.** A line starting with `#` is a comment, and so is the rest of a line after a space and `#` (`image: a.png  # white-hot`). `#` inside a word or in `"quotes"` is text (`PR #42`).

**Problems.** `mdeck --check` reports, with its line in the file (category `visual`), every line a visual cannot read: a line that is neither a setting nor an item, a setting after the first item, an unknown setting or attribute, and a value that does not parse. Such lines are never drawn as labels.

**Numbers:** Values may carry a currency prefix (`$4200`, `€40`), a `%` suffix, thousands separators (`1,000` or `1_000`), or a trailing unit (`40 users`, `4.2M`). The decoration is ignored; only the number is used. In comma-separated series (`- Revenue: 1,000, 2,000`), a comma followed by exactly three digits is a thousands separator only when items are separated by `", "`.

**Axis labels:** Chart types with axes take `x-label:` and `y-label:` settings. The Y-axis label is rendered rotated 90° counter-clockwise.

**Automatic scaling:** All visualizations scale proportionally to the available slide area. Axes end on a "nice" round number above the largest value (1, 2, 5, 10, 20, 50, 100, ...), so the tallest bar never touches the top of the chart.

**Labels:** Category labels, legend entries and KPI values shrink to a shared minimum size and are then truncated with an ellipsis instead of overflowing. Crowded axis labels (many line-chart points, long Gantt timelines) are thinned automatically.

### 14.2 Bar Chart (`@bar`)

Vertical or horizontal bar chart with category labels and values.

````markdown
```@bar
orientation: vertical
x-label: Programming Language
y-label: Popularity Index
- JavaScript: 65
- Python: 48
+ TypeScript: 38
+ Rust: 22
```
````

**Settings:**

| Setting       | Values                      | Default    | Description                |
|---------------|-----------------------------|------------|----------------------------|
| `orientation` | `vertical`, `horizontal`    | `vertical` | Bar direction              |
| `x-label`     | string                      | none       | Label for the X axis       |
| `y-label`     | string                      | none       | Label for the Y axis (rotated 90° CCW) |

**Items:** `- Label: value`.

### 14.3 Line Chart (`@line`)

Line chart with one or more data series plotted over shared X-axis categories.

````markdown
```@line
x-labels: Jan, Feb, Mar, Apr, May, Jun
x-label: Month
y-label: Temperature (°C)
- London: 5, 6, 10, 14, 17, 20
+ Madrid: 10, 12, 16, 19, 23, 28
```
````

**Settings:**

| Setting     | Values              | Default | Description                              |
|-------------|---------------------|---------|------------------------------------------|
| `x-labels`  | comma-separated     | none    | Category labels along the X axis         |
| `x-label`   | string              | none    | Label for the X axis                     |
| `y-label`   | string              | none    | Label for the Y axis (rotated 90° CCW)   |

**Items:** `- Series Name: value1, value2, value3, ...`

Each series is a separate line. All series share the X-axis categories. A legend is displayed at the top-right.

Items of one value each and no `x-labels` (`- Jan: 3`, `- Feb: 5`, as a bar chart
takes them) are one line over those labels, without a legend, unless an item is revealed with `+`.

### 14.4 Scatter Plot (`@scatter`)

2D scatter plot with labeled data points and optional custom sizes.

````markdown
```@scatter
x-label: Hours Studied
y-label: Test Score
- Alice: 80, 90
- Bob: 65, 75
- Carol: 90, 95 (size: 30)
```
````

**Settings:** `x-label`, `y-label` (as for `@bar`).

**Items:** `- Label: x, y`, with an optional `(size: N)` attribute for the radius of the point.

### 14.5 Stacked Bar Chart (`@stackedbar`)

Stacked bar chart showing multiple series stacked on top of each other for each category.

````markdown
```@stackedbar
categories: Q1, Q2, Q3, Q4
x-label: Quarter
y-label: Revenue ($M)
- Product A: 40, 45, 50, 55
- Product B: 30, 35, 40, 45
+ Product C: 15, 20, 25, 30
```
````

**Settings:**

| Setting      | Values          | Default | Description                            |
|--------------|-----------------|---------|----------------------------------------|
| `categories` | comma-separated | `1..n`  | Category labels along the X axis (numbered when omitted) |
| `x-label`    | string          | none    | Label for the X axis                   |
| `y-label`    | string          | none    | Label for the Y axis (rotated 90° CCW) |

**Items:** `- Series Name: value1, value2, value3, ...`

Each series provides one value per category. Values are stacked vertically. A legend is displayed at the top.

### 14.6 Pie Chart (`@pie`)

Pie chart showing proportional segments. Values are automatically normalized to 100%.

````markdown
```@pie
- Frontend: 35%
- Backend: 30%
+ DevOps: 20%
+ Testing: 15%
```
````

**Items:** `- Label: value%` or `- Label: value`. No settings.

### 14.7 Donut Chart (`@donut`)

Like a pie chart but with a hollow center that can display a label.

````markdown
```@donut
center: Total
- Completed: 65%
- In Progress: 25%
- Not Started: 10%
```
````

**Settings:**

| Setting   | Values | Default | Description                |
|-----------|--------|---------|----------------------------|
| `center`  | string | none    | Text displayed in the center hole |

**Items:** as for `@pie`.

### 14.8 Word Cloud (`@wordcloud`)

Word cloud with words sized proportionally and laid out using spiral packing. Some words are automatically rotated 90° counter-clockwise for the classic word cloud aesthetic.

````markdown
```@wordcloud
- Artificial Intelligence (size: 50)
- Machine Learning (size: 45)
- Data Science (size: 42)
- Python (size: 38)
- Docker (size: 30)
- React (size: 27)
```
````

**Items:** `- Word or Phrase (size: N)` or `- Word or Phrase`. No settings.

The `size` value controls relative importance (larger = bigger font). Without a size, the default is 20. Words are placed largest-first using spiral search. Approximately 35% of words are rendered vertically (rotated 90° CCW) for visual variety; the two largest words are always horizontal.

For best results with word clouds, use 30-100 words with a good spread of sizes (e.g., 10-50).

### 14.9 Timeline (`@timeline`)

Horizontal timeline showing events in order, alternating above and below the line.

````markdown
```@timeline
- 2000: Y2K Bug
+ 2004: Facebook Founded
+ 2007: iPhone Released
+ 2010: Instagram Launched
```
````

**Items:** `- Date: Event description`, or `- Event description` without a date. No settings.

### 14.10 Funnel Chart (`@funnel`)

Funnel chart showing progressive narrowing stages.

````markdown
```@funnel
- Awareness: 10000
- Interest: 5000
- Consideration: 2500
- Decision: 1000
```
````

**Items:** `- Stage: value`. No settings.

### 14.11 KPI Cards (`@kpi`)

Key Performance Indicator cards showing metric values with optional trend indicators.

````markdown
```@kpi
- Revenue: $4.2M (trend: +12%)
- Users: 1.2M (trend: +8%)
- Churn: 2.1% (trend: -0.3%)
- NPS: 61 (trend: steady)
```
````

**Items:** `- Metric: value`, with an optional `(trend: text)` attribute. The trend is shown under the value; its sign picks the arrow and colour: `+` is up (the theme's positive colour), `-` is down (its negative colour), and text without a sign has no arrow. No settings.

### 14.12 Progress Bars (`@progress`)

Horizontal progress bar indicators.

````markdown
```@progress
- Frontend: 85%
- Backend: 60%
- Testing: 30%
```
````

**Items:** `- Label: value%` (clamped to 0 to 100). No settings.

### 14.13 Radar Chart (`@radar`)

Spider/radar chart comparing multiple items across shared axes.

````markdown
```@radar
axes: Speed, Power, Range, Agility, Defense
- Fighter A: 9, 7, 5, 8, 6
+ Fighter B: 4, 9, 8, 6, 7
```
````

**Settings:**

| Setting | Values          | Default | Description                    |
|---------|-----------------|---------|--------------------------------|
| `axes`  | comma-separated | none    | Names of the radar axes (required) |

**Items:** `- Series: v1, v2, v3, ...` (one value per axis)

### 14.14 Venn Diagram (`@venn`)

Venn diagram of two or three overlapping sets, with a label in each overlap.

````markdown
```@venn
- Design (size: 30)
- Engineering (size: 30)
- Business (size: 25)
+ Design & Engineering: Prototypes
+ Engineering & Business: Platforms
+ Design & Engineering & Business: Products
```
````

**Items:**
- `- Name` or `- Name (size: N)`: a set, drawn as a circle whose area follows its size (default 30).
- `- A & B: label`: the label written in the overlap of sets A and B (`A & B & C` for the middle). The names must be sets of the diagram.

No settings.

### 14.15 Organization Chart (`@orgchart`)

Hierarchical org chart drawn as a tree with connecting lines.

````markdown
```@orgchart
- CEO
- CEO -> CTO
- CEO -> CFO
+ CTO -> VP Engineering
+ CTO -> VP Product
```
````

**Items:**
- `- Manager -> Report`: a reporting line (the parent first). Links take no label.
- `- Name`: a root of the tree, or a person with no reporting line. Without any `- Name` item the roots are the people who report to no one.

No settings.

### 14.16 Gantt Chart (`@gantt`)

Project timeline with tasks, durations, dependencies, and automatic time scaling.

````markdown
```@gantt
title: Project Plan
- Research: 2024-01-15, 10d
- Design: 5d, after Research
- Frontend: 15d, after Design
- Backend: 15d, after Design
- Testing: 5wd, after Frontend
- Launch: 2d, after Testing + 3d
```
````

**Items:** `- Task Name: spec1, spec2, ...` where specs can be:

| Spec | Description |
|------|-------------|
| `YYYY-MM-DD` | Absolute date (start or end) |
| `Nd` | Duration in calendar days |
| `Nwd` | Duration in working days (Mon-Fri) |
| `Nw` | Duration in weeks |
| `Nm` | Duration in months (~30 days) |
| `after TaskName` | Start when TaskName ends |
| `after TaskName + Nd` | Start N days after TaskName ends |

**Valid combinations:**
- Start date + end date: `2024-01-01, 2024-02-01`
- Start date + duration: `2024-01-01, 10d`
- Duration + dependency: `5d, after Research`
- Duration + dependency with delay: `3wd, after Design + 2d`

**Settings:**

| Setting  | Values            | Default | Description |
|----------|-------------------|---------|-------------|
| `title`  | string            | none    | Chart title displayed above the bars |
| `labels` | `side`, `inside`  | `side`  | `inside` renders task names inside the bars instead of in a left column, giving the full width to the timeline. When a bar is too short for the name, the name shows to the right of the bar |

**Timeline auto-scaling:** The time axis automatically selects the appropriate unit:
- Days (for timelines up to ~3 weeks)
- Weeks (for timelines up to ~4 months)
- Months (for longer timelines)

Dependencies are shown as connector arrows between tasks.

### 14.17 Git Graph (`@gitgraph`)

Visualizes git branching, committing, and merging as a horizontal lane diagram. Useful for illustrating branching strategies like Git Flow, or showing actual repository history.

````markdown
```@gitgraph
- lane main
- lane develop
- lane feature/login
- commit main
+ commit main
+ branch main -> develop
+ commit develop
+ branch develop -> feature/login
+ commit feature/login: "Add login form"
+ merge feature/login -> develop: "PR #42"
+ tag main: "v1.0"
+ merge develop -> main
```
````

**Items** (each starts with its verb):
- `lane <name>`: declare a branch lane (order determines vertical position, rendered as a dotted background line)
- `commit <branch>`: add a commit dot on the named branch (optional `: "message"`)
- `branch <source> -> <target>`: fork a new branch (S-curve connector with arrow)
- `merge <source> -> <target>`: merge one branch into another (curved connector with arrow), optionally with `: "label"`
- `tag <branch>: "label"`: tag box displayed above the most recent commit on the branch

No settings.

**Rendering:** Lanes are stacked vertically as parallel horizontal tracks with dotted background lines. Commits appear as dots on the lane. Forks and merges are shown as S-curve connections with arrows between lanes. Each lane gets a distinct color from the theme palette.

**Progressive reveal:** Use `+` markers to build the graph step by step, ideal for walking through a branching strategy one operation at a time.

### 14.18 Flower (`@flower`)

A platform in the middle and the teams around it: platform engineering, an
internal developer platform, any shared capability with peers that both use
it and contribute to it. Each petal is a bulb in its own colour whose outline
runs out of the centre, around the bulb and back into the centre with an
arrow.

````markdown
```@flower
- center Development Platform: Shared capabilities and services (icon: database)
- petal Team 1: Builds product features and contributes back
+ petal Team 2: Builds services and contributes back
+ petal Team 3: Builds tools and contributes back
- Team 1 -> Team 3: uses
```
````

**Items:**
- `center Name: description (icon: name)`: the platform, a circle in the theme's accent colour. One per flower (with two, the last wins).
- `petal Name: description (icon: name)`: a team or domain. An item with no verb that is not a link is a petal too, so `- Payments` is enough.
- `A -> B: label`: a link from one petal to another, drawn as a curve that keeps clear of the centre. Both ends must be petals.

The description and the icon are optional. Petals show the `team` icon unless
they name another (section 8.8) or `(icon: none)`; the centre shows an icon
only when it names one. No settings.

**Rendering:** The first petal is on top and the rest follow clockwise, evenly
spaced, each in the next colour of the theme's palette. Every petal is the same
size; the flower sizes its petals and centre for the text, then scales as a
whole to fit the slide, so long descriptions or many petals make it smaller.
Keep descriptions to a short sentence.

**Progressive reveal:** `+` on a petal makes it grow out of the centre
on its step; a link appears once both its petals have.

### 14.19 Artifact Flow (`@artifactflow`)

How artifacts (binaries, packages, container images, APIs) move from the teams
that produce them, through shared infrastructure, to the teams that consume
them. Producers sit on the left and consumers on the right, each in a titled
panel; services are larger cards in the middle. Every artifact is an arrow
with its name on it.

````markdown
```@artifactflow
producers: Producing Teams | Build and publish artifacts
consumers: Consuming Teams | Retrieve and use artifacts
- producer Build Team: Produces binaries and container images
- producer Platform Team: Produces reusable libraries
- service Artifactory: Artifact repository / registry
  - Container images
  - Libraries
- consumer Integration Team: Pulls artifacts for test environments
- consumer Product Team: Pulls approved artifacts for production
+ Build Team -> Artifactory: Container image v1.2.3 (icon: package)
+ Platform Team -> Artifactory: Library v4.5.0 (icon: code)
+ Artifactory -> Integration Team: Pull image (icon: package)
+ Artifactory -> Product Team: Pull package (icon: code)
```
````

**Items:**
- `producer Name: description (icon: name)`, `service ...`, `consumer ...`: a card in that column. Producers and consumers show the `team` icon and services the `database` icon unless they name another (section 8.8) or `(icon: none)`.
- An indented `- item` under a card adds a bullet to it.
- `A -> B: label (icon: name)`: an artifact moving from A to B. The label and its icon are optional; both ends must be cards.

Without any `->` item, every producer publishes to every service and every
service feeds every consumer (or producers feed consumers directly when there
is no service).

**Settings:**
- `producers: Title | subtitle`, `services: ...`, `consumers: ...`: a column's heading. Producers and consumers are titled "Producers" and "Consumers" by default and services have none; `none` removes a heading.

**Rendering:** Edges are smooth curves that leave a card's right side and
reach the next card's left side; edges sharing a side are spread along it in
the order of their other ends, so they never cross there. Labels sit above
their arrow at its quieter end. Text shrinks together when a column is too
tall for the slide.

**Progressive reveal:** `+` works on cards and on edges; an edge appears
(drawing itself toward its arrowhead) once both of its ends have.

### 14.20 Thermal images (`@thermal`)

A `@thermal` block shows one thermal image, coloured with a thermal palette,
with a legend, spots, and reveals for telling what the image shows. The
theme never changes how an image looks and MDeck never guesses: ordinary
images (`![...](photo.jpg)`) are always shown as they are. Only an image in
a `@thermal` block is read as thermal.

````markdown
```@thermal
image: cabinet-4.png             # white-hot export: brighter is hotter
visible: cabinet-4-visible.jpg   # optional registered photo for the lens
palette: iron
label: Cabinet 4, breaker row B
+ lens 76% 43% 16%
+ reveal
+ spot Hotspot 76% 43%
+ spot Reference 30% 52%
+ above 85%
```
````

**Settings** (`key: value` lines before the first step):

| Key | Value |
|---|---|
| `image:` | the thermal image: a grayscale export, brighter is hotter (white-hot), with no palette, scale bar or text burned in. It may be stored as an RGB file; the pixel values decide |
| `data:` | instead of `image:`, temperature data: a 16-bit (or 8-bit) grayscale PNG with a sidecar `<name>.yaml` beside it |
| `visible:` | a visible-light photo of the same scene, registered (same framing) with the thermal image. The lens reveals the thermal image over it. Shown as it is |
| `palette:` | `iron` (default), `white-hot`, `black-hot`, `rainbow`, `arctic`, `lava`; else the deck's `palette` |
| `mapping:` | `linear 18..92 °C`: the gray levels of `image:` run linearly over this range (units `°C`, `°F`, `K`, or any other unit, kept as written) |
| `window:` | `40..90 °C`: the values the palette spans (level and span). Only with a mapping or data |
| `polarity:` | `black-hot` for a source where darker is hotter (inverted before palette and thresholds); `white-hot` is the default |
| `label:` | a caption under the image |

**Steps** (items with `-` and `+` like every visual, each starting with its verb; the state at a
step never depends on how it was reached, so going back shows that step
exactly):

| Step | What it does |
|---|---|
| `lens X% Y% R%` | a lens at the point (fractions of the image) with radius R (of the image's width) shows the thermal image over the visible photo (or over the thermal image in gray, without `visible:`). A later `lens` glides there; the first glides in. It stays put until the next click |
| `reveal` | the lens opens until the thermal image fills its frame; the lens's edge dissolves |
| `above 85%` | only what is above the threshold keeps the palette; the rest goes gray. A share of the displayed range; a later `above` changes the threshold (cross-fading) |
| `above 60 °C` | the same in the source's unit: only with a mapping or data, converted between temperature units |
| `spot Name X% Y%` | a crosshair with a label. With a mapping or data the label shows the value sampled there (`≈` for a linear mapping, exact for data, `≥`/`≤` when the source clipped there, `no data`) |
| `spot Name X% Y%: text` | the label shows the author's text, marked `†` with the note "† value supplied by the author" |

Without a `lens` or `reveal` line the thermal image shows from the start.
The legend appears with the thermal image: a palette bar with a caption,
reading **relative intensity** (high, low) for an image without a mapping,
and values with round ticks for a mapping or data; the threshold is marked
on it. Clipped or missing pixels are hatched and the legend notes them.

**What the source can say.** The block's source decides what it may claim:

| Source | Legend | Spots | Threshold |
|---|---|---|---|
| `image:` only (a display image) | relative intensity, no numbers | the author's text, marked `†` | relative (`above 85%`) |
| `image:` with `mapping:` | values in the unit | sampled, shown with `≈` | relative or in the unit |
| `data:` with its sidecar | values in the unit | sampled | relative or in the unit |

A display export from a camera's histogram mode is not a temperature map,
so it is never labelled in degrees. `mdeck --check` (category `thermal`) and
presenting and exporting report what a source cannot honour: a `window:`
or a `°C` threshold on a display image, a spot text that reads like a
measurement on one, `mapping:` on a data file. Steps the source cannot show
are left out of the slide's steps, so no click does nothing.

**Colour input.** A colour image in a `@thermal` block (an export already in
iron, or a photo) is detected from its pixel values (with a tolerance) and
shown as it is, without palette, legend or threshold; the note "shown as
exported" appears under it, and a diagnostic says "unsupported chromatic
input" when presenting, exporting and in `--check`. Its threshold steps are
left out; the lens still works, since it only lays one picture over the
other.

**The data sidecar.** `cabinet.thermal.png` is read with
`cabinet.thermal.yaml`:

```yaml
unit: °C          # °C, °F, K, or another unit
scale: 0.001526   # value = raw * scale + offset
offset: 10
nodata: 0         # optional: the raw code of pixels without a value
clipped_low: 1    # optional: raw codes at or below this clipped
clipped_high: 65535 # optional: raw codes at or above this clipped
```

**Comparing images.** Put two blocks on one slide (for example
`design: columns`) and give the slide one scale with
`thermal-window: 25..90 °C`. Every block on the slide then uses that window,
converted into its own unit, so the same colour means the same value on both
sides, even when the sources have different mappings. A source without a
mapping or data cannot be compared (`--check` says so); clipped and missing
values stay hatched, since a common window cannot recover them.

**The palette key.** While presenting, `C` cycles every `@thermal` image and
legend through the palettes, and `Shift+C` returns to the palettes as
written. Headings, charts and the theme do not change. Export always uses the
palettes as written.

**Zoom into a spot.** `zoom-to: Hotspot` on a slide makes the step into it a
zoom into the spot named `Hotspot` on the previous slide's `@thermal` image:
the old slide magnifies around the spot and fades as the new one settles. If
the previous slide shows no such spot, the transition is the usual one.
`transition: zoom` beside it says the same thing in so many words; `zoom-to`
alone implies it, and `transition: zoom` without `zoom-to` is reported by
`--check`.

```markdown
# The hotspot, up close
<!--
transition: zoom
zoom-to: Hotspot
-->
```

**Engines.** The block draws the same on every engine. The thermal engine
keeps its field dark around it; a board engine (`splitflap`) shows the block,
annotations included, in its image panel.

**Palettes.** Iron, white-hot, black-hot and lava keep their order in
lightness (hotter is lighter, darker for black-hot), so they read in gray and
for colour-blind viewers. Rainbow and arctic trade that for hue; prefer
iron for audiences you do not know.

---

## 15. Presenting: Keyboard and Mouse

The presentation window is driven by the keyboard, the mouse, or a presentation
clicker (which sends PageUp/PageDown or Enter). `mdeck spec --short` and the
in-app HUD (`H`) show the same table.

| Key | Action |
|-----|--------|
| Space, N, Right, PageDown, Enter | Next slide or next reveal step |
| P, Left, PageUp, Backspace | Previous slide (previous slides are shown fully revealed) |
| Up, Down, mouse wheel | Scroll a slide that overflows |
| Home, End | First / last slide |
| G | Grid overview; arrows move the selection, Enter / E / click opens it |
| T | Cycle transition (slide, fade, spatial, none, then any an extension registers) |
| Shift+T | Cycle theme (the built-ins, then user and deck themes) |
| F | Toggle fullscreen |
| M | Move the fullscreen slides to the next display (remembered in config) |
| H | Toggle the presenter HUD |
| V | Presenter view: a second window with the current slide, the next slide or step, the notes and the elapsed time, on another display, or beside the slides with one display. V again closes it |
| Shift+V | Reset the presenter timer |
| Digits, then Enter | Jump to that slide (the number shows in the bottom-left corner while typed; Backspace edits, Esc cancels) |
| C | Next thermal palette for every `@thermal` image and legend (section 14.20) |
| Shift+C | Thermal palettes as written |
| S | AI for the current slide, in the background: a picture on an art engine (section 9.7) |
| `.` or B | Blackout |
| R | Debug overlay (left, right, off) |
| Esc | Clear drawings on the current slide; twice within a second quits |
| Q twice, Ctrl+C twice | Quit |

| Mouse | Action |
|-------|--------|
| Left click | Next slide |
| Right click | Previous slide |
| Left drag | Freehand pen (blue) |
| Right drag | Arrow (orange) |

Drawings fade out after about eight seconds (on the thermal engine a pen
stroke is a heat trace: white-hot, cooling, gone after about four). Keys
pressed during a transition are queued and applied when it finishes, so fast
presses never lose a step.

**Presenter view.** `V` (or `mdeck deck.md --presenter`) opens the
presenter's window: the current slide large, the next slide or reveal step,
the slide's notes rendered as markdown (headings, emphasis, lists, code,
quotes, tables, math) and the elapsed time (`Shift+V` resets it). Keys typed
in either window drive the deck. With more than one display it opens on
another display than the slides, the computer's own screen when the slides
are on a projector; `M` moves the slides to the next display. With one
display the slides leave fullscreen and the two windows sit side by side,
for rehearsing; either can be dragged to another screen. `V` again closes
the presenter window and the slides go back to fullscreen.
`--theme <name>` presents in another theme without editing the deck.

**Reduced motion.** `mdeck deck.md --reduced-motion` (or
`mdeck config set defaults.reduced_motion true`) presents every slide and
reveal step in its settled state: no transitions, no entry or reveal
animations, no countdown, and engines show their finished still, as in an
export. Steps still arrive one click at a time, so a `@thermal` block goes
photo, lens, full image on separate clicks; only the motion between them is
left out.

While presenting, the file is watched: saving it reloads the deck in place and
keeps the current slide and reveal state.

**Starting.** `mdeck deck.md` presents fullscreen from the first slide.

| Option | Effect |
|---|---|
| `--windowed` | a window instead of fullscreen |
| `--slide N` | start on slide N (skips the countdown) |
| `--overview` | start in the grid overview |
| `--presenter` | open the presenter view at once |
| `--theme <name>` | present in another theme without editing the deck |
| `--engine <name>` | present on another engine (section 9.6) |
| `--reduced-motion` | settled states only (see above) |

The user config (`mdeck config show`, `mdeck config set <key> <value>`) holds
the defaults a deck does not set: `defaults.theme`, `defaults.transition`,
`defaults.start_mode` (`first`, `overview` or a slide number) and
`defaults.reduced_motion`. A deck's own settings win over the config, and the
config wins over the built-in values.

---

## 16. Export

Export renders the same slides the window shows, with the same theme, engine,
designs and chrome (footer, slide counter, logo), at any size.

```bash
mdeck export talk.md                          # PNGs: export/slide-01.png, ...
mdeck export talk.md --format pdf             # export/talk.pdf, one page per slide
mdeck export talk.md --format pdf --notes     # export/talk-notes.pdf, with notes pages
mdeck export talk.md --width 3840 --height 2160 -o out
mdeck export talk.md --slide 4                # one slide, keeping its number
mdeck export talk.md --range 3-7
```

| Option | Effect |
|---|---|
| `-o`, `--output-dir <dir>` | where the files go (default `export`) |
| `--format png\|pdf` | one PNG per slide (default), or one PDF with a page and an outline entry per slide |
| `--notes` | PDF only: a notes page per slide, the slide on top and its notes below (section 3.4) |
| `--width`, `--height` | the size in pixels (default 1920x1080) |
| `--slide N`, `--range A-B` | only these slides; file names keep the deck's numbering |
| `--theme <name>`, `--engine <name>` | another theme or engine, without editing the deck |
| `--debug` | every reveal step of every slide, as its own file |
| `--at <seconds>` | a still of the motion: run the engine this many seconds from a cold start, instead of the settled still |
| `--moment <m>` | a moment instead of the slides: `countdown` (its 3), `3`, `2`, `1`, `burst`, `end`, or `transition` (the change into `--slide`, else the second slide, halfway or `--at` seconds in) |

Every slide is exported with all its steps revealed and its engine settled,
so exports are reproducible. A PNG is exactly `--width` by `--height` pixels
on any display.

---

## 17. Checking a deck

`mdeck --check deck.md` reads the deck as presenting would and reports what
will not show as written, without opening a window. Each warning names its
slide, its line in the file and a category:

| Category | What it reports |
|---|---|
| `settings` | unknown settings (with a "did you mean"), invalid values, deck settings in a slide, duplicates, comments that look like a misspelt setting, v1 syntax with its v2 form, a chosen `design` that cannot hold the slide |
| `visual` | unknown or v1 fence tags, and lines inside a visual it cannot read (section 14.1) |
| `content` | markdown that will not show as written: an image inside running text, a remote image, an HTML `<video>`, an unknown or invalid image option |
| `engine` | content the deck's engine does not show, and an unknown `engine` |
| `point-cloud` | a `picture` that resolves nowhere, or that the slide's design cannot show |
| `assets` | generated assets that are missing or stale, and a manifest that cannot be read (section 9.7) |
| `background` | background images that are missing or unreadable, opacities that do not parse |
| `theme` | an unknown or invalid theme, fallbacks, contrast below WCAG AA |
| `thermal` | `@thermal` sources and steps the source cannot honour (section 14.20) |
| `architecture` | diagram edges that cannot be routed cleanly |
| `math` | formulas that do not parse |
| `fonts` | scripts no available font can draw |
| `extensions` | packs and extensions named in `requires` that are not installed |

`mdeck --check -v` also prints, per slide, its design and the rule that
matched, its steps and the settings that apply to it. `--engine <name>`
checks the deck on another engine. The exit status is 0 when nothing was
found and 1 otherwise, so `--check` fits in CI.

---

## 18. Extending mdeck

mdeck can be extended privately: nothing has to be published, and opening a
deck never installs or fetches anything.

**Packs** carry data only: themes, design sets, point clouds, styles and
fonts. A pack is a folder (or a zip of one) with an `mdeck-pack.yaml`
manifest and any of the folders `themes/`, `designs/`, `point-clouds/`,
`styles/` and `fonts/`:

```yaml
# mdeck-pack.yaml
name: acme-brand          # lowercase letters, digits and hyphens
version: 1.2.0
description: Acme's slide themes and pictures
min-mdeck: "2.0"          # optional
```

```bash
mdeck pack install ./acme-brand        # a folder, a .zip or a git URL; into the user folder
mdeck pack install ./acme-brand --deck # into this folder's packs/, next to the deck
mdeck pack list
mdeck pack remove acme-brand
```

Themes, design sets and point clouds in packs are found after the deck's own
and the user's, and before the built-ins: deck packs first, then user packs.
What each folder provides:

- `themes/`: themes, chosen by name (`theme: acme`).
- `designs/`: design sets a theme names with `designs:` (section 9.9).
- `point-clouds/`: `.mdpc` point clouds, used by name (`<!-- picture: name -->`).
- `styles/`: named AI styles, one `<name>.yaml` each, usable wherever a style
  name is (`--style`, `image-style`, `icon-style`, `defaults.image_style`,
  `defaults.icon_style`); the user's own styles of the same name win.
- `fonts/`: font files the pack's own themes name. A font file a pack theme
  names is looked up in the theme's folder, then in the pack's `fonts/`.

```yaml
# styles/brand.yaml
prompt: Flat shapes in Acme orange and navy, soft grain, no text
kind: image                    # image (default) or icon
references: [refs/look.png]    # optional, relative to styles/
```

**Code extensions** (engines, visual kinds, design sets, transitions) are Rust
crates built on the `mdeck-sdk` crate:

```bash
mdeck sdk new engine glow                # a crate that builds and tests as it is
mdeck sdk preview --engine glow -o shots # every design, a chart, images, a picture, both moments
mdeck build --with ./glow                # an mdeck with it inside: ./target/release/mdeck
mdeck build --with ./glow --with acme-visuals@1.2 --out ~/bin/
mdeck build --with git+ssh://git@github.com/acme/glow.git#v1.0.0   # a (private) git repository
```

`mdeck sdk new` makes an `engine`, `visual`, `design-set` or `transition`.
`mdeck build` needs a Rust toolchain: it generates a cargo project that
registers the built-ins and each extension, builds it in release mode and
copies the binary to `--out` (a file, or a folder: one that exists or ends in
a slash). `--with` takes a crate folder, a crates.io name with an optional
version, or a git URL (`git+https://`, `git+ssh://`, `git+file://` or
`git@host:org/repo`) with an optional `#tag`, `#branch` or `#commit`. An
extension's `register` may also embed themes, fonts (`Registry::font`, for
the `fonts:` of those themes) and point clouds. `mdeck extensions list` shows the installed
packs and the engines, visuals, transitions and themes this mdeck provides,
with their origin. The SDK guide is the repository's `docs/sdk/` folder.

**Visual programs** let a team draw a visual kind in any language: the user
config maps a fence tag to a command, which reads the block's content, the
theme colours and the size as JSON on stdin and writes a PNG to stdout. The
PNG is cached in `<deck>.assets/visuals/`, so presenting never waits on it.

```yaml
# config.yaml in the user folder
visuals:
  plantuml: ~/bin/plantuml-png
```

**`requires`.** A deck names the packs and extensions it expects in its
frontmatter (`requires: [acme-brand, glow]`); `mdeck --check` reports each
one that is not installed (category `extensions`).

---

## 19. Upgrading from v1

mdeck 2 breaks with the v1 syntax on purpose and does not read it. There is
no converter: this section maps every v1 construct to its v2 form, precisely
enough to hand to an AI agent with "convert this deck to mdeck 2". `mdeck
--check` recognises v1 constructs and names the v2 form, for example
`slide 4 (line 37): [settings] "@layout: quote" is v1 syntax; write <!-- design: quote -->`.

**Deck settings** (frontmatter): drop the `@`. The frontmatter is plain YAML.

| v1 | v2 |
|---|---|
| `@theme: ember` | `theme: ember` |
| `@engine`, `@transition`, `@slide-level`, `@footer` | `engine`, `transition`, `slide-level`, `footer` |
| `@countdown: true` / `false` | `countdown: on` / `off` |
| `@logo`, `@logo-position`, `@logo-opacity`, `@logo-height` | `logo`, `logo-position`, `logo-opacity`, `logo-height` |
| `@background`, `@background-opacity` | `background`, `background-opacity` |
| `@palette` | `palette` |
| `@art: <world>` | `art-world: <world>` |
| `@image-style`, `@icon-style` | `image-style`, `icon-style` |
| `@story` | removed (stories are gone) |
| `date`, `@aspect`, `@code-theme` | removed |

**Slide settings**: a visible `@key: value` line becomes a settings comment,
`<!-- key: value -->`, anywhere in the slide. Several settings can share one
comment, one `key: value` per line.

| v1 | v2 |
|---|---|
| `@layout: bullet` or `bullets` | `<!-- design: points -->` |
| `@layout: two-column` | `<!-- design: columns -->` (or nothing: `+++` is recognised) |
| `@layout: image` | `<!-- design: media -->` (or `split` for an image beside text) |
| `@layout: diagram`, `architecture`, `visualization` | `<!-- design: visual -->` |
| `@layout: title`, `section`, `quote`, `code`, `gallery`, `content` | `<!-- design: ... -->` with the same name |
| `@illustration: rocket` | `<!-- picture: rocket -->` |
| `@art: none` | `<!-- picture: none -->` |
| `@art: <scene>` (on a slide) | `<!-- picture-prompt: <scene> -->` |
| `@zoom: Hotspot` | `<!-- zoom-to: Hotspot -->` |
| `@logo`, `@background`, `@background-opacity`, `@thermal-window` | the same key in a settings comment |
| `@class` | removed |

**Structure and blocks**

| v1 | v2 |
|---|---|
| `???` and the notes after it | a ```` ```@notes ```` fenced block, anywhere in the slide |
| three blank lines as a slide break | `---` with blank lines around it, or a heading |
| `*` items revealed with the `+` item before them | nest them under that `+` item; `*` is an ordinary bullet |
| ```` ```@barchart ````, ```` ```@linechart ````, ```` ```@piechart ````, ```` ```@donutchart ```` | ```` ```@bar ````, ```` ```@line ````, ```` ```@pie ````, ```` ```@donut ```` |
| `# key: value` settings inside a visual | `key: value` (a `#` line is now a comment) |
| visual item lines without a list marker | start each item with `-` or `+` |
| `@orgchart` with `(parent: X)` | `- X -> Name` |
| `@kpi` with `(trend: up, change: +12%)` | `(trend: +12%)` |
| `@venn` with `Set: items` | `- Name (size: N)` and `- A & B: label` |
| `@flower` `centre` | `center` |
| `![prompt](image-generation)` | `![prompt](generate:)` |
| `(icon: generate-image, prompt: "...")` | `(icon: generate:, prompt: "...")` |
| `@width:80%`, `@height:100px` in image alt text | `@width: 80%`, `@height: 100px` (the space is optional) |
| `@fit`, `@left`, `@right`, `@center` in image alt text | leave them out: the design places images |
| ```` ```@story ````, ```` ```@scene ```` | removed |

**Themes and files**

| v1 | v2 |
|---|---|
| `engine: blueprint` | `engine: { name: line, surface: sheet }` |
| `engine: chalkboard` | `engine: { name: line, surface: slate }` |
| `engine: laser`, theme `etch` | removed |
| top-level `particles:`, `heat:`, `art:`, `surface:` in a theme | the same keys inside the theme's `engine:` block |
| `countdown: none`, `plain` or `burst` in a theme | `countdown: on` or `off` (the engine decides the look) |
| `<deck>.art.yaml` and `art/` | `<deck>.assets/manifest.yaml` and `<deck>.assets/artworks/` |
| point clouds in `illustrations/` (next to the deck and in the user folder) | `point-clouds/` (rename the folder; `--check` reports a v1 one) |
| `mdeck ai art`, `mdeck ai create`, `mdeck ai generate` | `mdeck ai pictures`, `mdeck ai deck`, `mdeck ai images` and `mdeck ai icons` |
| `mdeck illustration generate` | `mdeck ai point-cloud` |
| `mdeck illustration import`, `list`, `show`, `contribute` | `mdeck point-cloud import`, `list`, `show`, `contribute` |
| `--check` category `illustration` | `point-cloud` |
| `mdeck theme new --from <dir>` | `mdeck ai theme <name> --from <dir>` |
| `MDECK_EXPORT_AT`, `MDECK_EXPORT_MOMENT` | `mdeck export --at`, `--moment` |

What looks different without any change: the default theme is `dark` (plain,
standard designs, fades, no countdown); the default transition is `fade`; a
heading with a sentence is a large `statement` slide; no slide drops content;
and editorial themes such as `ember` arrange every slide editorially.
