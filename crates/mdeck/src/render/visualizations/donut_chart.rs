use std::time::Instant;

use eframe::egui::{self, FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    LegendItem, VIZ_FONT_MIN, VIZ_OPACITY_BORDER_RING, VIZ_OPACITY_FILL, VIZ_STROKE_BORDER,
    VIZ_STROKE_SEPARATOR, VizReveal, assign_steps, draw_legend_column, fit_text, parse_label_value,
    parse_reveal_prefix, reveal_anim_progress, sector_mesh, side_legend_width,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct DonutEntry {
    label: String,
    value: f32,
    reveal: VizReveal,
}

fn parse_donut_chart(content: &str) -> (Vec<DonutEntry>, Option<String>) {
    let mut entries = Vec::new();
    let mut center_text = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse center text directive
        if trimmed.starts_with('#') {
            if let Some(rest) = trimmed
                .strip_prefix("# center:")
                .or_else(|| trimmed.strip_prefix("#center:"))
            {
                center_text = Some(rest.trim().to_string());
            }
            continue;
        }

        let (text, reveal) = parse_reveal_prefix(trimmed);
        if text.is_empty() {
            continue;
        }

        // Parse "Label: 40%" or "Label: 40"; a negative share is meaningless → 0
        if let Some((label, value)) = parse_label_value(text) {
            entries.push(DonutEntry {
                label,
                value: value.max(0.0),
                reveal,
            });
        }
    }

    (entries, center_text)
}

// ─── Renderer ───────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_donut_chart(
    ui: &egui::Ui,
    content: &str,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
    opacity: f32,
    reveal_step: usize,
    reveal_timestamp: Option<Instant>,
    scale: f32,
) -> f32 {
    let (entries, center_text) = parse_donut_chart(content);
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

    // Compute total for percentages
    let total: f32 = entries.iter().map(|e| e.value).sum();
    if total <= 0.0 {
        return height;
    }

    // Layout: donut on left side, legend on right
    let legend_width = side_legend_width(max_width, scale);
    let donut_area_width = max_width - legend_width;
    let outer_radius = (donut_area_width.min(height) / 2.0 - 30.0 * scale).max(40.0 * scale);
    let inner_radius = outer_radius * 0.5; // 50% thickness (thick ring)
    let donut_cx = pos.x + donut_area_width / 2.0;
    let donut_cy = pos.y + height / 2.0;

    // Draw donut slices
    let mut angle_offset = -std::f32::consts::FRAC_PI_2; // start at top
    let mut needs_repaint = false;

    let bg_color = Theme::with_opacity(theme.background, opacity);

    for (i, entry) in entries.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }

        let (anim, repaint) = reveal_anim_progress(step, reveal_step, reveal_timestamp);
        if repaint {
            needs_repaint = true;
        }

        let full_sweep = (entry.value / total) * 2.0 * std::f32::consts::PI;
        let sweep = full_sweep * anim;
        let color = Theme::with_opacity(palette[i % palette.len()], opacity * VIZ_OPACITY_FILL);

        // Single mesh per slice: no anti-aliasing seams between segments
        painter.add(sector_mesh(
            Pos2::new(donut_cx, donut_cy),
            inner_radius,
            outer_radius,
            angle_offset,
            sweep,
            color,
        ));

        // Separator line between slices
        let end_angle = angle_offset + sweep;
        let sep_inner = Pos2::new(
            donut_cx + (inner_radius - 1.0) * end_angle.cos(),
            donut_cy + (inner_radius - 1.0) * end_angle.sin(),
        );
        let sep_outer = Pos2::new(
            donut_cx + (outer_radius + 1.0) * end_angle.cos(),
            donut_cy + (outer_radius + 1.0) * end_angle.sin(),
        );
        painter.line_segment(
            [sep_inner, sep_outer],
            Stroke::new(VIZ_STROKE_SEPARATOR * scale, bg_color),
        );

        angle_offset += sweep;
    }

    if needs_repaint {
        ui.ctx().request_repaint();
    }

    // Draw center hole (background color circle to create donut effect)
    painter.circle_filled(Pos2::new(donut_cx, donut_cy), inner_radius, bg_color);

    // Draw subtle border rings
    let ring_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_BORDER_RING);
    painter.circle_stroke(
        Pos2::new(donut_cx, donut_cy),
        outer_radius,
        Stroke::new(VIZ_STROKE_BORDER * scale, ring_color),
    );
    painter.circle_stroke(
        Pos2::new(donut_cx, donut_cy),
        inner_radius,
        Stroke::new(1.0 * scale, ring_color),
    );

    // Draw center text, fitted inside the hole
    if let Some(ref text) = center_text {
        let center_font = FontId::proportional(theme.body_size * 1.2 * scale);
        let text_color = Theme::with_opacity(theme.foreground, opacity);
        let galley = fit_text(
            painter,
            text,
            center_font,
            text_color,
            inner_radius * 2.0 * 0.85,
            theme.body_size * VIZ_FONT_MIN * scale,
        );
        painter.galley(
            Pos2::new(
                donut_cx - galley.rect.width() / 2.0,
                donut_cy - galley.rect.height() / 2.0,
            ),
            galley,
            text_color,
        );
    }

    // Draw legend on the right
    let legend_gap = 20.0 * scale;
    let items: Vec<LegendItem> = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| LegendItem {
            label: entry.label.clone(),
            suffix: format!(" ({:.0}%)", entry.value / total * 100.0),
            color: Theme::with_opacity(palette[i % palette.len()], opacity * VIZ_OPACITY_FILL),
            visible: steps.get(i).copied().unwrap_or(0) <= reveal_step,
        })
        .collect();
    draw_legend_column(
        painter,
        &items,
        theme,
        opacity,
        pos.x + donut_area_width + legend_gap,
        pos.y,
        legend_width - legend_gap,
        height,
        scale,
    );

    height
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_donut_chart_basic() {
        let content = "- Complete: 78\n- Remaining: 22";
        let (entries, center) = parse_donut_chart(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "Complete");
        assert_eq!(entries[0].value, 78.0);
        assert_eq!(entries[1].label, "Remaining");
        assert_eq!(entries[1].value, 22.0);
        assert!(center.is_none());
    }

    #[test]
    fn test_parse_donut_chart_with_center() {
        let content = "# center: 78%\n- Complete: 78\n- Remaining: 22";
        let (entries, center) = parse_donut_chart(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(center, Some("78%".to_string()));
    }

    #[test]
    fn test_parse_donut_chart_reveal_markers() {
        let content = "- A: 40%\n+ B: 30%\n* C: 30%";
        let (entries, _) = parse_donut_chart(content);
        assert_eq!(entries[0].reveal, VizReveal::Static);
        assert_eq!(entries[1].reveal, VizReveal::NextStep);
        assert_eq!(entries[2].reveal, VizReveal::WithPrev);
    }

    #[test]
    fn test_parse_donut_chart_skips_invalid() {
        let content = "# center: Done\n- Valid: 50%\n- no_value\n# comment\n- Also Valid: 50%";
        let (entries, center) = parse_donut_chart(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(center, Some("Done".to_string()));
    }

    #[test]
    fn test_parse_donut_chart_rejects_non_finite_and_clamps_negative() {
        let (entries, _) = parse_donut_chart("- A: inf\n- B: -3\n- C: 1,000");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].value, 0.0);
        assert_eq!(entries[1].value, 1000.0);
    }
}
