use std::time::Instant;

use eframe::egui::{self, Color32, FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_CORNER_BAR, VIZ_FONT_AXIS_LABEL, VIZ_FONT_CATEGORY_LABEL, VIZ_FONT_GRID_LABEL,
    VIZ_FONT_MIN, VIZ_FONT_VALUE_LABEL, VIZ_LABEL_REVEAL_THRESHOLD, VIZ_OPACITY_AXIS,
    VIZ_OPACITY_FILL, VIZ_OPACITY_GRID, VIZ_OPACITY_GRID_LABEL, VIZ_OPACITY_LABEL, VIZ_STROKE_AXIS,
    VIZ_STROKE_GRID, VizReveal, assign_steps, draw_x_axis_label, draw_y_axis_label, fit_font_size,
    fit_text, format_axis_value, format_value, grid_values, label_fade, nice_axis_max,
    nice_grid_step, parse_axis_label_directive, parse_label_value, parse_reveal_prefix,
    reveal_anim_progress,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone)]
struct BarEntry {
    label: String,
    value: f32,
    reveal: VizReveal,
}

struct BarChartData {
    entries: Vec<BarEntry>,
    orientation: Orientation,
    x_label: Option<String>,
    y_label: Option<String>,
}

fn parse_bar_chart(content: &str) -> BarChartData {
    let mut entries = Vec::new();
    let mut orientation = Orientation::Vertical;
    let mut x_label = None;
    let mut y_label = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse directives from comments
        if trimmed.starts_with('#') {
            if let Some(rest) = trimmed
                .strip_prefix("# orientation:")
                .or_else(|| trimmed.strip_prefix("#orientation:"))
            {
                let val = rest.trim();
                if val.eq_ignore_ascii_case("horizontal") {
                    orientation = Orientation::Horizontal;
                } else if val.eq_ignore_ascii_case("vertical") {
                    orientation = Orientation::Vertical;
                }
            } else if let Some((key, val)) = parse_axis_label_directive(trimmed) {
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

        // Parse "Label: 40", "Label: 40%", "Label: $1,000", ...
        if let Some((label, value)) = parse_label_value(text) {
            entries.push(BarEntry {
                label,
                value,
                reveal,
            });
        }
    }

    BarChartData {
        entries,
        orientation,
        x_label,
        y_label,
    }
}

// ─── Renderer ───────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_bar_chart(
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
    let data = parse_bar_chart(content);
    if data.entries.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    let reveals: Vec<VizReveal> = data.entries.iter().map(|e| e.reveal).collect();
    let steps = assign_steps(&reveals);
    let palette = theme.edge_palette();
    let painter = ui.painter();

    let max_value = data.entries.iter().map(|e| e.value).fold(0.0f32, f32::max);
    if max_value <= 0.0 {
        return height;
    }
    // Scale the axis to a round number so the tallest bar never touches the top
    let max_value = nice_axis_max(max_value, 5);

    let needs_repaint = match data.orientation {
        Orientation::Vertical => draw_vertical(
            painter,
            &data.entries,
            &steps,
            &palette,
            theme,
            pos,
            max_width,
            height,
            max_value,
            opacity,
            reveal_step,
            reveal_timestamp,
            scale,
            data.x_label.as_deref(),
            data.y_label.as_deref(),
        ),
        Orientation::Horizontal => draw_horizontal(
            painter,
            &data.entries,
            &steps,
            &palette,
            theme,
            pos,
            max_width,
            height,
            max_value,
            opacity,
            reveal_step,
            reveal_timestamp,
            scale,
            data.x_label.as_deref(),
            data.y_label.as_deref(),
        ),
    };

    if needs_repaint {
        ui.ctx().request_repaint();
    }

    height
}

