//! The plain engine: slides on a flat background. It paints nothing; the
//! slide's own layouts do all the drawing.

use eframe::egui;

use super::Engine;
use super::stage::{FrameCx, Stage};
use crate::render::illustration::Library;

pub struct Plain;

impl Engine for Plain {
    fn update(&mut self, _cx: &FrameCx, _stage: &Stage, _lib: &mut Library) {}

    fn paint(&mut self, _ui: &egui::Ui, _cx: &FrameCx, _stage: &Stage) {}
}
