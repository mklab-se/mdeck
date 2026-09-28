//! The ring pies and donuts share: slices swept clockwise from the top,
//! divided by background-coloured separators, beside a legend column.

use eframe::egui::{Pos2, Stroke};

use crate::theme::Theme;

use super::{VIZ_STROKE_SEPARATOR, VizCtx, sector_mesh, side_legend_width};

/// Where a pie's or donut's parts go: the ring centred in the area left of
/// the legend column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RingLayout {
    pub center: Pos2,
    /// 0 for a pie.
    pub inner_radius: f32,
    pub outer_radius: f32,
    /// The ring's area, from the chart's left edge to the legend.
    pub area_width: f32,
    /// The legend column right of the area, gap included.
    pub legend_width: f32,
}

/// A ring as large as the area allows with a margin, its hole `hole` of the
/// outer radius (0 for a pie).
pub fn ring_layout(pos: Pos2, max_width: f32, height: f32, scale: f32, hole: f32) -> RingLayout {
    let legend_width = side_legend_width(max_width, scale);
    let area_width = max_width - legend_width;
    let outer_radius = (area_width.min(height) / 2.0 - 30.0 * scale).max(40.0 * scale);
    let inner_radius = if hole > 0.0 { outer_radius * hole } else { 0.0 };
    RingLayout {
        center: Pos2::new(pos.x + area_width / 2.0, pos.y + height / 2.0),
        inner_radius,
        outer_radius,
        area_width,
        legend_width,
    }
}

/// The angle a slice of `value` out of `total` sweeps, grown to `anim`.
pub fn slice_sweep(value: f32, total: f32, anim: f32) -> f32 {
    let full_sweep = (value / total) * 2.0 * std::f32::consts::PI;
    full_sweep * anim
}

impl RingLayout {
    /// The separator along `angle`: from the centre of a pie, or just inside
    /// a donut's hole, to just outside the ring.
    pub fn separator(&self, angle: f32) -> [Pos2; 2] {
        let Pos2 { x, y } = self.center;
        let inner = if self.inner_radius > 0.0 {
            Pos2::new(
                x + (self.inner_radius - 1.0) * angle.cos(),
                y + (self.inner_radius - 1.0) * angle.sin(),
            )
        } else {
            self.center
        };
        let outer = Pos2::new(
            x + (self.outer_radius + 1.0) * angle.cos(),
            y + (self.outer_radius + 1.0) * angle.sin(),
        );
        [inner, outer]
    }

    /// The revealed slices of `values` (shares of `total`, shown at `steps`),
    /// each one mesh followed by its separator, from the top clockwise.
    pub fn draw_slices(&self, cx: &VizCtx, values: &[f32], total: f32, steps: &[usize]) {
        let painter = cx.ui.painter();
        let palette = cx.theme.edge_palette();
        crate::render::hints::push(
            cx.ui.ctx(),
            crate::render::hints::Hint::Circle {
                center: self.center,
                radius: self.outer_radius,
            },
        );

        let mut angle_offset = -std::f32::consts::FRAC_PI_2; // start at top
        let bg_color = Theme::with_opacity(cx.theme.background, cx.opacity);

        for (i, &value) in values.iter().enumerate() {
            let step = steps.get(i).copied().unwrap_or(0);
            if step > cx.reveal_step {
                continue;
            }
            let anim = cx.anim(step);
            let sweep = slice_sweep(value, total, anim);

            // Single mesh per slice: no anti-aliasing seams between segments
            painter.add(sector_mesh(
                self.center,
                self.inner_radius,
                self.outer_radius,
                angle_offset,
                sweep,
                cx.fill(&palette, i),
            ));

            // Separator line between slices
            painter.line_segment(
                self.separator(angle_offset + sweep),
                Stroke::new(VIZ_STROKE_SEPARATOR * cx.scale, bg_color),
            );

            angle_offset += sweep;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_layout_leaves_room_for_the_legend() {
        let pie = ring_layout(Pos2::new(100.0, 50.0), 1800.0, 600.0, 1.0, 0.0);
        assert_eq!(pie.legend_width, 380.0);
        assert_eq!(pie.area_width, 1420.0);
        assert_eq!(pie.center, Pos2::new(810.0, 350.0));
        assert_eq!(pie.outer_radius, 270.0);
        assert_eq!(pie.inner_radius, 0.0);

        let donut = ring_layout(Pos2::new(0.0, 0.0), 1800.0, 600.0, 1.0, 0.5);
        assert_eq!(donut.inner_radius, 135.0);
    }

    #[test]
    fn test_ring_layout_keeps_a_minimum_radius() {
        let tiny = ring_layout(Pos2::new(0.0, 0.0), 100.0, 50.0, 1.0, 0.0);
        assert_eq!(tiny.outer_radius, 40.0);
    }

    #[test]
    fn test_slice_sweep_grows_with_anim() {
        let full = slice_sweep(1.0, 4.0, 1.0);
        assert!((full - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert_eq!(slice_sweep(1.0, 4.0, 0.0), 0.0);
        assert!((slice_sweep(1.0, 4.0, 0.5) - full / 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_separator_starts_at_centre_of_pie_and_inside_donut_hole() {
        let pie = ring_layout(Pos2::new(0.0, 0.0), 1800.0, 600.0, 1.0, 0.0);
        let [a, b] = pie.separator(0.0);
        assert_eq!(a, pie.center);
        assert_eq!(b, Pos2::new(pie.center.x + 271.0, pie.center.y));

        let donut = ring_layout(Pos2::new(0.0, 0.0), 1800.0, 600.0, 1.0, 0.5);
        let [a, _] = donut.separator(0.0);
        assert_eq!(a, Pos2::new(donut.center.x + 134.0, donut.center.y));
    }
}
