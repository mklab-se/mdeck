## Writing a deck: a guide for AI agents

The format reference above is complete. This part is the short version of what matters when you
write or convert a deck for someone.

### The shape of a good deck

- Start with frontmatter: `title`, `author` and, when the talk wants a look, `theme` (plain YAML
  keys, no `@`). Without a theme the deck is `dark`: plain, calm and readable.
- One idea per slide. A heading at the slide level starts a slide; you rarely need `---`.
- Write for meaning and let mdeck choose the design: a heading and a sentence becomes a big
  `statement`, a heading and a list `points`, a quote `quote`, a chart `visual`. Choose a design
  with `<!-- design: name -->` only when recognition gets it wrong.
- Keep slides short. A `points` slide reads best with three to six items of one line each.
- Use `+` items for things the presenter wants to reveal one at a time; `-` and `*` are static.
  Nest items under a `+` item to reveal them together.
- Put speaker notes on every slide in a ```` ```@notes ```` block: the slide's intent and how to
  deliver it. Notes are markdown and never show on the slide.
- Use `+++` lines to split a slide into columns for comparisons.
- Write formulas in LaTeX (KaTeX syntax): `$E = mc^2$` inline, `$$...$$` on a line of its own.

### Visuals

Charts and diagrams are fenced blocks tagged with the visual's name (```` ```@bar ````,
```` ```@architecture ````). Inside: `key: value` settings first, then `-` or `+` items with
optional `(key: value)` attributes, relations as `- A -> B: label`, and `#` comments.

- `@bar`, `@line`, `@stackedbar`, `@scatter` for numbers over categories or time; `@pie` and
  `@donut` for shares; `@kpi` for a few headline numbers with trends (`(trend: +12%)`);
  `@progress` for completion; `@funnel` for conversion; `@radar` to compare profiles.
- `@timeline` for dated events, `@gantt` for plans (durations `10d`, `5wd`, `2w`, `3m`,
  `after Task`).
- `@architecture` for systems: components with `(icon: database, pos: 2,1)`, then relations;
  `+` relations build the diagram step by step.
- `@orgchart` (`- Manager -> Report`), `@venn` (`- Name (size: N)`, `- A & B: label`),
  `@wordcloud`, `@gitgraph` (lanes, commits, `branch a -> b`, `merge a -> b`, tags),
  `@flower` (`- center Platform: ...` and one `- petal Team: ...` per team, 3 to 8 petals) and
  `@artifactflow` (`producer`, `service` and `consumer` lines, then `A -> B: artifact` edges).
- `@thermal` for infrared images: `image:` a grayscale white-hot export, `visible:` the same
  scene as a photo, then `+ lens X% Y% R%`, `+ reveal`, `+ above 85%` and `- spot Name X% Y%`.
  Without a `mapping:` or `data:` file never write temperatures as measurements.

### Pictures, images and generated assets

- `<!-- picture: rocket -->` puts a point cloud on the slide's stage on engines that draw
  pictures (every engine but `plain` and `splitflap`, on designs with a stage). Built in:
  person, hooded, man, woman, thermographer, presenter-up, presenter-down, box, orb, doc, docs,
  inbox, db, cloud, laptop, folder, mail, gate, server, robot, agent-friendly, agent-evil, ai,
  phone, globe, lock, gear, rocket, punchcard, camera, gauge, glasses, eyes, flag, blackhole,
  account, lightbulb, question. A name that resolves nowhere can be generated with
  `mdeck ai point-cloud <deck.md>`.
- `![A specific, concrete scene](generate:)` asks for a generated image; the alt text is the
  prompt, so be specific about subject, mood and composition. `(icon: generate:, prompt: "...")`
  on an `@architecture` component asks for a generated icon.
- On the art engines (`line`, `sketch`, `watercolour`, `darkroom`) every slide with a stage can
  get a drawing made for it: `art-world: ...` in the frontmatter sets the deck's world,
  `<!-- picture-prompt: ... -->` a slide's scene, `<!-- picture: none -->` skips a slide.
- Nothing is generated while presenting. Run `mdeck ai <deck.md>` once to make every missing
  asset; everything lands in `<deck>.assets/` with `manifest.yaml`, and the deck is never
  rewritten. Ungenerated placeholders show as quiet cards with their prompt.

### Checking your work

Always run `mdeck --check <deck.md>` after writing or editing a deck, and fix what it reports.
`mdeck --check -v <deck.md>` prints each slide's design and the rule that matched, so you can
see whether a slide became what you meant. To look at a slide, export it:
`mdeck export <deck.md> --slide 4 -o /tmp/look` and read the PNG.

### Converting a v1 deck

Use section 19 of the format reference. In short: frontmatter keys lose their `@`; visible
`@key: value` slide lines become `<!-- key: value -->` comments (`@layout` is `design`,
`@illustration` is `picture`); `???` notes become a ```` ```@notes ```` block; `@barchart`,
`@linechart`, `@piechart` and `@donutchart` are `@bar`, `@line`, `@pie` and `@donut`;
`# key: value` lines inside visuals lose their `#`. Then run `mdeck --check`, which names the
v2 form of anything left over.

