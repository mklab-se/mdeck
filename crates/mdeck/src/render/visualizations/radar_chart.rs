use std::time::Instant;

use eframe::egui::{self, Color32, FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_CORNER_SWATCH, VIZ_DOT_RADIUS, VIZ_FONT_AXIS_LABEL, VIZ_FONT_LEGEND, VIZ_OPACITY_LABEL,
    VIZ_STROKE_SEPARATOR, VIZ_SWATCH_SIZE, VizReveal, assign_steps, parse_label_values,
    parse_reveal_prefix, reveal_anim_progress,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct RadarSeries {
    label: String,
    values: Vec<f32>,
    reveal: VizReveal,
}

#[derive(Debug, Clone)]
struct RadarData {
    axes: Vec<String>,
    series: Vec<RadarSeries>,
}

fn parse_radar_chart(content: &str) -> RadarData {
    let mut axes = Vec::new();
    let mut series = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse axes directive
        if trimmed.starts_with('#') {
            if let Some(rest) = trimmed
                .strip_prefix("# axes:")
                .or_else(|| trimmed.strip_prefix("#axes:"))
            {
                axes = rest.split(',').map(|s| s.trim().to_string()).collect();
            }
            continue;
        }

        let (text, reveal) = parse_reveal_prefix(trimmed);
        if text.is_empty() {
            continue;
        }

        // Parse "Series Name: v1, v2, v3, ..."
        if let Some((label, values)) = parse_label_values(text) {
            series.push(RadarSeries {
                label,
                values,
                reveal,
            });
        }
    }

    RadarData { axes, series }
}

// ─── Renderer ───────────────────────────────────────────────────────────────

/// Fill for a series polygon as a fan of triangles from the centre. Radar
/// polygons are star-shaped around the centre but usually not convex, so a
/// convex fill would paint over the notches between low and high axes.
fn series_fill_mesh(center: Pos2, points: &[Pos2], color: Color32) -> egui::Shape {
    use eframe::epaint::{Mesh, Vertex, WHITE_UV};

    let mut mesh = Mesh::default();
    let vertex = |pos: Pos2| Vertex {
        pos,
        uv: WHITE_UV,
        color,
    };
    mesh.vertices.push(vertex(center));
    for &p in points {
        mesh.vertices.push(vertex(p));
    }
    let n = points.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    egui::Shape::mesh(mesh)
}

