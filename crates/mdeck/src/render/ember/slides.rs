//! Title, section and copy slides.

use eframe::egui::{self, Pos2, Rect};

use super::jobs::{display_job, eyebrow_job, lead_job};
use super::pieces::{content_pieces, draw_pieces, full_height};
use super::pillow::pillow;
use super::{Frame, copy_column, slide_eyebrow, stagger};
use crate::parser::{Block, Slide};
use crate::render::SlideContext;

/// Eyebrow over the title page: author and deck title, whichever are set.
fn title_eyebrow(deck: &SlideContext) -> String {
    match (&deck.author, &deck.deck_title) {
        (Some(a), Some(t)) if a != t => format!("{a} · {t}"),
        (Some(a), _) => a.clone(),
        (None, Some(t)) => t.clone(),
        (None, None) => "mdeck".to_string(),
    }
}

pub(super) fn render_title(f: &Frame, slide: &Slide) {
    let (ui, theme, rect, sz, scale) = (f.ui, f.theme, f.rect, &f.sz, f.scale);
    let mut heading = None;
    let mut subtitle = None;
    for block in &slide.blocks {
        match block {
            Block::Heading { level: 1, inlines } => heading = Some(inlines),
            Block::Heading { inlines, .. } if subtitle.is_none() => subtitle = Some(inlines),
            Block::Paragraph { inlines } if subtitle.is_none() => subtitle = Some(inlines),
            _ => {}
        }
    }
    let width = rect.width() * 0.62;
    let eyebrow_text = title_eyebrow(f.deck);
    let eyebrow = ui
        .painter()
        .layout_job(eyebrow_job(theme, "", &eyebrow_text, sz.eyebrow, 1.0));
    let title = heading.map(|h| {
        ui.painter()
            .layout_job(display_job(h, sz.h1, theme.heading_color, width, theme))
    });
    let sub = subtitle.map(|s| {
        ui.painter()
            .layout_job(lead_job(s, sz.lead * 1.05, 1.0, width * 0.8, theme))
    });

    let gap1 = 30.0 * scale;
    let gap2 = 40.0 * scale;
    let total = eyebrow.rect.height()
        + gap1
        + title.as_ref().map(|g| g.rect.height()).unwrap_or(0.0)
        + sub.as_ref().map(|g| gap2 + g.rect.height()).unwrap_or(0.0);
    let copy_w = title
        .as_ref()
        .map(|g| g.rect.width())
        .unwrap_or(width)
        .max(sub.as_ref().map(|g| g.rect.width()).unwrap_or(0.0));
    let top = rect.center().y - total / 2.0 - rect.height() * 0.02;
    let copy = Rect::from_center_size(
        Pos2::new(rect.center().x, top + total / 2.0),
        egui::vec2(copy_w, total),
    );
    let centered = |g: &egui::Galley| rect.center().x - g.rect.width() / 2.0;
    if let Some(g) = &title {
        f.heading_hint(
            g,
            Pos2::new(centered(g), top + eyebrow.rect.height() + gap1),
        );
    }
    pillow(ui.painter(), copy, f.pillow_alpha(), theme);
    if f.age < 0.0 {
        ui.ctx().request_repaint();
        return;
    }

    let mut y = top;

    f.place(eyebrow.clone(), Pos2::new(centered(&eyebrow), y), 0);
    y += eyebrow.rect.height() + gap1;
    if let Some(g) = title {
        f.place(g.clone(), Pos2::new(centered(&g), y), 1);
        y += g.rect.height() + gap2;
    }
    if let Some(g) = sub {
        f.place(g.clone(), Pos2::new(centered(&g), y), 2);
    }
    draw_begin_hint(f);
}

/// "Space to begin" hint, as on the site, on the very first slide only,
/// and only live: exports and thumbnails are stills, not waiting for a key.
fn draw_begin_hint(f: &Frame) {
    let painter = f.ui.painter();
    if f.deck.index == 0 && f.deck.animate {
        let p = stagger(f.age, 6);
        let hint = painter.layout_job(eyebrow_job(
            f.theme,
            "",
            "Space to begin",
            f.sz.eyebrow * 0.85,
            1.0,
        ));
        crate::render::math::galley_faded(
            painter,
            Pos2::new(
                f.rect.center().x - hint.rect.width() / 2.0,
                f.rect.bottom() - 62.0 * f.scale,
            ),
            hint,
            f.opacity * p * 0.9,
        );
    }
    if stagger(f.age, 6) < 1.0 {
        f.ui.ctx().request_repaint();
    }
}

