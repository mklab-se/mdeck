use eframe::egui::{Pos2, Stroke};

use crate::theme::Theme;

use super::{
    AxisTitles, PlotFrame, VIZ_FONT_GRID_LABEL, VIZ_FONT_SECONDARY_LABEL,
    VIZ_LABEL_REVEAL_THRESHOLD, VIZ_OPACITY_GRID, VIZ_OPACITY_GRID_LABEL, VIZ_OPACITY_LABEL,
    VIZ_SCATTER_RADIUS, VIZ_STROKE_GRID, ValueRange, VizReveal, assign_steps, format_axis_value,
    grid_range_values, header_directive, label_fade, nice_grid_step, parse_reveal_prefix,
    parse_value, strip_thousands_separators,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ScatterPoint {
    label: String,
    x: f32,
    y: f32,
    size: Option<f32>,
    reveal: VizReveal,
}

struct ScatterData {
    points: Vec<ScatterPoint>,
    x_label: Option<String>,
    y_label: Option<String>,
}

fn parse_scatter_plot(content: &str) -> ScatterData {
    let mut points = Vec::new();
    let mut x_label = None;
    let mut y_label = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('#') {
            match header_directive(trimmed) {
                Some(("x-label", val)) => x_label = Some(val.to_string()),
                Some(("y-label", val)) => y_label = Some(val.to_string()),
                _ => {}
            }
            continue;
        }
        let (text, reveal) = parse_reveal_prefix(trimmed);
        if text.is_empty() {
            continue;
        }

        // Parse "Label: X, Y" or "Label: X, Y (size: N)"
        if let Some(colon_pos) = text.find(": ") {
            let label = text[..colon_pos].trim().to_string();
            let rest = text[colon_pos + 2..].trim();

            // Extract optional (size: N) suffix
            let (coords_str, size) = if let Some(paren_start) = rest.find('(') {
                let coords = rest[..paren_start].trim().trim_end_matches(',').trim();
                let inner = rest[paren_start..]
                    .trim_start_matches('(')
                    .trim_end_matches(')');
                let sz = inner
                    .strip_prefix("size:")
                    .and_then(parse_value)
                    .filter(|s| *s > 0.0);
                (coords, sz)
            } else {
                (rest, None)
            };

            // Parse "X, Y"
            let coords = strip_thousands_separators(coords_str);
            let parts: Vec<&str> = coords.split(',').collect();
            if parts.len() == 2
                && let (Some(x), Some(y)) = (parse_value(parts[0]), parse_value(parts[1]))
            {
                points.push(ScatterPoint {
                    label,
                    x,
                    y,
                    size,
                    reveal,
                });
            }
        }
    }
    ScatterData {
        points,
        x_label,
        y_label,
    }
}

// ─── Layout ─────────────────────────────────────────────────────────────────

/// Space around the plot (multiplied by scale).
const PADDING: f32 = 60.0;

/// The span of `values` widened by a tenth on each side (a unit at least), so
/// no point sits on the plot's edge.
fn padded_range(values: impl Iterator<Item = f32> + Clone) -> ValueRange {
    let min = values.clone().fold(f32::INFINITY, f32::min);
    let max = values.fold(f32::NEG_INFINITY, f32::max);
    let span = (max - min).max(1.0);
    ValueRange {
        min: min - span * 0.1,
        max: max + span * 0.1,
    }
}

/// The plot, with room for grid values left of and below it.
fn scatter_frame(pos: Pos2, max_width: f32, height: f32, scale: f32) -> PlotFrame {
    let padding = PADDING * scale;
    let axis_label_space = 40.0 * scale;
    PlotFrame::from_edges(
        pos.x + padding + axis_label_space,
        pos.y + padding,
        pos.x + max_width - padding,
        pos.y + height - padding - axis_label_space,
    )
}

// ─── Renderer ───────────────────────────────────────────────────────────────

pub fn draw_scatter_plot(
    cx: &super::VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let scale = cx.scale;
    let data = parse_scatter_plot(content);
    let points = &data.points;
    if points.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    let reveals: Vec<VizReveal> = points.iter().map(|p| p.reveal).collect();
    let steps = assign_steps(&reveals);
    let palette = cx.theme.edge_palette();

    let x_range = padded_range(points.iter().map(|p| p.x));
    let y_range = padded_range(points.iter().map(|p| p.y));
    let frame = scatter_frame(pos, max_width, height, scale);

    frame.draw_x_axis(cx);
    frame.draw_y_axis(cx);
    draw_x_grid(cx, &frame, x_range);
    let y_step = nice_grid_step(y_range.max - y_range.min, 5);
    frame.draw_y_grid(
        cx,
        grid_range_values(y_range.min, y_range.max, y_step),
        y_range,
        y_step,
        false,
    );
    frame.draw_titles(
        cx,
        &AxisTitles {
            x: data.x_label.as_deref(),
            x_top: frame.bottom + 28.0 * scale,
            y: data.y_label.as_deref(),
            y_left: pos.x + PADDING * scale * 0.3,
        },
    );

    // Data points
    let painter = cx.ui.painter();
    let label_font = cx.font(VIZ_FONT_SECONDARY_LABEL);
    let default_radius = VIZ_SCATTER_RADIUS * scale;
    for (i, point) in points.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > cx.reveal_step {
            continue;
        }
        let anim = cx.anim(step);
        let center = Pos2::new(frame.x_at(point.x, x_range), frame.y_at(point.y, y_range));
        let radius = point.size.map_or(default_radius, |s| s * scale * 0.5) * anim;

        painter.circle_filled(center, radius, cx.fill(&palette, i));
        crate::render::hints::push(cx.ui.ctx(), crate::render::hints::Hint::Point(center));

        // Label near the dot
        if anim > VIZ_LABEL_REVEAL_THRESHOLD {
            let label_color = Theme::with_opacity(
                cx.theme.foreground,
                cx.opacity * VIZ_OPACITY_LABEL * label_fade(anim),
            );
            let galley =
                painter.layout_no_wrap(point.label.clone(), label_font.clone(), label_color);
            painter.galley(
                Pos2::new(
                    center.x + radius + 4.0 * scale,
                    center.y - galley.rect.height() / 2.0,
                ),
                galley,
                label_color,
            );
        }
    }

    height
}

