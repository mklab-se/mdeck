use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::BlockCx;
use crate::render::diagram;
use crate::render::text;

/// Diagram slide layout: heading at top, diagram filling remaining space.
pub fn render(cx: &BlockCx, slide: &Slide, rect: egui::Rect) {
    let (ui, theme, scale) = (cx.ui, cx.theme, cx.scale);
    let padding = 60.0 * scale;
    crate::render::hints::push(
        ui.ctx(),
        crate::render::hints::Hint::Frame(rect.shrink(padding * 0.5)),
    );
    let content_width = rect.width() - padding * 2.0;
    let content_left = rect.left() + padding;
    let mut y = rect.top() + padding;

    // Find heading and diagram blocks
    let mut heading: Option<&Block> = None;
    let mut diagram_content: Option<&str> = None;

    for block in &slide.blocks {
        match block {
            Block::Heading { .. } if heading.is_none() => {
                heading = Some(block);
            }
            Block::Diagram { content } if diagram_content.is_none() => {
                diagram_content = Some(content);
            }
            _ => {}
        }
    }

    // Draw heading if present
    if let Some(Block::Heading { level, inlines }) = heading {
        let pos = Pos2::new(content_left, y);
        let h = text::draw_heading(&cx.text(), inlines, *level, pos, content_width);
        y += h + text::heading_spacing(theme, *level, scale);
    }

    // Draw diagram filling the remaining vertical space
    if let Some(content) = diagram_content {
        let remaining_height = rect.bottom() - y - padding;
        if remaining_height > 50.0 * scale {
            diagram::draw_diagram_sized(
                ui,
                content,
                theme,
                Pos2::new(content_left, y),
                content_width,
                remaining_height,
                cx.opacity,
                cx.image_cache,
                cx.reveal_step,
                cx.reveal_timestamp,
                scale,
            );
        }
    }
}
