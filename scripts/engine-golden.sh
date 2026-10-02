#!/usr/bin/env bash
# Prove a change did not move an engine: export the sample decks with two
# mdeck binaries (a baseline, say the last release, and your build) and name
# every image that differs. Exports are deterministic, so identical output
# means identical pixels.
#
#   scripts/engine-golden.sh <baseline-mdeck> <candidate-mdeck> [deck.md ...]
#
# Without decks it uses the Ember, theme, engine and visualization samples.
# samples/ember/with-images.md can differ from run to run with image loading.
set -euo pipefail

if [ $# -lt 2 ]; then
  echo "usage: $0 <baseline-mdeck> <candidate-mdeck> [deck.md ...]" >&2
  exit 2
fi
base=$1
cand=$2
shift 2
cd "$(dirname "$0")/.."
if [ $# -gt 0 ]; then
  decks=("$@")
else
  decks=(samples/ember/*.md samples/themes/*.md samples/engines/*.md
         samples/layouts/points.md samples/visualizations/all.md)
fi

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
for deck in "${decks[@]}"; do
  name=$(echo "$deck" | tr / _ | sed 's/\.md$//')
  "$base" export "$deck" --output-dir "$out/base/$name" >/dev/null 2>&1
  "$cand" export "$deck" --output-dir "$out/cand/$name" >/dev/null 2>&1
done

if diff -rq "$out/base" "$out/cand" >"$out/diff.txt"; then
  echo "identical: $(find "$out/cand" -name '*.png' | wc -l | tr -d ' ') images"
else
  sed -n "s|^Files $out/base/\([^ ]*\) and .*|differs: \1|p; s|^Only in $out/\(.*\)|missing: \1|p" "$out/diff.txt"
  exit 1
fi
