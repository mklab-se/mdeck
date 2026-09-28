use eframe::egui::{self, Color32, FontId, Pos2};

use crate::theme::Theme;

use super::{
    VIZ_CORNER_CARD, VIZ_FONT_MIN, VIZ_FONT_PRIMARY_LABEL, VIZ_FONT_SECONDARY_LABEL,
    VIZ_OPACITY_SUBTLE_BG, VizCtx, VizReveal, assign_steps, fit_text, parse_reveal_prefix,
};

/// Headline values shrink to fit the card, but never below this multiple of the
/// body size: a KPI that is not readable from the back of the room is pointless.
const KPI_VALUE_MIN_FONT: f32 = 0.75;

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct KpiEntry {
    label: String,
    value: String,
    trend: Option<String>,
    reveal: VizReveal,
}

fn parse_kpi_cards(content: &str) -> Vec<KpiEntry> {
    let mut entries = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (text, reveal) = parse_reveal_prefix(trimmed);
        if text.is_empty() {
            continue;
        }

        // Parse "Label: Value (trend: +12%)" or "Label: Value"
        if let Some(colon_pos) = text.find(": ") {
            let label = text[..colon_pos].trim().to_string();
            let rest = text[colon_pos + 2..].trim();

            // Check for trend in parentheses
            let (value, trend) = if let Some(paren_start) = rest.find("(trend:") {
                let value = rest[..paren_start].trim().to_string();
                let trend_part = &rest[paren_start..];
                let trend_text = trend_part
                    .trim_start_matches("(trend:")
                    .trim_end_matches(')')
                    .trim()
                    .to_string();
                (value, Some(trend_text))
            } else {
                (rest.to_string(), None)
            };

            entries.push(KpiEntry {
                label,
                value,
                trend,
                reveal,
            });
        }
    }

    entries
}

// ─── Layout ─────────────────────────────────────────────────────────────────

/// A centred row of equal cards.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CardRow {
    start_x: f32,
    card_width: f32,
    card_gap: f32,
}

impl CardRow {
    /// Cards as wide as `max_width` allows, capped so a few cards do not
    /// stretch across the slide.
    fn new(left: f32, max_width: f32, n: usize, scale: f32) -> Self {
        let card_gap = 24.0 * scale;
        let total_gaps = (n as f32 - 1.0).max(0.0) * card_gap;
        let card_width = ((max_width - total_gaps) / n as f32).min(320.0 * scale);
        let total_width = n as f32 * card_width + total_gaps;
        Self {
            start_x: left + (max_width - total_width) / 2.0,
            card_width,
            card_gap,
        }
    }

    fn card_x(&self, i: usize) -> f32 {
        self.start_x + i as f32 * (self.card_width + self.card_gap)
    }
}

/// Which way a trend points, from its sign.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Trend {
    Up,
    Down,
    Flat,
}

impl Trend {
    fn of(text: &str) -> Self {
        if text.starts_with('+') {
            Trend::Up
        } else if text.starts_with('-') {
            Trend::Down
        } else {
            Trend::Flat
        }
    }
}

/// The triangle of an up or down arrow `w` wide and `h` tall around `center`.
fn arrow_points(trend: Trend, center: Pos2, w: f32, h: f32) -> Vec<Pos2> {
    let (cx, cy) = (center.x, center.y);
    if trend == Trend::Up {
        vec![
            Pos2::new(cx, cy - h / 2.0),
            Pos2::new(cx - w / 2.0, cy + h / 2.0),
            Pos2::new(cx + w / 2.0, cy + h / 2.0),
        ]
    } else {
        vec![
            Pos2::new(cx - w / 2.0, cy - h / 2.0),
            Pos2::new(cx + w / 2.0, cy - h / 2.0),
            Pos2::new(cx, cy + h / 2.0),
        ]
    }
}

// ─── Renderer ───────────────────────────────────────────────────────────────

/// What every card shares: fonts and the size of its content.
struct CardStyle {
    value_font: FontId,
    label_font: FontId,
    trend_font: FontId,
    /// Height of the value row at full size.
    value_row: f32,
    /// Height of value, label and trend rows together.
    content_height: f32,
    card_width: f32,
    card_height: f32,
    text_max_w: f32,
}

