#!/usr/bin/env bash
# Publish a PNG so it can be embedded in a GitHub issue comment.
#
# GitHub has no API for comment attachments, so screenshots live on the orphan
# branch `issue-screenshots` of this repo (created on first use), one folder per
# issue, written through the Contents API without touching the working tree.
# Prints the markdown image line to paste into the comment.
#
# usage: upload-screenshot.sh <issue-number> <file.png> [alt text]
set -euo pipefail

issue="${1:?issue number}"
file="${2:?png file}"
alt="${3:-$(basename "${file%.*}")}"
branch="issue-screenshots"
repo="$(gh repo view --json nameWithOwner -q .nameWithOwner)"
name="$(basename "$file")"
path="$issue/$name"

[[ -f "$file" ]] || { echo "no such file: $file" >&2; exit 1; }

# First use: create the orphan branch (a README in an otherwise empty tree,
# a parentless commit, and the ref).
if ! gh api "repos/$repo/branches/$branch" --silent 2>/dev/null; then
  readme=$'Screenshots embedded in GitHub issue comments, one folder per issue.\nWritten by .claude/skills/fix-issue/upload-screenshot.sh; never merged into main.\n'
  blob=$(gh api "repos/$repo/git/blobs" -f content="$readme" -f encoding=utf-8 -q .sha)
  tree=$(jq -n --arg sha "$blob" '{tree:[{path:"README.md",mode:"100644",type:"blob",sha:$sha}]}' \
    | gh api "repos/$repo/git/trees" --input - -q .sha)
  commit=$(jq -n --arg tree "$tree" '{message:"Issue screenshots", tree:$tree}' \
    | gh api "repos/$repo/git/commits" --input - -q .sha)
  gh api "repos/$repo/git/refs" -f ref="refs/heads/$branch" -f sha="$commit" --silent
  echo "created branch $branch" >&2
fi

# Replacing a file of the same name needs its current blob sha.
sha=$(gh api "repos/$repo/contents/$path?ref=$branch" -q .sha 2>/dev/null || true)

jq -n --arg msg "#$issue: $name" --arg branch "$branch" --arg sha "$sha" \
      --rawfile content <(base64 < "$file") \
      '{message:$msg, branch:$branch, content:($content|gsub("\n";""))}
       + (if $sha != "" then {sha:$sha} else {} end)' \
  | gh api -X PUT "repos/$repo/contents/$path" --input - --silent

echo "![$alt](https://raw.githubusercontent.com/$repo/$branch/$path)"
