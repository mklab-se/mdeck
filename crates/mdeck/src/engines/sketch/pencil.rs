//! The pencil: a sharpened pencil held at the drawing point, and graphite
//! lines for the pen strokes a sketch draws when a slide has no artwork.

use mdeck_sdk::paint::{Color, Painter, Pos2, Rect, Stroke, Vec2, mix, premul};
use mdeck_sdk::tokens::Tokens;

use crate::engines::art::across;
use crate::engines::art::strokes::{Strokes, to_screen};
use crate::engines::hash01;

/// The sketch's colours.
pub(super) struct Graphite {
    /// Pencil lines.
    pub(super) lead: Color,
    /// The pencil's painted body.
    pub(super) body: Color,
}

impl Graphite {
    pub(super) fn of(t: &Tokens) -> Self {
        Graphite {
            lead: mix(t.heading, t.background, 0.12),
            body: t.particle_cool,
        }
    }
}

/// A pencil whose point rests on `tip`, held from the lower right, with its
/// shadow on the paper. `wobble` (0..1) tilts it a little as the hand moves.
pub(super) fn pencil(
    painter: &Painter,
    tip: Pos2,
    scale: f32,
    g: &Graphite,
    wobble: f32,
    opacity: f32,
) {
    if opacity <= 0.0 {
        return;
    }
    let pose = PencilPose::new(tip, scale, wobble);
    pose.shadow(painter, opacity);
    pose.body(painter, g, opacity);
    pose.point(painter, g, opacity);
}

/// A black shadow at opacity `a` (premultiplied).
fn shadow_colour(a: f32) -> Color {
    Color::from_rgba_premultiplied(0, 0, 0, (a * 255.0) as u8)
}

/// Where the pencil lies: its axis `d` from the tip toward the eraser, the
/// normal `n` across it, its width and the stations along it.
struct PencilPose {
    tip: Pos2,
    d: Vec2,
    n: Vec2,
    w: f32,
    scale: f32,
    /// Where the sharpened cone meets the painted body.
    base: Pos2,
    /// Where the metal ferrule starts.
    ferrule: Pos2,
    end: Pos2,
}

impl PencilPose {
    fn new(tip: Pos2, scale: f32, wobble: f32) -> Self {
        // from the point up and to the right, about 55 degrees above level
        let theta: f32 = 0.96 + 0.05 * (wobble * std::f32::consts::TAU).sin();
        let d = Vec2::new(theta.cos(), -theta.sin());
        let cone = 30.0 * scale;
        let body = 230.0 * scale;
        let base = tip + d * cone;
        let end = base + d * body;
        Self {
            tip,
            d,
            n: across(d),
            w: 15.0 * scale,
            scale,
            base,
            ferrule: end - d * 18.0 * scale,
            end,
        }
    }

    /// A band along the axis from `a` to `b`, `half` wide on each side.
    fn quad(&self, painter: &Painter, a: Pos2, b: Pos2, half: f32, c: Color) {
        let n = self.n;
        painter.convex_polygon(
            vec![a + n * half, b + n * half, b - n * half, a - n * half],
            c,
            Stroke::NONE,
        );
    }

    /// The shadow falls down and to the right of the pencil.
    fn shadow(&self, painter: &Painter, opacity: f32) {
        let (n, w) = (self.n, self.w);
        let off = Vec2::new(14.0, 18.0) * self.scale;
        for (k, a) in [(1.0, 0.07), (0.6, 0.08)] {
            painter.convex_polygon(
                vec![
                    self.tip + off * 0.15,
                    self.base + off + n * w * 0.5 * k,
                    self.end + off + n * w * 0.55 * k,
                    self.end + off - n * w * 0.55 * k,
                    self.base + off - n * w * 0.5 * k,
                ],
                shadow_colour(a * opacity),
                Stroke::NONE,
            );
        }
    }

