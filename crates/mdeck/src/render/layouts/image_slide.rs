use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::image_cache::ImageCache;
use crate::render::layouts::MEDIA_PADDING;
use crate::render::text;
use crate::theme::Theme;

/// Vertical padding inside the heading band drawn over a fill image.
const BAND_PADDING: f32 = 16.0;
/// Distance from the slide bottom to the heading band.
const BAND_BOTTOM_MARGIN: f32 = 40.0;

/// Height of the heading band over a fill image: the wrapped heading height
/// plus padding, so any heading level fits.
fn fill_band_height(heading_height: f32, scale: f32) -> f32 {
    heading_height + BAND_PADDING * 2.0 * scale
}

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
    let padding = MEDIA_PADDING * scale;

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

    let content_width = rect.width() - padding * 2.0;

    // Check if this is a fill image (covers entire slide)
    if directives.fill {
        text::draw_image_in_area(
            ui,
            path,
            alt,
            directives,
            theme,
            rect,
            opacity,
            image_cache,
            scale,
        );

        // Draw heading on top of the image in a semi-transparent band sized
        // from the heading's real (possibly wrapped) height.
        if let Some(heading_block @ Block::Heading { level, inlines }) = heading {
            let heading_h =
                text::measure_single_block_height(ui, heading_block, theme, content_width, scale);
            let band_h = fill_band_height(heading_h, scale);
            let band_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left(),
                    rect.bottom() - band_h - BAND_BOTTOM_MARGIN * scale,
                ),
                egui::vec2(rect.width(), band_h),
            );
            let band_bg = Theme::with_opacity(theme.background, opacity * 0.6);
            ui.painter().rect_filled(band_rect, 0.0, band_bg);

            text::draw_heading(
                ui,
                inlines,
                *level,
                theme,
                Pos2::new(
                    band_rect.left() + padding,
                    band_rect.top() + BAND_PADDING * scale,
                ),
                content_width,
                opacity,
                scale,
            );
        }
        return;
    }

    // Non-fill: heading at top, image centered, optional caption below
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
        scale,
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
            theme,
        );
        let galley = ui.painter().layout_job(job);
        let caption_x =
            image_drawn_rect.left() + (image_drawn_rect.width() - galley.rect.width()) / 2.0;
        ui.painter()
            .galley(Pos2::new(caption_x, caption_y), galley, caption_color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Inline;
    use crate::render::test_support::with_ui;

    #[test]
    fn fill_band_is_taller_than_the_heading_it_holds() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let h1 = Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("A fill image heading".into())],
            };
            let heading_h = text::measure_single_block_height(ui, &h1, &theme, 1800.0, 1.0);
            let band = fill_band_height(heading_h, 1.0);
            assert!(heading_h >= theme.h1_size, "{heading_h}");
            assert!(band > heading_h);
            // The previous fixed band (80px) could not hold an H1.
            assert!(band > 80.0);
        });
    }
}
