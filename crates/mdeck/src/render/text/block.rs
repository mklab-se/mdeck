//! Block dispatch: one block, or a sequence stacked with the shared spacing
//! rule, drawn or measured the same way.

use eframe::egui::{self, Pos2, Stroke};

use super::code::{draw_code_block, measure_code_block_height};
use super::image::{IMAGE_MAX_HEIGHT, draw_image};
use super::inline::{draw_heading, draw_paragraph, heading_job, measure_inlines};
use super::list::{Reveal, draw_list, measure_list_height};
use super::quote::{draw_callout, draw_quote, measure_callout, measure_quote};
use super::table::{draw_table, measure_table_height};
use crate::parser::Block;
use crate::render::BlockCx;
use crate::render::diagram::draw_diagram_sized;
use crate::render::visualizations::{self, VizCtx};
use crate::theme::Theme;

// ---------------------------------------------------------------------------
// Spacing rules shared by layouts and measurement
// ---------------------------------------------------------------------------

/// Vertical gap between a heading and the block that follows it.
/// One rule for every layout: half the heading's font size.
pub fn heading_spacing(theme: &Theme, level: u8, scale: f32) -> f32 {
    theme.heading_size(level) * 0.5 * scale
}

/// Vertical gap that follows `block` when blocks are stacked vertically.
/// Used by both drawing and measurement so overflow detection stays exact.
pub fn block_spacing(block: &Block, theme: &Theme, scale: f32) -> f32 {
    match block {
        Block::Heading { level, .. } => heading_spacing(theme, *level, scale),
        Block::HorizontalRule => 10.0 * scale,
        _ => 20.0 * scale,
    }
}

// ---------------------------------------------------------------------------
// Block sequences
// ---------------------------------------------------------------------------

/// Draw blocks sequentially with the shared spacing rule. Returns the total
/// height used (spacing is added between blocks only, matching
/// [`measure_blocks_height`]).
pub fn draw_blocks<'a>(
    cx: &BlockCx,
    blocks: impl IntoIterator<Item = &'a Block>,
    pos: Pos2,
    max_width: f32,
) -> f32 {
    let mut y_offset = 0.0;
    let mut pending_spacing = 0.0;

    for block in blocks {
        y_offset += pending_spacing;
        let block_pos = Pos2::new(pos.x, pos.y + y_offset);
        y_offset += draw_block(cx, block, block_pos, max_width);
        pending_spacing = block_spacing(block, cx.theme, cx.scale);
    }

    y_offset
}

/// Measure total height of a block sequence without drawing.
pub fn measure_blocks_height<'a>(
    ui: &egui::Ui,
    blocks: impl IntoIterator<Item = &'a Block>,
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    let mut total = 0.0;
    let mut pending_spacing = 0.0;
    for block in blocks {
        total += pending_spacing;
        total += measure_single_block_height(ui, block, theme, max_width, scale);
        pending_spacing = block_spacing(block, theme, scale);
    }
    total
}

/// Measure the height of a single block without drawing. Text-like blocks are
/// laid out exactly as their drawing counterparts; visualizations are sized by
/// the layout that hosts them and get a nominal height here.
pub fn measure_single_block_height(
    ui: &egui::Ui,
    block: &Block,
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    match block {
        Block::Heading { level, inlines } => ui
            .painter()
            .layout_job(heading_job(
                inlines,
                *level,
                theme,
                theme.heading_color,
                max_width,
                scale,
            ))
            .rect
            .height(),
        Block::Paragraph { inlines } => {
            measure_inlines(ui, inlines, theme.body_size * scale, max_width, theme)
        }
        Block::BlockQuote { blocks } => measure_quote(ui, blocks, theme, max_width, scale),
        Block::Callout { kind, blocks } => {
            measure_callout(ui, *kind, blocks, theme, max_width, scale)
        }
        Block::List { items, .. } => measure_list_height(ui, items, theme, max_width, 0, scale),
        Block::CodeBlock { code, language, .. } => {
            measure_code_block_height(ui, code, language.as_deref(), theme, max_width, scale)
        }
        Block::Table { headers, rows, .. } => {
            measure_table_height(ui, headers, rows, theme, max_width, scale)
        }
        Block::HorizontalRule => 20.0 * scale,
        Block::Diagram { .. } | Block::Chart { .. } => 500.0 * scale, // visualizations fill available space
        Block::Image { .. } => IMAGE_MAX_HEIGHT * scale,
        Block::ColumnSeparator => 0.0,
    }
}

