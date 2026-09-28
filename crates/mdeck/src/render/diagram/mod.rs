//! Architecture diagrams: nodes on a grid joined by orthogonally routed edges.
//!
//! A frame runs in passes: parse (`parsing`), assign reveal steps (`reveal`),
//! lay out the nodes (`scene`, `layout`), draw the nodes (`nodes`), then
//! route and draw the edges (`routes`, `ports`, `polyline`, `edges`).

mod debug;
mod edges;
mod icons;
mod layout;
mod metadata;
mod nodes;
mod parsing;
mod polyline;
mod ports;
mod reveal;
mod routes;
pub mod routing;
mod scene;
mod types;

#[cfg(test)]
mod tests;

pub use debug::diagram_debug_info;
pub use reveal::count_diagram_steps;
pub use routes::{check_diagram_routes, clear_route_cache, precache_all_diagrams_with_report};

use std::time::Instant;

use eframe::egui::{self, FontId, Pos2};

use crate::render::image_cache::ImageCache;
use crate::theme::Theme;
use parsing::parse_diagram;
use scene::Scene;

// ─── Diagram renderer ────────────────────────────────────────────────────────

/// What every drawing pass of one diagram shares.
struct DiagramCx<'a> {
    ui: &'a egui::Ui,
    painter: &'a egui::Painter,
    theme: &'a Theme,
    image_cache: &'a ImageCache,
    opacity: f32,
    scale: f32,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
}

/// Draw a diagram parsed from `- Node: label` and `- A -> B: label` lines.
/// `max_height` controls the vertical space; pass 0 for a default. Returns
/// the height used.
#[allow(clippy::too_many_arguments)]
pub fn draw_diagram_sized(
    ui: &egui::Ui,
    content: &str,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
    opacity: f32,
    image_cache: &ImageCache,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
    scale: f32,
) -> f32 {
    let (nodes, edges, scale_directive) = parse_diagram(content);
    let (node_steps, edge_steps) = reveal::reveal_steps(&nodes, &edges);
    let cx = DiagramCx {
        ui,
        painter: ui.painter(),
        theme,
        image_cache,
        opacity,
        scale,
        reveal_step,
        reveal_timestamp,
    };

    if nodes.is_empty() {
        return draw_placeholder(&cx, pos, max_width);
    }

    // Use the provided max_height, or default to 500px
    let diagram_height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };
    let padding = 30.0 * scale;
    let area_size = egui::vec2(max_width - padding * 2.0, diagram_height - padding * 2.0);
    let origin = Pos2::new(pos.x + padding, pos.y + padding);
    let scene = Scene::layout(&nodes, scale_directive, origin, area_size, scale);

    nodes::draw_nodes(&cx, &nodes, &scene, &node_steps);
    let growing = edges::draw_edges(&cx, &nodes, &edges, &edge_steps, &scene);

    // Request repaint while edges are still animating
    if growing {
        ui.ctx().request_repaint();
    }

    diagram_height
}

/// Fallback for unparseable diagrams: a labelled panel.
fn draw_placeholder(cx: &DiagramCx, pos: Pos2, max_width: f32) -> f32 {
    let theme = cx.theme;
    let color = Theme::with_opacity(theme.foreground, cx.opacity * 0.6);
    let bg = Theme::with_opacity(theme.code_background, cx.opacity);
    let height = 200.0 * cx.scale;
    let rect = egui::Rect::from_min_size(pos, egui::vec2(max_width, height));
    cx.painter.rect_filled(rect, 8.0 * cx.scale, bg);
    let galley = cx.painter.layout(
        "[Diagram]".to_string(),
        FontId::new(theme.body_size * 0.8 * cx.scale, theme.body_family()),
        color,
        max_width,
    );
    let text_pos = Pos2::new(
        pos.x + (max_width - galley.rect.width()) / 2.0,
        pos.y + (height - galley.rect.height()) / 2.0,
    );
    cx.painter.galley(text_pos, galley, color);
    height
}
