use eframe::egui::{FontId, Pos2, Stroke};

use crate::theme::Theme;

use super::{
    VIZ_FONT_PRIMARY_LABEL, VIZ_FONT_SECONDARY_LABEL, VIZ_STROKE_SEPARATOR, VizReveal,
    assign_steps, parse_reveal_prefix, parse_value,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

/// Default circle size when none is given.
const DEFAULT_SIZE: f32 = 30.0;

/// A circle size must be finite and positive; anything else would give NaN
/// radii (0/0) or negative geometry. Non-positive sizes become a tiny circle.
fn sanitize_size(size: Option<f32>) -> f32 {
    match size {
        Some(v) if v.is_finite() && v > 0.0 => v,
        Some(v) if v.is_finite() => 1.0,
        _ => DEFAULT_SIZE,
    }
}

#[derive(Debug, Clone)]
struct VennCircle {
    label: String,
    size: f32,
    reveal: VizReveal,
}

#[derive(Debug, Clone)]
struct VennIntersection {
    sets: Vec<String>,
    label: String,
    reveal: VizReveal,
}

fn parse_venn_diagram(content: &str) -> (Vec<VennCircle>, Vec<VennIntersection>) {
    let mut circles = Vec::new();
    let mut intersections = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (text, reveal) = parse_reveal_prefix(trimmed);
        if text.is_empty() {
            continue;
        }

        if text.contains(" & ") {
            // Intersection line: "A & B: Label"
            if let Some(colon_pos) = text.find(": ") {
                let sets_part = &text[..colon_pos];
                let label = text[colon_pos + 2..].trim().to_string();
                let sets: Vec<String> = sets_part
                    .split(" & ")
                    .map(|s| s.trim().to_string())
                    .collect();
                intersections.push(VennIntersection {
                    sets,
                    label,
                    reveal,
                });
            }
        } else {
            // Circle line: "Label (size: N)" or "Label"
            let (label, size) = if let Some(paren_start) = text.find('(') {
                let label = text[..paren_start].trim().to_string();
                let inner = text[paren_start..]
                    .trim_start_matches('(')
                    .trim_end_matches(')');
                let size = sanitize_size(inner.strip_prefix("size:").and_then(parse_value));
                (label, size)
            } else {
                // Could be "Label: value" format
                if let Some(colon_pos) = text.find(": ") {
                    let label = text[..colon_pos].trim().to_string();
                    let val_str = text[colon_pos + 2..].trim();
                    // Check if it looks like a size value (not an intersection)
                    if let Some(v) = parse_value(val_str) {
                        (label, sanitize_size(Some(v)))
                    } else {
                        (text.to_string(), DEFAULT_SIZE)
                    }
                } else {
                    (text.to_string(), DEFAULT_SIZE)
                }
            };
            circles.push(VennCircle {
                label,
                size,
                reveal,
            });
        }
    }

    (circles, intersections)
}

// ─── Renderer ───────────────────────────────────────────────────────────────

/// Circle radii proportional to the square root of each size (so area tracks
/// the value), with the largest circle at `max_radius`. Never produces NaN.
fn circle_radii(sizes: &[f32], max_radius: f32) -> Vec<f32> {
    let max_size = sizes.iter().copied().fold(0.0f32, f32::max);
    let ratio = if sizes.len() >= 3 { 0.62 } else { 0.7 };
    sizes
        .iter()
        .map(|&s| {
            let frac = if max_size > 0.0 && s > 0.0 {
                (s / max_size).sqrt()
            } else {
                1.0
            };
            frac * max_radius * ratio
        })
        .collect()
}

/// Position for an intersection label: the mean of the member centers. When
/// only some circles are involved, the point is pushed away from the overall
/// diagram center so pairwise labels land in their lens rather than piling up
/// near the middle where all lenses meet.
fn intersection_label_pos(
    centers: &[Pos2],
    radii: &[f32],
    members: &[usize],
    cx: f32,
    cy: f32,
) -> (f32, f32) {
    let n = members.len() as f32;
    let mx = members.iter().map(|&i| centers[i].x).sum::<f32>() / n;
    let my = members.iter().map(|&i| centers[i].y).sum::<f32>() / n;
    if members.len() >= centers.len() || centers.len() < 3 {
        return (mx, my);
    }
    let r = members.iter().map(|&i| radii[i]).fold(f32::MAX, f32::min);
    let dx = mx - cx;
    let dy = my - cy;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-3 {
        return (mx, my);
    }
    let push = r * 0.5;
    (mx + dx / len * push, my + dy / len * push)
}

