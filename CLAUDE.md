# mdeck

A markdown-based presentation tool.

## Design Principles

- **Visual appeal is paramount.** Every rendered element — text, code, transitions, scroll effects — must look polished and professional. Prefer smooth animations over instant state changes.
- **Simplicity over complexity.** Fewer controls, fewer options, fewer edge cases. The tool should feel effortless to use. When in doubt, leave it out.
- **Any markdown file should be presentable.** Overflow handling, layout inference, and sensible defaults mean users shouldn't need to tailor their markdown to the tool.

## Layout

Rust workspace with one crate, `crates/mdeck` (package and binary `mdeck`). `crates/mdeck/doc/mdeck-spec.md` is the format spec, embedded in the binary via `include_str!` and printed by `mdeck spec`. Visualization design rules and the docs that must stay in sync with visualizations live in `crates/mdeck/src/render/CLAUDE.md`.

## Key Patterns

- **Rendering:** Scale factor `min(w/1920, h/1080)` applied to all pixel sizes for resolution independence
- **Syntax highlighting:** `syntect` with `LazyLock`-cached `SyntaxSet` / `ThemeSet`; theme maps to syntect theme via `Theme::syntect_theme_name()`; highlighted `LayoutJob`s are cached per (code, language, size, theme)
- **PDF export:** `--format pdf` feeds the same rendered canvases to `export::pdf::PdfDoc` (Flate-compressed RGB image per page, 0.5 pt per px, outline entry per slide). `--notes` adds a second pass per slide that draws a portrait notes page in the light theme (`export::notes`), then composites the slide image into it; notes paginate by block, never shrink.
- **PNG export:** eframe (glow renderer) window with `pixels_per_point` forced to 1; the slide is rendered in window-sized tiles via `ViewportCommand::Screenshot` / `Event::Screenshot` and stitched, so output is exactly `--width`×`--height` on any display. The glow renderer is required: wgpu's screenshot readback is asynchronous and never completes in this loop
- **Transitions:** fade, horizontal slide, spatial (directional pan), with smooth easing; animated overview zoom in/out
- **Scroll/overflow:** Per-slide smooth animated scroll with fade gradients; Up/Down keys; `scroll_targets` + lerp for animation. Code blocks first shrink to fit (`layouts::stacked::fit_code`, height and line width, floor `CODE_FIT_FLOOR`); measurement and drawing share the fitted theme so scroll detection agrees.
- **Keyboard:** one shared table in `app/keys.rs` (`SHORTCUTS`, `map_key`) drives key handling, the HUD and `mdeck spec --short`; add new bindings there. Space/N/Right/PageDown/Enter forward, P/Left/PageUp/Backspace back, Up/Down scroll, Home/End, G grid, T transition, Shift+T theme, F fullscreen, M next monitor, H HUD, `.`/B blackout, R debug overlay, Esc×2 / Q×2 / Ctrl+C×2 quit
- **End slide:** Virtual "The End" slide with MDeck logo shown when navigating past the last slide
- **Math:** `$...$` / `$$...$$` parse to `Inline::Math`. `render::math::append` lays the formula out with RaTeX (KaTeX fonts in `fonts/katex/`, one egui family each) and reserves its width in the text job with an invisible placeholder (word joiner + invisible id digits + an end char carrying the width as letter spacing); `render::math::galley`/`galley_tinted` paint a galley and then its formulas on the row's text baseline. Paint any galley that may hold markdown inlines through these helpers, never `painter.galley` directly.
- **Visualization helpers:** shared axis/value helpers live in `render/visualizations/mod.rs` (`nice_grid_step`, `nice_axis_max`, `format_value`, `sector_mesh`, `parse_value`); reuse them instead of re-implementing per chart
- **Diagrams:** Grid layout (when `pos:` specified) or auto-layout; geometric fallback icons; AI-generated icon images from `media/diagram-icons/`; 5 arrow types (`->`, `<-`, `<->`, `--`, `-->`)
- **AI integration:** `ailloy` crate for unified AI access (chat + image generation); config via `~/.config/ailloy/config.yaml`; async via `tokio`
- FPS overlay in the top-right corner while the HUD (`H`) is shown
- **Config precedence:** frontmatter > `~/.config/mdeck/config.yaml` defaults > built-in (theme, transition, start mode)
- **Backdrops and formations:** `scenes::backdrop(seed, share)` fills the dark on non-title slides with one of `Backdrop::{Stars, Dust, Galaxy, Nebula}`, and `scenes::formation_points` places bullet clusters in one of six `Formation`s; both rotate by slide number (`backdrop_for`, `formation_for`) so neighbours differ and exports stay reproducible. The star field uses `Drift::Forward` (outward from a vanishing point, parallax by size, reborn at the centre).
- **Ember field:** `app/ember.rs` owns one `particles::Field`; each frame it picks a scene from the slide (countdown digit, end act, story, hints from renderers, or the inferred layout scene), ticks and paints it under the slide. Renderers publish geometry with `render::hints::push` (a no-op unless Ember is drawing). Export uses the same state with `still = true`.
- **Stories:** `render::story` defines the script schema; `render::story::sidecar` resolves per slide (pinned by number, then by content hash, then stale). Beats extend `max_steps` only under Ember (`slide_max_steps`). The AI prompt and JSON handling live in `commands/story.rs`. Cast `kind`s are illustration names resolved through the `Library`; `FIGURES` in `render::story` (`person`, `hooded`, `man`, `woman`, `thermographer`, `presenter-up`, `presenter-down`) are the people.
- **Illustrations:** `render::illustration` owns the `.mdpc` format (`Cloud`, importance-ordered points, `aspect` = height/width), the lookup order (deck `illustrations/` > `~/.config/mdeck/illustrations/` > `BUILTIN`), and the per-deck `Library` cache the app, export and check pass into `EmberState::frame`. `convert.rs` turns an image into points: composite over black, dilate dots into strokes, threshold, greedy farthest-point ordering weighted toward silhouette edges. `Home::Mask` takes the first *n* points for a group of *n* particles, so glyph masks are shuffled once when built. Placement: `scenes::illustration_stage` (copy slides) and `scenes::illustration_backdrop` (title slides); only layouts `ember::handles` shows show one. Built-ins live in `crates/mdeck/illustrations/`; `build.rs` registers every `.mdpc` there (no source change needed to add one). To promote generated clouds to built-ins use the [`/include-illustrations`](.claude/skills/include-illustrations/SKILL.md) skill; to regenerate, run `mdeck illustration generate` from `crates/mdeck/`.

