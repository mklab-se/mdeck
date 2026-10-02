# Packaging and sharing

How to bundle what you made (engines, themes, fonts, point clouds) and get it to the people who
should use it: colleagues, customers, or everyone.

**What you will learn:**

- the difference between **code** (engines, visuals, design sets, transitions) and **data**
  (themes, fonts, point clouds, styles), and why it decides how you share;
- how to bundle an engine with its themes and fonts in one crate, the usual company case;
- when a separate theme pack is better;
- every way to distribute, with the exact commands, and which one fits you;
- how your versions relate to mdeck's over time.

**Before you start:** you have an extension that builds, for example the `aurora` engine from
[Your first engine](tutorial-0-your-first-engine.md). Code blocks are labelled as in the
tutorials: **TYPE** (you write it), **READ** (shown for understanding), **RUN** (terminal
commands, with the folder to run them in).

## Code or data: two kinds of extension

| | Data | Code |
|---|---|---|
| What | themes, design sets, point clouds, AI styles, fonts | engines, visual kinds, design sets in code, transitions |
| Written in | YAML and files | Rust, against `mdeck-sdk` |
| Shipped as | a **pack**: a folder (or zip, or git repository) with an `mdeck-pack.yaml` | a **crate**, built into mdeck with `mdeck build` |
| Users install it with | `mdeck pack install <folder|zip|git-url>` | `mdeck build --with <path|crate|git-url>` (or they receive a built binary) |
| Users need | mdeck | mdeck, Rust and a C toolchain (or your binary) |
| Changing it needs | a text editor | a Rust developer and a rebuild |

mdeck never runs code from a pack: a theme can choose an engine, but only one that the running
mdeck already has, built in or built into it with `mdeck build`. That is why an engine always
travels as a crate.

**Rule of thumb:** if you can do it in a theme, do it in a theme. Write an engine only for motion
or drawing no theme setting can give you.

## Bundling an engine with its themes and fonts

The usual company case: a brand engine plus one or more brand themes that use it, with the
company's fonts. Put them in **one crate**, and `register` registers all of it. Colleagues then
get everything with one `mdeck build --with`.

```text
acme-brand/
├── Cargo.toml
├── src/lib.rs
├── themes/
│   ├── acme.yaml            the dark brand theme
│   └── acme-light.yaml      the light one
├── fonts/
│   ├── AcmeSans-Regular.ttf
│   └── AcmeSerif-Light.ttf
└── point-clouds/
    └── acme-logo.mdpc       a picture engines can draw (`<!-- picture: acme-logo -->`)
```

**TYPE** `src/lib.rs`, the `register` function:

```rust
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.engine(&DEF)?;
    // Fonts first, under the file names the themes use.
    r.font("AcmeSans-Regular.ttf", include_bytes!("../fonts/AcmeSans-Regular.ttf"))?;
    r.font("AcmeSerif-Light.ttf", include_bytes!("../fonts/AcmeSerif-Light.ttf"))?;
    r.theme("acme", include_str!("../themes/acme.yaml"))?;
    r.theme("acme-light", include_str!("../themes/acme-light.yaml"))?;
    r.point_cloud("acme-logo", include_bytes!("../point-clouds/acme-logo.mdpc"))
}
```

**TYPE** `themes/acme.yaml`:

```yaml
name: acme
extends: dark
engine:
  name: acme-brand
fonts:
  display: AcmeSerif-Light.ttf    # found among the fonts the crate registers
  body: AcmeSans-Regular.ttf
colors:
  accent: "#c2410c"
```

`include_str!` and `include_bytes!` copy the files into the compiled program, so the custom mdeck
needs no files on disk: hand it to someone and the themes, fonts and logo come along.

- `Registry::theme(name, yaml)` registers a theme under a name; decks select it with
  `theme: acme`.
- `Registry::font(file, bytes)` registers a `.ttf` or `.otf` under the file name your themes write
  in `fonts:`. A theme folder on disk finds fonts next to it; an embedded theme has no folder, so it
  finds them here. (We verified this end to end: an embedded theme naming a registered font draws
  in that font.)
- `Registry::point_cloud(name, bytes)` registers a `.mdpc` point cloud that slides can ask for by
  name.
- Visual kinds, design sets and transitions register the same way (`r.visual`, `r.design_set`,
  `r.transition`). One crate can hold all of a company's extensions.

Names are global: if two crates (or a crate and a built-in) register the same name, the custom
mdeck stops at start-up with an error naming both. Prefix your names (`acme-...`).

**Font licences.** Embedding a font copies it into every binary you build and, if you publish the
crate, into the published source. Many commercial fonts allow neither. Check the licence; if it
only allows internal use, keep the repository private and do not publish the crate.

