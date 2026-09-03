use eframe::egui;

use crate::parser::Slide;
use crate::render::image_cache::ImageCache;
use crate::render::layouts::stacked;
use crate::theme::Theme;

/// Bullet layout: heading plus lists, stacked in a 70%-wide centred column
/// (or beside an image). Shares its implementation with the content layout;
/// the column width comes from [`crate::render::layouts::content_width`].
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
