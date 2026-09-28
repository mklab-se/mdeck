use eframe::egui::{self, Color32, FontId, Pos2, Stroke};

use super::DiagramCx;
use super::polyline::*;
use super::ports::{PortClaims, direct_waypoints, route_ports, waypoints_to_pixels};
use super::reveal::edge_progress;
use super::routes::{RoutingInput, configured_weights, is_routable};
use super::routing::types::RouteResult;
use super::scene::Scene;
use super::types::*;
use crate::render::hints;
use crate::theme::Theme;

// ─── Edges ───────────────────────────────────────────────────────────────────

/// How one edge is drawn.
struct EdgeStyle<'a> {
    color: Color32,
    label_bg: Color32,
    label_text: Color32,
    label_font: &'a FontId,
    metrics: &'a EdgeMetrics,
}

/// Route and draw every edge revealed by the current step. Returns whether an
/// edge is still growing in, so the caller keeps repainting.
pub(super) fn draw_edges(
    cx: &DiagramCx,
    nodes: &[DiagramNode],
    edges: &[DiagramEdge],
    steps: &[usize],
    scene: &Scene,
) -> bool {
    let metrics = EdgeMetrics::new(cx.scale);
    let palette = cx.theme.edge_palette();
    let label_text = Theme::with_opacity(cx.theme.foreground, cx.opacity * 0.8);
    let label_font = FontId::new(cx.theme.body_size * 0.65 * cx.scale, cx.theme.body_family());
    let step_of = |i: usize| steps.get(i).copied().unwrap_or(0);

    let visible: Vec<(usize, &DiagramEdge)> = edges
        .iter()
        .enumerate()
        .filter(|&(i, edge)| step_of(i) <= cx.reveal_step && is_routable(edge, &scene.rects))
        .collect();
    let routes = RoutingInput::new(
        nodes,
        visible.iter().map(|&(_, edge)| edge),
        &scene.grid,
        &scene.rects,
        metrics.lane_spacing,
        configured_weights(),
    )
    .route();

    let mut claims = PortClaims::new(metrics.port_spacing);
    let mut growing = false;
    for (&(edge_idx, edge), (_, result)) in visible.iter().zip(&routes.results) {
        let from_rect = &scene.rects[&edge.from];
        let to_rect = &scene.rects[&edge.to];

        // Each edge gets a distinct color from the palette
        let base_color = palette[edge_idx % palette.len()];
        let alpha = if edge.arrow.is_dashed() { 0.55 } else { 0.85 };
        let color = Theme::with_opacity(base_color, cx.opacity * alpha);

        let (progress, still_growing) =
            edge_progress(step_of(edge_idx), cx.reveal_step, cx.reveal_timestamp);
        growing |= still_growing;

        let points = match result {
            RouteResult::Success(route) => waypoints_to_pixels(
                route,
                &scene.grid,
                (from_rect, to_rect),
                route_ports(route, metrics.lane_spacing),
                &metrics,
            ),
            // Fallback: direct connection
            RouteResult::Failure { .. } => {
                direct_waypoints(edge, from_rect, to_rect, &mut claims, metrics.node_margin)
            }
        };

        hints::push(cx.ui.ctx(), hints::Hint::Path(points.clone()));
        let style = EdgeStyle {
            color,
            // The edge color as label background, so labels visually match their edge
            label_bg: Theme::with_opacity(color, cx.opacity * 0.80),
            label_text,
            label_font: &label_font,
            metrics: &metrics,
        };
        draw_routed_edge(cx.painter, &points, edge, &style, progress);
    }
    growing
}

/// Draw a routed edge with rounded corners, arrowheads, and optional label.
/// `anim_progress` below 1.0 draws the edge grown that far along its length.
fn draw_routed_edge(
    painter: &egui::Painter,
    waypoints: &[Pos2],
    edge: &DiagramEdge,
    style: &EdgeStyle,
    anim_progress: f32,
) {
    if waypoints.len() < 2 {
        return;
    }

    let smooth_points = apply_rounded_corners(waypoints, style.metrics.corner_radius);
    let total_len = polyline_length(&smooth_points);
    let effective_len = total_len * anim_progress;

    draw_edge_line(painter, &smooth_points, edge.arrow, effective_len, style);
    draw_arrowheads(
        painter,
        (waypoints, &smooth_points),
        edge.arrow,
        (effective_len, anim_progress),
        style,
    );

    // Edge label only when animation is complete
    if !edge.label.is_empty() && anim_progress >= 1.0 {
        draw_edge_label(painter, &smooth_points, total_len, &edge.label, style);
    }
}

