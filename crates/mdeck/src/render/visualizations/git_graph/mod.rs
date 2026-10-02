//! Gitgraph: branch lanes with commits, forks, merges and tags along a
//! timeline.

mod layout;
mod parse;
pub use parse::check;

use eframe::egui::{self, Color32, FontId, Pos2, Stroke};
use eframe::epaint::CubicBezierShape;

use crate::theme::Theme;

use super::{VIZ_FONT_PRIMARY_LABEL, VIZ_FONT_SECONDARY_LABEL, VizCtx, VizReveal, assign_steps};
use layout::{Activity, GitLayout};
use parse::{GitGraphItem, parse_gitgraph};

pub fn draw_gitgraph(
    cx: &VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let scale = cx.scale;
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

    let layout = GitLayout::new(&items, pos, max_width, height, scale);
    let activity = Activity::new(&items, &steps, cx.reveal_step, &layout);
    let paint = GitPaint {
        cx: *cx,
        painter: cx.ui.painter(),
        layout: &layout,
        activity: &activity,
        palette: cx.theme.edge_palette(),
        line_width: 7.0 * scale,
        curve_width: 6.0 * scale,
        dot_radius: 14.0 * scale,
        outline: Theme::with_opacity(cx.theme.background, cx.opacity * 0.6),
        msg_font: cx.font(VIZ_FONT_SECONDARY_LABEL),
    };

    paint.dotted_lanes();
    paint.solid_segments();
    for (i, item) in items.iter().enumerate() {
        if steps.get(i).copied().unwrap_or(0) > cx.reveal_step {
            continue;
        }
        let x = layout.item_x(i);
        match item {
            GitGraphItem::Lane { .. } => {} // already drawn as dotted line
            GitGraphItem::Commit {
                branch, message, ..
            } => paint.commit(x, branch, message),
            GitGraphItem::Branch { source, target, .. } => paint.branch(x, source, target),
            GitGraphItem::Merge {
                source,
                target,
                label,
                ..
            } => paint.merge(x, source, target, label),
            GitGraphItem::Tag { branch, label, .. } => paint.tag(x, branch, label),
        }
    }
    paint.lane_names(pos.x);

    height
}

/// Everything the gitgraph's paint steps share.
struct GitPaint<'a> {
    cx: VizCtx<'a>,
    painter: &'a egui::Painter,
    layout: &'a GitLayout,
    activity: &'a Activity,
    palette: [Color32; crate::theme::EDGE_PALETTE_LEN],
    line_width: f32,
    curve_width: f32,
    dot_radius: f32,
    /// Dark ring around every dot.
    outline: Color32,
    msg_font: FontId,
}

