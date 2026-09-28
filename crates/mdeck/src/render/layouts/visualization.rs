use std::time::Instant;

use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::image_cache::ImageCache;
use crate::render::text;
use crate::render::visualizations;
use crate::theme::Theme;

fn is_viz_block(block: &Block) -> bool {
    matches!(block, Block::Chart { .. })
}

/// Visualization slide layout: heading at top, optional text blocks, visualization
/// filling remaining space.
#[allow(clippy::too_many_arguments)]
pub fn render(
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
    let padding = 60.0 * scale;
    let content_width = rect.width() - padding * 2.0;
    let content_left = rect.left() + padding;
    let mut y = rect.top() + padding;

    // Separate blocks into: first heading, text blocks, first viz block
    let mut heading: Option<&Block> = None;
    let mut viz_block: Option<&Block> = None;
    let mut text_blocks: Vec<&Block> = Vec::new();

    for block in &slide.blocks {
        match block {
            Block::Heading { .. } if heading.is_none() => {
                heading = Some(block);
            }
            _ if is_viz_block(block) && viz_block.is_none() => {
                viz_block = Some(block);
            }
            _ if !is_viz_block(block) && !matches!(block, Block::Heading { .. }) => {
                text_blocks.push(block);
            }
            _ => {}
        }
    }

    // The field keeps clear of the whole content area below the padding.
    crate::render::hints::push(
        ui.ctx(),
        crate::render::hints::Hint::Frame(rect.shrink(padding * 0.5)),
    );

    // Draw heading if present
    if let Some(Block::Heading { level, inlines }) = heading {
        let h = text::draw_heading(
            ui,
            inlines,
            *level,
            theme,
            Pos2::new(content_left, y),
            content_width,
            opacity,
            scale,
        );
        y += h + text::heading_spacing(theme, *level, scale);
    }

    // Draw any text blocks (paragraphs, lists, etc.) between heading and visualization
    for block in &text_blocks {
        let h = text::draw_block(
            ui,
            block,
            theme,
            Pos2::new(content_left, y),
            content_width,
            opacity,
            image_cache,
            reveal_step,
            scale,
        );
        y += h + text::block_spacing(block, theme, scale);
    }

    // Draw visualization filling the remaining vertical space
    if let Some(block) = viz_block {
        let remaining_height = rect.bottom() - y - padding;
        if remaining_height > 50.0 * scale {
            let viz_pos = Pos2::new(content_left, y);
            if let Block::Chart { kind, content } = block {
                let cx = visualizations::VizCtx {
                    ui,
                    theme,
                    opacity,
                    scale,
                    reveal_step,
                    reveal_timestamp,
                };
                visualizations::draw(
                    *kind,
                    content,
                    &cx,
                    viz_pos,
                    content_width,
                    remaining_height,
                );
            }
        }
    }
}
