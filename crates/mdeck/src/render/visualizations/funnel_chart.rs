use eframe::egui::{self, FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_CORNER_NODE, VIZ_FONT_MIN, VIZ_FONT_SECONDARY_LABEL, VIZ_FONT_TITLE,
    VIZ_LABEL_REVEAL_THRESHOLD, VizReveal, assign_steps, fit_text, format_value, label_fade,
    parse_label_value, parse_reveal_prefix, reveal_anim_progress,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct FunnelEntry {
    label: String,
    value: f32,
    reveal: VizReveal,
}

fn parse_funnel_chart(content: &str) -> Vec<FunnelEntry> {
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

        // Parse "Label: 10000"; a negative stage count is meaningless → 0
        if let Some((label, value)) = parse_label_value(text) {
            entries.push(FunnelEntry {
                label,
                value: value.max(0.0),
                reveal,
            });
        }
    }
    entries
}

// ─── Renderer ───────────────────────────────────────────────────────────────

/// A trapezoid with its top edge centred on `top_center`, `half_widths`
/// (top, bottom) either side, `h` tall, its corners cut by `corner_r`.
fn trapezoid(top_center: Pos2, half_widths: (f32, f32), h: f32, corner_r: f32) -> Vec<Pos2> {
    let (center_x, top_y) = (top_center.x, top_center.y);
    let (half_top, half_bot) = half_widths;
    vec![
        // Top edge
        Pos2::new(center_x - half_top + corner_r, top_y),
        Pos2::new(center_x + half_top - corner_r, top_y),
        // Right side slopes down
        Pos2::new(center_x + half_top, top_y + corner_r),
        Pos2::new(center_x + half_bot, top_y + h - corner_r),
        // Bottom edge
        Pos2::new(center_x + half_bot - corner_r, top_y + h),
        Pos2::new(center_x - half_bot + corner_r, top_y + h),
        // Left side slopes up
        Pos2::new(center_x - half_bot, top_y + h - corner_r),
        Pos2::new(center_x - half_top, top_y + corner_r),
    ]
}

/// A stage's name above its middle and its value below, fading in as the
/// stage finishes growing.
fn draw_stage_labels(
    cx: &super::VizCtx,
    (label, value): (&str, &str),
    (label_font, value_font): (&FontId, &FontId),
    mid: Pos2,
    text_max_w: f32,
    anim: f32,
) {
    let super::VizCtx {
        theme,
        opacity,
        scale,
        ..
    } = *cx;
    let painter = cx.ui.painter();
    let label_opacity = label_fade(anim);
    let min_font = theme.body_size * VIZ_FONT_MIN * scale;

    let label_color = Theme::with_opacity(theme.foreground, opacity * label_opacity);
    let galley = fit_text(
        painter,
        label,
        label_font.clone(),
        label_color,
        text_max_w,
        min_font,
    );
    let lx = mid.x - galley.rect.width() / 2.0;
    painter.galley(
        Pos2::new(lx, mid.y - galley.rect.height() - 1.0 * scale),
        galley,
        label_color,
    );

    let val_color = Theme::with_opacity(theme.foreground, opacity * 0.7 * label_opacity);
    let val_galley = fit_text(
        painter,
        value,
        value_font.clone(),
        val_color,
        text_max_w,
        min_font,
    );
    let vx = mid.x - val_galley.rect.width() / 2.0;
    painter.galley(Pos2::new(vx, mid.y + 1.0 * scale), val_galley, val_color);
}

