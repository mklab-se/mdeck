use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::BlockCx;
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

/// The blocks an image slide shows: the first image, the heading before it
/// and the paragraph after it (the caption).
struct Parts<'a> {
    heading: Option<&'a Block>,
    image: Option<&'a Block>,
    caption: Option<&'a Block>,
}

fn parts(blocks: &[Block]) -> Parts<'_> {
    let mut parts = Parts {
        heading: None,
        image: None,
        caption: None,
    };
    for block in blocks {
        match block {
            Block::Heading { .. } if parts.heading.is_none() && parts.image.is_none() => {
                parts.heading = Some(block);
            }
            Block::Image { .. } if parts.image.is_none() => {
                parts.image = Some(block);
            }
            Block::Paragraph { .. } if parts.image.is_some() && parts.caption.is_none() => {
                parts.caption = Some(block);
            }
            _ => {}
        }
    }
    parts
}

/// Image slide layout: prominent image with optional heading and caption.
pub fn render(cx: &BlockCx, slide: &Slide, rect: egui::Rect) {
    let padding = MEDIA_PADDING * cx.scale;
    let parts = parts(&slide.blocks);

    let Some(Block::Image {
        alt,
        path,
        directives,
    }) = parts.image
    else {
        // Fallback to content layout if no image found
        text::draw_blocks(
            cx,
            &slide.blocks,
            Pos2::new(rect.left() + padding, rect.top() + padding),
            rect.width() - padding * 2.0,
        );
        return;
    };

    // A fill image covers the entire slide
    if directives.fill {
        text::draw_image_in_area(cx, path, alt, directives, rect);
        if let Some(heading) = parts.heading {
            draw_band_heading(cx, heading, rect);
        }
    } else {
        draw_framed(cx, &parts, rect);
    }
}

/// Draw the heading over a fill image, in a semi-transparent band sized
/// from the heading's real (possibly wrapped) height.
fn draw_band_heading(cx: &BlockCx, heading: &Block, rect: egui::Rect) {
    let Block::Heading { level, inlines } = heading else {
        return;
    };
    let (theme, scale) = (cx.theme, cx.scale);
    let padding = MEDIA_PADDING * scale;
    let content_width = rect.width() - padding * 2.0;
    let heading_h = text::measure_single_block_height(cx.ui, heading, theme, content_width, scale);
    let band_h = fill_band_height(heading_h, scale);
    let band_rect = egui::Rect::from_min_size(
        egui::pos2(
            rect.left(),
            rect.bottom() - band_h - BAND_BOTTOM_MARGIN * scale,
        ),
        egui::vec2(rect.width(), band_h),
    );
    let band_bg = Theme::with_opacity(theme.background, cx.opacity * 0.6);
    cx.ui.painter().rect_filled(band_rect, 0.0, band_bg);

    let pos = Pos2::new(
        band_rect.left() + padding,
        band_rect.top() + BAND_PADDING * scale,
    );
    text::draw_heading(&cx.text(), inlines, *level, pos, content_width);
}

/// Heading at the top, the image centred below it and the caption, if any,
/// centred under the image.
fn draw_framed(cx: &BlockCx, parts: &Parts, rect: egui::Rect) {
    let Some(Block::Image {
        alt,
        path,
        directives,
    }) = parts.image
    else {
        return;
    };
    let (theme, scale) = (cx.theme, cx.scale);
    let padding = MEDIA_PADDING * scale;
    let content_width = rect.width() - padding * 2.0;
    let mut y = rect.top() + padding;

    if let Some(Block::Heading { level, inlines }) = parts.heading {
        let pos = Pos2::new(rect.left() + padding, y);
        let h = text::draw_heading(&cx.text(), inlines, *level, pos, content_width);
        y += h + 20.0 * scale;
    }

    let caption_reserve = if parts.caption.is_some() {
        50.0 * scale
    } else {
        0.0
    };
    let image_area_height = rect.bottom() - y - padding - caption_reserve;

    let image_available = egui::Rect::from_min_size(
        Pos2::new(rect.left() + padding, y),
        egui::vec2(content_width, image_area_height),
    );

    let image_drawn_rect = text::draw_image_in_area(cx, path, alt, directives, image_available);

    if let Some(Block::Paragraph { inlines }) = parts.caption {
        let caption_color = Theme::with_opacity(theme.foreground, cx.opacity * 0.7);
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
        let galley = cx.ui.painter().layout_job(job);
        let caption_x =
            image_drawn_rect.left() + (image_drawn_rect.width() - galley.rect.width()) / 2.0;
        crate::render::math::galley(
            cx.ui.painter(),
            Pos2::new(caption_x, caption_y),
            galley,
            caption_color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Inline;
    use crate::render::test_support::with_ui;

    #[test]
    fn parts_take_the_heading_before_and_the_caption_after_the_image() {
        let para = |s: &str| Block::Paragraph {
            inlines: vec![Inline::Text(s.into())],
        };
        let image = Block::Image {
            alt: String::new(),
            path: "a.png".into(),
            directives: Default::default(),
        };
        let heading = Block::Heading {
            level: 2,
            inlines: vec![Inline::Text("Title".into())],
        };
        let blocks = vec![
            para("before"),
            heading.clone(),
            image.clone(),
            para("caption"),
            para("more"),
            image,
        ];
        let p = parts(&blocks);
        assert!(std::ptr::eq(p.heading.unwrap(), &blocks[1]));
        assert!(std::ptr::eq(p.image.unwrap(), &blocks[2]));
        assert!(std::ptr::eq(p.caption.unwrap(), &blocks[3]));

        // A heading after the image is not the slide's heading
        let late = vec![blocks[2].clone(), heading];
        let p = parts(&late);
        assert!(p.heading.is_none() && p.caption.is_none());
    }

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