pub(super) fn render_section(f: &Frame, slide: &Slide) {
    let (ui, theme, rect, sz, scale) = (f.ui, f.theme, f.rect, &f.sz, f.scale);
    let heading = slide.blocks.iter().find_map(|b| match b {
        Block::Heading { inlines, .. } => Some(inlines),
        _ => None,
    });
    let column = copy_column(rect);
    let width = rect.width() * 0.56;
    let (num, rest) = slide_eyebrow(f.deck);
    let eyebrow = ui
        .painter()
        .layout_job(eyebrow_job(theme, &num, &rest, sz.eyebrow, 1.0));
    let title = heading.map(|h| {
        ui.painter()
            .layout_job(display_job(h, sz.h1, theme.heading_color, width, theme))
    });
    let gap = 30.0 * scale;
    let total =
        eyebrow.rect.height() + gap + title.as_ref().map(|g| g.rect.height()).unwrap_or(0.0);
    // Sits low, like the site's `slide--bottom`.
    let top = rect.bottom() - rect.height() * 0.14 - total;
    let copy_w = title.as_ref().map(|g| g.rect.width()).unwrap_or(width);
    let copy = Rect::from_min_size(Pos2::new(column.left(), top), egui::vec2(copy_w, total));
    if let Some(g) = &title {
        f.heading_hint(
            g,
            Pos2::new(column.left(), top + eyebrow.rect.height() + gap),
        );
    }
    pillow(ui.painter(), copy, f.pillow_alpha(), theme);
    if f.age < 0.0 {
        ui.ctx().request_repaint();
        return;
    }

    f.place(eyebrow.clone(), Pos2::new(column.left(), top), 0);
    if let Some(g) = title {
        let y = top + eyebrow.rect.height() + gap;
        f.place(g, Pos2::new(column.left(), y), 1);
    }
}

pub(super) fn render_copy(f: &Frame, slide: &Slide) {
    let (ui, theme, rect, sz, scale) = (f.ui, f.theme, f.rect, &f.sz, f.scale);
    let column = copy_column(rect);
    let (num, rest) = slide_eyebrow(f.deck);
    let eyebrow = ui
        .painter()
        .layout_job(eyebrow_job(theme, &num, &rest, sz.eyebrow, 1.0));
    let pieces = content_pieces(ui, slide, theme, sz, column.width(), scale);

    let eyebrow_gap = 22.0 * scale;
    let total = eyebrow.rect.height() + eyebrow_gap + full_height(&pieces);
    let available = rect.height() * 0.80;
    let top = if total < available {
        rect.center().y - total / 2.0
    } else {
        rect.top() + rect.height() * 0.10
    };
    let copy_w = pieces
        .iter()
        .map(|p| p.galley.rect.width() + p.indent)
        .fold(eyebrow.rect.width(), f32::max);
    let copy = Rect::from_min_size(
        Pos2::new(column.left(), top),
        egui::vec2(copy_w.min(column.width()), total.min(rect.height() * 0.9)),
    );
    pillow(ui.painter(), copy, f.pillow_alpha(), theme);

    f.place(eyebrow.clone(), Pos2::new(column.left(), top), 0);
    let pieces_top = top + eyebrow.rect.height() + eyebrow_gap;
    draw_pieces(f, &pieces, column.left(), pieces_top, 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_eyebrow_names_author_and_deck_once() {
        let mut deck = SlideContext::default();
        assert_eq!(title_eyebrow(&deck), "mdeck");
        deck.deck_title = Some("Talk".into());
        assert_eq!(title_eyebrow(&deck), "Talk");
        deck.author = Some("Ada".into());
        assert_eq!(title_eyebrow(&deck), "Ada · Talk");
        deck.deck_title = Some("Ada".into());
        assert_eq!(title_eyebrow(&deck), "Ada");
    }
}
