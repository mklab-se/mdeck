## CLI Command Reference

### Presenting

```bash
mdeck <file.md>                # Launch presentation (fullscreen)
mdeck <file.md> --windowed     # Launch in a window
mdeck <file.md> --slide 5      # Start on slide 5
mdeck <file.md> --overview     # Start in grid overview mode
mdeck <file.md> --check        # Validate presentation without launching GUI
```

### Format Specification

```bash
mdeck spec                     # Print full format specification
mdeck spec --short             # Print quick reference card
mdeck completion <shell>       # Shell completions (bash, zsh, fish, powershell)
```

### Export

```bash
mdeck export <file.md>                          # Export slides as PNG (1920x1080)
mdeck export <file.md> --width 3840 --height 2160  # Export at custom resolution
mdeck export <file.md> --output-dir ./slides    # Export to specific directory
mdeck export <file.md> --slide 7 --debug        # One slide, every reveal step (fast way to check a slide)
mdeck export <file.md> --range 3-5              # A range of slides
mdeck export <file.md> --format pdf             # One PDF, a page per slide (export/<file>.pdf)
mdeck export <file.md> --format pdf --notes     # PDF of notes pages: slide on top, speaker notes below
mdeck export <file.md> --theme winter           # In another theme, without editing the deck
```

### Themes

```bash
mdeck theme list                        # Every theme visible from here (deck, user, built-in)
mdeck theme new <name>                  # Commented starter theme in ./themes/<name>.yaml
mdeck ai theme <name> --from <dir>      # Convert a design system folder with AI (copies fonts and logos)
mdeck theme check <name>                # Errors, fallbacks, weak contrast (exit 1 when invalid)
mdeck theme preview <name> -o <dir>     # Sampler deck in the theme, as PNGs to look at
```

Custom themes are YAML files (spec section 9.4): `themes/<name>.yaml` or
`themes/<name>/theme.yaml` next to the deck, or in the user folder. Unset keys
come from `extends` (default `dark`); unknown keys are errors. To turn a design
system into a theme yourself: read its rules and tokens, map its roles to slide
roles with the table in spec section 9.4, write the file, run `mdeck theme
check`, then `mdeck theme preview` and look at the PNGs; repeat until it looks
like the brand. Logos (spec section 9.5): a theme's `logo:` block, or
`logo: file.svg` (plus `logo-position`, `logo-opacity`, `logo-height`) in
any deck's frontmatter; PNG with transparency or SVG. Background images (spec
section 9.8): `background: file.jpg` and `background-opacity: 30%` in the
frontmatter put an image behind every slide; the same keys in a slide.s
settings comment override it there, and `background: none` turns it off. Keep the
opacity low (the default 0.3) behind text.

### Configuration

```bash
mdeck config show              # Display current configuration
mdeck config set <key> <value> # Set a config value
```

Available config keys:
- `defaults.theme`: default theme, a built-in (`light`, `dark`, `nord`, `ember`,
  `spring`, `summer`, `autumn`, `winter`) or a user theme
- `mdeck ai pictures <deck.md> [--slide N] [--stale] [--force] [--dry-run] [--engine E] [--node N]`:
  draw a picture per slide for an art engine (`line`, `sketch`, `watercolour`, `darkroom`) with the image model; pictures go in
  `<deck>.assets/artworks/` and `<deck>.assets/manifest.yaml` records them. `art-world: "..."` in the frontmatter is
  the deck's world (setting, era, characters); `picture-prompt: "..."` in a slide's settings is that slide's scene
  (otherwise the chat model writes one from the copy and notes); `<!-- picture: none -->` skips a slide.
  Only title, section, quote, bullet and copy slides take art. Pictures never contain text.
