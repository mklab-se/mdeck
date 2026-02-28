use eframe::egui::{self, FontId, Pos2};

use crate::parser::{Block, Inline, Slide};
use crate::render::text;
use crate::theme::Theme;

#[allow(clippy::too_many_arguments)]
pub fn render(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: egui::Rect,
    opacity: f32,
    scale: f32,
) {
    let padding = 80.0 * scale;
    let content_rect = rect.shrink(padding);

    // Find heading, quote, and attribution
    let mut heading: Option<(u8, &Vec<Inline>)> = None;
    let mut quote_inlines: Option<&Vec<Inline>> = None;
    let mut attribution: Option<&Vec<Inline>> = None;

    for block in &slide.blocks {
        match block {
            Block::Heading { level, inlines } => heading = Some((*level, inlines)),
            Block::BlockQuote { inlines } => quote_inlines = Some(inlines),
            Block::Paragraph { inlines } => {
                if quote_inlines.is_some() {
                    attribution = Some(inlines);
                }
            }
            _ => {}
        }
    }

    let quote_size = theme.body_size * 1.3 * scale;

    // Estimate total height for vertical centering
    let mut total_height = 0.0;
    if heading.is_some() {
        total_height += theme.h2_size * scale + 40.0 * scale;
    }
    if quote_inlines.is_some() {
        total_height += quote_size * 3.0; // rough estimate
    }
    if attribution.is_some() {
        total_height += theme.body_size * scale + 20.0 * scale;
    }

    let start_y = if heading.is_some() {
        content_rect.top() + 20.0 * scale
    } else {
        (content_rect.center().y - total_height / 2.0).max(content_rect.top() + 20.0 * scale)
    };
    let mut y = start_y;

    // Draw heading if present
    if let Some((level, inlines)) = heading {
        let h = text::draw_heading(
            ui,
            inlines,
            level,
            theme,
            Pos2::new(content_rect.left(), y),
            content_rect.width(),
            opacity,
            scale,
        );
        y += h + 40.0 * scale;
    }

    // Draw quote - centered with larger text
    if let Some(inlines) = quote_inlines {
        let color = Theme::with_opacity(theme.foreground, opacity);
        let accent = Theme::with_opacity(theme.accent, opacity);
        let quote_width = content_rect.width() * 0.8;
        let quote_x = content_rect.left() + (content_rect.width() - quote_width) / 2.0;

        // Draw left accent bar
        let bar_width = 4.0 * scale;
        let bar_x = quote_x - 16.0 * scale;

        // Layout quote text to determine height
        let job = text::inlines_to_job(inlines, quote_size, color, quote_width);
        let galley = ui.painter().layout_job(job);
        let text_height = galley.rect.height();
        let text_width = galley.rect.width();
        let text_x = quote_x + (quote_width - text_width) / 2.0;

        // Draw opening quote mark flush with left edge of text, above it
        let quote_mark_size = quote_size * 2.0;
        let quote_mark_color = Theme::with_opacity(theme.accent, opacity * 0.5);
        let open_mark_galley = ui.painter().layout_no_wrap(
            "\u{201C}".to_string(),
            FontId::proportional(quote_mark_size),
            quote_mark_color,
        );
        ui.painter().galley(
            Pos2::new(text_x, y - quote_mark_size * 0.3),
            open_mark_galley,
            quote_mark_color,
        );

        // Draw the quote text
        let text_y = y + quote_mark_size * 0.4;
        ui.painter()
            .galley(Pos2::new(text_x, text_y), galley, color);

        // Draw left accent bar spanning the quote text
        let bar_rect =
            egui::Rect::from_min_size(Pos2::new(bar_x, text_y), egui::vec2(bar_width, text_height));
        ui.painter().rect_filled(bar_rect, 2.0, accent);

        // Draw closing quote mark at end of quote text
        let close_mark_galley = ui.painter().layout_no_wrap(
            "\u{201D}".to_string(),
            FontId::proportional(quote_mark_size),
            quote_mark_color,
        );
        let close_x = text_x + text_width;
        let close_y = text_y + text_height - quote_mark_size * 0.5;
        ui.painter().galley(
            Pos2::new(close_x, close_y),
            close_mark_galley,
            quote_mark_color,
        );

        y = text_y + text_height + 30.0 * scale;
    }

    // Draw attribution - right-aligned, italic
    if let Some(inlines) = attribution {
        let color = Theme::with_opacity(theme.foreground, opacity * 0.7);
        let attr_size = theme.body_size * 0.9 * scale;

        // Strip leading -- or --- from attribution
        let cleaned = clean_attribution(inlines);
        let job = text::inlines_to_job(&cleaned, attr_size, color, content_rect.width());

        let galley = ui.painter().layout_job(job);
        let x = content_rect.right() - galley.rect.width() - 40.0 * scale;
        ui.painter().galley(Pos2::new(x, y), galley, color);
    }
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
