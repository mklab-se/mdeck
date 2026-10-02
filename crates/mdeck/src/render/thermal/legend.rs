//! The legend of a `@thermal` block: the palette bar, its caption and
//! ticks, the threshold mark, and the note about clipped pixels.

use eframe::egui::{self, FontId, Pos2, Rect, Stroke, Vec2};

use super::compose::has_marks;
use super::draw::{Drawer, fg};
use super::spec::Unit;

impl Drawer<'_> {
    /// The palette bar with its scale: relative intensity for a display
    /// image, values for a mapped or data source; the threshold marked.
    pub(super) fn legend(&self, r: Rect, threshold: Option<f32>, a: f32) {
        let cx = self.cx;
        let scale = cx.scale;
        let painter = cx.ui.painter();
        let a = a * cx.opacity;
        let caption_font = FontId::new(cx.theme.body_size * 0.5 * scale, cx.theme.mono_family());
        let tick_font = FontId::new(cx.theme.body_size * 0.5 * scale, cx.theme.mono_family());
        let (caption, unit) = match self.source.unit() {
            None => ("RELATIVE INTENSITY".to_string(), None),
            Some(u) => (u.symbol().to_uppercase(), Some(u)),
        };
        let cap = painter.layout(caption, caption_font, fg(cx, 0.6 * a), r.width());
        let cap_h = cap.size().y;
        painter.galley(r.left_top(), cap, fg(cx, 0.6 * a));
        let bar = Rect::from_min_size(
            Pos2::new(r.left(), r.top() + cap_h + 10.0 * scale),
            Vec2::new(
                24.0 * scale,
                r.height() - cap_h - 10.0 * scale - 34.0 * scale,
            ),
        );
        // the gradient, cold at the bottom
        let lut = self.palette.lut();
        let mut mesh = egui::Mesh::default();
        let n = 64;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let y = bar.bottom() - t * bar.height();
            let c = lut[(t * 255.0) as usize].gamma_multiply(a);
            mesh.colored_vertex(Pos2::new(bar.left(), y), c);
            mesh.colored_vertex(Pos2::new(bar.right(), y), c);
            if i > 0 {
                let k = (i * 2) as u32;
                mesh.add_triangle(k - 2, k - 1, k);
                mesh.add_triangle(k - 1, k + 1, k);
            }
        }
        painter.add(egui::Shape::mesh(mesh));
        painter.rect_stroke(
            bar,
            0.0,
            Stroke::new(1.0 * scale, fg(cx, 0.25 * a)),
            egui::StrokeKind::Outside,
        );
        let (lo, hi) = self.window;
        let y_of = |v: f32| bar.bottom() - ((v - lo) / (hi - lo)).clamp(0.0, 1.0) * bar.height();
        let tick = |v: f32, text: String, strong: bool| {
            let y = y_of(v);
            painter.line_segment(
                [
                    Pos2::new(bar.right(), y),
                    Pos2::new(bar.right() + 8.0 * scale, y),
                ],
                Stroke::new(1.5 * scale, fg(cx, if strong { 0.9 } else { 0.4 } * a)),
            );
            let g = painter.layout_no_wrap(
                text,
                tick_font.clone(),
                fg(cx, if strong { 1.0 } else { 0.75 } * a),
            );
            let h = g.size().y;
            painter.galley(
                Pos2::new(bar.right() + 14.0 * scale, y - h / 2.0),
                g,
                fg(cx, 0.75 * a),
            );
        };
        match unit {
            None => {
                tick(hi, "high".into(), false);
                tick(lo, "low".into(), false);
            }
            Some(_) => {
                // as many ticks as fit the bar with room between them
                let fit = (bar.height() / (tick_font.size * 2.2)).floor() as u32;
                for v in nice_ticks(lo, hi, fit.clamp(2, 7)) {
                    tick(v, format_number(v, hi - lo), false);
                }
            }
        }
        if let Some(th) = threshold.filter(|t| t.is_finite()) {
            let y = y_of(th);
            painter.line_segment(
                [
                    Pos2::new(bar.left() - 6.0 * scale, y),
                    Pos2::new(bar.right(), y),
                ],
                Stroke::new(2.5 * scale, fg(cx, a)),
            );
            let text = match unit {
                None => format!("▸ {:.0}%", (th - lo) / (hi - lo) * 100.0),
                Some(u) => format!("▸ {}", format_value(th, hi - lo, u)),
            };
            let g = painter.layout_no_wrap(text, tick_font.clone(), fg(cx, a));
            painter.galley(
                Pos2::new(bar.left(), bar.bottom() + 8.0 * scale),
                g,
                fg(cx, a),
            );
        }
        if has_marks(self.source) {
            let g = painter.layout_no_wrap(
                "▨ clipped / no data".into(),
                FontId::new(cx.theme.body_size * 0.5 * scale, cx.theme.mono_family()),
                fg(cx, 0.55 * a),
            );
            painter.galley(
                Pos2::new(bar.left(), r.bottom() - g.size().y),
                g,
                fg(cx, 0.55 * a),
            );
        }
    }
}

/// A value with as many decimals as its span needs, and its unit.
pub fn format_value(v: f32, span: f32, unit: &Unit) -> String {
    format!("{} {}", format_number(v, span), unit.symbol())
}

/// A number with as many decimals as its span needs.
fn format_number(v: f32, span: f32) -> String {
    let decimals = if span.abs() < 10.0 { 1 } else { 0 };
    format!("{v:.decimals$}")
}

/// Round values for the legend's ticks between `lo` and `hi`: every
/// multiple of a nice step (five to seven of them).
fn nice_ticks(lo: f32, hi: f32, lines: u32) -> Vec<f32> {
    let step = crate::render::visualizations::nice_grid_step(hi - lo, lines);
    let mut v = (lo / step).ceil() * step;
    let mut out = Vec::new();
    while v <= hi + step * 1e-3 && out.len() < 12 {
        out.push(v);
        v += step;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::thermal::spec::Unit;

    #[test]
    fn legend_ticks_are_round_and_inside_the_window() {
        assert_eq!(
            nice_ticks(25.0, 90.0, 7),
            vec![30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0]
        );
        assert_eq!(nice_ticks(18.0, 92.0, 7), vec![20.0, 40.0, 60.0, 80.0]);
        let t = nice_ticks(0.0, 1.0, 7);
        assert!(
            nice_ticks(0.0, 100.0, 2).len() <= 3,
            "a short bar gets few ticks"
        );
        assert!(t.len() >= 3 && t.iter().all(|v| (0.0..=1.0).contains(v)));
    }

    #[test]
    fn values_get_decimals_only_for_narrow_spans() {
        assert_eq!(format_value(86.43, 50.0, &Unit::Celsius), "86 °C");
        assert_eq!(format_value(86.43, 5.0, &Unit::Celsius), "86.4 °C");
    }
}
