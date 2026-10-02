# Contributing to MDeck

Thank you for thinking about contributing. MDeck is a small project with one maintainer and a
high bar for how things look, so this guide is specific: it says what to put where, and what a
pull request needs before it can be merged. Following it is the fastest way to get your change in.

- [Ways to contribute](#ways-to-contribute)
- [Setting up](#setting-up)
- [The bar every pull request meets](#the-bar-every-pull-request-meets)
- [Contributing a theme](#contributing-a-theme)
- [Contributing an engine](#contributing-an-engine)
- [Contributing a visual kind](#contributing-a-visual-kind)
- [Contributing point clouds](#contributing-point-clouds)
- [Fixing bugs](#fixing-bugs)
- [How reviews work](#how-reviews-work)

## Ways to contribute

| You want to | Start with |
|---|---|
| Report a bug | A [bug report](https://github.com/mklab-se/mdeck/issues/new/choose) with the deck (or the smallest part of it) that shows the problem |
| Improve the docs | A pull request; small fixes need no issue |
| Share a picture | `mdeck point-cloud contribute <name>` ([below](#contributing-point-clouds)) |
| Add a theme | A [theme proposal](https://github.com/mklab-se/mdeck/issues/new/choose) issue, then a pull request ([below](#contributing-a-theme)) |
| Add a chart or diagram | An issue describing the fence, then a pull request ([below](#contributing-a-visual-kind)) |
| Add an engine | Build it as an extension first, then an [engine proposal](https://github.com/mklab-se/mdeck/issues/new/choose) ([below](#contributing-an-engine)) |

You do not need to contribute anything to MDeck to use your own work. Themes, design sets, point
clouds and styles can live in your deck folder, your user folder or a pack
([Themes](docs/themes.md#packs)), and engines, visuals and transitions can live in your own crate
built into your own mdeck with `mdeck build` ([SDK](docs/sdk/README.md)). Contribute when you
think everyone should have it.

## Setting up

You need Rust 1.95 or newer ([rustup](https://rustup.rs/)). Building on Windows also needs NASM
and CMake on `PATH`. Fork the repository, clone your fork and run:

```bash
cargo build
cargo test --workspace
cargo run -p mdeck -- samples/introducing-mdeck.md --windowed
```

[Development](docs/development.md) describes the workspace (`crates/mdeck` and
`crates/mdeck-sdk`), the sample decks and the release process. The format reference is
[`crates/mdeck/doc/mdeck-spec.md`](crates/mdeck/doc/mdeck-spec.md) (also `mdeck spec`), and the
mdeck 2 specification with its numbered requirements (`THM-14`, `ENG-17` and so on) are in
[`docs/spec/`](docs/spec/README.md). [`CLAUDE.md`](CLAUDE.md) is a dense map of the code's
patterns; it is written for AI agents but reads fine for people too.

## The bar every pull request meets

1. **CI passes.** Run the same gate locally before you push:

   ```bash
   cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
   ```

   CI also builds `mdeck` with no engines and with each engine alone
   (`cargo check -p mdeck --all-targets --no-default-features --features <engine>`), so code
   that only one engine uses must sit behind that engine's feature.
2. **Tests.** New behaviour has unit tests, including edge cases and error paths. Every bug fix
   has a regression test that fails before the fix and passes after it.
3. **Pictures for anything visual.** MDeck's first design principle is that everything on screen
   looks polished. For a change that can move a pixel, export the affected slides before and
   after and attach them to the pull request:

   ```bash
   cargo run -p mdeck --release -- export samples/layouts/code.md --slide 3 --output-dir /tmp/after
   ```

   For motion, add `--at <seconds>` (a still that far into the slide) or
   `--moment countdown|end`. Then present the deck and step through it: timing is felt, not
   seen in stills.
4. **Docs and changelog in the same pull request.** A user-visible change updates
   [`CHANGELOG.md`](CHANGELOG.md) under `## [Unreleased]`, the format reference if the format or
   a command changed, the page under [`docs/`](docs) that describes it, and the sample decks
   that show it. A pull request with the code but not the docs is not finished.
5. **One change per pull request**, with a description that says what changed and why.
6. **No em-dashes** in anything you write: docs, comments, commit messages, help text. Use a
   comma, a colon, parentheses or a new sentence. Searching your changes for U+2014 finds them (`rg '\x{2014}'`).
7. **Dependencies.** Do not add one without a reason the pull request explains. New
   dependencies are taken at their latest version.

By contributing you agree that your contribution is licensed under the [MIT License](LICENSE.md),
and that anything you add (fonts, images, point clouds) is yours to license that way or carries
a compatible license of its own.

## Contributing a theme

A theme is data: one YAML file that sets the look (colours, fonts, sizes, spacing), the design
set, the engine and its settings, the transition, the countdown and optionally a page and a logo.
Nobody needs a built-in theme to use their own: `mdeck theme new acme` writes a commented starter
into `./themes/`, and a deck selects it with `theme: acme`. A theme becomes a built-in when it is
good enough that every MDeck user should get it.

### Theme or variant?

The built-ins come in two tiers (THM-14, VIS-08):

- A **theme** is a distinctive package. It looks unlike every other built-in on a still screen:
  its own type, its own design set or arrangement choices, often its own engine.
- A **variant** is a recolouring of an existing theme: same fonts, same design set, same engine,
  other colours. `spring` and `summer` are variants of `light`; `autumn` and `winter` of `ember`.

If your theme is mostly another theme with new colours, it is a variant. Write it as one:

```yaml
name: Harbour
extends: nord
variant-of: nord
colors:
  background: "#0f1b24"
  # only what differs
```

Variants are listed after the themes in `mdeck theme list` and `Shift+T`. A colour-only
proposal submitted as a theme is accepted as a variant or not at all.

### What to add

| File | What |
|---|---|
| `crates/mdeck/themes/<name>.yaml` | The theme. `extends` the closest built-in and states only what differs. Open with a two- or three-line comment that says what the theme is, like the existing files. |
| `crates/mdeck/src/theme/lookup.rs` | One line in `BUILTIN`: `("<name>", include_str!("../../themes/<name>.yaml"))`, among the themes or after them among the variants. A theme whose engine is a cargo feature sits behind that feature (`#[cfg(feature = "led")]`), so a build without the engine leaves the theme out. |
| `samples/themes/<name>.md` or `samples/engines/<engine>.md` | A sample deck when the theme shows something no existing sample shows (a page, a logo, an engine). A variant needs none. |
| `media/gallery/theme-<name>.jpg` | One exported slide, 1280 by 720, JPEG, that shows the theme at its best. |
| Docs | The table in [`docs/themes.md`](docs/themes.md#built-in-themes), the list in section 9.1 of [`crates/mdeck/doc/mdeck-spec.md`](crates/mdeck/doc/mdeck-spec.md), the Themes section of [`GALLERY.md`](GALLERY.md), and `CHANGELOG.md`. |

**Names.** The file name is the theme's name: lowercase letters, digits, `-` and `_`, short, a
word that evokes the look (`departures`, `darkroom`). Not a brand, a product or a person. The
`name:` field inside is the display name (`Departures`).

**Fonts.** A built-in theme can use the bundled faces by name (`spectral-light`,
`hanken-light`, `hanken-regular`, `hanken-medium`, `jetbrains-mono`, `sans`, `mono`; see
`BUNDLED_FACES` in `crates/mdeck/src/theme/mod.rs`). A new face for a built-in theme is a
bigger change, so propose it in the theme issue first. It must be under the SIL Open Font License
or an equally permissive license, and it ships like the existing faces:

- the `.ttf` in `crates/mdeck/fonts/`, with its license text next to it as
  `crates/mdeck/fonts/OFL-<Family>.txt`;
- embedded and registered in `crates/mdeck/src/render/fonts/mod.rs`;
- named in `BUNDLED_FACES` (`crates/mdeck/src/theme/mod.rs`) and mapped in `bundled_family`
  (`crates/mdeck/src/theme/build/fonts.rs`).

Every font is in every mdeck download, so bring one or two weights, not a family. A theme
outside the binary (deck, user folder or pack) needs none of this: it names a `.ttf` or `.otf`
file in its own folder or the pack's `fonts/`.

### Check it

```bash
cargo run -p mdeck --release -- theme check <name>
cargo run -p mdeck --release -- theme preview <name> -o /tmp/<name>
cargo test -p mdeck theme
```

- `mdeck theme check` must report **no issues**: no fallbacks, no contrast warnings, no keys
  that do nothing on the theme's engine. Every built-in passes WCAG AA for every text colour it
  draws (THM-09); the tests `builtins_read_comfortably` and `builtins_have_no_inert_keys` in
  `crates/mdeck/src/theme/validate.rs` fail for a built-in that does not.
- `mdeck theme preview` exports one slide per design. Put the sampler (or a contact sheet of it)
  in the pull request, together with a few slides of a real deck in the theme, for example
  `samples/introducing-mdeck.md --theme <name>` and `samples/visualizations/all.md --theme <name>`:
  charts and code are where themes most often fail.
- Look at it presented, not only exported: `cargo run -p mdeck -- samples/introducing-mdeck.md --theme <name>`.

### What gets a theme accepted

- **Distinct.** Someone who knows the existing themes can tell yours apart in a thumbnail.
- **Finished.** Every design, chart, table and code block reads well in it. Charts get eight
  series colours that tell apart, code gets a fitting syntax theme.
- **Readable.** Contrast passes, from the back of a room.
- **Generic.** It is a look, not an identity. Themes that copy a company's, product's or event's
  brand (its name, logo, trademarked colours and type as a set) are declined: a brand theme
  belongs in that brand's own pack.
- **Small.** It states only what differs from the theme it extends, and needs no Rust beyond its
  line in `BUILTIN` (unless it brings a font).

A theme that does not make the built-in set is still useful: ship it as a
[pack](docs/themes.md#packs) and tell people about it.

## Contributing an engine

An engine is what brings slides to life around their content: the living ground under the
slide, the slide's picture in the engine's medium, the countdown and the end act, reactions to
charts and images, and for a board its own transitions (ENG-01). Engines are code, they are
visible on every slide, and every built-in engine is maintained for every release. So an engine
takes the longest route into MDeck, and it starts outside the repository.

### 1. Build it as an extension

Built-in engines and extension engines are written the same way, against the public SDK
(`crates/mdeck-sdk`) and nothing else of mdeck (EXT-06, ENG-13). So write yours as an extension
crate first:

```bash
mdeck sdk new engine glow       # a crate with an engine, a showcase theme, a sample deck and a test
cd glow
cargo test
mdeck build --with .            # an mdeck with your engine inside, in ./target/release/mdeck
./target/release/mdeck deck.md
```

Building against an unreleased mdeck: `mdeck build --with . --mdeck-path <your mdeck checkout>`.

The [SDK documentation](docs/sdk/README.md) takes you from there:
[getting started](docs/sdk/getting-started.md), [concepts](docs/sdk/concepts.md) (the model, the
frame lifecycle and the contract) and three tutorials, each a tested engine under
[`examples/`](examples). The API reference is `cargo doc -p mdeck-sdk --open`.

This way you never wait on a review to use your engine, and the proposal can show it working.

### 2. Propose it

Open an [engine proposal](https://github.com/mklab-se/mdeck/issues/new/choose) before writing a
pull request. Include:

- **exports or a short recording** of the engine on a title slide, a points slide, a slide with a
  picture, a chart slide and an image slide, plus its countdown and end act if it has them
  (`mdeck export deck.md --slide 2 --at 1.5`, `--moment countdown`, `--moment end`);
- **what makes it visibly distinct** from every engine MDeck has (ENG-17). Compare it with the
  closest one. "A new medium" (light, paper, mechanics, heat) is a strong case; "particles, but
  squares" is not;
- the link to your extension's repository, if it is public.

The answer is one of: yes, as a built-in; yes, but as an official optional extension (a crate in
its own repository that MDeck's docs point to, built in with `mdeck build`); or not in MDeck,
with reasons. The second is a good outcome: the engine is just as usable, and nothing about its
code changes.

### 3. Promote it to a built-in

Once the proposal is accepted, the pull request moves the extension into the repository. The
engine's code barely changes: it already uses only `mdeck_sdk`. These are the places it touches
([`crates/mdeck/doc/engines.md`](crates/mdeck/doc/engines.md) explains what the host does for a
built-in engine):

| Where | What |
|---|---|
| `crates/mdeck/src/engines/<name>.rs` or `crates/mdeck/src/engines/<name>/` | The engine: its type implementing `mdeck_sdk::engine::Engine`, its `pub static DEF: EngineDef`, and its unit tests (`mdeck_sdk::testing::Headless` renders frames headlessly). |
| `crates/mdeck/src/engines/mod.rs` | `#[cfg(feature = "<name>")] pub mod <name>;` and, in `register`, `#[cfg(feature = "<name>")] r.engine(&<name>::DEF)?;`. |
| `crates/mdeck/Cargo.toml` | A feature `<name> = []` with a comment like its neighbours, added to `default`. An art engine's feature turns on `art` too. |
| `crates/mdeck/build.rs` | The feature's name, upper case, in `ENGINES`, so the full build still checks shared engine code for dead code (`cfg(all_engines)`). |
| `.github/workflows/ci.yml` | The feature in the `features` matrix: CI builds mdeck with your engine alone. |
| `crates/mdeck/themes/<theme>.yaml` and `crates/mdeck/src/theme/lookup.rs` | A showcase theme that selects the engine, in `BUILTIN` behind the engine's feature ([Contributing a theme](#contributing-a-theme) applies). |
| `samples/engines/<name>.md` | A deck that shows what the engine is good at: titles, pictures, charts, images, its moments. |
| `media/gallery/engine-<name>*.jpg` | Two or three exported slides, 1280 by 720, JPEG. |
| Docs | [`docs/engines.md`](docs/engines.md) (a section for the engine), the engine table in [`README.md`](README.md), the engine list and table in section 9.6 of [`crates/mdeck/doc/mdeck-spec.md`](crates/mdeck/doc/mdeck-spec.md) (and the `engine: name:` comment in 9.4), the lists in `crates/mdeck/src/commands/spec.rs` (`mdeck spec --short`) where engines are named, [`docs/themes.md`](docs/themes.md), [`GALLERY.md`](GALLERY.md), [`samples/README.md`](samples/README.md), the engine table in [`docs/spec/05-engines.md`](docs/spec/05-engines.md) (ENG-17a), and `CHANGELOG.md`. |

The engine must stay inside its boundary: files under `engines/` (except `engines/host/`) use
only `mdeck_sdk`, std and helpers under `crate::engines::`, never egui or the rest of mdeck. The
test `engines::tests::engines_stay_inside_their_boundary` checks a new engine without being told
about it. If the SDK lacks something your engine needs, add it to the SDK in the same pull
request: small, additive, documented, with a doctest.

**Prove nothing else moved.** Exports are deterministic, so build the last release (or `main`)
and your branch, and compare:

```bash
scripts/engine-golden.sh <baseline-mdeck> target/release/mdeck
```

It exports the Ember, theme, engine and visualization samples with both binaries and names every
image that differs. Only images of your own engine and theme may change.
(`samples/ember/with-images.md` can differ between runs with image loading.)

### The engine contract

The pull request description ticks off each of these (the contract is in
[concepts](docs/sdk/concepts.md#the-contract) and ENG-07 to ENG-16):

- [ ] **Deterministic stills.** The same export, at any `--at` and `--moment`, gives the same
  image every time. No wall-clock time, no unseeded randomness; variation comes from the slide
  number or a hash.
- [ ] **Reduced motion.** Under `frame.reduced_motion` (`--reduced-motion`) the engine shows its
  settled state and `animating` returns `false`.
- [ ] **Scale.** Designed at 1920 by 1080, every size multiplied by `frame.scale`; it looks the
  same at 1280 by 720 and at 4K.
- [ ] **Theme tokens only.** Colours come from `frame.tokens` and the engine's settings, never
  constants that ignore the theme. It works on a light theme as well as a dark one.
- [ ] **Clear of the content.** It paints only inside the slide (or the page), stays dark or calm
  behind the copy and inside the frames visuals and images publish, and never draws over text.
- [ ] **Performance.** 60 frames per second at 4K on a 2020-class integrated GPU (check with the
  HUD, `H`). Expensive state is derived when its input changes, not every frame.
- [ ] **Honest `animating`.** It returns `false` as soon as nothing moves, so the app stops
  repainting. Only an engine that always moves says `true`.
- [ ] **Settings declared and validated.** Every key the engine reads from the theme's `engine:`
  block is a `SettingSpec` in `DEF.settings`, so `--check` and `mdeck theme check` report wrong
  types and unknown keys. What it needs from the theme is in `DEF.needs`.
- [ ] **Problems reach `--check`.** Anything the engine cannot show (a board's unsupported
  designs, for example) is reported, not silently dropped.
- [ ] **Graceful fallback.** Without a picture, artwork or optional asset, the slide still looks
  finished (ENG-16).
- [ ] **Works in export.** PNG and PDF exports (`frame.still`) show what the window shows.

### What gets an engine accepted

- **Visibly distinct.** It looks unlike every existing engine, in stills and in motion (ENG-17).
  Ten engines are a lot to keep beautiful; an eleventh has to add something new.
- **Showcase quality.** Its showcase theme and sample deck look as good as `ember`, `departures`
  or `darkroom`, on every kind of slide, not only on the title.
- **Low maintenance.** It stays inside the SDK boundary, has tests for its geometry and its
  stills, and does not need changes in the core beyond SDK additions everyone can use.
- **Maintained.** Built-in engines are kept working through every release by the maintainer.
  Say in the proposal whether you can help with that. An engine that would cost more to keep than
  it gives is better as an official extension, and that is where it goes.

## Contributing a visual kind

A visual kind is a fence (```` ```@roadmap ````) that MDeck draws as a chart or diagram, and
that reads as a code block on GitHub. Like engines, a new kind can live outside MDeck: the
[visual kind guide](docs/sdk/visuals.md) covers the `Visual` trait (`tag`, `summary`, `check`,
`steps`, `draw`), the fence grammar every visual shares, and testing. Open an issue with a sketch
of the fence and a picture before a pull request.

A built-in visual lives in `crates/mdeck/src/render/visualizations/`; the checklist is in
[`crates/mdeck/src/render/CLAUDE.md`](crates/mdeck/src/render/CLAUDE.md). In short:

- a module whose geometry is pure and unit-tested, painted separately, reusing the shared pieces
  (`grammar`, `PlotFrame`, the legend and reveal helpers, the `VIZ_FONT_*` sizes);
- registered in `visualizations::builtin::register` (and `BUILTIN_TAGS`), with its tag in
  `language::FENCES`;
- a `check` that reports mistakes in the fence instead of guessing (`--check` shows them);
- `samples/visualizations/<kind>.md`, and the kind added to
  [`samples/visualizations/all.md`](samples/visualizations/all.md);
- the format reference (section 14 of `mdeck-spec.md`, Visualization Syntax), `mdeck spec --short`
  (`commands/spec.rs`), [`docs/visualizations.md`](docs/visualizations.md), the AI deck prompt
  (`ANALYSIS_SYSTEM_PROMPT` in `commands/create/prompts.rs`) and
  `crates/mdeck/doc/ai-reference-supplement.md`, so people and AI both know it exists.

Visuals are read from the back of a room: generous spacing, large labels, consistent sizes with
the other visuals. Exports of the new kind in `dark`, `light` and `ember` go in the pull request.

## Contributing point clouds

Point clouds (`.mdpc`) are the pictures engines draw: a rocket, a server, a lightbulb. To offer
one of yours (made with `mdeck point-cloud import` or `mdeck ai point-cloud`) to the built-in set:

```bash
mdeck point-cloud contribute <name>
```

It writes a copy of the cloud as `<name>.mdpc.json` (GitHub accepts `.json` attachments, not
`.mdpc`) and opens a prefilled GitHub issue with the cloud's details and a sketch of it; drag that
file onto the issue and submit (`--no-open` prints the link instead). The maintainer reviews it and, if it reads well at a glance, adds it as
`crates/mdeck/illustrations/<name>.mdpc` (the build picks up every file there). Names are
lowercase letters, digits and hyphens. Please contribute only pictures you made or have the
right to license under MIT.

## Fixing bugs

- For an obvious bug with an obvious fix, a pull request is welcome without an issue. When you
  are not sure it is a bug, or the fix changes behaviour, open an issue first.
- Reproduce it on the smallest deck you can. Add that deck (or the slide) to the regression test
  or to the matching sample in `samples/` if it shows something the samples did not.
- Write the test first: it fails before the fix and passes after it. If an existing test was
  wrong, the pull request says why.
- For a visual bug, attach the before and after exports.
- Reference the issue as `(#123)` in the commit message, without closing keywords: the issue is
  closed after the fix is confirmed.

## How reviews work

MDeck is maintained by one person, alongside other work. Expect a first response within a week
or two; a large pull request (an engine, a theme with a font) can take longer, and may come back
with requests about how it looks rather than how it works. Pull requests that follow this guide,
with pictures and docs included, are much faster to review. If you have heard nothing after two
weeks, a friendly ping on the pull request is welcome.

A pull request may be declined even when the code is good, when the feature does not fit MDeck's
principles (simple, presentable from any markdown, polished on screen) or would cost more to
maintain than it gives. When that happens you will get the reason, and usually a way to ship the
work anyway: as a pack, an extension, or a custom build.