/// Circle centres around `mid`: one in the middle, two side by side
/// overlapping, or three and more in a ring close enough that every pair
/// overlaps substantially (the classic Venn layout).
fn circle_centers(mid: Pos2, radii: &[f32]) -> Vec<Pos2> {
    let (cx, cy) = (mid.x, mid.y);
    match radii.len() {
        1 => vec![Pos2::new(cx, cy)],
        2 => {
            let overlap = radii[0].min(radii[1]) * 0.6;
            let dist = radii[0] + radii[1] - overlap;
            vec![
                Pos2::new(cx - dist / 2.0, cy),
                Pos2::new(cx + dist / 2.0, cy),
            ]
        }
        n => {
            let base_dist = radii.iter().cloned().fold(0.0f32, f32::max) * 0.62;
            (0..n)
                .map(|i| {
                    let angle = -std::f32::consts::FRAC_PI_2
                        + (i as f32 / n as f32) * 2.0 * std::f32::consts::PI;
                    Pos2::new(cx + base_dist * angle.cos(), cy + base_dist * angle.sin())
                })
                .collect()
        }
    }
}

/// Where the diagram's circles go.
#[derive(Debug, Clone, PartialEq)]
struct VennLayout {
    mid: Pos2,
    radii: Vec<f32>,
    centers: Vec<Pos2>,
}

/// Circles sized by `sizes` around the middle of the chart, the largest as
/// big as the space allows with a margin.
fn venn_layout(pos: Pos2, max_width: f32, height: f32, scale: f32, sizes: &[f32]) -> VennLayout {
    let mid = Pos2::new(pos.x + max_width / 2.0, pos.y + height / 2.0);
    let max_radius = (max_width.min(height) / 2.0 - 60.0 * scale).max(40.0 * scale);
    let radii = circle_radii(sizes, max_radius);
    let centers = circle_centers(mid, &radii);
    VennLayout {
        mid,
        radii,
        centers,
    }
}

impl VennLayout {
    /// Where the label of the intersection of `members` goes, and how wide it
    /// may wrap: 1.3 times the smallest member's radius.
    fn intersection_spot(&self, members: &[usize]) -> (Pos2, f32) {
        let (x, y) =
            intersection_label_pos(&self.centers, &self.radii, members, self.mid.x, self.mid.y);
        let wrap_width = members
            .iter()
            .map(|&i| self.radii[i])
            .fold(f32::MAX, f32::min)
            * 1.3;
        (Pos2::new(x, y), wrap_width)
    }
}

pub fn draw_venn_diagram(
    cx: &super::VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let scale = cx.scale;
    let (circles, intersections) = parse_venn_diagram(content);
    if circles.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    // Assign reveal steps for circles and intersections together
    let mut all_reveals: Vec<VizReveal> = circles.iter().map(|c| c.reveal).collect();
    let intersection_start = all_reveals.len();
    for inter in &intersections {
        all_reveals.push(inter.reveal);
    }
    let steps = assign_steps(&all_reveals);

    // Compute circle radii proportional to size values
    let sizes: Vec<f32> = circles.iter().map(|c| c.size).collect();
    let layout = venn_layout(pos, max_width, height, scale, &sizes);

    draw_circles(cx, &layout, &circles, &steps);
    draw_intersection_labels(
        cx,
        &layout,
        &circles,
        &intersections,
        &steps[intersection_start..],
    );

    height
}