/// Offset of an axis label's top-left corner from its anchor point so the text
/// sits clear of the ring: left-aligned on the right side, right-aligned on the
/// left side, centred above/below at the top and bottom.
fn axis_label_anchor(angle: f32, width: f32, height: f32) -> (f32, f32) {
    let (sin, cos) = angle.sin_cos();
    let dx = if cos > 0.2 {
        0.0
    } else if cos < -0.2 {
        -width
    } else {
        -width / 2.0
    };
    let dy = if sin > 0.2 {
        0.0
    } else if sin < -0.2 {
        -height
    } else {
        -height / 2.0
    };
    (dx, dy)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_radar_chart(
    ui: &egui::Ui,
    content: &str,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
    opacity: f32,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
    scale: f32,
) -> f32 {
    let data = parse_radar_chart(content);
    if data.series.is_empty() || data.axes.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    let reveals: Vec<VizReveal> = data.series.iter().map(|s| s.reveal).collect();
    let steps = assign_steps(&reveals);
    let palette = theme.edge_palette();
    let painter = ui.painter();

    let num_axes = data.axes.len();
    if num_axes < 3 {
        return height;
    }

    // Find max value across all series for normalization
    let max_value = data
        .series
        .iter()
        .flat_map(|s| s.values.iter())
        .fold(0.0f32, |a, &b| a.max(b));
    if max_value <= 0.0 {
        return height;
    }

    // Layout: radar polygon centered, legend at bottom
    let legend_height = 40.0 * scale;
    let padding = 60.0 * scale;
    let label_margin = 50.0 * scale; // space for axis labels outside the polygon
    let radar_area_height = height - legend_height - padding;
    let radar_radius = ((max_width - padding * 2.0 - label_margin * 2.0)
        .min(radar_area_height - label_margin * 2.0)
        / 2.0)
        .max(40.0 * scale);
    let cx = pos.x + max_width / 2.0;
    let cy = pos.y + padding / 2.0 + (radar_area_height + label_margin) / 2.0;

    let angle_step = 2.0 * std::f32::consts::PI / num_axes as f32;
    let start_angle = -std::f32::consts::FRAC_PI_2; // start at top

    // Draw concentric circular grid rings (spider web)
    let grid_levels = 4u32;
    let grid_color = Theme::with_opacity(theme.foreground, opacity * 0.25);
    for level in 1..=grid_levels {
        let frac = level as f32 / grid_levels as f32;
        let r = radar_radius * frac;
        painter.circle_stroke(Pos2::new(cx, cy), r, Stroke::new(1.0 * scale, grid_color));
    }

    // Draw axis lines (spokes of the spider web)
    let axis_line_color = Theme::with_opacity(theme.foreground, opacity * 0.3);
    let axis_label_font = FontId::proportional(theme.body_size * VIZ_FONT_AXIS_LABEL * scale);
    let label_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_LABEL);

    for (i, axis_name) in data.axes.iter().enumerate() {
        let angle = start_angle + i as f32 * angle_step;
        let outer = Pos2::new(
            cx + radar_radius * angle.cos(),
            cy + radar_radius * angle.sin(),
        );
        painter.line_segment(
            [Pos2::new(cx, cy), outer],
            Stroke::new(1.0 * scale, axis_line_color),
        );

        // Axis label outside the polygon, anchored by its angle
        let label_r = radar_radius + 14.0 * scale;
        let label_pos = Pos2::new(cx + label_r * angle.cos(), cy + label_r * angle.sin());
        let galley =
            painter.layout_no_wrap(axis_name.clone(), axis_label_font.clone(), label_color);
        let (offset_x, offset_y) =
            axis_label_anchor(angle, galley.rect.width(), galley.rect.height());
        painter.galley(
            Pos2::new(label_pos.x + offset_x, label_pos.y + offset_y),
            galley,
            label_color,
        );
    }

    // Draw series polygons
    let mut needs_repaint = false;

    for (si, series) in data.series.iter().enumerate() {
        let step = steps.get(si).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }

        let (anim, repaint) = reveal_anim_progress(step, reveal_step, reveal_timestamp);
        if repaint {
            needs_repaint = true;
        }

        let base_color = palette[si % palette.len()];
        let fill_alpha = (0.2 * 255.0 * opacity * anim) as u8;
        let fill_color = Color32::from_rgba_unmultiplied(
            base_color.r(),
            base_color.g(),
            base_color.b(),
            fill_alpha,
        );
        let stroke_color = Theme::with_opacity(base_color, opacity * anim);

        // Build polygon points, scaled by anim_progress (grows from center)
        let points: Vec<Pos2> = (0..num_axes)
            .map(|i| {
                let val = series.values.get(i).copied().unwrap_or(0.0);
                let frac = (val / max_value).clamp(0.0, 1.0) * anim;
                let r = radar_radius * frac;
                let angle = start_angle + i as f32 * angle_step;
                Pos2::new(cx + r * angle.cos(), cy + r * angle.sin())
            })
            .collect();

        // Filled polygon (fan mesh: correct for concave shapes) plus outline
        if points.len() >= 3 {
            painter.add(series_fill_mesh(Pos2::new(cx, cy), &points, fill_color));
            painter.add(egui::Shape::closed_line(
                points.clone(),
                Stroke::new(VIZ_STROKE_SEPARATOR * scale, stroke_color),
            ));
        }

        // Dots at vertices
        let dot_radius = VIZ_DOT_RADIUS * scale;
        for point in &points {
            painter.circle_filled(*point, dot_radius, stroke_color);
        }
    }

    // Draw legend at bottom
    let legend_font = FontId::proportional(theme.body_size * VIZ_FONT_LEGEND * scale);
    let swatch_size = VIZ_SWATCH_SIZE * scale;
    let item_spacing = 28.0 * scale;

    // Calculate total legend width to center it
    let legend_items: Vec<(String, Color32)> = data
        .series
        .iter()
        .enumerate()
        .filter(|(si, _)| {
            let step = steps.get(*si).copied().unwrap_or(0);
            step <= reveal_step
        })
        .map(|(si, s)| {
            let color = Theme::with_opacity(palette[si % palette.len()], opacity);
            (s.label.clone(), color)
        })
        .collect();

    if !legend_items.is_empty() {
        // Estimate total width
        let mut total_w = 0.0f32;
        let galleys: Vec<_> = legend_items
            .iter()
            .map(|(name, color)| {
                let g = painter.layout_no_wrap(name.clone(), legend_font.clone(), *color);
                let w = swatch_size + 6.0 * scale + g.rect.width() + item_spacing;
                total_w += w;
                (g, *color)
            })
            .collect();
        total_w -= item_spacing; // remove trailing spacing

        let legend_y = pos.y + height - legend_height;
        let mut lx = pos.x + (max_width - total_w) / 2.0;

        for (galley, color) in galleys {
            let swatch_rect = egui::Rect::from_min_size(
                Pos2::new(lx, legend_y + (legend_height - swatch_size) / 2.0),
                egui::vec2(swatch_size, swatch_size),
            );
            painter.rect_filled(swatch_rect, VIZ_CORNER_SWATCH * scale, color);
            lx += swatch_size + 6.0 * scale;

            let text_y = legend_y + (legend_height - galley.rect.height()) / 2.0;
            let w = galley.rect.width();
            let text_color = Theme::with_opacity(theme.foreground, opacity);
            painter.galley(Pos2::new(lx, text_y), galley, text_color);
            lx += w + item_spacing;
        }
    }

    if needs_repaint {
        ui.ctx().request_repaint();
    }

    height
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_radar_chart_basic() {
        let content = "# axes: Speed, Power, Range\n- Fighter: 9, 7, 5\n- Bomber: 4, 9, 8";
        let data = parse_radar_chart(content);
        assert_eq!(data.axes, vec!["Speed", "Power", "Range"]);
        assert_eq!(data.series.len(), 2);
        assert_eq!(data.series[0].label, "Fighter");
        assert_eq!(data.series[0].values, vec![9.0, 7.0, 5.0]);
        assert_eq!(data.series[1].label, "Bomber");
        assert_eq!(data.series[1].values, vec![4.0, 9.0, 8.0]);
    }

    #[test]
    fn test_parse_radar_chart_reveal_markers() {
        let content = "# axes: A, B, C\n- Series1: 1, 2, 3\n+ Series2: 4, 5, 6\n* Series3: 7, 8, 9";
        let data = parse_radar_chart(content);
        assert_eq!(data.series[0].reveal, VizReveal::Static);
        assert_eq!(data.series[1].reveal, VizReveal::NextStep);
        assert_eq!(data.series[2].reveal, VizReveal::WithPrev);
    }

    #[test]
    fn test_parse_radar_chart_skips_invalid() {
        let content = "# axes: X, Y, Z\n# other comment\n- Valid: 1, 2, 3\n- no colon here\n";
        let data = parse_radar_chart(content);
        assert_eq!(data.axes, vec!["X", "Y", "Z"]);
        assert_eq!(data.series.len(), 1);
    }

    #[test]
    fn test_parse_radar_chart_no_axes_directive() {
        let content = "- Fighter: 9, 7, 5";
        let data = parse_radar_chart(content);
        assert!(data.axes.is_empty());
        assert_eq!(data.series.len(), 1);
    }

    #[test]
    fn test_parse_radar_chart_rejects_non_finite_and_thousands() {
        let data = parse_radar_chart("# axes: A, B, C\n- S: inf, nan, inf\n- T: 1,000, 2,000, 500");
        assert_eq!(data.series.len(), 1);
        assert_eq!(data.series[0].values, vec![1000.0, 2000.0, 500.0]);
    }

    #[test]
    fn test_series_fill_mesh_covers_star_shape() {
        // A 6-point star: a convex fill would cover the notches between the spikes;
        // the fan mesh must leave them empty.
        let center = Pos2::new(0.0, 0.0);
        let points: Vec<Pos2> = (0..6)
            .map(|i| {
                let r = if i % 2 == 0 { 100.0 } else { 10.0 };
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::FRAC_PI_3;
                Pos2::new(r * a.cos(), r * a.sin())
            })
            .collect();
        let egui::Shape::Mesh(mesh) = series_fill_mesh(center, &points, Color32::RED) else {
            panic!("expected a mesh");
        };
        assert_eq!(mesh.indices.len(), 6 * 3);
        // A point in a notch (between spike 0 at the top and spike 2) is outside every triangle
        let notch_angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_3;
        let notch = Pos2::new(60.0 * notch_angle.cos(), 60.0 * notch_angle.sin());
        assert!(!mesh_contains(&mesh, notch));
        // A point on a spike is inside
        let tip = Pos2::new(0.0, -50.0);
        assert!(mesh_contains(&mesh, tip));
    }

    fn mesh_contains(mesh: &eframe::epaint::Mesh, p: Pos2) -> bool {
        mesh.indices.chunks(3).any(|tri| {
            let a = mesh.vertices[tri[0] as usize].pos;
            let b = mesh.vertices[tri[1] as usize].pos;
            let c = mesh.vertices[tri[2] as usize].pos;
            let sign = |p1: Pos2, p2: Pos2, p3: Pos2| {
                (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
            };
            let d1 = sign(p, a, b);
            let d2 = sign(p, b, c);
            let d3 = sign(p, c, a);
            let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
            let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
            !(has_neg && has_pos)
        })
    }

    #[test]
    fn test_axis_label_anchor_by_angle() {
        use std::f32::consts::{FRAC_PI_2, PI};
        // Top: centred horizontally, sits above the point
        assert_eq!(axis_label_anchor(-FRAC_PI_2, 100.0, 20.0), (-50.0, -20.0));
        // Bottom: centred, below the point
        assert_eq!(axis_label_anchor(FRAC_PI_2, 100.0, 20.0), (-50.0, 0.0));
        // Right side: left-aligned (text starts at the point), vertically centred
        assert_eq!(axis_label_anchor(0.0, 100.0, 20.0), (0.0, -10.0));
        // Left side: right-aligned (text ends at the point)
        assert_eq!(axis_label_anchor(PI, 100.0, 20.0), (-100.0, -10.0));
    }
}
