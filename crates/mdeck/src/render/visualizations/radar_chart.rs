use eframe::egui::{self, Color32, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_DOT_RADIUS, VIZ_FONT_AXIS_LABEL, VIZ_OPACITY_LABEL, VIZ_STROKE_SEPARATOR, VizCtx,
    VizReveal, assign_steps, draw_legend_row,
    grammar::{Problem, Source, label_values_items, name_list},
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

fn read(src: &Source) -> RadarData {
    src.check_settings(&["axes"]);
    let axes = src.setting("axes").map(name_list).unwrap_or_default();
    if axes.is_empty() && !src.items.is_empty() {
        src.problem(0, "a radar chart needs 'axes: A, B, C'");
    }
    let series = label_values_items(src, "- Fighter A: 9, 7, 5")
        .into_iter()
        .map(|s| RadarSeries {
            label: s.label,
            values: s.values,
            reveal: s.reveal,
        })
        .collect();
    RadarData { axes, series }
}

fn parse_radar_chart(content: &str) -> RadarData {
    read(&Source::parse(content))
}

/// The problems in a `@radar` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
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

/// Where the radar's parts go.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RadarLayout {
    center: Pos2,
    radius: f32,
    /// The legend band along the bottom.
    legend_top: f32,
    legend_height: f32,
}

/// Centre the web above the legend, leaving room around it for axis labels.
fn radar_layout(pos: Pos2, max_width: f32, height: f32, scale: f32) -> RadarLayout {
    let legend_height = 40.0 * scale;
    let padding = 60.0 * scale;
    let label_margin = 50.0 * scale; // space for axis labels outside the polygon
    let radar_area_height = height - legend_height - padding;
    let radius = ((max_width - padding * 2.0 - label_margin * 2.0)
        .min(radar_area_height - label_margin * 2.0)
        / 2.0)
        .max(40.0 * scale);
    RadarLayout {
        center: Pos2::new(
            pos.x + max_width / 2.0,
            pos.y + padding / 2.0 + (radar_area_height + label_margin) / 2.0,
        ),
        radius,
        legend_top: pos.y + height - legend_height,
        legend_height,
    }
}

/// Angle of axis `i` of `num_axes`, clockwise from the top.
fn axis_angle(i: usize, num_axes: usize) -> f32 {
    let angle_step = 2.0 * std::f32::consts::PI / num_axes as f32;
    let start_angle = -std::f32::consts::FRAC_PI_2; // start at top
    start_angle + i as f32 * angle_step
}

/// The point `r` from `center` along axis `i`.
fn on_axis(center: Pos2, r: f32, i: usize, num_axes: usize) -> Pos2 {
    let angle = axis_angle(i, num_axes);
    Pos2::new(center.x + r * angle.cos(), center.y + r * angle.sin())
}

/// A series' vertices: each value as a share of `max_value` (clamped to the
/// web) along its axis, scaled by `anim` so the polygon grows from the centre.
fn series_points(
    values: &[f32],
    max_value: f32,
    anim: f32,
    layout: &RadarLayout,
    num_axes: usize,
) -> Vec<Pos2> {
    (0..num_axes)
        .map(|i| {
            let val = values.get(i).copied().unwrap_or(0.0);
            let frac = (val / max_value).clamp(0.0, 1.0) * anim;
            on_axis(layout.center, layout.radius * frac, i, num_axes)
        })
        .collect()
}

pub fn draw_radar_chart(
    cx: &VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let scale = cx.scale;
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
    let palette = cx.theme.edge_palette();
    let painter = cx.ui.painter();

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

    let layout = radar_layout(pos, max_width, height, scale);
    draw_web(cx, &layout, &data.axes);

    let dot_radius = VIZ_DOT_RADIUS * scale;
    for (si, series) in data.series.iter().enumerate() {
        let step = steps.get(si).copied().unwrap_or(0);
        if step > cx.reveal_step {
            continue;
        }
        let anim = cx.anim(step);

        let base_color = palette[si % palette.len()];
        let fill_alpha = (0.2 * 255.0 * cx.opacity * anim) as u8;
        let fill_color = Color32::from_rgba_unmultiplied(
            base_color.r(),
            base_color.g(),
            base_color.b(),
            fill_alpha,
        );
        let stroke_color = Theme::with_opacity(base_color, cx.opacity * anim);
        let points = series_points(&series.values, max_value, anim, &layout, num_axes);

        // Filled polygon (fan mesh: correct for concave shapes) plus outline
        if points.len() >= 3 {
            painter.add(series_fill_mesh(layout.center, &points, fill_color));
            painter.add(egui::Shape::closed_line(
                points.clone(),
                Stroke::new(VIZ_STROKE_SEPARATOR * scale, stroke_color),
            ));
        }

        // Dots at vertices
        for point in &points {
            painter.circle_filled(*point, dot_radius, stroke_color);
        }
    }

    // Legend at bottom
    let legend: Vec<(String, Color32)> = data
        .series
        .iter()
        .enumerate()
        .filter(|(si, _)| steps.get(*si).copied().unwrap_or(0) <= cx.reveal_step)
        .map(|(si, s)| {
            let color = Theme::with_opacity(palette[si % palette.len()], cx.opacity);
            (s.label.clone(), color)
        })
        .collect();
    draw_legend_row(
        cx,
        &legend,
        pos.x,
        max_width,
        layout.legend_top,
        layout.legend_height,
    );

    height
}

