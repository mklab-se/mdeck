use eframe::egui::{self, Color32, FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_CORNER_TRACK, VIZ_FONT_MIN, VIZ_FONT_PRIMARY_LABEL, VIZ_FONT_TITLE, VIZ_OPACITY_GRID,
    VIZ_OPACITY_LABEL, VIZ_STROKE_BORDER, VizReveal, assign_steps, fit_font_size, fit_text,
    grammar::{Problem, Source, label_value_items},
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ProgressEntry {
    label: String,
    value: f32,
    reveal: VizReveal,
}

fn read(src: &Source) -> Vec<ProgressEntry> {
    src.check_settings(&[]);
    label_value_items(src, "- Label: 40")
        .into_iter()
        .map(|e| ProgressEntry {
            label: e.label,
            value: e.value.clamp(0.0, 100.0),
            reveal: e.reveal,
        })
        .collect()
}

fn parse_progress_bars(content: &str) -> Vec<ProgressEntry> {
    read(&Source::parse(content))
}

/// The problems in a `@progress` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
}

// ─── Renderer ───────────────────────────────────────────────────────────────

/// Rows of label, bar and percentage, centred vertically.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RowLayout {
    label_width: f32,
    bar_left: f32,
    bar_width: f32,
    bar_height: f32,
    row_spacing: f32,
    start_y: f32,
}

impl RowLayout {
    /// Bars as tall as `n` rows allow within limits, so rows fill the space
    /// but keep breathing room; generous columns for reading from a distance.
    fn new(pos: Pos2, max_width: f32, height: f32, scale: f32, n: usize) -> Self {
        let padding = 30.0 * scale;
        let label_width = 200.0 * scale;
        let pct_width = 90.0 * scale;
        let available_height = height - padding * 2.0;
        let max_bar_height = 44.0 * scale;
        let min_bar_height = 28.0 * scale;
        let row_spacing = 20.0 * scale;
        let bar_height = ((available_height - (n as f32 - 1.0) * row_spacing) / n as f32)
            .clamp(min_bar_height, max_bar_height);
        let total_rows_height = n as f32 * (bar_height + row_spacing) - row_spacing;
        Self {
            label_width,
            bar_left: pos.x + padding + label_width + 12.0 * scale,
            bar_width: max_width - padding * 2.0 - label_width - 12.0 * scale - pct_width,
            bar_height,
            row_spacing,
            start_y: pos.y + (height - total_rows_height) / 2.0,
        }
    }

    fn row_y(&self, i: usize) -> f32 {
        self.start_y + i as f32 * (self.bar_height + self.row_spacing)
    }

    /// Row `i`'s full track.
    fn track_rect(&self, i: usize) -> egui::Rect {
        egui::Rect::from_min_size(
            Pos2::new(self.bar_left, self.row_y(i)),
            egui::vec2(self.bar_width, self.bar_height),
        )
    }

    /// Width of row `i`'s fill for `value` percent, grown to `anim`.
    fn fill_width(&self, value: f32, anim: f32) -> f32 {
        let fill_frac = (value / 100.0) * anim;
        self.bar_width * fill_frac
    }
}

pub fn draw_progress_bars(
    cx: &super::VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let super::VizCtx {
        ui,
        theme,
        scale,
        reveal_step,
        ..
    } = *cx;
    let entries = parse_progress_bars(content);
    if entries.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    let reveals: Vec<VizReveal> = entries.iter().map(|e| e.reveal).collect();
    let steps = assign_steps(&reveals);
    let palette = theme.edge_palette();
    let painter = ui.painter();

    let n = entries.len();
    let label_font = FontId::new(
        theme.body_size * VIZ_FONT_TITLE * scale,
        theme.body_family(),
    );
    let pct_font = FontId::new(
        theme.body_size * VIZ_FONT_PRIMARY_LABEL * scale,
        theme.body_family(),
    );

    let rows = RowLayout::new(pos, max_width, height, scale, n);

    // All labels share one font size so rows read as a unit
    let label_texts: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
    let label_font = FontId::new(
        fit_font_size(
            painter,
            &label_texts,
            &label_font,
            rows.label_width,
            theme.body_size * VIZ_FONT_MIN * scale,
        ),
        theme.body_family(),
    );

    let bars = BarRows {
        rows,
        label_font,
        pct_font,
        palette: &palette,
    };
    for (i, entry) in entries.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }
        let anim = cx.anim(step);
        bars.draw_label(cx, i, &entry.label);
        bars.draw_bar(cx, i, entry.value, anim);
    }

    height
}

/// Paints the rows of a `RowLayout`.
struct BarRows<'a> {
    rows: RowLayout,
    label_font: FontId,
    pct_font: FontId,
    palette: &'a [Color32],
}

