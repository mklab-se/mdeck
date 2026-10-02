use eframe::egui::{self, Color32, FontId, Pos2};

use crate::theme::Theme;

use super::{
    AxisTitles, PlotFrame, VIZ_CORNER_BAR, VIZ_FONT_CATEGORY_LABEL, VIZ_FONT_GRID_LABEL,
    VIZ_FONT_MIN, VIZ_FONT_VALUE_LABEL, VIZ_LABEL_REVEAL_THRESHOLD, VIZ_OPACITY_LABEL, ValueRange,
    VizCtx, VizReveal, assign_steps, fit_font_size, fit_text, format_axis_value, format_value,
    grammar::{LabelValue, Problem, Source, label_value_items},
    grid_values, label_fade, nice_axis_max, nice_grid_step,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Orientation {
    Vertical,
    Horizontal,
}

type BarEntry = LabelValue;

struct BarChartData {
    entries: Vec<BarEntry>,
    orientation: Orientation,
    x_label: Option<String>,
    y_label: Option<String>,
}

const SETTINGS: &[&str] = &["orientation", "x-label", "y-label"];

fn read(src: &Source) -> BarChartData {
    src.check_settings(SETTINGS);
    let orientation = match src.choice("orientation", &["vertical", "horizontal"]) {
        Some("horizontal") => Orientation::Horizontal,
        _ => Orientation::Vertical,
    };
    BarChartData {
        entries: label_value_items(src, "- Sales: 40"),
        orientation,
        x_label: src.setting("x-label").map(str::to_string),
        y_label: src.setting("y-label").map(str::to_string),
    }
}

fn parse_bar_chart(content: &str) -> BarChartData {
    read(&Source::parse(content))
}

/// The problems in a `@bar` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
}

// ─── Layout ─────────────────────────────────────────────────────────────────

/// Where a bar chart's parts go.
#[derive(Debug, Clone, Copy, PartialEq)]
struct BarLayout {
    frame: PlotFrame,
    titles_x_top: f32,
    titles_y_left: f32,
}

/// Vertical bars: room on the left for grid values `grid_label_w` wide (and
/// the y title), above the plot for value labels and below it for category
/// labels (and the x title).
fn vertical_layout(
    pos: Pos2,
    max_width: f32,
    height: f32,
    scale: f32,
    grid_label_w: f32,
    titles: (bool, bool),
) -> BarLayout {
    let (has_x_title, has_y_title) = titles;
    let padding = 60.0 * scale;
    let label_area = 40.0 * scale; // space for labels below bars
    let value_area = 30.0 * scale; // space for value labels above bars
    let y_label_space = if has_y_title { 30.0 * scale } else { 0.0 };
    let x_label_space = if has_x_title { 30.0 * scale } else { 0.0 };
    let chart_height = height - padding - label_area - value_area - x_label_space;
    // Reserve room on the left for the widest grid label (plus the axis title)
    let left_inset =
        (padding * 0.3 + y_label_space + grid_label_w + 16.0 * scale).max(padding + y_label_space);
    let frame = PlotFrame::from_size(
        pos.x + left_inset,
        pos.y + padding + value_area,
        max_width - left_inset - padding,
        chart_height,
    );
    BarLayout {
        frame,
        titles_x_top: frame.bottom + label_area + 4.0 * scale,
        titles_y_left: pos.x + padding * 0.3,
    }
}

/// Gap between and width of `n` vertical bars across `chart_width`. The gap
/// grows with the bars so they read as distinct columns.
fn vertical_slots(chart_width: f32, n: usize, scale: f32) -> (f32, f32) {
    let bar_gap = (chart_width / n as f32 * 0.22).clamp(10.0 * scale, 48.0 * scale);
    let total_gaps = (n + 1) as f32 * bar_gap;
    let bar_width = ((chart_width - total_gaps) / n as f32).max(8.0 * scale);
    (bar_gap, bar_width)
}