## Reported issues

GitHub issues are handled with the [`/fix-issue`](.claude/skills/fix-issue/SKILL.md) skill: read and classify the report, reproduce a bug before fixing it, ask the reporter when the report is not reproducible, fix with test, sample and changelog, then thank the reporter in a closing comment that shows before/after screenshots (published to the `issue-screenshots` branch by the skill's `upload-screenshot.sh`) before the issue is closed. Commits reference the issue as `(#n)` and never use closing keywords.

## Releasing

Releases are driven by the [`/release`](.claude/skills/release/SKILL.md) skill (`major`, `minor`, or `patch`): it runs the pre-flight checks, bumps `version` in the root `Cargo.toml`, renames `[Unreleased]` in `CHANGELOG.md` to the dated version, commits `Release vX.Y.Z`, pushes main, and pushes the tag `vX.Y.Z`.

## Code Style

- Building from source on Windows needs NASM and CMake on `PATH` — `aws-lc-rs` (the TLS crypto
  backend pulled in transitively via `ailloy`) compiles optimized assembly routines at build time.
  macOS and Linux need nothing extra. The release workflow's Windows leg installs NASM via
  `ilammy/setup-nasm@v1`; CMake and MSVC are already on the `windows-latest` image.
- **File size guideline:** When a source file exceeds ~500 lines, evaluate whether it would benefit from being split into smaller modules (`mod` in Rust). Look for natural boundaries: distinct type groups, self-contained algorithms, test helpers, or feature areas that could live in their own files. Propose a split plan before refactoring.

## Dependency Policy

We keep this tool's dependencies at their latest compatible versions, not just the versions that
happen to still compile. Staying current is the default, not something we get to eventually —
letting dependencies drift is how technical debt accumulates unnoticed until a security advisory or
a forced breaking upgrade makes it urgent. When a newer major is available and there's no concrete,
documented reason not to take it (see any `# Stays on ...` comments in `Cargo.toml` for the current
exceptions and why), take it during the next maintenance round rather than deferring it. The
cross-repo `maintaining-rust-tools` skill drives this for the whole fleet (ailloy + cosq + deemer +
mdeck + pidge + rigg + rusty-tmpl).

## Quality Requirements

### Testing
- **Always run the full test suite before declaring work complete:** `cargo test --workspace`
- **Always run the full CI check before pushing:** `cargo fmt --all -- --check && cargo clippy --workspace -- -D warnings && cargo test --workspace`
- Write unit tests for all new functionality
- Test edge cases and error paths, not just the happy path
- **Every bug fix must include a regression test.** When fixing a bug, first write a test that reproduces it (fails before the fix, passes after). This prevents the bug from coming back and documents the expected behavior.
- When fixing incorrect tests, explain why the original assertion was wrong before updating it

### Visual Testing
- **Always verify rendering changes visually before declaring work complete.** Use the export command to generate slide PNGs and inspect them:
  ```bash
  cargo run -p mdeck -- export samples/layouts/code.md --output-dir /tmp/slides
  ```
  Then read the exported PNGs to check layout, syntax highlighting, spacing, and overall visual quality.
  To check one slide without rendering the whole deck, add `--slide N` (and `--debug` for its reveal steps).
- Per-feature test decks live in `samples/visualizations/`, `samples/layouts/`, `samples/transitions/`, `samples/features/` and `samples/ember/`; `samples/visualizations/all.md` has every visualization type. When working on a specific visualization type, use its dedicated test file for faster iteration.
- When fixing visual issues, export before and after to confirm the fix.

### Runtime Testing
- **After making changes, run the application and check for runtime incidents.** Launch a relevant test presentation, navigate through slides, then check for errors:
  ```bash
  cargo run -p mdeck -- samples/visualizations/all.md
  ```
  After quitting, if the application reports incidents, read the log file and fix any issues. Incident logs are at `~/Library/Application Support/mdeck/logs/` (macOS) or `~/.config/mdeck/logs/` (Linux).
- Common issues to watch for: `time_jump` false positives (threshold must exceed the repaint heartbeat interval), rendering panics, and layout overflow.

### Documentation & Sample Presentations
- **Before considering any task done, ensure all documentation and sample presentations are up to date.** This is a blocking requirement — incomplete docs or outdated samples mean the task is not finished.
- **Review all documentation for accuracy before pushing or releasing:**
  - `README.md` — features, quick start, badges, gallery preview images
  - `GALLERY.md` — visual showcase with exported slide screenshots from `media/gallery/`
  - `CHANGELOG.md` — new entries for every user-visible change
  - `CLAUDE.md` — architecture, commands, patterns
  - `crates/mdeck/doc/mdeck-spec.md` — format specification (embedded in binary via `mdeck spec`)
- **The format spec (`mdeck-spec.md`) must be updated whenever features are added or changed.** This includes new visualization types, directives, keyboard shortcuts, layouts, or any other user-facing feature. The spec is used by both humans and AI agents to understand how to write presentations.
- **Sample presentations must reflect all features.** When adding a new visualization type, layout, or feature:
  - Add it to `samples/visualizations/all.md` (comprehensive showcase)
  - Create a dedicated file in `samples/visualizations/` or `samples/layouts/`
  - Update `introducing-mdeck.md` if the feature is significant enough for the intro presentation
- When adding new commands, flags, or crates, update all relevant docs in the same commit
- `CHANGELOG.md` must be updated for every release with a dated entry following Keep a Changelog format
