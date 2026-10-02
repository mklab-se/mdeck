//! Tables: columns sized to their content, a header band and zebra rows.

use eframe::egui::{self, Pos2, Stroke};

use super::inline::{inlines_to_job, measure_inlines};
use crate::parser::{Align, Inline};
use crate::render::TextCx;
use crate::theme::Theme;

/// Resolved geometry for a table: font size and per-column widths.
struct TableLayout {
    font_size: f32,
    /// Width of each column, including the inter-column gap (`cell_padding`).
    col_widths: Vec<f32>,
    cell_padding: f32,
}

impl TableLayout {
    /// Text width available inside column `col`.
    fn text_width(&self, col: usize) -> f32 {
        self.col_widths[col] - self.cell_padding
    }

    /// Left edge of column `col` relative to the table's left edge.
    fn col_offset(&self, col: usize) -> f32 {
        self.cell_padding + self.col_widths[..col].iter().sum::<f32>()
    }
}

/// Compute column widths from content. Columns get their natural (unwrapped)
/// width, capped so one long column cannot starve the others; the font shrinks
/// (down to 0.7x body) when the natural width exceeds `max_width`, and any
/// remaining space is distributed proportionally.
fn layout_table(
    ui: &egui::Ui,
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> TableLayout {
    let cell_padding = 12.0 * scale;
    let num_cols = headers.len().max(1);
    let available = (max_width - cell_padding * 2.0).max(cell_padding * num_cols as f32);
    let max_col_width = available * 0.6;

    let natural_widths = |font_size: f32| -> Vec<f32> {
        let mut widths = vec![cell_padding; num_cols];
        let all_rows = std::iter::once(headers).chain(rows.iter().map(Vec::as_slice));
        for row in all_rows {
            for (col, cell) in row.iter().enumerate().take(num_cols) {
                let job = inlines_to_job(cell, font_size, theme.foreground, f32::INFINITY, theme);
                let w = ui.painter().layout_job(job).rect.width() + cell_padding;
                widths[col] = widths[col].max(w).min(max_col_width);
            }
        }
        widths
    };

    let base_font = theme.body_size * 0.85 * scale;
    let min_font = theme.body_size * 0.7 * scale;

    let mut font_size = base_font;
    let mut widths = natural_widths(font_size);
    let mut total: f32 = widths.iter().sum();
    if total > available {
        font_size = (font_size * available / total).max(min_font);
        widths = natural_widths(font_size);
        total = widths.iter().sum();
    }

    // Scale to fit (wrapping if still too wide) or hand out the slack.
    let factor = available / total;
    for w in &mut widths {
        *w *= factor;
    }

    TableLayout {
        font_size,
        col_widths: widths,
        cell_padding,
    }
}

/// Height of the tallest cell in a row (cells beyond the header count are ignored).
fn table_row_height(
    ui: &egui::Ui,
    cells: &[Vec<Inline>],
    layout: &TableLayout,
    theme: &Theme,
) -> f32 {
    cells
        .iter()
        .enumerate()
        .take(layout.col_widths.len())
        .map(|(col, cell)| {
            measure_inlines(ui, cell, layout.font_size, layout.text_width(col), theme)
        })
        .fold(0.0, f32::max)
}

/// Total table height for a given layout (shared by draw and measure).
fn table_height(
    ui: &egui::Ui,
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    layout: &TableLayout,
    theme: &Theme,
    scale: f32,
) -> f32 {
    let pad = layout.cell_padding;
    let mut h = table_row_height(ui, headers, layout, theme) + pad * 2.0;
    h += TABLE_ROW_SPACING * scale;
    for row in rows {
        h += table_row_height(ui, row, layout, theme) + pad * 1.5;
    }
    h
}

const TABLE_ROW_SPACING: f32 = 4.0;

/// Measure a table exactly as [`draw_table`] lays it out.
pub fn measure_table_height(
    ui: &egui::Ui,
    headers: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    theme: &Theme,
    max_width: f32,
    scale: f32,
) -> f32 {
    let layout = layout_table(ui, headers, rows, theme, max_width, scale);
    table_height(ui, headers, rows, &layout, theme, scale)
}

/// Draw a cell's text in its column, aligned as the column asks.
fn draw_cell(
    cx: &TextCx,
    cell: &[Inline],
    pos: Pos2,
    font_size: f32,
    color: egui::Color32,
    width: f32,
    align: Align,
) {
    let job = inlines_to_job(cell, font_size, color, width, cx.theme);
    let galley = cx.ui.painter().layout_job(job);
    let slack = (width - galley.rect.width()).max(0.0);
    let dx = match align {
        Align::Left => 0.0,
        Align::Center => slack / 2.0,
        Align::Right => slack,
    };
    crate::render::math::galley(cx.ui.painter(), pos + egui::vec2(dx, 0.0), galley, color);
}

/// Draw a table. Returns height used.
pub fn draw_table(
    cx: &TextCx,
    headers: &[Vec<Inline>],
    align: &[Align],
    rows: &[Vec<Vec<Inline>>],
    pos: Pos2,
    max_width: f32,
) -> f32 {
    let col_align = |col: usize| align.get(col).copied().unwrap_or_default();
    let (ui, theme, opacity, scale) = (cx.ui, cx.theme, cx.opacity, cx.scale);
    let color = Theme::with_opacity(theme.foreground, opacity);
    let heading_color = Theme::with_opacity(theme.heading_color, opacity);
    let accent = Theme::with_opacity(theme.accent, opacity);
    let header_bg = Theme::with_opacity(theme.accent, opacity * 0.12);
    let zebra = Theme::with_opacity(theme.foreground, opacity * 0.04);

    let layout = layout_table(ui, headers, rows, theme, max_width, scale);
    let pad = layout.cell_padding;
    let num_cols = layout.col_widths.len();
    let rounding = 6.0 * scale;
    let band_left = pos.x + pad * 0.5;
    let band_right = pos.x + max_width - pad * 0.5;

    let mut y = pos.y;

    // Header row: subtle accent band behind the header text
    let header_h = table_row_height(ui, headers, &layout, theme);
    let header_rect = egui::Rect::from_min_max(
        Pos2::new(band_left, y),
        Pos2::new(band_right, y + header_h + pad * 2.0),
    );
    ui.painter().rect_filled(header_rect, rounding, header_bg);
    for (col, header) in headers.iter().enumerate().take(num_cols) {
        let cell_pos = Pos2::new(pos.x + layout.col_offset(col), y + pad);
        draw_cell(
            cx,
            header,
            cell_pos,
            layout.font_size,
            heading_color,
            layout.text_width(col),
            col_align(col),
        );
    }
    y += header_h + pad * 2.0;

    // Separator line
    let row_spacing = TABLE_ROW_SPACING * scale;
    let line_y = y + row_spacing / 2.0;
    ui.painter().line_segment(
        [
            Pos2::new(pos.x + pad, line_y),
            Pos2::new(pos.x + max_width - pad, line_y),
        ],
        Stroke::new(1.0 * scale, accent),
    );
    y += row_spacing;

    // Data rows with a faint zebra stripe on every other row
    for (row_idx, row) in rows.iter().enumerate() {
        let row_h = table_row_height(ui, row, &layout, theme);
        let band_h = row_h + pad * 1.5;
        if row_idx % 2 == 1 {
            let band = egui::Rect::from_min_max(
                Pos2::new(band_left, y),
                Pos2::new(band_right, y + band_h),
            );
            ui.painter().rect_filled(band, rounding, zebra);
        }
        for (col, cell) in row.iter().enumerate().take(num_cols) {
            let cell_pos = Pos2::new(pos.x + layout.col_offset(col), y + pad * 0.75);
            draw_cell(
                cx,
                cell,
                cell_pos,
                layout.font_size,
                color,
                layout.text_width(col),
                col_align(col),
            );
        }
        y += band_h;
    }

    y - pos.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::test_support::with_ui;

    const LONG: &str = "This is a deliberately long list item whose text is guaranteed to wrap \
        onto several rows when laid out inside a narrow column, so that the measured height \
        depends on real galley layout rather than a per-item constant.";

    fn text(s: &str) -> Vec<Inline> {
        vec![Inline::Text(s.to_string())]
    }

    #[test]
    fn table_columns_size_to_content_and_never_exceed_width() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let headers = vec![text("Id"), text("Description")];
            let rows = vec![vec![text("1"), text(LONG)], vec![text("2"), text("x")]];
            let layout = layout_table(ui, &headers, &rows, &theme, 1000.0, 1.0);
            assert!(layout.col_widths[1] > layout.col_widths[0]);
            let total: f32 = layout.col_widths.iter().sum::<f32>() + layout.cell_padding * 2.0;
            assert!((total - 1000.0).abs() < 0.5, "columns fill width: {total}");
        });
    }

    #[test]
    fn wide_table_shrinks_font_and_clamps_extra_cells() {
        with_ui(|ui| {
            let theme = Theme::dark();
            let headers: Vec<_> = (0..8).map(|i| text(&format!("Column {i}"))).collect();
            // A row with more cells than headers must not affect the width.
            let rows = vec![(0..12).map(|i| text(&format!("value {i}"))).collect()];
            let layout = layout_table(ui, &headers, &rows, &theme, 900.0, 1.0);
            assert_eq!(layout.col_widths.len(), 8);
            assert!(layout.font_size < theme.body_size * 0.85);
            assert!(layout.font_size >= theme.body_size * 0.7 - 0.01);
            let total: f32 = layout.col_widths.iter().sum::<f32>() + layout.cell_padding * 2.0;
            assert!(total <= 900.5, "{total}");
        });
    }
}