/// Each revealed circle, growing from its centre, with its label in the
/// non-overlapping part.
fn draw_circles(cx: &super::VizCtx, layout: &VennLayout, circles: &[VennCircle], steps: &[usize]) {
    let super::VizCtx {
        ui,
        theme,
        opacity,
        scale,
        reveal_step,
        ..
    } = *cx;
    let painter = ui.painter();
    let palette = theme.edge_palette();
    let mid = layout.mid;
    let label_font = FontId::new(
        theme.body_size * VIZ_FONT_PRIMARY_LABEL * scale,
        theme.body_family(),
    );

    for (i, circle) in circles.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }

        let anim = cx.anim(step);

        let color = palette[i % palette.len()];
        let fill_color = Theme::with_opacity(color, opacity * 0.25 * anim);
        let stroke_color = Theme::with_opacity(color, opacity * anim);
        let radius = layout.radii[i] * anim;
        let center = layout.centers[i];

        painter.circle_filled(center, radius, fill_color);
        crate::render::hints::push(
            ui.ctx(),
            crate::render::hints::Hint::Circle { center, radius },
        );
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(VIZ_STROKE_SEPARATOR * scale, stroke_color),
        );

        // Label in the non-overlapping region (offset away from center)
        let label_offset_x = (center.x - mid.x) * 0.4;
        let label_offset_y = (center.y - mid.y) * 0.4;
        let label_color = Theme::with_opacity(theme.foreground, opacity * anim);
        let galley = painter.layout_no_wrap(circle.label.clone(), label_font.clone(), label_color);
        let lx = center.x + label_offset_x - galley.rect.width() / 2.0;
        let ly = center.y + label_offset_y - galley.rect.height() / 2.0;
        painter.galley(Pos2::new(lx, ly), galley, label_color);
    }
}

