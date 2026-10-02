## What and why

<!-- What does this change, and why? Reference issues as (#123). -->

## Checklist

See [CONTRIBUTING.md](https://github.com/mklab-se/mdeck/blob/main/CONTRIBUTING.md) for the details.

- [ ] `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` passes
- [ ] New behaviour has tests; a bug fix has a regression test that fails without the fix
- [ ] Anything visual: before and after exports are attached below
- [ ] Docs, the format reference and sample decks are updated where they describe this change
- [ ] `CHANGELOG.md` has an entry under `[Unreleased]` (for a user-visible change)
- [ ] No em-dashes in anything I wrote

## Pictures

<!-- Before and after exports (mdeck export deck.md --slide N --output-dir out). Delete for
non-visual changes. -->

<!--
## A theme (delete this section otherwise)

- [ ] Accepted in a theme proposal: #
- [ ] Theme or variant (`variant-of:`) chosen as CONTRIBUTING.md describes
- [ ] `mdeck theme check <name>` reports no issues
- [ ] `mdeck theme preview <name>` sampler attached, plus a few slides of a real deck
- [ ] Listed in `theme::lookup::BUILTIN`, docs/themes.md, spec 9.1 and GALLERY.md
- [ ] Any new font is OFL (or similar) with its license file in crates/mdeck/fonts/

## An engine (delete this section otherwise)

- [ ] Accepted in an engine proposal: #
- [ ] Feature on by default, in build.rs `ENGINES` and in the CI feature matrix
- [ ] Showcase theme, samples/engines/<name>.md, gallery images
- [ ] `scripts/engine-golden.sh` shows no other engine moved
- [ ] The engine contract in CONTRIBUTING.md holds (deterministic stills, reduced motion, scale,
      theme tokens, clear of content, performance, honest `animating`, settings, --check, export)
-->
