//! Lists: markers, nesting and incremental reveal.

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

/// The reveal step an item with `marker` appears on. `counter` is the last
/// step handed out: `+` items take the next one, `*` items share it and
/// plain items always show.
pub(crate) fn item_step(marker: ListMarker, counter: &mut usize) -> usize {
    match marker {
        ListMarker::Static | ListMarker::Ordered => 0,
        ListMarker::NextStep => {
            *counter += 1;
            *counter
        }
        ListMarker::WithPrev => *counter,
    }
}

/// The marker drawn before the item at `idx`: its number in an ordered
/// list, otherwise a bullet.
fn marker_text(ordered: bool, marker: ListMarker, idx: usize) -> String {
    if ordered || marker == ListMarker::Ordered {
        format!("{}.", idx + 1)
    } else {
        "\u{2022}".to_string()
    }
}

/// Where a list stands in its reveal while it is drawn.
struct Reveal {
    /// Reveal steps shown so far.
    shown: usize,
    /// The last step handed out to an item (see [`item_step`]).
    counter: usize,
}

/// Draw a list with incremental reveal support. Returns height used.
pub fn draw_list(
    cx: &TextCx,
    items: &[ListItem],
    ordered: bool,
    pos: Pos2,
    max_width: f32,
    reveal_step: usize,
) -> f32 {
    let mut reveal = Reveal {
        shown: reveal_step,
        counter: 0,
    };
    draw_items(cx, items, ordered, pos, max_width, 0, &mut reveal)
}

fn draw_items(
    cx: &TextCx,
    items: &[ListItem],
    ordered: bool,
    pos: Pos2,
    max_width: f32,
    indent_level: usize,
    reveal: &mut Reveal,
) -> f32 {
    let (indent, marker_width, text_width) = list_metrics(max_width, indent_level, cx.scale);
    let item_spacing = LIST_ITEM_SPACING * cx.scale;
    let mut y_offset = 0.0;

    for (idx, item) in items.iter().enumerate() {
        // Skip items not yet revealed
        if item_step(item.marker, &mut reveal.counter) > reveal.shown {
            continue;
        }

        let marker = marker_text(ordered, item.marker, idx);
        let row = Pos2::new(pos.x + indent, pos.y + y_offset);
        let text_height = draw_item(cx, &item.inlines, &marker, row, marker_width, text_width);
        y_offset += text_height + item_spacing;

        // Draw children
        if !item.children.is_empty() {
            let children_ordered = item
                .children
                .first()
                .is_some_and(|c| c.marker == ListMarker::Ordered);
            y_offset += draw_items(
                cx,
                &item.children,
                children_ordered,
                Pos2::new(pos.x, pos.y + y_offset),
                max_width,
                indent_level + 1,
                reveal,
            );
        }
    }

    y_offset
}

/// Draw one item's marker at `pos` and its text beside it. Returns the text
/// height.
fn draw_item(
    cx: &TextCx,
    inlines: &[crate::parser::Inline],
    marker: &str,
    pos: Pos2,
    marker_width: f32,
    text_width: f32,
) -> f32 {
    let color = Theme::with_opacity(cx.theme.foreground, cx.opacity);
    let font_size = cx.theme.body_size * cx.scale;
    let painter = cx.ui.painter();
    let marker_galley = painter.layout_no_wrap(
        marker.to_string(),
        FontId::new(font_size, cx.theme.body_family()),
        color,
    );
    let job = inlines_to_job(inlines, font_size, color, text_width, cx.theme);
    let text_galley = painter.layout_job(job);
    // Sit the marker on the first line's baseline, which a tall inline
    // formula pushes down.
    let marker_dy = crate::render::math::first_baseline(&text_galley)
        .zip(crate::render::math::first_baseline(&marker_galley))
        .map_or(0.0, |(t, m)| (t - m).max(0.0));
    let marker_pos = Pos2::new(pos.x, pos.y + marker_dy);
    crate::render::math::galley(painter, marker_pos, marker_galley, color);

    let text_pos = Pos2::new(pos.x + marker_width, pos.y);
    let text_height = text_galley.rect.height();
    crate::render::math::galley(painter, text_pos, text_galley, color);
    text_height
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
    fn item_steps_follow_the_markers() {
        let mut counter = 0;
        let steps: Vec<usize> = [
            ListMarker::Static,
            ListMarker::NextStep,
            ListMarker::WithPrev,
            ListMarker::NextStep,
            ListMarker::Ordered,
        ]
        .into_iter()
        .map(|m| item_step(m, &mut counter))
        .collect();
        assert_eq!(steps, [0, 1, 1, 2, 0]);
        assert_eq!(counter, 2);
    }

    #[test]
    fn markers_number_ordered_items_and_bullet_the_rest() {
        assert_eq!(marker_text(true, ListMarker::Static, 2), "3.");
        assert_eq!(marker_text(false, ListMarker::Ordered, 0), "1.");
        assert_eq!(marker_text(false, ListMarker::NextStep, 4), "\u{2022}");
        assert_eq!(marker_text(false, ListMarker::WithPrev, 4), "\u{2022}");
    }
}
