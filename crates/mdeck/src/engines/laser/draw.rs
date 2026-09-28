//! Painting the etching: the colours a mark cools through, the marks
//! themselves, the beam, and the grain of the surface.

use eframe::egui::{self, Color32, Pos2, Rect};

use super::{HEAT, LINGER};
use crate::engines::hash01;
use crate::engines::paint::{SPRITE_CORE, additive, mix, premul};
use crate::engines::stage::FrameCx;
use crate::render::strokes::{Picture, to_screen};
use crate::theme::Theme;

/// The etching's colours: white-hot, hot, cooling, and the etched line.
pub(super) struct Ink {
    pub(super) white: Color32,
    pub(super) hot: Color32,
    pub(super) warm: Color32,
    pub(super) etched: Color32,
    pub(super) groove: Color32,
    pub(super) smoke: Color32,
    pub(super) light: bool,
}

impl Ink {
    pub(super) fn of(theme: &Theme) -> Self {
        let light = theme.is_light();
        Ink {
            white: theme.particle_light,
            hot: theme.accent,
            warm: theme.secondary,
            etched: if light {
                mix(theme.background, theme.heading_color, 0.7)
            } else {
                mix(
                    mix(theme.background, theme.heading_color, 0.80),
                    theme.accent_soft,
                    0.12,
                )
            },
            groove: mix(
                theme.background,
                Color32::BLACK,
                if light { 0.25 } else { 0.55 },
            ),
            smoke: if light {
                mix(theme.background, Color32::BLACK, 0.4)
            } else {
                mix(theme.background, theme.foreground, 0.55)
            },
            light,
        }
    }

    /// A mark's colour `heat` (1: just etched, 0: cold).
    pub(super) fn at(&self, heat: f32) -> Color32 {
        if heat > 0.66 {
            mix(self.warm, self.white, (heat - 0.66) / 0.34)
        } else if heat > 0.33 {
            mix(self.hot, self.warm, (heat - 0.33) / 0.33)
        } else {
            mix(self.etched, self.hot, heat / 0.33)
        }
    }
}

/// Draw an etching as far as the beam has come: each mark cools with age,
/// rough edged, over a groove, with a lingering glow while fresh. `flare`
/// (the countdown's burst) heats everything again as it burns away.
pub(super) fn etching(
    mesh: &mut egui::Mesh,
    cx: &FrameCx,
    pic: &Picture,
    now: f32,
    ink: &Ink,
    opacity: f32,
    flare: Option<f32>,
) {
    let (rect, scale) = (cx.rect, cx.scale);
    if opacity <= 0.0 || pic.points.len() < 2 {
        return;
    }
    let t = now - pic.born;
    let width = 2.8 * scale;
    let mut glows: Vec<(Pos2, Pos2, f32)> = Vec::new();
    for i in 1..pic.points.len() {
        if !pic.pen[i] || pic.at[i - 1] > t {
            continue;
        }
        let a = to_screen(pic.points[i - 1], rect);
        let mut b = to_screen(pic.points[i], rect);
        if pic.at[i] > t {
            // the segment being etched right now
            let f = (t - pic.at[i - 1]) / (pic.at[i] - pic.at[i - 1]).max(1e-4);
            b = a + (b - a) * f.clamp(0.0, 1.0);
        }
        let age = (t - pic.at[i]).max(0.0);
        let mut heat = (-age / HEAT).exp();
        if let Some(f) = flare {
            heat = heat.max((1.0 - f * 1.6).clamp(0.0, 1.0));
        }
        let color = ink.at(heat);
        let w = width * (1.0 + 0.7 * heat) * pic.weight.max(0.6);
        // groove under the mark, the mark, and a rough second pass beside it
        let n = (b - a).normalized().rot90();
        line(
            mesh,
            a + n * 0.7 * scale,
            b + n * 0.7 * scale,
            w,
            premul(ink.groove, 0.5 * opacity * pic.weight),
        );
        line(
            mesh,
            a,
            b,
            w,
            premul(color, opacity * (0.55 + 0.45 * pic.weight)),
        );
        let j = (hash01(i as u32 * 13) - 0.5) * 1.6 * scale;
        line(
            mesh,
            a + n * j,
            b - n * j,
            w * 0.55,
            premul(mix(color, ink.white, 0.15), 0.45 * opacity * pic.weight),
        );
        // a faint warmth stays in the engraving after the glow has gone
        let linger = (-age / LINGER).exp() * 0.5 + heat * 0.8 + 0.12;
        if linger > 0.03 && !ink.light {
            glows.push((a, b, linger));
        }
    }
    for (a, b, g) in glows {
        line(
            mesh,
            a,
            b,
            9.0 * scale,
            additive(ink.hot, 0.16 * g * opacity * pic.weight),
        );
    }
}

