//! Shared renderer for layouts that stack blocks vertically in a centred column
//! (bullet, code, content). If the slide contains an image, the blocks go in a
//! left column and the image in a right column.

use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::image_cache::ImageCache;
use crate::render::layouts::{
    SLIDE_PADDING, centered_left, centered_top, content_width, image_split,
};
use crate::render::text;
use crate::theme::Theme;

/// Column geometry for a stacked slide: where the text column starts and how
/// wide it is, plus the vertical band the content is centred in.
struct Column {
    left: f32,
    width: f32,
    top: f32,
    available: f32,
}

fn text_column(slide: &Slide, rect: egui::Rect, scale: f32) -> Column {
    let padding = SLIDE_PADDING * scale;
    if image_split::has_image(&slide.blocks) {
        let (left, _) = image_split::image_split_rects(rect.shrink(padding));
        Column {
            left: left.left(),
            width: left.width(),
            top: left.top(),
            available: left.height(),
        }
    } else {
        let width = content_width(slide.layout, rect, scale);
        Column {
            left: centered_left(rect, width),
            width,
            top: rect.top() + padding,
            available: rect.height() - padding * 2.0,
        }
    }
}

/// Blocks that go in the text column (everything except the side image).
fn text_blocks(slide: &Slide) -> Vec<&Block> {
    image_split::split_image(&slide.blocks).0
}

/// Height of the text column content, laid out exactly as [`render`] draws it.
pub fn measure_content_height(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: egui::Rect,
    scale: f32,
) -> f32 {
    let column = text_column(slide, rect, scale);
    text::measure_blocks_height(ui, text_blocks(slide), theme, column.width, scale)
}

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
    let column = text_column(slide, rect, scale);
    let blocks = text_blocks(slide);

    let total_height =
        text::measure_blocks_height(ui, blocks.iter().copied(), theme, column.width, scale);
    let start_y = centered_top(column.top, column.available, total_height);

    text::draw_blocks(
        ui,
        blocks.iter().copied(),
        theme,
        Pos2::new(column.left, start_y),
        column.width,
        opacity,
        image_cache,
        reveal_step,
        scale,
    );

    // Side image, vertically centred in the right column
    if let (
        _,
        Some(Block::Image {
            alt,
            path,
            directives,
        }),
    ) = image_split::split_image(&slide.blocks)
    {
        let padding = SLIDE_PADDING * scale;
        let (_, right_rect) = image_split::image_split_rects(rect.shrink(padding));
        text::draw_image_in_area(
            ui,
            path,
            alt,
            directives,
            theme,
            right_rect,
            opacity,
            image_cache,
            scale,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ImageDirectives, Inline, Layout, ListItem, ListMarker};
    use crate::render::test_support::with_ui;

    fn slide(layout: Layout, blocks: Vec<Block>) -> Slide {
        Slide {
            directives: vec![],
            blocks,
            layout,
            raw_source: String::new(),
            notes: None,
        }
    }

    fn long_list() -> Block {
        Block::List {
            ordered: false,
            items: (0..12)
                .map(|i| ListItem {
                    marker: ListMarker::Static,
                    // Varying lengths so the row count differs between column widths
                    inlines: vec![Inline::Text(format!(
                        "Item {i}: {}",
                        "lorem ipsum ".repeat(6 + i)
                    ))],
                    children: vec![],
                })
                .collect(),
        }
    }

    #[test]
    fn measurement_uses_the_same_column_width_as_drawing() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
            let s = slide(Layout::Bullet, vec![long_list()]);

            let measured = measure_content_height(ui, &s, &theme, rect, 1.0);
            let column = text_column(&s, rect, 1.0);
            assert_eq!(column.width, 1920.0 * 0.70);
            let at_column = text::measure_blocks_height(ui, &s.blocks, &theme, column.width, 1.0);
            assert_eq!(measured, at_column);

            // Measuring at the old (wider) width under-reports the height.
            let at_full = text::measure_blocks_height(ui, &s.blocks, &theme, 1920.0 - 160.0, 1.0);
            assert!(measured > at_full, "{measured} > {at_full}");
            assert!(measured > 1080.0 - 160.0, "this slide overflows");
        });
    }

    #[test]
    fn image_slides_measure_the_narrow_text_column() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
            let s = slide(
                Layout::Content,
                vec![
                    long_list(),
                    Block::Image {
                        alt: String::new(),
                        path: "missing.png".into(),
                        directives: ImageDirectives::default(),
                    },
                ],
            );
            let column = text_column(&s, rect, 1.0);
            assert!((column.width - (1920.0 - 160.0) * 0.55).abs() < 0.01);
            let measured = measure_content_height(ui, &s, &theme, rect, 1.0);
            let list_only =
                text::measure_blocks_height(ui, &s.blocks[..1], &theme, column.width, 1.0);
            assert_eq!(measured, list_only);
        });
    }
}