/// The line itself, solid or dashed, stopping short of the arrowheads. While
/// animating, the end arrowhead rides on the growing tip, so the line stops
/// one arrow length before it.
fn draw_edge_line(
    painter: &egui::Painter,
    smooth_points: &[Pos2],
    arrow: ArrowKind,
    effective_len: f32,
    style: &EdgeStyle,
) {
    let m = style.metrics;
    let draw_start_d = if arrow.has_start_arrow() {
        m.arrow_size
    } else {
        0.0
    };
    let draw_end_d = if arrow.has_end_arrow() {
        (effective_len - m.arrow_size).max(0.0)
    } else {
        effective_len
    };
    if draw_end_d <= draw_start_d + 1.0 {
        return;
    }

    let draw_points = clip_polyline(smooth_points, draw_start_d, draw_end_d);
    if draw_points.len() < 2 {
        return;
    }
    let stroke = Stroke::new(m.line_width, style.color);
    if arrow.is_dashed() {
        for dash in dash_segments(&draw_points, m.dash_len, m.gap_len) {
            painter.line_segment(dash, stroke);
        }
    } else {
        painter.add(egui::Shape::line(draw_points, stroke));
    }
}

/// Arrowheads: at the animated tip while growing, snapped to the true end once
/// complete.
fn draw_arrowheads(
    painter: &egui::Painter,
    (waypoints, smooth_points): (&[Pos2], &[Pos2]),
    arrow: ArrowKind,
    (effective_len, anim_progress): (f32, f32),
    style: &EdgeStyle,
) {
    let size = style.metrics.arrow_size;
    let head = |(tip, direction): (Pos2, egui::Vec2)| {
        draw_arrowhead(painter, tip, direction, size, style.color);
    };

    if arrow.has_end_arrow() && anim_progress < 1.0 {
        if effective_len > size * 1.2 {
            let tip = polyline_point_at_distance(smooth_points, effective_len);
            let tail =
                polyline_point_at_distance(smooth_points, (effective_len - size * 0.5).max(0.0));
            head((tip, tip - tail));
        }
    } else if arrow.has_end_arrow() {
        let n = waypoints.len();
        let before = n.checked_sub(3).map(|i| waypoints[i]);
        head(arrowhead_at(
            waypoints[n - 1],
            waypoints[n - 2],
            before,
            size,
        ));
    }

    if arrow.has_start_arrow() && effective_len > size * 1.2 {
        let before = waypoints.get(2).copied();
        head(arrowhead_at(waypoints[0], waypoints[1], before, size));
    }
}

fn draw_arrowhead(
    painter: &egui::Painter,
    tip: Pos2,
    direction: egui::Vec2,
    size: f32,
    color: Color32,
) {
    let d = direction.normalized();
    let p = egui::vec2(-d.y, d.x);
    let p1 = tip - d * size + p * size * 0.4;
    let p2 = tip - d * size - p * size * 0.4;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, p1, p2],
        color,
        Stroke::NONE,
    ));
}

/// A pill-shaped label a fifth of the way along the edge.
fn draw_edge_label(
    painter: &egui::Painter,
    smooth_points: &[Pos2],
    total_len: f32,
    label: &str,
    style: &EdgeStyle,
) {
    let m = style.metrics;
    let mid = polyline_point_at_distance(smooth_points, total_len * 0.20);
    let galley = painter.layout_no_wrap(
        label.to_string(),
        style.label_font.clone(),
        style.label_text,
    );
    let label_w = galley.rect.width() + m.label_pad_h * 2.0;
    let label_h = galley.rect.height() + m.label_pad_v * 2.0;
    let label_rect = egui::Rect::from_center_size(mid, egui::vec2(label_w, label_h));
    painter.rect_filled(label_rect, label_h / 2.0, style.label_bg);
    painter.galley(
        egui::pos2(
            label_rect.left() + m.label_pad_h,
            label_rect.top() + m.label_pad_v,
        ),
        galley,
        style.label_text,
    );
}

#[cfg(test)]
mod tests {
    use crate::theme::Theme;

    #[test]
    fn test_edge_palette_dark_has_entries() {
        let theme = Theme::dark();
        let palette = theme.edge_palette();
        assert!(!palette.is_empty());
        assert!(palette.len() >= 6);
    }

    #[test]
    fn test_edge_palette_light_has_entries() {
        let theme = Theme::light();
        let palette = theme.edge_palette();
        assert!(!palette.is_empty());
        assert!(palette.len() >= 6);
    }

    #[test]
    fn test_edge_palette_colors_are_distinct() {
        let theme = Theme::dark();
        let palette = theme.edge_palette();
        for i in 0..palette.len() {
            for j in i + 1..palette.len() {
                assert_ne!(
                    palette[i], palette[j],
                    "Colors at {i} and {j} should differ"
                );
            }
        }
    }
}
