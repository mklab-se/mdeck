//! Lists: markers, nesting and incremental reveal.

use std::time::Instant;

use eframe::egui::{FontId, Pos2};

use super::inline::{inlines_to_job, measure_inlines};
use crate::parser::{ListItem, ListMarker};
use crate::render::TextCx;
use crate::theme::Theme;

const LIST_INDENT: f32 = 30.0;
const LIST_MARKER_WIDTH: f32 = 45.0;
const LIST_ITEM_SPACING: f32 = 8.0;
/// Deeply nested items never get less than this much room for their text.
const LIST_MIN_TEXT_WIDTH: f32 = 200.0;

/// Horizontal metrics for a list at a given nesting level:
/// `(indent, marker_width, text_width)`.
fn list_metrics(max_width: f32, indent_level: usize, scale: f32) -> (f32, f32, f32) {
    let indent = LIST_INDENT * scale * indent_level as f32;
    let marker_width = LIST_MARKER_WIDTH * scale;
    let text_width = (max_width - indent - marker_width).max(LIST_MIN_TEXT_WIDTH * scale);
    (indent, marker_width, text_width)
}

/// The marker drawn before the item at `idx`: its number in an ordered
/// list (counting from `start`), otherwise a bullet.
fn marker_text(ordered: bool, marker: ListMarker, start: u32, idx: usize) -> String {
    if ordered || marker == ListMarker::Ordered {
        format!("{}.", start as usize + idx)
    } else {
        "\u{2022}".to_string()
    }
}

/// How long a revealed item takes to settle.
const ITEM_REVEAL_SECONDS: f32 = 0.45;
/// How far a revealed item slides in from, at reference scale.
const ITEM_REVEAL_SHIFT: f32 = 24.0;

/// Where a list stands in its reveal: the steps shown so far and when the
/// latest one was shown (`None` draws it settled).
#[derive(Clone, Copy, Debug)]
pub struct Reveal {
    pub shown: usize,
    pub at: Option<Instant>,
}

impl Reveal {
    /// How far into its entrance an item at `step` is: 0 just revealed, 1
    /// settled. Asks for a repaint while it runs.
    fn progress(&self, ui: &eframe::egui::Ui, step: usize) -> f32 {
        if step == 0 || step != self.shown {
            return 1.0;
        }
        let Some(at) = self.at else {
            return 1.0;
        };
        let t = (at.elapsed().as_secs_f32() / ITEM_REVEAL_SECONDS).min(1.0);
        if t < 1.0 {
            ui.ctx().request_repaint();
        }
        // ease out cubic
        1.0 - (1.0 - t).powi(3)
    }
}

/// Draw a list. Items not yet revealed keep their place (nothing below
/// moves when they appear) and the newest ones slide and fade in. Returns
/// the height of the whole list, as [`measure_list_height`] gives it.
pub fn draw_list(
    cx: &TextCx,
    items: &[ListItem],
    ordered: bool,
    start: u32,
    pos: Pos2,
    max_width: f32,
    reveal: Reveal,
) -> f32 {
    draw_items(cx, items, (ordered, start), pos, max_width, 0, reveal)
}

fn draw_items(
    cx: &TextCx,
    items: &[ListItem],
    (ordered, start): (bool, u32),
    pos: Pos2,
    max_width: f32,
    indent_level: usize,
    reveal: Reveal,
) -> f32 {
    let (indent, marker_width, text_width) = list_metrics(max_width, indent_level, cx.scale);
    let item_spacing = LIST_ITEM_SPACING * cx.scale;
    let mut y_offset = 0.0;

    for (idx, item) in items.iter().enumerate() {
        let row = Pos2::new(pos.x + indent, pos.y + y_offset);
        let text_height = if item.step > reveal.shown {
            // Hidden: hold its space.
            let font_size = cx.theme.body_size * cx.scale;
            measure_inlines(cx.ui, &item.inlines, font_size, text_width, cx.theme)
        } else {
            let t = reveal.progress(cx.ui, item.step);
            let shifted = TextCx {
                opacity: cx.opacity * t,
                ..*cx
            };
            let row = row + eframe::egui::vec2((1.0 - t) * ITEM_REVEAL_SHIFT * cx.scale, 0.0);
            let marker = match item.checked {
                Some(checked) => Marker::Box(checked),
                None => Marker::Text(marker_text(ordered, item.marker, start, idx)),
            };
            draw_item(
                &shifted,
                &item.inlines,
                &marker,
                row,
                marker_width,
                text_width,
            )
        };
        y_offset += text_height + item_spacing;

        if !item.children.is_empty() {
            let children_ordered = item
                .children
                .first()
                .is_some_and(|c| c.marker == ListMarker::Ordered);
            y_offset += draw_items(
                cx,
                &item.children,
                (children_ordered, 1),
                Pos2::new(pos.x, pos.y + y_offset),
                max_width,
                indent_level + 1,
                reveal,
            );
        }
    }

    y_offset
}

