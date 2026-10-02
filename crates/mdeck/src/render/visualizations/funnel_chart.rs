use eframe::egui::{self, FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_CORNER_NODE, VIZ_FONT_MIN, VIZ_FONT_SECONDARY_LABEL, VIZ_FONT_TITLE,
    VIZ_LABEL_REVEAL_THRESHOLD, VizReveal, assign_steps, fit_text, format_value,
    grammar::{Problem, Source, label_value_items},
    label_fade,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct FunnelEntry {
    label: String,
    value: f32,
    reveal: VizReveal,
}

fn read(src: &Source) -> Vec<FunnelEntry> {
    src.check_settings(&[]);
    label_value_items(src, "- Label: 40")
        .into_iter()
        .map(|e| FunnelEntry {
            label: e.label,
            value: e.value.max(0.0),
            reveal: e.reveal,
        })
        .collect()
}

fn parse_funnel_chart(content: &str) -> Vec<FunnelEntry> {
    read(&Source::parse(content))
}

/// The problems in a `@funnel` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
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

/// The narrowest trapezoid is at least this share of the widest.
const MIN_WIDTH_RATIO: f32 = 0.2;

/// Where the funnel's stages go: stacked trapezoids centred horizontally,
/// the widest spanning the chart inside its padding.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FunnelLayout {
    center_x: f32,
    /// Top of the first stage.
    top: f32,
    trapezoid_height: f32,
    gap: f32,
    /// Width of a stage at the largest value.
    funnel_max_width: f32,
}

fn funnel_layout(pos: Pos2, max_width: f32, height: f32, scale: f32, n: usize) -> FunnelLayout {
    let padding = 40.0 * scale;
    let gap = 4.0 * scale;
    let total_gaps = (n.saturating_sub(1)) as f32 * gap;
    let available_height = height - padding * 2.0 - total_gaps;
    FunnelLayout {
        center_x: pos.x + max_width / 2.0,
        top: pos.y + padding,
        trapezoid_height: available_height / n as f32,
        gap,
        funnel_max_width: max_width - padding * 2.0,
    }
}

impl FunnelLayout {
    /// Top edge of stage `i`.
    fn top_y(&self, i: usize) -> f32 {
        self.top + i as f32 * (self.trapezoid_height + self.gap)
    }

    /// Width of a stage edge at `width_frac` of the largest value.
    fn width_at(&self, width_frac: f32) -> f32 {
        self.funnel_max_width * (MIN_WIDTH_RATIO + (1.0 - MIN_WIDTH_RATIO) * width_frac)
    }
}

/// Each stage's (top, bottom) edge as a share of `max_value`: its own value on
/// top and the next stage's below, or a bit narrower for the last stage.
fn stage_fracs(values: &[f32], max_value: f32) -> Vec<(f32, f32)> {
    values
        .iter()
        .enumerate()
        .map(|(i, &value)| {
            let width_frac = value / max_value;
            let next_width_frac = values
                .get(i + 1)
                .map(|v| v / max_value)
                .unwrap_or(width_frac * 0.6);
            (width_frac, next_width_frac)
        })
        .collect()
}

pub fn draw_funnel_chart(
    cx: &super::VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let super::VizCtx {
        theme,
        scale,
        reveal_step,
        ..
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

    let max_value = entries.iter().map(|e| e.value).fold(0.0f32, f32::max);
    if max_value <= 0.0 {
        return height;
    }

    let values: Vec<f32> = entries.iter().map(|e| e.value).collect();
    let fracs = stage_fracs(&values, max_value);
    let funnel = Funnel {
        layout: funnel_layout(pos, max_width, height, scale, entries.len()),
        label_font: FontId::new(
            theme.body_size * VIZ_FONT_TITLE * scale,
            theme.body_family(),
        ),
        value_font: FontId::new(
            theme.body_size * VIZ_FONT_SECONDARY_LABEL * scale,
            theme.body_family(),
        ),
        max_value,
    };

    for (i, entry) in entries.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }
        let anim = cx.anim(step);
        funnel.draw_stage(cx, i, entry, fracs[i], anim);
    }

    height
}

/// Paints the stages of a `FunnelLayout`.
struct Funnel {
    layout: FunnelLayout,
    label_font: FontId,
    value_font: FontId,
    max_value: f32,
}

impl Funnel {
    /// Stage `i` with edges at `(width_frac, next_width_frac)`, grown down
    /// from its top edge to `anim`, then its labels.
    fn draw_stage(
        &self,
        cx: &super::VizCtx,
        i: usize,
        entry: &FunnelEntry,
        (width_frac, next_width_frac): (f32, f32),
        anim: f32,
    ) {
        let scale = cx.scale;
        let palette = cx.theme.edge_palette();
        let color = Theme::with_opacity(
            palette[i % palette.len()],
            cx.opacity * cx.theme.fill_opacity(),
        );
        let center_x = self.layout.center_x;

        // Width proportional to value relative to max
        let top_width = self.layout.width_at(width_frac);
        let bottom_width = self.layout.width_at(next_width_frac);

        let top_y = self.layout.top_y(i);
        let h = self.layout.trapezoid_height * anim;

        // Interpolate bottom width based on animation progress
        let anim_bottom_width = top_width + (bottom_width - top_width) * anim;

        let half_top = top_width / 2.0;
        let half_bot = anim_bottom_width / 2.0;

        // Round the corners by cutting them, keeping the cut within the shape
        let corner_r = (VIZ_CORNER_NODE * scale)
            .min(h * 0.3)
            .min((half_top - half_bot).abs() * 0.3);

        let points = trapezoid(
            Pos2::new(center_x, top_y),
            (half_top, half_bot),
            h,
            corner_r,
        );
        cx.ui
            .painter()
            .add(egui::Shape::convex_polygon(points, color, Stroke::NONE));

        // Label centered in trapezoid (only when sufficiently visible)
        if anim > VIZ_LABEL_REVEAL_THRESHOLD {
            let pct = entry.value / self.max_value * 100.0;
            let value_text = format!("{} ({:.0}%)", format_value(entry.value), pct);
            let texts = (entry.label.as_str(), value_text.as_str());
            // Width available at mid height, with a little inset
            let text_max_w = (top_width + anim_bottom_width) / 2.0 - 16.0 * scale;
            let mid = Pos2::new(center_x, top_y + h / 2.0);
            let fonts = (&self.label_font, &self.value_font);
            draw_stage_labels(cx, texts, fonts, mid, text_max_w, anim);
        }
    }
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

    #[test]
    fn test_funnel_layout_stacks_stages_inside_padding() {
        let layout = funnel_layout(Pos2::new(100.0, 20.0), 1000.0, 500.0, 1.0, 3);
        assert_eq!(layout.center_x, 600.0);
        assert_eq!(layout.funnel_max_width, 920.0);
        assert_eq!(layout.trapezoid_height, (500.0 - 80.0 - 8.0) / 3.0);
        assert_eq!(layout.top_y(0), 60.0);
        assert_eq!(
            layout.top_y(2),
            60.0 + 2.0 * (layout.trapezoid_height + 4.0)
        );
        assert_eq!(layout.width_at(1.0), 920.0);
        assert_eq!(layout.width_at(0.0), 920.0 * MIN_WIDTH_RATIO);
    }

    #[test]
    fn test_stage_fracs_narrow_to_the_next_stage() {
        let fracs = stage_fracs(&[100.0, 50.0, 10.0], 100.0);
        assert_eq!(fracs, vec![(1.0, 0.5), (0.5, 0.1), (0.1, 0.1 * 0.6)]);
    }
}