/// Horizontal bars: category labels `label_area` wide on the left (after the
/// y title), value labels on the right, the x title below.
fn horizontal_layout(
    pos: Pos2,
    max_width: f32,
    height: f32,
    scale: f32,
    label_area: f32,
    titles: (bool, bool),
) -> BarLayout {
    let (has_x_title, has_y_title) = titles;
    let padding = 40.0 * scale;
    let value_area = 60.0 * scale; // space for value labels on the right
    let x_label_space = if has_x_title { 30.0 * scale } else { 0.0 };
    let y_label_space = if has_y_title { 30.0 * scale } else { 0.0 };
    let frame = PlotFrame::from_size(
        pos.x + padding + label_area + y_label_space,
        pos.y + padding,
        max_width - padding * 2.0 - label_area - value_area - y_label_space,
        height - padding * 2.0 - x_label_space,
    );
    BarLayout {
        frame,
        titles_x_top: frame.bottom + 10.0 * scale,
        titles_y_left: pos.x + padding * 0.3,
    }
}

/// Height of each of `n` horizontal bars `gap` apart in `chart_height`.
fn horizontal_bar_height(chart_height: f32, n: usize, gap: f32, scale: f32) -> f32 {
    let total_gaps = (n + 1) as f32 * gap;
    ((chart_height - total_gaps) / n as f32).max(8.0 * scale)
}

// ─── Renderer ───────────────────────────────────────────────────────────────

/// What both orientations draw.
struct Bars<'a> {
    entries: &'a [BarEntry],
    steps: &'a [usize],
    palette: &'a [Color32],
    range: ValueRange,
    titles: (Option<&'a str>, Option<&'a str>),
}

impl Bars<'_> {
    fn has_titles(&self) -> (bool, bool) {
        (self.titles.0.is_some(), self.titles.1.is_some())
    }

    fn axis_titles(&self, layout: &BarLayout) -> AxisTitles<'_> {
        AxisTitles {
            x: self.titles.0,
            x_top: layout.titles_x_top,
            y: self.titles.1,
            y_left: layout.titles_y_left,
        }
    }

    /// The category labels at one shared size that fits them all in `max_w`.
    fn label_font(&self, cx: &VizCtx, max_w: f32) -> FontId {
        let label_texts: Vec<&str> = self.entries.iter().map(|e| e.label.as_str()).collect();
        let theme = cx.theme;
        let min_font = theme.body_size * VIZ_FONT_MIN * cx.scale;
        FontId::new(
            fit_font_size(
                cx.ui.painter(),
                &label_texts,
                &FontId::proportional(theme.body_size * VIZ_FONT_CATEGORY_LABEL * cx.scale),
                max_w,
                min_font,
            ),
            theme.body_family(),
        )
    }
}

pub fn draw_bar_chart(
    cx: &VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let data = parse_bar_chart(content);
    if data.entries.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * cx.scale
    };

    let reveals: Vec<VizReveal> = data.entries.iter().map(|e| e.reveal).collect();
    let steps = assign_steps(&reveals);
    let palette = cx.theme.edge_palette();

    let max_value = data.entries.iter().map(|e| e.value).fold(0.0f32, f32::max);
    if max_value <= 0.0 {
        return height;
    }
    let bars = Bars {
        entries: &data.entries,
        steps: &steps,
        palette: &palette,
        // Scale the axis to a round number so the tallest bar never touches the top
        range: ValueRange::to(nice_axis_max(max_value, 5)),
        titles: (data.x_label.as_deref(), data.y_label.as_deref()),
    };

    match data.orientation {
        Orientation::Vertical => draw_vertical(cx, &bars, pos, max_width, height),
        Orientation::Horizontal => draw_horizontal(cx, &bars, pos, max_width, height),
    }

    height
}

/// Width of the widest grid value label on an axis up to `max` in `step`s.
fn widest_grid_label(painter: &egui::Painter, font: &FontId, max: f32, step: f32) -> f32 {
    grid_values(max, step)
        .into_iter()
        .map(|v| {
            painter
                .layout_no_wrap(format_axis_value(v, step), font.clone(), Color32::WHITE)
                .rect
                .width()
        })
        .fold(0.0f32, f32::max)
}

