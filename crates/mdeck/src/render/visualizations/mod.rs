use std::time::Instant;

use eframe::egui::{self, Color32, FontId, Pos2};
use eframe::epaint::TextShape;

pub mod bar_chart;
pub mod donut_chart;
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

const REVEAL_ANIMATION_DURATION: f32 = 0.4; // seconds

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
pub const VIZ_OPACITY_FILL: f32 = 0.85;
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

/// Opacity ramp for value labels that fade in during the last part of a reveal
/// animation: 0 at `VIZ_LABEL_REVEAL_THRESHOLD`, 1 when the animation completes.
pub fn label_fade(anim: f32) -> f32 {
    ((anim - VIZ_LABEL_REVEAL_THRESHOLD) / (1.0 - VIZ_LABEL_REVEAL_THRESHOLD)).clamp(0.0, 1.0)
}

/// Compute eased animation progress (0.0→1.0) for an element revealed at `item_step`.
/// Returns `(progress, needs_repaint)`.
pub fn reveal_anim_progress(
    item_step: usize,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
) -> (f32, bool) {
    // Only animate items that just appeared on the current step
    if item_step == reveal_step
        && item_step > 0
        && let Some(ts) = reveal_timestamp
    {
        let elapsed = ts.elapsed().as_secs_f32();
        let t = (elapsed / REVEAL_ANIMATION_DURATION).min(1.0);
        // Ease-in-out quadratic
        let eased = if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0_f32 * t + 2.0).powi(2) / 2.0
        };
        return (eased, t < 1.0);
    }
    (1.0, false)
}

/// Reveal marker for visualization elements (mirrors diagram semantics).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VizReveal {
    /// Always visible (prefix `-` or no prefix).
    Static,
    /// Appears on the next reveal step (prefix `+`).
    NextStep,
    /// Appears together with the previous `+` element (prefix `*`).
    WithPrev,
}

/// Parse a line's reveal prefix, returning the trimmed content and its reveal marker.
pub fn parse_reveal_prefix(line: &str) -> (&str, VizReveal) {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("+ ") {
        (rest, VizReveal::NextStep)
    } else if let Some(rest) = trimmed.strip_prefix("* ") {
        (rest, VizReveal::WithPrev)
    } else if let Some(rest) = trimmed.strip_prefix("- ") {
        (rest, VizReveal::Static)
    } else {
        (trimmed, VizReveal::Static)
    }
}

/// Count the number of `+` (NextStep) markers in a visualization content string.
pub fn count_viz_steps(content: &str) -> usize {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#') && trimmed.starts_with("+ ")
        })
        .count()
}

/// Assign reveal step numbers to items based on their reveal markers.
/// Returns a Vec of step numbers (0 = always visible).
pub fn assign_steps(reveals: &[VizReveal]) -> Vec<usize> {
    let mut step_counter = 0usize;
    reveals
        .iter()
        .map(|r| match r {
            VizReveal::Static => 0,
            VizReveal::NextStep => {
                step_counter += 1;
                step_counter
            }
            VizReveal::WithPrev => step_counter,
        })
        .collect()
}

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

/// Parse an axis label directive from a comment line.
/// Returns Some((key, value)) for lines like "# x-label: Foo" or "# y-label: Bar".
pub fn parse_axis_label_directive(trimmed: &str) -> Option<(&str, String)> {
    for key in &["x-label", "y-label"] {
        let prefixed = format!("# {key}:");
        let compact = format!("#{key}:");
        if let Some(rest) = trimmed.strip_prefix(prefixed.as_str()) {
            return Some((key, rest.trim().to_string()));
        }
        if let Some(rest) = trimmed.strip_prefix(compact.as_str()) {
            return Some((key, rest.trim().to_string()));
        }
    }
    None
}

// ─── Value parsing ──────────────────────────────────────────────────────────