/// What stands before an item's text.
enum Marker {
    Text(String),
    /// A task list box, checked or not.
    Box(bool),
}

/// Draw one item's marker at `pos` and its text beside it. Returns the text
/// height.
fn draw_item(
    cx: &TextCx,
    inlines: &[crate::parser::Inline],
    marker: &Marker,
    pos: Pos2,
    marker_width: f32,
    text_width: f32,
) -> f32 {
    let color = Theme::with_opacity(cx.theme.foreground, cx.opacity);
    let font_size = cx.theme.body_size * cx.scale;
    let painter = cx.ui.painter();
    let job = inlines_to_job(inlines, font_size, color, text_width, cx.theme);
    let text_galley = painter.layout_job(job);
    match marker {
        Marker::Text(text) => {
            let marker_galley = painter.layout_no_wrap(
                text.clone(),
                FontId::new(font_size, cx.theme.body_family()),
                color,
            );
            // Sit the marker on the first line's baseline, which a tall
            // inline formula pushes down.
            let marker_dy = crate::render::math::first_baseline(&text_galley)
                .zip(crate::render::math::first_baseline(&marker_galley))
                .map_or(0.0, |(t, m)| (t - m).max(0.0));
            let marker_pos = Pos2::new(pos.x, pos.y + marker_dy);
            crate::render::math::galley(painter, marker_pos, marker_galley, color);
        }
        Marker::Box(checked) => draw_task_box(cx, &text_galley, pos, font_size, *checked),
    }

    let text_pos = Pos2::new(pos.x + marker_width, pos.y);
    let text_height = text_galley.rect.height();
    crate::render::math::galley(painter, text_pos, text_galley, color);
    text_height
}

/// A task list box beside the first line of text, ticked when `checked`.
fn draw_task_box(
    cx: &TextCx,
    text: &eframe::egui::Galley,
    pos: Pos2,
    font_size: f32,
    checked: bool,
) {
    use eframe::egui::{Rect, Stroke, vec2};
    let side = font_size * 0.62;
    let first_row = text.rows.first().map_or(font_size, |r| r.rect().height());
    let top = pos.y + (first_row - side) / 2.0;
    let rect = Rect::from_min_size(Pos2::new(pos.x, top), vec2(side, side));
    let accent = Theme::with_opacity(cx.theme.accent, cx.opacity);
    let painter = cx.ui.painter();
    let rounding = side * 0.22;
    if checked {
        painter.rect_filled(rect, rounding, accent);
        let ink = Theme::with_opacity(cx.theme.background, cx.opacity);
        let stroke = Stroke::new((side * 0.14).max(1.0), ink);
        let p = |x: f32, y: f32| Pos2::new(rect.left() + x * side, rect.top() + y * side);
        painter.line_segment([p(0.22, 0.52), p(0.42, 0.72)], stroke);
        painter.line_segment([p(0.42, 0.72), p(0.78, 0.30)], stroke);
    } else {
        let stroke = Stroke::new((side * 0.1).max(1.0), accent);
        painter.rect_stroke(rect, rounding, stroke, eframe::egui::StrokeKind::Inside);
    }
}

/// Measure the height a list will occupy when fully revealed, using the same
/// wrapping and spacing as [`draw_list`].
pub fn measure_list_height(
    ui: &eframe::egui::Ui,
    items: &[ListItem],
    theme: &Theme,
    max_width: f32,
    indent_level: usize,
    scale: f32,
) -> f32 {
    let (_, _, text_width) = list_metrics(max_width, indent_level, scale);
    let item_spacing = LIST_ITEM_SPACING * scale;
    let font_size = theme.body_size * scale;
    let mut total = 0.0;

    for item in items {
        total += measure_inlines(ui, &item.inlines, font_size, text_width, theme) + item_spacing;
        if !item.children.is_empty() {
            total += measure_list_height(
                ui,
                &item.children,
                theme,
                max_width,
                indent_level + 1,
                scale,
            );
        }
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deeply_nested_list_keeps_minimum_text_width() {
        let (_, _, w) = list_metrics(300.0, 20, 1.0);
        assert_eq!(w, LIST_MIN_TEXT_WIDTH);
        let (_, _, w) = list_metrics(1000.0, 0, 1.0);
        assert_eq!(w, 1000.0 - LIST_MARKER_WIDTH);
    }

    #[test]
    fn markers_number_ordered_items_and_bullet_the_rest() {
        assert_eq!(marker_text(true, ListMarker::Static, 1, 2), "3.");
        assert_eq!(marker_text(false, ListMarker::Ordered, 1, 0), "1.");
        assert_eq!(
            marker_text(true, ListMarker::Ordered, 7, 1),
            "8.",
            "start number"
        );
        assert_eq!(marker_text(false, ListMarker::NextStep, 1, 4), "\u{2022}");
    }
}
