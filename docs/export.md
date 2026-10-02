<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Export

Slides to PNG at any resolution, or to one PDF, optionally with speaker notes. An export shows
exactly what the presenting window shows: the same layout, theme, engine layer, logo, footer and
slide counter.

```bash
mdeck export talk.md                              # export/slide-01.png ... at 1920x1080
mdeck export talk.md --width 3840 --height 2160   # 4K
mdeck export talk.md --output-dir slides/         # choose the folder (-o)
mdeck export talk.md --slide 7                    # just slide 7 (file names keep the deck numbering)
mdeck export talk.md --range 3-5                  # slides 3 to 5
mdeck export talk.md --debug                      # one PNG per reveal step
mdeck export talk.md --format pdf                 # export/talk.pdf, one page per slide
mdeck export talk.md --format pdf --notes         # export/talk-notes.pdf, slide and notes
mdeck export talk.md --theme winter               # in another theme, without editing the deck
mdeck export talk.md --engine plain               # on another engine
```

Output is always exactly the requested size, whatever your screen's size or DPI: a slide larger
than the display is rendered in tiles and stitched.

## PDF

A PDF is the answer to "can I have the slides?": every page is the slide as presented at its last
step, with a bookmark per slide. At the default size a page is 13.33 x 7.5 in, like a widescreen
PowerPoint deck.

`--notes` makes printable notes pages instead: the slide on top and its speaker notes below,
rendered as markdown (headings, emphasis, lists, code, quotes, tables, math), dark on white
whatever the theme, in A4 proportions. Long notes continue on the next page; they never shrink.

Pages are images, so text in the PDF is not selectable.

## Stills of the motion

An export is the settled slide: the particle field frozen, a drawing finished, the heat settled.
To capture a moment of the motion instead:

```bash
mdeck export talk.md --slide 3 --at 0.8            # the engine 0.8 s after the slide opens
mdeck export talk.md --moment countdown            # the opening countdown (its 3)
mdeck export talk.md --moment burst                # or 3, 2, 1, burst, end
mdeck export talk.md --moment transition --slide 4 # the change into slide 4, halfway
```

`--at <seconds>` runs the engine from a cold start for that long (simulated at 60 frames a
second) and exports that frame. `--moment` exports the opening countdown, one of its digits, the
burst after it, the engine's end act, or the transition into a slide (halfway, or `--at` seconds
into it), instead of the slides. Both are reproducible: the same
command gives the same picture.
A moment is one image, `countdown.png` (`countdown-2.png`, `countdown-burst.png`, ...),
`end.png` or `transition.png`, drawn on the slide `--slide` names, else the first slide for the
countdown, the last for the end and the second for a transition.
