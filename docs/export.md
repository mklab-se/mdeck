<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Export

Slides to PNG at any resolution, or to one PDF, optionally with speaker notes.

```bash
mdeck export talk.md                              # slide-01.png ... at 1920x1080
mdeck export talk.md --width 3840 --height 2160   # 4K
mdeck export talk.md --output-dir slides/         # choose the folder
mdeck export talk.md --debug                      # one PNG per reveal step
mdeck export talk.md --slide 7                    # just slide 7 (file names keep the deck numbering)
mdeck export talk.md --range 3-5 --debug          # slides 3 to 5, every step
mdeck export talk.md --format pdf                 # export/talk.pdf, one page per slide
mdeck export talk.md --format pdf --notes         # export/talk-notes.pdf, slide + speaker notes
mdeck export talk.md --theme winter               # in another theme, without editing the deck
mdeck export talk.md --slide 3 --at 0.3           # a still of the engine's motion, 0.3 s in
mdeck export talk.md --slide 1 --moment countdown # the opening countdown (or 3, 2, 1, burst, end)
```

The footer (`footer`), the slide counter and the editorial chrome are drawn
in export exactly as in the window.

`--at <seconds>` runs the engine from a cold start for that long (simulated
at 60 frames a second) and exports that frame; `--moment countdown|end`
exports the opening countdown or the end act instead of the slide (`3`, `2`,
`1` and `burst` pick a single countdown phase). A moment is one image,
`countdown.png` (`countdown-2.png`, `countdown-burst.png`, ...) or `end.png`,
drawn on the slide `--slide` names, else the first slide for the countdown
and the last for the end. They replace the
`MDECK_EXPORT_AT` and `MDECK_EXPORT_MOMENT` environment variables.

Output is always exactly the requested size, independent of your screen's
size or DPI: slides larger than the display are rendered in tiles and stitched.

A PDF is the answer to "can I have the slides?": every page is the slide
exactly as presented (final reveal step, Ember field frozen), 13.33 x 7.5 in
like a widescreen PowerPoint deck, with a bookmark per slide. `--notes` makes
printable notes pages instead: the slide on top and its speaker notes (the `@notes` blocks)
below, dark on white whatever the theme, in A4 proportions; long notes
continue on the next page. Pages are images, so text in the PDF is not
selectable.
