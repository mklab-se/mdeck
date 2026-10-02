//! Gantt chart: dated tasks as bars on a time axis, with dependency arrows.

mod date;
mod grid;
mod parse;
pub use parse::check;

use eframe::egui::{self, Color32, FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    PlotFrame, VIZ_CORNER_BAR, VIZ_FONT_GRID_LABEL, VIZ_FONT_SECONDARY_LABEL, VIZ_FONT_TITLE,
    VIZ_LABEL_REVEAL_THRESHOLD, VIZ_OPACITY_GRID, VIZ_STROKE_AXIS, VIZ_STROKE_CONNECTOR,
    VIZ_STROKE_GRID, VizCtx, VizReveal, assign_steps, label_fade, label_stride,
};
use date::Date;
use grid::{TimeGrid, TimeScale, compute_time_grid, format_duration_label};
use parse::{LabelMode, ResolvedTask, parse_gantt, resolve_tasks};

// ─── Layout ─────────────────────────────────────────────────────────────────

/// The dates the time axis shows: the tasks' span plus a little on each side.
#[derive(Debug, Clone, Copy, PartialEq)]
struct DateSpan {
    min: Date,
    max: Date,
    /// Days from `min` to `max` (at least 1).
    total: i64,
}

impl DateSpan {
    fn of(tasks: &[ResolvedTask]) -> Self {
        let min_date = tasks.iter().map(|t| t.start).min().unwrap();
        let max_date = tasks.iter().map(|t| t.end).max().unwrap();
        let total_days = min_date.days_between(&max_date).max(1);
        // Pad by 3% (a day at least) on each side
        let pad_days = (total_days as f32 * 0.03).max(1.0) as i64;
        let min = min_date.add_days(-pad_days);
        let max = max_date.add_days(pad_days);
        Self {
            min,
            max,
            total: min.days_between(&max).max(1),
        }
    }

    /// Where `date` sits along the axis: 0 at `min`, 1 at `max`.
    fn frac(&self, date: &Date) -> f32 {
        self.min.days_between(date) as f32 / self.total as f32
    }
}

/// Where the gantt chart's parts go.
#[derive(Debug, Clone, Copy, PartialEq)]
struct GanttLayout {
    /// The time axis area.
    frame: PlotFrame,
    /// Width left of the axis for task names.
    label_area_width: f32,
    row_height: f32,
    bar_height: f32,
    /// Offset that centres the rows when they do not fill the frame.
    y_offset: f32,
}

impl GanttLayout {
    fn new(
        pos: Pos2,
        max_width: f32,
        height: f32,
        scale: f32,
        (labels, has_title): (LabelMode, bool),
        total_tasks: usize,
    ) -> Self {
        let padding = 40.0 * scale;
        let label_area_width = if labels == LabelMode::Inside {
            padding * 0.5 // Minimal left margin when labels are inside bars
        } else {
            max_width * 0.22 // Left area for task names
        };
        let timeline_label_height = 35.0 * scale; // Bottom area for date labels
        let header_height = if has_title {
            40.0 * scale
        } else {
            10.0 * scale
        };
        let chart_height = height - header_height - timeline_label_height - padding * 0.5;
        let frame = PlotFrame::from_size(
            pos.x + label_area_width,
            pos.y + header_height,
            max_width - label_area_width - padding,
            chart_height,
        );

        let row_height = (chart_height / total_tasks as f32).min(64.0 * scale);
        let bar_height = (row_height * 0.6).min(40.0 * scale).max(12.0 * scale);
        let total_task_height = total_tasks as f32 * row_height;
        let y_offset = if total_task_height < chart_height {
            (chart_height - total_task_height) / 2.0
        } else {
            0.0
        };
        Self {
            frame,
            label_area_width,
            row_height,
            bar_height,
            y_offset,
        }
    }

    fn row_y(&self, i: usize) -> f32 {
        self.frame.top + self.y_offset + i as f32 * self.row_height
    }
}

// ─── Renderer ───────────────────────────────────────────────────────────────

