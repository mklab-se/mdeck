# MDeck Backlog

Ideas and larger changes collected during the September 2026 "fresh eyes"
review of the whole product. Everything here needs a product decision or is
big enough to deserve its own design pass; small fixes from the same review
were applied directly (see `CHANGELOG.md`).

Each item has a rough size (S < 1 day, M = a few days, L = a week or more)
and a recommendation. Items are grouped, and roughly ordered by expected value
within each group.

Updated for mdeck 2: items that 2.0 delivered (the presenter view and its timer,
slide settings, `--check` validation, `+` as the only step marker) or made
obsolete (stories) are gone. The mdeck 2 specification in `docs/spec/` records the
decisions; what it defers to 2.x is listed in its README (`docs/spec/README.md`).

---

## 1. Presenting

### 1.5 Laser pointer (S)
The annotation system (pen, arrow) already exists; a laser dot mode
(e.g. hold `L`) is a small addition.

### 1.6 Remote control from a phone (L)
Localhost WebSocket + QR code so a phone can advance slides and show notes.
Overlaps with 1.1; decide together.

### 1.7 Auto-fit text instead of scroll (M, needs a decision)
When a slide overflows, shrink heading/body up to ~30% before falling back to
scrolling. Best fit for "any markdown should be presentable", but changes the
look of existing decks and interacts with reveal steps. Prototype behind a
`@fit: shrink|scroll` directive first.

### 1.8 Bundled fonts: real bold/italic and colour emoji (M, needs a decision)
Only egui's Ubuntu-Light is loaded, so `**bold**` is barely visible (now
rendered in the heading colour as an interim fix) and emoji are monochrome.
Bundling a family with bold/italic (e.g. Inter or IBM Plex) and Noto Color
Emoji adds 1–3 MB to the binary. Decision: which family, and whether to allow
`@font:` overrides.

### 1.10 Crash recovery via re-exec (S)
The old "retry up to 5 times" loop never worked (winit refuses a second event
loop per process) and has been removed. Real recovery would re-exec the binary
with `--slide N`.

---

## 2. Export and sharing

### 2.1 PowerPoint export: decided against (#10)
PDF export shipped (`--format pdf`, `--notes`). PPTX is not planned: mdeck
exists because good-looking PowerPoint decks are hard to generate, and an
export would either be an image-per-slide file that only pretends to be a
deck or a second renderer that cannot look like mdeck. The PDF is the handout.

### 2.1b Selectable text in PDF export (M)
PDF pages are images, so text cannot be searched or copied. An invisible text
layer (render mode 3) with each block's text at its position would fix that;
it needs positions from the renderers and a font with a Unicode mapping.

### 2.2 HTML export / `mdeck serve` (L)
A static HTML export (images + navigation) or a local web server so decks can
be shared without installing mdeck. Needs a rendering strategy (server-side
PNGs vs. a WASM build of the renderer).

### 2.3 Headless export (S/M)
Export still opens a visible window per run. eframe only paints visible
windows, so a truly headless path needs an offscreen glow context. Evaluate
`with_visible(false)` per platform; may not be possible without a custom
event loop.

### 2.4 Evaluate the wgpu renderer (S)
eframe 0.36 defaults to wgpu (Metal/Vulkan/DX12). mdeck stays on glow because
wgpu's asynchronous screenshot never completed in the export loop. Worth
re-testing for the presentation window itself (smoother on macOS, where
OpenGL is deprecated) while keeping glow for export.

---

## 3. Markdown compatibility

### 3.1 Replace the hand-written block/inline parser with `pulldown-cmark` (L, recommended)
The review found a long tail of CommonMark gaps (many fixed now: lazy list
continuation, setext headings, HTML comments, escapes, `_emphasis_`, double
backticks, escaped pipes, image titles, code info strings). Building on
`pulldown-cmark` events and keeping only the mdeck layer (settings comments, `+++`,
`@notes` and visual fences, `+` steps) would close the remaining
differences for good. Decision: worth a parser rewrite with the existing
test-suite as the safety net.

