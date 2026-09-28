//! The darkroom engine: slides in a darkroom under a red safelight. The
//! slide's generated black-and-white photograph (`mdeck ai art`) is a print
//! on white fibre paper that develops in place, the shadows first and the
//! highlights last, under the safelight's red; when it is done the white
//! light comes on and the print shows its true greys. Without art, the
//! slide's `@illustration` becomes a photogram: its shape left white on a
//! black print. The countdown and the end words glow the same way. Exports
//! show the finished print.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

use super::art::{Canvas, Drawing, Hand, Reveal};
use super::paint::{SPRITE_GLOW, Sprites, additive, mix, premul, smoothstep};
use super::stage::{FrameCx, Moment, Stage};
use super::{Engine, EngineDef};
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

pub static DEF: EngineDef = EngineDef {
    capabilities: super::art::CAPABILITIES,
    create: || Box::new(Darkroom::new()),
    end_caption_delay: END_CAPTION_DELAY,
    medium: Some(&MEDIUM),
    render_slide: None,
    problems: None,
};
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
    sprites: Sprites,
}

impl Darkroom {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(DRAW, LIGHTS, END_WORDS, 0.6),
            sprites: Sprites::new("mdeck-darkroom-sprites"),
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
        let texture = self.sprites.id(ui.ctx());
        let rect = cx.rect;
        // under the safelight the paper reads a deep, dim red, not the
        // lamp's own colour
        let safelight = mix(cx.theme.accent, Color32::from_rgb(96, 40, 36), 0.5);

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
        ui.painter().add(egui::Shape::mesh(mesh));

        let hand = Bench {
            safelight,
            slide: matches!(stage.moment, Moment::Slide),
        };
        self.canvas.paint(ui, cx, &hand);
    }
}

/// The bench under the safelight: prints on white fibre paper that develop
/// and then see the white light, and photograms for pen strokes.
struct Bench {
    safelight: Color32,
    /// On a slide (not the countdown or the end): photograms lie on a print.
    slide: bool,
}

impl Hand for Bench {
    fn backdrop(&self) -> f32 {
        0.3
    }

    fn picture(
        &self,
        ui: &egui::Ui,
        cx: &FrameCx,
        d: &mut Drawing,
        now: f32,
        k: f32,
        current: bool,
    ) {
        let scale = cx.scale;
        // the old print fades under the white light; a new one develops
        // under the safelight, then the light comes on
        let on = if current {
            smoothstep(0.0, 1.0, (now - d.born - DRAW) / LIGHTS)
        } else {
            1.0
        };
        let line = d.picture.strategy == Strategy::Draw;
        let lit = light(self.safelight, on);
        let box_ = d.screen(cx.rect);
        if !d.backdrop {
            // the print: white fibre paper with a border, and its shadow
            let border = box_.expand(box_.width().max(box_.height()) * 0.035);
            ui.painter().rect_filled(
                border.translate(egui::vec2(8.0, 12.0) * scale),
                2.0 * scale,
                Color32::from_black_alpha((110.0 * k) as u8),
            );
            ui.painter()
                .rect_filled(border, 2.0 * scale, premul(multiply(PAPER, lit), k));
        }
        let tint = if line {
            premul(multiply(Color32::from_gray(24), lit), k)
        } else {
            premul(lit, k)
        };
        d.paint(
            ui,
            cx.rect,
            now,
            tint,
            if line { LINE_REVEAL } else { REVEAL },
        );
    }

    fn strokes(
        &self,
        painter: &egui::Painter,
        cx: &FrameCx,
        p: &Picture,
        now: f32,
        k: f32,
        current: bool,
    ) {
        let print = current && self.slide;
        photogram(painter, cx, p, now, self.safelight, k, print);
    }
}

/// White fibre paper.
const PAPER: Color32 = Color32::from_rgb(244, 241, 234);

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
fn photogram(
    painter: &egui::Painter,
    cx: &FrameCx,
    pic: &Picture,
    now: f32,
    safelight: Color32,
    opacity: f32,
    print: bool,
) {
    let (rect, scale) = (cx.rect, cx.scale);
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
        painter.rect_filled(border, 2.0 * scale, premul(multiply(PAPER, lit), opacity));
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
