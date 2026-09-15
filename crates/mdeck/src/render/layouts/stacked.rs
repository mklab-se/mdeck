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

/// Code never shrinks below this fraction of the theme's code size; past it
/// the slide scrolls as before.
pub const CODE_FIT_FLOOR: f32 = 0.4;

/// The theme to lay the slide out with: the same theme, or a copy whose code
/// size is reduced so the slide's code blocks fit the column, in height and
/// in line width, down to [`CODE_FIT_FLOOR`]. Long code then shows whole on a
/// still export instead of being cut off (GitHub issue 8), and long lines
/// stop wrapping (backlog 1.9). Prose is never shrunk.
fn fit_code(ui: &egui::Ui, blocks: &[&Block], theme: &Theme, column: &Column, scale: f32) -> Theme {
    let code: Vec<(&str, Option<&str>)> = blocks
        .iter()
        .filter_map(|b| match b {
            Block::CodeBlock { code, language, .. } => Some((code.as_str(), language.as_deref())),
            _ => None,
        })
        .collect();
    if code.is_empty() {
        return theme.clone();
    }
    let measure =
        |t: &Theme| text::measure_blocks_height(ui, blocks.iter().copied(), t, column.width, scale);
    let total = measure(theme);
    // the block padding does not scale with the font, so fit the text inside it
    let padding = 2.0 * text::CODE_PADDING * scale * code.len() as f32;
    let code_text: f32 = code
        .iter()
        .map(|(c, l)| text::measure_code_block_height(ui, c, *l, theme, column.width, scale))
        .sum::<f32>()
        - padding;
    let budget = column.available - (total - code_text - padding) - padding;
    let by_height = if code_text > 0.0 {
        budget / code_text
    } else {
        1.0
    };
    let widest = code
        .iter()
        .map(|(c, _)| text::widest_code_line(ui, c, theme, scale))
        .fold(0.0, f32::max);
    let inner = column.width - 2.0 * text::CODE_PADDING * scale;
    let by_width = if widest > 0.0 { inner / widest } else { 1.0 };
    let mut factor = by_height.min(by_width);
    if factor >= 1.0 {
        return theme.clone();
    }
    let mut fitted = theme.clone();
    // row heights round, so re-measure and nudge down until it truly fits
    for _ in 0..3 {
        factor = factor.max(CODE_FIT_FLOOR);
        fitted.code_size = theme.code_size * factor;
        let h = measure(&fitted);
        if h <= column.available || factor <= CODE_FIT_FLOOR {
            break;
        }
        factor *= (column.available / h) * 0.995;
    }
    fitted
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
    let blocks = text_blocks(slide);
    let theme = fit_code(ui, &blocks, theme, &column, scale);
    text::measure_blocks_height(ui, blocks, &theme, column.width, scale)
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
    let fitted = fit_code(ui, &blocks, theme, &column, scale);
    let theme = &fitted;

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
            story_hint: None,
            scene_script: None,
            illustration: None,
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

    fn code_slide(lines: usize, line: &str) -> Slide {
        let body: Vec<String> = (0..lines)
            .map(|i| line.replace("{i}", &i.to_string()))
            .collect();
        slide(
            Layout::Code,
            vec![
                Block::Heading {
                    level: 1,
                    inlines: vec![crate::parser::Inline::Text("Code".into())],
                },
                Block::CodeBlock {
                    language: Some("rust".into()),
                    code: body.join("\n"),
                    highlight_lines: vec![],
                },
            ],
        )
    }

    /// Regression for GitHub issue 8: a code block taller than the slide is
    /// shrunk until it fits (down to the floor) instead of being cut off.
    #[test]
    fn long_code_shrinks_to_fit_the_slide() {
        let ctx = egui::Context::default();
        crate::render::fonts::install(&ctx);
        let theme = Theme::light();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let short = code_slide(8, "let v{i} = {i};");
            let column = text_column(&short, rect, 1.0);
            let fitted = fit_code(ui, &text_blocks(&short), &theme, &column, 1.0);
            assert_eq!(
                fitted.code_size, theme.code_size,
                "short code must not shrink"
            );

            let long = code_slide(32, "let value_{i} = compute({i}); // line {i}");
            let fitted = fit_code(ui, &text_blocks(&long), &theme, &column, 1.0);
            assert!(
                fitted.code_size < theme.code_size,
                "long code did not shrink"
            );
            assert!(fitted.code_size >= theme.code_size * CODE_FIT_FLOOR);
            let height = measure_content_height(ui, &long, &theme, rect, 1.0);
            assert!(
                height <= column.available + 1.0,
                "fitted code still overflows: {height} > {}",
                column.available
            );

            // past the floor the slide scrolls, as before
            let huge = code_slide(200, "let value_{i} = compute({i});");
            let fitted = fit_code(ui, &text_blocks(&huge), &theme, &column, 1.0);
            assert_eq!(fitted.code_size, theme.code_size * CODE_FIT_FLOOR);
            assert!(measure_content_height(ui, &huge, &theme, rect, 1.0) > column.available);
        });
        output.textures_delta.clear();
    }

    #[test]
    fn long_lines_shrink_instead_of_wrapping() {
        let ctx = egui::Context::default();
        crate::render::fonts::install(&ctx);
        let theme = Theme::light();
        let rect = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let wide = code_slide(3, &format!("let x{{i}} = \"{}\";", "x".repeat(110)));
            let column = text_column(&wide, rect, 1.0);
            let fitted = fit_code(ui, &text_blocks(&wide), &theme, &column, 1.0);
            assert!(
                fitted.code_size < theme.code_size,
                "long lines did not shrink"
            );
            let widest = text::widest_code_line(
                ui,
                &match &wide.blocks[1] {
                    Block::CodeBlock { code, .. } => code.clone(),
                    _ => unreachable!(),
                },
                &fitted,
                1.0,
            );
            assert!(widest <= column.width - 2.0 * text::CODE_PADDING + 1.0);
        });
        output.textures_delta.clear();
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
