# MDeck Backlog

Ideas and larger changes collected during the September 2026 "fresh eyes"
review of the whole product. Everything here needs a product decision or is
big enough to deserve its own design pass; small fixes from the same review
were applied directly (see `CHANGELOG.md`).

Each item has a rough size (S < 1 day, M = a few days, L = a week or more)
and a recommendation. Items are grouped, and roughly ordered by expected value
within each group.

---

## 1. Presenting

### 1.1 Presenter view on a second display — L, recommended
Speaker notes (`???`) are parsed but never shown. A second egui viewport on the
other monitor with the current slide, the next slide, notes, and a clock would
make `M` (move to monitor) genuinely useful and is the single most requested
feature class for a presentation tool.
Decision: scope (notes + next slide + timer?) and whether the audience window
or the presenter window is the "main" one.

### 1.2 Elapsed / countdown timer in the HUD — S
A small timer in the HUD (start on first slide change, `Shift+R` to reset).
Could ship before 1.1 and be reused by it.

### 1.3 Per-slide directives (`@theme`, `@transition`, `@background`, `@footer`) — M
The spec and `samples/introducing-mdeck.md` promise per-slide `@theme`,
`@background` and `@transition`, and `@footer`/`@class`/`@code-theme`/`@aspect`
globally. Only `@layout` is read today; the rest are parsed and silently
dropped (`Slide.directives` is unused). The spec now marks them as reserved.
Decision: implement per-slide theme + transition + footer (M), or remove them
from the spec for good. `@aspect` (letterboxing 4:3) is a separate decision.

### 1.4 Warn on unknown / unsupported directives in `--check` — S
The spec says unknown directives warn; nothing does. `--check` only validates
diagram routing. Extend it to unknown directives, unresolved image paths,
values that failed to parse in visualizations, and theme-name typos
(`Theme::from_name` silently falls back to light).

### 1.5 Laser pointer — S
The annotation system (pen, arrow) already exists; a laser dot mode
(e.g. hold `L`) is a small addition.

### 1.6 Remote control from a phone — L
Localhost WebSocket + QR code so a phone can advance slides and show notes.
Overlaps with 1.1; decide together.

### 1.7 Auto-fit text instead of scroll — M, needs a decision
When a slide overflows, shrink heading/body up to ~30% before falling back to
scrolling. Best fit for "any markdown should be presentable", but changes the
look of existing decks and interacts with reveal steps. Prototype behind a
`@fit: shrink|scroll` directive first.

### 1.8 Bundled fonts: real bold/italic and colour emoji — M, needs a decision
Only egui's Ubuntu-Light is loaded, so `**bold**` is barely visible (now
rendered in the heading colour as an interim fix) and emoji are monochrome.
Bundling a family with bold/italic (e.g. Inter or IBM Plex) and Noto Color
Emoji adds 1–3 MB to the binary. Decision: which family, and whether to allow
`@font:` overrides.

### 1.9 Code blocks: long lines — S, needs a decision
Wrapped code is rarely what presenters want. Options: shrink the font until
the longest line fits (down to a floor), then clip with a fade. Same for very
long code blocks (already scrollable).

### 1.10 Crash recovery via re-exec — S
The old "retry up to 5 times" loop never worked (winit refuses a second event
loop per process) and has been removed. Real recovery would re-exec the binary
with `--slide N`.

---

## 2. Export and sharing

### 2.1 PDF export — M, recommended
Reuse the PNG export pipeline (now pixel-exact and tiled) and write one page
per slide with `printpdf` or `pdf-writer`. Speaker notes as PDF annotations or
a "notes" variant with two slides per page.

### 2.2 HTML export / `mdeck serve` — L
A static HTML export (images + navigation) or a local web server so decks can
be shared without installing mdeck. Needs a rendering strategy (server-side
PNGs vs. a WASM build of the renderer).

### 2.3 Headless export — S/M
Export still opens a visible window per run. eframe only paints visible
windows, so a truly headless path needs an offscreen glow context. Evaluate
`with_visible(false)` per platform; may not be possible without a custom
event loop.

### 2.4 Evaluate the wgpu renderer — S
eframe 0.36 defaults to wgpu (Metal/Vulkan/DX12). mdeck stays on glow because
wgpu's asynchronous screenshot never completed in the export loop. Worth
re-testing for the presentation window itself (smoother on macOS, where
OpenGL is deprecated) while keeping glow for export.

---

## 3. Markdown compatibility

### 3.1 Replace the hand-written block/inline parser with `pulldown-cmark` — L, recommended
The review found a long tail of CommonMark gaps (many fixed now: lazy list
continuation, setext headings, HTML comments, escapes, `_emphasis_`, double
backticks, escaped pipes, image titles, code info strings). Building on
`pulldown-cmark` events and keeping only the mdeck layer (directives, `+++`,
`???`, `@viz` fences, `+`/`*` reveal markers) would close the remaining
differences for good. Decision: worth a parser rewrite with the existing
test-suite as the safety net.

