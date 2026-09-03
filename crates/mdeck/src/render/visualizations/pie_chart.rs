use std::time::Instant;

use eframe::egui::{self, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    LegendItem, VIZ_OPACITY_BORDER_RING, VIZ_OPACITY_FILL, VIZ_STROKE_BORDER, VIZ_STROKE_SEPARATOR,
    VizReveal, assign_steps, draw_legend_column, parse_label_value, parse_reveal_prefix,
    reveal_anim_progress, sector_mesh, side_legend_width,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct PieEntry {
    label: String,
    value: f32,
    reveal: VizReveal,
}

fn parse_pie_chart(content: &str) -> Vec<PieEntry> {
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

        // Parse "Label: 40%" or "Label: 40"; a negative share is meaningless → 0
        if let Some((label, value)) = parse_label_value(text) {
            entries.push(PieEntry {
                label,
                value: value.max(0.0),
                reveal,
            });
        }
    }
    entries
}

// ─── Renderer ───────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_pie_chart(
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
    let entries = parse_pie_chart(content);
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

    // Layout: pie on left side, legend on right
    let legend_width = side_legend_width(max_width, scale);
    let pie_area_width = max_width - legend_width;
    let pie_radius = (pie_area_width.min(height) / 2.0 - 30.0 * scale).max(40.0 * scale);
    let pie_cx = pos.x + pie_area_width / 2.0;
    let pie_cy = pos.y + height / 2.0;

    // Draw pie slices
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
            Pos2::new(pie_cx, pie_cy),
            0.0,
            pie_radius,
            angle_offset,
            sweep,
            color,
        ));

        // Separator line between slices
        let end_angle = angle_offset + sweep;
        let sep_end = Pos2::new(
            pie_cx + (pie_radius + 1.0) * end_angle.cos(),
            pie_cy + (pie_radius + 1.0) * end_angle.sin(),
        );
        painter.line_segment(
            [Pos2::new(pie_cx, pie_cy), sep_end],
            Stroke::new(VIZ_STROKE_SEPARATOR * scale, bg_color),
        );

        angle_offset += sweep;
    }

    if needs_repaint {
        ui.ctx().request_repaint();
    }

    // Draw subtle border ring
    let ring_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_BORDER_RING);
    painter.circle_stroke(
        Pos2::new(pie_cx, pie_cy),
        pie_radius,
        Stroke::new(VIZ_STROKE_BORDER * scale, ring_color),
    );

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
        pos.x + pie_area_width + legend_gap,
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
    fn test_parse_pie_chart_percentages() {
        let content = "- Category A: 40%\n- Category B: 25%";
        let entries = parse_pie_chart(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "Category A");
        assert_eq!(entries[0].value, 40.0);
        assert_eq!(entries[1].label, "Category B");
        assert_eq!(entries[1].value, 25.0);
    }

    #[test]
    fn test_parse_pie_chart_raw_values() {
        let content = "- Sales: 100\n- Costs: 60";
        let entries = parse_pie_chart(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].value, 100.0);
        assert_eq!(entries[1].value, 60.0);
    }

    #[test]
    fn test_parse_pie_chart_reveal_markers() {
        let content = "- A: 40%\n+ B: 30%\n* C: 30%";
        let entries = parse_pie_chart(content);
        assert_eq!(entries[0].reveal, VizReveal::Static);
        assert_eq!(entries[1].reveal, VizReveal::NextStep);
        assert_eq!(entries[2].reveal, VizReveal::WithPrev);
    }

    #[test]
    fn test_parse_pie_chart_skips_invalid() {
        let content = "- Valid: 50%\n- no_value\n# comment\n- Also Valid: 50%";
        let entries = parse_pie_chart(content);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_parse_pie_chart_rejects_non_finite_and_clamps_negative() {
        let entries = parse_pie_chart("- A: inf\n- B: nan\n- C: -5\n- D: 5");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "C");
        assert_eq!(entries[0].value, 0.0);
        assert_eq!(entries[1].value, 5.0);
    }

    #[test]
    fn test_parse_pie_chart_decorated_values() {
        let entries = parse_pie_chart("- A: 1,000\n- B: $250\n- C: 40 users");
        let values: Vec<f32> = entries.iter().map(|e| e.value).collect();
        assert_eq!(values, vec![1000.0, 250.0, 40.0]);
    }
}
