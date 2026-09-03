pub mod bullet;
pub mod code;
pub mod content;
pub mod diagram;
pub mod gallery;
pub mod image_slide;
pub mod image_split;
pub mod quote;
pub mod section;
pub mod stacked;
pub mod title;
pub mod two_column;
pub mod visualization;

use eframe::egui;

use crate::parser::Layout;

/// Vertical/horizontal padding (at reference resolution) used by text layouts.
pub const SLIDE_PADDING: f32 = 80.0;

/// Padding used by layouts that fill the slide with a large element
/// (image, diagram, visualization).
pub const MEDIA_PADDING: f32 = 60.0;

/// Padding used by the gallery layout.
pub const GALLERY_PADDING: f32 = 50.0;

/// Width of the column a layout stacks its blocks in, for a slide drawn in
/// `rect`. Both the layouts and overflow measurement use this so the two can
/// never disagree about where text wraps.
pub fn content_width(layout: Layout, rect: egui::Rect, scale: f32) -> f32 {
    match layout {
        Layout::Bullet | Layout::Content => rect.width() * 0.70,
        Layout::Code => rect.width() * 0.75,
        Layout::TwoColumn => rect.width() * 0.80,
        Layout::Diagram | Layout::Visualization | Layout::Image => {
            rect.width() - MEDIA_PADDING * 2.0 * scale
        }
        Layout::Gallery => rect.width() - GALLERY_PADDING * 2.0 * scale,
        Layout::Title | Layout::Section | Layout::Quote => {
            rect.width() - SLIDE_PADDING * 2.0 * scale
        }
    }
}

/// Left edge of a horizontally centred column of `width` inside `rect`.
pub fn centered_left(rect: egui::Rect, width: f32) -> f32 {
    rect.left() + (rect.width() - width) / 2.0
}

/// Top of a block of `content_height` vertically centred in a band starting
/// at `top` with `available` height; clamps to `top` when the content overflows
/// so scrolling starts from the first line.
pub fn centered_top(top: f32, available: f32, content_height: f32) -> f32 {
    if content_height < available {
        top + (available - content_height) / 2.0
    } else {
        top
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_width_matches_layout_fractions() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        assert_eq!(content_width(Layout::Bullet, rect, 1.0), 1920.0 * 0.70);
        assert_eq!(content_width(Layout::Content, rect, 1.0), 1920.0 * 0.70);
        assert_eq!(content_width(Layout::Code, rect, 1.0), 1920.0 * 0.75);
        assert_eq!(content_width(Layout::TwoColumn, rect, 1.0), 1920.0 * 0.80);
        assert_eq!(content_width(Layout::Title, rect, 2.0), 1920.0 - 320.0);
        assert_eq!(content_width(Layout::Visualization, rect, 1.0), 1800.0);
    }

    #[test]
    fn centered_top_clamps_on_overflow() {
        assert_eq!(centered_top(80.0, 900.0, 300.0), 380.0);
        assert_eq!(centered_top(80.0, 900.0, 1200.0), 80.0);
    }
}
