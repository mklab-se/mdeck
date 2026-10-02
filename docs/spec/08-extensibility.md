# 08. Extensibility

How anyone extends mdeck with data or code, privately if they want to.

## Why this matters

mdeck grows when its owner finds a use good enough to include. That is not enough for others.
A company that wants its own brand theme, its own engine, or a visual that draws its own
architecture vocabulary must be able to build it **without forking mdeck and without publishing
it**. Extensions are owned by whoever writes them.

## Today

- **Themes, point clouds and fonts are already data** and can live outside the binary:
  - the deck's `themes/` and `illustrations/` folders;
  - the user's config folder;
  - fonts and logos inside a theme folder.

  These can be private today.
- **Everything else is compiled in:**
  - engines are modules in the single `mdeck` crate, registered in a closed `EngineKind` enum;
  - visual kinds are a closed `Chart` enum, with `@architecture` and `@thermal` as special cases;
  - layouts are a closed `Layout` enum.
- **The only way to add code is to fork mdeck.**
- **The engine guide** (`crates/mdeck/doc/engines.md`, "Why engines are modules, not crates")
  records the decision from #16 to keep engines as modules. Its reasoning: an engine needs nearly
  all of mdeck (stage, theme, slide, point clouds, hints, render helpers), so a separate core
  crate would hold almost everything and isolate little.

## Assessment

The #16 reasoning is correct for mdeck's *own* engines, but it answered a different question:
"does splitting our engines into crates make *our* code better?" Private extension asks
something else: "can someone else add an engine without touching our code?" Today the answer is
no, and the closed enums make it structurally impossible. Private extension requires:

1. a **public, versioned interface** for each extension point;
2. **registries by name** instead of closed enums;
3. a **way to get extension code into the running program** that does not need the mdeck source.

The useful observation from #16 survives: the extension interface is roughly "what an engine sees
today" (stage, theme tokens, slide, point clouds, published geometry). That boundary already
exists as a module boundary, and a boundary test enforces it (`engines/mod.rs:357`). It is a good
starting point for a public SDK.

## Trust model

- **EXT-01** MUST `new`: Extensions are trusted like any program the user installs. An extension
  may run native code, read and write files, use the network and render anything. mdeck does not
  sandbox, sign-check or police extensions (VIS-19, NG-06).
- **EXT-02** MUST `new`: Only the user installs extensions, with an explicit command or by building
  their own mdeck. Nothing installs implicitly.
- **EXT-03** MUST `new`: A deck never causes code to be fetched, installed or run. A deck may
  *name* an extension it needs; when that extension is not installed, mdeck presents the deck
  with fallbacks and `--check` says what is missing (VIS-20). This is the one hard security line:
  opening a markdown file from someone else is safe.
- **EXT-04** SHOULD `new`: `mdeck extensions list` shows every installed extension, where it came
  from and what it provides, so the owner always knows what runs.

## Extension points

| Extension point | Data or code | Examples |
|---|---|---|
| **Theme** | data | a company brand theme with fonts and logos |
| **Arrangements / design set** | data | a company's own title, quote and points designs |
| **Point cloud** | data | the company mascot as a picture |
| **Artwork style card** | data | a company illustration style for `mdeck ai art` |
| **Visual kind** | code | `@acme-roadmap`, a chart of the company's own planning format |
| **Engine** | code | an engine that animates the company's brand motif |
| **Design set (code)** | code | a board-like renderer for a kiosk wall |
| **Transition** | code | a branded slide transition |

- **EXT-05** MUST `new`: Every extension point in the table is available to third parties.
- **EXT-06** MUST `new`: mdeck's built-ins use the same extension points, registered the same
  way (VIS-18). Built-in engines, visuals and design sets are extensions that happen to ship with
  mdeck.
- **EXT-07** MUST `change`: Engines, visual kinds, design sets and transitions are looked up in
  registries by name, not in closed enums. `--check` reports a name that resolves to nothing,
  and the deck falls back:
  - a visual becomes a code block of its source;
  - an engine becomes `plain`;
  - a design set becomes `standard`;
  - a theme becomes the default theme.
- **EXT-08** MUST `new`: Extension names cannot shadow built-in names, except themes and point
  clouds, which the user can deliberately override by lookup order (deck, user, extension,
  built-in). Two extensions providing the same name is an error at startup that names both.

## Data extensions: packs

- **EXT-09** MUST `new`: A **pack** is a folder (or zip of one) with a manifest, `mdeck-pack.yaml`
  (name, version, description, minimum mdeck version), and any of these subfolders:
  - `themes/`
  - `designs/`
  - `point-clouds/`
  - `styles/`
  - `fonts/`
- **EXT-10** MUST `new`: Packs install from a local path or a git URL:
  - `mdeck pack install <path|url>` installs into the user folder;
  - a deck can also carry packs in a deck-local `packs/` folder.

  Both are private by default: nothing is published anywhere.
