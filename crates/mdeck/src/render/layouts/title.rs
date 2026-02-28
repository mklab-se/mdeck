use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::text;
use crate::theme::Theme;

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

    // Find heading and subtitle
    let mut heading_inlines = None;
    let mut subtitle_inlines = None;

    for block in &slide.blocks {
        match block {
            Block::Heading { level: 1, inlines } => heading_inlines = Some(inlines),
            Block::Heading { level: 2, inlines } => subtitle_inlines = Some(inlines),
            Block::Paragraph { inlines } => {
                if subtitle_inlines.is_none() {
                    subtitle_inlines = Some(inlines);
                }
            }
            _ => {}
        }
    }

    // Center vertically
    let title_size = theme.h1_size * 1.1 * scale;
    let subtitle_size = theme.h2_size * 0.7 * scale;

    // Estimate total height for centering
    let mut total_height = 0.0;
    if heading_inlines.is_some() {
        total_height += title_size * 1.2;
    }
    if subtitle_inlines.is_some() {
        total_height += subtitle_size * 1.2 + 20.0 * scale;
    }

    let start_y = content_rect.center().y - total_height / 2.0;
    let mut y = start_y;

    // Draw title centered
    if let Some(inlines) = heading_inlines {
        let color = Theme::with_opacity(theme.heading_color, opacity);
        let job = text::inlines_to_job(inlines, title_size, color, content_rect.width());
        let galley = ui.painter().layout_job(job);
        let x = content_rect.left() + (content_rect.width() - galley.rect.width()) / 2.0;
        let pos = Pos2::new(x, y);
        ui.painter().galley(pos, galley, color);
        y += title_size * 1.2 + 20.0 * scale;
    }

    // Draw subtitle centered
    if let Some(inlines) = subtitle_inlines {
        let color = Theme::with_opacity(theme.foreground, opacity * 0.8);
        let job = text::inlines_to_job(inlines, subtitle_size, color, content_rect.width());
        let galley = ui.painter().layout_job(job);
        let x = content_rect.left() + (content_rect.width() - galley.rect.width()) / 2.0;
        let pos = Pos2::new(x, y);
        ui.painter().galley(pos, galley, color);
    }
}
