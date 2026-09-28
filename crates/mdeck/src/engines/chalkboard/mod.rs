//! The chalkboard engine: every slide is a slate in a wooden frame (the
//! theme's `page:`), with the ghosts of earlier drawings wiped off it. The
//! slide's generated line art (`mdeck ai art`) is drawn in chalk, stroke by
//! stroke, the chalk breaking up on the slate and shedding dust as it goes.
//! Without art, the slide's `@illustration` is drawn in chalk; the
//! countdown and the end words are drawn the same way. Exports show the
//! finished board.

use eframe::egui;

use super::Engine;
use super::art::{Canvas, Reveal};
use super::led::{premul, sprite_sheet};
use super::stage::{FrameCx, Stage};
use crate::render::art::prepare::Strategy;
use crate::render::art::{ArtKind, Medium, style};
use crate::render::illustration::Library;
use crate::render::strokes::to_screen;
use chalk::{Chalk, Mote, chalk_lines, dust, emit, slate, step, stick};

mod chalk;

/// A chalkboard asks for line art and draws it in chalk.
pub static MEDIUM: Medium = Medium {
    name: "chalkboard",
    kind: ArtKind::Line,
    tonal: &style::SKETCH,
    tonal_strategy: Strategy::Hatch,
};

/// Seconds into the end slide when the caption fades in.
pub const END_CAPTION_DELAY: f32 = 5.2;
/// The end words hold this long, then fade.
const END_WORDS: f32 = 3.6;
/// Seconds to draw a picture.
const DRAW: f32 = 3.6;

const REVEAL: Reveal = Reveal {
    soft: 0.02,
    ghost: 0.0,
    ghost_speed: 1.0,
    grain: 0.72,
};

pub struct Chalkboard {
    canvas: Canvas,
    motes: Vec<Mote>,
    seed: u32,
    sprites: Option<egui::TextureHandle>,
}

impl Chalkboard {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(DRAW, 0.0, END_WORDS, 0.7),
            motes: Vec::new(),
            seed: 0x1234_5679,
            sprites: None,
        }
    }
}

impl Default for Chalkboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Chalkboard {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, lib: &mut Library) {
        self.canvas.update(cx, stage, lib);
        if cx.still {
            self.motes.clear();
            return;
        }
        let c = &self.canvas;
        let tip = c
            .drawing
            .as_ref()
            .filter(|d| !d.backdrop)
            .and_then(|d| d.tip(c.now, cx.rect))
            .or_else(|| {
                let p = c.strokes.as_ref()?;
                let (tip, on) = p.tip(c.now - p.born)?;
                on.then(|| to_screen(tip, cx.rect))
            });
        if let Some(tip) = tip {
            emit(&mut self.motes, tip, cx.scale, cx.dt, &mut self.seed);
        }
        step(&mut self.motes, cx.dt, cx.scale);
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, _stage: &Stage) {
        let texture = self
            .sprites
            .get_or_insert_with(|| {
                ui.ctx().load_texture(
                    "mdeck-chalkboard-sprites",
                    sprite_sheet(),
                    egui::TextureOptions::LINEAR,
                )
            })
            .id();
        let rect = cx.rect;
        let scale = cx.scale;
        let chalk = Chalk::of(cx.theme);
        let painter = ui.painter();
        slate(painter, texture, rect, scale, &chalk, cx.opacity);

        let c = &mut self.canvas;
        let now = c.now;
        let left = c.fading.as_ref().map(|(_, since)| c.fade_left(*since));
        if let (Some((old, _)), Some(left)) = (&mut c.fading, left) {
            old.paint(
                ui,
                rect,
                now,
                premul(chalk.white, left * cx.opacity),
                REVEAL,
            );
        }
        if let Some((old, since)) = &c.fading_strokes {
            let k = c.fade_left(*since) * cx.opacity;
            chalk_lines(painter, old, now, rect, scale, &chalk, k);
        }
        let burst = c.burst_left();
        let mut tip = None;
        if let Some(d) = &mut c.drawing {
            let k = if d.backdrop { 0.42 } else { 1.0 } * cx.opacity;
            d.paint(ui, rect, now, premul(chalk.white, k), REVEAL);
            if !d.backdrop {
                tip = d.tip(now, rect);
            }
        }
        if let Some(p) = &c.strokes {
            chalk_lines(painter, p, now, rect, scale, &chalk, burst * cx.opacity);
            if let Some((t, true)) = p.tip(now - p.born) {
                tip = Some(to_screen(t, rect));
            }
        }
        dust(painter, &self.motes, &chalk, cx.opacity);
        if !cx.still
            && let Some(tip) = tip
        {
            stick(painter, tip, scale, &chalk, cx.opacity);
        }
        if (c.busy() || !self.motes.is_empty()) && !cx.still {
            ui.ctx().request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chalkboard_asks_for_line_art_and_breaks_it_up() {
        assert_eq!(MEDIUM.kind, ArtKind::Line);
        const { assert!(REVEAL.grain > 0.0) };
        assert_eq!(
            super::super::EngineKind::Chalkboard
                .medium()
                .map(|m| m.name),
            Some("chalkboard")
        );
    }
}