/// Parse a single numeric value as written in presentation markdown.
///
/// Accepts plain numbers (`40`, `3.5`, `-2`, `1e3`), currency prefixes (`$40`,
/// `€40`, `£40`), a `%` suffix (`12%`), `_` digit separators (`1_000`),
/// thousands separators in groups of three (`1,000`, `12,345.5`), and a
/// trailing unit made of letters (`40 units`, `4.2M` — the unit is dropped,
/// not scaled). Returns `None` for anything else and for non-finite numbers
/// (`inf`, `nan`), which would otherwise make axis loops run forever.
pub fn parse_value(raw: &str) -> Option<f32> {
    let s = raw.trim();
    let s = s.trim_start_matches(['$', '€', '£']).trim_start();
    let s = s.trim_end_matches('%').trim_end();
    let cleaned: String = s.chars().filter(|&c| c != '_').collect();
    if cleaned.is_empty() {
        return None;
    }

    // Fast path: a plain number (also covers scientific notation).
    if let Ok(v) = cleaned.parse::<f32>() {
        return v.is_finite().then_some(v);
    }

    // Split into a numeric prefix and an optional alphabetic unit suffix.
    let split = cleaned
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+')))
        .map_or(cleaned.len(), |(i, _)| i);
    let (number, unit) = cleaned.split_at(split);
    if !unit.chars().all(|c| c.is_alphabetic() || c.is_whitespace()) {
        return None;
    }

    let number = if number.contains(',') {
        if !is_thousands_grouped(number) {
            return None;
        }
        number.replace(',', "")
    } else {
        number.to_string()
    };
    number.parse::<f32>().ok().filter(|v| v.is_finite())
}

/// True when `s` is a number whose commas are all thousands separators:
/// an optional sign, 1-3 digits, then groups of exactly three digits,
/// optionally followed by a decimal fraction.
fn is_thousands_grouped(s: &str) -> bool {
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s, None),
    };
    if let Some(f) = frac_part
        && (f.is_empty() || !f.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    let mut groups = int_part.split(',');
    let Some(first) = groups.next() else {
        return false;
    };
    if first.is_empty() || first.len() > 3 || !first.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let mut any = false;
    for g in groups {
        if g.len() != 3 || !g.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        any = true;
    }
    any
}

/// Remove thousands separators from a comma-separated list of values.
///
/// A comma directly followed by exactly three digits (`1,000`) is only treated
/// as a thousands separator when the list also uses `", "` (comma + space) to
/// separate its items, i.e. `1,000, 2,000`. A list written without spaces
/// (`100,200,300`) keeps every comma as an item separator.
pub fn strip_thousands_separators(list: &str) -> String {
    if !list.contains(", ") {
        return list.to_string();
    }
    let bytes = list.as_bytes();
    let mut out = String::with_capacity(list.len());
    for (i, &b) in bytes.iter().enumerate() {
        if b == b','
            && i > 0
            && bytes[i - 1].is_ascii_digit()
            && bytes.len() >= i + 4
            && bytes[i + 1..i + 4].iter().all(u8::is_ascii_digit)
            && !bytes.get(i + 4).is_some_and(u8::is_ascii_digit)
        {
            continue;
        }
        out.push(b as char);
    }
    out
}

/// Split a `"Label: value"` item into its label and numeric value.
/// Returns `None` when there is no `": "` or the value does not parse.
pub fn parse_label_value(text: &str) -> Option<(String, f32)> {
    let (label, value) = text.split_once(": ")?;
    let value = parse_value(value)?;
    Some((label.trim().to_string(), value))
}

/// Split a `"Label: v1, v2, v3"` item into its label and the values that parse.
/// Returns `None` when there is no `": "` or no value parses.
pub fn parse_label_values(text: &str) -> Option<(String, Vec<f32>)> {
    let (label, values) = text.split_once(": ")?;
    let values: Vec<f32> = strip_thousands_separators(values)
        .split(',')
        .filter_map(parse_value)
        .collect();
    if values.is_empty() {
        return None;
    }
    Some((label.trim().to_string(), values))
}

// ─── Text fitting ───────────────────────────────────────────────────────────

