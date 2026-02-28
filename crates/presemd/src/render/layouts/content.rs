use eframe::egui::{self, Pos2};

use crate::parser::Slide;
use crate::render::image_cache::ImageCache;
use crate::render::text;
use crate::theme::Theme;

/// Fallback layout: render all blocks top-to-bottom, vertically centered.
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
    let v_padding = 80.0 * scale;
    let content_width = rect.width() * 0.70;
    let content_left = rect.left() + (rect.width() - content_width) / 2.0;

    // Measure content height for vertical centering
    let total_height = text::measure_blocks_height(ui, &slide.blocks, theme, content_width, scale);

    let available_height = rect.height() - v_padding * 2.0;
    let start_y = if total_height < available_height {
        rect.top() + v_padding + (available_height - total_height) / 2.0
    } else {
        rect.top() + v_padding
    };

    text::draw_blocks(
        ui,
        &slide.blocks,
        theme,
        Pos2::new(content_left, start_y),
        content_width,
        opacity,
        image_cache,
        reveal_step,
        scale,
    );
}