    /// The painted body in three facets, the ferrule and the eraser.
    fn body(&self, painter: &Painter, g: &Graphite, opacity: f32) {
        let (d, n, w, scale) = (self.d, self.n, self.w, self.scale);
        let (base, ferrule, end) = (self.base, self.ferrule, self.end);
        let shade = |c: Color, k: f32| premul(mix(c, Color::BLACK, k), opacity);
        let light = |c: Color, k: f32| premul(mix(c, Color::WHITE, k), opacity);
        self.quad(painter, base, ferrule, w / 2.0, shade(g.body, 0.25));
        self.quad(
            painter,
            base + n * w * 0.18,
            ferrule + n * w * 0.18,
            w * 0.2,
            light(g.body, 0.12),
        );
        self.quad(
            painter,
            base - n * w * 0.30,
            ferrule - n * w * 0.30,
            w * 0.12,
            shade(g.body, 0.45),
        );
        self.quad(
            painter,
            ferrule,
            end,
            w * 0.52,
            light(Color::from_rgb(170, 168, 160), 0.1),
        );
        for k in 1..4 {
            let p = ferrule + d * (k as f32 * 4.5 * scale);
            painter.line_segment(
                [p + n * w * 0.52, p - n * w * 0.52],
                Stroke::new(1.0 * scale, shade(Color::from_rgb(150, 148, 140), 0.3)),
            );
        }
        self.quad(
            painter,
            end,
            end + d * 16.0 * scale,
            w * 0.48,
            premul(Color::from_rgb(214, 132, 128), opacity),
        );
    }

    /// The sharpened wood, scalloped where the blade cut it, and the lead.
    fn point(&self, painter: &Painter, g: &Graphite, opacity: f32) {
        let (tip, d, n, w, scale, base) = (self.tip, self.d, self.n, self.w, self.scale, self.base);
        let wood = Color::from_rgb(231, 203, 164);
        painter.convex_polygon(
            vec![
                base + n * w / 2.0,
                tip + d * 7.0 * scale + n * w * 0.12,
                tip + d * 7.0 * scale - n * w * 0.12,
                base - n * w / 2.0,
            ],
            premul(wood, opacity),
            Stroke::NONE,
        );
        painter.convex_polygon(
            vec![
                base - n * w / 2.0,
                tip + d * 7.0 * scale - n * w * 0.12,
                base - n * w * 0.1,
            ],
            premul(mix(wood, Color::BLACK, 0.12), opacity),
            Stroke::NONE,
        );
        painter.convex_polygon(
            vec![
                tip + d * 8.0 * scale + n * w * 0.14,
                tip,
                tip + d * 8.0 * scale - n * w * 0.14,
            ],
            premul(g.lead, opacity),
            Stroke::NONE,
        );
    }
}

/// Pencil strokes as far as the pencil has come: a soft graphite line with
/// a lighter, slightly offset second pass, the way a pencil line breaks up
/// on paper.
pub(super) fn graphite_lines(
    painter: &Painter,
    pic: &Strokes,
    now: f32,
    rect: Rect,
    scale: f32,
    g: &Graphite,
    opacity: f32,
) {
    if opacity <= 0.0 || pic.points.len() < 2 {
        return;
    }
    let t = now - pic.born;
    let w = 2.0 * scale * pic.weight.max(0.6);
    for i in 1..pic.points.len() {
        if !pic.pen[i] || pic.at[i - 1] > t {
            continue;
        }
        let a = to_screen(pic.points[i - 1], rect);
        let mut b = to_screen(pic.points[i], rect);
        if pic.at[i] > t {
            let f = (t - pic.at[i - 1]) / (pic.at[i] - pic.at[i - 1]).max(1e-4);
            b = a + (b - a) * f.clamp(0.0, 1.0);
        }
        let pressure = 0.7 + 0.3 * hash01(i as u32 * 7);
        if pic.fill[i] {
            // shading: light, quick hatching
            painter.line_segment(
                [a, b],
                Stroke::new(
                    w * 0.45,
                    premul(g.lead, 0.42 * pressure * opacity * pic.weight.max(0.4)),
                ),
            );
            continue;
        }
        let n = across((b - a).normalized()) * (0.9 * scale);
        painter.line_segment(
            [a, b],
            Stroke::new(
                w,
                premul(g.lead, 0.78 * pressure * opacity * pic.weight.max(0.4)),
            ),
        );
        painter.line_segment(
            [a + n, b + n],
            Stroke::new(
                w * 0.5,
                premul(g.lead, 0.28 * opacity * pic.weight.max(0.4)),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pencil_leans_up_and_to_the_right() {
        let pose = PencilPose::new(Pos2::new(100.0, 100.0), 1.0, 0.0);
        assert!(pose.end.x > pose.tip.x && pose.end.y < pose.tip.y);
        assert!((pose.n.length() - 1.0).abs() < 1e-5);
    }
}