- **EXT-11** SHOULD `new`: A deck may declare the packs and extensions it expects
  (`requires: [acme-brand]` in its settings), so `--check` can say "this deck expects acme-brand,
  which is not installed".

## Code extensions

### Requirements

- **EXT-12** MUST `new`: mdeck publishes an **SDK**: a Rust crate (`mdeck-sdk`) with the
  interfaces for engines, visual kinds, design sets and transitions, together with the types they
  receive (stage, theme tokens, slide model, point clouds, published geometry, drawing context).
  The SDK is versioned in lockstep with mdeck: SDK 2.x supports mdeck 2.x, and an extension
  states the mdeck version it was built for. See the compatibility promise below.
- **EXT-13** MUST `new`: A code extension is an ordinary Rust crate that depends on `mdeck-sdk`.
  It can live in a private repository and never be published.
- **EXT-14** MUST `new`: A user can produce an mdeck that includes any set of extension crates
  without editing mdeck's source.
- **EXT-15** MUST `new`: The SDK's documentation includes a complete example extension for every
  extension point. Each example is built and tested in mdeck's CI, so it cannot rot (ENG-13). See
  [SDK documentation](#sdk-documentation) for what the documentation must cover.
- **EXT-16** MUST `new`: An extension has the same quality contract as a built-in:
  - deterministic stills;
  - honours reduced motion;
  - scales with the slide;
  - reports its own problems to `--check`;
  - works in export.

### Mechanisms considered

| Mechanism | How it works | Strengths | Weaknesses |
|---|---|---|---|
| **A. Custom build** (recommended) | mdeck becomes a library plus a thin binary. A company's crate `acme-mdeck` depends on `mdeck` and its extension crates and calls `mdeck::App::new().engine(Glow).visual(Roadmap).run()`. A helper command, `mdeck build --with acme-glow --with ../roadmap`, generates and compiles that crate, like Caddy's `xcaddy` or the OpenTelemetry Collector Builder. | Full power, full speed, no ABI problems, compile-time checked, private by construction, and the same code path as the built-ins. | Needs a Rust toolchain to build (but not to run). The custom binary is rebuilt for each mdeck release. |
| **B. Native dynamic plugins** | `.dylib` / `.so` / `.dll` loaded at startup from the user's plugin folder. | No rebuild of mdeck; drop-in installation. | Rust has no stable ABI: plugins must be built with the exact same compiler and mdeck version, or go through a C ABI or `abi_stable`. Engines paint with egui types, which do not cross a C ABI easily. High maintenance for the SDK. |
| **C. WebAssembly components** | Extensions compiled to WASM run inside mdeck (wasmtime). | Portable binaries, any source language, one build for every platform and version within an SDK major. | Drawing must go through a command interface, not egui. Particle-scale engines pay a performance cost. Large runtime dependency. The sandbox is not needed (EXT-01). |
| **D. External visual programs** | A visual kind backed by an executable: mdeck passes the fence source, the theme tokens and the size as JSON and receives SVG or PNG back. Registered in config: `visuals: { acme-roadmap: /usr/local/bin/acme-roadmap }`. Rendered before presenting and cached. | Any language, trivial to write, no Rust needed, private. | Static output only: no animation, no reveal beyond whole-image steps, no engine reactions. |

### Recommendation

- **EXT-17** MUST `new`: **Custom builds (A)** are the primary code extension mechanism. `mdeck
  build` makes them a single command.
- **EXT-18** MUST `new`: **External visual programs (D)** are a second, lightweight tier for
  visuals, for teams that do not write Rust. Their output is cached like other generated assets
  ([10](10-generated-assets-and-ai.md)), so presenting never runs them live unless the cache is
  stale.
- **EXT-19** MAY `new`: Dynamic loading (B or C) is reconsidered once the SDK has been stable
  across at least one major version. Nothing in the SDK may prevent adding it later; the own
  drawing interface (EXT-25) keeps option C open.
- **Decided:** extensions draw through mdeck's own drawing interface, not through `egui`
  directly (see EXT-25). An earlier decision to re-export `egui` was reversed, because it would
  have let every egui release break extensions in the middle of a major version.

## SDK documentation

An SDK is only as useful as its documentation. The goal: a Rust developer who has never seen
mdeck's source writes a simple engine in an afternoon, and a serious one (pictures, reactions to
charts, its own countdown and end act) without having to read mdeck's internals. If they need the
source to understand something, the documentation has a gap.

- **EXT-28** MUST `new`: **A scaffold.** `mdeck sdk new <kind> <name>` (kinds: `engine`,
  `visual`, `design-set`, `transition`) creates a crate that builds and runs immediately. It
  contains:
  - a working minimal implementation with comments that explain each hook;
  - a showcase theme and a sample deck that exercises it;
  - a test with a golden image;
  - a README with the commands to build, run, export and test it.
- **EXT-29** MUST `new`: **A getting-started guide.** It takes a developer from nothing to their
  own engine running in a custom mdeck build (`mdeck build --with ./my-engine`), in one page, and
  tells them how to see it live, export stills and run its tests.
