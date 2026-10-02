<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Writing slides

Any markdown file is a deck. MDeck splits it into slides, picks a layout for
each from what it contains, and handles overflow, so you write content and not
layout. Charts and diagrams have their own page: [Visualizations](visualizations.md).
The complete reference is the [format specification](../crates/mdeck/doc/mdeck-spec.md)
(also `mdeck spec`).

## Slides and layouts

Three things create a new slide, and they combine freely:

1. A `---` line with blank lines around it
2. Three blank lines
3. A heading. If the file has one `#` title and `##` sections, both levels split; set `@slide-level: 2` in the frontmatter to control it explicitly.

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
| `@barchart`, `@piechart`, ... | Visualization |
| Anything else | Content |

Override with `@layout: name` on its own line under the slide's heading when
you want a specific one. Slide directives (`@layout`, `@illustration`, `@logo`, `@background`)
work wherever they stand at the top level of the slide, and `mdeck --check`
flags typos and directives that were not applied.

## Progressive reveal

List items that start with `+` appear one per key press; `*` items appear
together with the previous `+` item. The same markers work inside every
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

Everything after a `???` line is a note for the presenter and is never shown
on screen. AI-generated decks include detailed notes on every slide.

```markdown
# Key Decision

- We chose microservices for team autonomy

???

Emphasise that this was about letting teams ship independently, not scale.
```

## Images

Standard markdown images work, with optional directives in the alt text:
`@fill`, `@fit`, `@width:80%`, `@left`, `@right`. A slide with one image
becomes a full-screen image slide, two to four become a gallery, and bullets
plus an image become a split layout. Images decode in the background, so big
photos never stall a transition.