fn draw_vertical(cx: &VizCtx, bars: &Bars, pos: Pos2, max_width: f32, height: f32) {
    let painter = cx.ui.painter();
    let scale = cx.scale;
    let range = bars.range;
    let grid_step = nice_grid_step(range.max, 5);
    let grid_label_w =
        widest_grid_label(painter, &cx.font(VIZ_FONT_GRID_LABEL), range.max, grid_step);
    let layout = vertical_layout(
        pos,
        max_width,
        height,
        scale,
        grid_label_w,
        bars.has_titles(),
    );
    let frame = layout.frame;

    frame.draw_x_axis(cx);
    // Grid lines with nice round numbers
    frame.draw_y_grid(
        cx,
        grid_values(range.max, grid_step),
        range,
        grid_step,
        false,
    );

    let (bar_gap, bar_width) = vertical_slots(frame.width, bars.entries.len(), scale);
    let value_font = cx.font(VIZ_FONT_VALUE_LABEL);
    // One label size for every category so the axis reads as a unit
    let label_font = bars.label_font(cx, bar_width + bar_gap);

    for (i, entry) in bars.entries.iter().enumerate() {
        let step = bars.steps.get(i).copied().unwrap_or(0);
        if step > cx.reveal_step {
            continue;
        }
        let anim = cx.anim(step);

        // Negative values are clamped to the axis so nothing draws below the chart
        let bar_height = range.frac(entry.value.max(0.0)) * frame.height * anim;
        let bx = frame.left + bar_gap + i as f32 * (bar_width + bar_gap);
        let by = frame.bottom - bar_height;

        let bar_rect =
            egui::Rect::from_min_size(Pos2::new(bx, by), egui::vec2(bar_width, bar_height));
        painter.rect_filled(bar_rect, VIZ_CORNER_BAR * scale, cx.fill(bars.palette, i));
        crate::render::hints::push(painter.ctx(), crate::render::hints::Hint::Bar(bar_rect));

        // Value label above bar (only show when animation is near-complete)
        if anim > VIZ_LABEL_REVEAL_THRESHOLD {
            let val_color = value_color(cx, anim);
            let val_galley =
                painter.layout_no_wrap(format_value(entry.value), value_font.clone(), val_color);
            let val_x = bx + (bar_width - val_galley.rect.width()) / 2.0;
            painter.galley(
                Pos2::new(val_x, by - val_galley.rect.height() - 4.0 * scale),
                val_galley,
                val_color,
            );
        }

        // Category label below bar, shrunk/truncated to its slot
        let label_color = cx.fg(VIZ_OPACITY_LABEL);
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
            Pos2::new(lx, frame.bottom + 6.0 * scale),
            galley,
            label_color,
        );
    }

    frame.draw_titles(cx, &bars.axis_titles(&layout));
}

/// The colour of a value label fading in as its bar finishes growing.
fn value_color(cx: &VizCtx, anim: f32) -> Color32 {
    Theme::with_opacity(cx.theme.foreground, cx.opacity * 0.7 * label_fade(anim))
}

