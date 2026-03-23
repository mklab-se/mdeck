use eframe::egui::{self, FontId, Pos2, Stroke};
use eframe::epaint::CubicBezierShape;

use crate::theme::Theme;

use super::{
    VIZ_FONT_PRIMARY_LABEL, VIZ_FONT_SECONDARY_LABEL, VizReveal, assign_steps, parse_reveal_prefix,
};

// ─── Data model ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum GitGraphItem {
    /// Declare a branch lane (rendered as a dotted background line).
    Lane { name: String },
    /// A commit on a branch (dot on the lane).
    Commit {
        branch: String,
        message: String,
        reveal: VizReveal,
    },
    /// Fork: source branch creates target branch (S-curve, target becomes active).
    Branch {
        source: String,
        target: String,
        reveal: VizReveal,
    },
    /// Merge: source branch merges into target (S-curve, source becomes inactive).
    Merge {
        source: String,
        target: String,
        label: String,
        reveal: VizReveal,
    },
    /// Tag on a branch's latest commit.
    Tag {
        branch: String,
        label: String,
        reveal: VizReveal,
    },
}

// ─── Parsing ────────────────────────────────────────────────────────────────

fn parse_gitgraph(content: &str) -> Vec<GitGraphItem> {
    let mut items = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (text, reveal) = parse_reveal_prefix(trimmed);
        if text.is_empty() {
            continue;
        }

        let lower = text.to_lowercase();

        if lower.starts_with("lane ") {
            let name = text["lane ".len()..].trim().to_string();
            items.push(GitGraphItem::Lane { name });
        } else if lower.starts_with("branch ") {
            let rest = &text["branch ".len()..];
            if let Some(arrow) = rest.find(" -> ") {
                let source = rest[..arrow].trim().to_string();
                let target = rest[arrow + " -> ".len()..].trim().to_string();
                items.push(GitGraphItem::Branch {
                    source,
                    target,
                    reveal,
                });
            }
        } else if lower.starts_with("commit ") {
            let rest = &text["commit ".len()..];
            let (branch, message) = if let Some(colon) = rest.find(": ") {
                (
                    rest[..colon].trim().to_string(),
                    rest[colon + 2..].trim().trim_matches('"').to_string(),
                )
            } else {
                (rest.trim().to_string(), String::new())
            };
            items.push(GitGraphItem::Commit {
                branch,
                message,
                reveal,
            });
        } else if lower.starts_with("merge ") {
            let rest = &text["merge ".len()..];
            if let Some(arrow) = rest.find(" -> ") {
                let source = rest[..arrow].trim().to_string();
                let after_arrow = &rest[arrow + " -> ".len()..];
                let (target, label) = if let Some(colon) = after_arrow.find(": ") {
                    (
                        after_arrow[..colon].trim().to_string(),
                        after_arrow[colon + 2..]
                            .trim()
                            .trim_matches('"')
                            .to_string(),
                    )
                } else {
                    (after_arrow.trim().to_string(), String::new())
                };
                items.push(GitGraphItem::Merge {
                    source,
                    target,
                    label,
                    reveal,
                });
            }
        } else if lower.starts_with("tag ") {
            let rest = &text["tag ".len()..];
            if let Some(colon) = rest.find(": ") {
                let branch = rest[..colon].trim().to_string();
                let label = rest[colon + 2..].trim().trim_matches('"').to_string();
                items.push(GitGraphItem::Tag {
                    branch,
                    label,
                    reveal,
                });
            }
        }
    }
    items
}