## Command reference

### Presenting and checking

```bash
mdeck <file.md>                # Present fullscreen
mdeck <file.md> --windowed     # In a window
mdeck <file.md> --slide 5      # Start on slide 5
mdeck <file.md> --overview     # Start in the grid overview
mdeck <file.md> --presenter    # With the presenter view (notes, next slide, timer)
mdeck <file.md> --theme ember  # In another theme, without editing the deck
mdeck <file.md> --engine plain # On another engine
mdeck <file.md> --check        # Validate without opening a window (exit 1 on warnings)
mdeck <file.md> --check -v     # Plus each slide's design, steps and settings
```

### The format

```bash
mdeck spec                     # The full format reference
mdeck spec --short             # The quick reference card
mdeck completion <shell>       # Shell completions (bash, zsh, fish, powershell)
```

### Export

```bash
mdeck export <file.md>                             # PNGs, 1920x1080, in ./export
mdeck export <file.md> --width 3840 --height 2160  # Another size
mdeck export <file.md> --output-dir ./slides       # Another folder
mdeck export <file.md> --slide 7 --debug           # One slide, every reveal step
mdeck export <file.md> --range 3-5                 # A range of slides
mdeck export <file.md> --format pdf                # One PDF, a page per slide
mdeck export <file.md> --format pdf --notes        # Notes pages: slide on top, notes below
mdeck export <file.md> --theme winter              # In another theme
mdeck export <file.md> --at 3                      # A still of the engine's motion at 3 s
mdeck export <file.md> --moment end                # The countdown or the end instead
```

### Themes

```bash
mdeck theme list                        # Every theme visible from here, then the variants
mdeck theme new <name>                  # A commented starter theme in ./themes/<name>.yaml
mdeck ai theme <name> --from <dir>      # Convert a design system folder with AI
mdeck theme check <name>                # Errors, fallbacks, weak contrast, inert keys
mdeck theme preview <name> -o <dir>     # A sampler deck in the theme, one slide per design
```

Custom themes are YAML files (format reference, section 9.4): `themes/<name>.yaml` or
`themes/<name>/theme.yaml` next to the deck, or in the user folder. Unset keys come from
`extends` (default `dark`); unknown keys are errors. To turn a design system into a theme
yourself: read its rules and tokens, map its roles with the table in section 9.4, write the
file, run `mdeck theme check`, then `mdeck theme preview` and look at the PNGs; repeat until it
looks like the brand. `designs: editorial` gives the magazine layout, `arrangements:` adjusts
any design (section 9.9). Logos (section 9.5) and background images (section 9.8) are deck
settings too: `logo: file.svg`, `background: file.jpg`, `background-opacity: 30%`.

### Configuration

```bash
mdeck config show              # The current configuration and its file
mdeck config set <key> <value> # Set a value
```

Keys: `defaults.theme` (a built-in or user theme), `defaults.transition` (`fade`, `slide`,
`spatial`, `none`), `defaults.start_mode` (`first`, `overview` or a slide number),
`defaults.reduced_motion`, `defaults.image_style` and `defaults.icon_style`.

### AI

```bash
mdeck ai                        # AI status (also: mdeck ai status)
mdeck ai config                 # Choose the provider and models
mdeck ai enable | disable | test
mdeck ai <file.md>              # Every asset the deck is missing
mdeck ai images <file.md>       # Images for ![prompt](generate:)
mdeck ai icons <file.md>        # Icons for (icon: generate:, prompt: "...")
mdeck ai pictures <file.md>     # A drawing per slide, on an art engine
mdeck ai point-cloud <file.md>  # Point clouds for picture names that resolve nowhere
mdeck ai images --prompt "..." --output path.png   # One image
mdeck ai deck --input report.pdf --output talk.md  # A deck from a file, text or stdin
mdeck ai deck --input doc.md --prompt "For engineers" --output talk.md
mdeck ai skill --emit           # This skill, as a file
```

The deck forms take `--slide N`, `--stale`, `--force` and `--dry-run`.

### Styles

```bash
mdeck ai style list
mdeck ai style add <name> "<description>" [--reference look.png] [--icon]
mdeck ai style remove <name> [--icon]
mdeck ai style set-default <name>
mdeck ai style set-icon-default <name>
mdeck ai style show-defaults
```

A deck picks a style with `image-style:` and `icon-style:` in its frontmatter (a style name or a
literal description); otherwise the configured default applies.

### Extending

```bash
mdeck pack install <folder|zip|git-url>   # Themes, designs, point clouds, styles, fonts
mdeck pack list | remove <name>
mdeck extensions list                     # What this mdeck provides
mdeck sdk new engine <name>               # A Rust extension crate (engine, visual, design-set, transition)
mdeck build --with ./<crate>              # An mdeck with the extension inside
```
