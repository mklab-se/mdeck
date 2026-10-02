<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# AI features

AI is optional in mdeck: a deck without any generated asset is complete. When you want it,
mdeck uses [ailloy](https://github.com/mklab-se/ailloy) to talk to OpenAI, Anthropic, Azure
OpenAI, Microsoft Foundry, Ollama and others. Run `mdeck ai enable` once to pick a provider
(`mdeck ai config` for the full wizard, `mdeck ai test` to check it). Presenting and exporting
never call an AI and never touch the network: everything is generated ahead of time with
`mdeck ai`, stored next to the deck and versioned with it.

Everything AI makes lives under `mdeck ai`, one subcommand per kind:

| Command | Makes |
|---|---|
| `mdeck ai <deck.md>` | everything the deck is missing, all kinds below |
| `mdeck ai images <deck.md>` | images for `![prompt](generate:)` placeholders |
| `mdeck ai icons <deck.md>` | diagram icons for `(icon: generate:, prompt: "...")` |
| `mdeck ai pictures <deck.md>` | a picture per slide for an art engine |
| `mdeck ai point-cloud <deck.md>` | point clouds for `picture` names that resolve nowhere |
| `mdeck ai theme <name> --from <dir>` | a theme from a design system |
| `mdeck ai deck --input ...` | a whole deck from a file, text or stdin |
| `mdeck ai skill` | the skill that teaches AI agents to write decks |

The deck commands share four options: `--slide N` (only that slide, made again
even when current), `--stale` (only assets that have gone stale), `--force`
(current ones too) and `--dry-run` (list what would be made, make nothing).
Without them, what is missing or stale is made. Pinned assets are never
remade.

## Create a presentation from anything

```bash
mdeck ai deck --input "Git Flow for software teams" --output git-flow/
mdeck ai deck --input company-report.pdf --output report.md
mdeck ai deck --input camera-manual.docx --output manual.md
cat research-notes.txt | mdeck ai deck --output research.md
mdeck ai deck -i --input environment-report.md        # interactive: audience, purpose, mood
mdeck ai deck --input hobbits.md --prompt "For 10-year-olds, focus on the adventures"
```

MDeck extracts the text, analyses it for key points and visualization
opportunities, and writes a concise deck with varied layouts, charts, image
placeholders, and speaker notes, in the v2 format. The source stays the
handout; the deck tells the story. `--style` sets the style of its image
placeholders. Before writing, mdeck checks the deck as `mdeck --check` would;
when it finds problems it sends them back to the model once, and anything still
left is printed after the deck is written.

## Generated assets

Everything `mdeck ai` makes for `talk.md` goes in one folder next to it,
`talk.assets/`, with one manifest, `talk.assets/manifest.yaml`, that records
each asset: its kind (`artwork`, `image`, `icon`, `point-cloud`), what it is
for (a slide, or a placeholder's prompt), the prompt, the style, the file and
its state:

- **current**: made from the slide or placeholder as it reads now, in the style in use;
- **stale**: the slide or the style changed since; still shown, and `mdeck talk.md --check` says so;
- **pinned**: kept whatever the slide says, never made again. Set `state: pinned` on an entry yourself.

The deck is never rewritten: placeholders stay in the source and are matched
to their files when the deck opens. The files are ordinary images you can
commit, review or replace by hand. `mdeck talk.md --check` reports missing and
stale assets under the `assets` category.

## Images and icons

Add placeholders, then generate them:

```markdown
![A sweeping savanna at golden hour with acacia trees](generate:)
![](generate:)        <!-- no prompt: the chat model writes one from the slide -->
```

```bash
mdeck ai images slides.md             # every placeholder that has no current image
mdeck ai images slides.md --dry-run   # what would be made, and in which style
mdeck ai images --prompt "A database server" --output db.png   # one picture, no deck
```

Diagram nodes can ask for their own icons (the label is the prompt when
`prompt:` is left out):

```markdown
- Gateway (icon: generate:, prompt: "An API gateway router")
```

```bash
mdeck ai icons slides.md
mdeck ai icons --prompt "A database" --output db.png
```

Until it is generated, an image placeholder shows as a quiet card with its
prompt, and an icon as the generic node icon.

## Styles

There is one style system. A style is a prompt that carries the look plus
optional reference images. Each kind of asset has a default style: a clean
presentation look for images, a minimalist icon look for icons, the engine's
medium for pictures (line art, graphite, watercolour, a darkroom print) and
glowing particles for point clouds. A deck chooses its own with `image-style`
and `icon-style`; a theme's engine block (`kind`, `style`, `references`) sets the house style of pictures
([Themes](themes.md#the-engine-and-its-settings)). A style is a name or a literal
description:

```yaml
---
image-style: "Cinematic photography, vivid colours, dramatic lighting"
icon-style: Flat
---
```

Named styles live in your config and may carry reference images:

```bash
mdeck ai style add Cinematic "Vivid colours, dramatic lighting, sweeping vistas"
mdeck ai style add Brand "Our house illustration style" --reference look-1.png --reference look-2.png
mdeck ai style add Flat "Flat icons, two colours" --icon
mdeck ai style set-default Cinematic
mdeck ai style list
```

Installed [packs](themes.md#packs) add named styles of their own
(a `styles/` folder, one YAML file per style); `mdeck ai style list` marks
them `(pack)`, and a style of yours with the same name wins.

For images and icons the order is `--style`, then the deck, then the config
default, then the built-in default. Every asset records the style it was made
in, so changing the style makes it stale.

## Draw a picture for every slide

On an art engine (`line`, `sketch`, `watercolour` or `darkroom`, see [Engines](engines.md#art-engines-a-picture-made-for-every-slide)),
each slide gets a picture made for it, drawn in as the slide opens:

```bash
mdeck ai pictures talk.md               # every slide that takes art and has none
mdeck ai pictures talk.md --dry-run     # list the slides and where each scene comes from
mdeck ai pictures talk.md --stale       # redraw the slides you edited since
mdeck ai pictures talk.md --slide 4     # redraw one
mdeck ai pictures talk.md --node microsoft-foundry/gpt-image-2   # another image node
```

`art-world:` in the frontmatter is the deck's world and `picture-prompt:` in a slide's settings
that slide's scene; otherwise the chat model writes the scene from the
slide's copy and notes. Pictures are made four at a time (about 20 seconds
each) and kept in `talk.assets/artworks/`. Pressing `S` while presenting draws
the current slide's picture in the background.

## Point clouds

Engines that draw pictures draw a slide's `picture` name as a point cloud
([Engines](engines.md#pictures)). A name that is not built in and not in your
libraries can be generated:

```bash
mdeck ai point-cloud talk.md                    # every name the deck uses that resolves nowhere
mdeck ai point-cloud --name kettle --description "A kettle on a stove"   # into ./point-clouds
```

Generated clouds for a deck go in `talk.assets/point-clouds/`; the deck finds
them there first. `mdeck point-cloud list | show | import | contribute`
manage the libraries.

## Themes

```bash
mdeck ai theme acme --from ./brand     # read a design system and write themes/acme/
```

## AI agents

`mdeck ai skill` prints a setup guide for coding agents such as Claude Code;
`mdeck ai skill --emit` writes a ready-to-use skill file and
`mdeck ai skill --reference` prints the complete format reference for an
agent to read.
