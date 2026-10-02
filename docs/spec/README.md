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
| `deferred to 2.x` | Still wanted, but not (or not fully) in 2.0; the requirement says what is missing and why. Listed below. |
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

Everything mdeck 2.0 leaves for a 2.x release, with the requirements that record it. The
release notes (`CHANGELOG.md`, "Deferred to 2.x") carry the same list.

**Extensions and the SDK**

- Built-in visuals still draw with egui through an internal bridge (the SDK's `unstable-egui`
  feature), not through the SDK's `paint` interface, which has no math layout yet; `@thermal` and
  `@architecture` keep their own renderers (VIS-18, VIZ-07, EXT-25).
- The built-in transitions and the `standard` and `editorial` design sets are not registered
  through the SDK, and mdeck never looks up a transition or a code design set an extension
  registers: a theme's `transition:` takes the built-in names, its `designs:` a YAML design set,
  and a code design set works only as a board engine's own (VIS-18, DES-14, EXT-05, EXT-06).
- Only the engine examples and scaffold are compiled in CI; the visual, design-set and transition
  scaffolds are checked for their files only (EXT-15).
- Built-in visuals are not cargo features; engines are (EXT-22).
- Dynamic loading (native plugins or WebAssembly), by design not before the SDK has proven stable
  (EXT-19).
- The SDK content model is converted from the parser's: quotes and callouts reach an extension as
  one run of text and lists carry no start number (the split-flap board numbers lists from 1). A
  heading reaches engines as one `Hint::Text` per glyph, without letter spacing.

**Pictures and visuals**

- A generated artwork shows on any slide the art pipeline has one for, also where the design has
  no stage; point clouds follow the design (ENG-14).
- Diagram icons and point clouds have separate name vocabularies (`database` against `db`)
  (VIZ-10).

**Checking**

- `--check -v` does not say that a theme's engine settings are ignored when the deck or
  `--engine` runs another engine (THM-11, ENG-11).
- `--check --json` (RUN-22).
- Editor completion generated from the language table (LANG-04).

**Presenting, export and AI**

- Video or animated export (RUN-18, decision Q13).
- The split-flap board shows the empty panel colour while a panel image is still loading.
- Inline images in a copy column sit in a fixed box (60% of the column width, at most 400 px), so
  a portrait image is letterboxed.
- Italic display text is a synthetic slant; no italic faces are bundled.

**Code**

- mdeck's tests that read `docs/`, `samples/` or `examples/` run from the repository only, not
  from the published crate.

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
