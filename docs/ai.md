<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# AI features

MDeck uses [ailloy](https://github.com/mklab-se/ailloy) to talk to OpenAI,
Anthropic, Azure OpenAI, Ollama, and others. Run `mdeck ai enable` once to
pick a provider; everything below is optional.

## Create a presentation from anything

```bash
mdeck ai create --input "Git Flow for software teams" --output git-flow/
mdeck ai create --input company-report.pdf --output report.md
mdeck ai create --input camera-manual.docx --output manual.md
cat research-notes.txt | mdeck ai create --output research.md
mdeck ai create -i --input environment-report.md        # interactive: audience, purpose, mood
mdeck ai create --input hobbits.md --prompt "For 10-year-olds, focus on the adventures"
```

MDeck extracts the text, analyses it for key points and visualization
opportunities, and writes a concise deck with varied layouts, charts, image
placeholders, and speaker notes. The source stays the handout; the deck tells
the story.

## Generate images

Add placeholders, then generate them all at once:

```markdown
![A sweeping savanna at golden hour with acacia trees](image-generation)
```

```bash
mdeck ai generate slides.md          # generates every placeholder, rewrites the paths
mdeck ai generate-image --prompt "A database server" --icon --output db.png
```

Control the look with named styles or an inline description:

```yaml
---
@image-style: "Cinematic photography, vivid colours, dramatic lighting"
@icon-style: "Clean minimalist icon, subtle 3D feel"
---
```

```bash
mdeck ai style add Cinematic "Vivid colours, dramatic lighting, sweeping vistas"
mdeck ai style set-default Cinematic
mdeck ai style list
```

Diagram nodes can request their own icons:

```markdown
- Gateway (icon: generate-image, prompt: "An API gateway router")
```

## Draw a picture for every slide

On an art engine (`blueprint` or `sketch`, see [Engines](engines.md#art-engines-a-drawing-made-for-every-slide)),
each slide gets a picture made for it, drawn in as the slide opens:

```bash
mdeck ai art talk.md               # every slide that takes art and has none
mdeck ai art talk.md --dry-run     # list the slides and where each scene comes from
mdeck ai art talk.md --stale       # redraw the slides you edited since
mdeck ai art talk.md --slide 4     # redraw one
mdeck ai art talk.md --node microsoft-foundry/gpt-image-2   # another image node
```

`@art:` in the frontmatter is the deck's world and under a slide's heading
that slide's scene; otherwise the chat model writes the scene from the
slide's copy and notes. Pictures are made four at a time (about 20 seconds
each), kept in `art/` next to the deck, and recorded in `talk.art.yaml`. A
theme's `art:` block can set the house style ([Themes](themes.md#pages-and-art)).

## AI agents

`mdeck ai skill` prints a setup guide for coding agents such as Claude Code;
`mdeck ai skill --emit` writes a ready-to-use skill file and
`mdeck ai skill --reference` prints the complete format reference for an
agent to read.
