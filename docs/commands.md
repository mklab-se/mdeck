<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Command reference

```bash
mdeck <file.md>                    # present (add --windowed, --slide N, --overview, --reduced-motion, --check)
mdeck export <file.md>             # PNG or PDF export (--width, --height, --output-dir, --debug, --slide, --range, --format, --notes, --theme)
mdeck theme list                   # Every theme visible from here (deck, user, built-in)
mdeck theme new <n>                # Starter theme in ./themes (--user, --force)
mdeck theme check <n>              # Errors, fallbacks and weak contrast in a theme
mdeck theme preview <n> -o <dir>   # Sampler deck in a theme, as PNGs
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

mdeck ai                           # AI status (also `mdeck ai status`)
mdeck ai <file.md>                 # every asset the deck is missing: pictures, images, icons, point clouds
mdeck ai images <file.md>          # images for ![prompt](generate:) (--style); or --prompt "..." [--output]
mdeck ai icons <file.md>           # diagram icons for icon: generate: (--style); or --prompt "..." [--output]
mdeck ai pictures <file.md>        # a picture per slide for an art engine (--engine, --node)
mdeck ai point-cloud <file.md>     # the deck's missing @illustration names (--description)
mdeck ai point-cloud --name <n> --description "..."   # one library cloud in ./illustrations (--user, --force)
mdeck ai theme <n> --from <dir>    # a theme from a design system (--user, --force)
mdeck ai deck ...                  # a deck from a file, prompt, or stdin
mdeck ai skill [--emit | --reference]
mdeck ai enable | disable | test   # provider setup and check
mdeck ai config                    # interactive provider and model wizard
mdeck ai style list | add | remove | clear | set-default | set-icon-default | show-defaults   # add --reference <image>
```

The deck forms of `mdeck ai` (bare, `images`, `icons`, `pictures`, `point-cloud`) take
`--slide N`, `--stale`, `--force` and `--dry-run`. Generated assets go in `<deck>.assets/`,
recorded in `<deck>.assets/manifest.yaml` (see [AI](ai.md#generated-assets)).

Global flags: `-q/--quiet`, `-v/--verbose`, `--no-color`.

Shell completions:

```bash
mdeck completion zsh > ~/.zfunc/_mdeck        # static
source <(COMPLETE=zsh mdeck)                  # dynamic (recommended)
```