- **EXT-30** MUST `new`: **A tutorial in three steps.** Each step is a complete, tested engine
  in the SDK repository, with screenshots, built on the previous one:
  1. **Ambience.** A calm animated ground: the frame lifecycle (`update`, then `paint`), the
     drawing interface, scaling, theme colours, reduced motion and deterministic stills.
  2. **Pictures and moments.** Drawing the slide's picture (a point cloud) in the engine's own
     medium; the countdown and the end act; transitions between slides; staying clear of the copy.
  3. **Reacting to content.** Using the geometry visuals publish (bars, lines, frames) to react to
     charts and stay dark behind images; typed, validated engine settings from the theme; what
     the engine needs from the theme; reporting problems to `--check`; performance.

  A board engine and a visual kind each get their own shorter guide.
- **EXT-31** MUST `new`: **A concepts guide.** It explains the model an extension lives in, with
  diagrams: deck, slide, design, theme and engine ([01](01-concepts.md)); what the core does and
  what the extension does; the frame lifecycle and the stage; the contract every extension must
  keep (EXT-16, ENG-07 to ENG-12); and the compatibility promise (EXT-23).
- **EXT-32** MUST `new`: **A complete API reference.** Every public item in `mdeck-sdk` has rustdoc
  with a short example, the examples compile as doctests, and the reference is published on
  docs.rs for every SDK version. CI fails on an undocumented public item.
- **EXT-33** MUST `new`: **Tools for extension authors**, documented in the guides:
  - export a still at any moment of the motion (`mdeck export --at <seconds>`, with `--moment
    countdown|end`), the documented successor of today's `MDECK_EXPORT_AT` and
    `MDECK_EXPORT_MOMENT` environment variables;
  - a preview deck that shows an extension on every design, a chart, an image and both moments
    (`mdeck sdk preview <name>`);
  - a test helper in the SDK (`mdeck_sdk::testing`) that renders frames headlessly and compares
    them against golden images, which is how mdeck tests its own engines.
- **EXT-34** MUST `new`: **Honest change notes.** The documentation is versioned with the SDK.
  Each major version has an "Upgrading your extension" page that lists every breaking change with
  before and after code.
- **EXT-35** SHOULD `new`: The built-in engines are the advanced examples. Their source is linked
  from the guides and follows the same documentation standard, so "how does the particles engine
  do this?" always has a readable answer.

## Compatibility promise

Whoever builds on mdeck (a deck author, a theme designer, a company with a private engine) must
be able to rely on it for a whole major version.

- **EXT-23** MUST `new`: Within a major version, nothing that users build on breaks:
  - the deck format;
  - the theme, arrangement and pack formats;
  - the CLI commands and flags;
  - the stable SDK surface.

  A deck, theme or extension made for 2.0 works unchanged on every 2.x release. Breaking changes
  are collected and shipped only in the next major version.
- **EXT-24** MUST `new`: The stable SDK surface exposes **no third-party types**. Anything an
  extension touches is an mdeck type, so upgrading a dependency can never break an extension by
  accident.
- **EXT-25** MUST `new`: Extensions draw through mdeck's own drawing interface
  (`mdeck_sdk::paint`). It covers what the built-in engines and visuals use today:
  - shapes, paths and meshes;
  - text laid out in the theme's fonts, including math;
  - images and textures;
  - clipping, opacity and blend modes, including the additive blending the particle glow uses.

  mdeck's own engines and visuals use the same interface (VIS-18). Today it is a thin layer over
  egui. Because only mdeck depends on egui directly, mdeck can upgrade egui freely without
  touching any extension.
- **EXT-26** MAY `new`: An extension that needs something the drawing interface does not offer
  yet can opt into raw egui access through an `unstable-egui` cargo feature. That feature is
  explicitly outside the compatibility promise, and every need for it is a request to extend
  `mdeck_sdk::paint` in the next minor version.
- **EXT-27** MUST `change`: The dependency policy (keep every dependency at its latest version) is
  kept, with one constraint: an upgrade that would break anything in EXT-23 waits for the next
  major version. Thanks to EXT-24 and EXT-25 this is rare. A dependency upgrade normally changes
  only mdeck's internals, and goes out in a minor or patch release as today. When an upgrade has
  to wait, `Cargo.toml` says why with a `# Stays on ...` comment, as the existing exceptions do.

## Consequences for the codebase

- **EXT-20** MUST `change`: The workspace splits into at least:
  - `mdeck-sdk` (the public interfaces and the types they use);
  - `mdeck` (the library: parser, designs, built-in extensions, app, export);
  - the `mdeck` binary.

  The split follows the existing engine boundary test, which becomes the SDK's definition.
- **EXT-21** MUST `change`: `EngineKind`, `Chart` and `Layout` become registries. Capabilities stay
  as the interface between core and engine (ENG-04).
- **EXT-22** SHOULD `keep`: Built-in engines and visuals remain cargo features. A custom build can
  leave out what it does not need.
