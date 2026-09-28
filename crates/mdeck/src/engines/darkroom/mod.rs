//! The darkroom engine: slides in a darkroom under a red safelight. The
//! slide's generated black-and-white photograph (`mdeck ai art`) is a print
//! on white fibre paper that develops in place, the shadows first and the
//! highlights last, under the safelight's red; when it is done the white
//! light comes on and the print shows its true greys. Without art, the
//! slide's `@illustration` becomes a photogram: its shape left white on a
//! black print. The countdown and the end words glow the same way. Exports
//! show the finished print.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

use super::Engine;
use super::art::{Canvas, Reveal};
use super::led::{SPRITE_GLOW, additive, mix, premul, sprite_sheet};
use super::stage::{FrameCx, Moment, Stage};
use crate::render::art::prepare::Strategy;
use crate::render::art::{ArtKind, Medium, style};
use crate::render::illustration::Library;
use crate::render::strokes::{Picture, to_screen};

/// A darkroom asks for photographs and develops them.
pub static MEDIUM: Medium = Medium {
    name: "darkroom",
    kind: ArtKind::Tonal,
    tonal: &style::DARKROOM,
    tonal_strategy: Strategy::Develop,
};

/// Seconds into the end slide when the caption fades in.
pub const END_CAPTION_DELAY: f32 = 5.4;
/// The end words hold this long, then fade.
const END_WORDS: f32 = 3.8;
/// Seconds for a print to develop.
const DRAW: f32 = 4.0;
/// Seconds for the white light to come on after.
const LIGHTS: f32 = 0.9;

const REVEAL: Reveal = Reveal {
    soft: 0.35,
    ghost: 0.0,
    ghost_speed: 1.0,
    grain: 0.0,
};

/// Line art in the darkroom is printed like a drawing on photo paper.
const LINE_REVEAL: Reveal = Reveal {
    soft: 0.05,
    ghost: 0.0,
    ghost_speed: 1.0,
    grain: 0.0,
};

pub struct Darkroom {
    canvas: Canvas,
    sprites: Option<egui::TextureHandle>,
}

impl Darkroom {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(DRAW, LIGHTS, END_WORDS, 0.6),
            sprites: None,
        }
    }
}

impl Default for Darkroom {
    fn default() -> Self {
        Self::new()
    }
}

/// The safelight's red over the white light, as the light comes on (0..1).
fn light(safelight: Color32, on: f32) -> Color32 {
    mix(safelight, Color32::WHITE, on)
}

impl Engine for Darkroom {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, lib: &mut Library) {
        self.canvas.update(cx, stage, lib);
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, stage: &Stage) {
        let texture = self
            .sprites
            .get_or_insert_with(|| {
                ui.ctx().load_texture(
                    "mdeck-darkroom-sprites",
                    sprite_sheet(),
                    egui::TextureOptions::LINEAR,
                )
            })
            .id();
        let theme = cx.theme;
        let rect = cx.rect;
        let scale = cx.scale;
        let painter = ui.painter();
        // under the safelight the paper reads a deep, dim red, not the
        // lamp's own colour
        let safelight = mix(theme.accent, Color32::from_rgb(96, 40, 36), 0.5);
        let paper = Color32::from_rgb(244, 241, 234);

        // the safelight hangs above the bench and reddens the room
        let mut mesh = egui::Mesh::with_texture(texture);
        let r = rect.width() * 0.55;
        mesh.add_rect_with_uv(
            Rect::from_center_size(
                Pos2::new(
                    rect.right() - rect.width() * 0.18,
                    rect.top() - rect.height() * 0.05,
                ),
                egui::vec2(r * 2.0, r * 1.6),
            ),
            SPRITE_GLOW,
            additive(safelight, 0.16 * cx.opacity),
        );
        painter.add(egui::Shape::mesh(mesh));

        let c = &mut self.canvas;
        let now = c.now;
        let paint_print = |ui: &egui::Ui, d: &mut super::art::Drawing, k: f32, on: f32| {
            let line = d.picture.strategy == Strategy::Draw;
            let lit = light(safelight, on);
            let box_ = d.screen(rect);
            if !d.backdrop {
                // the print: white fibre paper with a border, and its shadow
                let border = box_.expand(box_.width().max(box_.height()) * 0.035);
                ui.painter().rect_filled(
                    border.translate(egui::vec2(8.0, 12.0) * scale),
                    2.0 * scale,
                    Color32::from_black_alpha((110.0 * k) as u8),
                );
                ui.painter()
                    .rect_filled(border, 2.0 * scale, premul(multiply(paper, lit), k));
            }
            let tint = if line {
                premul(multiply(Color32::from_gray(24), lit), k)
            } else {
                premul(lit, k)
            };
            d.paint(ui, rect, now, tint, if line { LINE_REVEAL } else { REVEAL });
        };

        let left = c.fading.as_ref().map(|(_, since)| c.fade_left(*since));
        if let (Some((old, _)), Some(left)) = (&mut c.fading, left) {
            paint_print(ui, old, left * cx.opacity, 1.0);
        }
        if let Some((old, since)) = &c.fading_strokes {
            let k = c.fade_left(*since) * cx.opacity;
            photogram(painter, old, now, rect, scale, safelight, k, false);
        }
        let burst = c.burst_left();
        if let Some(d) = &mut c.drawing {
            let k = if d.backdrop { 0.3 } else { 1.0 } * cx.opacity;
            let on = ((now - d.born - DRAW) / LIGHTS).clamp(0.0, 1.0);
            let on = on * on * (3.0 - 2.0 * on);
            paint_print(ui, d, k, on);
        }
        if let Some(p) = &c.strokes {
            let print = matches!(stage.moment, Moment::Slide);
            photogram(
                painter,
                p,
                now,
                rect,
                scale,
                safelight,
                burst * cx.opacity,
                print,
            );
        }
        if c.busy() && !cx.still {
            ui.ctx().request_repaint();
        }
    }
}

