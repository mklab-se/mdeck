<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Development and releasing

```bash
cargo build                                 # build
cargo test --workspace                      # tests
cargo clippy --workspace --all-targets -- -D warnings   # lint (CI-enforced)
cargo fmt --all -- --check                  # formatting (CI-enforced)
cargo run -p mdeck -- samples/gallery.md    # run the app on a sample
```

Sample decks live in `samples/`, with one file per layout and visualization
type for quick visual checks.

## Releasing

Releases are driven by the [`/release`](../.claude/skills/release/SKILL.md) skill (run it in Claude
Code with `major`, `minor`, or `patch`). It updates the toolchain and dependencies, runs the CI
gates, bumps the version, updates the changelog, then commits, pushes, and tags `vX.Y.Z`. Pushing
the tag triggers `.github/workflows/release.yml`, which:

1. Re-runs the full CI suite
2. Builds [auditable](https://github.com/rust-secure-code/cargo-auditable) binaries for Linux, macOS
   (Intel + ARM), and Windows, with a CycloneDX SBOM per target
3. Creates a GitHub Release with the archives and SBOMs (see the
   [SBOM notes](install.md) for how to read them)
4. Publishes `mdeck` to crates.io
5. Updates the Homebrew formula in [`mklab-se/homebrew-tap`](https://github.com/mklab-se/homebrew-tap)

### Required secrets

Configure these once on the GitHub repository (the same secrets are used by the other MKLab tools):

| Secret | Where | Purpose | How to create |
| --- | --- | --- | --- |
| `CARGO_REGISTRY_TOKEN` | Environment **`crates-io`** | Publish to crates.io | [crates.io/settings/tokens](https://crates.io/settings/tokens) → new token with publish scope |
| `HOMEBREW_TAP_TOKEN` | Repository secret | Push the formula to the tap | A GitHub PAT with `repo` scope for `mklab-se/homebrew-tap` |

If `HOMEBREW_TAP_TOKEN` is missing, the release still succeeds; the Homebrew step just logs a warning.