pub fn draw_gantt_chart(
    cx: &VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let scale = cx.scale;
    let data = parse_gantt(content);
    let resolved = resolve_tasks(&data);
    if resolved.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    let reveals: Vec<VizReveal> = resolved.iter().map(|t| t.reveal).collect();
    let steps = assign_steps(&reveals);

    let span = DateSpan::of(&resolved);
    let time_grid = compute_time_grid(&span.min, &span.max, span.total);
    let layout = GanttLayout::new(
        pos,
        max_width,
        height,
        scale,
        (data.labels, data.title.is_some()),
        resolved.len(),
    );
    let frame = layout.frame;

    if steps.iter().all(|&s| s > cx.reveal_step) {
        return height;
    }

    if let Some(ref title_text) = data.title {
        draw_title(cx, title_text, pos, max_width);
    }
    draw_time_grid(cx, &frame, &time_grid);
    if matches!(time_grid.scale, TimeScale::Days) {
        draw_weekends(cx, &frame, &span);
    }

    // Axis line at bottom
    let painter = cx.ui.painter();
    painter.line_segment(
        [
            Pos2::new(frame.left, frame.bottom),
            Pos2::new(frame.right, frame.bottom),
        ],
        Stroke::new(VIZ_STROKE_AXIS * scale, cx.rule(1.0)),
    );

    let tasks = GanttPaint {
        cx: *cx,
        painter,
        tasks: &resolved,
        steps: &steps,
        layout,
        span,
        time_scale: time_grid.scale,
        labels: data.labels,
        palette: cx.theme.edge_palette(),
        task_name_font: cx.font(VIZ_FONT_SECONDARY_LABEL),
        bar_label_font: cx.font(VIZ_FONT_GRID_LABEL),
        row_left: pos.x,
        row_width: max_width,
    };
    for i in 0..resolved.len() {
        if steps.get(i).copied().unwrap_or(0) <= cx.reveal_step {
            tasks.task(i);
        }
    }

    // Separator between the label area and the chart
    let sep_x = frame.left - 4.0 * scale;
    painter.line_segment(
        [Pos2::new(sep_x, frame.top), Pos2::new(sep_x, frame.bottom)],
        Stroke::new(0.5 * scale, cx.rule(VIZ_OPACITY_GRID)),
    );

    height
}

fn draw_title(cx: &VizCtx, title: &str, pos: Pos2, max_width: f32) {
    let painter = cx.ui.painter();
    let title_color = cx.fg(0.9);
    let galley = painter.layout_no_wrap(title.to_string(), cx.font(VIZ_FONT_TITLE), title_color);
    let tx = pos.x + (max_width - galley.rect.width()) / 2.0;
    painter.galley(Pos2::new(tx, pos.y + 4.0 * cx.scale), galley, title_color);
}

/// Vertical grid lines at the time grid's dates, with date labels below,
/// thinned out when they would overlap.
fn draw_time_grid(cx: &VizCtx, frame: &PlotFrame, time_grid: &TimeGrid) {
    let painter = cx.ui.painter();
    let scale = cx.scale;
    let grid_color = cx.rule(VIZ_OPACITY_GRID);
    let label_font = cx.font(VIZ_FONT_GRID_LABEL);
    let label_color = cx.fg(0.45);

    let date_galleys: Vec<_> = time_grid
        .labels
        .iter()
        .map(|(_, text)| painter.layout_no_wrap(text.clone(), label_font.clone(), label_color))
        .collect();
    let widest_date = date_galleys
        .iter()
        .map(|g| g.rect.width())
        .fold(0.0f32, f32::max);
    let slot_width = match time_grid.labels.as_slice() {
        [(a, _), (b, _), ..] => (b - a) * frame.width,
        _ => frame.width,
    };
    let date_stride = label_stride(widest_date + 12.0 * scale, slot_width);

    for (idx, ((frac, _), galley)) in time_grid.labels.iter().zip(date_galleys).enumerate() {
        let x = frame.left + frac * frame.width;
        painter.line_segment(
            [Pos2::new(x, frame.top), Pos2::new(x, frame.bottom)],
            Stroke::new(VIZ_STROKE_GRID * scale, grid_color),
        );
        if idx % date_stride != 0 {
            continue;
        }
        let lx = x - galley.rect.width() / 2.0;
        painter.galley(
            Pos2::new(lx, frame.bottom + 6.0 * scale),
            galley,
            label_color,
        );
    }
}

