---
name: include-illustrations
description: Make generated point cloud illustrations official built-ins of mdeck. Use when Kristofer says to include, add, promote or ship illustrations he has created (typically the .mdpc files in ./illustrations/ at the repo root, or files he names), or to pull an illustration someone contributed.
---

# Include illustrations as built-ins

A built-in illustration is exactly one file: `crates/mdeck/illustrations/<name>.mdpc`.
The build script registers every file in that folder, so nothing in `src/` changes.
Everything else in this skill is review and bookkeeping.

## Step 1: Find the candidates

- Default source: `illustrations/*.mdpc` in the repo root (where `mdeck illustration
  generate` writes when run from the root). Also accept paths, names, or a deck's
  `illustrations/` folder if Kristofer names one.
- List each candidate with its name, description, point count and aspect (`head -8` of
  the file is enough). Skip the `.png` source images; they never enter the repo.
- Refuse a name that is not lowercase letters, digits and hyphens, and say so.

## Step 2: Check the name against the built-in set

```bash
cargo run -q -p mdeck -- illustration list | grep built-in
```

- New name: proceed.
- Name already built in: this is a replacement. Show both previews (Step 3) side by
  side and ask before overwriting, unless Kristofer already said to replace it.

## Step 3: Look at every candidate

```bash
mdeck illustration show <name> --output /tmp/<name>.png --quiet   # from the repo root
```

Read each preview PNG. Accept a cloud that reads as its subject at a glance and is not
a filled blob. If one does not read, say which and why, and leave it out unless told
otherwise; do not silently drop it.

## Step 4: Move the files in

```bash
git mv illustrations/<name>.mdpc crates/mdeck/illustrations/<name>.mdpc   # or cp, if untracked
rm illustrations/<name>.png                                                # source image, never committed
```

Make sure the moved file keeps its `prompt` and `generated` fields (a generated cloud
has them; an imported one has `prompt: null`, which is fine).

## Step 5: Build, verify, and update the docs

```bash
cargo build -p mdeck && cargo test -p mdeck illustration
cargo run -q -p mdeck -- illustration list | grep -c built-in
```

Then update every place the built-in set is listed or counted:

- `crates/mdeck/doc/mdeck-spec.md` — the "The built-in set:" list in the Illustrations section
- `crates/mdeck/doc/ai-reference-supplement.md` — the "Built in:" list
- `README.md` and `GALLERY.md` — the count ("Twenty illustrations are built in")
- `CHANGELOG.md` — an `[Unreleased]` line naming the new built-ins
- `samples/ember/illustrations.md` — only if the new cloud makes a better example than
  one already there

Run the full check before declaring done:

```bash
cargo fmt --all -- --check && cargo clippy --workspace -- -D warnings && cargo test --workspace
```

## Step 6: Commit

One commit, `illustration: add <names> to the built-in set`, with the standard
trailers. Do not release; Kristofer decides that separately.

## Contributed clouds

If the cloud arrives from someone else (a pull request touching
`crates/mdeck/illustrations/`, or a file attached to an issue), the same steps apply
from Step 2. For a pull request: `gh pr checkout <n>` and review with Step 3. For an
issue attachment: download it with `gh api` or the browser, rename to `<name>.mdpc`,
and validate it with `mdeck illustration show` before anything else.
