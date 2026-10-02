# 01. Concepts and vocabulary

mdeck has grown to the point where its own owner struggles to name its parts. That is the first
thing to fix. This document gives every concept exactly one name. The rest of the specification,
the code, the docs and the UI use these names and nothing else.

## Today: one word, many meanings

Collisions found in v1.19:

| Word | Means today |
|---|---|
| **diagram** | The `@architecture` fence (code module `diagram`, `Block::Diagram`, `Layout::Diagram`). Also `@venn` ("Venn diagram", but that is a chart). Also the docs' umbrella "charts and diagrams". |
| **chart / visualization** | The enum is `Chart`, the layout is `Visualization`, the module is `visualizations`. `@thermal` is a `Chart` variant that the visualization dispatcher does not draw. |
| **illustration** | A `.mdpc` point cloud named by `@illustration`. In code it becomes a `Figure`. |
| **figure** | A resolved illustration (`stage::Figure`). Also `FIGURES`, the seven illustration names stories treat as people. |
| **art** | AI-generated pictures for art engines. Also a theme key, a frontmatter key (the deck's "world"), a slide key (the slide's "scene") and an engine capability. |
| **scene** | The particle field's state (`particles::Scene`). Also the obsolete `@scene` fence. Also the story sidecar `<deck>.scenes.yaml`. Also the `@art` slide prompt. |
| **backdrop** | The particles engine's star/dust/galaxy/nebula field. Also the title-slide placement of a figure or art. `@background` is a third, unrelated thing. |
| **palette** | The theme's `heat.palette`. Also `@palette` (thermal images). Also the chart series colours. |
| **ember** | A theme. Also the `render::ember` module, which holds the editorial layouts every picture engine uses. Also the hint store id. |
| **particles** | An engine. Also a theme section (`particles.light/cool`) read by six engines. |
| **engine** | Four things at once: a background animation, a family of text layouts (`editorial`), a whole-slide renderer (`board`), and an AI art consumer. See [05](05-engines.md). |
| **layout** | The kind of slide (title, bullet, ...), but also the code that draws it, and the geometry inside it. |
| **thermal** | A theme, an engine, and a fenced block, which work independently of each other. |

## The v2 model

Read the model from the inside out: a **deck** has **slides**; a slide has **content**; a
**design** arranges that content; a **theme** decides how the design looks and which **engine**
brings it to life.

```
Deck ─┬─ settings (frontmatter)
      └─ Slide* ─┬─ Content: Block* (text, list, quote, code, table, image, math, Visual)
                 ├─ Design (recognised or chosen): which roles the content fills
                 ├─ Picture (optional): the one image on the slide's stage
                 └─ Notes (optional)

Theme ─┬─ Look: tokens (colours, type, sizes, spacing)
       ├─ Design set + per-design styling: how each design is arranged
       ├─ Engine choice + engine settings: the living layer
       └─ Chrome: logo, counter, footer, countdown, end
```

### Glossary

| Term | Definition | Replaces |
|---|---|---|
| **Deck** | One markdown file presented as a sequence of slides, plus the files next to it (images, sidecars, deck-local themes). | |
| **Settings** | Deck-wide options in the frontmatter (theme, engine, transition, ...). | "global directives" |
| **Slide** | One screen. Produced by splitting the deck ([02](02-markdown-and-slides.md)). | |
| **Block** | One piece of content in a slide: heading, paragraph, list, quote, code, table, image, visual, math block. | |
| **Visual** | A block that mdeck draws from structured text in a fenced block: charts, diagrams, thermal images. Content: it carries the author's meaning. | "chart", "visualization", "diagram" as umbrella |
| **Chart** | A visual that plots data (bar, line, pie, ...). | |
| **Diagram** | A visual that shows structure: nodes and connections (architecture, org chart, git graph, flower, artifact flow, Venn). | "architecture diagram" only |
| **Design** | A named kind of slide (title, points, quote, ...) with named **roles** (title, subtitle, body, media, ...). mdeck *recognises* a design from the content, or the author *chooses* one. | "layout" (as a kind) |
| **Recogniser** | The rule that says which content matches a design. | layout inference |
| **Arrangement** | Where a design places its roles on the slide, and how they look. The theme can change it. | "layout" (as geometry) |
| **Design set** | A complete set of arrangements for every design. `standard` and `editorial` are data; a board engine supplies its own. | `editorial` capability, `render::ember` |
| **Look** | A theme's tokens: colours, fonts, sizes, spacing, code syntax theme. | |
| **Theme** | A named package of a look, a design set with styling, an engine choice with settings, and chrome. | |
| **Engine** | Code that brings the slide to life: the animated layer around the content, the rendering of the slide's picture, the countdown and the end act. Never responsible for arranging text, except a **board** engine. | engine (narrowed) |
| **Board engine** | An engine that renders whole slides in its own medium (split-flap). It supplies its own design set and owns transitions. | `board` capability |
| **Picture** | The one image that sits on a slide's stage beside or behind the copy. Its *source* is a point cloud, a generated artwork, or an image file; the engine draws it in its own medium. | `@illustration` + `@art` + `Figure` + `Art` |
| **Point cloud** | A `.mdpc` file: an image reduced to importance-ordered points. One possible picture source. | "illustration" |
| **Artwork** | An AI-generated picture made for a deck in an engine's medium. One possible picture source. | "art" |
| **Decoration** | What an engine adds by itself, never authored: backdrops, formations, reactions to visuals. | backdrops, formations, hints-scenes |
| **Chrome** | Persistent furniture around slides: logo, slide counter, footer, progress line. | |
| **Opening and ending** | The countdown before the first slide and the end act after the last. | countdown, end slide, end act |
| **Step** | One press of "next" within a slide that reveals more content. | reveal step |
| **Notes** | Speaker notes: markdown in a ```` ```@notes ```` block, shown only to the presenter. | `???` |
| **Sidecar** | A file next to the deck that mdeck reads and tools write (generated artworks, data). | |
| **Generated asset** | Anything an AI produced for the deck, stored in the deck folder. | |
| **Extension** | A package (data or code) that adds themes, designs, engines, visuals or point clouds. | |

## Requirements

- **CON-01** MUST `new`: The glossary above is the vocabulary of the code, the docs, the CLI, the
  `--check` categories and error messages. A term not in the glossary is not user-facing.
- **CON-02** MUST `change`: Code module and type names follow the glossary (for example
  `render::ember` becomes the `editorial` design set; `Figure` and `Art` become `Picture`; the
  `Chart` enum becomes a visual-kind registry). Internal names may be more specific, but never
  reuse a glossary term for something else.
- **CON-03** MUST `change`: No product name doubles as a concept. A theme called "ember" is fine;
  a module, setting or capability called "ember" that is not the theme is not.
- **CON-04** MUST `new`: Content and decoration are separate. Content is what the author writes
  and is shown by every theme and engine (or reported by `--check` when it cannot be). Decoration
  is what an engine adds and may differ freely between engines.
- **CON-05** SHOULD `new`: A thing called the same in two places behaves the same in both. If
  `palette` names thermal colour maps, it names nothing else.
