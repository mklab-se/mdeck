---
name: include-point-clouds
description: Make generated point clouds official built-ins of mdeck. Use when Kristofer says to include, add, promote or ship point clouds (pictures, .mdpc files) he has created (typically the .mdpc files in ./point-clouds/ at the repo root, or files he names), or to pull a point cloud someone contributed.
---

# Include point clouds as built-ins

A built-in point cloud is exactly one file: `crates/mdeck/point-clouds/<name>.mdpc`.
The build script registers every file in that folder, so nothing in `src/` changes.
Everything else in this skill is review and bookkeeping.

## Step 1: Find the candidates

- Default source: `point-clouds/*.mdpc` in the repo root (where `mdeck ai point-cloud
  --name ...` and `mdeck point-cloud import` write when run from the root). Also accept
  paths, names, a deck's `point-clouds/` folder or its `<deck>.assets/point-clouds/` if
  Kristofer names one.
- List each candidate with its name, description, point count and aspect (`head -8` of
  the file is enough). Skip the `.png` source images; they never enter the repo.
- Refuse a name that is not lowercase letters, digits and hyphens, and say so.

## Step 2: Check the name against the built-in set

```bash
cargo run -q -p mdeck -- point-cloud list | grep built-in
```

- New name: proceed.
- Name already built in: this is a replacement. Show both previews (Step 3) side by
  side and ask before overwriting, unless Kristofer already said to replace it.

## Step 3: Look at every candidate

```bash
mdeck point-cloud show <name> --output /tmp/<name>.png --quiet   # from the repo root
```

Read each preview PNG. Accept a cloud that reads as its subject at a glance and is not
a filled blob. If one does not read, say which and why, and leave it out unless told
otherwise; do not silently drop it.

## Step 4: Move the files in

```bash
git mv point-clouds/<name>.mdpc crates/mdeck/point-clouds/<name>.mdpc   # or cp, if untracked
rm point-clouds/<name>.png                                               # source image, never committed
```

Make sure the moved file keeps its `prompt` and `generated` fields (a generated cloud
has them; an imported one has `prompt: null`, which is fine).

## Step 5: Build, verify, and update the docs

```bash
cargo build -p mdeck && cargo test -p mdeck illustration
cargo run -q -p mdeck -- point-cloud list | grep -c built-in
```

Then update every place the built-in set is listed or counted:

- `crates/mdeck/doc/mdeck-spec.md`: the "The built-in set:" list in the point cloud section
- `crates/mdeck/doc/ai-reference-supplement.md`: the "Built in:" list
- `docs/engines.md` and `GALLERY.md`: the count ("Thirty-eight pictures are built in")
- `CHANGELOG.md`: an `[Unreleased]` line naming the new built-ins
- `samples/ember/illustrations.md`: only if the new cloud makes a better example than
  one already there

Run the full check before declaring done:

```bash
cargo fmt --all -- --check && cargo clippy --workspace -- -D warnings && cargo test --workspace
```

## Step 6: Commit

One commit, `point clouds: add <names> to the built-in set`, with the standard
trailers. Do not release; Kristofer decides that separately.

## Contributed clouds

Contributions arrive as GitHub issues labelled `illustration` (the label keeps its v1
name), filed by `mdeck point-cloud contribute` with a `<name>.mdpc.json` attachment. To
include one:

```bash
gh issue list --label illustration
gh issue view <n> --json title,body -q '.body'          # description, prompt, braille sketch
```

Find the attachment link in the body (`https://github.com/user-attachments/files/...`),
download it, and rename it:

```bash
mkdir -p point-clouds
curl -sL -o point-clouds/<name>.mdpc "<attachment url>"
mdeck point-cloud show <name> --output /tmp/<name>.png --quiet
```

Then continue from Step 2. Close the issue with a short note either way (`gh issue
close <n> --comment "..."`): included and in which release, or why not. If the same name
is already built in, it is a replacement and needs the side-by-side look from Step 2.
