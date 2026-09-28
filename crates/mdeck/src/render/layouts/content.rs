use eframe::egui;

use crate::parser::Slide;
use crate::render::BlockCx;
use crate::render::layouts::stacked;

/// Fallback layout: render all blocks top-to-bottom, vertically centred in a
/// 70%-wide column. If the slide contains one image, split into content (left)
/// + image (right).
pub fn render(cx: &BlockCx, slide: &Slide, rect: egui::Rect) {
    stacked::render(cx, slide, rect);
}
