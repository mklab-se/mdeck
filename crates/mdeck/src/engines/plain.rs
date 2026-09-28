//! The plain engine: slides on a flat background. It paints nothing; the
//! slide's own layouts do all the drawing.

use eframe::egui;

use super::stage::{FrameCx, Stage};
use super::{Capabilities, Engine, EngineDef};
use crate::render::illustration::Library;

pub static DEF: EngineDef = EngineDef {
    capabilities: Capabilities::NONE,
    create: || Box::new(Plain),
    end_caption_delay: 0.0,
    medium: None,
    render_slide: None,
    problems: None,
};

pub struct Plain;

impl Engine for Plain {
    fn update(&mut self, _cx: &FrameCx, _stage: &Stage, _lib: &mut Library) {}

    fn paint(&mut self, _ui: &egui::Ui, _cx: &FrameCx, _stage: &Stage) {}
}
