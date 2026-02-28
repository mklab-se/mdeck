# presemd

A markdown-based presentation tool.

## Commands

```bash
cargo build              # Build all crates
cargo test --workspace   # Run all tests
cargo clippy --workspace -- -D warnings  # Lint (CI-enforced)
cargo fmt --all -- --check               # Format check (CI-enforced)
cargo run -p presemd     # Run the app
```

## Architecture

Rust workspace with a single crate:

```
crates/
  presemd/           # GUI binary (package and binary name: presemd)
    src/
      main.rs        # Entry point, CLI bootstrap
      cli.rs         # Clap argument definitions (Cli, Commands, subcommands)
      app.rs         # GUI presentation app (eframe/egui rendering)
      banner.rs      # Version banner display
      config.rs      # Config struct, load/save (~/.config/presemd/config.yaml)
      commands/
        mod.rs       # Re-exports
        ai.rs        # AI provider init/status/remove
        completion.rs # Shell completion generation
        config.rs    # Config show/set
        export.rs    # PNG export via headless eframe rendering
        spec.rs      # Format specification printer
      parser/          # Markdown-to-slide parser (frontmatter, blocks, inlines, splitter)
      render/          # Slide rendering engine
        mod.rs       # render_slide entry point
        text.rs      # Block-level drawing (headings, lists, code, tables, diagrams, images)
        layouts/     # Layout strategies (title, section, bullet, code, content, two_column, quote, image_slide)
        image_cache.rs # Async image loading and caching
      theme.rs       # Theme definitions (light, dark, solarized, etc.)
```

- **Workspace root** `Cargo.toml` defines shared dependencies and metadata
- All crates inherit `version`, `edition`, `authors`, `license`, `repository`, `rust-version` from workspace
- Single version bump in root `Cargo.toml` updates everything

## CLI Usage

```bash
presemd <file.md>              # Launch presentation
presemd ai init                # Set up AI provider (interactive)
presemd ai status              # Show AI config
presemd ai remove              # Remove AI config
presemd config show            # Display configuration
presemd config set <key> <val> # Set config value (defaults.theme, defaults.transition, defaults.aspect)
presemd export <file.md>       # Export slides as PNG images (1920x1080 default)
presemd export <file.md> --width 3840 --height 2160  # Export at custom resolution
presemd completion <shell>     # Generate shell completions (bash, zsh, fish, powershell)
presemd spec                   # Print format specification
presemd spec --short           # Print quick reference card
presemd version                # Show version banner
presemd --help                 # Show help
```

## Key Patterns

- **CLI framework:** `clap` with derive macros, `clap_complete` for shell completions
- **GUI framework:** `eframe` / `egui`
- **Config:** YAML via `serde_yaml`, stored at `~/.config/presemd/config.yaml` (via `dirs`)
- **Interactive prompts:** `inquire` for selections (e.g., AI provider picker)
- **Terminal output:** `colored` for styled CLI output
- **Error handling:** `anyhow` for ergonomic error propagation
- **Rendering:** Scale factor `min(w/1920, h/1080)` applied to all pixel sizes for resolution independence
- **PNG export:** Headless eframe window using `ViewportCommand::Screenshot` / `Event::Screenshot`
- Slide transitions: fade and horizontal slide with easing
- Keyboard navigation: arrow keys for forward/backward
- FPS overlay in top-right corner

## Releasing

1. Bump `version` in root `Cargo.toml`
2. Commit and push to main
3. Tag: `git tag v0.X.Y && git push origin v0.X.Y`
4. Release workflow builds binaries (Linux, macOS Intel+ARM, Windows), creates GitHub Release, updates Homebrew tap (`mklab-se/homebrew-tap`), publishes to crates.io

**Required GitHub secrets:**
- `CARGO_REGISTRY_TOKEN` (in `crates-io` environment)
- `HOMEBREW_TAP_TOKEN` (GitHub PAT with repo scope for `mklab-se/homebrew-tap`)

## Code Style

- Edition 2024, MSRV 1.85
- `cargo clippy` with `-D warnings` (zero warnings policy)
- `cargo fmt` enforced in CI

## Quality Requirements

### Testing
- **Always run the full test suite before declaring work complete:** `cargo test --workspace`
- **Always run the full CI check before pushing:** `cargo fmt --all -- --check && cargo clippy --workspace -- -D warnings && cargo test --workspace`
- Write unit tests for all new functionality
- Test edge cases and error paths, not just the happy path

### Documentation
- **Before pushing or releasing, review all documentation for accuracy:**
  - `README.md` — features, quick start, badges
  - `CHANGELOG.md` — new entries for every user-visible change
  - `CLAUDE.md` — architecture, commands, patterns
- When adding new commands, flags, or crates, update all relevant docs in the same commit
- `CHANGELOG.md` must be updated for every release with a dated entry following Keep a Changelog format
