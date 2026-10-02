use std::time::Instant;

use eframe::egui::{self, Color32, FontId, Pos2};

use crate::parser::Chart;
use crate::theme::Theme;

mod axis;
mod curve;
mod fit;
mod frame;
mod legend;
mod node_text;
mod reveal;
mod ring;
mod values;

pub mod artifact_flow;
pub mod bar_chart;
pub mod donut_chart;
pub mod flower;
pub mod funnel_chart;
pub mod gantt_chart;
pub mod git_graph;
pub mod kpi_cards;
pub mod line_chart;
pub mod org_chart;
pub mod pie_chart;
pub mod progress_bars;
pub mod radar_chart;
pub mod scatter_plot;
pub mod stacked_bar;
pub mod timeline;
pub mod venn_diagram;
pub mod word_cloud;

pub use axis::*;
pub use fit::*;
pub use frame::*;
pub use legend::*;
pub use reveal::*;
pub use ring::*;
pub use values::*;

/// What every chart draws with: where, in which theme, how far revealed.
#[derive(Clone, Copy)]
pub struct VizCtx<'a> {
    pub ui: &'a egui::Ui,
    pub theme: &'a Theme,
    pub opacity: f32,
    pub scale: f32,
    /// Reveal steps shown so far.
    pub reveal_step: usize,
    /// When the latest step was revealed (animates it in); `None` draws it
    /// settled.
    pub reveal_timestamp: Option<Instant>,
}

impl VizCtx<'_> {
    /// The body font at `size` times the body size, scaled.
    pub fn font(&self, size: f32) -> FontId {
        FontId::new(
            self.theme.body_size * size * self.scale,
            self.theme.body_family(),
        )
    }

    /// The foreground colour at `alpha` of the chart's opacity.
    pub fn fg(&self, alpha: f32) -> Color32 {
        Theme::with_opacity(self.theme.foreground, self.opacity * alpha)
    }

    /// Palette colour `i` at the theme's fill opacity.
    pub fn fill(&self, palette: &[Color32], i: usize) -> Color32 {
        Theme::with_opacity(
            palette[i % palette.len()],
            self.opacity * self.theme.fill_opacity(),
        )
    }

    /// Reveal progress (0 to 1) of an item shown at `step`, asking for another
    /// frame while it is still animating.
    pub fn anim(&self, step: usize) -> f32 {
        let (anim, repaint) = reveal_anim_progress(step, self.reveal_step, self.reveal_timestamp);
        if repaint {
            self.ui.ctx().request_repaint();
        }
        anim
    }
}

/// Draw `kind` at `pos` within `max_width` by `max_height` (`0.0`: as tall as
/// it needs). Returns the height used.
pub fn draw(
    kind: Chart,
    content: &str,
    cx: &VizCtx,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    match kind {
        Chart::WordCloud => word_cloud::draw_word_cloud(cx, content, pos, max_width, max_height),
        Chart::Timeline => timeline::draw_timeline(cx, content, pos, max_width, max_height),
        Chart::Pie => pie_chart::draw_pie_chart(cx, content, pos, max_width, max_height),
        Chart::Bar => bar_chart::draw_bar_chart(cx, content, pos, max_width, max_height),
        Chart::Line => line_chart::draw_line_chart(cx, content, pos, max_width, max_height),
        Chart::Donut => donut_chart::draw_donut_chart(cx, content, pos, max_width, max_height),
        Chart::KpiCards => kpi_cards::draw_kpi_cards(cx, content, pos, max_width, max_height),
        Chart::Funnel => funnel_chart::draw_funnel_chart(cx, content, pos, max_width, max_height),
        Chart::Radar => radar_chart::draw_radar_chart(cx, content, pos, max_width, max_height),
        Chart::StackedBar => stacked_bar::draw_stacked_bar(cx, content, pos, max_width, max_height),
        Chart::VennDiagram => {
            venn_diagram::draw_venn_diagram(cx, content, pos, max_width, max_height)
        }
        Chart::ProgressBars => {
            progress_bars::draw_progress_bars(cx, content, pos, max_width, max_height)
        }
        Chart::ScatterPlot => {
            scatter_plot::draw_scatter_plot(cx, content, pos, max_width, max_height)
        }
        Chart::Org => org_chart::draw_org_chart(cx, content, pos, max_width, max_height),
        Chart::Gantt => gantt_chart::draw_gantt_chart(cx, content, pos, max_width, max_height),
        Chart::GitGraph => git_graph::draw_gitgraph(cx, content, pos, max_width, max_height),
        Chart::Flower => flower::draw_flower(cx, content, pos, max_width, max_height),
        Chart::ArtifactFlow => {
            artifact_flow::draw_artifact_flow(cx, content, pos, max_width, max_height)
        }
    }
}

