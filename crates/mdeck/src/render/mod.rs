pub mod diagram;
pub mod image_cache;
pub mod layouts;
pub mod syntax;
pub mod text;
pub mod transition;
pub mod visualizations;

use std::time::Instant;

use eframe::egui;

use crate::parser::{Layout, Slide};
use crate::theme::Theme;

use image_cache::ImageCache;

/// Measure the content height of a slide (for scroll/overflow detection),
/// laying blocks out at the same column width the slide's layout draws them.
/// Returns (content_height, available_height) where available_height is the
/// usable area within the slide rect after padding.
pub fn measure_slide_content_height(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: egui::Rect,
    scale: f32,
) -> (f32, f32) {
    let padding = layouts::SLIDE_PADDING * scale;
    let available_height = rect.height() - padding * 2.0;

    let content_height = match slide.layout {
        Layout::Bullet | Layout::Content | Layout::Code => {
            layouts::stacked::measure_content_height(ui, slide, theme, rect, scale)
        }
        Layout::TwoColumn => {
            layouts::two_column::measure_content_height(ui, slide, theme, rect, scale)
        }
        layout => {
            let width = layouts::content_width(layout, rect, scale);
            text::measure_blocks_height(ui, &slide.blocks, theme, width, scale)
        }
    };

    (content_height, available_height)
}

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
    reveal_timestamp: Option<Instant>,
    scale: f32,
) {
    match slide.layout {
        Layout::Title => layouts::title::render(ui, slide, theme, rect, opacity, scale),
        Layout::Section => layouts::section::render(ui, slide, theme, rect, opacity, scale),
        Layout::Quote => {
            layouts::quote::render(ui, slide, theme, rect, opacity, image_cache, scale)
        }
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
        Layout::Gallery => layouts::gallery::render(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            scale,
        ),
        Layout::Diagram => layouts::diagram::render(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            reveal_timestamp,
            scale,
        ),
        Layout::Visualization => layouts::visualization::render(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            reveal_timestamp,
            scale,
        ),
    }
}

/// Helpers for tests that need a live `egui::Ui` to lay out text.
#[cfg(test)]
pub(crate) mod test_support {
    use eframe::egui;

    /// Run `f` with a `Ui` inside a headless egui frame sized like a 1080p slide.
    pub fn with_ui(mut f: impl FnMut(&egui::Ui)) {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1920.0, 1080.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| f(ui));
        });
        // Headless: nobody uploads the font atlas, so discard the deltas explicitly.
        output.textures_delta.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Block, Inline, ListItem, ListMarker};
    use test_support::with_ui;

    fn slide(layout: Layout, blocks: Vec<Block>) -> Slide {
        Slide {
            directives: vec![],
            blocks,
            layout,
            raw_source: String::new(),
            notes: None,
        }
    }

    #[test]
    fn long_bullet_slide_reports_overflow() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));
            let items = (0..10)
                .map(|i| ListItem {
                    marker: ListMarker::Static,
                    inlines: vec![Inline::Text(format!(
                        "Bullet {i} is long enough to wrap onto a second row at seventy percent width of the slide"
                    ))],
                    children: vec![],
                })
                .collect();
            let s = slide(
                Layout::Bullet,
                vec![
                    Block::Heading {
                        level: 1,
                        inlines: vec![Inline::Text("Overflowing".into())],
                    },
                    Block::List {
                        ordered: false,
                        items,
                    },
                ],
            );
            let (content, available) = measure_slide_content_height(ui, &s, &theme, rect, 1.0);
            assert_eq!(available, 1080.0 - 160.0);
            assert!(content > available, "{content} should overflow {available}");
        });
    }

    #[test]
    fn short_slide_does_not_overflow() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));
            let s = slide(
                Layout::Content,
                vec![Block::Paragraph {
                    inlines: vec![Inline::Text("Hello".into())],
                }],
            );
            let (content, available) = measure_slide_content_height(ui, &s, &theme, rect, 1.0);
            assert!(content < available);
        });
    }
}
