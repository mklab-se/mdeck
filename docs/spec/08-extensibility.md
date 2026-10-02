# 08. Extensibility

How anyone extends mdeck with data or code, privately if they want to.

## Why this matters

mdeck grows when its owner finds a use good enough to include. That is not enough for others.
A company that wants its own brand theme, its own engine, or a visual that draws its own
architecture vocabulary must be able to build it **without forking mdeck and without publishing
it**. Extensions are owned by whoever writes them.

mdeck 2 has two tiers of extension:

- **Data** (no compiler needed): themes, design sets, point clouds, AI styles and fonts, as files
  next to the deck, in the user's folder, or bundled in a **pack**.
- **Code** (Rust): engines, visual kinds, design sets and transitions, written against the
  public `mdeck-sdk` crate and compiled into a **custom build** of mdeck with `mdeck build`.
  Teams that do not write Rust can add a visual kind as an **external visual program** in any
  language.

## Trust model

- **EXT-01** MUST `implemented`: Extensions are trusted like any program the user installs. An
  extension may run native code, read and write files, use the network and render anything. mdeck
  does not sandbox, sign-check or police extensions (VIS-19, NG-06).
- **EXT-02** MUST `implemented`: Only the user installs extensions, with an explicit command
  (`mdeck pack install`) or by building their own mdeck (`mdeck build`). Nothing installs
  implicitly.
- **EXT-03** MUST `implemented`: A deck never causes code to be fetched, installed or run. A deck
  may *name* the extensions it needs (`requires:`, EXT-11); when one is not installed, mdeck
  presents the deck with fallbacks (EXT-07) and `--check` says what is missing (VIS-20). This is
  the one hard security line: opening a markdown file from someone else is safe. (An external
  visual program the user configured runs when a deck uses its tag and the cached image is
  missing; the deck cannot add one.)
- **EXT-04** SHOULD `implemented`: `mdeck extensions list` shows the installed packs, the external
  visual programs, and every engine, visual, transition and theme this mdeck provides, with where
  each came from (`built-in` or the extension crate), so the owner always knows what runs.

## Extension points

| Extension point | Data or code | Examples |
|---|---|---|
| **Theme** | data | a company brand theme with fonts and logos |
| **Design set** | data (YAML arrangements) | a company's own title, quote and points designs |
| **Point cloud** | data | the company mascot as a picture |
| **AI style** | data | a company illustration style for `mdeck ai` |
| **Visual kind** | code, or an external program | `@acme-roadmap`, a chart of the company's own planning format |
| **Engine** | code | an engine that animates the company's brand motif |
| **Design set (code)** | code | a board-like renderer for a kiosk wall |
| **Transition** | code | a branded slide transition |

