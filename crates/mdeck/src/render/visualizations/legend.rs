//! Legends: a colour swatch and a label per series or slice.

use eframe::egui::{self, Color32, FontId, Pos2};

use super::{
    VIZ_CORNER_SWATCH, VIZ_FONT_LEGEND, VIZ_FONT_MIN, VIZ_SWATCH_SIZE, VizCtx, fit_font_size,
    fit_text,
};

/// One row of a vertical legend.
pub struct LegendItem {
    pub label: String,
    /// Text that must survive truncation, e.g. `" (43%)"`; appended after the label.
    pub suffix: String,
    pub color: Color32,
    /// Hidden rows keep their slot so earlier rows do not shift during a reveal.
    pub visible: bool,
}

/// Row height range for `draw_legend_column` (multiplied by scale).
const LEGEND_ROW_MAX: f32 = 48.0;
const LEGEND_ROW_MIN: f32 = 30.0;

/// Draw a vertical legend (colour swatch + label per row), centred vertically
/// in the `height` available. Rows shrink towards a minimum height, then spill
/// into a second column; whatever still does not fit is clipped. Labels are
/// shrunk/truncated to the column width so they never overflow the slide.
pub fn draw_legend_column(
    cx: &VizCtx,
    items: &[LegendItem],
    left: f32,
    top: f32,
    width: f32,
    height: f32,
) {
    let VizCtx {
        theme,
        opacity,
        scale,
        ..
    } = *cx;
    let painter = cx.ui.painter();
    if items.is_empty() || width <= 0.0 || height <= 0.0 {
        return;
    }
    let n = items.len();
    let row_min = LEGEND_ROW_MIN * scale;
    let rows_fit = ((height / row_min).floor() as usize).max(1);
    let columns = if n <= rows_fit { 1 } else { 2 };
    let rows_per_col = n.div_ceil(columns).min(rows_fit);
    let row_h = (height / rows_per_col as f32).clamp(row_min, LEGEND_ROW_MAX * scale);
    let col_w = width / columns as f32;
    let start_y = top + (height - rows_per_col as f32 * row_h) / 2.0;

    let swatch = VIZ_SWATCH_SIZE * scale;
    let gap = 10.0 * scale;
    let min_font = theme.body_size * VIZ_FONT_MIN * scale;
    let text_color = crate::theme::Theme::with_opacity(theme.foreground, opacity);
    let text_max_w = col_w - swatch - gap - 6.0 * scale;

    // One font size for the whole legend, chosen so as many entries as
    // possible fit; the rest are truncated (keeping their suffix).
    let full_texts: Vec<String> = items
        .iter()
        .map(|item| format!("{}{}", item.label, item.suffix))
        .collect();
    let refs: Vec<&str> = full_texts.iter().map(String::as_str).collect();
    let base_font = FontId::new(
        theme.body_size * VIZ_FONT_LEGEND * scale,
        theme.body_family(),
    );
    let font = FontId::new(
        fit_font_size(painter, &refs, &base_font, text_max_w, min_font),
        theme.body_family(),
    );

    for (i, item) in items.iter().enumerate() {
        let col = i / rows_per_col;
        if col >= columns {
            break; // clipped: no room left
        }
        if !item.visible {
            continue;
        }
        let row = i % rows_per_col;
        let x = left + col as f32 * col_w;
        let y = start_y + row as f32 * row_h;

        let swatch_rect = egui::Rect::from_min_size(
            Pos2::new(x, y + (row_h - swatch) / 2.0),
            egui::vec2(swatch, swatch),
        );
        painter.rect_filled(swatch_rect, VIZ_CORNER_SWATCH * scale, item.color);

        let full = painter.layout_no_wrap(full_texts[i].clone(), font.clone(), text_color);
        let galley = if full.rect.width() <= text_max_w {
            full
        } else {
            // Truncate only the label; the suffix (e.g. the percentage) stays.
            let suffix_w = painter
                .layout_no_wrap(item.suffix.clone(), font.clone(), text_color)
                .rect
                .width();
            let label = fit_text(
                painter,
                &item.label,
                font.clone(),
                text_color,
                (text_max_w - suffix_w).max(0.0),
                font.size,
            );
            painter.layout_no_wrap(
                format!("{}{}", label.text(), item.suffix),
                font.clone(),
                text_color,
            )
        };
        let text_y = y + (row_h - galley.rect.height()) / 2.0;
        painter.galley(Pos2::new(x + swatch + gap, text_y), galley, text_color);
    }
}

/// Draw a horizontal legend: one row of `(label, colour)` items centred in
/// the `width` from `left`, vertically centred in the band from `top`
/// `height` tall.
pub fn draw_legend_row(
    cx: &VizCtx,
    items: &[(String, Color32)],
    left: f32,
    width: f32,
    top: f32,
    height: f32,
) {
    if items.is_empty() {
        return;
    }
    let painter = cx.ui.painter();
    let scale = cx.scale;
    let legend_font = cx.font(VIZ_FONT_LEGEND);
    let swatch_size = VIZ_SWATCH_SIZE * scale;
    let item_spacing = 28.0 * scale;

    let mut total_w = 0.0f32;
    let galleys: Vec<_> = items
        .iter()
        .map(|(name, color)| {
            let g = painter.layout_no_wrap(name.clone(), legend_font.clone(), *color);
            total_w += swatch_size + 6.0 * scale + g.rect.width() + item_spacing;
            (g, *color)
        })
        .collect();
    total_w -= item_spacing; // no spacing after the last item

    let mut lx = left + (width - total_w) / 2.0;
    for (galley, color) in galleys {
        let swatch_rect = egui::Rect::from_min_size(
            Pos2::new(lx, top + (height - swatch_size) / 2.0),
            egui::vec2(swatch_size, swatch_size),
        );
        painter.rect_filled(swatch_rect, VIZ_CORNER_SWATCH * scale, color);
        lx += swatch_size + 6.0 * scale;

        let text_y = top + (height - galley.rect.height()) / 2.0;
        let w = galley.rect.width();
        painter.galley(Pos2::new(lx, text_y), galley, cx.fg(1.0));
        lx += w + item_spacing;
    }
}

/// Width of the legend column beside a pie/donut chart: a fixed width that
/// shrinks when the chart itself is narrow.
pub fn side_legend_width(max_width: f32, scale: f32) -> f32 {
    (380.0 * scale).min(max_width * 0.45)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_side_legend_width_shrinks_for_narrow_charts() {
        assert_eq!(side_legend_width(1800.0, 1.0), 380.0);
        assert_eq!(side_legend_width(600.0, 1.0), 270.0);
    }
}