/// Vertical grid lines across `range`, each labelled below the plot.
fn draw_x_grid(cx: &super::VizCtx, frame: &PlotFrame, range: ValueRange) {
    let painter = cx.ui.painter();
    let scale = cx.scale;
    let grid_color = cx.fg(VIZ_OPACITY_GRID);
    let grid_font = cx.font(VIZ_FONT_GRID_LABEL);
    let grid_label_color = cx.fg(VIZ_OPACITY_GRID_LABEL);
    let x_step = nice_grid_step(range.max - range.min, 5);
    for gx in grid_range_values(range.min, range.max, x_step) {
        let px = frame.x_at(gx, range);
        painter.line_segment(
            [Pos2::new(px, frame.top), Pos2::new(px, frame.bottom)],
            Stroke::new(VIZ_STROKE_GRID * scale, grid_color),
        );
        let label = format_axis_value(gx, x_step);
        let galley = painter.layout_no_wrap(label, grid_font.clone(), grid_label_color);
        painter.galley(
            Pos2::new(px - galley.rect.width() / 2.0, frame.bottom + 6.0 * scale),
            galley,
            grid_label_color,
        );
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scatter_basic() {
        let content = "- Alice: 80, 90\n- Bob: 65, 75";
        let data = parse_scatter_plot(content);
        assert_eq!(data.points.len(), 2);
        assert_eq!(data.points[0].label, "Alice");
        assert_eq!(data.points[0].x, 80.0);
        assert_eq!(data.points[0].y, 90.0);
        assert!(data.points[0].size.is_none());
    }

    #[test]
    fn test_parse_scatter_with_size() {
        let content = "- Dave: 40, 60 (size: 30)";
        let data = parse_scatter_plot(content);
        assert_eq!(data.points.len(), 1);
        assert_eq!(data.points[0].label, "Dave");
        assert_eq!(data.points[0].x, 40.0);
        assert_eq!(data.points[0].y, 60.0);
        assert_eq!(data.points[0].size, Some(30.0));
    }

    #[test]
    fn test_parse_scatter_reveal_markers() {
        let content = "- A: 10, 20\n+ B: 30, 40\n* C: 50, 60";
        let data = parse_scatter_plot(content);
        assert_eq!(data.points[0].reveal, VizReveal::Static);
        assert_eq!(data.points[1].reveal, VizReveal::NextStep);
        assert_eq!(data.points[2].reveal, VizReveal::WithPrev);
    }

    #[test]
    fn test_parse_scatter_skips_invalid() {
        let content = "- Valid: 10, 20\n- Bad: only_one\n# comment\n- Also: 30, 40";
        let data = parse_scatter_plot(content);
        assert_eq!(data.points.len(), 2);
    }

    #[test]
    fn test_parse_scatter_axis_labels() {
        let content = "# x-label: Hours Studied\n# y-label: Test Score\n- Alice: 80, 90";
        let data = parse_scatter_plot(content);
        assert_eq!(data.x_label, Some("Hours Studied".to_string()));
        assert_eq!(data.y_label, Some("Test Score".to_string()));
        assert_eq!(data.points.len(), 1);
    }

    #[test]
    fn test_parse_scatter_rejects_non_finite() {
        let data = parse_scatter_plot("- A: inf, 1\n- B: 1, nan\n- C: 2, 3 (size: inf)\n- D: 4, 5");
        assert_eq!(data.points.len(), 2);
        assert_eq!(data.points[0].label, "C");
        assert_eq!(data.points[0].size, None);
        assert_eq!(data.points[1].label, "D");
    }

    #[test]
    fn test_parse_scatter_decorated_values() {
        let data = parse_scatter_plot("- A: $1,000, 2,500\n- B: 40%, 60%");
        assert_eq!(data.points.len(), 2);
        assert_eq!((data.points[0].x, data.points[0].y), (1000.0, 2500.0));
        assert_eq!((data.points[1].x, data.points[1].y), (40.0, 60.0));
    }

    #[test]
    fn test_parse_scatter_compact_axis_directives() {
        let data = parse_scatter_plot("#x-label: Hours\n#y-label: Score\n- A: 1, 2");
        assert_eq!(data.x_label.as_deref(), Some("Hours"));
        assert_eq!(data.y_label.as_deref(), Some("Score"));
    }

    #[test]
    fn test_padded_range_widens_by_a_tenth() {
        let r = padded_range([10.0, 30.0].into_iter());
        assert_eq!((r.min, r.max), (8.0, 32.0));
        // A single value still gets a unit-wide span around it
        let r = padded_range([5.0].into_iter());
        assert_eq!((r.min, r.max), (4.9, 5.1));
    }

    #[test]
    fn test_scatter_frame_leaves_room_for_grid_values() {
        let f = scatter_frame(Pos2::new(0.0, 0.0), 1000.0, 600.0, 1.0);
        assert_eq!(
            (f.left, f.top, f.right, f.bottom),
            (100.0, 60.0, 940.0, 500.0)
        );
    }
}