### 3.2 Reveal markers vs. ordinary bullets — needs a decision
`+` and `*` bullets are repurposed as reveal markers, so any README that uses
`*` bullets gets step-wise reveal in mdeck. Options: opt-in per slide
(`@reveal: true`), or only treat them as markers when a slide mixes markers.

### 3.3 Small CommonMark features — S each
Task lists (`- [ ] item`) as checkboxes, hard line breaks (trailing two spaces
or `\`), ordered list start numbers (`5.`), nested blockquotes, footnotes.

---

## 4. Visualizations

### 4.1 Negative values and a real axis minimum — M
Bar/line/stacked charts assume a 0 baseline; a series of 98, 99, 100 flat-lines
and negatives are clamped. Support `# min:`/`# max:` and auto-detect negative
ranges (zero line inside the chart).

### 4.2 Unit-aware numbers and formatting — S/M
Parsing now accepts `$4,200`, `12%`, `40 units`; formatting still prints
`1200000`. Add thousands separators / SI suffixes (`1.2M`) in value and axis
labels, and let a `# format:` directive control it.

### 4.3 Stacked bar: optional categories, percent mode — S
`# mode: percent` normalises each stack to 100%.

### 4.4 Gantt extensions — M
Quarter/year time scales, milestones (`- Launch: 2026-03-01, milestone`), a
"today" marker, and two-pass dependency resolution so `after X` can reference
a later task.

### 4.5 Org chart: real tree layout — S/M
Children are evenly spaced regardless of node width and are not centred under
their parent. A subtree-width layout (Reingold–Tilford) fixes both.

### 4.6 Architecture diagrams: route all edges once — S, behaviour change
Edges are re-routed on every reveal step (only visible edges are routed), so
already-drawn edges can jump lanes when a new one appears, and A* runs on the
UI thread per step. Routing all edges once and hiding unrevealed ones is
simpler and faster but changes layouts of existing decks slightly.

### 4.7 Diagram edge labels avoid each other — M
In dense diagrams (see "Large System" in `samples/visualizations/architecture.md`)
edge labels overlap ("serves pages"/"routes", the two "observes") and lines run
under labels. Place labels along the longest free segment of each route and
nudge them apart with a simple repulsion pass.

### 4.8 Legend placement below the chart when the column is narrow — S
Pie/donut/line legends live in a fixed right column that breaks in two-column
layouts.

### 4.9 Git graph: separate "simultaneous merge" syntax from the `*` reveal marker — S, needs a decision
`*` currently changes both reveal grouping and geometry (vertical line vs.
S-curve).

### 4.10 Reveal animation for timeline and git graph — S
Both reveal instantly; every other visualization animates.

### 4.11 Parse visualizations once at parse time — M (tech debt, performance)
Every `draw_*` re-parses the block text, rebuilds vectors, and lays out labels
every frame. Parsing into typed data in the parser removes all of it and
enables `--check` validation of chart data.

---

## 5. Code health

### 5.1 A `Layout::measure` API — M
Overflow detection uses a single estimate in `render/mod.rs`; each layout
draws with its own geometry. A per-layout `measure` next to `render` (sharing
the geometry helpers) is the structural fix behind the remaining clipping
edge cases (visualization and two-column layouts have no overflow strategy).

### 5.2 Shared visualization helpers — S each
Legends (vertical/horizontal), Y-grid loop, pill labels (git graph vs. diagram
edges), `DiagramReveal` = `VizReveal`, diagram step assignment = `assign_steps`,
routing setup built twice (`check_diagram_routes` vs `draw_diagram_sized`),
pie and donut are ~90% identical.

### 5.3 Theme as an enum — S
Theme identity is a string compared in six places.

### 5.4 Test coverage — ongoing
Layouts (`render/layouts/*`) and `render/text.rs` have few tests; `config.rs`
had none until this pass. Headless egui (`Context::default()` + `run`) can
cover measure-vs-draw agreement. Consider snapshot tests of exported PNGs for
the gallery deck (perceptual hash, tolerance) to catch visual regressions in CI.

### 5.5 CI: add `cargo audit` and `cargo deny` — S
Advisories were found (and fixed by upgrading) only because the audit was run
by hand. Add a scheduled audit workflow.

---

## 6. Documentation and onboarding

- Generate the keyboard-shortcut table for the HUD, README, spec and
  `mdeck spec --short` from one shared table so they cannot drift again (the
  card and HUD now share `app::SHORTCUTS`; README/spec are still hand-written).
- A short animated GIF/video in the README showing transitions and reveals.
- A "from README to talk in 60 seconds" tutorial page.
