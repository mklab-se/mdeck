use eframe::egui::{Color32, Pos2, Stroke};

use super::{
    AxisTitles, PlotFrame, VIZ_DOT_RADIUS, VIZ_FONT_GRID_LABEL, VIZ_FONT_LEGEND,
    VIZ_STROKE_DATA_LINE, VIZ_SWATCH_SIZE, ValueRange, VizReveal, assign_steps,
    grammar::{Problem, Source, label_values_items, name_list},
    grid_values, label_stride, nice_axis_max, nice_grid_step,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct LineSeries {
    label: String,
    values: Vec<f32>,
    reveal: VizReveal,
}

struct LineChartData {
    x_labels: Vec<String>,
    series: Vec<LineSeries>,
    x_label: Option<String>,
    y_label: Option<String>,
}

const SETTINGS: &[&str] = &["x-labels", "x-label", "y-label"];

fn read(src: &Source) -> LineChartData {
    src.check_settings(SETTINGS);
    LineChartData {
        x_labels: src.setting("x-labels").map(name_list).unwrap_or_default(),
        series: label_values_items(src, "- Revenue: 10, 20, 30")
            .into_iter()
            .map(|s| LineSeries {
                label: s.label,
                values: s.values,
                reveal: s.reveal,
            })
            .collect(),
        x_label: src.setting("x-label").map(str::to_string),
        y_label: src.setting("y-label").map(str::to_string),
    }
}

fn parse_line_chart(content: &str) -> LineChartData {
    read(&Source::parse(content))
}

/// The problems in a `@line` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
}
// ─── Layout ─────────────────────────────────────────────────────────────────

/// Where the line chart's parts go.
#[derive(Debug, Clone, Copy, PartialEq)]
struct LineLayout {
    frame: PlotFrame,
    legend_left: f32,
    titles_x_top: f32,
    titles_y_left: f32,
}

/// Place the plot, leaving room for grid values and the y title on the left,
/// category labels and the x title below and the legend on the right.
fn line_layout(
    pos: Pos2,
    max_width: f32,
    height: f32,
    scale: f32,
    titles: (bool, bool),
) -> LineLayout {
    let (has_x_title, has_y_title) = titles;
    let padding = 60.0 * scale;
    let label_area = 50.0 * scale; // space for x-axis labels below
    let legend_width = 200.0 * scale;
    let y_axis_label_width = 60.0 * scale;
    let y_label_space = if has_y_title { 25.0 * scale } else { 0.0 };
    let x_label_space = if has_x_title { 30.0 * scale } else { 0.0 };
    let chart_left = pos.x + padding + y_axis_label_width + y_label_space;
    let chart_top = pos.y + padding;
    let chart_width = max_width - padding * 2.0 - y_axis_label_width - y_label_space - legend_width;
    let chart_height = height - padding * 2.0 - label_area - x_label_space;
    let frame = PlotFrame::from_size(chart_left, chart_top, chart_width, chart_height);
    LineLayout {
        frame,
        legend_left: pos.x + max_width - legend_width,
        titles_x_top: frame.bottom + label_area + 4.0 * scale,
        titles_y_left: pos.x + padding * 0.3,
    }
}

/// X of data point `i` when `max_points` points span the plot's width.
fn point_x(frame: &PlotFrame, i: usize, max_points: usize) -> f32 {
    frame.left + (i as f32 / (max_points - 1).max(1) as f32) * frame.width
}

/// Screen positions of a series' values. Negative values sit on the axis
/// rather than below the chart.
fn series_points(
    values: &[f32],
    frame: &PlotFrame,
    max_points: usize,
    range: ValueRange,
) -> Vec<Pos2> {
    values
        .iter()
        .enumerate()
        .map(|(i, &v)| Pos2::new(point_x(frame, i, max_points), frame.y_at(v.max(0.0), range)))
        .collect()
}

/// The segments of the polyline through `points` left of `clip_x`, the last
/// one cut at `clip_x`, so a line draws in from left to right.
fn clipped_segments(points: &[Pos2], clip_x: f32) -> Vec<[Pos2; 2]> {
    let mut segments = Vec::new();
    for pair in points.windows(2) {
        let (p1, p2) = (pair[0], pair[1]);
        if p1.x > clip_x {
            break;
        }
        let end = if p2.x > clip_x {
            // Interpolate to clip boundary
            let t = (clip_x - p1.x) / (p2.x - p1.x);
            Pos2::new(clip_x, p1.y + t * (p2.y - p1.y))
        } else {
            p2
        };
        segments.push([p1, end]);
    }
    segments
}

// ─── Renderer ───────────────────────────────────────────────────────────────

