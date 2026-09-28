use eframe::egui::{self, Pos2};

use crate::parser::{Block, Slide};
use crate::render::BlockCx;
use crate::render::text;
use crate::theme::Theme;

pub fn render(cx: &BlockCx, slide: &Slide, rect: egui::Rect) {
    let (ui, theme, opacity, scale) = (cx.ui, cx.theme, cx.opacity, cx.scale);
    let padding = 80.0 * scale;
    let content_rect = rect.shrink(padding);

    // Find the heading
    for block in &slide.blocks {
        if let Block::Heading { level, inlines } = block {
            let size = if *level == 1 {
                theme.h1_size * 1.2 * scale
            } else {
                theme.h2_size * 1.1 * scale
            };
            let color = Theme::with_opacity(theme.heading_color, opacity);
            let job = text::display_inlines_job(inlines, size, color, content_rect.width(), theme);
            let galley = ui.painter().layout_job(job);

            // Center both horizontally and vertically
            let x = content_rect.left() + (content_rect.width() - galley.rect.width()) / 2.0;
            let y = content_rect.center().y - galley.rect.height() / 2.0;
            let pos = Pos2::new(x, y);
            crate::render::math::galley(ui.painter(), pos, galley, color);
            return;
        }
    }
}
