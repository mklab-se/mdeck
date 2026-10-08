<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Presenting

`mdeck talk.md` opens the deck fullscreen. Edit the file while presenting and mdeck reloads it
in place, on the same slide and with the same steps revealed. Press `H` at any time for the
shortcuts.

## Keys and mouse

| Key | Action |
|-----|--------|
| Space, N, Right, PageDown, Enter | Next slide or step |
| P, Left, PageUp, Backspace | Previous slide (shown fully revealed) |
| Up, Down, mouse wheel | Scroll a slide that overflows |
| Home, End | First, last slide |
| Digits, then Enter | Jump to that slide (`1` `2` Enter goes to slide 12; Backspace edits, Esc cancels) |
| G | Grid overview: arrows move, Enter, E or a click opens a slide |
| V | Presenter view (beside the slides with one display) |
| Shift+V | Reset the presenter timer |
| `.` or B | Black out the screen |
| T | Next transition (fade, slide, spatial, none, then any an extension adds) |
| Shift+T | Next theme (the built-ins, then your own) |
| F | Toggle fullscreen |
| M | Move the slides to the next display (remembered next time) |
| H | Shortcuts and status (with a frame rate counter) |
| C, Shift+C | Next palette for every `@thermal` image; back to the palettes as written |
| S | Draw this slide's picture in the background (art engines, needs AI) |
| R | Debug overlay (left, right, off) |
| Esc | Clear drawings; twice quits (so do Q twice and Ctrl+C twice) |

| Mouse | Action |
|-------|--------|
| Left click | Next slide |
| Right click | Previous slide |
| Left drag | Freehand pen |
| Right drag | Arrow |

Drawings fade away after about eight seconds (on the thermal engine a pen stroke arrives white-hot
and cools away). Clickers that send PageUp/PageDown or Enter work out of the box, and keys pressed
during a transition are queued, never lost. `T` and `Shift+T` last for the session; the deck file
is never changed.

## Presenter view

Press `V` (or start with `mdeck talk.md --presenter`) and a second window opens: the current
slide large, the next slide or step, the slide's speaker notes rendered as markdown (headings,
**emphasis**, lists, code, quotes, tables, math) and the elapsed time. `Shift+V` resets the timer;
keys typed in either window drive the deck, and `V` closes it.

**With a projector or a second screen**, the presenter window opens on another display than the
slides, your laptop's own screen when the slides are on the projector, wherever the displays sit
(left, right, above or below; on Linux mdeck looks beside the slides' display). If the slides open on the wrong screen, press `M` to move them to
the next display; mdeck remembers it for next time. Then press `V`.

**With one screen**, for rehearsing, the slides leave fullscreen and the two windows sit side by
side. Either can be dragged to another screen. `V` closes the presenter window and the slides go
back to fullscreen.

Notes are ```` ```@notes ```` blocks anywhere in a slide ([Writing slides](writing-slides.md#speaker-notes)).

## Starting

```bash
mdeck talk.md                    # fullscreen, from the first slide
mdeck talk.md --windowed         # in a window
mdeck talk.md --presenter        # with the presenter view open
mdeck talk.md --slide 7          # on slide 7 (skips the countdown)
mdeck talk.md --overview         # in the grid overview
mdeck talk.md --theme nord       # in another theme, without editing the deck
mdeck talk.md --engine plain     # on another engine
mdeck talk.md --reduced-motion   # settled states only (below)
mdeck talk.md --check            # validate without opening a window
```

**Transitions** come from the deck's `transition`, then the theme's `transition:`, then
`defaults.transition` in your config, then `fade`. A slide's own `transition` (in its settings
comment) sets how that slide is entered, and `zoom-to: <spot>` enters it by zooming into a named
thermal spot on the slide before.

**The countdown** before the first slide is on with `countdown: on` in the deck, off with
`countdown: off`, and otherwise follows the theme (Ember's is counted in particles; `dark` has
none). The engine decides how it looks, and any key skips it.

**Reduced motion** shows every slide and step in its settled state: no transitions, entry or
reveal animations, countdown or engine motion, as in an export. Steps still arrive one click at a
time. Make it the default with `mdeck config set defaults.reduced_motion true`.

## Your defaults

`mdeck config show` prints your configuration and `mdeck config set <key> <value>` changes it.
The file is `config.yaml` in your user folder (`~/.config/mdeck/` on Linux and macOS, or
`$XDG_CONFIG_HOME/mdeck/` when that is set; `%APPDATA%\mdeck\` on Windows).

| Key | Values |
|---|---|
| `defaults.theme` | a built-in or user theme (default `dark`) |
| `defaults.transition` | `fade`, `slide`, `spatial`, `none` |
| `defaults.start_mode` | `first`, `overview`, or a slide number |
| `defaults.reduced_motion` | `true`, `false` |
| `defaults.image_style`, `defaults.icon_style` | a named style (`mdeck ai style`) for generated images and icons |

A deck's own settings win over the config, and the config wins over the built-in values.

When mdeck runs into a problem while presenting (a slow frame, a failed image), it says so when
you quit and writes the details to a log in the `logs/` folder of your user folder.