#[allow(clippy::too_many_arguments)]
fn draw_vertical(
    painter: &egui::Painter,
    entries: &[BarEntry],
    steps: &[usize],
    palette: &[Color32],
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    height: f32,
    max_value: f32,
    opacity: f32,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
    scale: f32,
    x_label: Option<&str>,
    y_label: Option<&str>,
) -> bool {
    let mut needs_repaint = false;
    let n = entries.len();
    let padding = 60.0 * scale;
    let label_area = 40.0 * scale; // space for labels below bars
    let value_area = 30.0 * scale; // space for value labels above bars
    let y_label_space = if y_label.is_some() { 30.0 * scale } else { 0.0 };
    let x_label_space = if x_label.is_some() { 30.0 * scale } else { 0.0 };
    let chart_height = height - padding - label_area - value_area - x_label_space;
    let chart_bottom = pos.y + padding + value_area + chart_height;

    // Reserve room on the left for the widest grid label (plus the axis title)
    let grid_step = nice_grid_step(max_value, 5);
    let grid_font = FontId::proportional(theme.body_size * VIZ_FONT_GRID_LABEL * scale);
    let grid_label_w = grid_values(max_value, grid_step)
        .into_iter()
        .map(|v| {
            painter
                .layout_no_wrap(
                    format_axis_value(v, grid_step),
                    grid_font.clone(),
                    Color32::WHITE,
                )
                .rect
                .width()
        })
        .fold(0.0f32, f32::max);
    let left_inset =
        (padding * 0.3 + y_label_space + grid_label_w + 16.0 * scale).max(padding + y_label_space);
    let chart_left = pos.x + left_inset;
    let chart_width = max_width - left_inset - padding;

    // Axis line
    let axis_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_AXIS);
    painter.line_segment(
        [
            Pos2::new(chart_left, chart_bottom),
            Pos2::new(chart_left + chart_width, chart_bottom),
        ],
        Stroke::new(VIZ_STROKE_AXIS * scale, axis_color),
    );

    // Grid lines with nice round numbers
    let grid_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_GRID);
    let grid_label_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_GRID_LABEL);
    for grid_val in grid_values(max_value, grid_step) {
        let frac = grid_val / max_value;
        let gy = chart_bottom - frac * chart_height;
        painter.line_segment(
            [
                Pos2::new(chart_left, gy),
                Pos2::new(chart_left + chart_width, gy),
            ],
            Stroke::new(VIZ_STROKE_GRID * scale, grid_color),
        );
        let label = format_axis_value(grid_val, grid_step);
        let galley = painter.layout_no_wrap(label, grid_font.clone(), grid_label_color);
        painter.galley(
            Pos2::new(
                chart_left - galley.rect.width() - 8.0 * scale,
                gy - galley.rect.height() / 2.0,
            ),
            galley,
            grid_label_color,
        );
    }

    // Bars
    // Gap grows with bar width so bars read as distinct columns
    let bar_gap = (chart_width / n as f32 * 0.22).clamp(10.0 * scale, 48.0 * scale);
    let total_gaps = (n + 1) as f32 * bar_gap;
    let bar_width = ((chart_width - total_gaps) / n as f32).max(8.0 * scale);
    let value_font = FontId::proportional(theme.body_size * VIZ_FONT_VALUE_LABEL * scale);
    let min_font = theme.body_size * VIZ_FONT_MIN * scale;
    // One label size for every category so the axis reads as a unit
    let label_texts: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
    let label_font = FontId::proportional(fit_font_size(
        painter,
        &label_texts,
        &FontId::proportional(theme.body_size * VIZ_FONT_CATEGORY_LABEL * scale),
        bar_width + bar_gap,
        min_font,
    ));

    for (i, entry) in entries.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }

        let (anim, repaint) = reveal_anim_progress(step, reveal_step, reveal_timestamp);
        if repaint {
            needs_repaint = true;
        }

        let color = Theme::with_opacity(palette[i % palette.len()], opacity * VIZ_OPACITY_FILL);
        // Negative values are clamped to the axis so nothing draws below the chart
        let full_bar_height = (entry.value.max(0.0) / max_value) * chart_height;
        let bar_height = full_bar_height * anim;
        let bx = chart_left + bar_gap + i as f32 * (bar_width + bar_gap);
        let by = chart_bottom - bar_height;

        // Bar with rounded corners
        let bar_rect =
            egui::Rect::from_min_size(Pos2::new(bx, by), egui::vec2(bar_width, bar_height));
        painter.rect_filled(bar_rect, VIZ_CORNER_BAR * scale, color);

        // Value label above bar (only show when animation is near-complete)
        if anim > VIZ_LABEL_REVEAL_THRESHOLD {
            let val_text = format_value(entry.value);
            let val_color = Theme::with_opacity(theme.foreground, opacity * 0.7 * label_fade(anim));
            let val_galley = painter.layout_no_wrap(val_text, value_font.clone(), val_color);
            let val_x = bx + (bar_width - val_galley.rect.width()) / 2.0;
            painter.galley(
                Pos2::new(val_x, by - val_galley.rect.height() - 4.0 * scale),
                val_galley,
                val_color,
            );
        }

        // Category label below bar, shrunk/truncated to its slot
        let label_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_LABEL);
        let galley = fit_text(
            painter,
            &entry.label,
            label_font.clone(),
            label_color,
            bar_width + bar_gap,
            label_font.size,
        );
        let lx = bx + (bar_width - galley.rect.width()) / 2.0;
        painter.galley(
            Pos2::new(lx, chart_bottom + 6.0 * scale),
            galley,
            label_color,
        );
    }

    // Axis labels
    let axis_label_font = FontId::proportional(theme.body_size * VIZ_FONT_AXIS_LABEL * scale);
    let axis_label_color = Theme::with_opacity(theme.foreground, opacity * 0.7);
    if let Some(text) = x_label {
        draw_x_axis_label(
            painter,
            text,
            axis_label_font.clone(),
            axis_label_color,
            chart_left,
            chart_width,
            chart_bottom + label_area + 4.0 * scale,
        );
    }
    if let Some(text) = y_label {
        draw_y_axis_label(
            painter,
            text,
            axis_label_font,
            axis_label_color,
            pos.x + padding * 0.3,
            pos.y + padding + value_area,
            chart_height,
        );
    }

    needs_repaint
}

