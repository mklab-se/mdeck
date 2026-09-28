use eframe::egui::{self, Pos2};

use crate::parser::{Block, Inline, Slide};
use crate::render::layouts::{SLIDE_PADDING, image_split};
use crate::render::text;
use crate::render::{BlockCx, TextCx};
use crate::theme::Theme;

pub fn render(cx: &BlockCx, slide: &Slide, rect: egui::Rect) {
    let padding = SLIDE_PADDING * cx.scale;
    let content_rect = rect.shrink(padding);

    if image_split::has_image(&slide.blocks) {
        let (left_rect, right_rect) = image_split::image_split_rects(content_rect);

        // Render quote content in the left area
        render_quote_content(&cx.text(), slide, left_rect);

        // Render image in the right area
        if let (
            _,
            Some(Block::Image {
                alt,
                path,
                directives,
            }),
        ) = image_split::split_image(&slide.blocks)
        {
            text::draw_image_in_area(cx, path, alt, directives, right_rect);
        }
    } else {
        render_quote_content(&cx.text(), slide, content_rect);
    }
}

fn render_quote_content(cx: &TextCx, slide: &Slide, content_rect: egui::Rect) {
    let (ui, theme, opacity, scale) = (cx.ui, cx.theme, cx.opacity, cx.scale);
    // Find heading, quote, and attribution
    let mut heading: Option<(u8, &Vec<Inline>)> = None;
    let mut quote_inlines: Option<&Vec<Inline>> = None;
    let mut attribution: Option<&Vec<Inline>> = None;

    for block in &slide.blocks {
        match block {
            Block::Heading { level, inlines } => heading = Some((*level, inlines)),
            Block::BlockQuote { inlines } => quote_inlines = Some(inlines),
            Block::Paragraph { inlines } if quote_inlines.is_some() => {
                attribution = Some(inlines);
            }
            _ => {}
        }
    }

    let quote_size = theme.body_size * 1.3 * scale;
    let quote_gap = 30.0 * scale;
    let quote_width = content_rect.width() * 0.8;
    let quote_x = content_rect.left() + (content_rect.width() - quote_width) / 2.0;

    // Lay everything out first so vertical centring uses real wrapped heights
    let heading_color = Theme::with_opacity(theme.heading_color, opacity);
    let heading_galley = heading.map(|(level, inlines)| {
        let size = theme.heading_size(level) * scale;
        let job =
            text::display_inlines_job(inlines, size, heading_color, content_rect.width(), theme);
        (level, ui.painter().layout_job(job))
    });

    let quote_color = Theme::with_opacity(theme.foreground, opacity);
    let quote_galley = quote_inlines.map(|inlines| {
        // Build inlines with quotation marks baked in (if not already present)
        let quoted = wrap_with_quotes(inlines);
        let job = text::inlines_to_job(&quoted, quote_size, quote_color, quote_width, theme);
        ui.painter().layout_job(job)
    });

    let attr_color = Theme::with_opacity(theme.foreground, opacity * 0.7);
    let attr_galley = attribution.map(|inlines| {
        let attr_size = theme.body_size * 0.9 * scale;
        // Strip leading -- or --- from attribution
        let cleaned = clean_attribution(inlines);
        let job = text::inlines_to_job(&cleaned, attr_size, attr_color, quote_width, theme);
        ui.painter().layout_job(job)
    });

    let mut total_height = 0.0;
    if let Some((level, galley)) = &heading_galley {
        total_height += galley.rect.height() + text::heading_spacing(theme, *level, scale);
    }
    if let Some(galley) = &quote_galley {
        total_height += galley.rect.height();
    }
    if let Some(galley) = &attr_galley {
        total_height += quote_gap + galley.rect.height();
    }

    let start_y = (content_rect.center().y - total_height / 2.0).max(content_rect.top());
    let mut y = start_y;

    // Draw heading if present
    if let Some((level, galley)) = heading_galley {
        let h = galley.rect.height();
        crate::render::math::galley(
            ui.painter(),
            Pos2::new(content_rect.left(), y),
            galley,
            heading_color,
        );
        y += h + text::heading_spacing(theme, level, scale);
    }

    // Draw quote - centred with larger text, quotation marks inline.
    // Remember its right edge so the attribution can align to it.
    let mut quote_right = content_rect.right();
    if let Some(galley) = quote_galley {
        let accent = Theme::with_opacity(theme.accent, opacity);
        let text_height = galley.rect.height();
        let text_width = galley.rect.width();
        let text_x = quote_x + (quote_width - text_width) / 2.0;
        quote_right = text_x + text_width;

        crate::render::math::galley(ui.painter(), Pos2::new(text_x, y), galley, quote_color);

        // Left accent bar spanning the quote text
        let bar_width = 4.0 * scale;
        let bar_x = quote_x - 16.0 * scale;
        let bar_rect =
            egui::Rect::from_min_size(Pos2::new(bar_x, y), egui::vec2(bar_width, text_height));
        ui.painter().rect_filled(bar_rect, 2.0, accent);

        y += text_height;
    }

    // Draw attribution - right-aligned to the quote text
    if let Some(galley) = attr_galley {
        y += quote_gap;
        let x = quote_right - galley.rect.width();
        crate::render::math::galley(ui.painter(), Pos2::new(x, y), galley, attr_color);
    }
}