/// `a` lit by `light`.
fn multiply(a: Color32, light: Color32) -> Color32 {
    let m = |x: u8, y: u8| ((x as u16 * y as u16) / 255) as u8;
    Color32::from_rgb(
        m(a.r(), light.r()),
        m(a.g(), light.g()),
        m(a.b(), light.b()),
    )
}

/// A photogram: the strokes left white where they shielded the paper, with
/// a soft halo. On a slide it lies on a black print with a white border.
#[allow(clippy::too_many_arguments)]
fn photogram(
    painter: &egui::Painter,
    pic: &Picture,
    now: f32,
    rect: Rect,
    scale: f32,
    safelight: Color32,
    opacity: f32,
    print: bool,
) {
    if opacity <= 0.0 || pic.points.len() < 2 {
        return;
    }
    let t = now - pic.born;
    let done = (t / pic.duration.max(0.1)).clamp(0.0, 1.0);
    let lit = light(safelight, ((t - pic.duration) / LIGHTS).clamp(0.0, 1.0));
    if print {
        let (mut lo, mut hi) = (Pos2::new(1.0, 1.0), Pos2::new(0.0, 0.0));
        for p in &pic.points {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        let b = Rect::from_min_max(to_screen(lo, rect), to_screen(hi, rect)).expand(34.0 * scale);
        let border = b.expand(18.0 * scale);
        painter.rect_filled(
            border.translate(egui::vec2(8.0, 12.0) * scale),
            2.0 * scale,
            Color32::from_black_alpha((110.0 * opacity) as u8),
        );
        painter.rect_filled(
            border,
            2.0 * scale,
            premul(multiply(Color32::from_rgb(244, 241, 234), lit), opacity),
        );
        // the paper darkens as it develops; the shape stays white
        painter.rect_filled(
            b,
            0.0,
            premul(Color32::from_gray(14), opacity * (0.35 + 0.65 * done)),
        );
    }
    let white = multiply(Color32::from_rgb(236, 233, 226), lit);
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
        shapes.push(egui::Shape::line_segment(
            [a, b],
            Stroke::new(12.0 * scale, premul(white, 0.10 * opacity)),
        ));
        shapes.push(egui::Shape::line_segment(
            [a, b],
            Stroke::new(
                4.0 * scale,
                premul(white, 0.9 * opacity * pic.weight.max(0.4)),
            ),
        ));
    }
    painter.extend(shapes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_darkroom_develops_photographs() {
        assert_eq!(MEDIUM.kind, ArtKind::Tonal);
        assert_eq!(MEDIUM.tonal_strategy, Strategy::Develop);
        assert_eq!(
            super::super::EngineKind::Darkroom.medium().map(|m| m.name),
            Some("darkroom")
        );
    }

    #[test]
    fn the_light_comes_on_from_red_to_white() {
        let red = Color32::from_rgb(255, 60, 40);
        assert_eq!(light(red, 0.0), red);
        assert_eq!(light(red, 1.0), Color32::WHITE);
        assert_eq!(multiply(Color32::WHITE, red), red);
    }
}
