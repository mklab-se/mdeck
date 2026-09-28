use eframe::egui::{self, Color32, Pos2};

use crate::theme::Theme;

use super::{
    AxisTitles, PlotFrame, VIZ_CORNER_BAR, VIZ_FONT_CATEGORY_LABEL, VIZ_FONT_VALUE_LABEL,
    VIZ_LABEL_REVEAL_THRESHOLD, VIZ_OPACITY_LABEL, ValueRange, VizCtx, VizReveal, assign_steps,
    draw_legend_row, format_value, grid_values, header_directive, label_fade, nice_axis_max,
    nice_grid_step, parse_label_values, parse_reveal_prefix,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct StackedSeries {
    label: String,
    values: Vec<f32>,
    reveal: VizReveal,
}

#[derive(Debug, Clone)]
struct StackedBarData {
    categories: Vec<String>,
    series: Vec<StackedSeries>,
    x_label: Option<String>,
    y_label: Option<String>,
}

fn parse_stacked_bar(content: &str) -> StackedBarData {
    let mut categories = Vec::new();
    let mut series = Vec::new();
    let mut x_label = None;
    let mut y_label = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse directives
        if trimmed.starts_with('#') {
            match header_directive(trimmed) {
                Some(("categories", rest)) => {
                    categories = rest.split(',').map(|s| s.trim().to_string()).collect();
                }
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

        // Parse "Series Name: v1, v2, v3, ..."
        if let Some((label, values)) = parse_label_values(text) {
            series.push(StackedSeries {
                label,
                values,
                reveal,
            });
        }
    }

    // Without a `# categories:` directive, number the columns after the longest series
    if categories.is_empty() {
        let n = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        categories = (1..=n).map(|i| i.to_string()).collect();
    }

    StackedBarData {
        categories,
        series,
        x_label,
        y_label,
    }
}

// ─── Layout ─────────────────────────────────────────────────────────────────

/// Corner rounding for one stacked segment: only the topmost segment of a stack
/// gets rounded (top) corners, so there are no notches between segments.
fn segment_corner_radius(radius: f32, is_top: bool) -> egui::CornerRadius {
    if is_top {
        egui::CornerRadius {
            nw: radius as u8,
            ne: radius as u8,
            sw: 0,
            se: 0,
        }
    } else {
        egui::CornerRadius::ZERO
    }
}

/// The tallest stack: the largest sum over a category of its values
/// (negatives count as 0).
fn max_stack(series: &[StackedSeries], num_categories: usize) -> f32 {
    (0..num_categories)
        .map(|ci| {
            series
                .iter()
                .map(|s| s.values.get(ci).copied().unwrap_or(0.0).max(0.0))
                .sum::<f32>()
        })
        .fold(0.0f32, f32::max)
}

/// Where the stacked bar chart's parts go.
#[derive(Debug, Clone, Copy, PartialEq)]
struct StackedLayout {
    frame: PlotFrame,
    titles_x_top: f32,
    titles_y_left: f32,
    /// The legend row across the top.
    legend_top: f32,
    legend_height: f32,
}

/// The legend row on top, grid values and the y title on the left, category
/// labels and the x title below.
fn stacked_layout(
    pos: Pos2,
    max_width: f32,
    height: f32,
    scale: f32,
    titles: (bool, bool),
) -> StackedLayout {
    let (has_x_title, has_y_title) = titles;
    let padding = 60.0 * scale;
    let legend_height = 40.0 * scale;
    let label_area = 40.0 * scale; // space for category labels below bars
    let y_axis_width = 50.0 * scale;
    let y_label_space = if has_y_title { 25.0 * scale } else { 0.0 };
    let x_label_space = if has_x_title { 30.0 * scale } else { 0.0 };
    let frame = PlotFrame::from_size(
        pos.x + padding + y_axis_width + y_label_space,
        pos.y + legend_height + padding / 2.0,
        max_width - padding * 2.0 - y_axis_width - y_label_space,
        height - legend_height - padding - label_area - x_label_space,
    );
    StackedLayout {
        frame,
        titles_x_top: frame.bottom + label_area + 4.0 * scale,
        titles_y_left: pos.x + padding * 0.3,
        legend_top: pos.y + padding / 4.0,
        legend_height,
    }
}

/// Gap between and width of `n` stacks across `chart_width`.
fn stack_slots(chart_width: f32, n: usize, scale: f32) -> (f32, f32) {
    let bar_gap = 12.0 * scale;
    let total_gaps = (n + 1) as f32 * bar_gap;
    let bar_width = ((chart_width - total_gaps) / n as f32).max(8.0 * scale);
    (bar_gap, bar_width)
}

// ─── Renderer ───────────────────────────────────────────────────────────────

pub fn draw_stacked_bar(
    cx: &VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let scale = cx.scale;
    let data = parse_stacked_bar(content);
    if data.series.is_empty() || data.categories.is_empty() {
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

    let num_categories = data.categories.len();
    let max_stack = max_stack(&data.series, num_categories);
    if max_stack <= 0.0 {
        return height;
    }
    // Scale the axis to a round number so the tallest stack never touches the top
    let range = ValueRange::to(nice_axis_max(max_stack, 5));

    let titles = (data.x_label.is_some(), data.y_label.is_some());
    let layout = stacked_layout(pos, max_width, height, scale, titles);
    let frame = layout.frame;

    frame.draw_x_axis(cx);
    let grid_step = nice_grid_step(range.max, 5);
    frame.draw_y_grid(
        cx,
        grid_values(range.max, grid_step),
        range,
        grid_step,
        false,
    );

    let (bar_gap, bar_width) = stack_slots(frame.width, num_categories, scale);
    let slot_x = |ci: usize| frame.left + bar_gap + ci as f32 * (bar_width + bar_gap);

    // Category labels below bars
    let painter = cx.ui.painter();
    let label_font = cx.font(VIZ_FONT_CATEGORY_LABEL);
    let label_color = cx.fg(VIZ_OPACITY_LABEL);
    for (ci, cat_name) in data.categories.iter().enumerate() {
        let bx = slot_x(ci);
        let galley = painter.layout(
            cat_name.clone(),
            label_font.clone(),
            label_color,
            bar_width + bar_gap,
        );
        let lx = bx + (bar_width - galley.rect.width()) / 2.0;
        painter.galley(
            Pos2::new(lx, frame.bottom + 6.0 * scale),
            galley,
            label_color,
        );
    }

    let stacks = Stacks {
        series: &data.series,
        steps: &steps,
        palette: &palette,
        range,
        frame,
        bar_width,
    };
    for ci in 0..num_categories {
        stacks.draw(cx, ci, slot_x(ci));
    }

    frame.draw_titles(
        cx,
        &AxisTitles {
            x: data.x_label.as_deref(),
            x_top: layout.titles_x_top,
            y: data.y_label.as_deref(),
            y_left: layout.titles_y_left,
        },
    );

    // Legend across the top
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

/// The segments of every category's stack.
struct Stacks<'a> {
    series: &'a [StackedSeries],
    steps: &'a [usize],
    palette: &'a [Color32],
    range: ValueRange,
    frame: PlotFrame,
    bar_width: f32,
}

impl Stacks<'_> {
    /// Category `ci`'s stack at `bx`, bottom segment first.
    fn draw(&self, cx: &VizCtx, ci: usize, bx: f32) {
        let painter = cx.ui.painter();
        let scale = cx.scale;
        let bar_width = self.bar_width;
        let value_font = cx.font(VIZ_FONT_VALUE_LABEL);
        let step_of = |si: usize| self.steps.get(si).copied().unwrap_or(0);

        // The last visible, non-empty segment is the top of this stack
        let top_series = self
            .series
            .iter()
            .enumerate()
            .filter(|(si, s)| {
                step_of(*si) <= cx.reveal_step && s.values.get(ci).copied().unwrap_or(0.0) > 0.0
            })
            .map(|(si, _)| si)
            .next_back();

        let mut cumulative_height = 0.0f32;
        for (si, series) in self.series.iter().enumerate() {
            let step = step_of(si);
            if step > cx.reveal_step {
                continue;
            }
            let anim = cx.anim(step);

            let val = series.values.get(ci).copied().unwrap_or(0.0).max(0.0);
            let seg_height = self.range.frac(val) * self.frame.height * anim;
            if seg_height <= 0.0 {
                continue;
            }

            let by = self.frame.bottom - cumulative_height - seg_height;
            let bar_rect =
                egui::Rect::from_min_size(Pos2::new(bx, by), egui::vec2(bar_width, seg_height));
            let corners = segment_corner_radius(VIZ_CORNER_BAR * scale, top_series == Some(si));
            painter.rect_filled(bar_rect, corners, cx.fill(self.palette, si));
            crate::render::hints::push(cx.ui.ctx(), crate::render::hints::Hint::Bar(bar_rect));

            // Value label inside segment if tall enough
            if seg_height > 18.0 * scale && anim > VIZ_LABEL_REVEAL_THRESHOLD {
                let val_color =
                    Theme::with_opacity(cx.theme.foreground, cx.opacity * 0.7 * label_fade(anim));
                let val_galley =
                    painter.layout_no_wrap(format_value(val), value_font.clone(), val_color);
                if val_galley.rect.width() < bar_width {
                    let vx = bx + (bar_width - val_galley.rect.width()) / 2.0;
                    let vy = by + (seg_height - val_galley.rect.height()) / 2.0;
                    painter.galley(Pos2::new(vx, vy), val_galley, val_color);
                }
            }

            cumulative_height += seg_height;
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stacked_bar_basic() {
        let content = "# categories: Q1, Q2, Q3\n- Product A: 40, 45, 50\n- Product B: 30, 35, 40";
        let data = parse_stacked_bar(content);
        assert_eq!(data.categories, vec!["Q1", "Q2", "Q3"]);
        assert_eq!(data.series.len(), 2);
        assert_eq!(data.series[0].label, "Product A");
        assert_eq!(data.series[0].values, vec![40.0, 45.0, 50.0]);
        assert_eq!(data.series[1].label, "Product B");
        assert_eq!(data.series[1].values, vec![30.0, 35.0, 40.0]);
    }

    #[test]
    fn test_parse_stacked_bar_reveal_markers() {
        let content = "# categories: A, B\n- S1: 10, 20\n+ S2: 30, 40\n* S3: 50, 60";
        let data = parse_stacked_bar(content);
        assert_eq!(data.series[0].reveal, VizReveal::Static);
        assert_eq!(data.series[1].reveal, VizReveal::NextStep);
        assert_eq!(data.series[2].reveal, VizReveal::WithPrev);
    }

    #[test]
    fn test_parse_stacked_bar_skips_invalid() {
        let content =
            "# categories: X, Y\n# some comment\n- Valid: 1, 2\n- no colon here\n- Also: 3, 4";
        let data = parse_stacked_bar(content);
        assert_eq!(data.categories, vec!["X", "Y"]);
        assert_eq!(data.series.len(), 2);
    }

    #[test]
    fn test_parse_stacked_bar_no_categories() {
        // Without a directive the columns are numbered so the chart still renders
        let content = "- Product A: 10, 20";
        let data = parse_stacked_bar(content);
        assert_eq!(data.categories, vec!["1", "2"]);
        assert_eq!(data.series.len(), 1);
    }

    #[test]
    fn test_parse_stacked_bar_rejects_non_finite() {
        let data = parse_stacked_bar("# categories: A, B\n- S: inf, nan\n- T: 1, inf");
        assert_eq!(data.series.len(), 1);
        assert_eq!(data.series[0].values, vec![1.0]);
    }

    #[test]
    fn test_parse_stacked_bar_thousands_separators() {
        let data = parse_stacked_bar("# categories: A, B\n- S: 1,000, 2,000");
        assert_eq!(data.series[0].values, vec![1000.0, 2000.0]);
    }

    #[test]
    fn test_parse_stacked_bar_infers_categories() {
        let data = parse_stacked_bar("- Product A: 10, 20\n- Product B: 5, 6, 7");
        assert_eq!(data.categories, vec!["1", "2", "3"]);
        // An explicit directive always wins
        let data = parse_stacked_bar("# categories: X, Y\n- Product A: 10, 20, 30");
        assert_eq!(data.categories, vec!["X", "Y"]);
    }

    #[test]
    fn test_segment_corners_round_only_the_top() {
        let top = segment_corner_radius(8.0, true);
        assert_eq!(top.nw, 8);
        assert_eq!(top.ne, 8);
        assert_eq!(top.sw, 0);
        assert_eq!(top.se, 0);
        let inner = segment_corner_radius(8.0, false);
        assert_eq!(inner, egui::CornerRadius::ZERO);
    }

    #[test]
    fn test_max_stack_ignores_negatives_and_gaps() {
        let data = parse_stacked_bar("# categories: A, B\n- X: 10, -5\n- Y: 20");
        assert_eq!(max_stack(&data.series, 2), 30.0);
        assert_eq!(max_stack(&data.series, 0), 0.0);
    }

    #[test]
    fn test_stacked_layout_and_slots() {
        let l = stacked_layout(Pos2::new(0.0, 0.0), 1000.0, 500.0, 1.0, (true, true));
        assert_eq!(l.frame.left, 60.0 + 50.0 + 25.0);
        assert_eq!(l.frame.top, 70.0);
        assert_eq!(l.frame.height, 500.0 - 40.0 - 60.0 - 40.0 - 30.0);
        assert_eq!(l.legend_top, 15.0);
        assert_eq!(stack_slots(412.0, 4, 1.0), (12.0, 88.0));
    }
}