### The alternative: engine crate plus a theme pack

Split them when the people who design the look are not the people who write Rust:

- the **engine** stays a crate (built once, rarely changed);
- the **themes, fonts and pictures** go in a pack that designers edit and reinstall in seconds,
  without a compiler.

**TYPE** `acme-themes/mdeck-pack.yaml` (the pack's manifest):

```yaml
name: acme-themes
version: 1.0.0
description: Acme's brand themes for the acme-brand engine
min-mdeck: "2.0"
```

```text
acme-themes/
├── mdeck-pack.yaml
├── themes/acme.yaml           engine: { name: acme-brand }, fonts: { body: AcmeSans-Regular.ttf }
├── fonts/AcmeSans-Regular.ttf
└── point-clouds/acme-logo.mdpc
```

**RUN** anywhere (each colleague, once per version):

```bash
mdeck build --with git+ssh://git@github.com/acme/acme-brand.git#v1.0.0 --out ~/bin/
mdeck pack install git@github.com:acme/acme-themes.git
```

A pack theme that selects `engine: acme-brand` shows the engine with an mdeck that has it; with
any other mdeck it presents with the `plain` engine, and `--check` says
`engine 'acme-brand' is not an engine in this build of MDeck`.
[Themes, packs](../themes.md#packs) has the pack format.

| Choose | When |
|---|---|
| **One crate** with engine, themes and fonts | One team owns everything; you want one install step; the look changes with the engine |
| **Engine crate + theme pack** | Designers iterate on the look without Rust; several themes share one engine; the look changes far more often than the engine |

Either way, add `requires: [acme-brand]` (and `acme-themes` if you use a pack) to the decks'
frontmatter: `mdeck --check` then tells anyone with the wrong mdeck exactly what is missing.

## Ways to distribute

### Local path

**RUN** in the folder that holds the crate:

```bash
mdeck build --with ./acme-brand
```

For you while developing, or a colleague who has a copy of the folder. Nothing to set up; but
every copy drifts on its own.

### A zip of the source

Zip the crate's folder (without `target/`), send it, and the colleague unzips and builds it:

**RUN** where the zip is:

```bash
unzip acme-brand.zip
mdeck build --with ./acme-brand --out ~/bin/
```

Works without any git server, for one-off hand-overs. There is no update path: every new version
is a new zip. (`mdeck build --with` does not read zip files directly; `mdeck pack install` does,
but only for packs.)

### A private git repository

The recommended way for a company. Push the crate to a private repository on GitHub, GitLab,
Bitbucket or your own server, and colleagues build from it:

**RUN** anywhere (each colleague):

```bash
mdeck build --with git+ssh://git@github.com/acme/acme-brand.git#v1.2.0 --out ~/bin/
mdeck build --with git+https://github.com/acme/acme-brand#v1.2.0 --out ~/bin/
mdeck build --with git@github.com:acme/acme-brand.git#v1.2.0 --out ~/bin/
```

All three fetch the same thing. After `#` comes a tag (`v1.2.0`), a branch (`main`), or a commit
(`1a2b3c4d`); write `#tag=...`, `#branch=...` or `#rev=...` to say which when the name is
ambiguous. Without `#`, the default branch. The crate's name is taken from the repository's
name; if they differ, add `?package=<crate-name>` before the `#`. A branch is updated to its
newest commit every time the command runs; a tag or commit stays fixed.

Cargo fetches the repository with the colleague's own credentials: their SSH key for `ssh` URLs,
their git credential helper for `https`. If that fails although `git clone` works, set
`CARGO_NET_GIT_FETCH_WITH_CLI=true` (see the tutorial's
[troubleshooting](tutorial-0-your-first-engine.md#part-8-troubleshooting)).

Access control is the repository's: whoever can clone it can build it. Updating is one command
with a new tag. Colleagues need Rust.

### A public GitHub repository

The same commands, without credentials. Good for open-source engines that are not ready for (or
not meant for) crates.io. Add a licence file and a README that says which mdeck versions the
engine supports.

### crates.io

Publishing on [crates.io](https://crates.io) makes the engine available to everyone by name:

**RUN** anywhere (anyone):

```bash
mdeck build --with mdeck-engine-aurora@1.2
```

`@1.2` means "1.2 or any newer 1.x"; leave it out for the newest version. A rebuild picks up new
compatible releases.

To publish:

1. **Name it** so people can find it: `mdeck-engine-<name>` for an engine, `mdeck-visual-<name>`,
   `mdeck-designs-<name>` or `mdeck-<company>` for a bundle. The engine's own name (what themes
   write) can stay short.
2. **Fill in the metadata** in `Cargo.toml`, and remove `publish = false`:

   **TYPE** `Cargo.toml`:

   ```toml
   [package]
   name = "mdeck-engine-aurora"
   version = "1.2.0"
   edition = "2024"
   description = "Northern lights for mdeck: an engine and its theme"
   license = "MIT OR Apache-2.0"
   repository = "https://github.com/you/mdeck-engine-aurora"
   readme = "README.md"
   keywords = ["mdeck", "mdeck-engine", "presentation"]

   [dependencies]
   mdeck-sdk = "2"
   ```

   Depend on the SDK by major version (`"2"`): `mdeck build` compiles your crate against the SDK
   of the mdeck being built, so one release of your crate serves every mdeck 2.x.
3. **Check the package**, then publish:

   **RUN** in the crate's folder:

   ```bash
   cargo package --list          # every file that will be uploaded: themes and fonts included?
   cargo publish --dry-run       # builds the package exactly as crates.io will
   cargo login                   # once, with a token from crates.io
   cargo publish
   ```

   `cargo package` uploads everything in the folder except what `.gitignore` excludes; files you
   `include_str!` or `include_bytes!` must be inside the crate's folder.

A published version is permanent (you can yank it, not delete it), and anyone can read the
source, fonts included. Do not publish what is confidential.

### A private cargo registry

`mdeck build --with name@version` fetches crate names from crates.io only; it cannot name another
registry (Artifactory, Cloudsmith, a self-hosted one). Use a private git repository instead: it
gives the same versioning through tags and the same access control.

### Share the built binary

`mdeck build` produces one self-contained program: mdeck, every built-in, and your extensions with
their themes, fonts and point clouds. Give that file to people who should not have to install
Rust.

**RUN** in the crate's folder:

```bash
mdeck build --with . --name mdeck-acme --out dist/
```

- **One build per platform.** A binary runs only on the operating system and processor it was
  built on: macOS on Apple Silicon, macOS on Intel, Linux x86-64, Windows x86-64 and so on are
  separate builds. Build each on a machine (or CI runner) of that kind.
- **macOS.** Files downloaded with a browser, chat or mail app are quarantined, and Gatekeeper
  refuses an unsigned program. Colleagues can allow it with
  `xattr -d com.apple.quarantine ./mdeck-acme`. For wide distribution, sign it with a Developer ID
  (`codesign --sign "Developer ID Application: Acme" --options runtime mdeck-acme`) and notarise
  it (`xcrun notarytool submit`).
- **Windows.** SmartScreen warns about unsigned programs from the internet ("More info", then
  "Run anyway"). Sign it with your company's code-signing certificate to avoid that.
- **Linux.** A binary built on a newer distribution may need a newer glibc than an older one has;
  build on the oldest distribution you support.
- **Updating** means sending a new binary for every mdeck release and every engine release. The
  recipients' installed `mdeck` stays as it is; give yours its own name (`--name mdeck-acme`) so
  both can live side by side.

### Which one should I use?

| I am... | and I want... | Use |
|---|---|---|
| a developer trying an idea | to see it working | local path (`--with .`) |
| a company | colleagues with Rust to get updates easily | private git repository, tags for releases |
| a company | everyone, including people without Rust, to present with the brand | private git repository for the source, plus a binary per platform built from a tag (CI can build and attach them to a release) |
| a company | designers to change the look without Rust | engine crate in git, themes in a pack |
| a company | to send it once to a partner | a zip of the source, or a binary for their platform |
| an open-source author | anyone to use it by name | crates.io |
| an open-source author | to share before it is polished | a public GitHub repository |

## Versions over time

Your extension has **its own version** (in `Cargo.toml`, and as a git tag), following
[semantic versioning](https://semver.org): patch for fixes, minor for additions (a new setting, a
new theme), major for anything that breaks the themes or decks that use it (a renamed setting, a
removed theme).

It also **supports a major version of mdeck**, the one in its `mdeck-sdk` dependency. Say so in
the README ("works with mdeck 2.x").

- **mdeck releases a minor or patch version (2.4, 2.4.1).** Nothing in your crate changes. The SDK
  is versioned in lockstep with mdeck, and SDK 2.x keeps everything 2.0 had
  ([Compatibility](compatibility.md)). Rebuild to get the new mdeck: users of the git or crates.io
  route run their `mdeck build` command again after upgrading mdeck; binary users get a new
  binary from you.
- **mdeck releases a new major version (3.0).** Read [Upgrading your extension](upgrading.md),
  move to `mdeck-sdk = "3"`, fix what the compiler reports, run the tests, and release a new
  major version of your crate. Keep the 2.x line on a branch if some colleagues stay on mdeck 2
  for a while; they build from its tags (`#v1.4.0`) while others move on.
