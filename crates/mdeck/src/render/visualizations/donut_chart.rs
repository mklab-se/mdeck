use eframe::egui::{FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_FONT_MIN, VIZ_OPACITY_BORDER_RING, VIZ_STROKE_BORDER, VizReveal, assign_steps,
    draw_side_legend, fit_text,
    grammar::{Problem, Source, label_value_items},
    ring_layout, share_legend_items,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct DonutEntry {
    label: String,
    value: f32,
    reveal: VizReveal,
}

fn read(src: &Source) -> (Vec<DonutEntry>, Option<String>) {
    src.check_settings(&["center"]);
    let entries = label_value_items(src, "- Completed: 65%")
        .into_iter()
        // a negative share is meaningless → 0
        .map(|e| DonutEntry {
            label: e.label,
            value: e.value.max(0.0),
            reveal: e.reveal,
        })
        .collect();
    (entries, src.setting("center").map(str::to_string))
}

fn parse_donut_chart(content: &str) -> (Vec<DonutEntry>, Option<String>) {
    read(&Source::parse(content))
}

/// The problems in a `@donut` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
}

// ─── Renderer ───────────────────────────────────────────────────────────────

/// The hole: background over the ring's inside, subtle border rings and
/// the centre text fitted inside.
fn draw_hole(
    cx: &super::VizCtx,
    center: Pos2,
    (inner_radius, outer_radius): (f32, f32),
    center_text: Option<&str>,
) {
    let super::VizCtx {
        theme,
        opacity,
        scale,
        ..
    } = *cx;
    let painter = cx.ui.painter();
    painter.circle_filled(
        center,
        inner_radius,
        Theme::with_opacity(theme.background, opacity),
    );

    let ring_color = Theme::with_opacity(theme.foreground, opacity * VIZ_OPACITY_BORDER_RING);
    painter.circle_stroke(
        center,
        outer_radius,
        Stroke::new(VIZ_STROKE_BORDER * scale, ring_color),
    );
    painter.circle_stroke(center, inner_radius, Stroke::new(1.0 * scale, ring_color));

    if let Some(text) = center_text {
        let center_font = FontId::new(theme.body_size * 1.2 * scale, theme.body_family());
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
                center.x - galley.rect.width() / 2.0,
                center.y - galley.rect.height() / 2.0,
            ),
            galley,
            text_color,
        );
    }
}

pub fn draw_donut_chart(
    cx: &super::VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let scale = cx.scale;
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

    // Compute total for percentages
    let total: f32 = entries.iter().map(|e| e.value).sum();
    if total <= 0.0 {
        return height;
    }

    // Layout: donut on left side, legend on right; 50% thickness (thick ring)
    let ring = ring_layout(pos, max_width, height, scale, 0.5);
    let values: Vec<f32> = entries.iter().map(|e| e.value).collect();
    ring.draw_slices(cx, &values, total, &steps);

    draw_hole(
        cx,
        ring.center,
        (ring.inner_radius, ring.outer_radius),
        center_text.as_deref(),
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
        let content = "center: 78%\n- Complete: 78\n- Remaining: 22";
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
        let content = "center: Done\n- Valid: 50%\n- no_value\n# comment\n- Also Valid: 50%";
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