// ─── Renderer ───────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_gitgraph(
    ui: &egui::Ui,
    content: &str,
    theme: &Theme,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
    opacity: f32,
    reveal_step: usize,
    scale: f32,
) -> f32 {
    let items = parse_gitgraph(content);
    if items.is_empty() {
        return 0.0;
    }

    let height = if max_height > 0.0 {
        max_height
    } else {
        500.0 * scale
    };

    // Assign reveal steps (lanes are always static)
    let reveals: Vec<VizReveal> = items
        .iter()
        .map(|item| match item {
            GitGraphItem::Lane { .. } => VizReveal::Static,
            GitGraphItem::Commit { reveal, .. }
            | GitGraphItem::Branch { reveal, .. }
            | GitGraphItem::Merge { reveal, .. }
            | GitGraphItem::Tag { reveal, .. } => *reveal,
        })
        .collect();
    let steps = assign_steps(&reveals);

    let palette = theme.edge_palette();
    let painter = ui.painter();

    // ── 1. Collect lanes (declared order = vertical position) ───────────
    let mut lane_order: Vec<String> = Vec::new();
    for item in &items {
        if let GitGraphItem::Lane { name } = item {
            if !lane_order.contains(name) {
                lane_order.push(name.clone());
            }
        }
    }
    // Also add any branch referenced in events but not declared as a lane
    for item in &items {
        let names: Vec<&str> = match item {
            GitGraphItem::Commit { branch, .. } | GitGraphItem::Tag { branch, .. } => {
                vec![branch.as_str()]
            }
            GitGraphItem::Branch { source, target, .. }
            | GitGraphItem::Merge { source, target, .. } => {
                vec![source.as_str(), target.as_str()]
            }
            _ => vec![],
        };
        for name in names {
            if !lane_order.iter().any(|l| l == name) {
                lane_order.push(name.to_string());
            }
        }
    }

    let num_lanes = lane_order.len().max(1);

    // ── 2. Layout dimensions ────────────────────────────────────────────
    let label_margin = 20.0 * scale; // small left margin (labels go inline)
    let right_margin = 80.0 * scale;
    let top_margin = 50.0 * scale; // space for tags above
    let bottom_margin = 30.0 * scale;
    let usable_width = max_width - label_margin - right_margin;
    let usable_height = height - top_margin - bottom_margin;
    let max_lane_spacing = 100.0 * scale;
    let natural_spacing = if num_lanes > 1 {
        usable_height / (num_lanes - 1) as f32
    } else {
        0.0
    };
    let lane_spacing = natural_spacing.min(max_lane_spacing);
    let total_lane_height = if num_lanes > 1 {
        lane_spacing * (num_lanes - 1) as f32
    } else {
        0.0
    };
    let lane_top = pos.y + top_margin + (usable_height - total_lane_height) / 2.0;

    let lane_y = |name: &str| -> f32 {
        let idx = lane_order.iter().position(|l| l == name).unwrap_or(0);
        if num_lanes == 1 {
            pos.y + top_margin + usable_height / 2.0
        } else {
            lane_top + idx as f32 * lane_spacing
        }
    };

    let lane_color = |name: &str, op: f32| -> egui::Color32 {
        let idx = lane_order.iter().position(|l| l == name).unwrap_or(0);
        Theme::with_opacity(palette[idx % palette.len()], op)
    };

    // ── 3. Timeline positions ───────────────────────────────────────────
    let mut timeline_positions: Vec<f32> = Vec::new();
    let mut timeline_pos: f32 = 0.0;
    for item in &items {
        match item {
            GitGraphItem::Lane { .. } => {
                timeline_positions.push(0.0); // lanes don't advance
            }
            _ => {
                timeline_pos += 1.0;
                timeline_positions.push(timeline_pos);
            }
        }
    }
    let max_timeline = timeline_pos.max(1.0);
    let event_spacing = usable_width / max_timeline;

    let item_x = |idx: usize| -> f32 {
        let tp = timeline_positions.get(idx).copied().unwrap_or(0.0);
        pos.x + label_margin + event_spacing * tp
    };

    // ── 4. Track active state per branch ────────────────────────────────
    // Collect X positions where each branch has events (for solid line segments)
    let mut branch_events: std::collections::HashMap<String, Vec<f32>> =
        std::collections::HashMap::new();
    // Track which branches have been merged away (source of a merge)
    let mut merged_away: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Track first active X per branch (for label placement)
    let mut first_active_x: std::collections::HashMap<String, f32> =
        std::collections::HashMap::new();

    for (i, item) in items.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }
        let x = item_x(i);
        match item {
            GitGraphItem::Lane { .. } => {}
            GitGraphItem::Commit { branch, .. } | GitGraphItem::Tag { branch, .. } => {
                first_active_x.entry(branch.clone()).or_insert(x);
                branch_events.entry(branch.clone()).or_default().push(x);
            }
            GitGraphItem::Branch { source, .. } => {
                // Source gets a dot at x. Target's first position is target_x
                // (registered during rendering) — don't add x here to avoid double dots.
                first_active_x.entry(source.clone()).or_insert(x);
                branch_events.entry(source.clone()).or_default().push(x);
            }
            GitGraphItem::Merge { source, target, .. } => {
                first_active_x.entry(source.clone()).or_insert(x);
                first_active_x.entry(target.clone()).or_insert(x);
                branch_events.entry(target.clone()).or_default().push(x);
                merged_away.insert(source.clone());
            }
        }
    }

    // Sizing
    let line_width = 7.0 * scale;
    let curve_width = 6.0 * scale;
    let dot_radius = 14.0 * scale;
    let dotted_width = 5.0 * scale;

    // ── 5. Draw dotted background lanes ─────────────────────────────────
    let full_left = pos.x + label_margin;
    let full_right = pos.x + max_width - right_margin;
    for lane in &lane_order {
        let y = lane_y(lane);
        // Gray dotted line — branch "lights up" with color when it becomes active
        let color = Theme::with_opacity(theme.foreground, opacity * 0.15);
        let dash_len = 10.0 * scale;
        let gap_len = 10.0 * scale;
        let mut cx = full_left;
        while cx < full_right {
            let end = (cx + dash_len).min(full_right);
            painter.line_segment(
                [Pos2::new(cx, y), Pos2::new(end, y)],
                Stroke::new(dotted_width, color),
            );
            cx += dash_len + gap_len;
        }
    }

    // ── 6. Draw solid segments between consecutive events per branch ────
    for lane in &lane_order {
        let Some(positions) = branch_events.get(lane) else {
            continue;
        };
        let y = lane_y(lane);
        let color = lane_color(lane, opacity);

        for pair in positions.windows(2) {
            let x1 = pair[0] + dot_radius;
            let x2 = pair[1] - dot_radius;
            if x2 > x1 {
                painter.line_segment(
                    [Pos2::new(x1, y), Pos2::new(x2, y)],
                    Stroke::new(line_width, color),
                );
            }
        }

        // Extend active (non-merged) branches past their last event
        if !merged_away.contains(lane) && !positions.is_empty() {
            let last_x = *positions.last().unwrap();
            if full_right > last_x + dot_radius {
                let faded = lane_color(lane, opacity * 0.4);
                painter.line_segment(
                    [Pos2::new(last_x + dot_radius, y), Pos2::new(full_right, y)],
                    Stroke::new(line_width, faded),
                );
            }
        }
    }

    let label_font = FontId::proportional(theme.body_size * VIZ_FONT_PRIMARY_LABEL * scale);
    let msg_font = FontId::proportional(theme.body_size * VIZ_FONT_SECONDARY_LABEL * scale);

    // ── 7. Draw events (S-curves, dots, labels) ─────────────────────────
    for (i, item) in items.iter().enumerate() {
        let step = steps.get(i).copied().unwrap_or(0);
        if step > reveal_step {
            continue;
        }
        let x = item_x(i);

        match item {
            GitGraphItem::Lane { .. } => {} // already drawn as dotted line

            GitGraphItem::Commit {
                branch, message, ..
            } => {
                let y = lane_y(branch);
                let color = lane_color(branch, opacity);

                // Commit dot with dark outline
                painter.circle_filled(Pos2::new(x, y), dot_radius, color);
                let outline = Theme::with_opacity(theme.background, opacity * 0.6);
                painter.circle_stroke(
                    Pos2::new(x, y),
                    dot_radius,
                    Stroke::new(2.5 * scale, outline),
                );

                // Commit message label
                if !message.is_empty() {
                    let label_y = y - dot_radius - 14.0 * scale;
                    draw_pill_label(
                        painter,
                        message,
                        &msg_font,
                        color,
                        theme.foreground,
                        opacity,
                        Pos2::new(x, label_y),
                        scale,
                    );
                }
            }

            GitGraphItem::Branch { source, target, .. } => {
                let source_y = lane_y(source);
                let target_y = lane_y(target);
                let target_color = lane_color(target, opacity);
                let outline = Theme::with_opacity(theme.background, opacity * 0.6);

                // One dot on source lane at x (the fork point)
                let source_color = lane_color(source, opacity);
                painter.circle_filled(Pos2::new(x, source_y), dot_radius, source_color);
                painter.circle_stroke(
                    Pos2::new(x, source_y),
                    dot_radius,
                    Stroke::new(2.5 * scale, outline),
                );

                // S-curve from source dot to target lane.
                // Target end is offset RIGHT to give the S real horizontal room.
                let offset = event_spacing * 0.8;
                let target_x = x + offset;
                let mid_x = (x + target_x) / 2.0;
                let bezier = CubicBezierShape::from_points_stroke(
                    [
                        Pos2::new(x, source_y),        // P0: start at source dot
                        Pos2::new(mid_x, source_y),    // P1: horizontal on source lane
                        Pos2::new(mid_x, target_y),    // P2: horizontal on target lane
                        Pos2::new(target_x, target_y), // P3: arrive at target lane
                    ],
                    false,
                    egui::Color32::TRANSPARENT,
                    Stroke::new(curve_width, target_color),
                );
                painter.add(bezier);

                // One dot at the S-curve end on the target lane
                painter.circle_filled(Pos2::new(target_x, target_y), dot_radius, target_color);
                painter.circle_stroke(
                    Pos2::new(target_x, target_y),
                    dot_radius,
                    Stroke::new(2.5 * scale, outline),
                );

                // Register target position for solid line tracking and label placement
                first_active_x.entry(target.clone()).or_insert(target_x);
                branch_events
                    .entry(target.clone())
                    .or_default()
                    .push(target_x);
            }

            GitGraphItem::Merge {
                source,
                target,
                label,
                reveal,
                ..
            } => {
                let source_y = lane_y(source);
                let target_y = lane_y(target);
                let merge_color = lane_color(source, opacity * 0.8);

                // Dot on target at merge point
                let tc = lane_color(target, opacity);
                let outline = Theme::with_opacity(theme.background, opacity * 0.6);
                painter.circle_filled(Pos2::new(x, target_y), dot_radius, tc);
                painter.circle_stroke(
                    Pos2::new(x, target_y),
                    dot_radius,
                    Stroke::new(2.5 * scale, outline),
                );

                // * marker (WithPrev) = simultaneous event = vertical line
                // + or - marker = normal merge = S-curve
                if *reveal == VizReveal::WithPrev {
                    // Straight vertical line for simultaneous operations
                    let (y_top, y_bot) = if source_y < target_y {
                        (source_y + dot_radius, target_y - dot_radius)
                    } else {
                        (target_y + dot_radius, source_y - dot_radius)
                    };
                    painter.line_segment(
                        [Pos2::new(x, y_top), Pos2::new(x, y_bot)],
                        Stroke::new(curve_width, merge_color),
                    );
                } else {
                    // S-curve merge: starts on source lane, curves to target dot.
                    // Start is offset LEFT to give the S real horizontal room.
                    let offset = event_spacing * 0.8;
                    let start_x = x - offset;
                    let mid_x = (start_x + x) / 2.0;
                    let bezier = CubicBezierShape::from_points_stroke(
                        [
                            Pos2::new(start_x, source_y), // P0: start on source lane
                            Pos2::new(mid_x, source_y),   // P1: horizontal on source
                            Pos2::new(mid_x, target_y),   // P2: horizontal on target
                            Pos2::new(x, target_y),       // P3: end at target dot
                        ],
                        false,
                        egui::Color32::TRANSPARENT,
                        Stroke::new(curve_width, merge_color),
                    );
                    painter.add(bezier);
                }

                // Merge label
                if !label.is_empty() {
                    let label_mid_x = x + dot_radius + 8.0 * scale;
                    let label_mid_y = (source_y + target_y) / 2.0;
                    draw_pill_label(
                        painter,
                        label,
                        &msg_font,
                        merge_color,
                        theme.foreground,
                        opacity,
                        Pos2::new(label_mid_x, label_mid_y),
                        scale,
                    );
                }
            }

            GitGraphItem::Tag { branch, label, .. } => {
                let y = lane_y(branch);

                // Find the latest commit X on this branch (for positioning)
                let tag_x = branch_events
                    .get(branch)
                    .and_then(|positions| positions.last().copied())
                    .unwrap_or(x);

                let tag_color = lane_color(branch, opacity);

                // Draw tag box above the commit
                let tag_font =
                    FontId::proportional(theme.body_size * VIZ_FONT_SECONDARY_LABEL * scale);
                let text_color = Theme::with_opacity(theme.foreground, opacity);
                let galley = painter.layout_no_wrap(label.clone(), tag_font, text_color);
                let pad_h = 8.0 * scale;
                let pad_v = 5.0 * scale;
                let box_w = galley.rect.width() + pad_h * 2.0;
                let box_h = galley.rect.height() + pad_v * 2.0;
                let box_y = y - dot_radius - box_h - 16.0 * scale;
                let box_rect = egui::Rect::from_min_size(
                    Pos2::new(tag_x - box_w / 2.0, box_y),
                    egui::vec2(box_w, box_h),
                );

                // Box with border
                let bg = Theme::with_opacity(tag_color, opacity * 0.2);
                let border = Theme::with_opacity(tag_color, opacity * 0.6);
                painter.rect_filled(box_rect, 4.0 * scale, bg);
                painter.rect_stroke(
                    box_rect,
                    4.0 * scale,
                    Stroke::new(2.0 * scale, border),
                    egui::StrokeKind::Outside,
                );
                painter.galley(
                    Pos2::new(box_rect.left() + pad_h, box_rect.top() + pad_v),
                    galley,
                    text_color,
                );

                // Arrow from box to commit dot
                let arrow_start = Pos2::new(tag_x, box_rect.bottom());
                let arrow_end = Pos2::new(tag_x, y - dot_radius);
                painter.line_segment([arrow_start, arrow_end], Stroke::new(2.0 * scale, border));
            }
        }
    }

    // ── 8. Draw branch name labels (at first active position) ───────────
    for lane in &lane_order {
        let Some(&first_x) = first_active_x.get(lane) else {
            continue; // lane was declared but never used
        };
        let y = lane_y(lane);
        let color = lane_color(lane, opacity);
        let galley = painter.layout_no_wrap(lane.clone(), label_font.clone(), color);
        // Place label to the left of the first dot
        let text_x = first_x - galley.rect.width() - dot_radius - 8.0 * scale;
        let text_y = y - galley.rect.height() / 2.0;
        painter.galley(Pos2::new(text_x, text_y), galley, color);
    }

    height
}

