# 10. Generated assets and AI

Everything an AI produces for mdeck, and how it is stored, kept fresh and used.

mdeck uses the `ailloy` crate for chat and image generation, configured with `mdeck ai config`.
Every AI feature lives under `mdeck ai`, and every asset generated for a deck follows one model:

| What | Command | Steered by | Stored in |
|---|---|---|---|
| Images in slides | `mdeck ai images deck.md` | `![prompt](generate:)`, `image-style` | `<stem>.assets/images/` |
| Diagram icons | `mdeck ai icons deck.md` | `(icon: generate:, prompt: "...")`, `icon-style` | `<stem>.assets/icons/` |
| Pictures (artworks) | `mdeck ai pictures deck.md`, or `S` while presenting | `art-world`, `picture-prompt`, the engine's style card, the theme's `art:` | `<stem>.assets/artworks/` |
| Point clouds | `mdeck ai point-cloud deck.md` (or `--name`/`--description`) | the deck's `picture` names | `<stem>.assets/point-clouds/` (or the user folder) |
| A theme | `mdeck ai theme --from <dir> <name>` | a design system | `themes/<name>/` |
| A whole deck | `mdeck ai deck` | a file, text or stdin | a new deck |
| Agent skill | `mdeck ai skill` | the format reference | printed |
| Named styles | `mdeck ai style ...` | the user | the user config |

`mdeck ai deck.md` generates everything the deck is missing. Every deck asset is recorded in
`<stem>.assets/manifest.yaml`. Presenting never calls the AI.

## Requirements

- **GEN-01** MUST `implemented`: AI is optional (NG-07). Presenting and exporting never call an AI
  or the network (VIS-22).
- **GEN-02** MUST `implemented`: All generation lives under `mdeck ai`, one subcommand per asset
  kind:
  - `mdeck ai images`
  - `mdeck ai icons`
  - `mdeck ai pictures` (artworks)
  - `mdeck ai point-cloud`
  - `mdeck ai theme`
  - `mdeck ai deck`
  - `mdeck ai skill`

  Bare `mdeck ai deck.md` generates everything the deck is missing (`--stale`, `--force`,
  `--slide`, `--dry-run` narrow or widen it).
- **GEN-03** MUST `implemented`: Every generated asset follows one model:
  - it is stored in the deck's folder, under one directory (`<stem>.assets/`, with `images/`,
    `icons/`, `artworks/` and `point-clouds/`), with one manifest (`manifest.yaml`) that records
    for each asset its kind, what it is for (a slide by number and source hash, or a
    placeholder's prompt), its prompt, its style and its file;
  - an asset is **current**, **stale** (its source or style changed; still shown) or **pinned**
    (`state: pinned`, kept regardless and never regenerated).

  `--check` (category `assets`) reports missing and stale assets.
- **GEN-04** MUST `implemented`: Generation never rewrites the author's markdown.
  - An image placeholder stays a placeholder in the source: `![a rocket at dawn](generate:)`
    (an empty alt text has the chat model write the prompt); a diagram icon is
    `(icon: generate:, prompt: "...")`.
  - The manifest maps the placeholder to its generated file when the deck opens.
  - The placeholder can be regenerated at any time.
- **GEN-05** MUST `implemented`: There is one style system. A named style is a prompt plus optional
  reference images (`mdeck ai style`, or a pack's `styles/`). Each asset kind has a default style,
  which the deck overrides (`image-style`, `icon-style`, or `--style`) and, for artworks, the
  theme's `art:` block replaces. An engine's medium (line, tonal) is a property of the style card
  it asks for. Every asset records the style it was made in, so a change of style (including an
  edited reference image) makes it stale.
- **GEN-06** MUST `implemented`: Generated assets are ordinary files that can be committed,
  reviewed, replaced by hand, or pinned. A hand-made file in the same place is used exactly like a
  generated one.
- **GEN-07** MUST `implemented`: AI replies that feed mdeck formats are validated by loading the
  result, and the model is asked once more with the error before failing. Themes
  (`mdeck ai theme`) and the scene list `mdeck ai pictures` asks for are. A deck written by
  `mdeck ai deck` is parsed and checked; its problems go back to the model once, and what the
  second answer still has is printed after the deck is written (a deck with a warning still
  presents).
- **GEN-08** MUST `implemented`: `mdeck ai skill` produces the AI agent skill from the format
  reference, whose settings sections are generated from the language table (LANG-04), so agents
  write valid v2 decks.
- **GEN-09** MUST `implemented`: The `S` key draws the current slide's picture while presenting on
  an art engine (on other engines it says so), in the background, and never blocks the window.
