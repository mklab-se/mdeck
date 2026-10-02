<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Command reference

Every command prints its own help with `--help`.

## Presenting and checking

```bash
mdeck talk.md                      # present fullscreen
  --windowed                       #   in a window
  --slide N                        #   start on slide N
  --overview                       #   start in the grid overview
  --presenter                      #   open the presenter view
  --theme <name>                   #   in another theme
  --engine <name>                  #   on another engine
  --reduced-motion                 #   settled states, no motion
mdeck talk.md --check              # validate without opening a window (exit status 1 on problems)
mdeck talk.md --check -v           #   with each slide's design, steps and settings
```

See [Presenting](presenting.md) and [Writing slides](writing-slides.md#check-your-deck).

## Export

```bash
mdeck export talk.md               # a PNG per slide in ./export
  -o, --output-dir <dir>           #   where the files go
  --format png|pdf                 #   one PNG per slide, or one PDF
  --notes                          #   PDF: notes pages, slide on top and notes below
  --width <px> --height <px>       #   size (default 1920x1080)
  --slide N | --range 3-7          #   only some slides
  --debug                          #   every reveal step
  --theme <name> --engine <name>   #   another theme or engine
  --at <seconds>                   #   a still of the engine's motion
  --moment countdown|3|2|1|burst|end   # the countdown or the end act instead of the slides
```

See [Export](export.md).

## Themes and pictures

```bash
mdeck theme list                   # every theme visible from here: themes, then variants
mdeck theme new <name>             # a starter theme in ./themes, every key commented (--user, --force)
mdeck theme check <name|file>      # errors, fallbacks, contrast, keys that do nothing
mdeck theme preview <name|file> -o <dir>   # one slide per design, as PNGs (--width, --height)

mdeck point-cloud list            # every point cloud visible from here
mdeck point-cloud show <name>     # a preview image
mdeck point-cloud import <image> --name <name>   # from an image of light strokes on dark (--user, --force)
mdeck point-cloud contribute <name>   # offer one to the built-in set: a prefilled GitHub issue (--no-open)
```

See [Themes](themes.md) and [Engines](engines.md#pictures).

## AI

```bash
mdeck ai                           # status (also `mdeck ai status`)
mdeck ai enable | disable | test | config   # set up and check a provider
mdeck ai <talk.md>                 # every asset the deck is missing: pictures, images, icons, point clouds
mdeck ai images <talk.md>          # images for ![prompt](generate:) placeholders (--style)
mdeck ai images --prompt "..." --output img.png   # one image, no deck
mdeck ai icons <talk.md>           # diagram icons for (icon: generate:) (--style; or --prompt, --output)
mdeck ai pictures <talk.md>        # a picture per slide on an art engine (--engine, --node)
mdeck ai point-cloud <talk.md>     # point clouds for picture names that exist nowhere (--description)
mdeck ai point-cloud --name <n> --description "..."   # one for your library (--user, --force)
mdeck ai theme <name> --from <dir> # a theme from a design system (--user, --force)
mdeck ai deck --input <file|text>  # a whole deck (--output, --prompt, -i, --style; or stdin)
mdeck ai skill                     # setup guide for AI agents (--emit: the skill file, --reference: the format reference)
mdeck ai style list | add | set | remove | clear | set-default | set-icon-default | show-defaults
```

The deck forms (bare, `images`, `icons`, `pictures`, `point-cloud`) share `--slide N`,
`--stale`, `--force` and `--dry-run`. Generated assets go in `<deck>.assets/`, recorded in
`<deck>.assets/manifest.yaml`. See [AI features](ai.md).

## Extending

```bash
mdeck sdk new <kind> <name>        # an extension crate: engine, visual, design-set or transition (--dir)
mdeck build --with <path|crate[@version]|git-url[#ref]>   # an mdeck with extensions built in (repeat --with; --out, --name, --mdeck-path)
mdeck pack install <folder|zip|git-url>     # install a pack for you (--deck: into ./packs)
mdeck pack list                    # packs installed for you and in ./packs
mdeck pack remove <name>           # remove a pack (--deck)
mdeck extensions list              # packs, engines, visuals, transitions, themes and external visual programs, with origins
```

**Code extensions** are Rust crates written against `mdeck-sdk` ([SDK](sdk/README.md)).
`mdeck sdk new engine glow` creates one in `./glow` that builds and tests as it is (it never
writes into a folder that is not empty). `mdeck build --with ./glow` generates a cargo project in
the cache folder (`~/Library/Caches/mdeck/build/` on macOS, `~/.cache/mdeck/build/` on Linux)
that registers mdeck's built-ins and then each extension, compiles it in release mode, copies the
binary to `./target/release/mdeck` (or `--out`, a file or a folder; `--name` names the binary)
and prints its path (`--out dist/`, with the slash, makes the folder). Extensions are crate
folders, crates.io names (`acme-engines@1.2`), or git repositories, private ones included:
`git+https://github.com/acme/aurora#v0.2.0`, `git+ssh://git@github.com/acme/aurora.git#main`,
`git@github.com:acme/aurora.git#1a2b3c4d` or `git+file:///srv/git/aurora.git`. After `#` comes a
tag, a branch or a commit (`#tag=`, `#branch=` or `#rev=` when the name is ambiguous; without `#`,
the default branch); the crate's name is the repository's, or `?package=<name>` before the `#`.
Cargo fetches with your own git credentials or SSH agent (set `CARGO_NET_GIT_FETCH_WITH_CLI=true`
if a private repository that `git clone` can read fails to fetch). A rebuild updates branches and
crates.io requirements to their newest match; tags and commits stay fixed. See
[Packaging and sharing](sdk/packaging-and-sharing.md). mdeck
itself comes from the checkout the running mdeck was built from (or `--mdeck-path`, or the
`MDECK_SOURCE` environment variable), else from crates.io at the running version. Building needs
a Rust toolchain ([rustup.rs](https://rustup.rs)); the people you give the binary to do not.

**Packs** are data extensions: themes, design sets, point clouds, AI styles and fonts as plain
files ([Themes](themes.md#packs)).

**Visuals in any language.** `visuals:` in the config maps a fence tag to a program that reads the
block, the theme's colours and the size as JSON on stdin and writes a PNG; its output is cached in
`<deck>.assets/visuals/`, so presenting never runs it. See
[Writing a visual kind](sdk/visuals.md#visuals-in-any-language).

A deck names what it expects with `requires: [acme-brand, glow]` in its frontmatter; `--check`
(category `extensions`) warns about each pack or extension this mdeck does not have.

## Everything else

```bash
mdeck spec                         # the full format reference
mdeck spec --short                 # a one-page quick reference card
mdeck config show                  # your configuration
mdeck config set <key> <value>     # defaults.theme, defaults.transition, defaults.start_mode, defaults.reduced_motion, ...
mdeck completion <shell>           # bash, zsh, fish, powershell
mdeck version                      # version and build information
```

Global flags: `-q/--quiet`, `-v/--verbose`, `--no-color`.

Shell completions:

```bash
mdeck completion zsh > ~/.zfunc/_mdeck        # static
source <(COMPLETE=zsh mdeck)                  # dynamic (recommended)
```
