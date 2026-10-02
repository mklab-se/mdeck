#!/usr/bin/env bash
# Regenerate every documentation image from the decks it shows: the README
# showcase and hero (media/showcase/), the gallery (media/gallery/), the
# tutorial (media/tutorial/) and the spec (docs/spec/images/). Each line below
# names an image, the deck, the slide and the export options it is made with,
# so a rendering change is one command away from current docs.
#
#   scripts/doc-images.sh [mdeck-binary]     # default: target/release/mdeck
#
# Needs python3 with Pillow (JPEG, GIF and animated WebP encoding). The presenter
# views (media/showcase/presenter.jpg, media/tutorial/07-presenter.jpg) and the
# SDK tutorial images (docs/sdk/images/) are not exports of a deck and are kept.
set -euo pipefail

cd "$(dirname "$0")/.."
mdeck=${1:-target/release/mdeck}
[ -x "$mdeck" ] || { echo "no mdeck binary at $mdeck (cargo build --release)" >&2; exit 2; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
src=scripts/doc-images

# run <export arguments...>: quiet, unless it fails
run() {
  "$mdeck" export "$@" >"$work/log" 2>&1 || { cat "$work/log" >&2; return 1; }
}

# shot <out> <width> <height> <deck> <slide> [export options...]
shot() {
  local out=$1 w=$2 h=$3 deck=$4 slide=$5
  shift 5
  local dir="$work/$(echo "$out" | tr / _)"
  run "$deck" --slide "$slide" --width "$w" --height "$h" -o "$dir" "$@"
  encode "$dir/slide-$(printf %02d "$slide").png" "$out"
}

# moment <out> <deck> <moment> [export options...]
moment() {
  local out=$1 deck=$2 which=$3
  shift 3
  local dir="$work/$(echo "$out" | tr / _)"
  run "$deck" --moment "$which" --width 1280 --height 720 -o "$dir" "$@"
  encode "$(ls "$dir"/*.png | head -1)" "$out"
}

# encode <png> <out>: JPEG (quality 86) or PNG by the output's extension
encode() {
  python3 - "$1" "$2" <<'EOF'
import sys
from PIL import Image
src, out = sys.argv[1], sys.argv[2]
im = Image.open(src).convert('RGB')
if out.endswith('.png'):
    im.save(out, optimize=True)
else:
    im.save(out, quality=86, optimize=True, progressive=True)
EOF
}

g() { shot "media/gallery/$1.jpg" 1280 720 "${@:2}"; }
s() { shot "media/showcase/$1.jpg" 1280 720 "${@:2}"; }
t() { shot "media/tutorial/$1.jpg" 1280 720 "${@:2}"; }
p() { shot "docs/spec/images/$1.png" 960 540 "${@:2}"; }

echo "gallery: designs"
designs=(title section statement points split media gallery quote code visual columns table content)
for i in "${!designs[@]}"; do
  g "design-${designs[$i]}-standard" samples/features/designs.md $((i + 1)) --theme dark
  g "design-${designs[$i]}-editorial" samples/features/designs.md $((i + 1)) --theme ember --at 8
done
g arrangement-points samples/features/designs.md 4
g arrangement-quote samples/features/designs.md 8

echo "gallery: engines"
g engine-plain samples/showcase/launch.md 2 --theme dark
g engine-particles samples/showcase/launch.md 2 --theme ember --at 8
g engine-particles-gear samples/showcase/launch.md 8 --theme ember --at 8
g engine-led-title samples/engines/led.md 1
g engine-led samples/engines/led.md 3
g engine-led-chart samples/engines/led.md 6
g engine-splitflap-schedule samples/engines/splitflap.md 2
g engine-splitflap-progress samples/engines/splitflap.md 4
g engine-splitflap samples/engines/splitflap.md 6
g engine-blocks-title samples/engines/blocks.md 1
g engine-blocks samples/engines/blocks.md 3
g engine-thermal-title samples/engines/thermal.md 1
g engine-thermal samples/engines/thermal.md 2
g engine-thermal-signature samples/engines/thermal.md 5
g engine-thermal-compare samples/engines/thermal.md 7
g engine-line-sheet-title samples/engines/blueprint.md 1
g engine-line-sheet samples/engines/blueprint.md 2
g engine-line-sheet-fallback samples/engines/blueprint.md 6
g engine-line-slate-title samples/engines/chalkboard.md 1
g engine-line-slate samples/engines/chalkboard.md 3
g engine-sketch-title samples/engines/sketch.md 1
g engine-sketch samples/engines/sketch.md 2
g engine-watercolour-title samples/engines/watercolour.md 1
g engine-watercolour samples/engines/watercolour.md 3
g engine-darkroom-title samples/engines/darkroom.md 1
g engine-darkroom-print samples/engines/darkroom.md 2
g engine-darkroom samples/engines/darkroom.md 3
moment media/gallery/moment-countdown-particles.jpg samples/showcase/launch.md countdown --theme ember
moment media/gallery/moment-countdown-led.jpg samples/showcase/launch.md countdown --theme marquee
moment media/gallery/moment-countdown-splitflap.jpg samples/showcase/launch.md countdown --theme departures
moment media/gallery/moment-end-thermal.jpg samples/engines/thermal.md end

echo "gallery: visuals"
visuals=(architecture wordcloud timeline pie bar bar-horizontal line donut kpi funnel radar
         stackedbar venn progress scatter orgchart gantt gitgraph flower artifactflow)
for i in "${!visuals[@]}"; do
  case ${visuals[$i]} in
    wordcloud) g visual-wordcloud samples/visualizations/all.md 3 --theme dark ;;
    *) g "visual-${visuals[$i]}" samples/visualizations/all.md $((i + 2)) --theme ember --at 8 ;;
  esac