/// Draw a single block. Returns height used.
///
/// Charts and diagrams in a block flow draw their latest reveal step settled
/// (no `reveal_timestamp`); the layouts that give them the slide animate it.
pub fn draw_block(cx: &BlockCx, block: &Block, pos: Pos2, max_width: f32) -> f32 {
    let text = cx.text();
    match block {
        Block::Heading { level, inlines } => draw_heading(&text, inlines, *level, pos, max_width),
        Block::Paragraph { inlines } => draw_paragraph(&text, inlines, pos, max_width),
        Block::List {
            ordered,
            start,
            items,
        } => draw_list(
            &text,
            items,
            *ordered,
            *start,
            pos,
            max_width,
            Reveal {
                shown: cx.reveal_step,
                at: cx.reveal_timestamp,
            },
        ),
        Block::CodeBlock {
            language,
            code,
            highlight_lines,
        } => draw_code_block(
            &text,
            code,
            language.as_deref(),
            highlight_lines,
            pos,
            max_width,
        ),
        Block::BlockQuote { blocks } => draw_quote(cx, blocks, pos, max_width),
        Block::Callout { kind, blocks } => draw_callout(cx, *kind, blocks, pos, max_width),
        Block::Table {
            headers,
            align,
            rows,
        } => draw_table(&text, headers, align, rows, pos, max_width),
        Block::Image {
            alt,
            path,
            directives,
        } => draw_image(cx, path, alt, directives, pos, max_width),
        Block::Diagram { content, step_base } => {
            let cx = BlockCx {
                reveal_timestamp: None,
                ..cx.after_steps(*step_base)
            };
            draw_diagram_sized(&cx, content, pos, max_width, 0.0)
        }
        Block::Chart {
            kind: crate::parser::Chart::Thermal,
            content,
            step_base,
        } => {
            crate::render::thermal::draw(&cx.after_steps(*step_base), content, pos, max_width, 0.0)
        }
        Block::Chart { kind, content, .. } if kind.is_external() => {
            draw_external(cx, kind.tag(), content, pos, max_width)
        }
        Block::Chart {
            kind,
            content,
            step_base,
        } => {
            let viz = VizCtx {
                reveal_timestamp: None,
                ..cx.after_steps(*step_base).viz()
            };
            visualizations::draw(*kind, content, &viz, pos, max_width, 0.0)
        }
        Block::HorizontalRule => {
            let color = Theme::with_opacity(cx.theme.accent, cx.opacity * 0.5);
            let y = pos.y + 10.0 * cx.scale;
            cx.ui.painter().line_segment(
                [Pos2::new(pos.x, y), Pos2::new(pos.x + max_width, y)],
                Stroke::new(1.0 * cx.scale, color),
            );
            20.0 * cx.scale
        }
        Block::ColumnSeparator => 0.0, // handled by two-column layout
    }
}

