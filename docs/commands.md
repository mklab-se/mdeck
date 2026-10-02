<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Command reference

```bash
mdeck <file.md>                    # present (add --windowed, --slide N, --overview, --reduced-motion, --check)
mdeck export <file.md>             # PNG or PDF export (--width, --height, --output-dir, --debug, --slide, --range, --format, --notes, --theme)
mdeck theme list                   # Every theme visible from here (deck, user, built-in)
mdeck theme new <n>                # Starter theme in ./themes (--from <design system> with AI, --user, --force)
mdeck theme check <n>              # Errors, fallbacks and weak contrast in a theme
mdeck theme preview <n> -o <dir>   # Sampler deck in a theme, as PNGs
mdeck illustration generate --name <n> --description "..."  # New point cloud illustration via AI (--user, --force)
mdeck illustration import <image> --name <n>                # Convert an image of light strokes on dark
mdeck illustration list            # Every illustration visible from here (deck, user, built-in)
mdeck illustration show <n>        # Preview an illustration
mdeck illustration contribute <n>  # Offer one to the built-in set (prefilled GitHub issue, --no-open)
mdeck spec                         # full format specification
mdeck spec --short                 # quick reference card
mdeck config show                  # show configuration
mdeck config set <key> <value>     # defaults.theme, defaults.transition, defaults.start_mode, defaults.reduced_motion, ...
mdeck completion <shell>           # bash, zsh, fish, powershell
mdeck version                      # version banner

mdeck ai                           # AI status
mdeck ai enable | disable | test   # provider setup and check
mdeck ai config                    # interactive provider and model wizard
mdeck ai create ...                # deck from a file, prompt, or stdin
mdeck ai generate <file.md>        # generate all image placeholders (--force, --style)
mdeck ai generate-image --prompt   # single image (--icon, --output, --style)
mdeck ai art <file.md>             # a picture per slide for an art engine (--slide, --stale, --force, --dry-run, --engine, --node)
mdeck ai story <file.md>           # particle stories for the particles engine (--slide, --range, --stale, --force, --dry-run)
mdeck ai style list | add | remove | clear | set-default | set-icon-default | show-defaults
mdeck ai skill [--emit | --reference]
```

Global flags: `-q/--quiet`, `-v/--verbose`, `--no-color`.

Shell completions:

```bash
mdeck completion zsh > ~/.zfunc/_mdeck        # static
source <(COMPLETE=zsh mdeck)                  # dynamic (recommended)
```