/// Wraps quote inlines with curly quotation marks if they don't already have them.
fn wrap_with_quotes(inlines: &[Inline]) -> Vec<Inline> {
    let starts_with_quote = inlines.first().is_some_and(|first| {
        if let Inline::Text(s) = first {
            let t = s.trim_start();
            t.starts_with('\u{201C}') || t.starts_with('"')
        } else {
            false
        }
    });
    let ends_with_quote = inlines.last().is_some_and(|last| {
        if let Inline::Text(s) = last {
            let t = s.trim_end();
            t.ends_with('\u{201D}') || t.ends_with('"')
        } else {
            false
        }
    });

    if starts_with_quote && ends_with_quote {
        return inlines.to_vec();
    }

    let mut result = Vec::with_capacity(inlines.len() + 2);
    if !starts_with_quote {
        result.push(Inline::Text("\u{201C}".to_string()));
    }
    result.extend(inlines.iter().cloned());
    if !ends_with_quote {
        result.push(Inline::Text("\u{201D}".to_string()));
    }
    result
}

fn clean_attribution(inlines: &[Inline]) -> Vec<Inline> {
    let mut result = inlines.to_vec();
    if let Some(Inline::Text(s)) = result.first_mut() {
        let trimmed = s.trim_start();
        if let Some(rest) = trimmed.strip_prefix("---") {
            *s = format!("\u{2014} {}", rest.trim_start());
        } else if let Some(rest) = trimmed.strip_prefix("--") {
            *s = format!("\u{2014} {}", rest.trim_start());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_with_curly_quotes_when_missing() {
        let out = wrap_with_quotes(&[Inline::Text("hello".into())]);
        assert_eq!(out.len(), 3);
        assert!(matches!(&out[0], Inline::Text(s) if s == "\u{201C}"));
        assert!(matches!(&out[2], Inline::Text(s) if s == "\u{201D}"));
    }

    #[test]
    fn keeps_existing_quotes() {
        let out = wrap_with_quotes(&[Inline::Text("\"hello\"".into())]);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn attribution_dashes_become_em_dash() {
        let out = clean_attribution(&[Inline::Text("-- Alan Kay".into())]);
        assert!(matches!(&out[0], Inline::Text(s) if s == "\u{2014} Alan Kay"));
        let out = clean_attribution(&[Inline::Text("--- Ada".into())]);
        assert!(matches!(&out[0], Inline::Text(s) if s == "\u{2014} Ada"));
    }
}
