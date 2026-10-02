<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Writing slides

Any markdown file is a deck. MDeck splits it into slides, picks a layout for
each from what it contains, and handles overflow, so you write content and not
layout. Charts and diagrams have their own page: [Visualizations](visualizations.md).
The complete reference is the [format specification](../crates/mdeck/doc/mdeck-spec.md)
(also `mdeck spec`).

## Slides and layouts

Two things create a new slide:

1. A heading (`#` or an underlined setext heading). If the file has one `#` title and `##` sections, both levels split; set `slide-level: 2` in the frontmatter to control it explicitly.
2. A `---` line with blank lines around it, for slides without a heading.

Each slide gets a layout from its content:

| Content | Layout |
|---------|--------|
| Heading + subtitle | Title |
| Lone heading | Section divider |
| Heading + bullet list | Bullet |
| Heading + code block | Code |
| Blockquote + attribution | Quote |
| Single image | Full-screen image |
| Two or more images | Gallery |
| Bullets + image | Split layout |
| `+++` separator | Two columns |
| `@architecture` block | Architecture diagram |
| `@bar`, `@pie`, ... | Visualization |
| Anything else | Content |

Override with `<!-- design: name -->` in the slide when you want a specific
one. Slide settings (`design`, `picture`, `logo`, `background`, ...) are
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
becomes a full-screen image slide, two to four become a gallery, and bullets
plus an image become a split layout. Images decode in the background, so big
photos never stall a transition.
