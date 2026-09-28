//! The watercolour engine: every slide is a sheet of cold-press paper (the
//! theme's `page:`), and the slide's generated watercolour (`mdeck ai art`)
//! blooms onto it: a pale first wash over the whole picture, then the
//! colour spreading outward from where the paint is heaviest, the dark
//! accents dropped in last. Without art, the slide's `@illustration` is
//! drawn in ink with a loose wash beside it; the countdown and the end
//! words are painted the same way. Exports show the dry painting.

use eframe::egui::{self, Color32, Stroke};

use super::Engine;
use super::art::{Canvas, Reveal};
use super::hash01;
use super::led::{mix, premul};
use super::stage::{FrameCx, Stage};
use crate::render::art::prepare::Strategy;
use crate::render::art::{ArtKind, Medium, style};
use crate::render::illustration::Library;
use crate::render::strokes::{Picture, to_screen};
use crate::theme::Theme;

/// Watercolour asks for finished paintings and lets them bloom.
pub static MEDIUM: Medium = Medium {
    name: "watercolour",
    kind: ArtKind::Tonal,
    tonal: &style::WATERCOLOUR,
    tonal_strategy: Strategy::Bloom,
};

/// Seconds into the end slide when the caption fades in.
pub const END_CAPTION_DELAY: f32 = 5.4;
/// The end words hold this long, then fade.
const END_WORDS: f32 = 3.8;
/// Seconds for a painting to bloom.
const DRAW: f32 = 4.6;

/// Wet edges: pixels take a long while to arrive, and a pale first wash
/// runs well ahead of the colour.
const REVEAL: Reveal = Reveal {
    soft: 0.10,
    ghost: 0.12,
    ghost_speed: 3.0,
    grain: 0.0,
};

/// Line art on watercolour paper is an ink drawing.
const LINE_REVEAL: Reveal = Reveal {
    soft: 0.03,
    ghost: 0.0,
    ghost_speed: 1.0,
    grain: 0.0,
};

pub struct Watercolour {
    canvas: Canvas,
}

impl Watercolour {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(DRAW, 0.0, END_WORDS, 0.8),
        }
    }
}

impl Default for Watercolour {
    fn default() -> Self {
        Self::new()
    }
}

/// The paints: an ink for lines and a wash colour.
struct Paint {
    ink: Color32,
    washes: [Color32; 3],
}

impl Paint {
    fn of(theme: &Theme) -> Self {
        Paint {
            ink: mix(theme.heading_color, theme.background, 0.1),
            washes: [theme.accent, theme.secondary, theme.accent_soft],
        }
    }
}

impl Engine for Watercolour {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, lib: &mut Library) {
        self.canvas.update(cx, stage, lib);
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, _stage: &Stage) {
        let rect = cx.rect;
        let scale = cx.scale;
        let paint = Paint::of(cx.theme);
        let painter = ui.painter();
        let c = &mut self.canvas;
        let now = c.now;
        let tint = |line: bool, k: f32| {
            if line {
                premul(paint.ink, k)
            } else {
                premul(Color32::WHITE, k)
            }
        };
        let left = c.fading.as_ref().map(|(_, since)| c.fade_left(*since));
        if let (Some((old, _)), Some(left)) = (&mut c.fading, left) {
            let line = old.picture.strategy == Strategy::Draw;
            let r = if line { LINE_REVEAL } else { REVEAL };
            old.paint(ui, rect, now, tint(line, left * cx.opacity), r);
        }
        if let Some((old, since)) = &c.fading_strokes {
            let k = c.fade_left(*since) * cx.opacity;
            ink_and_wash(painter, old, now, rect, scale, &paint, k);
        }
        let burst = c.burst_left();
        if let Some(d) = &mut c.drawing {
            let line = d.picture.strategy == Strategy::Draw;
            let r = if line { LINE_REVEAL } else { REVEAL };
            let k = if d.backdrop { 0.35 } else { 1.0 } * cx.opacity;
            d.paint(ui, rect, now, tint(line, k), r);
        }
        if let Some(p) = &c.strokes {
            ink_and_wash(painter, p, now, rect, scale, &paint, burst * cx.opacity);
        }
        if c.busy() && !cx.still {
            ui.ctx().request_repaint();
        }
    }
}

/// Pen strokes in ink, with a loose wash laid along them a moment later:
/// wide, pale and a little off the line, the way a quick sketch is washed.
fn ink_and_wash(
    painter: &egui::Painter,
    pic: &Picture,
    now: f32,
    rect: egui::Rect,
    scale: f32,
    paint: &Paint,
    opacity: f32,
) {
    if opacity <= 0.0 || pic.points.len() < 2 {
        return;
    }
    let t = now - pic.born;
    let wash_t = t - 0.5;
    let mut washes = Vec::new();
    let mut lines = Vec::new();
    for i in 1..pic.points.len() {
        if !pic.pen[i] {
            continue;
        }
        let a = to_screen(pic.points[i - 1], rect);
        let b = to_screen(pic.points[i], rect);
        if pic.at[i - 1] <= wash_t {
            let colour = paint.washes[(i / 40) % 3];
            let off = egui::vec2(
                hash01(i as u32 / 12) - 0.5,
                hash01(i as u32 / 12 + 99) - 0.5,
            ) * 14.0
                * scale;
            washes.push(egui::Shape::line_segment(
                [a + off, b + off],
                Stroke::new(30.0 * scale, premul(colour, 0.10 * opacity * pic.weight)),
            ));
        }
        if pic.at[i - 1] > t {
            continue;
        }
        let mut b = b;
        if pic.at[i] > t {
            let f = (t - pic.at[i - 1]) / (pic.at[i] - pic.at[i - 1]).max(1e-4);
            b = a + (b - a) * f.clamp(0.0, 1.0);
        }
        lines.push(egui::Shape::line_segment(
            [a, b],
            Stroke::new(
                1.8 * scale,
                premul(paint.ink, 0.85 * opacity * pic.weight.max(0.4)),
            ),
        ));
    }
    painter.extend(washes);
    painter.extend(lines);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watercolour_asks_for_paintings_that_bloom() {
        assert_eq!(MEDIUM.kind, ArtKind::Tonal);
        assert_eq!(MEDIUM.tonal_strategy, Strategy::Bloom);
        assert_eq!(
            super::super::EngineKind::Watercolour
                .medium()
                .map(|m| m.name),
            Some("watercolour")
        );
    }
}