/// A tapered beam from `from` (wide, faint) to `to` (thin, bright).
pub(super) fn beam(
    mesh: &mut egui::Mesh,
    from: Pos2,
    to: Pos2,
    w0: f32,
    w1: f32,
    ink: &Ink,
    k: f32,
) {
    let n = (to - from).normalized().rot90();
    let base = mesh.vertices.len() as u32;
    let uv = SOLID_UV;
    for (p, w, c) in [
        (from, w0, additive(ink.hot, 0.05 * k)),
        (to, w1 * 3.0, additive(ink.hot, 0.55 * k)),
    ] {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p + n * w,
            uv,
            color: c,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p - n * w,
            uv,
            color: c,
        });
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 1, base + 3, base + 2);
    // the white core, brightest at the tip
    let base = mesh.vertices.len() as u32;
    for (p, w, c) in [
        (from, w1 * 0.8, additive(ink.white, 0.0)),
        (to, w1, additive(ink.white, 0.9 * k)),
    ] {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p + n * w,
            uv,
            color: c,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p - n * w,
            uv,
            color: c,
        });
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 1, base + 3, base + 2);
}

/// The middle of the sprite sheet's core: a white texel, for untextured quads.
const SOLID_UV: Pos2 = Pos2 { x: 0.5, y: 0.5 };

pub(super) fn line(mesh: &mut egui::Mesh, a: Pos2, b: Pos2, w: f32, color: Color32) {
    let d = b - a;
    if d.length() < 0.01 {
        return;
    }
    let n = d.normalized().rot90() * (w / 2.0);
    let base = mesh.vertices.len() as u32;
    for p in [a + n, a - n, b + n, b - n] {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p,
            uv: SOLID_UV,
            color,
        });
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 1, base + 3, base + 2);
}

/// The surface being etched: a fine, even grain, the same on every slide.
pub(super) fn surface(mesh: &mut egui::Mesh, rect: Rect, theme: &Theme, scale: f32, opacity: f32) {
    let grain = if theme.is_light() {
        mix(theme.background, Color32::BLACK, 0.5)
    } else {
        mix(theme.background, Color32::WHITE, 0.5)
    };
    for k in 0..900u32 {
        let x = hash01(k * 3 + 1);
        let y = hash01(k * 3 + 2);
        let a = 0.025 + 0.05 * hash01(k * 3 + 3);
        let r = (0.6 + 0.9 * hash01(k * 7 + 5)) * scale;
        let p = Pos2::new(
            rect.left() + x * rect.width(),
            rect.top() + y * rect.height(),
        );
        mesh.add_rect_with_uv(
            Rect::from_center_size(p, egui::vec2(r * 2.0, r * 2.0)),
            SPRITE_CORE,
            premul(grain, a * opacity),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_cool_from_white_to_the_etched_colour() {
        let ink = Ink::of(&Theme::dark());
        assert_eq!(ink.at(1.0), ink.white);
        assert_eq!(ink.at(0.0), ink.etched);
    }
}