/// Shade Saturdays and Sundays (day-level axis only).
fn draw_weekends(cx: &VizCtx, frame: &PlotFrame, span: &DateSpan) {
    let painter = cx.ui.painter();
    let weekend_color = cx.fg(0.04);
    let mut d = span.min;
    while d <= span.max {
        if d.weekday() >= 5 {
            let x0 = frame.left + span.frac(&d) * frame.width;
            let x1 = frame.left + span.frac(&d.add_days(1)) * frame.width;
            let weekend_rect =
                egui::Rect::from_min_max(Pos2::new(x0, frame.top), Pos2::new(x1, frame.bottom));
            painter.rect_filled(weekend_rect, 0.0, weekend_color);
        }
        d = d.add_days(1);
    }
}

/// A task's bar as computed: its size is kept rather than re-derived from a
/// rectangle so every label position matches the bar exactly.
#[derive(Debug, Clone, Copy)]
struct Bar {
    x: f32,
    y: f32,
    w: f32,
}

/// Everything drawing one task row needs.
struct GanttPaint<'a> {
    cx: VizCtx<'a>,
    painter: &'a egui::Painter,
    tasks: &'a [ResolvedTask],
    steps: &'a [usize],
    layout: GanttLayout,
    span: DateSpan,
    time_scale: TimeScale,
    labels: LabelMode,
    palette: [Color32; crate::theme::EDGE_PALETTE_LEN],
    task_name_font: FontId,
    bar_label_font: FontId,
    /// The full width alternate rows are shaded across.
    row_left: f32,
    row_width: f32,
}