// ─── Standardized visualization design tokens ──────────────────────────────
// All visualizations use these constants for visual consistency within a theme.

// Font size multipliers (of theme.body_size * scale)
pub const VIZ_FONT_GRID_LABEL: f32 = 0.55;
pub const VIZ_FONT_CATEGORY_LABEL: f32 = 0.65;
pub const VIZ_FONT_VALUE_LABEL: f32 = 0.55;
pub const VIZ_FONT_AXIS_LABEL: f32 = 0.65;
pub const VIZ_FONT_LEGEND: f32 = 0.65;
pub const VIZ_FONT_PRIMARY_LABEL: f32 = 0.70;
pub const VIZ_FONT_SECONDARY_LABEL: f32 = 0.55;
pub const VIZ_FONT_TITLE: f32 = 0.75;

// Stroke widths (multiplied by scale)
pub const VIZ_STROKE_AXIS: f32 = 1.5;
pub const VIZ_STROKE_GRID: f32 = 0.5;
pub const VIZ_STROKE_DATA_LINE: f32 = 2.5;
pub const VIZ_STROKE_BORDER: f32 = 1.5;
pub const VIZ_STROKE_CONNECTOR: f32 = 1.5;
pub const VIZ_STROKE_SEPARATOR: f32 = 2.0;

// Corner radii (multiplied by scale)
pub const VIZ_CORNER_BAR: f32 = 4.0;
pub const VIZ_CORNER_CARD: f32 = 12.0;
pub const VIZ_CORNER_NODE: f32 = 8.0;
pub const VIZ_CORNER_TRACK: f32 = 6.0;
pub const VIZ_CORNER_SWATCH: f32 = 3.0;

// Legend swatch size (multiplied by scale)
pub const VIZ_SWATCH_SIZE: f32 = 18.0;

// Dot/point radii (multiplied by scale)
pub const VIZ_DOT_RADIUS: f32 = 4.0;
pub const VIZ_SCATTER_RADIUS: f32 = 8.0;
pub const VIZ_TIMELINE_DOT: f32 = 8.0;

// Opacity multipliers (applied to base opacity)
pub const VIZ_OPACITY_FILL: f32 = 0.85; // default; themes override via Theme::fill_opacity
pub const VIZ_OPACITY_GRID: f32 = 0.08;
pub const VIZ_OPACITY_AXIS: f32 = 0.2;
pub const VIZ_OPACITY_LABEL: f32 = 0.8;
pub const VIZ_OPACITY_GRID_LABEL: f32 = 0.4;
pub const VIZ_OPACITY_SUBTLE_BG: f32 = 0.05;
pub const VIZ_OPACITY_BORDER_RING: f32 = 0.15;

// Animation threshold for showing value labels
pub const VIZ_LABEL_REVEAL_THRESHOLD: f32 = 0.8;

/// Upper bound on grid lines drawn by any chart. Guards the `while grid_val <= max`
/// loops against pathological inputs (huge ranges, tiny steps) so they always terminate.
pub const VIZ_MAX_GRID_LINES: usize = 20;

/// Smallest font size (as a multiple of the body size) that `fit_text` may shrink to.
pub const VIZ_FONT_MIN: f32 = 0.5;

// ─── Shape helpers ──────────────────────────────────────────────────────────

