<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Presenting

`mdeck talk.md` opens the deck fullscreen. Edit the file while presenting and
MDeck reloads it in place, staying on the current slide.

| Key | Action |
|-----|--------|
| Space, N, Right, PageDown, Enter | Next slide or reveal step |
| P, Left, PageUp, Backspace | Previous slide |
| Up, Down, scroll wheel | Scroll a long slide |
| Home, End | First / last slide |
| G | Grid overview (click a slide to jump to it) |
| T | Cycle transition (slide, fade, spatial, none) |
| Shift+T | Cycle theme (the built-ins, then your own) |
| F | Toggle fullscreen |
| M | Move to the next monitor |
| `.` or B | Blackout |
| H | Presenter HUD with shortcuts (and the current story beat's line) |
| S | AI for this slide: a story on the particles engine, a picture on an art engine |
| Esc | Clear drawings; press twice to quit (Q twice and Ctrl+C twice also quit) |

| Mouse | Action |
|-------|--------|
| Left click | Next slide |
| Right click | Previous slide |
| Left drag | Freehand pen |
| Right drag | Arrow |

Drawings fade away after a few seconds. Presentation clickers that send
PageUp/PageDown or Enter work out of the box, and keys pressed during a
transition are queued rather than lost.

Start options:

```bash
mdeck talk.md --windowed     # in a window instead of fullscreen
mdeck talk.md --slide 7      # start on slide 7
mdeck talk.md --overview     # start in the grid overview
mdeck talk.md --check        # validate the deck without opening a window
```
