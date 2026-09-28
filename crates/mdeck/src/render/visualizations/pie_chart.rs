use eframe::egui::{Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_OPACITY_BORDER_RING, VIZ_STROKE_BORDER, VizReveal, assign_steps, draw_side_legend,
    parse_label_value, parse_reveal_prefix, ring_layout, share_legend_items,
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

pub fn draw_pie_chart(
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
        ..
    } = *cx;
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
    let painter = ui.painter();

    // Compute total for percentages
    let total: f32 = entries.iter().map(|e| e.value).sum();
    if total <= 0.0 {
        return height;
    }

    // Layout: pie on left side, legend on right
    let ring = ring_layout(pos, max_width, height, scale, 0.0);
    let values: Vec<f32> = entries.iter().map(|e| e.value).collect();
    ring.draw_slices(cx, &values, total, &steps);

    // Draw subtle border ring
    let ring_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_BORDER_RING);
    painter.circle_stroke(
        ring.center,
        ring.outer_radius,
        Stroke::new(VIZ_STROKE_BORDER * scale, ring_color),
    );

    // Legend on the right
    let shares = entries.iter().map(|e| (e.label.as_str(), e.value));
    let items = share_legend_items(cx, shares, total, &steps);
    draw_side_legend(
        cx,
        &items,
        pos.x + ring.area_width,
        pos.y,
        ring.legend_width,
        height,
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
