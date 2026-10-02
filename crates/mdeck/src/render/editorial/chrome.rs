//! Chrome drawn over every slide: the counter, beat ticks and progress
//! hairline, and the presenter's say line.

use eframe::egui::{self, Color32, Pos2, Rect};

use super::fade;
use crate::render::SlideContext;
use crate::theme::Theme;

/// Counter and progress hairline, as on the site's talk decks. Engines
/// that number their slides themselves (the line engine's title block) leave
/// the counter out.
pub fn draw_chrome(
    painter: &egui::Painter,
    theme: &Theme,
    rect: Rect,
    cx: &SlideContext,
    scale: f32,
) {
    let counter = !theme.numbers_slides();
    let (index, count) = (cx.index, cx.count);
    let size = 15.0 * scale;
    let font = egui::FontId::new(size, theme.mono_family());
    let mut job = egui::text::LayoutJob::default();
    let fmt = |c: Color32| egui::text::TextFormat {
        font_id: font.clone(),
        color: c,
        extra_letter_spacing: size * 0.18,
        ..Default::default()
    };
    job.append(&format!("{:02}", index + 1), 0.0, fmt(theme.bright_text()));
    job.append(&format!(" · {:02}", count), 0.0, fmt(theme.muted));
    let galley = painter.layout_job(job);
    if counter {
        crate::render::math::galley(
            painter,
            Pos2::new(
                rect.right() - 32.0 * scale - galley.rect.width(),
                rect.bottom() - 26.0 * scale - galley.rect.height(),
            ),
            galley,
            theme.muted,
        );
    }

    // progress hairline
    let y = rect.bottom() - 1.0;
    painter.line_segment(
        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
        egui::Stroke::new(1.0, fade(theme.rule, 0.9)),
    );
    let frac = if count > 1 {
        index as f32 / (count - 1) as f32
    } else {
        1.0
    };
    painter.line_segment(
        [
            Pos2::new(rect.left(), y),
            Pos2::new(rect.left() + rect.width() * frac, y),
        ],
        egui::Stroke::new(1.0, theme.accent),
    );
}