### 3.3 Small CommonMark features (S each)
Task lists (`- [ ] item`) as checkboxes, hard line breaks (trailing two spaces
or `\`), ordered list start numbers (`5.`), nested blockquotes, footnotes.

---

## 4. Visualizations

### 4.1 Negative values and a real axis minimum (M)
Bar/line/stacked charts assume a 0 baseline; a series of 98, 99, 100 flat-lines
and negatives are clamped. Support `# min:`/`# max:` and auto-detect negative
ranges (zero line inside the chart).

### 4.2 Unit-aware numbers and formatting (S/M)
Parsing now accepts `$4,200`, `12%`, `40 units`; formatting still prints
`1200000`. Add thousands separators / SI suffixes (`1.2M`) in value and axis
labels, and let a `# format:` directive control it.

### 4.3 Stacked bar: optional categories, percent mode (S)
`# mode: percent` normalises each stack to 100%.

### 4.4 Gantt extensions (M)
Quarter/year time scales, milestones (`- Launch: 2026-03-01, milestone`), a
"today" marker, and two-pass dependency resolution so `after X` can reference
a later task.

### 4.5 Org chart: real tree layout (S/M)
Children are evenly spaced regardless of node width and are not centred under
their parent. A subtree-width layout (Reingold–Tilford) fixes both.

### 4.6 Architecture diagrams: route all edges once (S, behaviour change)
Edges are re-routed on every reveal step (only visible edges are routed), so
already-drawn edges can jump lanes when a new one appears, and A* runs on the
UI thread per step. Routing all edges once and hiding unrevealed ones is
simpler and faster but changes layouts of existing decks slightly.

### 4.7 Diagram edge labels avoid each other (M)
In dense diagrams (see "Large System" in `samples/visualizations/architecture.md`)
edge labels overlap ("serves pages"/"routes", the two "observes") and lines run
under labels. Place labels along the longest free segment of each route and
nudge them apart with a simple repulsion pass.

### 4.8 Legend placement below the chart when the column is narrow (S)
Pie/donut/line legends live in a fixed right column that breaks in two-column
layouts.

### 4.10 Reveal animation for timeline and git graph (S)
Both reveal instantly; every other visualization animates.

### 4.11 Parse visualizations once at parse time (M (tech debt, performance))
Every `draw_*` re-parses the block text, rebuilds vectors, and lays out labels
every frame. Parsing into typed data in the parser removes all of it and
enables `--check` validation of chart data.

### 4.12 Architecture diagram: edge-aware auto layout (M)
Auto layout ignores edges: up to five nodes go in one row in declaration order,
more in a square grid, so a producer -> service -> consumer chain only comes out
right with explicit `pos:`. Rank nodes by longest path from a source, one
column per rank, rows ordered to keep edges short (a light Sugiyama).

### 4.13 Radial ecosystem and producer/consumer flow visualizations (M each, #3, #4)
Two new, purpose-built visualizations, each designed from scratch for what it
shows, not layouts bolted onto the architecture diagram (see the "Purpose-built
over reused" principle in CLAUDE.md). #3: a platform at the centre with its
ecosystem around it. #4: producers, the artifacts they hand over, and the
consumers that use them. Both get their own syntax, layout, typography and
reveal animation; they may share helpers (colours, font sizes, reveal) but not
the architecture grid. Decision (2026-09-25): implement as new visualizations,
not yet scheduled.

---

## 5. Code health

### 5.1 A `Layout::measure` API (M)
Overflow detection uses a single estimate in `render/mod.rs`; each layout
draws with its own geometry. A per-layout `measure` next to `render` (sharing
the geometry helpers) is the structural fix behind the remaining clipping
edge cases (visualization and two-column layouts have no overflow strategy).

### 5.2 Shared visualization helpers (S each)
Legends (vertical/horizontal), Y-grid loop, pill labels (git graph vs. diagram
edges), `DiagramReveal` = `VizReveal`, diagram step assignment = `assign_steps`,
routing setup built twice (`check_diagram_routes` vs `draw_diagram_sized`),
pie and donut are ~90% identical.

### 5.3 Theme as an enum (done)
Done with custom themes (#14): themes are data files and behaviour goes
through `Engine`; nothing compares theme names any more.

### 5.4 Test coverage (ongoing)
Layouts (`render/layouts/*`) and `render/text.rs` have few tests; `config.rs`
had none until this pass. Headless egui (`Context::default()` + `run`) can
cover measure-vs-draw agreement. Consider snapshot tests of exported PNGs for
the gallery deck (perceptual hash, tolerance) to catch visual regressions in CI.

### 5.5 Signed release attestations (S, recommended)
Done in 0.19.0: every release ships a CycloneDX 1.5 SBOM per target
(`cargo cyclonedx`) and binaries are built with `cargo auditable`, so the
dependency list is embedded in the executable (`cargo audit bin ./mdeck`).
Still open: sign the archives and SBOMs with GitHub artifact attestations
(`actions/attest-build-provenance`) so downloads can be verified with
`gh attestation verify`. MDeck is free MIT software and therefore outside the
EU Cyber Resilience Act's scope today, but corporate adopters ask for signed
provenance, and it would become mandatory if MDeck were ever sold or supported
commercially.

### 5.6 CI: add `cargo audit` and `cargo deny` (S)
Advisories were found (and fixed by upgrading) only because the audit was run
by hand. Add a scheduled audit workflow.

---

## 6. Ember follow-ups

Collected while shipping 1.0. All are polish on a working feature.

### 6.2 Reduced motion from the OS, and a particle budget (S)
`--reduced-motion` and `defaults.reduced_motion` shipped with #20 (settled
states, no transitions, countdown or engine motion). Still open: following
the OS setting where egui exposes it, and a particle budget that scales with
window size and drops on slow GPUs.

### 6.3 Theme as an enum (done)
See 5.3: `is_ember()` and the name comparisons are gone.

### 6.4 Bundled fonts for every theme (M, needs a decision)
Ember and the seasons use the bundled faces; dark, light and nord still use
egui's default sans (any theme can now pick faces with `fonts:`)
(backlog 1.8). Giving them Hanken Grotesk with a real bold changes their look.

### 6.7 Content-aware fields for the remaining visualizations (S each)
Funnel, word cloud and git graph only get the quiet frame today.

### 6.8 A shared illustration library (L)
`mdeck point-cloud contribute` files a prefilled issue and the
`/include-point-clouds` skill folds accepted clouds into the built-in set at
the next release. If contributions pile up, a separate `mdeck-illustrations`
repository that `mdeck point-cloud get <name>` fetches from would make a cloud
usable the day it is merged.

### 6.10 Art engines: what is left after #17 (M)
The art engines shipped in 1.12 to 1.15 (blueprint, sketch, chalkboard,
watercolour, darkroom; `@art`, `mdeck ai art`, `S`, `page:` and `art:` in
themes). Left out of that round, each a decision or a follow-up:

- **Medium-specific slide changes.** The proposal had a page turn (sketch),
  an eraser wipe (chalkboard), sheets swapped (blueprint), a new sheet laid on
  top (watercolour) and the print sliding out (darkroom). Today the old
  picture fades while the new one draws in, under the deck's own transition.
  Doing it properly needs an engine hook into slide transitions (only the
  `board` capability owns them now). Recommendation: add a transition hook
  for engines and do the page turn first.
- **The reveal runs on the CPU** (a time map, one pass per frame into a
  texture, pictures capped at 900 px). A GPU shader would allow full
  resolution pictures; not needed at today's sizes.
- **Line art on LED.** It could trace a slide's line art instead
  of its point cloud. Small once wanted.
- **A hand-lettered heading face** for sketch and chalkboard, as a theme
  option. Needs a bundled face with a suitable licence.

## 7. Documentation and onboarding

- Generate the keyboard-shortcut table for the HUD, README, spec and
  `mdeck spec --short` from one shared table so they cannot drift again (the
  card and HUD now share `app::keys::SHORTCUTS`; README and the spec are still hand-written).
