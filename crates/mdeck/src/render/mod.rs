pub mod art;
pub mod background;
pub mod board;
pub mod context;
pub mod designs;
pub mod diagram;
pub mod ember;
pub mod fonts;
pub mod hints;
pub mod illustration;
pub mod image_cache;
pub mod logo;
pub mod math;
pub mod page;
pub mod syntax;
pub mod text;
pub mod thermal;
pub mod transition;
pub mod visualizations;

use eframe::egui;

use crate::parser::Slide;
use crate::theme::Theme;

pub use context::{BlockCx, SlideContext, TextCx};

/// Whether the slide's design leaves room for the engine's picture of the
/// slide in `theme` (the arrangement's `stage`). A `content` slide that
/// holds a wide block (an image, code, a table or a visual) gives its stage
/// up to the copy. Seam for the engines: where pictures go is the design's
/// business, never the engine's.
pub fn design_has_stage(slide: &Slide, theme: &Theme) -> bool {
    design_stage(slide, theme) != crate::theme::arrangement::Stage::None
}

/// Where the slide's design leaves room for the picture (see
/// [`design_has_stage`]).
pub fn design_stage(slide: &Slide, theme: &Theme) -> crate::theme::arrangement::Stage {
    let a = theme.arrangement(slide.design);
    if a.wide.is_some() && slide.blocks.iter().any(is_wide_block) {
        return crate::theme::arrangement::Stage::None;
    }
    a.stage
}

/// A block that needs more width than a copy column gives it.
pub fn is_wide_block(block: &crate::parser::Block) -> bool {
    use crate::parser::Block;
    matches!(
        block,
        Block::Image { .. }
            | Block::CodeBlock { .. }
            | Block::Table { .. }
            | Block::Chart { .. }
            | Block::Diagram { .. }
    )
}

/// Measure the content height of a slide (for scroll/overflow detection),
/// with the same layout drawing uses. Returns `(content_height,
/// available_height)`; the slide scrolls when the first is larger.
pub fn measure_slide_content_height(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: egui::Rect,
    scale: f32,
    deck: &SlideContext,
) -> (f32, f32) {
    // a board never scrolls: what does not fit is cut (and reported)
    if theme.engine.is_board() {
        return (0.0, rect.height());
    }
    designs::measure(ui, slide, theme, rect, scale, deck)
}

/// Render a single slide in its design, as the theme arranges it.
pub fn render_slide(cx: &BlockCx, slide: &Slide, rect: egui::Rect, slide_cx: &SlideContext) {
    if let Some(board) = cx.theme.engine.board() {
        board::render(board, cx, slide, rect, slide_cx);
        return;
    }
    designs::render(cx, slide, rect, slide_cx);
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

    fn slide(design: crate::parser::Design, blocks: Vec<Block>) -> Slide {
        Slide {
            blocks,
            design,
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
                crate::parser::Design::Points,
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
            let (content, available) =
                measure_slide_content_height(ui, &s, &theme, rect, 1.0, &SlideContext::default());
            assert!((available - 1080.0 * 0.852).abs() < 1.0, "{available}");
            assert!(content > available, "{content} should overflow {available}");
        });
    }

    #[test]
    fn short_slide_does_not_overflow() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1920.0, 1080.0));
            let s = slide(
                crate::parser::Design::Content,
                vec![Block::Paragraph {
                    inlines: vec![Inline::Text("Hello".into())],
                }],
            );
            let (content, available) =
                measure_slide_content_height(ui, &s, &theme, rect, 1.0, &SlideContext::default());
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
