//! The built-in visuals as registry entries (EXT-06, EXT-21): each one is a
//! [`Visual`] registered under its fence tag, and every fence is drawn,
//! checked and stepped through the registry, so an extension's visual is
//! handled exactly like a built-in one.
//!
//! Deferral (2.0): the built-ins still draw with egui. Their [`Visual::draw`]
//! reaches the slide's [`VizCtx`] through [`with_viz`] instead of drawing
//! with the SDK painter; moving them onto `mdeck_sdk::paint` is a 2.x task.

use std::cell::Cell;

use eframe::egui::{self, Pos2};
use mdeck_sdk::host as h;
use mdeck_sdk::paint::Rect;
use mdeck_sdk::problem::Problem as SdkProblem;
use mdeck_sdk::registry::{Registry, RegistryError};
use mdeck_sdk::visual::{Visual, VisualCx};

use super::{VizCtx, grammar::Problem};

type Draw = fn(&VizCtx, &str, Pos2, f32, f32) -> f32;
type Check = fn(&str) -> Vec<Problem>;

/// A built-in visual: its tag, summary and the functions that do the work.
struct Builtin {
    tag: &'static str,
    summary: &'static str,
    draw: Draw,
    check: Check,
    steps: fn(&str) -> usize,
}

impl Visual for Builtin {
    fn tag(&self) -> &str {
        self.tag
    }

    fn summary(&self) -> &str {
        self.summary
    }

    fn check(&self, src: &str) -> Vec<SdkProblem> {
        (self.check)(src)
            .into_iter()
            .map(|p| SdkProblem::new("visual", p.message).at(p.offset + 1))
            .collect()
    }

    fn steps(&self, src: &str) -> usize {
        (self.steps)(src)
    }

    fn draw(&self, _cx: &mut VisualCx, src: &str, rect: Rect, _step: usize) -> f32 {
        current(|viz| {
            (self.draw)(
                viz,
                src,
                h::egui_pos(rect.min),
                rect.width(),
                rect.height(),
            )
        })
        .unwrap_or(0.0)
    }
}

thread_local! {
    /// The context of the visual being drawn (see [`with_viz`]).
    static CURRENT: Cell<*const ()> = const { Cell::new(std::ptr::null()) };
}

/// Run `f` with `viz` as the context built-in visuals draw with.
fn with_viz<R>(viz: &VizCtx, f: impl FnOnce() -> R) -> R {
    struct Reset(*const ());
    impl Drop for Reset {
        fn drop(&mut self) {
            CURRENT.with(|c| c.set(self.0));
        }
    }
    let previous = CURRENT.with(|c| c.replace(viz as *const VizCtx as *const ()));
    let _reset = Reset(previous);
    f()
}

/// The context [`with_viz`] set, if a draw is running on this thread.
fn current<R>(f: impl FnOnce(&VizCtx) -> R) -> Option<R> {
    let ptr = CURRENT.with(Cell::get);
    if ptr.is_null() {
        return None;
    }
    // SAFETY: `with_viz` stores a pointer to a `VizCtx` that outlives the
    // closure it runs, and restores the previous value (null outside any
    // draw) when that closure returns or unwinds, so a non-null pointer
    // always points at a live context for the duration of this call.
    let viz = unsafe { &*(ptr as *const VizCtx) };
    Some(f(viz))
}

/// Draw the visual `tag` names from `content` at `pos`, within `max_width`
/// by `max_height` (0: as tall as it needs; an extension visual then gets
/// 500 px), through the registry. Returns the height used, 0 when no visual
/// has the tag.
pub fn draw_tag(
    tag: &str,
    content: &str,
    viz: &VizCtx,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let Some(visual) = crate::registry::get().visual_for(tag) else {
        return 0.0;
    };
    // Built-ins read a height of 0 as "as tall as needed"; an extension
    // gets a box of its own.
    let height = if max_height > 0.0 || BUILTIN_TAGS.contains(&tag) {
        max_height
    } else {
        500.0 * viz.scale
    };
    let rect = h::rect(egui::Rect::from_min_size(
        pos,
        egui::vec2(max_width, height),
    ));
    let painter = h::painter(viz.ui.painter().clone(), h::Backend::Glow);
    let tokens = crate::engines::host::convert::tokens(viz.theme);
    let mut cx = h::visual_cx(
        painter,
        &tokens,
        viz.scale,
        viz.reveal_timestamp.is_some(),
        None,
        None,
    );
    let used = with_viz(viz, || visual.draw(&mut cx, content, rect, viz.reveal_step));
    for hint in h::take_visual_hints(&mut cx) {
        if let Some(hint) = egui_hint(hint) {
            crate::render::hints::push(viz.ui.ctx(), hint);
        }
    }
    used
}