impl BarRows<'_> {
    /// Row `i`'s label on the left, fitted to the label column.
    fn draw_label(&self, cx: &super::VizCtx, i: usize, label: &str) {
        let super::VizCtx {
            theme,
            opacity,
            scale,
            ..
        } = *cx;
        let painter = cx.ui.painter();
        let RowLayout {
            label_width,
            bar_left,
            bar_height,
            ..
        } = self.rows;
        let row_y = self.rows.row_y(i);

        let label_color = Theme::with_opacity(theme.foreground, opacity * 0.9);
        let galley = fit_text(
            painter,
            label,
            self.label_font.clone(),
            label_color,
            label_width,
            self.label_font.size,
        );
        let label_y = row_y + (bar_height - galley.rect.height()) / 2.0;
        let label_x = bar_left - 12.0 * scale - galley.rect.width();
        painter.galley(Pos2::new(label_x, label_y), galley, label_color);
    }

    /// Row `i`'s track, its fill grown to `anim`, and the percentage on the
    /// right.
    fn draw_bar(&self, cx: &super::VizCtx, i: usize, value: f32, anim: f32) {
        let super::VizCtx {
            ui,
            theme,
            opacity,
            scale,
            ..
        } = *cx;
        let painter = ui.painter();
        let RowLayout {
            bar_left,
            bar_width,
            bar_height,
            ..
        } = self.rows;
        let row_y = self.rows.row_y(i);

        // Track background
        let track_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_GRID);
        let track_rect = self.rows.track_rect(i);
        painter.rect_filled(track_rect, VIZ_CORNER_TRACK * scale, track_color);

        // Fill bar
        let color = self.palette[i % self.palette.len()];
        let fill_color = Theme::with_opacity(color, opacity * theme.fill_opacity());
        let fill_width = self.rows.fill_width(value, anim);
        if fill_width > 0.0 {
            let fill_rect = egui::Rect::from_min_size(
                Pos2::new(bar_left, row_y),
                egui::vec2(fill_width, bar_height),
            );
            painter.rect_filled(fill_rect, VIZ_CORNER_TRACK * scale, fill_color);
            crate::render::hints::push(ui.ctx(), crate::render::hints::Hint::Bar(fill_rect));
        }

        // Subtle border on track
        let border_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_GRID);
        painter.rect_stroke(
            track_rect,
            VIZ_CORNER_TRACK * scale,
            Stroke::new(VIZ_STROKE_BORDER * scale, border_color),
            egui::StrokeKind::Outside,
        );

        // Percentage on the right
        let pct_text = format!("{:.0}%", value);
        let pct_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_LABEL);
        let pct_galley = painter.layout_no_wrap(pct_text, self.pct_font.clone(), pct_color);
        let pct_y = row_y + (bar_height - pct_galley.rect.height()) / 2.0;
        let pct_x = bar_left + bar_width + 12.0 * scale;
        painter.galley(Pos2::new(pct_x, pct_y), pct_galley, pct_color);
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_progress_bars_basic() {
        let content = "- Design: 100%\n- Frontend: 75%";
        let entries = parse_progress_bars(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "Design");
        assert_eq!(entries[0].value, 100.0);
        assert_eq!(entries[1].label, "Frontend");
        assert_eq!(entries[1].value, 75.0);
    }

    #[test]
    fn test_parse_progress_bars_without_percent() {
        let content = "- A: 50\n- B: 80";
        let entries = parse_progress_bars(content);
        assert_eq!(entries[0].value, 50.0);
        assert_eq!(entries[1].value, 80.0);
    }

    #[test]
    fn test_parse_progress_bars_clamped() {
        let content = "- Over: 150%\n- Under: -10";
        let entries = parse_progress_bars(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].value, 100.0); // clamped to max
        assert_eq!(entries[1].value, 0.0); // clamped to min
    }

    #[test]
    fn test_parse_progress_bars_reveal_markers() {
        let content = "- A: 100%\n+ B: 75%\n* C: 50%";
        let entries = parse_progress_bars(content);
        assert_eq!(entries[0].reveal, VizReveal::Static);
        assert_eq!(entries[1].reveal, VizReveal::NextStep);
        assert_eq!(entries[2].reveal, VizReveal::WithPrev);
    }

    #[test]
    fn test_parse_progress_bars_skips_invalid() {
        let content = "- Valid: 50%\n- no_value\n# comment\n- Also: 30%";
        let entries = parse_progress_bars(content);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_parse_progress_bars_rejects_non_finite() {
        let entries = parse_progress_bars("- A: inf\n- B: nan\n- C: 1_0");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, 10.0);
    }

    #[test]
    fn test_row_layout_clamps_and_centres() {
        let few = RowLayout::new(Pos2::new(0.0, 0.0), 1000.0, 500.0, 1.0, 2);
        assert_eq!(few.bar_height, 44.0);
        assert_eq!(few.start_y, (500.0 - 108.0) / 2.0);
        assert_eq!(few.row_y(1), few.start_y + 64.0);
        assert_eq!(few.bar_left, 242.0);
        assert_eq!(few.bar_width, 1000.0 - 60.0 - 200.0 - 12.0 - 90.0);
        let many = RowLayout::new(Pos2::new(0.0, 0.0), 1000.0, 500.0, 1.0, 30);
        assert_eq!(many.bar_height, 28.0);
    }

    #[test]
    fn test_row_layout_track_and_fill() {
        let rows = RowLayout::new(Pos2::new(0.0, 0.0), 1000.0, 500.0, 1.0, 2);
        let track = rows.track_rect(1);
        assert_eq!(track.min, Pos2::new(rows.bar_left, rows.row_y(1)));
        assert_eq!(track.width(), rows.bar_width);
        assert_eq!(track.height(), rows.bar_height);
        assert_eq!(rows.fill_width(50.0, 1.0), rows.bar_width * 0.5);
        assert_eq!(rows.fill_width(50.0, 0.5), rows.bar_width * 0.25);
        assert_eq!(rows.fill_width(80.0, 0.0), 0.0);
    }
}