pub fn draw_line_chart(
    cx: &super::VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let scale = cx.scale;
    let data = parse_line_chart(content);
    let series = &data.series;
    if series.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    let reveals: Vec<VizReveal> = series.iter().map(|s| s.reveal).collect();
    let steps = assign_steps(&reveals);
    let palette = cx.theme.edge_palette();

    // Find global max value across all series
    let max_value = series
        .iter()
        .flat_map(|s| s.values.iter())
        .copied()
        .fold(0.0f32, f32::max);
    if max_value <= 0.0 {
        return height;
    }
    // Scale the axis to a round number so the top grid line sits above the data
    let range = ValueRange::to(nice_axis_max(max_value, 5));

    // Find max number of data points
    let max_points = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    if max_points == 0 {
        return height;
    }

    let titles = (data.x_label.is_some(), data.y_label.is_some());
    let layout = line_layout(pos, max_width, height, scale, titles);
    let frame = layout.frame;

    // Grid lines with nice numbers, then the axes
    let grid_step = nice_grid_step(range.max, 5);
    frame.draw_y_grid(
        cx,
        grid_values(range.max, grid_step),
        range,
        grid_step,
        true,
    );
    frame.draw_x_axis(cx);
    frame.draw_y_axis(cx);

    draw_x_labels(cx, &data.x_labels, &frame, max_points);

    for (si, s) in series.iter().enumerate() {
        let step = steps.get(si).copied().unwrap_or(0);
        if step > cx.reveal_step || s.values.is_empty() {
            continue;
        }
        let anim = cx.anim(step);
        let points = series_points(&s.values, &frame, max_points, range);
        draw_series(
            cx,
            &points,
            cx.fill(&palette, si),
            frame.left + anim * frame.width,
        );
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

    draw_legend(cx, series, &steps, &palette, layout.legend_left, frame.top);

    height
}

/// Category labels under the x axis, thinned out when they would overlap.
fn draw_x_labels(cx: &super::VizCtx, x_labels: &[String], frame: &PlotFrame, max_points: usize) {
    let painter = cx.ui.painter();
    let scale = cx.scale;
    let x_label_font = cx.font(VIZ_FONT_GRID_LABEL);
    let x_label_color = cx.muted(1.0);
    let x_galleys: Vec<_> = x_labels
        .iter()
        .take(max_points)
        .map(|label| painter.layout_no_wrap(label.clone(), x_label_font.clone(), x_label_color))
        .collect();
    let widest = x_galleys
        .iter()
        .map(|g| g.rect.width())
        .fold(0.0f32, f32::max);
    let slot_width = frame.width / (max_points - 1).max(1) as f32;
    let stride = label_stride(widest + 12.0 * scale, slot_width);
    for (i, galley) in x_galleys.into_iter().enumerate() {
        if i % stride != 0 {
            continue;
        }
        let x = point_x(frame, i, max_points);
        painter.galley(
            Pos2::new(x - galley.rect.width() / 2.0, frame.bottom + 8.0 * scale),
            galley,
            x_label_color,
        );
    }
}

/// One series' line and dots, drawn up to `clip_x`.
fn draw_series(cx: &super::VizCtx, points: &[Pos2], color: Color32, clip_x: f32) {
    let painter = cx.ui.painter();
    let scale = cx.scale;
    for segment in clipped_segments(points, clip_x) {
        painter.line_segment(segment, Stroke::new(VIZ_STROKE_DATA_LINE * scale, color));
    }

    // Dots at data points (only those within clip range)
    let dot_radius = VIZ_DOT_RADIUS * scale;
    for &pt in points {
        if pt.x > clip_x + 0.5 {
            break;
        }
        painter.circle_filled(pt, dot_radius, color);
    }
    crate::render::hints::push(
        cx.ui.ctx(),
        crate::render::hints::Hint::Path(
            points
                .iter()
                .copied()
                .filter(|p| p.x <= clip_x + 0.5)
                .collect(),
        ),
    );
}

/// The legend at the top right: a line swatch and the name of each shown
/// series.
fn draw_legend(
    cx: &super::VizCtx,
    series: &[LineSeries],
    steps: &[usize],
    palette: &[Color32],
    legend_x: f32,
    legend_start_y: f32,
) {
    let painter = cx.ui.painter();
    let scale = cx.scale;
    let legend_font = cx.font(VIZ_FONT_LEGEND);
    let legend_item_height = 32.0 * scale;
    let swatch_width = VIZ_SWATCH_SIZE * scale;
    let dot_radius = VIZ_DOT_RADIUS * scale;

    for (si, s) in series.iter().enumerate() {
        let step = steps.get(si).copied().unwrap_or(0);
        if step > cx.reveal_step {
            continue;
        }

        let ly = legend_start_y + si as f32 * legend_item_height;
        let color = cx.fill(palette, si);
        let text_color = cx.fg(1.0);

        // Color swatch (line style)
        let swatch_y = ly + legend_item_height / 2.0;
        painter.line_segment(
            [
                Pos2::new(legend_x, swatch_y),
                Pos2::new(legend_x + swatch_width, swatch_y),
            ],
            Stroke::new(VIZ_STROKE_DATA_LINE * scale, color),
        );
        painter.circle_filled(
            Pos2::new(legend_x + swatch_width / 2.0, swatch_y),
            dot_radius * 0.8,
            color,
        );

        // Label
        let galley = painter.layout_no_wrap(s.label.clone(), legend_font.clone(), text_color);
        painter.galley(
            Pos2::new(
                legend_x + swatch_width + 8.0 * scale,
                ly + (legend_item_height - galley.rect.height()) / 2.0,
            ),
            galley,
            text_color,
        );
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_chart_basic() {
        let content = "x-labels: Q1, Q2, Q3, Q4\n- Revenue: 100, 150, 200, 280";
        let data = parse_line_chart(content);
        assert_eq!(data.x_labels, vec!["Q1", "Q2", "Q3", "Q4"]);
        assert_eq!(data.series.len(), 1);
        assert_eq!(data.series[0].label, "Revenue");
        assert_eq!(data.series[0].values, vec![100.0, 150.0, 200.0, 280.0]);
    }

    #[test]
    fn test_parse_line_chart_multiple_series() {
        let content = "x-labels: A, B, C\n- Revenue: 100, 150, 200\n+ Costs: 80, 90, 120";
        let data = parse_line_chart(content);
        assert_eq!(data.x_labels.len(), 3);
        assert_eq!(data.series.len(), 2);
        assert_eq!(data.series[0].label, "Revenue");
        assert_eq!(data.series[0].reveal, VizReveal::Static);
        assert_eq!(data.series[1].label, "Costs");
        assert_eq!(data.series[1].reveal, VizReveal::NextStep);
    }

    #[test]
    fn test_parse_line_chart_no_labels() {
        let content = "- Sales: 10, 20, 30";
        let data = parse_line_chart(content);
        assert!(data.x_labels.is_empty());
        assert_eq!(data.series.len(), 1);
        assert_eq!(data.series[0].values, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_parse_line_chart_skips_invalid() {
        let content =
            "x-labels: A, B\n- Valid: 10, 20\n- no_colon_values\n# comment\n- Also: 30, 40";
        let data = parse_line_chart(content);
        assert_eq!(data.series.len(), 2);
    }

    #[test]
    fn test_parse_line_chart_axis_labels() {
        let content = "x-label: Quarter\ny-label: Revenue ($M)\nx-labels: Q1, Q2\n- Sales: 10, 20";
        let data = parse_line_chart(content);
        assert_eq!(data.x_label, Some("Quarter".to_string()));
        assert_eq!(data.y_label, Some("Revenue ($M)".to_string()));
        assert_eq!(data.series.len(), 1);
    }

    #[test]
    fn test_parse_line_chart_rejects_non_finite() {
        let data = parse_line_chart("- A: inf, nan\n- B: 1, inf, 3");
        assert_eq!(data.series.len(), 1);
        assert_eq!(data.series[0].values, vec![1.0, 3.0]);
    }

    #[test]
    fn test_parse_line_chart_thousands_separators() {
        let data = parse_line_chart("- Revenue: 1,000, 2,000, 3,500");
        assert_eq!(data.series[0].values, vec![1000.0, 2000.0, 3500.0]);
        let data = parse_line_chart("- Costs: $80, $90, $120");
        assert_eq!(data.series[0].values, vec![80.0, 90.0, 120.0]);
    }

    #[test]
    fn test_line_layout_reserves_title_space() {
        let pos = Pos2::new(0.0, 0.0);
        let plain = line_layout(pos, 1600.0, 800.0, 1.0, (false, false));
        assert_eq!(plain.frame.left, 120.0);
        assert_eq!(plain.frame.width, 1600.0 - 120.0 - 60.0 - 200.0);
        assert_eq!(plain.frame.height, 800.0 - 120.0 - 50.0);
        assert_eq!(plain.legend_left, 1400.0);
        let titled = line_layout(pos, 1600.0, 800.0, 1.0, (true, true));
        assert_eq!(titled.frame.left, plain.frame.left + 25.0);
        assert_eq!(titled.frame.height, plain.frame.height - 30.0);
    }

    #[test]
    fn test_series_points_span_the_frame() {
        let frame = PlotFrame::from_size(0.0, 0.0, 100.0, 50.0);
        let points = series_points(&[0.0, 10.0, -5.0], &frame, 3, ValueRange::to(10.0));
        assert_eq!(
            points,
            vec![
                Pos2::new(0.0, 50.0),
                Pos2::new(50.0, 0.0),
                Pos2::new(100.0, 50.0)
            ]
        );
    }

    #[test]
    fn test_clipped_segments_cut_at_clip() {
        let points = [
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(20.0, 0.0),
        ];
        assert_eq!(clipped_segments(&points, 30.0).len(), 2);
        let cut = clipped_segments(&points, 5.0);
        assert_eq!(cut, vec![[Pos2::new(0.0, 0.0), Pos2::new(5.0, 5.0)]]);
        assert!(clipped_segments(&points, -1.0).is_empty());
    }
}