/// An extension visual's published geometry as the renderers' own hints.
fn egui_hint(hint: mdeck_sdk::geometry::Hint) -> Option<crate::render::hints::Hint> {
    use crate::render::hints::Hint;
    use mdeck_sdk::geometry::Hint as S;
    Some(match hint {
        S::Bar(r) => Hint::Bar(h::egui_rect(r)),
        S::Frame(r) => Hint::Frame(h::egui_rect(r)),
        S::Path(pts) => Hint::Path(pts.into_iter().map(h::egui_pos).collect()),
        S::Circle { center, radius } => Hint::Circle {
            center: h::egui_pos(center),
            radius,
        },
        S::Point(p) => Hint::Point(h::egui_pos(p)),
        S::Text { .. } => return None,
    })
}

/// The tags of the built-in visuals.
const BUILTIN_TAGS: [&str; 20] = [
    "bar", "line", "pie", "donut", "scatter", "stackedbar", "funnel", "radar", "progress",
    "kpi", "wordcloud", "timeline", "gantt", "orgchart", "gitgraph", "flower", "artifactflow",
    "venn", "architecture", "thermal",
];

fn grammar_steps(src: &str) -> usize {
    super::count_viz_steps(src)
}

/// Register every built-in visual.
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    use super::*;
    let grammar = |tag, summary, draw: Draw, check: Check| Builtin {
        tag,
        summary,
        draw,
        check,
        steps: grammar_steps,
    };
    let all = [
        grammar("bar", "Bar chart", bar_chart::draw_bar_chart, bar_chart::check),
        grammar("line", "Line chart", line_chart::draw_line_chart, line_chart::check),
        grammar("pie", "Pie chart", pie_chart::draw_pie_chart, pie_chart::check),
        grammar("donut", "Donut chart", donut_chart::draw_donut_chart, donut_chart::check),
        grammar("scatter", "Scatter plot", scatter_plot::draw_scatter_plot, scatter_plot::check),
        grammar("stackedbar", "Stacked bar chart", stacked_bar::draw_stacked_bar, stacked_bar::check),
        grammar("funnel", "Funnel chart", funnel_chart::draw_funnel_chart, funnel_chart::check),
        grammar("radar", "Radar chart", radar_chart::draw_radar_chart, radar_chart::check),
        grammar("progress", "Progress bars", progress_bars::draw_progress_bars, progress_bars::check),
        grammar("kpi", "KPI cards", kpi_cards::draw_kpi_cards, kpi_cards::check),
        grammar("wordcloud", "Word cloud", word_cloud::draw_word_cloud, word_cloud::check),
        grammar("timeline", "Timeline", timeline::draw_timeline, timeline::check),
        grammar("gantt", "Gantt chart", gantt_chart::draw_gantt_chart, gantt_chart::check),
        grammar("orgchart", "Org chart", org_chart::draw_org_chart, org_chart::check),
        grammar("gitgraph", "Git branch graph", git_graph::draw_gitgraph, git_graph::check),
        grammar(
            "flower",
            "A platform in the middle and the teams around it",
            flower::draw_flower,
            flower::check,
        ),
        grammar(
            "artifactflow",
            "Artifacts from producers through services to consumers",
            artifact_flow::draw_artifact_flow,
            artifact_flow::check,
        ),
        grammar("venn", "Venn diagram", venn_diagram::draw_venn_diagram, venn_diagram::check),
        Builtin {
            tag: "architecture",
            summary: "Architecture diagram",
            // drawn by `render::diagram`, which needs the deck's icons
            draw: |_, _, _, _, _| 0.0,
            check: crate::render::diagram::check,
            steps: crate::render::diagram::count_diagram_steps,
        },
        Builtin {
            tag: "thermal",
            summary: "Thermal image with lens, reveals and spots",
            // drawn by `render::thermal`, which needs the deck's images
            draw: |_, _, _, _, _| 0.0,
            check: |src| crate::render::thermal::Spec::parse(src).problems,
            steps: crate::render::thermal::default_steps,
        },
    ];
    for visual in all {
        r.visual(Box::new(visual))?;
    }
    Ok(())
}
