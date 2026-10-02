# 09. Presenting, export and checking

The runtime: the presenting window and the presenter view, navigation, transitions, the opening
and the ending, export and `--check`.

`mdeck deck.md` opens the slide window; `mdeck export deck.md` writes PNG or PDF through the same
deck and drawing code; `mdeck --check deck.md` reports everything in the deck that will not show
as the author wrote it. The three share one understanding of the deck, so what `--check -v`
describes is what the window presents and what export writes.

## Requirements

### Presenting

- **RUN-01** MUST `implemented`: Presenting is a native, full-screen, GPU-rendered window. It is
  smooth at the display's refresh rate, resolution-independent (scale factor
  `min(w/1920, h/1080)`), and works on macOS, Linux and Windows.
- **RUN-02** MUST `implemented`: One key table (`app/keys.rs`) drives key handling, the HUD, the
  format reference and `mdeck spec --short`. Every binding has exactly one meaning per mode.
- **RUN-03** MUST `implemented`: There is a presenter view, opened with `V` while presenting or
  with `--presenter` at launch, in a second window placed on another display when there is one.
  It shows:
  - the current slide, with the engine's layer;
  - the next step or slide;
  - the notes, rendered as markdown (MD-15);
  - the elapsed time (`Shift+V` resets it).

  With one display, `V` shows a notes overlay over the slides instead.
- **RUN-04** SHOULD `implemented`: Typing a slide number and pressing Enter jumps to that slide.
- **RUN-05** MUST `implemented`: Live reload keeps the current slide and step when the deck
  changes on disk.
- **RUN-06** MUST `implemented`: Pen (left drag) and arrow (right drag) annotations are drawn with
  the mouse, are cleared with Esc, and are never saved into the deck.
- **RUN-07** MUST `implemented`: `--theme` works for presenting as well as for export and
  `--check`.

### Transitions

- **RUN-08** MUST `implemented`: Transitions are `slide`, `fade`, `spatial` and `none`, with
  smooth easing. The transition is set by the deck, then the theme's `transition:`, then the user
  config, then the built-in default, `fade` (the default theme is `dark`, VIS-07). A blank or
  unknown value falls through to the next in line. `T` cycles the transition while presenting.
- **RUN-09** MUST `implemented`: A slide can set its own transition into it (`transition:` in its
  settings), used when it is entered going forward and when it is left going back. The thermal
  spot zoom is `transition: zoom` with `zoom-to: <spot>` on the same slide.
- **RUN-10** MUST `deferred to 2.x`: A board engine owns transitions; a per-slide transition on a
  board is ignored, and `--check` says so. *Deferred:* the board ignores it, but `--check` does
  not yet warn that a slide's `transition` has no effect on a board engine.
- **RUN-11** MUST `implemented`: The overview (`G`) zooms in and out with animation.

### Opening and ending

- **RUN-12** MUST `implemented`: The countdown has one switch with a clear precedence:
  - the deck's `countdown: on|off` wins;
  - otherwise the theme's `countdown:` default applies;
  - the engine decides how it looks;
  - an engine without its own countdown shows plain numerals.
- **RUN-13** MUST `implemented`: Navigating past the last slide shows the end: the engine's end
  act or the default end slide.

### Reduced motion

- **RUN-14** MUST `implemented`: Reduced motion (`--reduced-motion` or
  `defaults.reduced_motion`) shows every settled state: no transitions, no reveal animation,
  still engine frames, no countdown. Content is never lost under reduced motion.

### Export

- **RUN-15** MUST `implemented`: Export shows exactly what the window shows (VIS-21), including
  chrome such as the footer and the slide counter.
- **RUN-16** MUST `implemented`: PNG output is exactly the requested size on any display. PDF has
  one page per slide (or per step with `--debug`), an outline entry per slide, and optional notes
  pages (`--notes`).
- **RUN-17** MUST `implemented`: Exports are reproducible: the same deck, theme and engine always
  give the same pixels. Stills of motion are rehearsed on a fixed clock: `--at <seconds>` runs
  the engine that long from a cold start, and `--moment countdown|3|2|1|burst|end` exports the
  opening or the ending as one image.
- **RUN-18** MAY `deferred to 2.x`: Export a video or animated image of a slide's motion
  (rehearsed with the same clock as `--at`), so decks can be shared with their wow effect intact.
  *Deferred:* decided as later, not in v2.0 (Q13).

### Checking

- **RUN-19** MUST `implemented`: `mdeck --check` validates everything the author writes:
  - deck and slide settings: names and values, and v1 syntax with its v2 form (`settings`);
  - slide boundaries and which settings apply to which slide;
  - design names, and chosen designs that cannot hold their content (`settings`);
  - visual tags and visual syntax (`visual`);
  - image options and content that cannot be presented (`content`);
  - missing or stale generated assets (`assets`), required packs and extensions (`extensions`),
    background images (`background`), point clouds (`point-cloud`), thermal sources (`thermal`);
  - engine support for what the deck uses (`engine`);
  - fonts, math, theme problems and contrast (`fonts`, `math`, `theme`), and diagram routing
    (`architecture`).

  It exits with 1 when there are warnings. `--check --theme <name>` checks the deck in that
  theme.
- **RUN-20** MUST `implemented`: Every warning names its file line and slide, and says what to do.
- **RUN-21** MUST `implemented`: `--check -v` prints the deck as mdeck understands it: for each
  slide, the design and the rule that recognised it (or why a chosen design fell back), the
  settings that apply and where they come from, the steps, the picture and whether it has notes.
  This is the primary debugging tool for authors and AI agents.
- **RUN-22** SHOULD `deferred to 2.x`: `--check --json` produces the same report as
  machine-readable JSON, for editors and AI agents. *Deferred:* not built for 2.0; the text report
  is stable enough for agents to read.
