use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::layouts::SLIDE_PADDING;
use crate::render::text;
use crate::theme::Theme;

const SUBTITLE_GAP: f32 = 20.0;

pub fn render(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: egui::Rect,
    opacity: f32,
    scale: f32,
) {
    let padding = SLIDE_PADDING * scale;
    let content_rect = rect.shrink(padding);

    // Find heading and subtitle
    let mut heading_inlines = None;
    let mut subtitle_inlines = None;

    for block in &slide.blocks {
        match block {
            Block::Heading { level: 1, inlines } => heading_inlines = Some(inlines),
            Block::Heading { level: 2, inlines } => subtitle_inlines = Some(inlines),
            Block::Paragraph { inlines } if subtitle_inlines.is_none() => {
                subtitle_inlines = Some(inlines);
            }
            _ => {}
        }
    }

    let title_size = theme.h1_size * 1.1 * scale;
    let subtitle_size = theme.h2_size * 0.7 * scale;
    let title_color = Theme::with_opacity(theme.heading_color, opacity);
    let subtitle_color = Theme::with_opacity(theme.foreground, opacity * 0.8);

    // Lay both out first so centring uses the real (possibly wrapped) heights
    let title_galley = heading_inlines.map(|inlines| {
        let job = text::inlines_to_job(
            inlines,
            title_size,
            title_color,
            content_rect.width(),
            theme,
        );
        ui.painter().layout_job(job)
    });
    let subtitle_galley = subtitle_inlines.map(|inlines| {
        let job = text::inlines_to_job(
            inlines,
            subtitle_size,
            subtitle_color,
            content_rect.width(),
            theme,
        );
        ui.painter().layout_job(job)
    });

    let gap = SUBTITLE_GAP * scale;
    let total_height = match (&title_galley, &subtitle_galley) {
        (Some(t), Some(s)) => t.rect.height() + gap + s.rect.height(),
        (Some(t), None) => t.rect.height(),
        (None, Some(s)) => s.rect.height(),
        (None, None) => 0.0,
    };

    // Centre vertically, but never start above the padded area
    let mut y = (content_rect.center().y - total_height / 2.0).max(content_rect.top());

    // Draw title centred
    if let Some(galley) = title_galley {
        let x = content_rect.left() + (content_rect.width() - galley.rect.width()) / 2.0;
        let h = galley.rect.height();
        ui.painter().galley(Pos2::new(x, y), galley, title_color);
        y += h + gap;
    }

    // Draw subtitle centred
    if let Some(galley) = subtitle_galley {
        let x = content_rect.left() + (content_rect.width() - galley.rect.width()) / 2.0;
        ui.painter().galley(Pos2::new(x, y), galley, subtitle_color);
    }
}