/// An external visual program's fence (EXT-18): the image it made, in a
/// box with the program's aspect, or the fence's source as code when there
/// is none (the EXT-07 fallback). Returns the height used.
fn draw_external(cx: &BlockCx, tag: &str, content: &str, pos: Pos2, max_width: f32) -> f32 {
    let Some(image) = cx.image_cache.external(tag, content) else {
        let code = Block::CodeBlock {
            language: None,
            code: content.trim_end().to_string(),
            highlight_lines: Vec::new(),
        };
        return draw_block(cx, &code, pos, max_width);
    };
    let (w, h) = crate::extensions::external::BOX;
    let height = max_width * h as f32 / w as f32;
    let rect = egui::Rect::from_min_size(pos, egui::vec2(max_width, height));
    crate::extensions::external::draw(cx.ui, cx.image_cache, image, rect, cx.opacity);
    height
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Inline, ListItem, ListMarker};
    use crate::render::image_cache::ImageCache;
    use crate::render::test_support::with_ui;

    fn text(s: &str) -> Vec<Inline> {
        vec![Inline::Text(s.to_string())]
    }

    fn item(s: &str, children: Vec<ListItem>) -> ListItem {
        ListItem::new(ListMarker::Static, text(s), children)
    }

    fn block_cx<'a>(ui: &'a egui::Ui, theme: &'a Theme, cache: &'a ImageCache) -> BlockCx<'a> {
        BlockCx {
            ui,
            theme,
            opacity: 1.0,
            scale: 1.0,
            image_cache: cache,
            reveal_step: usize::MAX,
            reveal_timestamp: None,
        }
    }

    const LONG: &str = "This is a deliberately long list item whose text is guaranteed to wrap \
        onto several rows when laid out inside a narrow column, so that the measured height \
        depends on real galley layout rather than a per-item constant.";

    #[test]
    fn wrapped_list_measures_exactly_as_drawn() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let cache = ImageCache::new(std::path::PathBuf::new());
            let items = vec![
                item(LONG, vec![item("child", vec![])]),
                item("short", vec![]),
            ];
            let block = Block::List {
                ordered: false,
                start: 1,
                items,
            };
            let width = 700.0;

            let measured = measure_single_block_height(ui, &block, &theme, width, 1.0);
            let drawn = draw_block(&block_cx(ui, &theme, &cache), &block, Pos2::ZERO, width);
            assert!((measured - drawn).abs() < 0.01, "{measured} vs {drawn}");

            // The old estimate was `count * (font_size + 8)`; a wrapped item must exceed it.
            let naive = 3.0 * (theme.body_size + 8.0);
            assert!(measured > naive * 1.5, "{measured} should reflect wrapping");
        });
    }

    #[test]
    fn block_sequence_measures_exactly_as_drawn() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let cache = ImageCache::new(std::path::PathBuf::new());
            let blocks = vec![
                Block::Heading {
                    level: 1,
                    inlines: text("A heading that is long enough to wrap in a narrow column"),
                },
                Block::Paragraph {
                    inlines: text(LONG),
                },
                Block::List {
                    ordered: true,
                    start: 1,
                    items: vec![item(LONG, vec![]), item("two", vec![])],
                },
                Block::CodeBlock {
                    language: Some("rust".into()),
                    code: "fn main() {\n    println!(\"hi\");\n}".into(),
                    highlight_lines: vec![],
                },
                Block::Table {
                    headers: vec![text("Name"), text("Value")],
                    align: vec![],
                    rows: vec![vec![text("a"), text(LONG)], vec![text("b"), text("2")]],
                },
                Block::BlockQuote {
                    blocks: vec![
                        Block::Paragraph {
                            inlines: text(LONG),
                        },
                        Block::Paragraph {
                            inlines: text("Someone"),
                        },
                    ],
                },
                Block::Callout {
                    kind: crate::parser::Alert::Tip,
                    blocks: vec![Block::Paragraph {
                        inlines: text(LONG),
                    }],
                },
                Block::HorizontalRule,
            ];
            let width = 640.0;
            let measured = measure_blocks_height(ui, &blocks, &theme, width, 1.0);
            let drawn = draw_blocks(&block_cx(ui, &theme, &cache), &blocks, Pos2::ZERO, width);
            assert!((measured - drawn).abs() < 0.01, "{measured} vs {drawn}");
        });
    }

    #[test]
    fn hidden_items_keep_their_space() {
        // MD-19, D11: a list measures and draws the same height whatever
        // its reveal, so nothing below moves when items appear.
        with_ui(|ui| {
            let theme = Theme::dark();
            let cache = ImageCache::new(std::path::PathBuf::new());
            let mut blocks = crate::parser::blocks::parse("+ one\n+ two\n  - child\n+ three");
            crate::parser::steps::number(
                &mut blocks,
                true,
                &crate::parser::steps::default_visual_steps,
            );
            let full = measure_single_block_height(ui, &blocks[0], &theme, 600.0, 1.0);
            for step in 0..=3 {
                let cx = BlockCx {
                    reveal_step: step,
                    ..block_cx(ui, &theme, &cache)
                };
                let drawn = draw_block(&cx, &blocks[0], Pos2::ZERO, 600.0);
                assert!(
                    (drawn - full).abs() < 0.01,
                    "step {step}: {drawn} vs {full}"
                );
            }
        });
    }

    #[test]
    fn heading_spacing_is_half_the_heading_size() {
        let theme = Theme::dark();
        assert_eq!(heading_spacing(&theme, 1, 1.0), theme.h1_size * 0.5);
        assert_eq!(heading_spacing(&theme, 2, 2.0), theme.h2_size);
        let h = Block::Heading {
            level: 3,
            inlines: vec![],
        };
        assert_eq!(block_spacing(&h, &theme, 1.0), theme.h3_size * 0.5);
    }
}
