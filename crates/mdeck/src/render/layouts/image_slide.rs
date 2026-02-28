use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::image_cache::ImageCache;
use crate::render::text;
use crate::theme::Theme;

/// Image slide layout: prominent image with optional heading and caption.
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
    let padding = 60.0 * scale;

    // Find the image block and optional heading/caption
    let mut heading: Option<&Block> = None;
    let mut image_block: Option<&Block> = None;
    let mut caption: Option<&Block> = None;

    for block in &slide.blocks {
        match block {
            Block::Heading { .. } if heading.is_none() && image_block.is_none() => {
                heading = Some(block);
            }
            Block::Image { .. } if image_block.is_none() => {
                image_block = Some(block);
            }
            Block::Paragraph { .. } if image_block.is_some() && caption.is_none() => {
                caption = Some(block);
            }
            _ => {}
        }
    }

    let Some(Block::Image {
        alt,
        path,
        directives,
    }) = image_block
    else {
        // Fallback to content layout if no image found
        text::draw_blocks(
            ui,
            &slide.blocks,
            theme,
            Pos2::new(rect.left() + padding, rect.top() + padding),
            rect.width() - padding * 2.0,
            opacity,
            image_cache,
            reveal_step,
            scale,
        );
        return;
    };

    // Check if this is a fill image (covers entire slide)
    if directives.fill {
        text::draw_image_in_area(ui, path, alt, directives, theme, rect, opacity, image_cache);

        // Draw heading on top of the image with a semi-transparent overlay
        if let Some(Block::Heading { level, inlines }) = heading {
            let overlay_height = 80.0 * scale;
            let overlay_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.bottom() - overlay_height - 40.0 * scale),
                egui::vec2(rect.width(), overlay_height),
            );
            let overlay_bg = Theme::with_opacity(theme.background, opacity * 0.6);
            ui.painter().rect_filled(overlay_rect, 0.0, overlay_bg);

            text::draw_heading(
                ui,
                inlines,
                *level,
                theme,
                Pos2::new(
                    overlay_rect.left() + padding,
                    overlay_rect.top() + 16.0 * scale,
                ),
                rect.width() - padding * 2.0,
                opacity,
                scale,
            );
        }
        return;
    }

    // Non-fill: heading at top, image centered, optional caption below
    let content_width = rect.width() - padding * 2.0;
    let mut y = rect.top() + padding;

    if let Some(Block::Heading { level, inlines }) = heading {
        let h = text::draw_heading(
            ui,
            inlines,
            *level,
            theme,
            Pos2::new(rect.left() + padding, y),
            content_width,
            opacity,
            scale,
        );
        y += h + 20.0 * scale;
    }

    let caption_reserve = if caption.is_some() { 50.0 * scale } else { 0.0 };
    let image_area_height = rect.bottom() - y - padding - caption_reserve;

    let image_available = egui::Rect::from_min_size(
        Pos2::new(rect.left() + padding, y),
        egui::vec2(content_width, image_area_height),
    );

    let image_drawn_rect = text::draw_image_in_area(
        ui,
        path,
        alt,
        directives,
        theme,
        image_available,
        opacity,
        image_cache,
    );

    if let Some(Block::Paragraph { inlines }) = caption {
        let caption_color = Theme::with_opacity(theme.foreground, opacity * 0.7);
        let caption_size = theme.body_size * 0.9 * scale;

        // Center caption under the drawn image
        let caption_y = image_drawn_rect.bottom() + 10.0 * scale;
        let job = text::inlines_to_job(
            inlines,
            caption_size,
            caption_color,
            image_drawn_rect.width(),
        );
        let galley = ui.painter().layout_job(job);
        let caption_x =
            image_drawn_rect.left() + (image_drawn_rect.width() - galley.rect.width()) / 2.0;
        ui.painter()
            .galley(Pos2::new(caption_x, caption_y), galley, caption_color);
    }
}
