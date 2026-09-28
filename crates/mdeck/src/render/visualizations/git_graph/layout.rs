//! Where a gitgraph's lanes and events go, and which branches are active at
//! the current reveal step. Pure geometry: no painting.

use std::collections::{HashMap, HashSet};

use eframe::egui::Pos2;

use super::parse::GitGraphItem;

/// Lane names in vertical order: declared lanes first, then any branch an
/// event mentions that was not declared.
pub(super) fn lane_order(items: &[GitGraphItem]) -> Vec<String> {
    let mut lanes: Vec<String> = Vec::new();
    for item in items {
        if let GitGraphItem::Lane { name } = item
            && !lanes.contains(name)
        {
            lanes.push(name.clone());
        }
    }
    for item in items {
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
            if !lanes.iter().any(|l| l == name) {
                lanes.push(name.to_string());
            }
        }
    }
    lanes
}

/// Position of each item along the timeline: events advance it by one,
/// lane declarations do not (and sit at 0).
fn timeline_positions(items: &[GitGraphItem]) -> Vec<f32> {
    let mut positions = Vec::with_capacity(items.len());
    let mut timeline_pos: f32 = 0.0;
    for item in items {
        match item {
            GitGraphItem::Lane { .. } => positions.push(0.0),
            _ => {
                timeline_pos += 1.0;
                positions.push(timeline_pos);
            }
        }
    }
    positions
}

/// The gitgraph's geometry.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct GitLayout {
    pub lanes: Vec<String>,
    lane_top: f32,
    lane_spacing: f32,
    /// Y of the only lane when there is just one.
    single_lane_y: f32,
    timeline: Vec<f32>,
    /// Where the timeline starts (right of the branch names) and ends.
    pub left: f32,
    pub right: f32,
    /// Horizontal distance between consecutive events.
    pub event_spacing: f32,
}

impl GitLayout {
    pub fn new(items: &[GitGraphItem], pos: Pos2, max_width: f32, height: f32, scale: f32) -> Self {
        let lanes = lane_order(items);
        let num_lanes = lanes.len().max(1);
        let label_margin = 120.0 * scale; // room for branch names on the left
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

        let timeline = timeline_positions(items);
        let max_timeline = timeline.iter().copied().fold(0.0f32, f32::max).max(1.0);
        Self {
            lanes,
            lane_top: pos.y + top_margin + (usable_height - total_lane_height) / 2.0,
            lane_spacing,
            single_lane_y: pos.y + top_margin + usable_height / 2.0,
            timeline,
            left: pos.x + label_margin,
            right: pos.x + max_width - right_margin,
            event_spacing: usable_width / max_timeline,
        }
    }

    /// Index of a lane (the first lane when unknown), which also picks its
    /// colour.
    pub fn lane_index(&self, name: &str) -> usize {
        self.lanes.iter().position(|l| l == name).unwrap_or(0)
    }

    pub fn lane_y(&self, name: &str) -> f32 {
        if self.lanes.len().max(1) == 1 {
            self.single_lane_y
        } else {
            self.lane_top + self.lane_index(name) as f32 * self.lane_spacing
        }
    }

    /// X of item `idx` on the timeline.
    pub fn item_x(&self, idx: usize) -> f32 {
        let tp = self.timeline.get(idx).copied().unwrap_or(0.0);
        self.left + self.event_spacing * tp
    }

    /// X of the next event on `branch` after item `after_idx`. Searches all
    /// events (not just visible ones) so positions stay stable across reveal
    /// steps.
    fn next_event_on_branch(
        &self,
        items: &[GitGraphItem],
        branch: &str,
        after_idx: usize,
    ) -> Option<f32> {
        for (j, item) in items.iter().enumerate().skip(after_idx + 1) {
            let is_on_branch = match item {
                GitGraphItem::Commit { branch: b, .. } | GitGraphItem::Tag { branch: b, .. } => {
                    b == branch
                }
                GitGraphItem::Branch { source, .. } => source == branch,
                GitGraphItem::Merge { target, .. } => target == branch,
                _ => false,
            };
            if is_on_branch {
                return Some(self.item_x(j));
            }
        }
        None
    }
}

/// What the visible events do to each branch.
#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct Activity {
    /// X of every visible event per branch, in order; solid line segments
    /// join them.
    pub events: HashMap<String, Vec<f32>>,
    /// Branches merged into another (their line stops at the merge).
    pub merged_away: HashSet<String>,
    /// Branches with at least one visible event (their names are shown).
    pub active: HashSet<String>,
}