/// Each revealed intersection's label (`steps` are the intersections' own),
/// in the lens its member circles share.
fn draw_intersection_labels(
    cx: &super::VizCtx,
    layout: &VennLayout,
    circles: &[VennCircle],
    intersections: &[VennIntersection],
    steps: &[usize],
) {
    let super::VizCtx {
        theme,
        opacity,
        scale,
        reveal_step,
        ..
    } = *cx;
    let painter = cx.ui.painter();
    let inter_font = FontId::new(
        theme.body_size * VIZ_FONT_SECONDARY_LABEL * scale,
        theme.body_family(),
    );

    for (j, inter) in intersections.iter().enumerate() {
        let step = steps.get(j).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }

        let anim = cx.anim(step);

        let members: Vec<usize> = inter
            .sets
            .iter()
            .filter_map(|name| circles.iter().position(|c| &c.label == name))
            .collect();
        if members.is_empty() {
            continue;
        }
        let (spot, wrap_width) = layout.intersection_spot(&members);

        let label_color = Theme::with_opacity(theme.foreground, opacity * anim);
        let galley = painter.layout(
            inter.label.clone(),
            inter_font.clone(),
            label_color,
            wrap_width.max(80.0 * scale),
        );
        let lx = spot.x - galley.rect.width() / 2.0;
        let ly = spot.y - galley.rect.height() / 2.0;
        painter.galley(Pos2::new(lx, ly), galley, label_color);
    }
}
// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersection_label_pairwise_pushed_outward() {
        // Three circles in a triangle around (0, 0)
        let centers = vec![
            Pos2::new(0.0, -100.0),
            Pos2::new(87.0, 50.0),
            Pos2::new(-87.0, 50.0),
        ];
        let radii = vec![100.0, 100.0, 100.0];
        // Pair (0, 2) lies up-left of the center; the label must move further that way
        let (x, y) = intersection_label_pos(&centers, &radii, &[0, 2], 0.0, 0.0);
        assert!(x < -43.5 && y < -25.0);
        // All three sets → exactly the centroid, no push
        let (x, y) = intersection_label_pos(&centers, &radii, &[0, 1, 2], 0.0, 0.0);
        assert!(x.abs() < 1e-3 && y.abs() < 1e-3);
        // Two-set diagrams keep the plain midpoint
        let (x, y) = intersection_label_pos(&centers[..2], &radii[..2], &[0, 1], 43.5, -25.0);
        assert!((x - 43.5).abs() < 1e-3 && (y + 25.0).abs() < 1e-3);
    }

    #[test]
    fn test_parse_venn_two_circles() {
        let content = "- Frontend (size: 40)\n- Backend (size: 35)";
        let (circles, intersections) = parse_venn_diagram(content);
        assert_eq!(circles.len(), 2);
        assert_eq!(circles[0].label, "Frontend");
        assert_eq!(circles[0].size, 40.0);
        assert_eq!(circles[1].label, "Backend");
        assert_eq!(circles[1].size, 35.0);
        assert!(intersections.is_empty());
    }

    #[test]
    fn test_parse_venn_with_intersections() {
        let content =
            "- Frontend (size: 40)\n- Backend (size: 35)\n+ Frontend & Backend: Fullstack";
        let (circles, intersections) = parse_venn_diagram(content);
        assert_eq!(circles.len(), 2);
        assert_eq!(intersections.len(), 1);
        assert_eq!(intersections[0].sets, vec!["Frontend", "Backend"]);
        assert_eq!(intersections[0].label, "Fullstack");
        assert_eq!(intersections[0].reveal, VizReveal::NextStep);
    }

    #[test]
    fn test_parse_venn_three_circles() {
        let content = "- A (size: 30)\n- B (size: 25)\n- C (size: 20)\n+ A & B: AB\n+ B & C: BC";
        let (circles, intersections) = parse_venn_diagram(content);
        assert_eq!(circles.len(), 3);
        assert_eq!(intersections.len(), 2);
    }

    #[test]
    fn test_parse_venn_no_size() {
        let content = "- Frontend\n- Backend";
        let (circles, _) = parse_venn_diagram(content);
        assert_eq!(circles.len(), 2);
        assert_eq!(circles[0].size, 30.0); // default
        assert_eq!(circles[1].size, 30.0);
    }

    #[test]
    fn test_parse_venn_skips_comments() {
        let content = "# header\n- A (size: 10)\n# note\n- B (size: 20)";
        let (circles, _) = parse_venn_diagram(content);
        assert_eq!(circles.len(), 2);
    }

    #[test]
    fn test_parse_venn_zero_and_negative_sizes_are_sanitised() {
        let content = "- A (size: 0)\n- B (size: -5)\n- C: 0";
        let (circles, _) = parse_venn_diagram(content);
        assert_eq!(circles.len(), 3);
        for c in &circles {
            assert!(
                c.size > 0.0 && c.size.is_finite(),
                "size {} for {}",
                c.size,
                c.label
            );
        }
        let (circles, _) = parse_venn_diagram("- A (size: inf)\n- B (size: nan)");
        for c in &circles {
            assert!(c.size > 0.0 && c.size.is_finite());
        }
    }

    #[test]
    fn test_circle_radii_never_nan() {
        let radii = circle_radii(&[0.0, 0.0], 100.0);
        assert!(radii.iter().all(|r| r.is_finite() && *r > 0.0));
        let radii = circle_radii(&[40.0, 10.0], 100.0);
        assert!(radii.iter().all(|r| r.is_finite() && *r > 0.0));
        assert!(radii[0] > radii[1]);
    }

    #[test]
    fn test_circle_centers_by_count() {
        let mid = Pos2::new(100.0, 100.0);
        assert_eq!(circle_centers(mid, &[50.0]), vec![mid]);
        let two = circle_centers(mid, &[50.0, 50.0]);
        assert_eq!(two, vec![Pos2::new(65.0, 100.0), Pos2::new(135.0, 100.0)]);
        let three = circle_centers(mid, &[50.0, 50.0, 50.0]);
        assert_eq!(three.len(), 3);
        // The first sits straight above the middle
        assert!((three[0].x - 100.0).abs() < 1e-4);
        assert!((three[0].y - 69.0).abs() < 1e-4);
    }

    #[test]
    fn test_venn_layout_centres_circles_in_the_chart() {
        let layout = venn_layout(Pos2::new(0.0, 0.0), 1000.0, 500.0, 1.0, &[30.0, 30.0]);
        assert_eq!(layout.mid, Pos2::new(500.0, 250.0));
        // Largest radius: half the short side less the margin, times the 2-circle ratio
        assert_eq!(layout.radii, vec![190.0 * 0.7; 2]);
        assert_eq!(layout.centers, circle_centers(layout.mid, &layout.radii));

        let (spot, wrap) = layout.intersection_spot(&[0, 1]);
        assert_eq!(spot, layout.mid);
        assert_eq!(wrap, 190.0 * 0.7 * 1.3);
    }

    #[test]
    fn test_venn_layout_keeps_a_minimum_radius() {
        let layout = venn_layout(Pos2::new(0.0, 0.0), 100.0, 100.0, 1.0, &[1.0]);
        assert_eq!(layout.radii, vec![40.0 * 0.7]);
    }
}
