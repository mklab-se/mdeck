# 09. Presenting, export and checking

The runtime: the presenting window, navigation, transitions, export and `--check`.

## Today

### Presenting

`mdeck deck.md` opens a window. It takes these flags:

- `--windowed`
- `--slide N`
- `--overview`
- `--engine E`
- `--reduced-motion`
- `--check`

There is no `--theme` flag for presenting, although `export` has one.

### Keys

One table, `app/keys.rs`, drives key handling, the HUD and `mdeck spec --short`.

| Purpose | Keys |
|---|---|
| Forward | Space, N, Right, PageDown, Enter |
| Back | P, Left, PageUp, Backspace |
| Scroll | Up, Down |
| First / last slide | Home, End |
| Grid overview | G |
| Cycle transition / theme | T / Shift+T |
| Fullscreen / next monitor | F / M |
| HUD | H |
| Blackout | `.` or B |
| Debug overlay | R |
| Generate AI art or story for the slide (v2: art only) | S |
| Thermal palette / reset | C / Shift+C |
| Clear drawings | Esc |
| Quit | Esc×2, Q×2, Ctrl+C×2 |

Mouse:

- left click: next;
- right click: back;
- left drag: draws with the pen;
- right drag: draws arrows.

Missing:

- a presenter view and notes on screen;
- a timer;
- jumping to a slide by number.

### Other runtime behaviour

**Transitions.** `slide`, `fade`, `spatial`, `none`, plus the thermal `@zoom`. They are set per
deck, and per slide only through `@zoom`. A board engine forces `none` and runs its own.

**Overview.** An animated zoom into a grid of all slides.

**Opening and ending.**

- A countdown before slide 1, configured in two places:
  - the theme: `countdown: none|plain|burst`;
  - the deck: `@countdown`, where only `false` has an effect.
- A virtual end slide after the last one: either the engine's end act, or "The End" with the
  mdeck logo.

**Reduced motion.** `--reduced-motion` or `defaults.reduced_motion` shows every settled state:

- no transitions;
- no reveal animation;
- still engine frames;
- no countdown.

**Live reload.** Edits to the deck reload while presenting.

### Export

`mdeck export deck.md` writes PNG or PDF. Options:

- `--width` / `--height`;
- `--slide N` or `--range A-B`;
- `--debug`: one image per reveal step;
- `--notes`: PDF notes pages;
- `--theme`, `--engine`.

It uses the same deck and drawing code as the window. Stills of motion come from the
`MDECK_EXPORT_AT` / `MDECK_EXPORT_MOMENT` environment variables.

Parity gap: `@footer` is drawn in the window but not in export.

### `--check`

Prints warnings with line numbers, and exits with 1 when there are any. Categories:

- `architecture`
- `story`
- `illustration`
- `fonts`
- `math`
- `theme`
- `directive`
- `engine`
- `art`
- `background`
- `thermal`

`-v` prints each slide's layout and step count.

Not checked:

- frontmatter keys and values;
- layout names;
- fence tags;
- visual syntax;
- image hints;
- content that will render as raw syntax;
- dropped content.

## Assessment

1. **The window is strong; the presenter is underserved.** Notes exist but can only be printed.
   For a tool whose promise is a better presentation than Keynote, a presenter view with notes,
   the next slide and a timer is expected.
2. **The countdown has two switches that disagree.** The theme can set `none`, and the deck
   cannot turn it on.
3. **`--check` is strong where it exists and blind where most author mistakes happen** (see
   [07](07-authoring-language.md)).
4. **Transitions are deck-wide only.** The one per-slide transition (`@zoom`) is a thermal-specific
   directive with its own name.

## Requirements

### Presenting

- **RUN-01** MUST `keep`: Presenting is a native, full-screen, GPU-rendered window. It is smooth
  at the display's refresh rate, resolution-independent (scale factor `min(w/1920, h/1080)`), and
  works on macOS, Linux and Windows.
- **RUN-02** MUST `keep`: One key table drives handling, the HUD, the format reference and
  `mdeck spec --short`. Every binding has exactly one meaning per mode.
- **RUN-03** MUST `new`: v2.0 has a presenter view on a second display. It shows:
  - the current slide;
  - the next slide or step;
  - the notes, rendered as markdown (MD-15);
  - the elapsed time.

  With one display, a toggleable notes overlay is visible only on the presenter's screen when
  mirroring is off.
- **RUN-04** SHOULD `new`: Typing a number and pressing Enter jumps to that slide.
- **RUN-05** MUST `keep`: Live reload keeps the current slide and step when the deck changes on
  disk.
- **RUN-06** MUST `keep`: Pen and arrow annotations are drawn with the mouse, are cleared with Esc,
  and are never saved into the deck.
- **RUN-07** MUST `new`: `--theme` works for presenting as well as for export.

### Transitions

- **RUN-08** MUST `change`: Transitions are `slide`, `fade`, `spatial` and `none`, with smooth
  easing. The transition is set by the deck, then the theme, then the user config, then the
  built-in default, which is `fade` (today it is `slide`).
- **RUN-09** MUST `change`: A slide can set its own transition into it (`transition:` in its
  settings). The thermal spot zoom becomes `transition: zoom` with `zoom-to: <spot>`.
- **RUN-10** MUST `keep`: A board engine owns transitions. A per-slide transition on a board is
  ignored and reported.
- **RUN-11** MUST `keep`: The overview zooms in and out with animation.

### Opening and ending

- **RUN-12** MUST `change`: The countdown has one switch with a clear precedence:
  - the deck's `countdown: on|off` wins;
  - otherwise the theme's default applies;
  - the engine decides how it looks;
  - an engine without its own countdown shows plain numerals.
- **RUN-13** MUST `keep`: Navigating past the last slide shows the end: the engine's end act or
  the default end slide.

### Reduced motion

- **RUN-14** MUST `keep`: Reduced motion shows every settled state: no transitions, no reveal
  animation, still engine frames, no countdown. Content is never lost under reduced motion.

### Export

- **RUN-15** MUST `keep`: Export shows exactly what the window shows (VIS-21), including chrome
  such as the footer.
- **RUN-16** MUST `keep`: PNG output is exactly the requested size on any display. PDF has one
  page per slide (or per step with `--debug`), an outline entry per slide, and optional notes
  pages.
- **RUN-17** MUST `keep`: Exports are reproducible: the same deck, theme and engine always give
  the same pixels.
- **RUN-18** MAY `new`: A later version can export a video or animated image of a slide's motion
  (rehearsed with the same clock as `MDECK_EXPORT_AT`), so decks can be shared with their wow
  effect intact. Not in v2.0.

### Checking

- **RUN-19** MUST `change`: `mdeck --check` validates everything the author writes:
  - deck and slide settings: names and values;
  - slide boundaries and which settings apply to which slide;
  - design names, and designs that cannot hold their content;
  - visual tags and visual syntax;
  - image options;
  - content that cannot be presented;
  - missing assets and extensions;
  - engine support for what the deck uses;
  - fonts, math and contrast.
- **RUN-20** MUST `keep`: Every warning names its file line and slide, and says what to do.
- **RUN-21** MUST `new`: `--check -v` prints the deck as mdeck understands it: for each slide, the
  design (and why), the settings that apply, the steps, the picture and any fallbacks. This is the
  primary debugging tool for authors and AI agents.
- **RUN-22** SHOULD `new`: `--check --json` produces the same report as machine-readable JSON, for
  editors and AI agents.
