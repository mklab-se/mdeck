use std::time::Instant;

use eframe::egui::{self, FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_FONT_AXIS_LABEL, VIZ_FONT_GRID_LABEL, VIZ_FONT_SECONDARY_LABEL, VIZ_LABEL_REVEAL_THRESHOLD,
    VIZ_OPACITY_AXIS, VIZ_OPACITY_FILL, VIZ_OPACITY_GRID, VIZ_OPACITY_GRID_LABEL,
    VIZ_OPACITY_LABEL, VIZ_SCATTER_RADIUS, VIZ_STROKE_AXIS, VIZ_STROKE_GRID, VizReveal,
    assign_steps, draw_x_axis_label, draw_y_axis_label, format_axis_value, grid_range_values,
    label_fade, nice_grid_step, parse_axis_label_directive, parse_reveal_prefix, parse_value,
    reveal_anim_progress, strip_thousands_separators,
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
            if let Some((key, val)) = parse_axis_label_directive(trimmed) {
                match key {
                    "x-label" => x_label = Some(val),
                    "y-label" => y_label = Some(val),
                    _ => {}
                }
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

// ─── Renderer ───────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_scatter_plot(
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
    let palette = theme.edge_palette();
    let painter = ui.painter();

    // Compute data bounds
    let x_min = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let x_max = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let y_min = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let y_max = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

    // Add some padding to data range
    let x_range = (x_max - x_min).max(1.0);
    let y_range = (y_max - y_min).max(1.0);
    let data_x_min = x_min - x_range * 0.1;
    let data_x_max = x_max + x_range * 0.1;
    let data_y_min = y_min - y_range * 0.1;
    let data_y_max = y_max + y_range * 0.1;

    // Chart area
    let padding = 60.0 * scale;
    let axis_label_space = 40.0 * scale;
    let chart_left = pos.x + padding + axis_label_space;
    let chart_right = pos.x + max_width - padding;
    let chart_top = pos.y + padding;
    let chart_bottom = pos.y + height - padding - axis_label_space;
    let chart_width = chart_right - chart_left;
    let chart_height = chart_bottom - chart_top;

    let axis_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_AXIS);
    let grid_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_GRID);
    let grid_font = FontId::proportional(theme.body_size * VIZ_FONT_GRID_LABEL * scale);
    let label_font = FontId::proportional(theme.body_size * VIZ_FONT_SECONDARY_LABEL * scale);

    // Draw axes
    painter.line_segment(
        [
            Pos2::new(chart_left, chart_bottom),
            Pos2::new(chart_right, chart_bottom),
        ],
        Stroke::new(VIZ_STROKE_AXIS * scale, axis_color),
    );
    painter.line_segment(
        [
            Pos2::new(chart_left, chart_top),
            Pos2::new(chart_left, chart_bottom),
        ],
        Stroke::new(VIZ_STROKE_AXIS * scale, axis_color),
    );

    // X-axis grid lines
    let x_step = nice_grid_step(data_x_max - data_x_min, 5);
    let grid_label_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_GRID_LABEL);
    for gx in grid_range_values(data_x_min, data_x_max, x_step) {
        let frac = (gx - data_x_min) / (data_x_max - data_x_min);
        let px = chart_left + frac * chart_width;
        painter.line_segment(
            [Pos2::new(px, chart_top), Pos2::new(px, chart_bottom)],
            Stroke::new(VIZ_STROKE_GRID * scale, grid_color),
        );
        let label = format_axis_value(gx, x_step);
        let galley = painter.layout_no_wrap(label, grid_font.clone(), grid_label_color);
        painter.galley(
            Pos2::new(px - galley.rect.width() / 2.0, chart_bottom + 6.0 * scale),
            galley,
            grid_label_color,
        );
    }

    // Y-axis grid lines
    let y_step = nice_grid_step(data_y_max - data_y_min, 5);
    for gy in grid_range_values(data_y_min, data_y_max, y_step) {
        let frac = (gy - data_y_min) / (data_y_max - data_y_min);
        let py = chart_bottom - frac * chart_height;
        painter.line_segment(
            [Pos2::new(chart_left, py), Pos2::new(chart_right, py)],
            Stroke::new(VIZ_STROKE_GRID * scale, grid_color),
        );
        let label = format_axis_value(gy, y_step);
        let galley = painter.layout_no_wrap(label, grid_font.clone(), grid_label_color);
        painter.galley(
            Pos2::new(
                chart_left - galley.rect.width() - 8.0 * scale,
                py - galley.rect.height() / 2.0,
            ),
            galley,
            grid_label_color,
        );
    }

    // Draw axis labels
    let axis_label_font = FontId::proportional(theme.body_size * VIZ_FONT_AXIS_LABEL * scale);
    let axis_label_color = Theme::with_opacity(theme.foreground, opacity * 0.7);

    if let Some(ref text) = data.x_label {
        draw_x_axis_label(
            painter,
            text,
            axis_label_font.clone(),
            axis_label_color,
            chart_left,
            chart_width,
            chart_bottom + 28.0 * scale,
        );
    }
    if let Some(ref text) = data.y_label {
        draw_y_axis_label(
            painter,
            text,
            axis_label_font,
            axis_label_color,
            pos.x + padding * 0.3,
            chart_top,
            chart_height,
        );
    }

    // Draw data points
    let mut needs_repaint = false;
    let default_radius = VIZ_SCATTER_RADIUS * scale;

    for (i, point) in points.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }

        let (anim, repaint) = reveal_anim_progress(step, reveal_step, reveal_timestamp);
        if repaint {
            needs_repaint = true;
        }

        let fx = (point.x - data_x_min) / (data_x_max - data_x_min);
        let fy = (point.y - data_y_min) / (data_y_max - data_y_min);
        let px = chart_left + fx * chart_width;
        let py = chart_bottom - fy * chart_height;

        let radius = point.size.map_or(default_radius, |s| s * scale * 0.5) * anim;
        let color = Theme::with_opacity(palette[i % palette.len()], opacity * VIZ_OPACITY_FILL);

        painter.circle_filled(Pos2::new(px, py), radius, color);

        // Label near the dot
        if anim > VIZ_LABEL_REVEAL_THRESHOLD {
            let label_color = Theme::with_opacity(
                theme.foreground,
                opacity * VIZ_OPACITY_LABEL * label_fade(anim),
            );
            let galley =
                painter.layout_no_wrap(point.label.clone(), label_font.clone(), label_color);
            painter.galley(
                Pos2::new(px + radius + 4.0 * scale, py - galley.rect.height() / 2.0),
                galley,
                label_color,
            );
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
}
