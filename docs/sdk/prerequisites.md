# Prerequisites

What you need on your computer before you write an mdeck extension, and why. Set this up once;
every tutorial assumes it.

**In short:** mdeck 2, Rust (through `rustup`), a C toolchain, and git. You do **not** need a copy
of mdeck's source code.

## What you need, and why

| You need | Why | Check it with |
|---|---|---|
| **mdeck 2.x** | It creates your extension (`mdeck sdk new`) and builds an mdeck with it inside (`mdeck build`) | `mdeck --version` |
| **Rust 1.95 or newer**, installed with `rustup` | An extension is Rust code. `mdeck build` runs Rust's build tool, `cargo`, to compile mdeck together with your code | `rustc --version` and `cargo --version` |
| **A C toolchain** (a linker and a C compiler) | Rust uses the system linker, and a few of mdeck's dependencies compile a little C | see your operating system below |
| **git** | To keep the history of your extension and to share it with colleagues | `git --version` |
| An editor with **rust-analyzer** (recommended) | Completion, inline errors and "go to definition" make the SDK much easier to explore | |

The people who only *use* your extension need much less: if you hand them a built mdeck binary,
they need nothing at all; if they build it themselves with `mdeck build`, they need the same list
as you (see [Packaging and sharing](packaging-and-sharing.md)).

### You do not need to clone the mdeck repository

`mdeck build` downloads the right versions of mdeck and the SDK for you, from
[crates.io](https://crates.io) (Rust's package registry), at exactly the version of the mdeck you
have installed. If `mdeck --version` says 2.3.1, your custom build is mdeck 2.3.1 plus your
extension. Your extension lives in its own folder, in its own git repository, wherever you like.

You only need a clone of mdeck if you want to change mdeck itself, or to build against an
unreleased version of it (see [Building against an mdeck checkout](#building-against-an-mdeck-checkout)).

## 1. Install mdeck

Pick one:

```bash
brew install mklab-se/tap/mdeck      # macOS and Linux, with Homebrew
cargo install mdeck                  # anywhere, once Rust is installed (step 2); compiles from source
```

Or download a binary for your system from
[GitHub Releases](https://github.com/mklab-se/mdeck/releases) and put it on your `PATH`.
See [Install](../install.md) for details.

Check:

```bash
mdeck --version          # mdeck 2.x.y
```

## 2. Install Rust

Install Rust with [rustup](https://rustup.rs), the official installer. It installs `rustc` (the
compiler), `cargo` (the build tool) and keeps them up to date. Avoid Rust from a Linux distribution's
package manager: it is often older than the 1.95 mdeck needs.

**macOS and Linux:**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Accept the default installation, then open a new terminal (or run `source "$HOME/.cargo/env"`) so
`cargo` is on your `PATH`.

**Windows:** download and run `rustup-init.exe` from [rustup.rs](https://rustup.rs). When it
offers to install the Visual Studio C++ build tools, say yes (see step 3).

**Check**, on every system:

```bash
rustc --version          # rustc 1.95.0 or newer
cargo --version
```

If your Rust is older than 1.95, update it:

```bash
rustup update stable
```

## 3. Install a C toolchain

| System | What to install | Command |
|---|---|---|
| **macOS** | Xcode Command Line Tools | `xcode-select --install` |
| **Linux** (Debian, Ubuntu) | `build-essential` and `pkg-config` | `sudo apt install build-essential pkg-config` |
| **Linux** (Fedora) | GCC and friends | `sudo dnf install gcc gcc-c++ make pkgconf-pkg-config` |
| **Windows** | Visual Studio Build Tools ("Desktop development with C++"), **NASM** and **CMake** | `rustup-init` offers the build tools; then `winget install NASM.NASM Kitware.CMake` |

On Windows, NASM and CMake must be on your `PATH`: one of mdeck's dependencies (`aws-lc-rs`, the
crypto library behind its network access) compiles assembly code with them. Open a new terminal
after installing them and check with `nasm -v` and `cmake --version`. macOS and Linux need nothing
extra.

## 4. Install git

You probably have it already (`git --version`). If not: on macOS it comes with the Command Line
Tools from step 3; on Linux use your package manager (`sudo apt install git`); on Windows install
[Git for Windows](https://git-scm.com/download/win).

To share your extension privately you will also want an account on GitHub, GitLab or your
company's git server, with an SSH key or a credential helper set up so `git clone` of a private
repository works without typing a password.

## 5. An editor (recommended)

Any editor works. [Visual Studio Code](https://code.visualstudio.com) with the
[rust-analyzer](https://rust-analyzer.github.io) extension is a good start: open your extension's
folder and it shows errors as you type, completes SDK names, and shows the documentation of
everything you hover. RustRover, Zed, Neovim and Helix support rust-analyzer too.

## How long the first build takes, and why

The first `mdeck build` compiles all of mdeck and its dependencies (a few hundred crates) in
release mode, with optimisations on. It takes from **about a minute** on a recent machine with many
cores (measured: 63 seconds on a 14-core Apple M4 Pro) to **10 minutes or more** on an older
laptop, plus the time to download the crates the first time. It uses about 1.2 GB of disk in mdeck's cache folder (`~/.cache/mdeck/build/` on Linux and macOS,
`%LOCALAPPDATA%\mdeck\build\` on Windows).

After that it is fast: the compiled dependencies are kept, so a rebuild after you change your
extension takes **15 to 60 seconds**. A new mdeck version (after you upgrade mdeck) compiles mdeck
again, but not its dependencies, unless they changed too.

`cargo test` in your extension's folder is quicker: it only compiles the SDK and your crate (about
a minute the first time, seconds after that).

## Check everything at once

```bash
mdeck --version && rustc --version && cargo --version && git --version
```

Four version lines and you are ready: start with
[Your first engine](tutorial-0-your-first-engine.md).

## Building against an mdeck checkout

This is for people who work on mdeck itself, or who test an extension against an unreleased mdeck.
Everyone else can skip it.

`mdeck build` normally takes mdeck from crates.io. To use a clone of the mdeck repository instead,
point it there:

```bash
git clone https://github.com/mklab-se/mdeck
mdeck build --with ./my-engine --mdeck-path ./mdeck     # or: export MDECK_SOURCE=$PWD/mdeck
```

`cargo test` in your extension's folder still fetches `mdeck-sdk` from crates.io. To test against
the checkout's SDK, add this to `.cargo/config.toml` in your extension's folder (or a folder above
it), and do not commit it:

```toml
[patch.crates-io]
mdeck-sdk = { path = "/path/to/mdeck/crates/mdeck-sdk" }
```
