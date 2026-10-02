# The mdeck SDK

`mdeck-sdk` is the Rust crate you build mdeck extensions with: engines, visual kinds, design sets
and transitions. An extension is an ordinary crate that depends on `mdeck-sdk`. It can live in a
private repository and never be published; `mdeck build --with ./my-extension` (or
`--with git+ssh://git@github.com/acme/my-extension.git#v1.0.0`) compiles an mdeck that includes
it.

The goal of these pages: a Rust developer who has never seen mdeck's source writes a simple engine
in an afternoon, and a serious one without reading mdeck's internals. If you need the source to
understand something, the documentation has a gap: please open an issue.

## Start here

New to mdeck extensions, or to Rust? Read these in order.

| Page | What it covers |
|---|---|
| [Prerequisites](prerequisites.md) | What to install and why: mdeck, Rust, a C toolchain, git. You do not need mdeck's source |
| [Your first engine](tutorial-0-your-first-engine.md) | The full cycle, step by step: create an engine, build and look, theme and settings, tests, git, sharing with colleagues, using it in a deck, troubleshooting |
| [Packaging and sharing](packaging-and-sharing.md) | Code or data, bundling an engine with its themes and fonts, theme packs, every way to distribute (git, zip, crates.io, binaries) and versioning |
| [Getting started](getting-started.md) | The same path on one page, for Rust developers in a hurry |
| [Concepts](concepts.md) | The model an extension lives in: deck, slide, design, theme, engine; the frame lifecycle; the contract |

### How code blocks are labelled

Every code block in these pages says where it belongs and what to do with it: **TYPE** `file`
(write or change this), **READ** `file` (shown so you understand it; it is generated, or already
in an example), **RUN** in `folder` (terminal commands, run from that folder).

## Engine tutorial

Three complete, tested engines, each built on the previous one. The code is in
[`examples/`](../../examples) and runs in mdeck's CI, so it cannot rot. Where
[your first engine](tutorial-0-your-first-engine.md) has you type every line, these steps explain
finished engines: read them next to their source.

| Step | Engine | What you learn |
|---|---|---|
| [1. Ambience](tutorial-1-ambience.md) | `ambience` | The frame lifecycle, drawing, scaling, theme colours, reduced motion, deterministic stills |
| [2. Pictures and moments](tutorial-2-pictures.md) | `pictures` | The slide's picture in your own medium, the countdown and the end act, easing between slides, staying clear of the copy |
| [3. Reacting to content](tutorial-3-reactive.md) | `reactive` | Published geometry (bars, lines, frames), typed and validated settings, what the engine needs from the theme, reporting problems, performance |

## Other extension points

| Page | What it covers |
|---|---|
| [Visuals](visuals.md) | A new fenced block kind (```` ```@my-chart ````): the `Visual` trait, the fence grammar, `--check` problems, publishing geometry |
| [Design sets and board engines](design-sets.md) | Arranging slides in code, and engines that draw every slide themselves |
| [Transitions](concepts.md#transitions) | How one slide gives way to the next: `look`, `duration` and `paint_over` |

## Reference

| Page | What it covers |
|---|---|
| [API reference](https://docs.rs/mdeck-sdk) | Every public item, with an example that compiles as a doctest |
| [Compatibility](compatibility.md) | What stays stable within a major version, and the `unstable-egui` escape hatch |
| [Upgrading your extension](upgrading.md) | Breaking changes between major versions, with before and after code |

## The scaffold

`mdeck sdk new <kind> <name>` creates a crate that builds and runs immediately, for each kind:
`engine`, `visual`, `design-set` and `transition`. Each has a working minimal implementation with
a comment on every hook, a showcase theme, a sample deck, a golden-image test and a README with the
commands to build, run, export and test it. The templates live in
[`crates/mdeck-sdk/templates`](../../crates/mdeck-sdk/templates).

## The built-in engines are the advanced examples

mdeck's own engines implement the same interface. When you wonder "how does the particles engine do
this?", read its source in [`crates/mdeck/src/engines`](../../crates/mdeck/src/engines).