/// Draw a label with a pill-shaped colored background centered at the given position.
#[allow(clippy::too_many_arguments)]
fn draw_pill_label(
    painter: &egui::Painter,
    text: &str,
    font: &FontId,
    bg_color: egui::Color32,
    text_color: egui::Color32,
    opacity: f32,
    center: Pos2,
    scale: f32,
) {
    let pad_h = 8.0 * scale;
    let pad_v = 4.0 * scale;
    let label_text_color = Theme::with_opacity(text_color, opacity);
    let label_bg = Theme::with_opacity(bg_color, opacity * 0.85);

    let galley = painter.layout_no_wrap(text.to_string(), font.clone(), label_text_color);
    let label_w = galley.rect.width() + pad_h * 2.0;
    let label_h = galley.rect.height() + pad_v * 2.0;
    let label_rect = egui::Rect::from_center_size(center, egui::vec2(label_w, label_h));

    painter.rect_filled(label_rect, label_h / 2.0, label_bg);
    painter.galley(
        Pos2::new(label_rect.left() + pad_h, label_rect.top() + pad_v),
        galley,
        label_text_color,
    );
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lane() {
        let content = "- lane main\n- lane develop";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[0], GitGraphItem::Lane { name } if name == "main"));
        assert!(matches!(&items[1], GitGraphItem::Lane { name } if name == "develop"));
    }

    #[test]
    fn test_parse_branch_arrow_syntax() {
        let content = "- branch main -> develop";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 1);
        match &items[0] {
            GitGraphItem::Branch { source, target, .. } => {
                assert_eq!(source, "main");
                assert_eq!(target, "develop");
            }
            _ => panic!("Expected Branch"),
        }
    }

    #[test]
    fn test_parse_commit_with_message() {
        let content = "- commit develop: \"Initial setup\"";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 1);
        match &items[0] {
            GitGraphItem::Commit {
                branch, message, ..
            } => {
                assert_eq!(branch, "develop");
                assert_eq!(message, "Initial setup");
            }
            _ => panic!("Expected Commit"),
        }
    }

    #[test]
    fn test_parse_commit_without_message() {
        let content = "- commit feature";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 1);
        match &items[0] {
            GitGraphItem::Commit {
                branch, message, ..
            } => {
                assert_eq!(branch, "feature");
                assert!(message.is_empty());
            }
            _ => panic!("Expected Commit"),
        }
    }

    #[test]
    fn test_parse_merge_with_label() {
        let content = "- merge feature -> develop: \"PR #42\"";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 1);
        match &items[0] {
            GitGraphItem::Merge {
                source,
                target,
                label,
                ..
            } => {
                assert_eq!(source, "feature");
                assert_eq!(target, "develop");
                assert_eq!(label, "PR #42");
            }
            _ => panic!("Expected Merge"),
        }
    }

    #[test]
    fn test_parse_tag() {
        let content = "- tag main: \"v1.0\"";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 1);
        match &items[0] {
            GitGraphItem::Tag { branch, label, .. } => {
                assert_eq!(branch, "main");
                assert_eq!(label, "v1.0");
            }
            _ => panic!("Expected Tag"),
        }
    }

    #[test]
    fn test_parse_reveal_markers() {
        let content = "- lane main\n- commit main\n+ branch main -> develop\n* commit develop";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 4);
        // Lane is always static (override)
        match &items[2] {
            GitGraphItem::Branch { reveal, .. } => assert_eq!(*reveal, VizReveal::NextStep),
            _ => panic!(),
        }
        match &items[3] {
            GitGraphItem::Commit { reveal, .. } => assert_eq!(*reveal, VizReveal::WithPrev),
            _ => panic!(),
        }
    }

    #[test]
    fn test_parse_full_gitflow() {
        let content = "\
- lane main
- lane hotfix
- lane release
- lane develop
- lane feature
- commit main
- branch main -> develop
+ branch develop -> feature
+ commit feature
+ commit feature
+ merge feature -> develop
+ branch develop -> release
+ commit release
+ merge release -> main: \"v1.0\"
* merge release -> develop
+ tag main: \"v1.0\"";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 16);
    }

    #[test]
    fn test_parse_ignores_comments_and_blanks() {
        let content = "# Comment\n\n- lane main\n# Another\n- commit main";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 2);
    }
}