/// Lay out `text` so it fits within `max_width`: first shrink the font down to
/// `min_font_size`, then truncate with an ellipsis if it still does not fit.
pub fn fit_text(
    painter: &egui::Painter,
    text: &str,
    font: FontId,
    color: Color32,
    max_width: f32,
    min_font_size: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut font = font;
    let mut galley = painter.layout_no_wrap(text.to_string(), font.clone(), color);
    if max_width <= 0.0 || galley.rect.width() <= max_width {
        return galley;
    }

    // Shrink proportionally in one go, then nudge down until it fits.
    let min_font_size = min_font_size.min(font.size);
    font.size = (font.size * max_width / galley.rect.width()).max(min_font_size);
    galley = painter.layout_no_wrap(text.to_string(), font.clone(), color);
    while galley.rect.width() > max_width && font.size > min_font_size {
        font.size = (font.size - 0.5).max(min_font_size);
        galley = painter.layout_no_wrap(text.to_string(), font.clone(), color);
    }
    if galley.rect.width() <= max_width {
        return galley;
    }

    // Truncate: binary search the longest prefix that fits with an ellipsis.
    let chars: Vec<char> = text.chars().collect();
    let candidate = |n: usize| -> String {
        let prefix: String = chars[..n].iter().collect();
        format!("{}…", prefix.trim_end())
    };
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let g = painter.layout_no_wrap(candidate(mid), font.clone(), color);
        if g.rect.width() <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    painter.layout_no_wrap(candidate(lo), font.clone(), color)
}

/// How many labels to skip between drawn labels so that labels `label_width`
/// wide do not overlap when their slots are `slot_width` apart. Returns 1 when
/// every label fits.
pub fn label_stride(label_width: f32, slot_width: f32) -> usize {
    if slot_width <= 0.0 || label_width <= slot_width {
        return 1;
    }
    (label_width / slot_width).ceil().max(1.0) as usize
}

// ─── Legends ────────────────────────────────────────────────────────────────

/// One row of a vertical legend.
pub struct LegendItem {
    pub label: String,
    pub color: Color32,
    /// Hidden rows keep their slot so earlier rows do not shift during a reveal.
    pub visible: bool,
}

/// Row height range for `draw_legend_column` (multiplied by scale).
const LEGEND_ROW_MAX: f32 = 48.0;
const LEGEND_ROW_MIN: f32 = 30.0;

/// Draw a vertical legend (colour swatch + label per row), centred vertically
/// in the `height` available. Rows shrink towards a minimum height, then spill
/// into a second column; whatever still does not fit is clipped. Labels are
/// shrunk/truncated to the column width so they never overflow the slide.
#[allow(clippy::too_many_arguments)]
pub fn draw_legend_column(
    painter: &egui::Painter,
    items: &[LegendItem],
    theme: &crate::theme::Theme,
    opacity: f32,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    scale: f32,
) {
    if items.is_empty() || width <= 0.0 || height <= 0.0 {
        return;
    }
    let n = items.len();
    let row_min = LEGEND_ROW_MIN * scale;
    let rows_fit = ((height / row_min).floor() as usize).max(1);
    let columns = if n <= rows_fit { 1 } else { 2 };
    let rows_per_col = n.div_ceil(columns).min(rows_fit);
    let row_h = (height / rows_per_col as f32).clamp(row_min, LEGEND_ROW_MAX * scale);
    let col_w = width / columns as f32;
    let start_y = top + (height - rows_per_col as f32 * row_h) / 2.0;

    let swatch = VIZ_SWATCH_SIZE * scale;
    let gap = 10.0 * scale;
    let font = FontId::proportional(theme.body_size * VIZ_FONT_LEGEND * scale);
    let min_font = theme.body_size * VIZ_FONT_MIN * scale;
    let text_color = crate::theme::Theme::with_opacity(theme.foreground, opacity);
    let text_max_w = col_w - swatch - gap - 6.0 * scale;

    for (i, item) in items.iter().enumerate() {
        let col = i / rows_per_col;
        if col >= columns {
            break; // clipped: no room left
        }
        if !item.visible {
            continue;
        }
        let row = i % rows_per_col;
        let x = left + col as f32 * col_w;
        let y = start_y + row as f32 * row_h;

        let swatch_rect = egui::Rect::from_min_size(
            Pos2::new(x, y + (row_h - swatch) / 2.0),
            egui::vec2(swatch, swatch),
        );
        painter.rect_filled(swatch_rect, VIZ_CORNER_SWATCH * scale, item.color);

        let galley = fit_text(
            painter,
            &item.label,
            font.clone(),
            text_color,
            text_max_w,
            min_font,
        );
        let text_y = y + (row_h - galley.rect.height()) / 2.0;
        painter.galley(Pos2::new(x + swatch + gap, text_y), galley, text_color);
    }
}

