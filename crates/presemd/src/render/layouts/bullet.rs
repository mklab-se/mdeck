use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::image_cache::ImageCache;
use crate::render::text;
use crate::theme::Theme;

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
    let v_padding = 80.0 * scale;
    // Center a 70% width content area on the slide
    let content_width = rect.width() * 0.70;
    let content_left = rect.left() + (rect.width() - content_width) / 2.0;

    // First pass: measure total content height
    let mut total_height = 0.0;
    for (i, block) in slide.blocks.iter().enumerate() {
        let h = measure_block_height(ui, block, theme, content_width, scale);
        total_height += h;
        if i < slide.blocks.len() - 1 {
            total_height += block_spacing(block, scale);
        }
    }

    // Vertically center the content block, but clamp so it doesn't go above top padding
    let available_height = rect.height() - v_padding * 2.0;
    let start_y = if total_height < available_height {
        rect.top() + v_padding + (available_height - total_height) / 2.0
    } else {
        rect.top() + v_padding
    };

    // Second pass: render
    let mut y = start_y;
    for (i, block) in slide.blocks.iter().enumerate() {
        match block {
            Block::Heading { level, inlines } => {
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
                y += h;
            }
            Block::List { ordered, items } => {
                let h = text::draw_list(
                    ui,
                    items,
                    *ordered,
                    theme,
                    Pos2::new(content_left, y),
                    content_width,
                    opacity,
                    0,
                    reveal_step,
                    scale,
                );
                y += h;
            }
            Block::Paragraph { inlines } => {
                let h = text::draw_paragraph(
                    ui,
                    inlines,
                    theme,
                    Pos2::new(content_left, y),
                    content_width,
                    opacity,
                    scale,
                );
                y += h;
            }
            _ => {
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
                y += h;
            }
        }
        if i < slide.blocks.len() - 1 {
            y += block_spacing(block, scale);
        }
    }
}

fn block_spacing(block: &Block, scale: f32) -> f32 {
    match block {
        Block::Heading { .. } => 30.0 * scale,
        Block::HorizontalRule => 10.0 * scale,
        _ => 20.0 * scale,
    }
}

fn measure_block_height(
    ui: &egui::Ui,
    block: &Block,
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    match block {
        Block::Heading { level, inlines } => {
            let size = theme.heading_size(*level) * scale;
            let job = text::inlines_to_job(inlines, size, theme.heading_color, max_width);
            ui.painter().layout_job(job).rect.height()
        }
        Block::Paragraph { inlines } | Block::BlockQuote { inlines } => {
            let size = theme.body_size * scale;
            let job = text::inlines_to_job(inlines, size, theme.foreground, max_width);
            ui.painter().layout_job(job).rect.height()
        }
        Block::List { items, .. } => {
            let font_size = theme.body_size * scale;
            let item_spacing = 8.0 * scale;
            count_list_items(items) as f32 * (font_size + item_spacing)
        }
        Block::CodeBlock { code, .. } => {
            let line_count = code.lines().count().max(1);
            let line_height = theme.code_size * scale * 1.4;
            let padding = 16.0 * scale;
            line_count as f32 * line_height + padding * 2.0
        }
        Block::Table { rows, .. } => {
            let row_height = theme.body_size * scale * 1.6;
            rows.len() as f32 * row_height + 10.0 * scale
        }
        Block::HorizontalRule => 2.0 * scale,
        _ => theme.body_size * scale * 1.5,
    }
}

fn count_list_items(items: &[crate::parser::ListItem]) -> usize {
    let mut count = items.len();
    for item in items {
        count += count_list_items(&item.children);
    }
    count
}
