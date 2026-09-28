pub mod art;
pub mod context;
pub mod diagram;
pub mod ember;
pub mod fonts;
pub mod hints;
pub mod illustration;
pub mod image_cache;
pub mod layouts;
pub mod logo;
pub mod math;
pub mod page;
pub mod particles;
pub mod story;
pub mod strokes;
pub mod syntax;
pub mod text;
pub mod transition;
pub mod visualizations;

use std::time::Instant;

use eframe::egui;

use crate::parser::{Layout, Slide};
use crate::theme::Theme;

pub use context::{BlockCx, SlideContext, TextCx};
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
    // a board never scrolls: what does not fit is cut (and reported)
    if theme.engine.is_board() {
        return (0.0, available_height);
    }

    if theme.engine.lays_out(slide) {
        let h = ember::measure_content_height(ui, slide, theme, rect, scale);
        return (h, rect.height() * 0.80);
    }

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
    cx: &SlideContext,
) {
    if theme.engine.is_board() {
        crate::engines::splitflap::render_slide(
            ui,
            slide,
            theme,
            rect,
            opacity,
            image_cache,
            reveal_step,
            scale,
            cx,
        );
        return;
    }
    let block_cx = BlockCx {
        ui,
        theme,
        opacity,
        scale,
        image_cache,
        reveal_step,
        reveal_timestamp,
    };
    if theme.engine.lays_out(slide) {
        ember::render(&block_cx, slide, rect, cx);
        return;
    }
    let render = match slide.layout {
        Layout::Title => layouts::title::render,
        Layout::Section => layouts::section::render,
        Layout::Quote => layouts::quote::render,
        Layout::Bullet => layouts::bullet::render,
        Layout::Code => layouts::code::render,
        Layout::TwoColumn => layouts::two_column::render,
        Layout::Content => layouts::content::render,
        Layout::Image => layouts::image_slide::render,
        Layout::Gallery => layouts::gallery::render,
        Layout::Diagram => layouts::diagram::render,
        Layout::Visualization => layouts::visualization::render,
    };
    render(&block_cx, slide, rect);
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
            story_hint: None,
            scene_script: None,
            illustration: None,
            logo: None,
            art: None,
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
