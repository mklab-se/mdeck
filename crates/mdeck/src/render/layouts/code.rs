use eframe::egui;

use crate::parser::Slide;
use crate::render::BlockCx;
use crate::render::layouts::stacked;

/// Code layout: heading plus code blocks, stacked in a 75%-wide centred column
/// (or beside an image). The column width comes from
/// [`crate::render::layouts::content_width`].
pub fn render(cx: &BlockCx, slide: &Slide, rect: egui::Rect) {
    stacked::render(cx, slide, rect);
}
