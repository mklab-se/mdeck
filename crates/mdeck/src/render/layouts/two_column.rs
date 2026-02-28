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
    let content_width = rect.width() * 0.80;
    let h_offset = (rect.width() - content_width) / 2.0;
    let content_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + h_offset, rect.top() + v_padding),
        egui::pos2(rect.right() - h_offset, rect.bottom() - v_padding),
    );
    let gap = 40.0 * scale;
    let col_width = (content_rect.width() - gap) / 2.0;

    // Split blocks at ColumnSeparator
    let mut left_blocks: Vec<&Block> = Vec::new();
    let mut right_blocks: Vec<&Block> = Vec::new();
    let mut heading_blocks: Vec<&Block> = Vec::new();
    let mut in_right = false;
    let mut found_separator = false;

    for block in &slide.blocks {
        if matches!(block, Block::ColumnSeparator) {
            in_right = true;
            found_separator = true;
            continue;
        }
        if !found_separator
            && matches!(
                block,
                Block::Heading { level: 1, .. } | Block::Heading { level: 2, .. }
            )
            && left_blocks.is_empty()
        {
            heading_blocks.push(block);
            continue;
        }
        if in_right {
            right_blocks.push(block);
        } else {
            left_blocks.push(block);
        }
    }

    // Measure heading height
    let mut heading_height = 0.0;
    for block in &heading_blocks {
        if let Block::Heading { level, inlines } = *block {
            let size = theme.heading_size(*level) * scale;
            let job =
                text::inlines_to_job(inlines, size, theme.heading_color, content_rect.width());
            heading_height += ui.painter().layout_job(job).rect.height() + 30.0 * scale;
        }
    }

    // Measure column content heights
    let left_height = measure_column_height(ui, &left_blocks, theme, col_width, scale);
    let right_height = measure_column_height(ui, &right_blocks, theme, col_width, scale);
    let col_height = left_height.max(right_height);
    let total_height = heading_height + col_height;

    // Vertically center
    let available_height = content_rect.height();
    let start_y = if total_height < available_height {
        content_rect.top() + (available_height - total_height) / 2.0
    } else {
        content_rect.top()
    };

    let mut y = start_y;

    // Draw heading spanning full width
    for block in &heading_blocks {
        if let Block::Heading { level, inlines } = block {
            let h = text::draw_heading(
                ui,
                inlines,
                *level,
                theme,
                Pos2::new(content_rect.left(), y),
                content_rect.width(),
                opacity,
                scale,
            );
            y += h + 30.0 * scale;
        }
    }

    // Draw left column
    draw_column_blocks(
        ui,
        &left_blocks,
        theme,
        Pos2::new(content_rect.left(), y),
        col_width,
        opacity,
        image_cache,
        reveal_step,
        scale,
    );

    // Draw right column
    draw_column_blocks(
        ui,
        &right_blocks,
        theme,
        Pos2::new(content_rect.left() + col_width + gap, y),
        col_width,
        opacity,
        image_cache,
        reveal_step,
        scale,
    );
}

fn measure_column_height(
    ui: &egui::Ui,
    blocks: &[&Block],
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    let block_spacing = 16.0 * scale;
    let mut total = 0.0;
    for (i, block) in blocks.iter().enumerate() {
        total += text::measure_single_block_height(ui, block, theme, max_width, scale);
        if i < blocks.len() - 1 {
            total += block_spacing;
        }
    }
    total
}

#[allow(clippy::too_many_arguments)]
fn draw_column_blocks(
    ui: &egui::Ui,
    blocks: &[&Block],
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    opacity: f32,
    image_cache: &ImageCache,
    reveal_step: usize,
    scale: f32,
) {
    let block_spacing = 16.0 * scale;
    let mut y_offset = 0.0;

    for block in blocks {
        let block_pos = Pos2::new(pos.x, pos.y + y_offset);
        let height = text::draw_block(
            ui,
            block,
            theme,
            block_pos,
            max_width,
            opacity,
            image_cache,
            reveal_step,
            scale,
        );
        y_offset += height + block_spacing;
    }
}
