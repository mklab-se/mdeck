pub mod art;
pub mod background;
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
pub mod syntax;
pub mod text;
pub mod thermal;
pub mod transition;
pub mod visualizations;

use eframe::egui;

use crate::parser::{Layout, Slide};
use crate::theme::Theme;

pub use context::{BlockCx, SlideContext, TextCx};

/// Whether `slide`'s design has a stage a picture stands on (ENG-14).
/// Interim: the slides the editorial arrangement draws; phase 3 asks the
/// slide's arrangement.
pub fn design_has_stage(slide: &Slide, theme: &Theme) -> bool {
    crate::theme::uses_editorial(theme) && ember::handles(slide)
}

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

    if crate::theme::uses_editorial(theme) && ember::handles(slide) {
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
pub fn render_slide(cx: &BlockCx, slide: &Slide, rect: egui::Rect, slide_cx: &SlideContext) {
    let theme = cx.theme;
    if let Some(board) = theme.engine.board() {
        board::render(board, cx, slide, rect, slide_cx);
        return;
    }
    if crate::theme::uses_editorial(theme) && ember::handles(slide) {
        ember::render(cx, slide, rect, slide_cx);
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
    render(cx, slide, rect);
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
            blocks,
            layout,
            raw_source: String::new(),
            notes: None,
            ..Default::default()
        }
    }

    #[test]
    fn long_bullet_slide_reports_overflow() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));
            let items = (0..10)
                .map(|i| ListItem::new(
                    ListMarker::Static,
                    vec![Inline::Text(format!(
                        "Bullet {i} is long enough to wrap onto a second row at seventy percent width of the slide"
                    ))],
                    vec![],
                ))
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
                        start: 1,
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

    /// The renderer's boundary: drawing code never reaches the app, the
    /// commands, the CLI, the deck or the configuration. What it needs is
    /// handed in (see `diagram::set_routing_weights`).
    #[test]
    fn the_renderer_stays_inside_its_boundary() {
        const FORBIDDEN: &[&str] = &[
            "crate::app",
            "crate::commands",
            "crate::cli",
            "crate::config",
            "crate::deck",
        ];
        fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("render dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(path);
                }
            }
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/render");
        let mut all = Vec::new();
        files(&dir, &mut all);
        let bad: Vec<String> = all
            .iter()
            .flat_map(|path| {
                let src = std::fs::read_to_string(path).expect("read");
                src.lines()
                    .enumerate()
                    .filter(|(_, line)| {
                        // a quoted path (this list) is not a use of it
                        FORBIDDEN.iter().any(|f| {
                            line.match_indices(f)
                                .any(|(at, _)| !line[..at].ends_with('"'))
                        })
                    })
                    .map(|(n, line)| format!("{}:{}: {}", path.display(), n + 1, line.trim()))
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(
            bad.is_empty(),
            "the renderer reaches outside:\n{}",
            bad.join("\n")
        );
    }
}