/// Width of the legend column beside a pie/donut chart: a fixed width that
/// shrinks when the chart itself is narrow.
pub fn side_legend_width(max_width: f32, scale: f32) -> f32 {
    (380.0 * scale).min(max_width * 0.45)
}

// ─── Axis helpers ───────────────────────────────────────────────────────────

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

    #[test]
    fn test_parse_reveal_prefix() {
        assert_eq!(parse_reveal_prefix("- foo"), ("foo", VizReveal::Static));
        assert_eq!(parse_reveal_prefix("+ bar"), ("bar", VizReveal::NextStep));
        assert_eq!(parse_reveal_prefix("* baz"), ("baz", VizReveal::WithPrev));
        assert_eq!(parse_reveal_prefix("plain"), ("plain", VizReveal::Static));
    }

    #[test]
    fn test_count_viz_steps() {
        let content = "- A\n+ B\n+ C\n* D";
        assert_eq!(count_viz_steps(content), 2);
    }

    #[test]
    fn test_count_viz_steps_skips_comments() {
        let content = "# comment\n+ A\n# another\n+ B";
        assert_eq!(count_viz_steps(content), 2);
    }

    #[test]
    fn test_assign_steps() {
        let reveals = vec![
            VizReveal::Static,
            VizReveal::NextStep,
            VizReveal::NextStep,
            VizReveal::WithPrev,
            VizReveal::NextStep,
        ];
        assert_eq!(assign_steps(&reveals), vec![0, 1, 2, 2, 3]);
    }

    #[test]
    fn test_parse_value_plain_and_decorated() {
        assert_eq!(parse_value("40"), Some(40.0));
        assert_eq!(parse_value(" 3.5 "), Some(3.5));
        assert_eq!(parse_value("-2"), Some(-2.0));
        assert_eq!(parse_value("1e3"), Some(1000.0));
        assert_eq!(parse_value("12%"), Some(12.0));
        assert_eq!(parse_value("$40"), Some(40.0));
        assert_eq!(parse_value("€ 40"), Some(40.0));
        assert_eq!(parse_value("£1,250.5"), Some(1250.5));
        assert_eq!(parse_value("1_000"), Some(1000.0));
        assert_eq!(parse_value("40 units"), Some(40.0));
        assert_eq!(parse_value("4.2M"), Some(4.2));
        assert_eq!(parse_value("1,000"), Some(1000.0));
        assert_eq!(parse_value("1,000,000"), Some(1_000_000.0));
    }

    #[test]
    fn test_parse_value_rejects_garbage_and_non_finite() {
        assert_eq!(parse_value("inf"), None);
        assert_eq!(parse_value("-infinity"), None);
        assert_eq!(parse_value("nan"), None);
        assert_eq!(parse_value("NaN"), None);
        assert_eq!(parse_value("1e40"), None);
        assert_eq!(parse_value(""), None);
        assert_eq!(parse_value("abc"), None);
        assert_eq!(parse_value("1,00"), None);
        assert_eq!(parse_value("1,0000"), None);
        assert_eq!(parse_value("12,34.5"), None);
        assert_eq!(parse_value("40 (size: 3)"), None);
        assert_eq!(parse_value("1.2.3"), None);
    }

    #[test]
    fn test_strip_thousands_separators_only_with_spaced_list() {
        assert_eq!(strip_thousands_separators("1,000, 2,000"), "1000, 2000");
        assert_eq!(
            strip_thousands_separators("$1,000, $2,500.75"),
            "$1000, $2500.75"
        );
        assert_eq!(strip_thousands_separators("1,000,000, 5"), "1000000, 5");
        assert_eq!(strip_thousands_separators("10, 20, 30"), "10, 20, 30");
        // No comma+space anywhere: every comma separates items
        assert_eq!(strip_thousands_separators("100,200,300"), "100,200,300");
        assert_eq!(strip_thousands_separators("1,000"), "1,000");
        // Four digits after the comma is not a group
        assert_eq!(strip_thousands_separators("1,0000, 2"), "1,0000, 2");
    }

    #[test]
    fn test_parse_label_value() {
        assert_eq!(parse_label_value("Sales: 40"), Some(("Sales".into(), 40.0)));
        assert_eq!(
            parse_label_value("Revenue: $1,000"),
            Some(("Revenue".into(), 1000.0))
        );
        assert_eq!(
            parse_label_value("Share: 12%"),
            Some(("Share".into(), 12.0))
        );
        assert_eq!(
            parse_label_value("Load: 40 units"),
            Some(("Load".into(), 40.0))
        );
        assert_eq!(parse_label_value("Bad: inf"), None);
        assert_eq!(parse_label_value("no colon"), None);
        assert_eq!(parse_label_value("Empty: "), None);
    }

    #[test]
    fn test_parse_label_values() {
        assert_eq!(
            parse_label_values("Revenue: 1,000, 2,000"),
            Some(("Revenue".into(), vec![1000.0, 2000.0]))
        );
        assert_eq!(
            parse_label_values("S: 100,200,300"),
            Some(("S".into(), vec![100.0, 200.0, 300.0]))
        );
        assert_eq!(
            parse_label_values("Costs: $80, $90, $120"),
            Some(("Costs".into(), vec![80.0, 90.0, 120.0]))
        );
        // Non-finite entries are dropped rather than poisoning the series
        assert_eq!(
            parse_label_values("X: 1, inf, 3"),
            Some(("X".into(), vec![1.0, 3.0]))
        );
        assert_eq!(parse_label_values("X: inf, nan"), None);
        assert_eq!(parse_label_values("no values"), None);
    }

    #[test]
    fn test_label_fade() {
        assert_eq!(label_fade(0.0), 0.0);
        assert_eq!(label_fade(VIZ_LABEL_REVEAL_THRESHOLD), 0.0);
        assert_eq!(label_fade(1.0), 1.0);
        assert_eq!(label_fade(1.5), 1.0);
        let mid = label_fade((VIZ_LABEL_REVEAL_THRESHOLD + 1.0) / 2.0);
        assert!((mid - 0.5).abs() < 1e-5);
    }

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
    fn test_side_legend_width_shrinks_for_narrow_charts() {
        assert_eq!(side_legend_width(1800.0, 1.0), 380.0);
        assert_eq!(side_legend_width(600.0, 1.0), 270.0);
    }

    #[test]
    fn test_label_stride() {
        assert_eq!(label_stride(40.0, 100.0), 1);
        assert_eq!(label_stride(100.0, 100.0), 1);
        assert_eq!(label_stride(101.0, 100.0), 2);
        assert_eq!(label_stride(250.0, 100.0), 3);
        assert_eq!(label_stride(50.0, 0.0), 1);
    }

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

    #[test]
    fn test_fit_text_keeps_short_text_unchanged() {
        with_test_painter(|painter| {
            let font = FontId::proportional(20.0);
            let g = fit_text(painter, "Short", font, Color32::WHITE, 500.0, 10.0);
            assert_eq!(g.text(), "Short");
            assert_eq!(g.job.sections[0].format.font_id.size, 20.0);
        });
    }

    #[test]
    fn test_fit_text_shrinks_then_truncates() {
        with_test_painter(|painter| {
            let font = FontId::proportional(20.0);
            let text = "A fairly long category label";
            let full = painter.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE);
            let full_w = full.rect.width();

            // Mild overflow: shrinks the font, keeps the full text
            let g = fit_text(
                painter,
                text,
                font.clone(),
                Color32::WHITE,
                full_w * 0.8,
                10.0,
            );
            assert_eq!(g.text(), text);
            assert!(g.rect.width() <= full_w * 0.8 + 0.01);
            let size = g.job.sections[0].format.font_id.size;
            assert!((10.0..20.0).contains(&size));

            // Severe overflow: hits the floor and truncates with an ellipsis
            let g = fit_text(painter, text, font, Color32::WHITE, full_w * 0.3, 16.0);
            assert!(g.text().ends_with('…'));
            assert!(g.text().len() < text.len());
            assert!(g.rect.width() <= full_w * 0.3 + 0.01);
            assert_eq!(g.job.sections[0].format.font_id.size, 16.0);
        });
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
