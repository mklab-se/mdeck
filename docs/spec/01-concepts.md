# 01. Concepts and vocabulary

This document gives every concept in mdeck exactly one name. The rest of the specification, the
docs, the CLI and the `--check` messages use these names and nothing else.

## The model

Read the model from the inside out: a **deck** has **slides**; a slide has **content**; a
**design** arranges that content; a **theme** decides how the design looks and which **engine**
brings it to life.

```
Deck ─┬─ settings (frontmatter)
      └─ Slide* ─┬─ Content: Block* (text, list, quote, code, table, image, math, Visual)
                 ├─ Settings (an HTML comment in the slide)
                 ├─ Design (recognised or chosen): which roles the content fills
                 ├─ Picture (optional): the one image on the slide's stage
                 └─ Notes (optional, ```@notes blocks)

Theme ─┬─ Look: tokens (colours, type, sizes, spacing)
       ├─ Design set + arrangements: how each design is arranged
       ├─ Engine choice + engine settings: the living layer
       └─ Chrome: logo, counter, footer, countdown, end
```

### Glossary

| Term | Definition |
|---|---|
| **Deck** | One markdown file presented as a sequence of slides, plus the files next to it (images, its generated assets, deck-local themes, design sets and point clouds). |
| **Setting** | A `key: value` option. A **deck setting** is written in the frontmatter; a **slide setting** in an HTML comment in the slide, where it overrides the deck's value. |
| **Slide** | One screen. Produced by splitting the deck ([02](02-markdown-and-slides.md)). |
| **Block** | One piece of content in a slide: heading, paragraph, list, quote, callout, code, table, image, visual, math block. |
| **Visual** | A block that mdeck draws from structured text in a fenced block (```` ```@bar ````): charts, diagrams, thermal images. Content: it carries the author's meaning. |
| **Chart** | A visual that plots data (bar, line, pie, ...). |
| **Diagram** | A visual that shows structure: nodes and connections (architecture, org chart, git graph, flower, artifact flow, Venn). |
| **Design** | A named kind of slide (title, points, quote, ...) with named **roles** (title, subtitle, body, media, ...). mdeck *recognises* a design from the content, or the author *chooses* one with `design:`. |
| **Recogniser** | The ordered table of rules that says which content matches which design. |
| **Arrangement** | Where a design places its roles on the slide, and how they look. A theme can override it. |
| **Design set** | A complete set of arrangements for every design. `standard` and `editorial` are built-in YAML files; more can be added as data; a board engine supplies its own. |
| **Look** | A theme's tokens: colours, fonts, sizes, spacing, code syntax theme. |
| **Theme** | A named package of a look, a design set with arrangements, an engine choice with settings, and chrome. A **variant** is a theme that recolours another (`variant-of:`). |
| **Engine** | Code that brings the slide to life: the animated layer around the content, the drawing of the slide's picture, the countdown and the end act. Never responsible for arranging text, except a **board** engine. |
| **Board engine** | An engine that renders whole slides in its own medium (split-flap). It supplies its own design set and owns transitions. |
| **Picture** | The one image that sits on a slide's stage beside or behind the copy, named with `picture:`. Its *source* is a generated artwork, a point cloud or an image file; the engine draws it in its own medium. |
| **Point cloud** | A `.mdpc` file: an image reduced to importance-ordered points. One possible picture source, found by name in `point-clouds/` folders and the built-ins. |
| **Artwork** | An AI-generated picture made for a slide in an engine's medium. One possible picture source. |
| **Decoration** | What an engine adds by itself, never authored: backdrops, formations, reactions to visuals. |
| **Chrome** | Persistent furniture around slides: logo, slide counter, footer, progress line. |
| **Opening and ending** | The countdown before the first slide and the end act after the last. |
| **Step** | One press of "next" within a slide that reveals more content (a `+` item). |
| **Notes** | Speaker notes: markdown in a ```` ```@notes ```` block, shown only to the presenter (presenter view, PDF notes pages). |
| **Generated asset** | Anything an AI produced for the deck (artworks, images, icons, point clouds), stored in `<deck>.assets/` and recorded in its `manifest.yaml`. |
| **Pack** | A folder of data extensions (themes, design sets, point clouds, styles, fonts) that mdeck reads without building anything. |
| **Extension** | A code crate that registers engines, visuals, themes or point clouds through `mdeck-sdk`, built into a custom mdeck with `mdeck build`. |

## Requirements

- **CON-01** MUST `implemented`: The glossary above is the vocabulary of the docs, the CLI, the
  settings, the `--check` categories and error messages. A term not in the glossary is not
  user-facing.
- **CON-02** MUST `deferred to 2.x`: Code module and type names follow the glossary. Internal names
  may be more specific, but never reuse a glossary term for something else. The SDK and the
  user-facing surface do (`Picture`, design names, fence tags as the visual registry key).
  *Deferred:* two internal modules keep v1 names: `render::ember` (the editorial chrome) and
  `render::illustration` (point clouds).
- **CON-03** MUST `deferred to 2.x`: No product name doubles as a concept. A theme called "ember"
  is fine; a module, setting or capability called "ember" that is not the theme is not. No setting,
  capability or CLI name does. *Deferred:* the internal `render::ember` module and the
  `ember-hints` store id remain.
- **CON-04** MUST `implemented`: Content and decoration are separate. Content is what the author
  writes and is shown by every theme and engine (or reported by `--check` when it cannot be).
  Decoration is what an engine adds and may differ freely between engines.
- **CON-05** SHOULD `implemented`: A thing called the same in two places behaves the same in both.
  `palette` names thermal colour maps (the `palette` deck setting, a `@thermal` block's
  `palette:`, the thermal engine's `palette`) and nothing else.
