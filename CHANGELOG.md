# Changelog

All notable changes to this project will be documented in this file.

## [0.18.0] - 2026-09-03

### Added

- **Clicker-friendly keys** — PageDown and Enter advance, PageUp and Backspace go back, so presentation remotes work out of the box. `B` is an alias for `.` (blackout). Keys pressed during a transition are queued instead of dropped.
- **Shared shortcut table** — the HUD and `mdeck spec --short` are generated from one table in `app/keys.rs`; the quick reference no longer lists a stale `D` theme key and now includes Home/End, PageUp/PageDown, blackout, and the debug overlay.
- **`mdeck <file> --check -v`** prints one line per slide (layout, block count, reveal steps, title).
- **Config defaults are honoured** — `mdeck config set defaults.theme|transition` now applies when the frontmatter does not set them (frontmatter > config > built-in). `mdeck config show` prints every key, including image and icon styles and the remembered monitor position.
- **Git graph in the gallery** — `samples/gallery.md` and `GALLERY.md` now include the `@gitgraph` visualization.
- **`BACKLOG.md`** — a roadmap of larger ideas and open decisions collected during a full review of the product.
- **Edge-case sample slides** in `samples/layouts/` and `samples/visualizations/` (wrapped titles, long quotes, overflowing lists, wide tables, long labels, star-shaped radar, thousands separators, legend overflow) for visual regression checks.

### Changed

- **Pixel-exact export** — `mdeck export` now produces images of exactly the requested size (1920x1080 by default) on every display. Previously HiDPI screens doubled the output and windows were clamped to the screen, so `--width 3840` could yield neither 3840 nor 1920 pixels. Slides larger than the display are rendered in tiles and stitched. Export also waits for images to finish loading.
- **Images load in the background** — decoding happens on a worker thread, and the next two slides' images are preloaded, so large photos no longer stall a transition.
- **Pie and donut charts** are drawn as single meshes instead of hundreds of thin polygons, removing the visible striping inside slices.
- **Charts pick round axis limits** — bar, line and stacked-bar axes now end on a round number above the data (the tallest bar no longer touches the top of the chart), and axis labels never print `-0`.
- **Word clouds fill the slide** — the layout is scaled up to use the available area instead of floating small in the centre, and no word is drawn below the readable floor (half the body size); words that cannot fit are dropped rather than shrunk to illegibility.
- **KPI cards** are sized to their content with centred text; **Gantt** rows get more room when there are few tasks; **bar charts** use gaps proportional to bar width.
- **Venn diagrams** with three sets overlap properly and place pairwise labels inside their lens instead of on top of each other; labels wrap.
- **Charts accept decorated numbers** — `$4,200`, `12%`, `1_000`, `40 users` all parse; `inf`/`nan` are rejected instead of hanging the renderer. Comma-separated series such as `1,000, 2,000` are read correctly.
- **Labels fit** — category labels, legend entries, progress-bar labels, KPI values and donut centre text shrink to a shared size and truncate with an ellipsis instead of overflowing; crowded line-chart and Gantt axis labels are thinned. Legend entries keep their percentage when truncated.
- **Radar charts** fill concave (star-shaped) series correctly; axis labels are anchored by angle so they stay clear of the rings.
- **Stacked bars** work without a `# categories:` line (numbered 1..n) and round only the top segment.
- **Gantt** dependency arrows point at the right task when an earlier task could not be resolved.
- **Overflow detection is accurate** — bullet, content and two-column slides are measured at the width they are drawn at, with wrapped list items and table rows counted, so long slides scroll instead of being cut off and short two-column slides no longer show a scroll indicator.
- **Wrapped titles and quotes** are centred on their real height; a fill-image heading band grows to fit an H1.
- **Tables** size columns to their content, shrink the font for wide tables, clamp extra cells, and get a subtle header background and zebra rows.
- **Bold text is visible** (rendered in the heading colour); links and inline code follow the theme and fade with transitions.
- **Images** upscale to their reference size so decks look the same at every resolution; `@width:300px` scales with resolution; photos above 4096 px are downscaled and mipmapped so they stay crisp in the grid overview.
- **Syntax highlighting** is cached per code block instead of recomputed every frame, and multi-line constructs (block comments) highlight correctly.
- **Transitions** use cubic easing; heading-to-body spacing is consistent across layouts.
- **README** rewritten with a sharper introduction, a sixty-second start, and complete presenting and command references.
- **Format spec** documents the nord theme, the spatial transition, and marks directives that are accepted but not yet applied (`@background`, `@footer`, `@class`, `@code-theme`, `@aspect`, per-slide `@theme`/`@transition`) as reserved instead of implemented.