impl GitPaint<'_> {
    fn lane_color(&self, name: &str, op: f32) -> Color32 {
        let idx = self.layout.lane_index(name);
        Theme::with_opacity(self.palette[idx % self.palette.len()], op)
    }

    /// A dot with a dark outline.
    fn dot(&self, center: Pos2, color: Color32) {
        self.painter.circle_filled(center, self.dot_radius, color);
        self.painter.circle_stroke(
            center,
            self.dot_radius,
            Stroke::new(2.5 * self.cx.scale, self.outline),
        );
    }

    /// An S-curve from `from` to `to`, flat at both ends.
    fn s_curve(&self, from: Pos2, to: Pos2, color: Color32) {
        let mid_x = (from.x + to.x) / 2.0;
        self.painter.add(CubicBezierShape::from_points_stroke(
            [from, Pos2::new(mid_x, from.y), Pos2::new(mid_x, to.y), to],
            false,
            Color32::TRANSPARENT,
            Stroke::new(self.curve_width, color),
        ));
    }

    /// Every lane as a grey dashed line; a branch "lights up" with colour
    /// when it becomes active.
    fn dotted_lanes(&self) {
        let scale = self.cx.scale;
        let (full_left, full_right) = (self.layout.left, self.layout.right);
        let color = self.cx.fg(0.15);
        let dash_len = 10.0 * scale;
        let gap_len = 10.0 * scale;
        for lane in &self.layout.lanes {
            let y = self.layout.lane_y(lane);
            let mut x = full_left;
            while x < full_right {
                let end = (x + dash_len).min(full_right);
                self.painter.line_segment(
                    [Pos2::new(x, y), Pos2::new(end, y)],
                    Stroke::new(5.0 * scale, color),
                );
                x += dash_len + gap_len;
            }
        }
    }

    /// Solid lines between consecutive events of each branch, and a faded
    /// one on from the last event of a branch that is still open.
    fn solid_segments(&self) {
        let dot_radius = self.dot_radius;
        let full_right = self.layout.right;
        for lane in &self.layout.lanes {
            let Some(positions) = self.activity.events.get(lane) else {
                continue;
            };
            let y = self.layout.lane_y(lane);
            let color = self.lane_color(lane, self.cx.opacity);

            for pair in positions.windows(2) {
                let x1 = pair[0] + dot_radius;
                let x2 = pair[1] - dot_radius;
                if x2 > x1 {
                    self.painter.line_segment(
                        [Pos2::new(x1, y), Pos2::new(x2, y)],
                        Stroke::new(self.line_width, color),
                    );
                }
            }

            if let Some(&last_x) = positions.last()
                && !self.activity.merged_away.contains(lane)
                && full_right > last_x + dot_radius
            {
                let faded = self.lane_color(lane, self.cx.opacity * 0.4);
                self.painter.line_segment(
                    [Pos2::new(last_x + dot_radius, y), Pos2::new(full_right, y)],
                    Stroke::new(self.line_width, faded),
                );
            }
        }
    }

    fn commit(&self, x: f32, branch: &str, message: &str) {
        let y = self.layout.lane_y(branch);
        let color = self.lane_color(branch, self.cx.opacity);
        self.dot(Pos2::new(x, y), color);
        if !message.is_empty() {
            let label_y = y - self.dot_radius - 14.0 * self.cx.scale;
            self.pill(message, color, Pos2::new(x, label_y));
        }
    }

    /// A fork: a dot on the source lane and an S-curve to the new branch's
    /// next event, which gives the curve real horizontal distance.
    fn branch(&self, x: f32, source: &str, target: &str) {
        let opacity = self.cx.opacity;
        let source_y = self.layout.lane_y(source);
        let target_y = self.layout.lane_y(target);
        let target_color = self.lane_color(target, opacity);
        self.dot(Pos2::new(x, source_y), self.lane_color(source, opacity));

        let target_next_x = self
            .activity
            .next_x(target, x)
            .unwrap_or(x + self.layout.event_spacing);
        let end = Pos2::new(target_next_x, target_y);
        self.s_curve(Pos2::new(x, source_y), end, target_color);
        self.dot(end, target_color);
    }

    /// A merge: a dot on the target and a curve from the source's last event.
    fn merge(&self, x: f32, source: &str, target: &str, label: &str) {
        let opacity = self.cx.opacity;
        let source_y = self.layout.lane_y(source);
        let target_y = self.layout.lane_y(target);
        let merge_color = self.lane_color(source, opacity * 0.8);
        self.dot(Pos2::new(x, target_y), self.lane_color(target, opacity));

        let source_last_x = self
            .activity
            .last_x(source)
            .unwrap_or(x - self.layout.event_spacing);
        self.s_curve(
            Pos2::new(source_last_x, source_y),
            Pos2::new(x, target_y),
            merge_color,
        );

        // Label on the curve's midpoint
        if !label.is_empty() {
            let mid = Pos2::new((source_last_x + x) / 2.0, (source_y + target_y) / 2.0);
            self.pill(label, merge_color, mid);
        }
    }

    /// A boxed tag above the branch's latest commit with an arrow down to it.
    fn tag(&self, x: f32, branch: &str, label: &str) {
        let scale = self.cx.scale;
        let opacity = self.cx.opacity;
        let y = self.layout.lane_y(branch);
        let tag_x = self.activity.last_x(branch).unwrap_or(x);
        let tag_color = self.lane_color(branch, opacity);

        let text_color = self.cx.fg(1.0);
        let galley = self.painter.layout_no_wrap(
            label.to_string(),
            self.cx.font(VIZ_FONT_SECONDARY_LABEL),
            text_color,
        );
        let pad_h = 8.0 * scale;
        let pad_v = 5.0 * scale;
        let box_w = galley.rect.width() + pad_h * 2.0;
        let box_h = galley.rect.height() + pad_v * 2.0;
        let box_y = y - self.dot_radius - box_h - 16.0 * scale;
        let box_rect = egui::Rect::from_min_size(
            Pos2::new(tag_x - box_w / 2.0, box_y),
            egui::vec2(box_w, box_h),
        );

        let bg = Theme::with_opacity(tag_color, opacity * 0.2);
        let border = Theme::with_opacity(tag_color, opacity * 0.6);
        self.painter.rect_filled(box_rect, 4.0 * scale, bg);
        self.painter.rect_stroke(
            box_rect,
            4.0 * scale,
            Stroke::new(2.0 * scale, border),
            egui::StrokeKind::Outside,
        );
        self.painter.galley(
            Pos2::new(box_rect.left() + pad_h, box_rect.top() + pad_v),
            galley,
            text_color,
        );

        let arrow_start = Pos2::new(tag_x, box_rect.bottom());
        let arrow_end = Pos2::new(tag_x, y - self.dot_radius);
        self.painter
            .line_segment([arrow_start, arrow_end], Stroke::new(2.0 * scale, border));
    }

    /// Branch names right-aligned left of the timeline, for branches that
    /// have become active (declared but unused lanes stay unnamed).
    fn lane_names(&self, left: f32) {
        let scale = self.cx.scale;
        let label_font = self.cx.font(VIZ_FONT_PRIMARY_LABEL);
        for lane in &self.layout.lanes {
            if !self.activity.active.contains(lane) {
                continue;
            }
            let y = self.layout.lane_y(lane);
            let color = self.lane_color(lane, self.cx.opacity);
            let galley = self
                .painter
                .layout_no_wrap(lane.clone(), label_font.clone(), color);
            let text_x = self.layout.left - galley.rect.width() - 8.0 * scale;
            let text_y = y - galley.rect.height() / 2.0;
            self.painter.galley(
                Pos2::new(text_x.max(left + 4.0 * scale), text_y),
                galley,
                color,
            );
        }
    }

    /// A label on a pill of `bg_color`, centred at `center`.
    fn pill(&self, text: &str, bg_color: Color32, center: Pos2) {
        let scale = self.cx.scale;
        let opacity = self.cx.opacity;
        let pad_h = 8.0 * scale;
        let pad_v = 4.0 * scale;
        let label_text_color = Theme::with_opacity(self.cx.theme.foreground, opacity);
        let label_bg = Theme::with_opacity(bg_color, opacity * 0.85);

        let galley =
            self.painter
                .layout_no_wrap(text.to_string(), self.msg_font.clone(), label_text_color);
        let label_w = galley.rect.width() + pad_h * 2.0;
        let label_h = galley.rect.height() + pad_v * 2.0;
        let label_rect = egui::Rect::from_center_size(center, egui::vec2(label_w, label_h));

        self.painter
            .rect_filled(label_rect, label_h / 2.0, label_bg);
        self.painter.galley(
            Pos2::new(label_rect.left() + pad_h, label_rect.top() + pad_v),
            galley,
            label_text_color,
        );
    }
}