/// The spider web: concentric rings, then a spoke and a label per axis.
fn draw_web(cx: &VizCtx, layout: &RadarLayout, axes: &[String]) {
    let painter = cx.ui.painter();
    let scale = cx.scale;
    let center = layout.center;
    let radius = layout.radius;

    let grid_levels = 4u32;
    let grid_color = cx.fg(0.25);
    for level in 1..=grid_levels {
        let frac = level as f32 / grid_levels as f32;
        let r = radius * frac;
        painter.circle_stroke(center, r, Stroke::new(1.0 * scale, grid_color));
        crate::render::hints::push(
            cx.ui.ctx(),
            crate::render::hints::Hint::Circle { center, radius: r },
        );
    }

    let axis_line_color = cx.fg(0.3);
    let axis_label_font = cx.font(VIZ_FONT_AXIS_LABEL);
    let label_color = cx.fg(VIZ_OPACITY_LABEL);
    let num_axes = axes.len();
    for (i, axis_name) in axes.iter().enumerate() {
        let outer = on_axis(center, radius, i, num_axes);
        painter.line_segment([center, outer], Stroke::new(1.0 * scale, axis_line_color));

        // Axis label outside the polygon, anchored by its angle
        let label_pos = on_axis(center, radius + 14.0 * scale, i, num_axes);
        let galley =
            painter.layout_no_wrap(axis_name.clone(), axis_label_font.clone(), label_color);
        let (offset_x, offset_y) = axis_label_anchor(
            axis_angle(i, num_axes),
            galley.rect.width(),
            galley.rect.height(),
        );
        painter.galley(
            Pos2::new(label_pos.x + offset_x, label_pos.y + offset_y),
            galley,
            label_color,
        );
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_radar_chart_basic() {
        let content = "axes: Speed, Power, Range\n- Fighter: 9, 7, 5\n- Bomber: 4, 9, 8";
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
        let content = "axes: A, B, C\n- Series1: 1, 2, 3\n+ Series2: 4, 5, 6\n* Series3: 7, 8, 9";
        let data = parse_radar_chart(content);
        assert_eq!(data.series[0].reveal, VizReveal::Static);
        assert_eq!(data.series[1].reveal, VizReveal::NextStep);
        assert_eq!(data.series[2].reveal, VizReveal::WithPrev);
    }

    #[test]
    fn test_parse_radar_chart_skips_invalid() {
        let content = "axes: X, Y, Z\n# other comment\n- Valid: 1, 2, 3\n- no colon here\n";
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
        let data = parse_radar_chart("axes: A, B, C\n- S: inf, nan, inf\n- T: 1,000, 2,000, 500");
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

    #[test]
    fn test_radar_layout_centres_the_web() {
        let l = radar_layout(Pos2::new(0.0, 0.0), 1000.0, 600.0, 1.0);
        assert_eq!(l.center.x, 500.0);
        // Height-bound: (600 - 40 - 60 - 100) / 2
        assert_eq!(l.radius, 200.0);
        assert_eq!(l.legend_top, 560.0);
        // Never smaller than the minimum radius
        assert_eq!(
            radar_layout(Pos2::new(0.0, 0.0), 100.0, 100.0, 1.0).radius,
            40.0
        );
    }

    #[test]
    fn test_series_points_grow_from_centre_and_clamp() {
        let l = radar_layout(Pos2::new(0.0, 0.0), 1000.0, 600.0, 1.0);
        let points = series_points(&[10.0, 20.0, 5.0, 0.0], 10.0, 1.0, &l, 4);
        // First axis points straight up, clamped to the web's radius
        assert!((points[0].x - l.center.x).abs() < 1e-3);
        assert!((points[0].y - (l.center.y - l.radius)).abs() < 1e-3);
        // Second axis points right and is clamped at the rim too
        assert!((points[1].x - (l.center.x + l.radius)).abs() < 1e-3);
        assert_eq!(points[3], on_axis(l.center, 0.0, 3, 4));
        let collapsed = series_points(&[10.0, 20.0, 5.0], 10.0, 0.0, &l, 3);
        assert!(collapsed.iter().all(|p| (*p - l.center).length() < 1e-3));
    }
}
