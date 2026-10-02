# 10. Generated assets and AI

Everything an AI produces for mdeck, and how it is stored, kept fresh and used.

## Today

mdeck uses the `ailloy` crate for chat and image generation, configured in ailloy's own config.
There are several separate AI paths, each with its own command, storage and freshness rules:

| What | Command | Steered by | Stored in | Freshness |
|---|---|---|---|---|
| Images in slides | `mdeck ai generate deck.md` | `![prompt](image-generation)`, `@image-style` | `images/`; the deck file is rewritten in place | n/a (placeholder replaced) |
| Diagram icons | `mdeck ai generate deck.md` | `(icon: generate-image, prompt: ...)`, `@icon-style` | `media/diagram-icons/`; the deck is rewritten | n/a |
| One-off image | `mdeck ai generate-image` | flags | a file | n/a |
| A whole deck | `mdeck ai create` | a prompt or an input file | a new deck | n/a |
| Point clouds | `mdeck illustration generate` (not under `ai`) | name + description | deck or user `illustrations/` | n/a |
| Artworks | `mdeck ai art deck.md`, or `S` while presenting | `@art` (deck world, slide scene), the theme's `art:`, style cards | `art/` + `<deck>.art.yaml` | by slide hash and style; pinned by number |
| Stories | `mdeck ai story deck.md`, or `S` | ```` ```@story ```` fence, `@story` | `<deck>.scenes.yaml` | by slide hash; pinned |
| A theme | `mdeck theme new --from <dir>` | a design system | `themes/<name>/` | n/a |
| Named styles | `mdeck ai style ...` | user | config | n/a |
| Agent skill | `mdeck ai skill` | n/a | emits a skill for AI agents | n/a |

Presenting never calls the AI.

## Assessment

1. **Five ways to generate a picture, four style systems:**
   - `@image-style`;
   - `@icon-style`;
   - named styles;
   - artwork style cards plus the theme's `art:`.

   The point cloud generator sits outside `mdeck ai`.
2. **Two freshness models.**
   - Placeholders are rewritten into the deck: the deck changes, and there is no way to
     regenerate.
   - Sidecars keep the deck clean and track staleness.

   The second is the better model.
3. **The settings that steer generation are mixed in with presentation settings** (`@art`), and
   share names with unrelated things ([01](01-concepts.md)).

## Requirements

- **GEN-01** MUST `keep`: AI is optional (NG-07). Presenting and exporting never call an AI or the
  network (VIS-22).
- **GEN-02** MUST `change`: All generation lives under `mdeck ai`, one subcommand per asset kind:
  - `mdeck ai images`
  - `mdeck ai icons`
  - `mdeck ai pictures` (artworks)
  - `mdeck ai point-cloud`
  - `mdeck ai theme`
  - `mdeck ai deck`
  - `mdeck ai skill`

  Bare `mdeck ai deck.md` generates everything the deck is missing.
- **GEN-03** MUST `change`: Every generated asset follows one model:
  - it is stored in the deck's folder, under one directory (`<deck>.assets/` or `assets/`), with
    one manifest that records for each asset what it is for, its prompt, its source hash and its
    style;
  - an asset is **current**, **stale** (its source changed) or **pinned** (kept regardless).
  `--check` reports missing and stale assets.
- **GEN-04** MUST `change`: Generation never rewrites the author's markdown.
  - An image placeholder stays a placeholder in the source: `![a rocket at dawn](generate:)` or
    similar.
  - The manifest maps the placeholder to its generated file.
  - The placeholder can be regenerated at any time.
  - **Decided:** the placeholder is `![prompt](generate:)`.
- **GEN-05** MUST `change`: There is one style system. A named style is a prompt plus optional
  reference images. Each asset kind has a default style, which the deck can override and the
  theme can suggest. An engine's medium (line, tonal) is a property of the style card it asks for.
- **GEN-06** MUST `keep`: Generated assets are ordinary files that can be committed, reviewed,
  replaced by hand, or pinned. A hand-made file in the same place is honoured exactly like a
  generated one.
- **GEN-07** MUST `keep`: AI replies that feed mdeck formats (themes, decks) are
  validated by loading the result, and the model is asked once more with the error before
  failing.
- **GEN-08** MUST `keep`: `mdeck ai skill` produces the AI agent skill from the same language
  table as the format reference (LANG-04), so agents always write valid v2 decks.
- **GEN-09** MUST `keep`: The `S` key generates the current slide's missing asset while presenting
  and never blocks the window.
