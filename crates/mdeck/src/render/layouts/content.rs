use eframe::egui;

use crate::parser::Slide;
use crate::render::image_cache::ImageCache;
use crate::render::layouts::stacked;
use crate::theme::Theme;

/// Fallback layout: render all blocks top-to-bottom, vertically centred in a
/// 70%-wide column. If the slide contains one image, split into content (left)
/// + image (right).
#[allow(clippy::too_many_arguments)]
pub fn render(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: egui::Rect,
    opacity: f32,
    image_cache: &ImageCache,
    reveal_step: usize,
    scale: f32,
) {
    stacked::render(
        ui,
        slide,
        theme,
        rect,
        opacity,
        image_cache,
        reveal_step,
        scale,
    );
}