impl Activity {
    /// Follow the events revealed so far (`steps[i] <= reveal_step`).
    pub fn new(
        items: &[GitGraphItem],
        steps: &[usize],
        reveal_step: usize,
        layout: &GitLayout,
    ) -> Self {
        let mut activity = Activity::default();
        for (i, item) in items.iter().enumerate() {
            if steps.get(i).copied().unwrap_or(0) > reveal_step {
                continue;
            }
            let x = layout.item_x(i);
            match item {
                GitGraphItem::Lane { .. } => {}
                GitGraphItem::Commit { branch, .. } | GitGraphItem::Tag { branch, .. } => {
                    activity.event(branch, x);
                }
                GitGraphItem::Branch { source, target, .. } => {
                    activity.event(source, x);
                    // The new branch starts at its next event
                    let target_next_x = layout
                        .next_event_on_branch(items, target, i)
                        .unwrap_or(x + layout.event_spacing);
                    activity.event(target, target_next_x);
                }
                GitGraphItem::Merge { source, target, .. } => {
                    activity.active.insert(source.clone());
                    activity.event(target, x);
                    activity.merged_away.insert(source.clone());
                }
            }
        }
        activity
    }

    fn event(&mut self, branch: &str, x: f32) {
        self.active.insert(branch.to_string());
        self.events.entry(branch.to_string()).or_default().push(x);
    }

    /// X of the last visible event on `branch`.
    pub fn last_x(&self, branch: &str) -> Option<f32> {
        self.events.get(branch).and_then(|xs| xs.last().copied())
    }

    /// X of the first visible event on `branch` after `x`.
    pub fn next_x(&self, branch: &str, x: f32) -> Option<f32> {
        self.events
            .get(branch)
            .and_then(|xs| xs.iter().find(|&&px| px > x).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse::parse_gitgraph;
    use super::*;

    const FLOW: &str = "- lane main\n- lane develop\n- commit main: \"init\"\n\
        - branch main -> develop\n- commit develop: \"work\"\n\
        - merge develop -> main: \"v1\"\n- tag main: \"v1.0\"\n- commit feature";

    #[test]
    fn test_lane_order_declared_first() {
        let items = parse_gitgraph(FLOW);
        assert_eq!(lane_order(&items), vec!["main", "develop", "feature"]);
    }

    #[test]
    fn test_layout_spaces_lanes_and_events() {
        let items = parse_gitgraph(FLOW);
        let l = GitLayout::new(&items, Pos2::new(0.0, 0.0), 1000.0, 500.0, 1.0);
        assert_eq!(l.left, 120.0);
        assert_eq!(l.right, 920.0);
        // Six events share the 800 px timeline
        assert_eq!(l.event_spacing, 800.0 / 6.0);
        assert_eq!(l.item_x(0), 120.0);
        assert_eq!(l.item_x(2), 120.0 + 800.0 / 6.0);
        // Three lanes at the maximum spacing, centred in the usable height
        assert_eq!(l.lane_y("develop") - l.lane_y("main"), 100.0);
        assert_eq!(l.lane_y("main"), 50.0 + (420.0 - 200.0) / 2.0);
        assert_eq!(l.lane_y("unknown"), l.lane_y("main"));
    }

    #[test]
    fn test_single_lane_is_centred() {
        let items = parse_gitgraph("- commit main\n- commit main");
        let l = GitLayout::new(&items, Pos2::new(0.0, 0.0), 1000.0, 500.0, 1.0);
        assert_eq!(l.lane_y("main"), 50.0 + 210.0);
    }

    #[test]
    fn test_activity_follows_reveal() {
        let items = parse_gitgraph(FLOW);
        let l = GitLayout::new(&items, Pos2::new(0.0, 0.0), 1000.0, 500.0, 1.0);
        let steps = vec![0; items.len()];
        let a = Activity::new(&items, &steps, 0, &l);
        // The fork reaches develop at develop's next commit, which is also
        // an event of its own
        assert_eq!(a.events["develop"], vec![l.item_x(4), l.item_x(4)]);
        assert_eq!(
            a.events["main"],
            vec![l.item_x(2), l.item_x(3), l.item_x(5), l.item_x(6)]
        );
        assert!(a.merged_away.contains("develop"));
        assert!(a.active.contains("feature"));
        assert_eq!(a.last_x("main"), Some(l.item_x(6)));
        assert_eq!(a.next_x("main", l.item_x(3)), Some(l.item_x(5)));

        // Nothing past step 0 is visible when later items are revealed later
        let steps: Vec<usize> = (0..items.len()).collect();
        let a = Activity::new(&items, &steps, 2, &l);
        assert!(a.merged_away.is_empty());
        assert!(!a.active.contains("develop"));
    }
}