impl GanttPaint<'_> {
    /// Row `i`: shading, name, bar, bar label and the arrows from the tasks
    /// it depends on.
    fn task(&self, i: usize) {
        let VizCtx { opacity, scale, .. } = self.cx;
        let task = &self.tasks[i];
        let layout = &self.layout;
        let frame = &layout.frame;
        let anim = self.cx.anim(self.steps.get(i).copied().unwrap_or(0));
        let row_y = layout.row_y(i);
        let bar_y = row_y + (layout.row_height - layout.bar_height) / 2.0;

        // Alternating row background
        if i.is_multiple_of(2) {
            let row_rect = egui::Rect::from_min_size(
                Pos2::new(self.row_left, row_y),
                egui::vec2(self.row_width, layout.row_height),
            );
            self.painter.rect_filled(row_rect, 0.0, self.cx.fg(0.02));
        }

        // Task name on the left (side mode only)
        if self.labels == LabelMode::Side {
            let name_color = Theme::with_opacity(self.cx.theme.foreground, opacity * 0.8 * anim);
            let galley = self.painter.layout(
                task.name.clone(),
                self.task_name_font.clone(),
                name_color,
                layout.label_area_width - 16.0 * scale,
            );
            let name_y = row_y + (layout.row_height - galley.rect.height()) / 2.0;
            self.painter.galley(
                Pos2::new(self.row_left + 8.0 * scale, name_y),
                galley,
                name_color,
            );
        }

        let start_frac = self.span.frac(&task.start);
        let end_frac = self.span.frac(&task.end);
        let bar_x = frame.left + start_frac * frame.width;
        let bar_w = ((end_frac - start_frac) * frame.width * anim).max(3.0 * scale);
        let bar_rect = egui::Rect::from_min_size(
            Pos2::new(bar_x, bar_y),
            egui::vec2(bar_w, layout.bar_height),
        );

        let base = self.palette[i % self.palette.len()];
        let bar_corner = VIZ_CORNER_BAR * scale;
        self.painter.rect_filled(
            bar_rect,
            bar_corner,
            Theme::with_opacity(base, opacity * 0.75 * anim),
        );
        crate::render::hints::push(self.cx.ui.ctx(), crate::render::hints::Hint::Bar(bar_rect));
        // Subtle border for definition
        self.painter.rect_stroke(
            bar_rect,
            bar_corner,
            Stroke::new(0.5 * scale, Theme::with_opacity(base, opacity * 0.3 * anim)),
            egui::StrokeKind::Outside,
        );

        // Bar label: task name inside (inside mode) or duration (side mode)
        if anim > VIZ_LABEL_REVEAL_THRESHOLD {
            let days = task.start.days_between(&task.end);
            let duration = format_duration_label(days, &self.time_scale);
            let bar = Bar {
                x: bar_x,
                y: bar_y,
                w: bar_w,
            };
            if self.labels == LabelMode::Inside {
                self.name_label(task, &duration, bar, label_fade(anim));
            } else {
                self.duration_label(duration, bar, label_fade(anim));
            }
        }

        self.dependency_arrows(task, Pos2::new(bar_x, bar_y), anim);
    }

    /// Inside mode: the task name and duration in the bar, else the name
    /// alone, else the name right of the bar when it fits the chart.
    fn name_label(&self, task: &ResolvedTask, duration: &str, bar: Bar, fade: f32) {
        let scale = self.cx.scale;
        let (bar_x, bar_y, bar_w) = (bar.x, bar.y, bar.w);
        let bar_height = self.layout.bar_height;
        let text_color =
            Theme::with_opacity(self.cx.theme.foreground, self.cx.opacity * 0.9 * fade);
        let layout = |text: String| {
            self.painter
                .layout_no_wrap(text, self.bar_label_font.clone(), text_color)
        };

        let name_galley = layout(format!("{}  {}", task.name, duration));
        if name_galley.rect.width() + 12.0 * scale < bar_w {
            // Fits inside: left-aligned with padding
            let dy = bar_y + (bar_height - name_galley.rect.height()) / 2.0;
            self.painter
                .galley(Pos2::new(bar_x + 6.0 * scale, dy), name_galley, text_color);
            return;
        }
        let name_only = layout(task.name.clone());
        let dy = bar_y + (bar_height - name_only.rect.height()) / 2.0;
        if name_only.rect.width() + 12.0 * scale < bar_w {
            self.painter
                .galley(Pos2::new(bar_x + 6.0 * scale, dy), name_only, text_color);
        } else {
            // Does not fit: place it right of the bar
            let dx = bar_x + bar_w + 6.0 * scale;
            if dx + name_only.rect.width() < self.layout.frame.right {
                self.painter
                    .galley(Pos2::new(dx, dy), name_only, text_color);
            }
        }
    }

    /// Side mode: the duration centred in the bar, else right of it.
    fn duration_label(&self, duration: String, bar: Bar, fade: f32) {
        let scale = self.cx.scale;
        let (bar_x, bar_y, bar_w) = (bar.x, bar.y, bar.w);
        let bar_height = self.layout.bar_height;
        let dur_color = Theme::with_opacity(self.cx.theme.foreground, self.cx.opacity * 0.7 * fade);
        let dur_galley =
            self.painter
                .layout_no_wrap(duration, self.bar_label_font.clone(), dur_color);
        let dy = bar_y + (bar_height - dur_galley.rect.height()) / 2.0;
        if dur_galley.rect.width() + 8.0 * scale < bar_w {
            let dx = bar_x + (bar_w - dur_galley.rect.width()) / 2.0;
            self.painter
                .galley(Pos2::new(dx, dy), dur_galley, dur_color);
        } else if bar_x + bar_w + dur_galley.rect.width() + 10.0 * scale < self.layout.frame.right {
            let dx = bar_x + bar_w + 6.0 * scale;
            self.painter
                .galley(Pos2::new(dx, dy), dur_galley, dur_color);
        }
    }

    /// An L-shaped arrow from the end of each shown dependency to the start
    /// of the bar at `bar_min` (its top-left corner).
    fn dependency_arrows(&self, task: &ResolvedTask, bar_min: Pos2, anim: f32) {
        let scale = self.cx.scale;
        let frame = &self.layout.frame;
        let arrow_color =
            Theme::with_opacity(self.cx.theme.foreground, self.cx.opacity * 0.25 * anim);
        let arrow_stroke = Stroke::new(VIZ_STROKE_CONNECTOR * scale, arrow_color);
        let arrow_end_x = bar_min.x;
        let arrow_end_y = bar_min.y + self.layout.bar_height / 2.0;
        let arrow_size = 4.0 * scale;

        for (dep_name, _) in &task.dependencies {
            let Some((di, dep_task)) = self
                .tasks
                .iter()
                .enumerate()
                .find(|(_, r)| r.name == *dep_name)
            else {
                continue;
            };
            if self.steps.get(di).copied().unwrap_or(0) > self.cx.reveal_step {
                continue;
            }

            let dep_center_y = self.layout.row_y(di) + self.layout.row_height / 2.0;
            let arrow_start_x = frame.left + self.span.frac(&dep_task.end) * frame.width;
            let mid_x = (arrow_start_x + arrow_end_x) / 2.0;
            let end = Pos2::new(arrow_end_x, arrow_end_y);
            for segment in [
                [
                    Pos2::new(arrow_start_x, dep_center_y),
                    Pos2::new(mid_x, dep_center_y),
                ],
                [
                    Pos2::new(mid_x, dep_center_y),
                    Pos2::new(mid_x, arrow_end_y),
                ],
                [Pos2::new(mid_x, arrow_end_y), end],
                // Arrowhead
                [
                    Pos2::new(arrow_end_x - arrow_size, arrow_end_y - arrow_size),
                    end,
                ],
                [
                    Pos2::new(arrow_end_x - arrow_size, arrow_end_y + arrow_size),
                    end,
                ],
            ] {
                self.painter.line_segment(segment, arrow_stroke);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(content: &str) -> Vec<ResolvedTask> {
        resolve_tasks(&parse_gantt(content))
    }

    #[test]
    fn test_date_span_pads_both_sides() {
        let tasks = resolved("- A: 2024-01-01, 10d\n- B: 2024-01-11, 20d");
        let span = DateSpan::of(&tasks);
        // 30 days of tasks: padded by 1 day (3% rounds down to 0)
        assert_eq!(span.min, Date::new(2023, 12, 31));
        assert_eq!(span.max, Date::new(2024, 2, 1));
        assert_eq!(span.total, 32);
        assert_eq!(span.frac(&span.min), 0.0);
        assert_eq!(span.frac(&span.max), 1.0);
    }

    #[test]
    fn test_gantt_layout_rows() {
        let pos = Pos2::new(0.0, 0.0);
        let side = GanttLayout::new(pos, 1000.0, 500.0, 1.0, (LabelMode::Side, true), 3);
        assert_eq!(side.label_area_width, 220.0);
        assert_eq!(side.frame.left, 220.0);
        assert_eq!(side.frame.width, 1000.0 - 220.0 - 40.0);
        assert_eq!(side.frame.top, 40.0);
        assert_eq!(side.frame.height, 500.0 - 40.0 - 35.0 - 20.0);
        // Few tasks: rows are capped and centred
        assert_eq!(side.row_height, 64.0);
        assert_eq!(side.bar_height, 38.4);
        assert_eq!(side.row_y(0), 40.0 + (405.0 - 192.0) / 2.0);

        let inside = GanttLayout::new(pos, 1000.0, 500.0, 1.0, (LabelMode::Inside, false), 100);
        assert_eq!(inside.frame.left, 20.0);
        assert_eq!(inside.y_offset, 0.0);
        assert_eq!(inside.bar_height, 12.0);
    }
}