pub fn draw_kpi_cards(
    cx: &VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let scale = cx.scale;
    let entries = parse_kpi_cards(content);
    if entries.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        300.0 * scale
    };

    let reveals: Vec<VizReveal> = entries.iter().map(|e| e.reveal).collect();
    let steps = assign_steps(&reveals);
    let painter = cx.ui.painter();

    let row = CardRow::new(pos.x, max_width, entries.len(), scale);
    let value_font = FontId::new(cx.theme.body_size * 2.0 * scale, cx.theme.body_family());
    let label_font = cx.font(VIZ_FONT_PRIMARY_LABEL);
    let trend_font = cx.font(VIZ_FONT_SECONDARY_LABEL);

    // Size cards to their content (value, label, optional trend) so the text
    // sits centered with even padding instead of floating in a tall box.
    let row_gap = 8.0 * scale;
    let measure = |text: &str, font: &FontId| {
        painter
            .layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE)
            .rect
            .height()
    };
    let value_row = measure("0", &value_font);
    let mut content_height = value_row + row_gap + measure("Ag", &label_font);
    if entries.iter().any(|e| e.trend.is_some()) {
        content_height += row_gap + measure("Ag", &trend_font);
    }
    let card_padding = 56.0 * scale;
    let style = CardStyle {
        value_font,
        label_font,
        trend_font,
        value_row,
        content_height,
        card_width: row.card_width,
        card_height: (content_height + card_padding * 2.0).min(height),
        text_max_w: row.card_width - 24.0 * scale,
    };
    let card_y = pos.y + (height - style.card_height) / 2.0;

    for (i, entry) in entries.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > cx.reveal_step {
            continue;
        }
        let anim = cx.anim(step);
        let card_pos = Pos2::new(row.card_x(i), card_y);
        draw_card(cx, entry, &style, card_pos, cx.opacity * anim);
    }

    height
}

/// One card with its top-left corner at `card_pos`: background, value,
/// label and trend, at `item_opacity`.
fn draw_card(cx: &VizCtx, entry: &KpiEntry, style: &CardStyle, card_pos: Pos2, item_opacity: f32) {
    let painter = cx.ui.painter();
    let theme = cx.theme;
    let scale = cx.scale;
    let (card_x, card_y) = (card_pos.x, card_pos.y);
    let card_width = style.card_width;

    // Card background
    let card_rect = egui::Rect::from_min_size(card_pos, egui::vec2(card_width, style.card_height));
    let bg_color = Theme::with_opacity(theme.foreground, item_opacity * VIZ_OPACITY_SUBTLE_BG);
    painter.rect_filled(card_rect, VIZ_CORNER_CARD * scale, bg_color);
    crate::render::hints::push(cx.ui.ctx(), crate::render::hints::Hint::Frame(card_rect));

    // Value text (centered, large, shrunk to fit the card)
    let text_color = Theme::with_opacity(theme.foreground, item_opacity);
    let value_galley = fit_text(
        painter,
        &entry.value,
        style.value_font.clone(),
        text_color,
        style.text_max_w,
        theme.body_size * KPI_VALUE_MIN_FONT * scale,
    );
    let value_x = card_x + (card_width - value_galley.rect.width()) / 2.0;
    let value_y = card_y
        + (style.card_height - style.content_height) / 2.0
        + (style.value_row - value_galley.rect.height()) / 2.0;
    painter.galley(
        Pos2::new(value_x, value_y),
        value_galley.clone(),
        text_color,
    );

    // Label text (centered, below value)
    let label_color = Theme::with_opacity(theme.foreground, item_opacity * 0.7);
    let label_galley = fit_text(
        painter,
        &entry.label,
        style.label_font.clone(),
        label_color,
        style.text_max_w,
        theme.body_size * VIZ_FONT_MIN * scale,
    );
    let label_x = card_x + (card_width - label_galley.rect.width()) / 2.0;
    let label_y = value_y + value_galley.rect.height() + 8.0 * scale;
    painter.galley(
        Pos2::new(label_x, label_y),
        label_galley.clone(),
        label_color,
    );

    // Trend indicator with arrow (centered, below label)
    if let Some(ref trend) = entry.trend {
        let trend_y = label_y + label_galley.rect.height() + 8.0 * scale;
        draw_trend(cx, trend, style, card_x, trend_y, item_opacity);
    }
}

