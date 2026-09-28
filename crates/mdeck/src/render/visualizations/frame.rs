//! The plot area of an axis chart (bar, line, stacked bar, scatter) and the
//! furniture drawn around it: axis lines, grid lines with their values and
//! axis titles. Each chart sizes its own frame; the drawing is shared.

use eframe::egui::{Pos2, Stroke};

use super::{
    VIZ_FONT_AXIS_LABEL, VIZ_FONT_GRID_LABEL, VIZ_OPACITY_AXIS, VIZ_OPACITY_GRID,
    VIZ_OPACITY_GRID_LABEL, VIZ_STROKE_AXIS, VIZ_STROKE_GRID, VizCtx, draw_x_axis_label,
    draw_y_axis_label, format_axis_value,
};

/// The rectangle data is plotted in. Charts compute it either from an origin
/// and a size or from its four edges; both forms are kept as computed so
/// every coordinate matches the arithmetic the chart used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotFrame {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub width: f32,
    pub height: f32,
}

/// The values an axis spans, from `min` (bottom or left) to `max`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueRange {
    pub min: f32,
    pub max: f32,
}

impl ValueRange {
    /// From zero up to `max`.
    pub fn to(max: f32) -> Self {
        Self { min: 0.0, max }
    }

    /// Where `value` sits along the range: 0 at `min`, 1 at `max`.
    pub fn frac(&self, value: f32) -> f32 {
        (value - self.min) / (self.max - self.min)
    }
}

/// Axis titles and where they go: the x title's top edge and the y title's
/// left edge.
pub struct AxisTitles<'a> {
    pub x: Option<&'a str>,
    pub x_top: f32,
    pub y: Option<&'a str>,
    pub y_left: f32,
}

impl PlotFrame {
    pub fn from_size(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self {
            left,
            top,
            right: left + width,
            bottom: top + height,
            width,
            height,
        }
    }

    pub fn from_edges(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
            width: right - left,
            height: bottom - top,
        }
    }

    /// Screen y of `value` on a vertical axis spanning `range`.
    pub fn y_at(&self, value: f32, range: ValueRange) -> f32 {
        self.bottom - range.frac(value) * self.height
    }

    /// Screen x of `value` on a horizontal axis spanning `range`.
    pub fn x_at(&self, value: f32, range: ValueRange) -> f32 {
        self.left + range.frac(value) * self.width
    }

    /// The axis line along the bottom edge.
    pub fn draw_x_axis(&self, cx: &VizCtx) {
        axis_line(
            cx,
            [
                Pos2::new(self.left, self.bottom),
                Pos2::new(self.right, self.bottom),
            ],
        );
    }

    /// The axis line along the left edge.
    pub fn draw_y_axis(&self, cx: &VizCtx) {
        axis_line(
            cx,
            [
                Pos2::new(self.left, self.top),
                Pos2::new(self.left, self.bottom),
            ],
        );
    }

    /// Horizontal grid lines at `ticks`, each labelled left of the plot with
    /// its value at the precision `step` implies. With `zero_label`, 0 gets a
    /// label first but no line, since the x axis already sits there.
    pub fn draw_y_grid(
        &self,
        cx: &VizCtx,
        ticks: impl IntoIterator<Item = f32>,
        range: ValueRange,
        step: f32,
        zero_label: bool,
    ) {
        let painter = cx.ui.painter();
        let scale = cx.scale;
        let grid_color = cx.fg(VIZ_OPACITY_GRID);
        let grid_font = cx.font(VIZ_FONT_GRID_LABEL);
        let label_color = cx.fg(VIZ_OPACITY_GRID_LABEL);
        let zero = zero_label.then_some(0.0);
        for (i, value) in zero.into_iter().chain(ticks).enumerate() {
            let gy = self.y_at(value, range);
            if !(zero_label && i == 0) {
                painter.line_segment(
                    [Pos2::new(self.left, gy), Pos2::new(self.right, gy)],
                    Stroke::new(VIZ_STROKE_GRID * scale, grid_color),
                );
            }
            let label = format_axis_value(value, step);
            let galley = painter.layout_no_wrap(label, grid_font.clone(), label_color);
            painter.galley(
                Pos2::new(
                    self.left - galley.rect.width() - 8.0 * scale,
                    gy - galley.rect.height() / 2.0,
                ),
                galley,
                label_color,
            );
        }
    }

    /// The x title centred under the plot and the y title rotated along it.
    pub fn draw_titles(&self, cx: &VizCtx, titles: &AxisTitles) {
        let painter = cx.ui.painter();
        let font = cx.font(VIZ_FONT_AXIS_LABEL);
        let color = cx.fg(0.7);
        if let Some(text) = titles.x {
            draw_x_axis_label(
                painter,
                text,
                font.clone(),
                color,
                self.left,
                self.width,
                titles.x_top,
            );
        }
        if let Some(text) = titles.y {
            draw_y_axis_label(
                painter,
                text,
                font,
                color,
                titles.y_left,
                self.top,
                self.height,
            );
        }
    }
}

fn axis_line(cx: &VizCtx, points: [Pos2; 2]) {
    let color = cx.fg(VIZ_OPACITY_AXIS);
    cx.ui
        .painter()
        .line_segment(points, Stroke::new(VIZ_STROKE_AXIS * cx.scale, color));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plot_frame_from_size_and_edges_agree() {
        let a = PlotFrame::from_size(10.0, 20.0, 100.0, 50.0);
        let b = PlotFrame::from_edges(10.0, 20.0, 110.0, 70.0);
        assert_eq!(a, b);
    }

    #[test]
    fn test_plot_frame_maps_values() {
        let f = PlotFrame::from_size(0.0, 0.0, 200.0, 100.0);
        assert_eq!(f.y_at(0.0, ValueRange::to(50.0)), 100.0);
        assert_eq!(f.y_at(50.0, ValueRange::to(50.0)), 0.0);
        assert_eq!(f.y_at(25.0, ValueRange::to(50.0)), 50.0);
        let r = ValueRange {
            min: -10.0,
            max: 10.0,
        };
        assert_eq!(f.x_at(0.0, r), 100.0);
        assert_eq!(f.y_at(-10.0, r), 100.0);
    }
}