- `mdeck ai point-cloud <deck.md>` (the deck's missing names) or `mdeck ai point-cloud --name <n> --description "..."`: make a point cloud
  for the particle field (`--user` for the user library, `--force` to overwrite);
  `mdeck point-cloud import <image> --name <n>` converts an existing image, `list` shows what
  resolves from here, `show <n>` previews. A slide asks for one with `<!-- picture: <n> -->` at its
  top; bullet, content, quote and section slides show it beside the copy, title slides behind
  it, other layouts never. Built in: person, hooded, man, woman, thermographer, presenter-up,
  presenter-down, box, orb, doc, docs, inbox, db, cloud, laptop, folder, mail, gate, server,
  robot, agent-friendly, agent-evil, ai, phone, globe, lock, gear, rocket, punchcard, camera,
  gauge, glasses, eyes, flag, blackhole, account, lightbulb, question. Story cast kinds are the same names. `mdeck point-cloud contribute <n>` offers a deck or user cloud
  to the built-in set through a prefilled GitHub issue (`--no-open` prints the link).
- `defaults.transition` — default transition (`fade`, `slide`, `spatial`, `none`)
- `defaults.start_mode` — `first`, `overview`, or a slide number
- `defaults.reduced_motion` — `true` or `false`: present with reduced motion by default
- `defaults.image_style` / `defaults.icon_style` — default AI image / icon style names (yours
  or a pack's)

### AI Commands

```bash
mdeck ai                       # Show AI status
mdeck ai status                # Show AI status (same as above)
mdeck ai enable                # Enable AI features
mdeck ai disable               # Disable AI features
mdeck ai config                # Interactive AI provider configuration
mdeck ai test                  # Test AI integration
```

### AI Presentation Creation

```bash
mdeck ai deck --input <file-or-text> --output <path>  # Create presentation from content
mdeck ai deck --input report.pdf --output slides.md    # From PDF
mdeck ai deck --input manual.docx --output slides.md   # From DOCX
mdeck ai deck --input "A talk about Rust" --output slides.md  # From text prompt
cat notes.txt | mdeck ai deck --output slides.md       # From piped input
mdeck ai deck -i --input doc.md --output slides.md     # Interactive mode
mdeck ai deck --input doc.md --prompt "For engineers" --output slides.md  # With audience context
```

### AI Image Generation

```bash
mdeck ai <file.md>                       # Generate every asset the deck is missing
mdeck ai images <file.md>                # Images for ![prompt](generate:) placeholders
mdeck ai images <file.md> --dry-run      # List what would be generated
mdeck ai images <file.md> --style name   # Override the image style
mdeck ai icons <file.md>                 # Diagram icons for icon: generate:
mdeck ai images --prompt "..."           # Generate a single image
mdeck ai images --prompt "..." --style "watercolor"
mdeck ai icons --prompt "..."            # Generate a single icon
mdeck ai images --prompt "..." --output path.png
```

### AI Style Management

```bash
mdeck ai style list                        # List all defined styles
mdeck ai style add <name> <description>    # Add a named image style
mdeck ai style add <name> <desc> --icon    # Add a named icon style
mdeck ai style add -i                      # Interactive style creation (AI-assisted)
mdeck ai style remove <name>               # Remove a named style
mdeck ai style remove <name> --icon        # Remove a named icon style
mdeck ai style clear                       # Remove all styles and reset defaults
mdeck ai style set-default <name>          # Set default image style
mdeck ai style set-icon-default <name>     # Set default icon style
mdeck ai style show-defaults               # Show current default styles
```

## AI Image Generation Guide

### Marking Images for Generation

Use `generate:` as the image path to mark an image for AI generation:

```markdown
![A futuristic cityscape at sunset](generate:)
```

The alt text becomes the image prompt. Leave alt text empty for auto-prompting from slide context (requires chat capability):

```markdown
![](generate:)
```

### Image Style Control

Image styles control the visual aesthetic of all generated images. Styles can be set at multiple levels (highest priority first):

1. **Per-file frontmatter:** `image-style: watercolor` (name or literal description)
2. **Config default:** `mdeck ai style set-default <name>`
3. **Hardcoded fallback:** A built-in default style

For icons (used in architecture diagrams):
1. **Per-file frontmatter:** `icon-style: flat-design`
2. **Config default:** `mdeck ai style set-icon-default <name>`
3. **Hardcoded fallback:** A built-in default icon style

### Diagram Icon Generation

In architecture diagrams, use `icon: generate:` with a `prompt` to mark a node for AI icon generation:

````markdown
```@architecture
- Gateway (icon: generate:, prompt: "An API gateway router icon", pos: 1,2)
- Database (icon: database, pos: 2,2)
- Gateway -> Database: queries
```
````

### The `mdeck ai images` Workflow

1. Write your presentation with `generate:` placeholders and/or diagram icon prompts
2. Run `mdeck ai <file.md>` (or `mdeck ai images` / `mdeck ai icons`)
3. The command detects orientation automatically (horizontal for full-slide images, vertical for side-panel layouts)
4. It applies the configured image style
5. The files go in `<deck>.assets/` and are recorded in `<deck>.assets/manifest.yaml`; the markdown is never rewritten, so a placeholder can be regenerated at any time

### Tips for AI Agents Writing Presentations

- Use descriptive alt text for image generation prompts — be specific about the scene, mood, and composition
- Set `image-style` in frontmatter when the presentation has a consistent visual theme
- Use `+` list markers for incremental reveal (appears on forward press)
- Nest items under a `+` item to reveal them in the same step
- Use `-` for static items that are always visible
- Keep slide content concise — presentations are meant to be visual aids, not documents
- Use the `---` separator or 3+ blank lines between slides
- Architecture diagrams with `+`/`*` markers create animated build-up sequences
- Use `@gitgraph` for git branching diagrams — declare lanes, add commits, fork with `branch source -> target`, merge with `merge source -> target`, and tag with `tag branch: "label"`; supports progressive reveal
- Use `@flower` when one platform or shared capability serves several peer teams that also contribute back: `- center Platform: ...`, then one `- petal Team: ...` per team (keep descriptions to a short sentence; 3-8 petals read best)
- Use `@artifactflow` for supply chains of builds, packages or images: `producer`, `service` and `consumer` lines, then `A -> B: artifact (icon: package)` edges; indented `- item` lines list what a service holds
- For infrared and thermal-imaging talks, use `theme: thermal` (headings form in heat) and put each thermal picture in a `@thermal` block: `image:` a grayscale white-hot export, `visible:` the same scene as a photo, then `+ lens X% Y% R%`, `+ reveal`, `+ above 85%` and `- spot Name X% Y%`. Without a `mapping:` or `data:` file never write temperatures as measurements (spot text is marked as author-supplied); ordinary images are never treated as thermal
- Use `design: columns` with `+++` separator for side-by-side comparisons
- Write formulas in LaTeX (KaTeX syntax): `$E = mc^2$` inline, `$$\frac{-b \pm \sqrt{b^2-4ac}}{2a}$$` on a line of its own; run `mdeck <file> --check` to catch formulas that do not parse
- Add speaker notes in a ```@notes block on every slide: explain the slide's intent and delivery guidance
- Use `mdeck ai deck` to generate presentations from any content, then refine with an AI agent
