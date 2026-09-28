//! Axis helpers: grid steps, value formatting and axis titles.

use eframe::egui::{self, Color32, FontId, Pos2};
use eframe::epaint::TextShape;

use super::VIZ_MAX_GRID_LINES;

/// Draw a horizontal axis label centered below the chart area.
pub fn draw_x_axis_label(
    painter: &egui::Painter,
    text: &str,
    font: FontId,
    color: Color32,
    chart_left: f32,
    chart_width: f32,
    y: f32,
) {
    let galley = painter.layout_no_wrap(text.to_string(), font, color);
    let lx = chart_left + (chart_width - galley.rect.width()) / 2.0;
    painter.galley(Pos2::new(lx, y), galley, color);
}

/// Draw a vertical axis label rotated 90° CCW, centered along the chart's Y axis.
pub fn draw_y_axis_label(
    painter: &egui::Painter,
    text: &str,
    font: FontId,
    color: Color32,
    x: f32,
    chart_top: f32,
    chart_height: f32,
) {
    let galley = painter.layout_no_wrap(text.to_string(), font, color);
    let text_width = galley.rect.width();
    // Place anchor so that the rotated text is vertically centered
    // After -90° rotation around anchor, text extends upward from anchor
    let anchor_x = x;
    let anchor_y = chart_top + (chart_height + text_width) / 2.0;
    let text_shape = TextShape::new(Pos2::new(anchor_x, anchor_y), galley, color)
        .with_angle(-std::f32::consts::FRAC_PI_2);
    painter.add(text_shape);
}

/// Compute a "nice" grid step for axis labels (1, 2, 5, 10, 20, 50, 100, ...)
/// so that roughly `target_lines` grid lines cover `max_value`.
pub fn nice_grid_step(max_value: f32, target_lines: u32) -> f32 {
    if max_value <= 0.0 || !max_value.is_finite() {
        return 1.0;
    }
    let rough = max_value / target_lines.max(1) as f32;
    let magnitude = 10.0f32.powf(rough.log10().floor());
    let residual = rough / magnitude;
    let nice = if residual <= 1.0 {
        1.0
    } else if residual <= 2.0 {
        2.0
    } else if residual <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * magnitude
}

/// Grid line values `step, 2·step, …` up to and including `max_value`, capped at
/// `VIZ_MAX_GRID_LINES` entries so pathological inputs cannot loop forever.
pub fn grid_values(max_value: f32, step: f32) -> Vec<f32> {
    grid_range_values(step, max_value, step)
}

/// Grid line values from the first multiple of `step` at or above `min_value`
/// up to and including `max_value`, capped at `VIZ_MAX_GRID_LINES` entries.
pub fn grid_range_values(min_value: f32, max_value: f32, step: f32) -> Vec<f32> {
    if !step.is_finite() || step <= 0.0 || !min_value.is_finite() || !max_value.is_finite() {
        return Vec::new();
    }
    let first = (min_value / step).ceil() * step;
    let limit = max_value + step * 0.001;
    (0..VIZ_MAX_GRID_LINES)
        .map(|i| first + i as f32 * step)
        .take_while(|v| *v <= limit)
        .collect()
}

/// Round `max_value` up to the next multiple of the nice grid step so the
/// topmost grid line sits at or above the largest data value.
pub fn nice_axis_max(max_value: f32, target_lines: u32) -> f32 {
    let step = nice_grid_step(max_value, target_lines);
    let rounded = (max_value / step).ceil() * step;
    if rounded <= 0.0 { step } else { rounded }
}

/// Format an axis or value label: integers without decimals, everything else
/// with the precision implied by `step` (at most 2 decimals). Never prints "-0".
pub fn format_axis_value(value: f32, step: f32) -> String {
    let decimals = if step >= 1.0 || step <= 0.0 {
        0
    } else if step >= 0.1 {
        1
    } else {
        2
    };
    let text = format!("{value:.decimals$}");
    if text.starts_with('-') && text.trim_start_matches(['-', '0', '.']).is_empty() {
        text[1..].to_string()
    } else {
        text
    }
}

/// Format a data value for display: integers without decimals, otherwise one
/// decimal. Never prints "-0".
pub fn format_value(value: f32) -> String {
    format_axis_value(value, if value == value.floor() { 1.0 } else { 0.1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_values_bounded_and_exact() {
        assert_eq!(
            grid_values(100.0, 20.0),
            vec![20.0, 40.0, 60.0, 80.0, 100.0]
        );
        assert_eq!(grid_values(1.0e30, 1.0).len(), VIZ_MAX_GRID_LINES);
        assert!(grid_values(f32::INFINITY, 1.0).is_empty());
        assert!(grid_values(10.0, 0.0).is_empty());
        assert!(grid_values(10.0, f32::NAN).is_empty());
        assert_eq!(
            grid_range_values(-25.0, 25.0, 10.0),
            vec![-20.0, -10.0, 0.0, 10.0, 20.0]
        );
        assert_eq!(grid_range_values(0.0, 10.0, 5.0), vec![0.0, 5.0, 10.0]);
    }

    #[test]
    fn test_nice_grid_step() {
        assert_eq!(nice_grid_step(100.0, 5), 20.0);
        assert_eq!(nice_grid_step(65.0, 5), 20.0);
        assert_eq!(nice_grid_step(95.0, 5), 20.0);
        assert_eq!(nice_grid_step(50.0, 5), 10.0);
        assert_eq!(nice_grid_step(420.0, 5), 100.0);
        assert_eq!(nice_grid_step(10.0, 5), 2.0);
        assert!((nice_grid_step(0.8, 5) - 0.2).abs() < 1e-6);
        assert_eq!(nice_grid_step(0.0, 5), 1.0);
    }

    #[test]
    fn test_nice_axis_max_covers_data() {
        assert_eq!(nice_axis_max(28.0, 5), 30.0);
        assert_eq!(nice_axis_max(100.0, 5), 100.0);
        assert_eq!(nice_axis_max(65.0, 5), 80.0);
        assert_eq!(nice_axis_max(130.0, 5), 150.0);
        assert!(nice_axis_max(0.83, 5) >= 0.83);
    }

    #[test]
    fn test_format_axis_value_never_negative_zero() {
        assert_eq!(format_axis_value(-0.0, 10.0), "0");
        assert_eq!(format_axis_value(-0.04, 1.0), "0");
        assert_eq!(format_axis_value(20.0, 10.0), "20");
        assert_eq!(format_axis_value(2.5, 0.5), "2.5");
        assert_eq!(format_axis_value(0.25, 0.05), "0.25");
        assert_eq!(format_value(3.0), "3");
        assert_eq!(format_value(3.25), "3.2");
        assert_eq!(format_value(-0.0), "0");
    }
}