/// Build a filled annular sector (pie slice when `inner_radius` is 0) as a
/// single mesh. Drawing one mesh instead of many thin polygons avoids the
/// anti-aliasing seams that show up as striping inside the slice.
pub fn sector_mesh(
    center: Pos2,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    sweep: f32,
    color: Color32,
) -> egui::Shape {
    use eframe::epaint::{Mesh, Vertex, WHITE_UV};

    let segments = ((sweep.abs() / (2.0 * std::f32::consts::PI)) * 180.0).ceil() as usize;
    let segments = segments.clamp(2, 180);
    let inner_radius = inner_radius.max(0.0);
    let mut mesh = Mesh::default();
    let vertex = |r: f32, a: f32| Vertex {
        pos: Pos2::new(center.x + r * a.cos(), center.y + r * a.sin()),
        uv: WHITE_UV,
        color,
    };
    for i in 0..=segments {
        let a = start_angle + sweep * (i as f32 / segments as f32);
        mesh.vertices.push(vertex(outer_radius, a));
        mesh.vertices.push(vertex(inner_radius, a));
    }
    for i in 0..segments as u32 {
        let o0 = i * 2;
        let i0 = o0 + 1;
        let o1 = o0 + 2;
        let i1 = o0 + 3;
        mesh.add_triangle(o0, o1, i0);
        mesh.add_triangle(i0, o1, i1);
    }
    egui::Shape::mesh(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run `f` with a painter backed by a headless egui context with fonts loaded.
    pub(crate) fn with_test_painter(f: impl FnOnce(&egui::Painter)) {
        let ctx = egui::Context::default();
        let mut f = Some(f);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let painter = ui.ctx().layer_painter(egui::LayerId::background());
            if let Some(f) = f.take() {
                f(&painter);
            }
        });
        // Headless: nobody uploads the font atlas, so discard the deltas explicitly.
        output.textures_delta.clear();
    }

    /// Draw `kind` with each of `contents` in a headless frame at 1920x1080
    /// and at a small size; returns the heights used.
    fn draw_headless(kind: Chart, contents: &[&str]) -> Vec<f32> {
        let ctx = egui::Context::default();
        let mut heights = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let theme = Theme::light();
            for content in contents {
                for (w, h, scale) in [(1700.0, 800.0, 1.0), (400.0, 220.0, 0.25)] {
                    let cx = VizCtx {
                        ui,
                        theme: &theme,
                        opacity: 1.0,
                        scale,
                        reveal_step: 1,
                        reveal_timestamp: None,
                    };
                    heights.push(draw(kind, content, &cx, Pos2::ZERO, w, h));
                }
            }
        });
        output.textures_delta.clear();
        heights
    }

    #[test]
    fn flowers_of_every_shape_draw() {
        let many: String = (0..16)
            .map(|i| format!("- Team {i}: does things\n"))
            .collect();
        let heights = draw_headless(
            Chart::Flower,
            &[
                "",
                "- center Only the centre",
                "- One petal",
                "- A\n- B\n- A -> B: x\n- A -> Nobody",
                "- center C\n+ petal P: a very long description that goes on and on and on and on",
                &many,
            ],
        );
        assert_eq!(heights[..2], [0.0, 0.0], "an empty flower takes no room");
        assert!(heights[2..].iter().all(|&h| h > 0.0));
    }

    #[test]
    fn artifact_flows_of_every_shape_draw() {
        let many: String = (0..12)
            .map(|i| format!("- producer P{i}\n- consumer C{i}\n"))
            .collect();
        let heights = draw_headless(
            Chart::ArtifactFlow,
            &[
                "",
                "- service Only a service\n  - item",
                "- producer A\n- consumer B",
                "- producer A\n- service S\n- consumer B\n+ A -> S: x (icon: package)\n* S -> B\n- B -> A: back\n- A -> A: self",
                "# producers: none\n# consumers: none\n- producer A\n- consumer B\n- A -> B: a label long enough to wrap over several lines in the gap",
                &many,
            ],
        );
        assert_eq!(heights[..2], [0.0, 0.0], "an empty flow takes no room");
        assert!(heights[2..].iter().all(|&h| h > 0.0));
    }

    #[test]
    fn test_sector_mesh_geometry() {
        let shape = sector_mesh(
            Pos2::new(0.0, 0.0),
            0.0,
            10.0,
            0.0,
            std::f32::consts::PI,
            Color32::RED,
        );
        let egui::Shape::Mesh(mesh) = shape else {
            panic!("expected a mesh");
        };
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);
        assert!(
            mesh.vertices
                .iter()
                .all(|v| v.pos.x.abs() <= 10.001 && v.pos.y >= -0.001)
        );
    }
}
