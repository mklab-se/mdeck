//! The copy stack of a bullet or content slide: headings, lead paragraphs,
//! list items and quotes laid out as pieces, then drawn with the entry
//! stagger and reveal fades.

use std::sync::Arc;

use eframe::egui::{self, Pos2};

use super::jobs::{display_job, item_job, lead_job};
use super::{Frame, Sizes, ease_out, fade, stagger};
use crate::parser::{Block, ListItem, Slide};
use crate::theme::Theme;

/// One element of the copy stack, laid out and ready to draw.
pub(super) struct Piece {
    pub(super) galley: Arc<egui::Galley>,
    /// Space below this piece.
    gap: f32,
    /// Reveal step (0 = always) for list items.
    step: usize,
    /// Draw an ember dot to the left (list items).
    dot: bool,
    /// Draw an ember hairline to the left spanning the piece (quotes).
    bar: bool,
    /// Left inset of the text (list items step in from the copy edge).
    pub(super) indent: f32,
    /// A nested list item: smaller dot.
    nested: bool,
}

impl Piece {
    /// A piece that always shows, flush with the copy edge.
    fn plain(galley: Arc<egui::Galley>, gap: f32) -> Self {
        Self {
            galley,
            gap,
            step: 0,
            dot: false,
            bar: false,
            indent: 0.0,
            nested: false,
        }
    }

    /// A list item on reveal `step`, stepped in by `indent`.
    fn item(galley: Arc<egui::Galley>, gap: f32, step: usize, indent: f32, nested: bool) -> Self {
        Self {
            galley,
            gap,
            step,
            dot: true,
            bar: false,
            indent,
            nested,
        }
    }
}

/// How far list items step in from the copy edge, at reference scale. The
/// dot sits in the gutter this opens, so paragraphs and bullets read as two
/// different things (GitHub issue 9).
const ITEM_INDENT: f32 = 34.0;

fn item_pieces(
    ui: &egui::Ui,
    items: &[ListItem],
    sz: &Sizes,
    width: f32,
    theme: &Theme,
    scale: f32,
    pieces: &mut Vec<Piece>,
) {
    for item in items {
        let step = item.step;
        let indent = ITEM_INDENT * scale;
        let job = item_job(&item.inlines, sz.lead, 1.0, width - indent, theme);
        let galley = ui.painter().layout_job(job);
        pieces.push(Piece::item(galley, 16.0 * scale, step, indent, false));
        // One level of nesting: a step further in, a little smaller.
        if !item.children.is_empty() {
            let indent = 2.0 * ITEM_INDENT * scale;
            for child in &item.children {
                let job = item_job(&child.inlines, sz.lead * 0.9, 0.85, width - indent, theme);
                let galley = ui.painter().layout_job(job);
                pieces.push(Piece::item(galley, 12.0 * scale, child.step, indent, true));
            }
        }
    }
}

/// Lay out the whole copy stack for a bullet/content slide.
pub(super) fn content_pieces(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    sz: &Sizes,
    width: f32,
    scale: f32,
) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for block in &slide.blocks {
        match block {
            Block::Heading { level, inlines } => {
                let size = if *level <= 1 {
                    sz.h2 * 1.1
                } else if *level == 2 {
                    sz.h2
                } else {
                    sz.h2 * 0.7
                };
                let job = display_job(inlines, size, theme.heading_color, width, theme);
                pieces.push(Piece::plain(ui.painter().layout_job(job), 28.0 * scale));
            }
            Block::Paragraph { inlines } => {
                let job = lead_job(inlines, sz.lead, 1.0, width, theme);
                pieces.push(Piece::plain(ui.painter().layout_job(job), 22.0 * scale));
            }
            Block::List { items, .. } => {
                item_pieces(ui, items, sz, width, theme, scale, &mut pieces);
                if let Some(last) = pieces.last_mut() {
                    last.gap = 26.0 * scale;
                }
            }
            Block::BlockQuote { blocks } | Block::Callout { blocks, .. } => {
                let inlines = Block::quote_inlines(blocks);
                let job = lead_job(&inlines, sz.lead, 1.0, width - 30.0 * scale, theme);
                pieces.push(Piece {
                    bar: true,
                    ..Piece::plain(ui.painter().layout_job(job), 22.0 * scale)
                });
            }
            Block::HorizontalRule => {}
            _ => {}
        }
    }
    pieces
}