#[allow(clippy::too_many_arguments)]
fn draw_horizontal(
    painter: &egui::Painter,
    entries: &[BarEntry],
    steps: &[usize],
    palette: &[Color32],
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    height: f32,
    max_value: f32,
    opacity: f32,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
    scale: f32,
    x_label: Option<&str>,
    y_label: Option<&str>,
) -> bool {
    let mut needs_repaint = false;
    let n = entries.len();
    let padding = 40.0 * scale;
    let value_area = 60.0 * scale; // space for value labels on the right
    let label_gap = 10.0 * scale;
    let label_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_LABEL);
    let min_font = theme.body_size * VIZ_FONT_MIN * scale;

    // Size the label column to the longest label, fitted into at most a third of the width.
    // All labels share one font size.
    let max_label_w = max_width * 0.33;
    let label_texts: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
    let label_font = FontId::proportional(fit_font_size(
        painter,
        &label_texts,
        &FontId::proportional(theme.body_size * VIZ_FONT_CATEGORY_LABEL * scale),
        max_label_w,
        min_font,
    ));
    let label_galleys: Vec<_> = entries
        .iter()
        .map(|e| {
            fit_text(
                painter,
                &e.label,
                label_font.clone(),
                label_color,
                max_label_w,
                label_font.size,
            )
        })
        .collect();
    let label_area = label_galleys
        .iter()
        .map(|g| g.rect.width())
        .fold(0.0f32, f32::max)
        + label_gap; // space for labels on the left
    let x_label_space = if x_label.is_some() { 30.0 * scale } else { 0.0 };
    let y_label_space = if y_label.is_some() { 30.0 * scale } else { 0.0 };
    let chart_left = pos.x + padding + label_area + y_label_space;
    let chart_width = max_width - padding * 2.0 - label_area - value_area - y_label_space;
    let chart_top = pos.y + padding;
    let chart_height = height - padding * 2.0 - x_label_space;

    // Axis line (vertical)
    let axis_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_AXIS);
    painter.line_segment(
        [
            Pos2::new(chart_left, chart_top),
            Pos2::new(chart_left, chart_top + chart_height),
        ],
        Stroke::new(VIZ_STROKE_AXIS * scale, axis_color),
    );

    // Bars
    let bar_gap = 10.0 * scale;
    let total_gaps = (n + 1) as f32 * bar_gap;
    let bar_height = ((chart_height - total_gaps) / n as f32).max(8.0 * scale);
    let value_font = FontId::proportional(theme.body_size * VIZ_FONT_VALUE_LABEL * scale);

    for (i, entry) in entries.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }

        let (anim, repaint) = reveal_anim_progress(step, reveal_step, reveal_timestamp);
        if repaint {
            needs_repaint = true;
        }

        let color = Theme::with_opacity(palette[i % palette.len()], opacity * VIZ_OPACITY_FILL);
        // Negative values are clamped to the axis so nothing draws left of it
        let full_bar_w = (entry.value.max(0.0) / max_value) * chart_width;
        let bar_w = full_bar_w * anim;
        let by = chart_top + bar_gap + i as f32 * (bar_height + bar_gap);

        // Bar with rounded corners
        let bar_rect =
            egui::Rect::from_min_size(Pos2::new(chart_left, by), egui::vec2(bar_w, bar_height));
        painter.rect_filled(bar_rect, VIZ_CORNER_BAR * scale, color);

        // Category label on the left
        let galley = label_galleys[i].clone();
        let lx = chart_left - galley.rect.width() - label_gap;
        let ly = by + (bar_height - galley.rect.height()) / 2.0;
        painter.galley(Pos2::new(lx, ly), galley, label_color);

        // Value label to the right of bar (fade in near end of animation)
        if anim > VIZ_LABEL_REVEAL_THRESHOLD {
            let val_text = format_value(entry.value);
            let val_color = Theme::with_opacity(theme.foreground, opacity * 0.7 * label_fade(anim));
            let val_galley = painter.layout_no_wrap(val_text, value_font.clone(), val_color);
            let vx = chart_left + bar_w + 8.0 * scale;
            let vy = by + (bar_height - val_galley.rect.height()) / 2.0;
            painter.galley(Pos2::new(vx, vy), val_galley, val_color);
        }
    }

    // Axis labels
    let axis_label_font = FontId::proportional(theme.body_size * VIZ_FONT_AXIS_LABEL * scale);
    let axis_label_color = Theme::with_opacity(theme.foreground, opacity * 0.7);
    if let Some(text) = x_label {
        draw_x_axis_label(
            painter,
            text,
            axis_label_font.clone(),
            axis_label_color,
            chart_left,
            chart_width,
            chart_top + chart_height + 10.0 * scale,
        );
    }
    if let Some(text) = y_label {
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

    needs_repaint
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bar_chart_basic() {
        let content = "- Sales: 40\n- Costs: 25";
        let data = parse_bar_chart(content);
        assert_eq!(data.entries.len(), 2);
        assert_eq!(data.entries[0].label, "Sales");
        assert_eq!(data.entries[0].value, 40.0);
        assert_eq!(data.orientation, Orientation::Vertical);
    }

    #[test]
    fn test_parse_bar_chart_horizontal() {
        let content = "# orientation: horizontal\n- A: 10\n- B: 20";
        let data = parse_bar_chart(content);
        assert_eq!(data.entries.len(), 2);
        assert_eq!(data.orientation, Orientation::Horizontal);
    }

    #[test]
    fn test_parse_bar_chart_percentage_suffix() {
        let content = "- A: 40%\n- B: 60%";
        let data = parse_bar_chart(content);
        assert_eq!(data.entries[0].value, 40.0);
        assert_eq!(data.entries[1].value, 60.0);
    }

    #[test]
    fn test_parse_bar_chart_reveal_markers() {
        let content = "- A: 10\n+ B: 20\n* C: 30";
        let data = parse_bar_chart(content);
        assert_eq!(data.entries[0].reveal, VizReveal::Static);
        assert_eq!(data.entries[1].reveal, VizReveal::NextStep);
        assert_eq!(data.entries[2].reveal, VizReveal::WithPrev);
    }

    #[test]
    fn test_parse_bar_chart_skips_invalid() {
        let content = "- Valid: 50\n- no_value\n# comment\n- Also: 30";
        let data = parse_bar_chart(content);
        assert_eq!(data.entries.len(), 2);
    }

    #[test]
    fn test_parse_bar_chart_decimal_values() {
        let content = "- A: 3.25\n- B: 2.71";
        let data = parse_bar_chart(content);
        assert!((data.entries[0].value - 3.25).abs() < 0.001);
        assert!((data.entries[1].value - 2.71).abs() < 0.001);
    }

    #[test]
    fn test_parse_bar_chart_axis_labels() {
        let content = "# x-label: Categories\n# y-label: Revenue ($M)\n- A: 10\n- B: 20";
        let data = parse_bar_chart(content);
        assert_eq!(data.x_label, Some("Categories".to_string()));
        assert_eq!(data.y_label, Some("Revenue ($M)".to_string()));
        assert_eq!(data.entries.len(), 2);
    }

    #[test]
    fn test_parse_bar_chart_rejects_non_finite() {
        let data = parse_bar_chart("- A: inf\n- B: nan\n- C: -infinity\n- D: 5");
        assert_eq!(data.entries.len(), 1);
        assert_eq!(data.entries[0].label, "D");
    }

    #[test]
    fn test_parse_bar_chart_decorated_values() {
        let data = parse_bar_chart("- A: 1,000\n- B: $40\n- C: 40 units\n- D: 12%");
        let values: Vec<f32> = data.entries.iter().map(|e| e.value).collect();
        assert_eq!(values, vec![1000.0, 40.0, 40.0, 12.0]);
    }

    #[test]
    fn test_grid_lines_are_bounded() {
        // Even a pathological max/step pair must terminate with a bounded number of lines
        let lines = grid_values(1.0e30, 1.0);
        assert!(lines.len() <= super::super::VIZ_MAX_GRID_LINES);
        let lines = grid_values(100.0, 20.0);
        assert_eq!(lines, vec![20.0, 40.0, 60.0, 80.0, 100.0]);
    }
}
