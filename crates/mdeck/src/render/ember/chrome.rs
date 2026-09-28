//! Chrome drawn over every slide: the counter, beat ticks and progress
//! hairline, and the presenter's say line.

use eframe::egui::{self, Color32, Pos2, Rect};

use super::fade;
use crate::render::SlideContext;
use crate::theme::Theme;

/// Counter and progress hairline, as on the site's talk decks. Engines
/// that number their slides themselves (the blueprint's title block) leave
/// the counter out.
pub fn draw_chrome(
    painter: &egui::Painter,
    theme: &Theme,
    rect: Rect,
    cx: &SlideContext,
    scale: f32,
) {
    let counter = !theme.engine.numbers_slides();
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

    // beat ticks: how many steps this slide still has in it
    if let Some((cur, total)) = cx.beats
        && total > 1
    {
        let w = 10.0 * scale;
        let gap = 5.0 * scale;
        let right = rect.right() - 32.0 * scale;
        let y = rect.bottom() - 26.0 * scale - galley_h(painter, theme, size) - 12.0 * scale;
        for k in 0..total {
            let x1 = right - (total - 1 - k) as f32 * (w + gap);
            let color = if k <= cur { theme.accent } else { theme.rule };
            painter.line_segment(
                [Pos2::new(x1 - w, y), Pos2::new(x1, y)],
                egui::Stroke::new(1.0, color),
            );
        }
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

fn galley_h(painter: &egui::Painter, theme: &Theme, size: f32) -> f32 {
    painter
        .layout_no_wrap(
            "00".into(),
            egui::FontId::new(size, theme.mono_family()),
            theme.muted,
        )
        .rect
        .height()
}

/// The presenter's line for the current beat, bottom-left, presenter-only.
pub fn draw_say_line(painter: &egui::Painter, theme: &Theme, rect: Rect, line: &str, scale: f32) {
    let size = 18.0 * scale;
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = rect.width() * 0.5;
    job.append(
        line,
        0.0,
        egui::text::TextFormat {
            font_id: egui::FontId::new(size, theme.body_family()),
            color: theme.secondary,
            line_height: Some(size * 1.4),
            ..Default::default()
        },
    );
    let galley = painter.layout_job(job);
    let pos = Pos2::new(
        rect.left() + rect.width() * 0.07,
        rect.bottom() - 30.0 * scale - galley.rect.height(),
    );
    let bg = Rect::from_min_size(pos, galley.rect.size()).expand(10.0 * scale);
    painter.rect_filled(bg, 4.0 * scale, fade(theme.background, 0.7));
    crate::render::math::galley(painter, pos, galley, theme.secondary);
}