/// Draw a stack of pieces from `top` at `left`, applying entry stagger and
/// reveal fades. The first piece is the `start_nth` element of the stagger.
pub(super) fn draw_pieces(f: &Frame, pieces: &[Piece], left: f32, top: f32, start_nth: usize) {
    let (theme, scale) = (f.theme, f.scale);
    let painter = f.ui.painter();
    let mut y = top;
    let mut nth = start_nth;
    let reveal_age = f.reveal_timestamp.map(|t| t.elapsed().as_secs_f32());
    for piece in pieces {
        if piece.step > f.reveal_step {
            continue;
        }
        let progress = piece_progress(piece.step, f.reveal_step, f.age, reveal_age, nth);
        if progress < 1.0 {
            f.ui.ctx().request_repaint();
        }
        let a = f.opacity * progress;
        let rise = (1.0 - progress) * 14.0 * scale;
        let pos = Pos2::new(left + piece.indent, y + rise);
        crate::render::math::galley_faded(painter, pos, piece.galley.clone(), a);
        if piece.dot {
            let first_line_h = piece
                .galley
                .rows
                .first()
                .map(|r| r.rect().height())
                .unwrap_or(20.0);
            let cy = pos.y + first_line_h * 0.55;
            let r = if piece.nested { 2.4 } else { 3.2 } * scale;
            painter.circle_filled(
                Pos2::new(pos.x - 18.0 * scale, cy),
                r,
                fade(theme.accent, a),
            );
        }
        if piece.bar {
            let h = piece.galley.rect.height();
            painter.line_segment(
                [
                    Pos2::new(left - 16.0 * scale, pos.y + 2.0),
                    Pos2::new(left - 16.0 * scale, pos.y + h - 2.0),
                ],
                egui::Stroke::new(1.0, fade(theme.secondary, a)),
            );
        }
        y += piece.galley.rect.height() + piece.gap;
        nth += 1;
    }
}

/// How far a copy element has risen into place (0..1). Elements rise in a
/// stagger when the slide is entered; the element of a step just revealed
/// with Next rises on its own, whenever that happens. `reveal_age` is the
/// time since that Next press: the app clears it on Back, so stepping back
/// only removes the last element and nothing that stays animates again.
fn piece_progress(
    step: usize,
    reveal_step: usize,
    age: f32,
    reveal_age: Option<f32>,
    nth: usize,
) -> f32 {
    match reveal_age {
        Some(r) if step > 0 && step == reveal_step && r < 1.0 => ease_out(r / 0.55),
        _ => stagger(age, nth),
    }
}

fn stack_height(pieces: &[Piece], reveal_step: usize) -> f32 {
    let mut h = 0.0;
    let mut last_gap = 0.0;
    for p in pieces.iter().filter(|p| p.step <= reveal_step) {
        h += p.galley.rect.height() + p.gap;
        last_gap = p.gap;
    }
    (h - last_gap).max(0.0)
}

pub(super) fn full_height(pieces: &[Piece]) -> f32 {
    stack_height(pieces, usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_bullet_revealed_with_next_rises() {
        // Long after the entrance: the bullet just revealed rises...
        assert!(piece_progress(3, 3, 20.0, Some(0.1), 4) < 0.5);
        // ...even after nine seconds on the slide (it used to pop in then)
        assert!(piece_progress(3, 3, 9.5, Some(0.1), 4) < 0.5);
        // the bullets above it stay put
        assert_eq!(piece_progress(2, 3, 20.0, Some(0.1), 3), 1.0);
        // Regression: after Back the timestamp is cleared, so the bullet
        // that is now the last one stays where it is instead of rising again
        assert_eq!(piece_progress(2, 2, 20.0, None, 3), 1.0);
    }
}
