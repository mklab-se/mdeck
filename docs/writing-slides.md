<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Writing slides

Any markdown file is a deck. MDeck splits it into slides, recognises each
slide's design from what it contains, and handles overflow, so you write
content and not layout. Charts and diagrams have their own page: [Visualizations](visualizations.md).
The complete reference is the [format specification](../crates/mdeck/doc/mdeck-spec.md)
(also `mdeck spec`).

## Slides and designs

Two things create a new slide:

1. A heading (`#` or an underlined setext heading). If the file has one `#` title and `##` sections, both levels split; set `slide-level: 2` in the frontmatter to control it explicitly.
2. A `---` line with blank lines around it, for slides without a heading.

Each slide gets one of thirteen **designs** from its content. The first row
of this table that matches wins:

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

A short line is at most 120 characters; a statement has at most 2
paragraphs of at most 240 characters; the lead of a code, visual or table
slide is at most 240 characters. `mdeck --check -v` prints every slide's
design and the row that matched.

The most common slide in a talk, a heading and a sentence, is a
`statement`: the sentence in large type. A diagram with bullets, two charts,
or code beside a picture is `content`, which shows everything in reading
order. **No design drops anything**: what a design has no place for shows
in its body.

How each design looks comes from the theme's design set: the classic
centred `standard` set or the magazine-style `editorial` set (see
[Themes](themes.md#designs-and-arrangements)). A slide is the same design
in every theme. Text that does not fit first shrinks (code down to 40%,
prose to 80%), then scrolls.

Choose a design with `<!-- design: name -->` in the slide when you want a
specific one; `mdeck --check` says when the chosen design has no place for
some of the slide's content (it then shows in the body) or lacks its core
block (the slide is then `content`). Slide settings (`design`, `picture`, `logo`, `background`, ...) are
`key: value` lines in an HTML comment, invisible on GitHub; they apply to the
slide they are written in, and `mdeck --check` flags typos, invalid values
and v1 syntax. Deck settings are plain YAML keys in the frontmatter
(`theme: ember`).

Plain markdown from anywhere presents cleanly: raw HTML keeps its text (and
`<img>` its image), task lists show their boxes, reference links and
footnotes resolve (footnote text goes to the notes), GitHub alerts
(`> [!NOTE]`) become callouts, and tables keep their column alignment.

## Progressive reveal

List items that start with `+` appear one per key press, with their nested
items; `-` and `*` items are always shown. Steps count across the whole slide. The same markers work inside every
visualization, so a bar chart can grow bar by bar and a diagram can build up
connection by connection.

## Math

Formulas are LaTeX between dollar signs, typeset with KaTeX's fonts and sharp
at any resolution: `$E = mc^2$` inline, `$$...$$` centred on its own line.

```markdown
The roots of $ax^2 + bx + c = 0$ are

$$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$$
```

Dollar amounts such as `$5 and $10` stay text.

## Speaker notes

A fenced block tagged `@notes`, anywhere in the slide, is a note for the
presenter and is never shown on screen. Notes are markdown, and nothing in
them starts a new slide. AI-generated decks include detailed notes on every slide.

````markdown
# Key Decision

- We chose microservices for team autonomy

```@notes
Emphasise that this was about letting teams ship independently, not scale.
```
````

## Images

Standard markdown images work, with options in the alt text:
`@fill`, `@fit`, `@width:80%`, `@left`, `@right`. A slide with one image
is a `media` slide (`@fill` covers the whole slide), two or more are a
`gallery` (alt texts become captions), and text plus one image is a `split`
(a `@fill` image there is cut at its panel's edge). Images decode in the background, so big
photos never stall a transition.