### Fixed

- **`Q` no longer quits on a single stray keypress** — it needs a double tap within a second, like Esc and Ctrl+C.
- **Hot reload survives atomic saves** (vim, emacs, JetBrains) on Linux by watching the directory instead of the file's inode; reloading keeps the current slide's reveal state and can no longer panic mid overview animation.
- **Overflowed slides no longer jump to the top before a transition**; the scroll position is reset when the transition completes. Revealing an item below the fold scrolls it into view.
- **Grid overview animation** honours the grid's scroll offset for slides in lower rows.
- **Mouse release outside the window** no longer fires a stray "next slide" or commits a half-drawn stroke.
- **Spurious "time_jump" incidents** after Cmd-Tab or display sleep are gone; a jump is only recorded when an animation was actually in flight.
- **`M` gives feedback** with a toast when the window is not fullscreen instead of silently doing nothing.
- **FPS overlay** is only shown together with the HUD, never to the audience.
- **Export** pads file names to the deck size (three digits from 100 slides) and exits non-zero when a PNG cannot be written.
- **`mdeck ai create -i` and `ai style add -i`** end cleanly on EOF (Ctrl-D or piped input) instead of looping forever; long prompts with non-ASCII text (Swedish, em dashes, emoji) no longer panic; temp files use the platform temp directory (fixes Windows); an explicit `--output` path is respected instead of being replaced by an AI-suggested name; `--style` no longer applies an image style name to icons.
- **Removed a dead retry loop** that printed five bogus "Restarting presentation" messages after a display error.
- **Parser hangs and panics** — a line such as `#hashtag` or `#include <stdio.h>`, or a malformed image like `![alt] text`, made the parser loop forever; a line consisting of a single emoji or accented character panicked; a highlight range such as `{1-99999999999}` allocated unbounded memory. All fixed with regression tests.
- **CRLF files** (Windows line endings) corrupted the frontmatter and leaked the closing `---` into the first slide.
- **Separators inside code blocks** — a `---` line or three blank lines inside a fenced code block no longer splits the slide (`samples/introducing-mdeck.md` renders its "How Slides Work" example on one slide again).
- **Wrapped list items** — continuation lines now stay in their bullet instead of breaking the list into list, paragraph, list.
- **Title slides** — `# Title` directly followed by `## Subtitle` is one title slide, as the spec always said.
- **Ordinary markdown that rendered literally** — setext headings (`Title` over `===`), closing hashes (`## Head ##`), HTML comments, `_italic_`/`__bold__`, `***bold italic***`, backslash escapes, double-backtick code spans, escaped pipes in tables, image titles (`![a](x.png "Title")`) and code info strings with extra words (` ```rust title=x`) are now handled. `5 * 3 * 2` is no longer italicised. A lone `| text |` line is text instead of vanishing.
- **Frontmatter numbers** — `date: 2026` displays as `2026` instead of `Number(2026)`.
- `mdeck config set` accepts `defaults.image_style` and `defaults.icon_style`; incident log files are unique per process and second.
- `mdeck ai create` no longer exits the whole process to show help, and its tests no longer read the real stdin (which could hang in CI).

### Dependencies

- Upgraded eframe/egui 0.33 → 0.36 (glow renderer), ailloy 1.0 → 2.0, colored 2 → 3, inquire 0.7 → 0.9, base64 0.22 → 0.23, pdf-extract 0.10 → 0.12, plus a full `cargo update`. `cargo audit` reports no known vulnerabilities (previously six advisories in lopdf, quick-xml, quinn-proto, webbrowser, crossbeam-epoch). MSRV is now Rust 1.88.

## [0.17.3] - 2026-07-07

### Changed

- Upgraded to ailloy 1.0: Azure OpenAI / Microsoft Foundry requests now use
  the unified `/openai/v1/` surface, models that reject sampling parameters
  are retried automatically, and current default models (gpt-5.4-mini,
  claude-sonnet-5) replace retiring ones.

## [0.17.2] - 2026-05-18

### Fixed

- **AI config parse error** — `mdeck ai` no longer fails with `Failed to parse config from ~/.config/ailloy/config.yaml` when the config contains embedding nodes (`capabilities: [embedding]`) or a `defaults.embedding:` key. Caused by ailloy's embedding capability being absent in the 0.6 line and re-added in 0.7; configs written by newer ailloy CLIs were unreadable.

### Changed

- **Bump `ailloy` 0.6 → 0.8** — adopts embedding support re-introduced in ailloy 0.7 and the auto-detected embedding dimensions in 0.7.3. No behavior changes for mdeck (mdeck does not use embeddings); chat and image APIs are unchanged.
- **Refresh transitive dependencies** via `cargo update`.
- **CI: bump `actions/checkout@v4` → `@v5`** across CI and release workflows; release workflow upload step bumped to `actions/upload-artifact@v5`.
- **Internal: satisfy newer clippy lints** (`collapsible_match`, redundant `.max(0)` on unsigned arithmetic) surfaced by Rust 1.95. No behavior changes.

## [0.17.1] - 2026-03-29

### Changed

- **Improved AI presentation generation quality** — AI-created presentations now use varied layouts (two-column, quotes, section breaks, image splits, tables), include atmospheric images on title and product slides, apply visual rhythm (alternating dense/sparse), and follow presentation archetypes (product comparison, tutorial, pitch, etc.). The interactive chat also asks about visual mood to inform styling.
- **Refactored large source files into modules** — Split `app.rs` (2,574 lines) into `app/` module (drawing, input, helpers), `commands/create.rs` (1,606 lines) into `create/` module (prompts, interactive, extractors, opportunities), and `render/diagram/mod.rs` (3,569 lines) into submodules (types, parsing, layout, edges, icons). No behavior changes.

## [0.17.0] - 2026-03-24

### Added

- **Grid view shows final reveal step** — pressing G now shows each slide fully revealed, making it easy to identify slides by their content. Fixes #5.
- **Move fullscreen to next monitor** — press M to cycle the presentation between monitors. The last used monitor is remembered in config and used on next launch. Fixes #6.

## [0.16.0] - 2026-03-24

### Added

- **Atlassian-style `@gitgraph` visualization** — complete rewrite with new visual model: dotted gray lanes for declared branches, solid colored segments for active branches, proper S-curves for forks (bowing left) and merges (bowing right), vertical lines for simultaneous `*` events, tag boxes with arrows, and pill-shaped merge labels on S-curve midpoints. New syntax: `lane`, `branch A -> B`, `merge A -> B`, `tag`, `commit`.
- **`--debug` flag for export** — `mdeck export --debug` exports every progressive reveal step as a separate PNG (e.g., `slide-01-step-00.png`), enabling systematic visual QA at full resolution.
- **`test-visualization` skill** — reusable testing methodology for visual QA of any mdeck visualization, committed to `.claude/skills/`.

### Changed

- Branch labels now left-aligned at a consistent margin, appearing only when the branch first becomes active.
- S-curves use proper cubic bezier control points with real horizontal distance — forks connect to the target's next event, merges connect from the source's last event.
- Fork endpoint positions are stable across progressive reveal steps (computed from all events, not just visible ones).

### Fixed

- Solid lines now connect fork endpoint dots to subsequent events on the same branch.
- No more double dots at branch/merge events.
- Merge labels positioned on the S-curve midpoint instead of floating.
- Vertical lines for `*` merges draw dots on both source and target lanes.

## [0.15.0] - 2026-03-23

### Added

- **Git graph visualization** (`@gitgraph`) — new visualization type rendering precise git branching diagrams from text. Branches as horizontal lanes, commits as dots, forks and merges as S-curves. Color-coded per branch with pill-shaped labels. Supports Git Flow and any branching strategy. Progressive reveal builds the graph step by step.
- **AI-driven interactive presentation creation** — `mdeck ai create -i` now features a true AI conversation (not fixed questions) that gathers context naturally, suggests a descriptive filename, and shows a confirmation before generating.
- **Visualization opportunity logging** — when AI identifies missing visualization types, detailed GitHub-issue-ready feature requests are logged to `visualization-opportunities.md` with data models, rendering specs, ASCII mockups, and proposed syntax.

### Changed

- **AI create improvements:** true AI chat for interactive mode, animated spinners during generation, no JSON output shown to user, smart filename suggestions, approval step before generation, auto-image generation as part of the pipeline.
- **Speaker notes in AI-generated presentations** are now detailed enough for inexperienced presenters — include core message, talking points, delivery approach, background context, and transitions.
- **AI image generation policy:** only decorative/mood images are generated. Precision diagrams (flowcharts, branch histories) are never AI-generated — visualization opportunities are logged instead.
- Visualization opportunities file appends new entries instead of overwriting, with deduplication by name.

### Fixed

- Unicode arrows (→, ←, ⇒) and symbols (✓, ✗) rendering as □ — AI now avoids these characters.
- `mdeck ai create` without arguments shows help (same as `--help`).
- `mdeck ai create -i` without `--input` prompts for input instead of showing help.
- `[READY]` marker no longer visible in AI chat output.
- `mdeck ai generate` respects quiet flag and shows progress indicators.

## [0.14.0] - 2026-03-22

### Added

- **AI presentation creation** (`mdeck ai create`) — create complete presentations from any content source. Supports text prompts, PDF files, DOCX files, markdown, plain text, and piped stdin input. AI analyzes the content, identifies key points, and generates a structured presentation with speaker notes, visualizations, and image generation markers. Includes interactive mode (`-i`) for guided creation with audience/purpose context, and custom prompt support (`--prompt`) for tailored presentations.
- **Speaker notes** (`???` separator) — add presenter-only notes to any slide. Notes are parsed and stored but never rendered in the presentation. Supports full markdown formatting. Designed to help presenters understand slide intent, especially valuable in AI-generated presentations where notes explain delivery guidance and talking points.
- **Git graph visualization** (`@gitgraph`) — precise, data-driven branch diagrams showing branches as horizontal lanes with commits, forks, and merges. Supports Git Flow and any branching strategy. Progressive reveal builds the graph step by step.

### Changed

- Upgraded ailloy dependency from 0.5 to 0.6.

### Dependencies

- Added `pdf-extract` for PDF text extraction.
- Added `zip` for DOCX text extraction.

## [0.13.0] - 2026-03-20

### Added

- **AI agent skill command** (`mdeck ai skill`) — setup guide and skill file emitter for AI agents like Claude Code. `--emit` outputs a ready-to-save skill file, `--reference` outputs the full format spec and AI reference documentation at runtime.
- **Explicit `mdeck ai status` subcommand** — alias for running `mdeck ai` without arguments.
- **AI reference supplement** (`ai-reference-supplement.md`) — comprehensive CLI and AI image generation reference bundled into the binary for AI agent consumption.

## [0.12.3] - 2026-03-19

### Added

- **Interactive AI config wizard** (`mdeck ai config`) — guided setup for AI providers and models, replacing the previous "open in editor" approach. Powered by ailloy's `config-tui` module.
- **Interactive style creation** (`mdeck ai style add -i`) — AI-assisted style crafting with interactive prompts. `set` is now an alias for `add`.
- **Color-coded edge labels** — architecture diagram edge labels now use the edge's color as background, making it easy to see which label belongs to which connection.

### Changed

- Upgraded ailloy dependency from 0.4 to 0.5 with `config-tui` feature for shared AI status/enable/disable logic.
- AI status, enable, and disable commands now delegate to ailloy's `config_tui` module for consistent behavior across ailloy-based tools.
- Reorganized sample presentations from `sample-presentations/` to `samples/` with subdirectories (`visualizations/`, `layouts/`, `transitions/`).
- Edge label horizontal padding increased for better readability.

## [0.12.2] - 2026-03-11

### Added

- **Nord theme** — an arctic, blue-gray theme inspired by the polar landscape. Calm, muted, and professional. Theme cycling is now dark → light → nord → dark (press `D`).
- **Standardized visualization design tokens** — all 15 visualization types now share centralized constants for font sizes, stroke widths, corner radii, opacities, and swatch sizes, ensuring visual consistency within each theme.
- **Theme-aware trend colors** — KPI cards now use theme-appropriate green/red instead of hardcoded values, ensuring readability across all three themes.

### Changed

- Synchronized font sizes, stroke widths, corner radii, and legend styling across bar charts, stacked bars, line charts, scatter plots, pie/donut charts, radar charts, Venn diagrams, funnel charts, KPI cards, org charts, gantt charts, progress bars, timelines, and word clouds.
- Stacked bar charts now have rounded corners matching regular bar charts.
- Radar chart axis labels reduced from 0.75 to 0.65 for consistency with other visualizations.
- Timeline date/description fonts adjusted for better readability at distance.

## [0.12.1] - 2026-03-11

### Changed

- **Improved README** — rewritten "What is MDeck?" section emphasizing presentation quality, built-in visualizations, and AI-native workflow. Removed minor features from the hero section.
- **Updated gallery images** — refreshed AI-generated visuals in GALLERY.md.

## [0.12.0] - 2026-03-11

### Added

- **AI image generation (`mdeck ai generate`):** Scan a presentation for `![prompt](image-generation)` markers and diagram nodes with `icon: generate-image`, then generate all images in one command. Automatically detects orientation (horizontal for full-slide, vertical for side-panel layouts, square for icons) and rewrites the markdown with actual file paths.
- **Style management (`mdeck ai style`):** Define named image and icon styles in config, set defaults, and override per-presentation via `@image-style` / `@icon-style` frontmatter directives. Hardcoded defaults ensure good results out of the box.
- **Ad-hoc image generation (`mdeck ai generate-image`):** Generate a single image from a prompt with `--prompt`, `--style`, `--output`, and `--icon` flags.
- **Diagram prompt metadata:** Diagram nodes now support `prompt: "..."` in parenthetical metadata for AI icon generation (e.g., `Gateway (icon: generate-image, prompt: "An API gateway")`).
- **Diagram icon aspect ratio preservation:** Non-square icon images are now rendered with correct aspect ratio instead of being stretched.
- **Ungenerated image warning:** Launching a presentation with `image-generation` markers prints a terminal warning suggesting `mdeck ai generate`.
- **Enhanced `mdeck ai test`:** Image generation test now lets you choose between normal image and icon, using the configured default styles.
- **Smart heading-level slide splitting:** Files with a single H1 heading (the common "title + H2 sections" pattern) now automatically split on both H1 and H2 headings. Files with multiple H1s keep the original behavior (only H1 splits). This makes standard markdown files work as presentations without needing explicit `---` separators.
- **`@slide-level` frontmatter directive:** Explicitly control which heading level triggers slide breaks (e.g., `@slide-level: 2` means H1 and H2 both split). Overrides the automatic inference when set.
- **Visual gallery (`GALLERY.md`):** Comprehensive showcase of all layouts, diagrams, and visualizations with exported slide screenshots. Linked from README.
- **Revamped `README.md`:** Restructured with feature overview, visualization table, AI documentation, gallery preview images, and navigation links.

## [0.11.2] - 2026-03-10

### Added

- **Image-aware layouts for Bullet, Code, and Quote slides:** Adding a single image to a bullet, code, or quote slide now renders the content on the left (55%) with the image as a side panel on the right (40%), instead of falling through to the generic Content layout. The Content (fallback) layout also gains the same image-split behavior.

## [0.11.1] - 2026-03-10

### Added

- **Gantt chart visualization (`@gantt`):** Project timelines with tasks, durations, dependencies, and automatic time scaling. Supports absolute dates (`YYYY-MM-DD`), calendar days (`Nd`), working days (`Nwd`), weeks (`Nw`), months (`Nm`), and dependency chains (`after Task`, `after Task + 3d`). Timeline auto-scales between days, weeks, and months based on project span.
- **Gantt weekend shading:** Non-working days (Saturday/Sunday) are shown as subtle gray columns when the timeline is at day-level scale.
- **Gantt labels inside bars (`# labels: inside`):** Option to render task names inside their bars instead of in a left column, giving the full width to the timeline.

### Removed

- **`architecture-diagrams.md`:** Removed redundant standalone diagram documentation. All specifications are now consolidated in `mdeck-spec.md`.

## [0.11.0] - 2026-03-10

### Added

- **Ten new visualization types:** Donut chart (`@donutchart`), line chart (`@linechart`), scatter plot (`@scatter`), stacked bar (`@stackedbar`), funnel chart (`@funnel`), KPI cards (`@kpi`), progress bars (`@progress`), radar chart (`@radar`), Venn diagram (`@venn`), org chart (`@orgchart`)
- **Chart axis labels:** `# x-label:` and `# y-label:` directives for bar chart, line chart, scatter plot, and stacked bar
- **Word cloud improvements:** Elliptical cloud shape, non-linear font size contrast (`t^1.5`), rotation restricted to smallest words only
- **Format specification command:** `mdeck spec` prints the full format spec, `mdeck spec --short` prints a quick reference card
- **Per-visualization test files:** Individual test presentations for each visualization type
- **MDeck intro presentation:** `introducing-mdeck.md` — a real presentation about MDeck itself

### Changed

- **Reorganized sample presentations:** Removed redundant files, added comprehensive `test-all-visualizations.md`

## [0.10.0] - 2026-03-10

### Added

- **Four new visualization types:** Word cloud (`@wordcloud`), timeline (`@timeline`), pie chart (`@piechart`), and bar chart (`@barchart`) — all using the same code-block DSL as diagrams with `@` language tags
- **Reveal step support for visualizations:** All new visualization types support `-` (static), `+` (next step), and `*` (with previous) reveal markers for progressive disclosure
- **Bar and pie chart reveal animations:** Bars grow from zero height/width and pie slices sweep from zero angle when revealed, with smooth ease-in-out easing over 0.4 seconds
- **Mixed content slides:** Visualization layout supports heading + text blocks + visualization on the same slide
- **Bar chart orientations:** Vertical (default) and horizontal via `# orientation: horizontal` directive
- **Bar chart grid labels:** Nice-number algorithm for clean axis labels (20, 40, 60 instead of 23.3, 46.7)
- **Word cloud layout:** Dense spiral placement with area-proportional font sizing, cached for stable positions across frames
- Sample presentation `test-visualizations.md` covering all visualization types

### Changed

- **Larger fonts across all visualizations and diagrams** for better readability in presentation settings: diagram node labels (0.55x → 0.8x), diagram edge labels (0.45x → 0.65x), timeline dates (0.55x → 0.85x), timeline descriptions (0.45x → 0.7x), pie chart legend (0.45x → 0.65x), bar chart labels (0.4x → 0.6x)

## [0.9.1] - 2026-03-10

### Fixed

- **Diagram reveal ordering:** Interleaved nodes and edges now reveal in file order instead of all nodes first then all edges. This fixes diagrams like "Pipeline Growth" where `+ Source -> Build` should appear between `+ Build` and `+ Test`, not after all nodes.
- **False time-jump warnings on Linux:** Raised the time-jump detection threshold from 200ms to 2000ms. The Linux repaint keepalive (500ms) was triggering spurious "power-state gap" incidents every frame cycle, flooding the incident log.

## [0.9.0] - 2026-03-06

### Changed

- **AI integration rewrite:** Migrated from custom AI provider system (direct OpenAI/Gemini API calls via `ureq`) to the [`ailloy`](https://github.com/mklab-se/ailloy) crate for unified AI access with async support
- New AI subcommands: `ai test`, `ai enable`, `ai disable`, `ai config` replace the old `ai init`, `ai status`, `ai remove`
- `ai` (no subcommand) now shows status directly
- `ai test` supports interactive testing of both chat completion and image generation with inline terminal image display (iTerm2, Kitty)
- `ai config` opens the ailloy configuration file in your editor

### Removed

- `generate-icons` command (AI icon generation now handled via ailloy)
- Custom `AiConfig`, `AiProvider`, and `ImageGenProvider` types from config (replaced by ailloy's config system)
- `ureq` and `serde_json` dependencies (replaced by `ailloy` and `tokio`)

## [0.8.1] - 2026-03-04

### Added

- **Power-state resilience (Linux):** More aggressive repaint keepalive (500ms vs 4s) prevents GPU context instability when presenting on battery or while screen-sharing
- **Time-jump detection:** Frame deltas >200ms are detected and all in-flight animation timestamps (transitions, overview zoom, pen strokes, arrows, toasts, reveal steps) are shifted forward so animations resume smoothly instead of snapping to completion
- Time-jump incidents are logged to the incident log for diagnostics
- Incident log header now includes `XDG_CURRENT_DESKTOP` and `DESKTOP_SESSION` environment variables for better desktop environment diagnostics

## [0.8.0] - 2026-03-03

### Added

- **Incident logging:** Lightweight `IncidentLog` module records all recovered and fatal errors (display errors, file watcher errors, reload failures) to `~/.config/mdeck/logs/incident-YYYY-MM-DD-HHMMSS.log` for diagnostics
- Log files are created lazily — no file is written during normal operation
- At session end, if any incidents occurred, the log file path is printed to stderr
- Log header includes version, presentation file, OS/arch, and display-related environment variables (DISPLAY, WAYLAND_DISPLAY, XDG_SESSION_TYPE) for Linux debugging
- File watcher errors are now logged (previously silently ignored)
- File reload errors are now logged in addition to the existing toast notification

## [0.7.1] - 2026-03-02

### Removed

- Debug frame profiling that wrote `/tmp/mdeck-profile.log` on every exit

## [0.7.0] - 2026-03-02

### Added

- **"The End" slide:** Virtual end slide shown when navigating past the last slide, with centered "The End" title and MDeck logo/attribution in the bottom-right corner
- **Blackout mode:** Press `.` (period) to toggle screen to solid black for audience attention; press `.` again to resume
- **`--check` CLI flag:** Validate presentations without launching the GUI — reports diagram routing warnings with exit code 1 on problems, 0 on success
- Structured warning system (`CheckReport`, `CheckWarning`, `CheckCategory`) for extensible presentation validation
- Diagram route warnings collected once during background precache instead of per-frame `eprintln!` spam
- Brief one-liner warning summary printed to stderr in GUI mode when routing issues are found

### Changed

- Replaced `precache_all_diagrams_background` with `precache_all_diagrams_with_report` that returns a `CheckReport` via channel
- Removed noisy per-frame `eprintln!("ROUTE WARNING: ...")` from `draw_diagram_sized`; fallback drawing logic preserved
- HUD (press H) now shows `.` blackout shortcut

## [0.6.0] - 2026-03-02

### Added

- Background pre-caching of diagram routes: all diagrams are pre-computed on a background thread at startup and after file reload, making transitions to diagram slides instant
- Diagram scale-to-fit: large diagrams (3+ rows) that overflow the slide area are automatically scaled down to fit
- `# scale:` directive in diagram blocks: `fit` (default), `scroll`, or a numeric factor (e.g. `0.7`)

### Changed

- Diagram route cache upgraded from thread-local `RefCell` to global `Mutex`, enabling cross-thread cache sharing between background precache and render threads
- Removed per-frame adjacent-slide precaching in favor of whole-presentation background precaching

## [0.5.0] - 2026-03-02

### Added

- Live file watching: presentation auto-reloads when the markdown file is saved, with slide position preservation
- Configurable routing cost weights (`routing.length`, `routing.turn`, `routing.lane_change`, `routing.crossing`) in config
- Crossing-aware edge routing: A* search now penalizes routes that cross existing edges
- Crossing detection at junctions and empty cell centers for perpendicular and pass-through segments
- Turn-conflict detection for lanes adjacent to turning routes
- 37 new unit tests for crossing avoidance, routing weights, and file watcher

### Changed

- Edge routing engine uses weighted cost function (length + turns + lane changes + crossings) instead of simple path length

## [0.4.0] - 2026-03-02

### Added

- Diagram rendering overhaul: proper grid layout, auto-layout, much larger nodes
- Diagram parser: skip comment lines, parse `icon:` and `pos:` metadata, detect all 5 arrow types (`->`, `<-`, `<->`, `--`, `-->`)
- Geometric fallback icons for 15+ node types (user, server, database, cloud, lock, api, cache, etc.)
- AI-generated diagram icons via `mdeck generate-icons <file.md>` command
- Icon images loaded from `media/diagram-icons/{name}.png` when available
- OpenAI DALL-E 3 and Google Gemini Imagen API support for icon generation
- `image_generation` config section for API provider and key
- Orthogonal edge routing engine with A* pathfinding and lane allocation
- Edge rendering with rounded corners, proper arrowheads, and lane-aligned connections
- Dashed lines for `--` and `-->` arrow types
- Edge label pills with semi-transparent backgrounds
- Diagram debug overlay (press R) showing routing details
- Gallery layout for image-heavy slides
- 244 unit tests covering parsing, routing, and rendering

### Changed

- Diagram nodes now render as rounded rectangles with icon + label (was: tiny pills in a single row)
- Diagram layout uses grid positioning or auto-layout (was: single horizontal row)

### Fixed

- Arrow port offsets now derived from lane assignments, eliminating diagonal "lane-switching" segments
- Entry face computation corrected with `.opposite()` to match routing direction
- Edge labels moved to 20% along polyline to prevent overlap on opposing edges (A->B and B->A)
- Debug overlay route format now shows lane labels between coordinates per routing spec

## [0.3.0] - 2026-02-28

### Changed

- Renamed project from `presemd` to `mdeck` across the entire codebase
- Binary name changed from `presemd` to `mdeck`
- Config directory changed from `~/.config/presemd/` to `~/.config/mdeck/`
- Crate name changed from `presemd` to `mdeck` on crates.io
- Homebrew formula changed from `presemd` to `mdeck`
- Repository URL changed from `mklab-se/presemd` to `mklab-se/mdeck`

## [0.2.0] - 2026-02-28

### Added

- Full CLI with clap: `mdeck <file.md>` to launch presentations
- Subcommands: `ai init/status/remove`, `config show/set`, `completion`, `export`, `spec`, `version`
- Shell completions for bash, zsh, fish, and powershell (static and dynamic)
- AI provider configuration with auto-detection (Claude, Codex, Copilot, Ollama)
- YAML-based configuration at `~/.config/mdeck/config.yaml`
- Configurable defaults: theme, transition, aspect ratio, start mode
- Global flags: `--verbose`, `--quiet`, `--no-color`, `--windowed`
- `--slide <N>` flag to start on a specific slide (1-indexed)
- `--overview` flag to start in grid overview mode
- `defaults.start_mode` config setting (`first`, `overview`, or slide number)
- Grid overview: mouse hover highlight, click to select slide, mouse wheel scrolling
- Grid overview: fade gradients at screen edges when content overflows
- Grid overview: presentation title shown instead of generic "Slide Overview"
- Freehand pen annotations (left-drag) with outline/glow effect
- Arrow annotations (right-drag) with large arrowhead and drop shadow
- Distinct colors: pen strokes in cyan/blue, arrows in yellow-orange/red
- ESC clears drawings on current slide before double-tap-to-quit
- Mouse input: left-click forward, right-click backward, scroll wheel for content
- PNG export via `mdeck export` with configurable resolution
- Format specification via `mdeck spec` (full and `--short` quick reference)
- Sample presentations for testing (`samples/`)

## [0.1.1] - 2026-02-28

### Added

- Initial implementation with hardcoded demo slides
- Slide transitions: fade and horizontal slide with easing
- Keyboard navigation with arrow keys
- FPS overlay
- `--version` flag support