done
g visual-thermal samples/engines/thermal.md 3

echo "gallery: themes"
g theme-light samples/showcase/launch.md 4 --theme light
g theme-nord samples/showcase/launch.md 2 --theme nord
g theme-spring samples/showcase/launch.md 5 --theme spring
g theme-summer samples/showcase/launch.md 5 --theme summer
g theme-autumn samples/showcase/launch.md 2 --theme autumn --at 8
g theme-winter samples/showcase/launch.md 3 --theme winter --at 8
g theme-from-design-system samples/themes/custom-theme.md 2 --at 8

echo "showcase"
s plain-title $src/readme.md 1
s plain-points $src/readme.md 2
s ember-points samples/showcase/launch.md 2 --theme ember --at 8
s ember-architecture samples/showcase/launch.md 4 --theme ember --at 8
s led-points samples/showcase/launch.md 2 --theme marquee
s chart samples/showcase/launch.md 5 --theme marquee
s departures samples/showcase/launch.md 7 --theme departures
s stack samples/showcase/launch.md 8 --theme stack
s thermal samples/engines/thermal.md 2
s blueprint samples/engines/blueprint.md 2
s chalkboard samples/engines/chalkboard.md 3
s sketchbook samples/engines/sketch.md 2
s watercolour samples/engines/watercolour.md 3
s darkroom samples/engines/darkroom.md 3

echo "tutorial"
t 01-title $src/tutorial-2.md 1
t 01-statement $src/tutorial-2.md 2
t 01-points $src/tutorial-2.md 3
t 03-chart $src/tutorial-5.md 4
t 04-diagram $src/tutorial-6.md 5
t 05-ember-title $src/tutorial-7.md 1 --at 8
t 05-ember-points $src/tutorial-7.md 3 --at 8
t 06-picture samples/tutorial/talk.md 6 --at 8
t 08-marquee samples/tutorial/talk.md 6 --theme marquee
run $src/tutorial-4.md --slide 3 --debug --width 800 --height 450 -o "$work/reveal"
python3 - "$work/reveal" media/tutorial/02-reveal.gif <<'EOF'
import glob, sys
from PIL import Image
frames = [Image.open(f).convert('RGB') for f in sorted(glob.glob(sys.argv[1] + '/slide-03-step-*.png'))]
pal = [f.quantize(colors=64, method=Image.Quantize.MEDIANCUT) for f in frames]
pal[0].save(sys.argv[2], save_all=True, append_images=pal[1:], duration=[1200] * (len(pal) - 1) + [2400], loop=0, optimize=True)
EOF

echo "spec"
p standard-points $src/orbit.md 2 --theme dark
p standard-quote $src/orbit.md 3 --theme dark
p editorial-points $src/orbit.md 2 --theme ember --at 8
p editorial-quote $src/orbit.md 3 --theme ember --at 8
p ember-on-plain-points $src/orbit.md 2 --theme ember --engine plain
p editorial-split $src/orbit-split.md 2 --theme ember --at 8

echo "hero"
# Six engines in motion: stills of a cold start every 0.12 s, a hold on the
# settled slide, and a short cross-fade into the next.
hero() {
  local name=$1 deck=$2 slide=$3 frames=$4 every=$5
  shift 5
  for ((i = 0; i < frames; i += every)); do
    run "$deck" --slide "$slide" --at "$(python3 -c "print(round($i * 0.12, 2))")" \
      --width 960 --height 540 -o "$work/hero/$name/$(printf %03d "$i")" "$@"
  done
}
hero 1-ember samples/showcase/launch.md 2 31 1 --theme ember
hero 2-marquee samples/showcase/launch.md 2 12 1 --theme marquee
hero 3-departures samples/showcase/launch.md 7 22 1 --theme departures
hero 4-blueprint samples/engines/blueprint.md 2 36 2
hero 5-watercolour samples/engines/watercolour.md 3 36 2
hero 6-thermal samples/engines/thermal.md 2 17 1
python3 - "$work/hero" media/showcase/hero.webp <<'EOF'
import glob, sys
from PIL import Image
root, out = sys.argv[1], sys.argv[2]
scenes = [[Image.open(f).convert('RGB') for f in sorted(glob.glob(d + '/*/*.png'))]
          for d in sorted(glob.glob(root + '/*'))]
frames, durations = [], []
for i, scene in enumerate(scenes):
    frames += scene
    durations += [120] * len(scene)
    durations[-1] = 1500
    following = scenes[(i + 1) % len(scenes)][0]
    for k in range(1, 6):
        frames.append(Image.blend(scene[-1], following, k / 6))
        durations.append(60)
frames[0].save(out, save_all=True, append_images=frames[1:], duration=durations, loop=0,
               quality=72, method=6, minimize_size=True, allow_mixed=True)
EOF
echo "done"
