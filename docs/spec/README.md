# mdeck 2 specification

This folder specifies mdeck 2 as built: what mdeck is, what it must do, and what it deliberately
does not do. It was written in October 2026 to decide how mdeck 2 would break with v1, and was
then brought up to date with what was built. Git history keeps the v1 analysis it started from.

## How to read it

Each area document explains its part of the model briefly and then lists its requirements. Each
requirement has an ID (`MD-07`, `THM-14`), a keyword and a status. Keywords follow RFC 2119:
**MUST**, **SHOULD**, **MAY**, **MUST NOT**.

| Status | Meaning |
|---|---|
| `implemented` | mdeck 2.0 does this, checked against the code. |
| `deferred to 2.x` | Still wanted, not in 2.0; the requirement says why. Listed below. |
| `dropped` | No longer wanted; the requirement says why (usually a later decision). |

The owner's decisions on the questions this specification raised are in
[decisions.md](decisions.md); the architecture that implements the requirements is in
[architecture.md](architecture.md).

## Documents

| # | Document | What it answers |
|---|---|---|
| 00 | [Vision and principles](00-vision.md) | What mdeck is for, the principles every other requirement is judged against, and non-goals. |
| 01 | [Concepts and vocabulary](01-concepts.md) | One name per concept: deck, slide, design, look, theme, engine, visual, picture, decoration, chrome. Start here if you are lost. |
| 02 | [Markdown and slides](02-markdown-and-slides.md) | How a markdown file becomes slides: splitting, supported markdown, notes, steps. |
| 03 | [Slide designs](03-slide-designs.md) | Which designs exist, how mdeck recognises them, and how a theme restyles them. |
| 04 | [Themes](04-themes.md) | What a theme is and what comes with one. |
| 05 | [Engines](05-engines.md) | What an engine is responsible for, and what it is not. |
| 06 | [Visuals and pictures](06-visuals-and-pictures.md) | Charts, diagrams, thermal images, images, math, point clouds and artworks. |
| 07 | [The authoring language](07-authoring-language.md) | Everything mdeck adds on top of markdown. |
| 08 | [Extensibility](08-extensibility.md) | How anyone, including a company, extends mdeck privately with data or code. |
| 09 | [Presenting, export and checking](09-presenting-export-check.md) | The window, keys, presenter view, transitions, export parity, `--check`. |
| 10 | [Generated assets and AI](10-generated-assets-and-ai.md) | Everything the AI produces, and how it is stored and kept fresh. |
| | [Architecture](architecture.md) | The architecture decisions (D1 to D16) behind the requirements. |
| | [Decisions](decisions.md) | The owner's decisions on the questions this specification raised. |

## Deferred to 2.x

DEFERRED_LIST

## Relationship to other documents

- `crates/mdeck/doc/mdeck-spec.md` is the **format reference**, embedded in the binary and
  printed by `mdeck spec`. It describes the language as shipped, precisely enough for people and
  AI agents to write decks. Design is decided here and then documented there; where the two
  disagree about behaviour, the format reference describes what the binary does and the
  difference is a defect.
- `docs/*.md` are the user guides, and `docs/upgrading-from-v1.md` maps every v1 construct to
  its v2 form.
- `docs/sdk/` and `crates/mdeck/doc/engines.md` are the guides for extension and engine authors;
  they follow [05](05-engines.md) and [08](08-extensibility.md).