- **EXT-05** MUST `implemented`: Every extension point in the table is available to third parties.
  A code design set and a code transition are written with the SDK traits (`DesignSet`,
  `Transition`; `mdeck sdk new design-set|transition`), registered, and looked up by name like the
  built-ins:
  - a theme's `designs:` names a data design set first, then a registered code design set, which
    then draws every slide (as a board engine's own set does); `--check` reports what its
    `unsupported` returns;
  - a deck's, a slide's or a theme's `transition:` names a built-in or a registered transition,
    which drives the slide change (its `duration`, the `look` of both slides, `paint_over`); `T`
    cycles through the registered ones after the built-ins, and `mdeck export --moment
    transition` shows one in a still.
- **EXT-06** MUST `deferred to 2.x`: mdeck's built-ins use the same extension points, registered the
  same way (VIS-18): `mdeck::builtins` fills the same `mdeck_sdk::registry::Registry` an extension
  fills, with the built-in engines, visuals, themes and point clouds, and an extension's
  transitions and design sets are looked up in it next to the built-ins (EXT-05). *Deferred to
  2.x:* the built-in transitions and the `standard` and `editorial` design sets are not
  registered through the SDK (a spatial transition needs the overview grid, which the
  `Transition` trait does not see, and the data design sets are arrangements, not code), and
  built-in visuals draw through an internal bridge rather than `mdeck_sdk::paint`.
- **EXT-07** MUST `implemented`: Engines, visual kinds and themes are looked up in registries by
  name, not in closed enums. `--check` reports a name that resolves to nothing, and the deck falls
  back:
  - an unknown visual tag shows its source as a code block;
  - an unknown engine gives way to the theme's engine;
  - an unknown design set (a theme's `designs:`) falls back to `standard`, and an unknown theme to
    the default theme, `dark`;
  - an unknown transition passes to the next one in line (RUN-08).
- **EXT-08** MUST `implemented`: Registered names are unique: an extension registering an engine,
  visual, design set, transition, theme or point cloud under a name already taken (by a built-in
  or another extension) is an error at startup that names both. Themes and point clouds can
  still be overridden deliberately by lookup order: the deck's folders, the user's folders, packs
  (the deck's, then the user's), then what the binary registers.

## Data extensions: packs

- **EXT-09** MUST `implemented`: A **pack** is a folder (or a zip of one) with a manifest,
  `mdeck-pack.yaml` (`name`, `version`, `description`, `min-mdeck`), and any of these subfolders:
  - `themes/`: themes, chosen by name;
  - `designs/`: design sets a theme names with `designs:`;
  - `point-clouds/`: `.mdpc` point clouds, used by name;
  - `styles/`: named AI styles (a prompt and reference images);
  - `fonts/`: font files the pack's own themes name.
- **EXT-10** MUST `implemented`: Packs install from a folder, a zip or a git URL:
  - `mdeck pack install <path|zip|git-url>` installs into the user folder, `--deck` into the
    current deck folder's `packs/`;
  - `mdeck pack list` and `mdeck pack remove <name>` manage them.

  Both are private by default: nothing is published anywhere.
- **EXT-11** SHOULD `implemented`: A deck may declare the packs and extensions it expects
  (`requires: [acme-brand]` in its frontmatter), so `--check` can say "this deck expects
  acme-brand, which is not installed" and how to get it.

## Code extensions

An extension crate exposes one function, `pub fn register(r: &mut mdeck_sdk::Registry)`, that
registers what it brings. `mdeck build --with <path|crate[@version]|git-url[#ref]>...` generates
a small cargo project that depends on `mdeck` and the extensions, calls `mdeck::run` with the
built-ins plus each `register`, and builds a release binary. The same code path serves the
built-ins, so an extension is as fast and as capable as a built-in.

Custom builds were chosen over dynamic plugins and WebAssembly: Rust has no stable ABI, a C ABI
or WASM boundary would force every drawing call through a command interface, and the sandbox WASM
offers is not needed (EXT-01). A custom build needs a Rust toolchain to build, not to run.

### Requirements

- **EXT-12** MUST `implemented`: mdeck publishes an **SDK**: a Rust crate (`mdeck-sdk`) with the
  interfaces for engines, visual kinds, design sets and transitions, together with the types they
  receive (stage, theme tokens, slide content model, point clouds, published geometry, the
  drawing interface). The SDK is versioned in lockstep with mdeck: SDK 2.x supports mdeck 2.x
  (`mdeck_sdk::VERSION` is the mdeck version it ships with). See the compatibility promise below.
- **EXT-13** MUST `implemented`: A code extension is an ordinary Rust crate that depends on
  `mdeck-sdk`. It can live in a private repository and never be published.
- **EXT-14** MUST `implemented`: A user can produce an mdeck that includes any set of extension
  crates without editing mdeck's source: `mdeck build --with` takes a crate folder, a crate name
  with an optional version (`acme-engines@1.2`) or a git repository (`git+https://...#v0.2.0`).
- **EXT-15** MUST `implemented`: The SDK's documentation includes complete example extensions,
  built and tested in mdeck's CI so they cannot rot (ENG-13): the tutorial engines
  (`examples/engine-aurora`, `engine-ambience`, `engine-pictures`, `engine-reactive`) and every
  scaffold, instantiated (`examples/template-engine`, `template-visual`, `template-design-set`,
  `template-transition`), are workspace members. A test keeps each instantiated scaffold identical
  to what `mdeck sdk new` writes.
- **EXT-16** MUST `implemented`: An extension has the same quality contract as a built-in,
  written down in the SDK concepts guide:
  - deterministic stills;
  - honours reduced motion;
  - scales with the slide;
  - reports its own problems to `--check` (`mdeck_sdk::problem`);
  - works in export.
- **EXT-17** MUST `implemented`: **Custom builds** are the primary code extension mechanism, and
  `mdeck build` makes them a single command.
- **EXT-18** MUST `implemented`: **External visual programs** are a second, lightweight tier for
  visuals, for teams that do not write Rust. The user config maps a fence tag to a command
  (`visuals: { acme-roadmap: ~/bin/acme-roadmap }`); mdeck passes the fence source, the theme
  tokens and the size as JSON on stdin and reads a PNG from stdout. The image is cached in the
  deck's `<stem>.assets/visuals/`, keyed by the command and the request, so a program runs at most
  once per block, when the deck opens and its image is missing, never while presenting a frame.
  Without an image the block shows its source.
- **EXT-19** MAY `deferred to 2.x`: Dynamic loading (native plugins or WebAssembly) is
  reconsidered once the SDK has been stable across at least one major version. Nothing in the SDK
  prevents adding it later; the own drawing interface (EXT-25) keeps a WebAssembly route open.
  *Deferred:* by design, not before the SDK has proven stable.

## SDK documentation

An SDK is only as useful as its documentation. The goal: a Rust developer who has never seen
mdeck's source writes a simple engine in an afternoon, and a serious one (pictures, reactions to
charts, its own countdown and end act) without having to read mdeck's internals. The guides live
in [`docs/sdk/`](../sdk/README.md).

- **EXT-28** MUST `implemented`: **A scaffold.** `mdeck sdk new <kind> <name>` (kinds: `engine`,
  `visual`, `design-set`, `transition`) creates a crate that builds immediately. It contains:
  - a working minimal implementation with comments that explain each hook;
  - a showcase theme and a sample deck that exercises it;
  - a test with a golden image;
  - a README with the commands to build, run, export and test it.

- **EXT-29** MUST `implemented`: **A getting-started guide** (`docs/sdk/getting-started.md`, with
  `prerequisites.md` and the full-cycle `tutorial-0-your-first-engine.md`) takes a developer from nothing to their own engine running in a custom mdeck build (`mdeck build --with
  ./my-engine`), and tells them how to see it live, export stills and run its tests.
- **EXT-30** MUST `implemented`: **A tutorial in three steps.** Each step is a complete, tested
  engine in the repository (`examples/engine-ambience`, `engine-pictures`, `engine-reactive`),
  with screenshots, built on the previous one:
  1. **Ambience.** A calm animated ground: the frame lifecycle (`update`, then `paint`), the
     drawing interface, scaling, theme colours, reduced motion and deterministic stills.
  2. **Pictures and moments.** Drawing the slide's picture (a point cloud) in the engine's own
     medium; the countdown and the end act; transitions between slides; staying clear of the copy.
  3. **Reacting to content.** Using the geometry visuals publish (bars, lines, frames) to react to
     charts and stay dark behind images; typed, validated engine settings from the theme; what
     the engine needs from the theme; reporting problems to `--check`; performance.

  A board engine and a visual kind each get their own shorter guide (`design-sets.md`,
  `visuals.md`).
- **EXT-31** MUST `implemented`: **A concepts guide** (`docs/sdk/concepts.md`) explains the model
  an extension lives in: deck, slide, design, theme and engine ([01](01-concepts.md)); what the
  core does and what the extension does; the frame lifecycle and the stage; the contract every
  extension must keep (EXT-16, ENG-07 to ENG-12); and the compatibility promise (EXT-23).
- **EXT-32** MUST `implemented`: **A complete API reference.** Every public item in `mdeck-sdk`
  has rustdoc (the crate denies `missing_docs`, so CI fails on an undocumented public item), the
  examples compile as doctests, and the reference is published on docs.rs with every SDK release.
- **EXT-33** MUST `implemented`: **Tools for extension authors**, documented in the guides:
  - export a still at any moment of the motion: `mdeck export --at <seconds>`, and `--moment
    countdown|end` for the opening and ending (these replace v1's `MDECK_EXPORT_AT` and
    `MDECK_EXPORT_MOMENT` environment variables);
  - `mdeck sdk preview [--engine <name>] [--theme <name>]` exports a built-in preview deck that
    shows an extension on every design, a chart, images and a picture, plus both moments;
  - a test helper in the SDK (`mdeck_sdk::testing`) that renders engines, visuals, designs and
    transitions headlessly and compares them against golden images, which is how mdeck tests its
    own engines.
- **EXT-34** MUST `implemented`: **Honest change notes.** The documentation is versioned with the
  SDK. Each major version has a section in "Upgrading your extension" (`docs/sdk/upgrading.md`)
  that lists every breaking change with before and after code.
- **EXT-35** SHOULD `implemented`: The built-in engines are the advanced examples. Their source
  (`crates/mdeck/src/engines/`) is linked from the guides and uses only `mdeck_sdk`, so "how does
  the particles engine do this?" always has a readable answer.

## Compatibility promise

Whoever builds on mdeck (a deck author, a theme designer, a company with a private engine) must
be able to rely on it for a whole major version. The SDK's side of the promise is written out in
`docs/sdk/compatibility.md`.

- **EXT-23** MUST `implemented`: Within a major version, nothing that users build on breaks:
  - the deck format;
  - the theme, design set and pack formats;
  - the CLI commands and flags;
  - the stable SDK surface.

  A deck, theme or extension made for 2.0 works unchanged on every 2.x release. Breaking changes
  are collected and shipped only in the next major version.
- **EXT-24** MUST `implemented`: The stable SDK surface exposes **no third-party types**. Anything
  an extension touches is an mdeck type (`mdeck_sdk::paint` has its own `Color`, `Pos2`, `Vec2`,
  `Rect`, `Stroke`, `Mesh`, `Texture` and `Painter`), so upgrading a dependency can never break an
  extension by accident. The SDK depends on egui privately.
- **EXT-25** MUST `deferred to 2.x`: Extensions draw through mdeck's own drawing interface
  (`mdeck_sdk::paint`). It covers what the built-in engines use:
  - shapes, paths and meshes;
  - text laid out in the theme's fonts, glyph meshes and glyph ink;
  - images and textures;
  - clipping, opacity and sprite blending, including the additive blending the particle glow
    uses.

  mdeck's own engines use only this interface (a test keeps egui out of `engines/`). Because only
  mdeck depends on egui directly, mdeck can upgrade egui freely without touching any extension.
  *Deferred to 2.x:* built-in visuals still draw with egui through an internal bridge, and the
  painter has no math layout yet.
- **EXT-26** MAY `implemented`: An extension that needs something the drawing interface does not
  offer yet can opt into raw egui access through the `unstable-egui` cargo feature
  (`Painter::egui_painter`). That feature is explicitly outside the compatibility promise, and
  every need for it is a request to extend `mdeck_sdk::paint` in a later minor version.
- **EXT-27** MUST `implemented`: The dependency policy (keep every dependency at its latest
  version) is kept, with one constraint: an upgrade that would break anything in EXT-23 waits for
  the next major version. Thanks to EXT-24 and EXT-25 this is rare: a dependency upgrade normally
  changes only mdeck's internals and goes out in a minor or patch release. When an upgrade has to
  wait, `Cargo.toml` says why with a `# Stays on ...` comment.

## Consequences for the codebase

- **EXT-20** MUST `implemented`: The workspace has:
  - `crates/mdeck-sdk` (the public interfaces and the types they use);
  - `crates/mdeck`, the library (parser, designs, built-in extensions, app, export, commands) and
    the `mdeck` binary, which is `mdeck::run` called with `mdeck::builtins`.
- **EXT-21** MUST `implemented`: Engines and visual kinds are registries filled at startup (v1's
  closed `EngineKind` and `Chart` enums are gone); designs are a fixed catalogue of 13, and design
  sets are YAML files found by name, or code design sets in the registry (EXT-05). Capabilities
  stay as the interface between core and engine (ENG-04).
- **EXT-22** SHOULD `deferred to 2.x`: Built-in engines remain cargo features, so a custom build can
  leave out the engines it does not need (CI builds with none and with each alone). *Deferred to
  2.x:* built-in visuals are always compiled in.
