---
name: release
description: "Release a new version: bump version, update docs, commit, push, and tag"
argument-hint: "<major|minor|patch>"
---

Release a new version of mdeck.

## Input

$ARGUMENTS must be one of: `major`, `minor`, `patch`. If empty or invalid, stop and ask.

## Steps

### 1. Determine the new version

- Read the current version from the `version` field in the workspace `Cargo.toml`
- Apply the semver bump based on $ARGUMENTS:
  - `patch`: 0.2.0 -> 0.2.1
  - `minor`: 0.2.0 -> 0.3.0
  - `major`: 0.2.0 -> 1.0.0
- Show the user: "Releasing mdeck v{OLD} -> v{NEW}"

### 2. Pre-flight checks

- Run `cargo update` to update dependencies to the latest compatible versions
- Run `cargo fmt --all -- --check`: abort if formatting issues. If you fix formatting with
  `cargo fmt --all`, re-run clippy afterwards: reformatting can change what clippy flags
- Run `cargo clippy --workspace --all-targets -- -D warnings`: abort if warnings
  (`--all-targets` matches CI: it also lints tests and benches)
- Run `cargo test --workspace`: abort if any test fails
- Run `cargo package -p mdeck-sdk -p mdeck --allow-dirty`: abort if it fails. Packaging both
  crates in one call verifies mdeck against the packaged SDK through a temporary local registry,
  so it works before the new SDK version is on crates.io. It catches data files the published
  crates would miss (anything `include_str!`/`include_bytes!` reads must live inside its own
  crate directory; a `Cargo.toml` below a crate root is dropped, which is why the SDK templates
  store theirs as `Cargo.toml.tmpl`)
- Run `git status`: abort if there are uncommitted changes that are NOT documentation, version,
  or dependency files

### 3. Verify documentation and spec are up to date

- **`crates/mdeck/doc/mdeck-spec.md`**: Review the format spec against current features. Run `cargo run -p mdeck -- spec` and `cargo run -p mdeck -- spec --short` to verify the output looks correct and covers all implemented features. If new visualizations, layouts, directives, or keyboard shortcuts have been added since the last release, update the spec (and the short reference in `commands/spec.rs`) before proceeding.
- **`README.md`**: the landing page: the pitch, the showcase images (`media/showcase/`, exported from `samples/showcase/launch.md`), the four-step start and the links. Keep details out of it. The details live in `docs/` (install, writing slides, visualizations, themes, engines, presenting, export, AI, commands, development): verify each is current. The README is the first thing users see; it must provide an excellent experience.
- **`GALLERY.md`** and the other doc images: if rendering has changed, run `scripts/doc-images.sh` (after `cargo build --release`): it re-exports every image in `media/gallery/`, `media/showcase/` (the README hero included), `media/tutorial/` and `docs/spec/images/` from the deck and slide it shows. Look at the result before committing. A new visualization type, design or engine gets its line in the script and its image in `GALLERY.md`.
- **`BACKLOG.md`**: Deferred ideas and decisions; move items out when implemented.
- **Dependencies**: Run `cargo audit`; bump major versions when the audit or `cargo info <crate>` shows a newer line.
- **`samples/`**: Test presentations cover all features; dedicated test files exist for each visualization type.
- **`CLAUDE.md`** (and `crates/mdeck/src/render/CLAUDE.md`): Verify patterns and conventions are accurate.
- If any documentation is out of date, update it now before proceeding to the version bump.

### 4. Bump version numbers

- Update `version` in the root `Cargo.toml` `[workspace.package]` section
- Update the pinned `mdeck-sdk = { ..., version = "=X.Y.Z" }` in the root `[workspace.dependencies]` to the same version. mdeck and mdeck-sdk release in lockstep, and both take `version.workspace = true`; the pin is the only second place the number lives. Forgetting it is caught at once: cargo cannot resolve `=OLD` against the path crate's new version, so step 6 fails

### 5. Update CHANGELOG

- **CHANGELOG.md**: Rename the `[Unreleased]` section to `[{NEW_VERSION}] - {TODAY}` (YYYY-MM-DD format). If there is no `[Unreleased]` section, create a new dated entry summarizing changes since the last release
- **README.md**: For a `minor` or `major` release, update the "New in X.Y" line under the badges
  to the release's headline feature in one sentence (a `patch` release leaves it alone). It is
  how returning users find what changed, so it names what they can now do, not internals

### 6. Verify the build

- Run `cargo build --workspace` to ensure everything compiles with the new version
- Run `cargo test --workspace` once more after version bump

### 7. Commit, push, and tag

- Stage all changed files: `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and any updated docs
- Commit with message: `Release v{NEW_VERSION}`
- Push to main: `git push`
- Create and push tag: `git tag v{NEW_VERSION} && git push origin v{NEW_VERSION}`

### 8. Watch and verify

- The tag push triggers the Release workflow. Do NOT declare success yet; watch it:
  `gh run list --repo mklab-se/mdeck --workflow release.yml --limit 1`, then
  `gh run watch <id> --repo mklab-se/mdeck --exit-status` until it completes
- If it fails, inspect with `gh run view <id> --log-failed`, fix the cause, and re-release as a patch
- When it is green, confirm the outputs:
  - `gh release view v{NEW_VERSION} --repo mklab-se/mdeck` lists 4 archives
    (3 × `.tar.gz`, 1 × `.zip`) plus 4 matching `.cdx.json` SBOMs
  - `cargo search mdeck --limit 1` and `cargo info mdeck-sdk` show the new version on crates.io
    (the workflow publishes `mdeck-sdk` first, then `mdeck`, which pins it exactly)
  - `Formula/mdeck.rb` in `mklab-se/homebrew-tap` carries the new version

### 9. Confirm

- Tell the user the release is tagged, pushed, and the workflow is green: auditable binaries and
  SBOMs are attached to the GitHub Release, crates.io is published (mdeck-sdk and mdeck), and the Homebrew tap is updated
- The publish jobs require the `CARGO_REGISTRY_TOKEN` (in the `crates-io` environment) and
  `HOMEBREW_TAP_TOKEN` (repo secret, a GitHub PAT with repo scope for `mklab-se/homebrew-tap`) to be configured (see docs/development.md)
