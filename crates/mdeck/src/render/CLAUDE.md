# Rendering and visualizations

Loaded when working under `crates/mdeck/src/render/`. The general design principles are in the root `CLAUDE.md`.

## Visualization Design Principles

All visualizations (charts, diagrams, etc.) must follow these principles:

- **Readability from distance.** This is a presentation tool: audiences may be far from the screen. All text must be large enough to read from the back of a room. Font sizes should be consistent across similar elements in all visualization types.
- **Space and margins.** Use generous padding and margins. Visualizations should fill available space without feeling cramped, but maintain a feeling of breathing room and negative space. Avoid tiny text crammed into corners.
- **Consistent font sizing.** Define a few standard font size categories (labels ~0.65-0.75, values ~0.55-0.65, grid labels ~0.55, legends ~0.65) and reuse them across all visualizations. Never go below 0.5 for any readable text.
- **Visual polish.** Smooth animations, subtle colors, no harsh borders between stacked elements. Prefer transparent overlapping areas (like in Venn/radar) over opaque blocking.
- **Purpose-built over reused.** If a better-looking visualization can be made by implementing it from scratch, build it from scratch. Never stretch an existing visualization to illustrate something it was not designed for (for example, do not bend the architecture diagram's grid into a radial ecosystem or an artifact flow). Shared helpers (axes, colours, font sizes, reveal) are welcome; shared layouts for different kinds of information are not.

## Code structure

- **A new chart:** add a variant to `parser::Chart` and its fence tag to `Chart::TAGS`, arms to `visualizations::draw`, `visualizations::check` and `kind_for_tag`, and a module whose `draw_*` takes `(cx: &VizCtx, content, pos, max_width, max_height) -> f32`. Keep parsing and geometry pure (a `layout` step with unit tests) and paint separately.
- **Shared pieces first:** `grammar` (`Source::parse` for settings, items, attributes and relations; `label_value_items` and friends; report problems with `src.problem`, and give the module a `check`), `PlotFrame` (axes, grid, value labels), the legend helpers, `reveal_anim_progress` / `assign_steps`, and the `VIZ_FONT_*` tokens.
- **No long argument lists:** pass `VizCtx`, `BlockCx` or `TextCx` plus small structs rather than adding `#[allow(clippy::too_many_arguments)]`.

## Keeping docs in sync

- **When adding or changing visualizations, update ALL of these** (they must stay in sync):
  1. `crates/mdeck/doc/mdeck-spec.md`: the format spec (embedded in binary)
  2. `commands/create/prompts.rs` `ANALYSIS_SYSTEM_PROMPT`: the visualization list with syntax hints (so `ai create` uses them)
  3. `crates/mdeck/doc/ai-reference-supplement.md`: the AI agent tips (used by `mdeck ai skill --reference`)
  4. `commands/spec.rs`: the quick reference card (used by `mdeck spec --short`)
  5. `docs/visualizations.md`: the visualization table

  If the AI doesn't know about a visualization, it won't use it: it will log a missing opportunity instead, which is wrong.