pub fn draw_funnel_chart(
    cx: &super::VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let super::VizCtx {
        ui,
        theme,
        opacity,
        scale,
        reveal_step,
        reveal_timestamp,
    } = *cx;
    let entries = parse_funnel_chart(content);
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

    let max_value = entries.iter().map(|e| e.value).fold(0.0f32, f32::max);
    if max_value <= 0.0 {
        return height;
    }

    let n = entries.len();
    let padding = 40.0 * scale;
    let gap = 4.0 * scale;
    let total_gaps = (n.saturating_sub(1)) as f32 * gap;
    let available_height = height - padding * 2.0 - total_gaps;
    let trapezoid_height = available_height / n as f32;

    // Funnel is centered horizontally with max width for the widest bar
    let funnel_max_width = max_width - padding * 2.0;
    let min_width_ratio = 0.2; // narrowest trapezoid is at least 20% of max
    let center_x = pos.x + max_width / 2.0;

    let label_font = FontId::new(
        theme.body_size * VIZ_FONT_TITLE * scale,
        theme.body_family(),
    );
    let value_font = FontId::new(
        theme.body_size * VIZ_FONT_SECONDARY_LABEL * scale,
        theme.body_family(),
    );

    let mut needs_repaint = false;

    for (i, entry) in entries.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }

        let (anim, repaint) = reveal_anim_progress(step, reveal_step, reveal_timestamp);
        if repaint {
            needs_repaint = true;
        }

        let color = Theme::with_opacity(palette[i % palette.len()], opacity * theme.fill_opacity());

        // Width proportional to value relative to max
        let width_frac = entry.value / max_value;
        let top_width = funnel_max_width * (min_width_ratio + (1.0 - min_width_ratio) * width_frac);

        // Next entry's width (for trapezoid bottom), or a bit narrower
        let next_width_frac = entries
            .get(i + 1)
            .map(|e| e.value / max_value)
            .unwrap_or(width_frac * 0.6);
        let bottom_width =
            funnel_max_width * (min_width_ratio + (1.0 - min_width_ratio) * next_width_frac);

        let top_y = pos.y + padding + i as f32 * (trapezoid_height + gap);
        let full_h = trapezoid_height;
        let h = full_h * anim;

        // Interpolate bottom width based on animation progress
        let anim_bottom_width = top_width + (bottom_width - top_width) * anim;

        let half_top = top_width / 2.0;
        let half_bot = anim_bottom_width / 2.0;

        // Build a smooth rounded trapezoid using line segments
        // Round the corners with small arcs approximated by extra points
        let corner_r = (VIZ_CORNER_NODE * scale)
            .min(h * 0.3)
            .min((half_top - half_bot).abs() * 0.3);

        let points = trapezoid(
            Pos2::new(center_x, top_y),
            (half_top, half_bot),
            h,
            corner_r,
        );
        painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));

        // Label centered in trapezoid (only when sufficiently visible)
        if anim > VIZ_LABEL_REVEAL_THRESHOLD {
            let pct = entry.value / max_value * 100.0;
            let value_text = format!("{} ({:.0}%)", format_value(entry.value), pct);
            let texts = (entry.label.as_str(), value_text.as_str());
            // Width available at mid height, with a little inset
            let text_max_w = (top_width + anim_bottom_width) / 2.0 - 16.0 * scale;
            let mid = Pos2::new(center_x, top_y + h / 2.0);
            draw_stage_labels(cx, texts, (&label_font, &value_font), mid, text_max_w, anim);
        }
    }

    if needs_repaint {
        ui.ctx().request_repaint();
    }

    height
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_funnel_chart_basic() {
        let content = "- Visitors: 10000\n- Signups: 5000\n- Paid: 1000";
        let entries = parse_funnel_chart(content);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].label, "Visitors");
        assert_eq!(entries[0].value, 10000.0);
        assert_eq!(entries[2].label, "Paid");
        assert_eq!(entries[2].value, 1000.0);
    }

    #[test]
    fn test_parse_funnel_chart_reveal_markers() {
        let content = "- Visitors: 10000\n+ Signups: 5000\n+ Activated: 2500\n+ Paid: 1000";
        let entries = parse_funnel_chart(content);
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].reveal, VizReveal::Static);
        assert_eq!(entries[1].reveal, VizReveal::NextStep);
        assert_eq!(entries[2].reveal, VizReveal::NextStep);
        assert_eq!(entries[3].reveal, VizReveal::NextStep);
    }

    #[test]
    fn test_parse_funnel_chart_skips_comments_and_empty() {
        let content = "# comment\n\n- A: 100\n- invalid line\n- B: 50";
        let entries = parse_funnel_chart(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "A");
        assert_eq!(entries[1].label, "B");
    }

    #[test]
    fn test_parse_funnel_chart_percentage_suffix() {
        let content = "- Top: 100%\n- Mid: 50%";
        let entries = parse_funnel_chart(content);
        assert_eq!(entries[0].value, 100.0);
        assert_eq!(entries[1].value, 50.0);
    }

    #[test]
    fn test_parse_funnel_chart_rejects_non_finite_and_clamps_negative() {
        let entries = parse_funnel_chart("- A: inf\n- B: -10\n- C: 10,000\n- D: $500");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].value, 0.0);
        assert_eq!(entries[1].value, 10000.0);
        assert_eq!(entries[2].value, 500.0);
    }

    #[test]
    fn test_trapezoid_corners() {
        let points = trapezoid(Pos2::new(100.0, 10.0), (50.0, 30.0), 40.0, 5.0);
        assert_eq!(points.len(), 8);
        assert_eq!(points[0], Pos2::new(55.0, 10.0));
        assert_eq!(points[3], Pos2::new(130.0, 45.0));
        assert_eq!(points[5], Pos2::new(75.0, 50.0));
        assert_eq!(points[7], Pos2::new(50.0, 15.0));
    }
}
