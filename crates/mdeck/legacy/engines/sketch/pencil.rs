//! The pencil: a sharpened pencil held at the drawing point, and graphite
//! lines for the pen strokes a sketch draws when a slide has no picture.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2, vec2};

use super::super::hash01;
use super::super::paint::{mix, premul};
use crate::render::strokes::{Picture, to_screen};
use crate::theme::Theme;

/// The sketch's colours.
pub(super) struct Graphite {
    /// Pencil lines.
    pub(super) lead: Color32,
    /// The pencil's painted body.
    pub(super) body: Color32,
}

impl Graphite {
    pub(super) fn of(theme: &Theme) -> Self {
        Graphite {
            lead: mix(theme.heading_color, theme.background, 0.12),
            body: theme.particle_cool,
        }
    }
}

/// A pencil whose point rests on `tip`, held from the lower right, with its
/// shadow on the paper. `wobble` (0..1) tilts it a little as the hand moves.
pub(super) fn pencil(
    painter: &egui::Painter,
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
        let d = vec2(theta.cos(), -theta.sin());
        let cone = 30.0 * scale;
        let body = 230.0 * scale;
        let base = tip + d * cone;
        let end = base + d * body;
        Self {
            tip,
            d,
            n: d.rot90(),
            w: 15.0 * scale,
            scale,
            base,
            ferrule: end - d * 18.0 * scale,
            end,
        }
    }

    /// A band along the axis from `a` to `b`, `half` wide on each side.
    fn quad(&self, a: Pos2, b: Pos2, half: f32, c: Color32) -> egui::Shape {
        let n = self.n;
        egui::Shape::convex_polygon(
            vec![a + n * half, b + n * half, b - n * half, a - n * half],
            c,
            Stroke::NONE,
        )
    }

    /// The shadow falls down and to the right of the pencil.
    fn shadow(&self, painter: &egui::Painter, opacity: f32) {
        let (n, w) = (self.n, self.w);
        let off = vec2(14.0, 18.0) * self.scale;
        for (k, a) in [(1.0, 0.07), (0.6, 0.08)] {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    self.tip + off * 0.15,
                    self.base + off + n * w * 0.5 * k,
                    self.end + off + n * w * 0.55 * k,
                    self.end + off - n * w * 0.55 * k,
                    self.base + off - n * w * 0.5 * k,
                ],
                Color32::from_black_alpha((a * 255.0 * opacity) as u8),
                Stroke::NONE,
            ));
        }
    }

    /// The painted body in three facets, the ferrule and the eraser.
    fn body(&self, painter: &egui::Painter, g: &Graphite, opacity: f32) {
        let (d, n, w, scale) = (self.d, self.n, self.w, self.scale);
        let (base, ferrule, end) = (self.base, self.ferrule, self.end);
        let shade = |c: Color32, k: f32| premul(mix(c, Color32::BLACK, k), opacity);
        let light = |c: Color32, k: f32| premul(mix(c, Color32::WHITE, k), opacity);
        painter.add(self.quad(base, ferrule, w / 2.0, shade(g.body, 0.25)));
        painter.add(self.quad(
            base + n * w * 0.18,
            ferrule + n * w * 0.18,
            w * 0.2,
            light(g.body, 0.12),
        ));
        painter.add(self.quad(
            base - n * w * 0.30,
            ferrule - n * w * 0.30,
            w * 0.12,
            shade(g.body, 0.45),
        ));
        painter.add(self.quad(
            ferrule,
            end,
            w * 0.52,
            light(Color32::from_rgb(170, 168, 160), 0.1),
        ));
        for k in 1..4 {
            let p = ferrule + d * (k as f32 * 4.5 * scale);
            painter.line_segment(
                [p + n * w * 0.52, p - n * w * 0.52],
                Stroke::new(1.0 * scale, shade(Color32::from_rgb(150, 148, 140), 0.3)),
            );
        }
        painter.add(self.quad(
            end,
            end + d * 16.0 * scale,
            w * 0.48,
            premul(Color32::from_rgb(214, 132, 128), opacity),
        ));
    }

    /// The sharpened wood, scalloped where the blade cut it, and the lead.
    fn point(&self, painter: &egui::Painter, g: &Graphite, opacity: f32) {
        let (tip, d, n, w, scale, base) = (self.tip, self.d, self.n, self.w, self.scale, self.base);
        let wood = Color32::from_rgb(231, 203, 164);
        painter.add(egui::Shape::convex_polygon(
            vec![
                base + n * w / 2.0,
                tip + d * 7.0 * scale + n * w * 0.12,
                tip + d * 7.0 * scale - n * w * 0.12,
                base - n * w / 2.0,
            ],
            premul(wood, opacity),
            Stroke::NONE,
        ));
        painter.add(egui::Shape::convex_polygon(
            vec![
                base - n * w / 2.0,
                tip + d * 7.0 * scale - n * w * 0.12,
                base - n * w * 0.1,
            ],
            premul(mix(wood, Color32::BLACK, 0.12), opacity),
            Stroke::NONE,
        ));
        painter.add(egui::Shape::convex_polygon(
            vec![
                tip + d * 8.0 * scale + n * w * 0.14,
                tip,
                tip + d * 8.0 * scale - n * w * 0.14,
            ],
            premul(g.lead, opacity),
            Stroke::NONE,
        ));
    }
}

/// Pencil strokes as far as the pencil has come: a soft graphite line with
/// a lighter, slightly offset second pass, the way a pencil line breaks up
/// on paper.
pub(super) fn graphite_lines(
    painter: &egui::Painter,
    pic: &Picture,
    now: f32,
    rect: egui::Rect,
    scale: f32,
    g: &Graphite,
    opacity: f32,
) {
    if opacity <= 0.0 || pic.points.len() < 2 {
        return;
    }
    let t = now - pic.born;
    let w = 2.0 * scale * pic.weight.max(0.6);
    let mut shapes = Vec::new();
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
        let n = (b - a).normalized().rot90() * (0.9 * scale);
        shapes.push(egui::Shape::line_segment(
            [a, b],
            Stroke::new(
                w,
                premul(g.lead, 0.78 * pressure * opacity * pic.weight.max(0.4)),
            ),
        ));
        shapes.push(egui::Shape::line_segment(
            [a + n, b + n],
            Stroke::new(
                w * 0.5,
                premul(g.lead, 0.28 * opacity * pic.weight.max(0.4)),
            ),
        ));
    }
    painter.extend(shapes);
}
