<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Command reference

```bash
mdeck <file.md>                    # present (add --windowed, --slide N, --overview, --theme, --engine, --presenter, --reduced-motion, --check)
mdeck export <file.md>             # PNG or PDF export (--width, --height, --output-dir, --debug, --slide, --range, --format, --notes, --theme, --engine, --at, --moment)
mdeck theme list                   # Every theme visible from here (deck, user, built-in)
mdeck theme new <n>                # Starter theme in ./themes (--user, --force)
mdeck theme check <n>              # Errors, fallbacks and weak contrast in a theme
mdeck theme preview <n> -o <dir>   # Sampler deck in a theme, as PNGs
mdeck point-cloud import <image> --name <n>   # Convert an image of light strokes on dark into a point cloud
mdeck point-cloud list             # Every point cloud visible from here (deck, user, pack, built-in)
mdeck point-cloud show <n>         # Preview a point cloud
mdeck point-cloud contribute <n>   # Offer one to the built-in set (prefilled GitHub issue, --no-open)
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
mdeck ai point-cloud <file.md>     # the deck's missing `picture` point clouds (--description)
mdeck ai point-cloud --name <n> --description "..."   # one library cloud in ./illustrations (--user, --force)
mdeck ai theme <n> --from <dir>    # a theme from a design system (--user, --force)
mdeck ai deck ...                  # a deck from a file, prompt, or stdin
mdeck ai skill [--emit | --reference]
mdeck ai enable | disable | test   # provider setup and check
mdeck ai config                    # interactive provider and model wizard
mdeck ai style list | add | remove | clear | set-default | set-icon-default | show-defaults   # add --reference <image>

mdeck sdk new <kind> <name>        # an extension crate: engine, visual, design-set or transition (--dir)
mdeck build --with <path|crate[@version]>...   # an mdeck with extensions built in (--out, --name, --mdeck-path)
mdeck pack install <path|zip|git-url>          # install a pack for the user (--deck: into ./packs)
mdeck pack list                    # packs installed for the user and in ./packs
mdeck pack remove <name>           # remove a pack (--deck)
mdeck extensions list              # packs, engines, visuals, transitions, themes and external visual programs, with origins
```

The deck forms of `mdeck ai` (bare, `images`, `icons`, `pictures`, `point-cloud`) take
`--slide N`, `--stale`, `--force` and `--dry-run`. Generated assets go in `<deck>.assets/`,
recorded in `<deck>.assets/manifest.yaml` (see [AI](ai.md#generated-assets)).

Global flags: `-q/--quiet`, `-v/--verbose`, `--no-color`.

## Extending mdeck

**Code extensions** are Rust crates written against `mdeck-sdk` ([SDK](sdk/README.md)).
`mdeck sdk new engine glow` creates one in `./glow` that builds and tests as it is; it refuses to
write into a folder that is not empty. `mdeck build --with ./glow` generates a cargo project in
the cache folder (`~/Library/Caches/mdeck/build/` on macOS, `~/.cache/mdeck/build/` on Linux)
that registers mdeck's built-ins and then each extension, compiles it in release mode, copies the
binary to `./target/release/mdeck` (or `--out`, a file or a folder; `--name` names the binary)
and prints its path. Extensions are crate folders or crates.io names (`acme-engines@1.2`). mdeck
itself comes from the checkout the running mdeck was built from (or `--mdeck-path`, or the
`MDECK_SOURCE` environment variable), else from crates.io at the running version. It needs a
Rust toolchain ([rustup.rs](https://rustup.rs)); the people you give the binary to do not.

**Packs** are data extensions: a folder (or a `.zip` of one, or a repository URL) with an
`mdeck-pack.yaml` and any of `themes/`, `designs/`, `point-clouds/`, `styles/` and `fonts/`:

```yaml
name: acme-brand          # lowercase letters, digits, hyphens; the install folder's name
version: 1.2.0
description: Acme's themes and point clouds
min-mdeck: "2.0"          # optional: older mdecks refuse the pack
```

What each folder provides:

- `themes/`: themes, chosen by name like your own (`theme: acme`).
- `designs/`: design sets a theme names with `designs:` (see
  [Themes](themes.md#designs-and-arrangements)).
- `point-clouds/`: `.mdpc` point clouds, used by name (`<!-- picture: name -->`).
- `styles/`: named AI styles, one `<name>.yaml` each, usable wherever a style name is
  (`--style`, `image-style`, `icon-style`, `defaults.image_style`) and listed by
  `mdeck ai style list`. Your own styles of the same name win.

  ```yaml
  prompt: Flat shapes in Acme orange and navy, soft grain, no text
  kind: image               # image (default) or icon
  references: [refs/look.png]   # optional, relative to styles/
  ```
- `fonts/`: font files the pack's own themes name. A pack theme writes
  `fonts: { body: AcmeSans-Regular.ttf }` and mdeck finds it in the theme's folder or,
  failing that, in the pack's `fonts/`.

`mdeck pack install` copies a pack into the `packs/<name>/` folder of your user config folder
(`mdeck config show` prints where that is; an installed pack of the same name is replaced) or,
with `--deck`, into `./packs/<name>/` next to the deck, so the deck carries it. Themes, design
sets and point clouds are looked up in the deck's own folder, then the user's, then packs (the
deck's packs first), then the built-ins.

A deck names what it expects with `requires: [acme-brand, glow]` in its frontmatter; `--check`
(category `extensions`) warns about each pack or extension this mdeck does not have.

Shell completions:

```bash
mdeck completion zsh > ~/.zfunc/_mdeck        # static
source <(COMPLETE=zsh mdeck)                  # dynamic (recommended)
```
