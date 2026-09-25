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
- Run `cargo fmt --all -- --check` — abort if formatting issues. If you fix formatting with
  `cargo fmt --all`, re-run clippy afterwards: reformatting can change what clippy flags
- Run `cargo clippy --workspace --all-targets -- -D warnings` — abort if warnings
  (`--all-targets` matches CI: it also lints tests and benches)
- Run `cargo test --workspace` — abort if any test fails
- Run `git status` — abort if there are uncommitted changes that are NOT documentation, version,
  or dependency files

### 3. Verify documentation and spec are up to date

- **`crates/mdeck/doc/mdeck-spec.md`**: Review the format spec against current features. Run `cargo run -p mdeck -- spec` and `cargo run -p mdeck -- spec --short` to verify the output looks correct and covers all implemented features. If new visualizations, layouts, directives, or keyboard shortcuts have been added since the last release, update the spec (and the short reference in `commands/spec.rs`) before proceeding.
- **`README.md`**: Verify features list, command reference, visualization table, AI section (image generation, style management, diagram icon generation, with clear examples) and gallery preview images are current. The README is the first thing users see; it must provide an excellent experience.
- **`GALLERY.md`**: If rendering has changed, re-export gallery slides (`cargo run -p mdeck -- export samples/gallery.md --output-dir media/gallery`) and verify screenshots reflect current rendering. If new visualization types, layouts, or features have been added, add them to `samples/gallery.md` and regenerate.
- **`BACKLOG.md`**: Deferred ideas and decisions; move items out when implemented.
- **Dependencies**: Run `cargo audit`; bump major versions when the audit or `cargo info <crate>` shows a newer line.
- **`samples/`**: Test presentations cover all features; dedicated test files exist for each visualization type.
- **`CLAUDE.md`** (and `crates/mdeck/src/render/CLAUDE.md`): Verify patterns and conventions are accurate.
- If any documentation is out of date, update it now before proceeding to the version bump.

### 4. Bump version numbers

- Update `version` in the root `Cargo.toml` `[workspace.package]` section

### 5. Update CHANGELOG

- **CHANGELOG.md**: Rename the `[Unreleased]` section to `[{NEW_VERSION}] - {TODAY}` (YYYY-MM-DD format). If there is no `[Unreleased]` section, create a new dated entry summarizing changes since the last release

### 6. Verify the build

- Run `cargo build --workspace` to ensure everything compiles with the new version
- Run `cargo test --workspace` once more after version bump

### 7. Commit, push, and tag

- Stage all changed files: `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and any updated docs
- Commit with message: `Release v{NEW_VERSION}`
- Push to main: `git push`
- Create and push tag: `git tag v{NEW_VERSION} && git push origin v{NEW_VERSION}`

### 8. Watch and verify

- The tag push triggers the Release workflow. Do NOT declare success yet — watch it:
  `gh run list --repo mklab-se/mdeck --workflow release.yml --limit 1`, then
  `gh run watch <id> --repo mklab-se/mdeck --exit-status` until it completes
- If it fails, inspect with `gh run view <id> --log-failed`, fix the cause, and re-release as a patch
- When it is green, confirm the outputs:
  - `gh release view v{NEW_VERSION} --repo mklab-se/mdeck` lists 4 archives
    (3 × `.tar.gz`, 1 × `.zip`) plus 4 matching `.cdx.json` SBOMs
  - `cargo search mdeck --limit 1` shows the new version on crates.io
  - `Formula/mdeck.rb` in `mklab-se/homebrew-tap` carries the new version

### 9. Confirm

- Tell the user the release is tagged, pushed, and the workflow is green — auditable binaries and
  SBOMs are attached to the GitHub Release, crates.io is published, and the Homebrew tap is updated
- The publish jobs require the `CARGO_REGISTRY_TOKEN` (in the `crates-io` environment) and
  `HOMEBREW_TAP_TOKEN` (repo secret, a GitHub PAT with repo scope for `mklab-se/homebrew-tap`) to be configured (see README.md)