/// A trend centred in the card at `trend_y`, with an arrow when it is signed.
fn draw_trend(
    cx: &VizCtx,
    trend: &str,
    style: &CardStyle,
    card_x: f32,
    trend_y: f32,
    item_opacity: f32,
) {
    let painter = cx.ui.painter();
    let card_width = style.card_width;
    let theme = cx.theme;
    let direction = Trend::of(trend);
    let trend_color = match direction {
        Trend::Up => Theme::with_opacity(theme.positive_color(), item_opacity),
        Trend::Down => Theme::with_opacity(theme.negative_color(), item_opacity),
        Trend::Flat => Theme::with_opacity(theme.foreground, item_opacity * 0.6),
    };

    let trend_galley =
        painter.layout_no_wrap(trend.to_string(), style.trend_font.clone(), trend_color);

    // Arrow size scales with the text height (so larger trends get larger arrows)
    let arrow_h = trend_galley.rect.height() * 0.7;
    let arrow_w = arrow_h * 0.8;
    let arrow_gap = 6.0 * cx.scale;

    if direction == Trend::Flat {
        let start_x = card_x + (card_width - trend_galley.rect.width()) / 2.0;
        painter.galley(Pos2::new(start_x, trend_y), trend_galley, trend_color);
        return;
    }

    let total_w = arrow_w + arrow_gap + trend_galley.rect.width();
    let start_x = card_x + (card_width - total_w) / 2.0;
    let center = Pos2::new(
        start_x + arrow_w / 2.0,
        trend_y + trend_galley.rect.height() / 2.0,
    );
    painter.add(egui::Shape::convex_polygon(
        arrow_points(direction, center, arrow_w, arrow_h),
        trend_color,
        egui::Stroke::NONE,
    ));
    // Text after arrow
    painter.galley(
        Pos2::new(start_x + arrow_w + arrow_gap, trend_y),
        trend_galley,
        trend_color,
    );
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kpi_basic() {
        let content = "- Revenue: $4.2M";
        let entries = parse_kpi_cards(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label, "Revenue");
        assert_eq!(entries[0].value, "$4.2M");
        assert!(entries[0].trend.is_none());
    }

    #[test]
    fn test_parse_kpi_with_trend() {
        let content = "- Revenue: $4.2M (trend: +12%)";
        let entries = parse_kpi_cards(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label, "Revenue");
        assert_eq!(entries[0].value, "$4.2M");
        assert_eq!(entries[0].trend, Some("+12%".to_string()));
    }

    #[test]
    fn test_parse_kpi_multiple() {
        let content = "- Revenue: $4.2M (trend: +12%)\n- Users: 1.2M (trend: +8%)\n+ Churn: 3.2% (trend: -0.5%)";
        let entries = parse_kpi_cards(content);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].reveal, VizReveal::Static);
        assert_eq!(entries[2].reveal, VizReveal::NextStep);
        assert_eq!(entries[2].trend, Some("-0.5%".to_string()));
    }

    #[test]
    fn test_parse_kpi_skips_invalid() {
        let content = "- Valid: $100\n- no_colon\n# comment\n- Also: $200";
        let entries = parse_kpi_cards(content);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_kpi_value_fits_card() {
        super::super::tests::with_test_painter(|painter| {
            let font = FontId::proportional(40.0);
            let g = fit_text(
                painter,
                "$1,234,567,890.00",
                font,
                Color32::WHITE,
                120.0,
                20.0,
            );
            assert!(g.rect.width() <= 120.0 + 0.01);
            assert!(g.text().ends_with('…'));
        });
    }

    #[test]
    fn test_card_row_centres_and_caps_width() {
        let row = CardRow::new(0.0, 1000.0, 2, 1.0);
        assert_eq!(row.card_width, 320.0);
        assert_eq!(row.start_x, (1000.0 - 664.0) / 2.0);
        assert_eq!(row.card_x(1), row.start_x + 344.0);
        let row = CardRow::new(10.0, 500.0, 4, 1.0);
        assert_eq!(row.card_width, (500.0 - 72.0) / 4.0);
        assert_eq!(row.start_x, 10.0);
    }

    #[test]
    fn test_trend_direction_and_arrow() {
        assert_eq!(Trend::of("+12%"), Trend::Up);
        assert_eq!(Trend::of("-0.5%"), Trend::Down);
        assert_eq!(Trend::of("flat"), Trend::Flat);
        let c = Pos2::new(10.0, 10.0);
        let up = arrow_points(Trend::Up, c, 4.0, 6.0);
        assert_eq!(up[0], Pos2::new(10.0, 7.0));
        let down = arrow_points(Trend::Down, c, 4.0, 6.0);
        assert_eq!(down[2], Pos2::new(10.0, 13.0));
    }
}
