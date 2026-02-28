pub mod image_cache;
pub mod layouts;
pub mod text;
pub mod transition;

use eframe::egui;

use crate::parser::{Layout, Slide};
use crate::theme::Theme;

use image_cache::ImageCache;

/// Render a single slide using its inferred layout.
#[allow(clippy::too_many_arguments)]
pub fn render_slide(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: egui::Rect,
    opacity: f32,
    image_cache: &ImageCache,
    reveal_step: usize,
    scale: f32,
) {
    match slide.layout {
        Layout::Title => layouts::title::render(ui, slide, theme, rect, opacity, scale),
        Layout::Section => layouts::section::render(ui, slide, theme, rect, opacity, scale),
        Layout::Quote => layouts::quote::render(ui, slide, theme, rect, opacity, scale),
        Layout::Bullet => layouts::bullet::render(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            scale,
        ),
        Layout::Code => layouts::code::render(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            scale,
        ),
        Layout::TwoColumn => layouts::two_column::render(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            scale,
        ),
        Layout::Content => layouts::content::render(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            scale,
        ),
        Layout::Image => layouts::image_slide::render(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            scale,
        ),
        Layout::Gallery => layouts::content::render(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            scale,
        ),
        Layout::Diagram => layouts::content::render(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            scale,
        ),
    }
}
