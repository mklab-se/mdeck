# mdeck specification (v2 draft)

This folder defines what mdeck is, what it must do, and what it deliberately does not do. It was
retrofitted in October 2026 from the code as of v1.19.0, and written critically: where today's
design is weak, the requirement describes what mdeck *should* be, not what it happens to be.

The next major version may break backward compatibility. These documents are where we decide how.

## How to read it

Every area document has the same three parts:

1. **Today.** How mdeck v1.19 actually behaves, verified against the code (with `file:line`
   references where it matters). Facts only.
2. **Assessment.** What is wrong, unclear, inconsistent or not worth its cost.
3. **Requirements.** What mdeck v2 must do. Each requirement has an ID, a keyword and a status.

Keywords follow RFC 2119: **MUST**, **SHOULD**, **MAY**, **MUST NOT**.

Each requirement's status says how it relates to today:

| Status | Meaning |
|---|---|
| `keep` | Today's behaviour, now written down. |
| `change` | Exists today but must change; usually a breaking change. |
| `new` | Does not exist today. |
| `remove` | Exists today and should go. |

Decisions made on the questions this spec raised are recorded in [open-questions.md](open-questions.md)
and marked **Decided** where they apply.

## Documents

| # | Document | What it answers |
|---|---|---|
| 00 | [Vision and principles](00-vision.md) | What mdeck is for, the principles every other requirement is judged against, and non-goals. |
| 01 | [Concepts and vocabulary](01-concepts.md) | One name per concept: deck, slide, design, look, theme, engine, visual, picture, decoration, chrome. Start here if you are lost. |
| 02 | [Markdown and slides](02-markdown-and-slides.md) | How a markdown file becomes slides: splitting, supported markdown, notes, reveal. |
| 03 | [Slide designs](03-slide-designs.md) | Which kinds of slide exist, how mdeck recognises them, and how a theme restyles them. |
| 04 | [Themes](04-themes.md) | What a theme is and what comes with one. |
| 05 | [Engines](05-engines.md) | What an engine is responsible for, and what it is not. |
| 06 | [Visuals and pictures](06-visuals-and-pictures.md) | Charts, diagrams, thermal images, images, math, illustrations, art, stories. |
| 07 | [The authoring language](07-authoring-language.md) | Everything mdeck adds on top of markdown, and the proposed v2 syntax. |
| 08 | [Extensibility](08-extensibility.md) | How anyone, including a company, extends mdeck privately with data or code. |
| 09 | [Presenting, export and checking](09-presenting-export-check.md) | The window, keys, transitions, export parity, `--check`. |
| 10 | [Generated assets and AI](10-generated-assets-and-ai.md) | Everything the AI produces, and how it is stored and kept fresh. |
| | [Known defects](known-defects.md) | Bugs and doc drift found while writing this spec, to fix regardless of v2. |
| | [Removal candidates](removal-candidates.md) | Features that may not earn their place. |
| | [Decisions](open-questions.md) | The owner's decisions on open questions, and any still open. |

## Relationship to other documents

- `crates/mdeck/doc/mdeck-spec.md` is the **user-facing format reference** embedded in the binary
  (`mdeck spec`). It describes v1 as shipped. Once a v2 decision is made here, the format reference
  is rewritten to match; it is never the place where design is decided.
- `docs/*.md` are the user guides.
- `crates/mdeck/doc/engines.md` is the contributor guide for engines. It will follow
  [05-engines.md](05-engines.md) and [08-extensibility.md](08-extensibility.md).
