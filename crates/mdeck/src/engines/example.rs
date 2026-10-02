//! The minimal engine from `doc/engines.md`, compiled and tested so the
//! guide cannot drift from the real interface. It draws the slide's
//! illustration as dots that fade in, and nothing else.

use eframe::egui;

use super::Engine;
use super::stage::{FrameCx, Stage};
use crate::render::illustration::Library;

// --- guide: begin ---
/// Dots: the slide's illustration as accent-coloured dots that fade in.
pub struct Dots {
    /// The slide the fade belongs to, and how far it has come (0..1).
    slide: Option<usize>,
    shown: f32,
}

impl Dots {
    pub fn new() -> Self {
        Self {
            slide: None,
            shown: 0.0,
        }
    }
}

impl Engine for Dots {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, _lib: &mut Library) {
        // A new slide starts the fade over; export (`still`) shows it done.
        if self.slide != Some(stage.index) {
            self.slide = Some(stage.index);
            self.shown = 0.0;
        }
        self.shown = if cx.still {
            1.0
        } else {
            (self.shown + cx.dt / 0.6).min(1.0)
        };
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, stage: &Stage) {
        let Some(figure) = &stage.figure else {
            return;
        };
        let rect = cx.rect;
        let place = figure.place;
        let alpha = self.shown * cx.opacity * if figure.backdrop { 0.35 } else { 1.0 };
        let color = cx.theme.accent.gamma_multiply(alpha);
        for p in figure.cloud.points.iter() {
            let pos = egui::pos2(
                rect.left() + (place.u + p[0] * place.w) * rect.width(),
                rect.top() + (place.v + p[1] * place.h) * rect.height(),
            );
            ui.painter().circle_filled(pos, 2.5 * cx.scale, color);
        }
        if self.shown < 1.0 {
            ui.ctx().request_repaint();
        }
    }
}
// --- guide: end ---

impl Default for Dots {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::engines::stage::{Figure, Moment, Place};
    use crate::render::illustration::{Cloud, VERSION};

    fn cloud() -> Arc<Cloud> {
        Arc::new(Cloud {
            version: VERSION,
            name: "dot".into(),
            description: String::new(),
            prompt: None,
            generated: None,
            aspect: 1.0,
            points: Arc::new(vec![[0.5, 0.5], [0.2, 0.8]]),
        })
    }

    /// The guide shows exactly the code between the markers above.
    #[test]
    fn the_guide_shows_this_code() {
        let src = include_str!("example.rs");
        let begin = src.find("// --- guide: begin ---\n").unwrap() + 24;
        let end = src.find("// --- guide: end ---").unwrap();
        let guide = include_str!("../../doc/engines.md");
        assert!(
            guide.contains(&src[begin..end]),
            "doc/engines.md must show engines/example.rs between the guide markers"
        );
    }

    #[test]
    fn dots_fade_in_live_and_are_done_in_export() {
        let theme = crate::theme::Theme::dark();
        let figure = Figure {
            cloud: cloud(),
            backdrop: false,
            place: Place {
                u: 0.5,
                v: 0.1,
                w: 0.4,
                h: 0.8,
            },
        };
        let stage = Stage {
            moment: Moment::Slide,
            index: 3,
            reveal: 0,
            slide: None,
            title: false,
            figure: Some(figure),
            art: None,
            hints: &[],
            hints_key: 0,
            deck_title: None,
            count: 1,
        };
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        let cx = |dt: f32, still: bool| FrameCx {
            rect,
            scale: 1.0,
            opacity: 1.0,
            dt,
            still,
            theme: &theme,
        };
        let mut lib = Library::for_deck(None);
        let mut dots = Dots::new();
        dots.update(&cx(0.3, false), &stage, &mut lib);
        assert!((dots.shown - 0.5).abs() < 1e-4, "{}", dots.shown);
        dots.update(&cx(0.0, true), &stage, &mut lib);
        assert_eq!(dots.shown, 1.0);
        // it paints without panicking in a headless frame
        crate::render::test_support::with_ui(|ui| dots.paint(ui, &cx(0.0, true), &stage));
    }
}