fn draw_horizontal(cx: &VizCtx, bars: &Bars, pos: Pos2, max_width: f32, height: f32) {
    let painter = cx.ui.painter();
    let scale = cx.scale;
    let range = bars.range;
    let label_gap = 10.0 * scale;
    let label_color = cx.fg(VIZ_OPACITY_LABEL);

    // Size the label column to the longest label, fitted into at most a third of the width.
    // All labels share one font size.
    let max_label_w = max_width * 0.33;
    let label_font = bars.label_font(cx, max_label_w);
    let label_galleys: Vec<_> = bars
        .entries
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
        + label_gap;
    let layout = horizontal_layout(pos, max_width, height, scale, label_area, bars.has_titles());
    let frame = layout.frame;

    frame.draw_y_axis(cx);

    let bar_gap = 10.0 * scale;
    let bar_height = horizontal_bar_height(frame.height, bars.entries.len(), bar_gap, scale);
    let value_font = cx.font(VIZ_FONT_VALUE_LABEL);

    for (i, entry) in bars.entries.iter().enumerate() {
        let step = bars.steps.get(i).copied().unwrap_or(0);
        if step > cx.reveal_step {
            continue;
        }
        let anim = cx.anim(step);

        // Negative values are clamped to the axis so nothing draws left of it
        let bar_w = range.frac(entry.value.max(0.0)) * frame.width * anim;
        let by = frame.top + bar_gap + i as f32 * (bar_height + bar_gap);

        let bar_rect =
            egui::Rect::from_min_size(Pos2::new(frame.left, by), egui::vec2(bar_w, bar_height));
        painter.rect_filled(bar_rect, VIZ_CORNER_BAR * scale, cx.fill(bars.palette, i));
        crate::render::hints::push(painter.ctx(), crate::render::hints::Hint::Bar(bar_rect));

        // Category label on the left
        let galley = label_galleys[i].clone();
        let lx = frame.left - galley.rect.width() - label_gap;
        let ly = by + (bar_height - galley.rect.height()) / 2.0;
        painter.galley(Pos2::new(lx, ly), galley, label_color);

        // Value label to the right of bar (fade in near end of animation)
        if anim > VIZ_LABEL_REVEAL_THRESHOLD {
            let val_color = value_color(cx, anim);
            let val_galley =
                painter.layout_no_wrap(format_value(entry.value), value_font.clone(), val_color);
            let vx = frame.left + bar_w + 8.0 * scale;
            let vy = by + (bar_height - val_galley.rect.height()) / 2.0;
            painter.galley(Pos2::new(vx, vy), val_galley, val_color);
        }
    }

    frame.draw_titles(cx, &bars.axis_titles(&layout));
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
        let content = "orientation: horizontal\n- A: 10\n- B: 20";
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
        let content = "x-label: Categories\ny-label: Revenue ($M)\n- A: 10\n- B: 20";
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

    #[test]
    fn test_vertical_layout_fits_grid_labels() {
        let pos = Pos2::new(0.0, 0.0);
        // Narrow grid labels: the padding decides the inset
        let l = vertical_layout(pos, 1000.0, 500.0, 1.0, 20.0, (false, false));
        assert_eq!(l.frame.left, 60.0);
        assert_eq!(l.frame.top, 90.0);
        assert_eq!(l.frame.width, 880.0);
        assert_eq!(l.frame.height, 500.0 - 60.0 - 40.0 - 30.0);
        // Wide grid labels push the plot right
        let l = vertical_layout(pos, 1000.0, 500.0, 1.0, 100.0, (false, true));
        assert_eq!(l.frame.left, 18.0 + 30.0 + 100.0 + 16.0);
        assert_eq!(l.titles_y_left, 18.0);
    }

    #[test]
    fn test_vertical_slots_keep_gaps_in_bounds() {
        let (gap, width) = vertical_slots(1000.0, 4, 1.0);
        assert_eq!(gap, 48.0);
        assert_eq!(width, (1000.0 - 5.0 * 48.0) / 4.0);
        let (gap, width) = vertical_slots(100.0, 50, 1.0);
        assert_eq!(gap, 10.0);
        assert_eq!(width, 8.0);
    }

    #[test]
    fn test_horizontal_layout_and_bar_height() {
        let l = horizontal_layout(
            Pos2::new(0.0, 0.0),
            1000.0,
            400.0,
            1.0,
            120.0,
            (true, false),
        );
        assert_eq!(l.frame.left, 160.0);
        assert_eq!(l.frame.width, 1000.0 - 80.0 - 120.0 - 60.0);
        assert_eq!(l.frame.height, 400.0 - 80.0 - 30.0);
        assert_eq!(l.titles_x_top, l.frame.bottom + 10.0);
        assert_eq!(horizontal_bar_height(290.0, 2, 10.0, 1.0), 130.0);
        assert_eq!(horizontal_bar_height(20.0, 5, 10.0, 1.0), 8.0);
    }
}
