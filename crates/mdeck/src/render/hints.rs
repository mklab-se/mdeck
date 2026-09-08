//! Geometry hints from renderers to the particle field.
//!
//! On slides whose content fills the frame (charts, diagrams, images), the
//! Ember field serves what is drawn instead of decorating around it: embers
//! rise off bar tops, runners follow a line series or a routed edge, sparks
//! circle a pie, dust keeps clear of the content box. Renderers publish the
//! geometry they draw through [`push`]; the field reads it with [`take`] on
//! the next frame. Publishing is a no-op unless the Ember field switched it
//! on, so every other theme pays nothing.

use std::hash::{Hash, Hasher};

use eframe::egui::{self, Pos2, Rect};

/// One piece of drawn geometry, in points.
#[derive(Clone, Debug)]
pub enum Hint {
    /// A filled bar (vertical or horizontal).
    Bar(Rect),
    /// A polyline: a line series, a routed edge, a timeline axis. Direction
    /// matters: runners travel from the first point to the last.
    Path(Vec<Pos2>),
    /// A circle: a pie, a donut, a radar ring, a Venn set.
    Circle { center: Pos2, radius: f32 },
    /// A marked point: a scatter dot, a timeline event.
    Point(Pos2),
    /// A box the field must stay out of: the content area, a node, a card.
    Frame(Rect),
}

fn store_id() -> egui::Id {
    egui::Id::new("ember-hints")
}

fn enabled_id() -> egui::Id {
    egui::Id::new("ember-hints-enabled")
}

/// Turn collection on or off (the Ember field turns it on every frame it draws).
pub fn set_enabled(ctx: &egui::Context, on: bool) {
    ctx.data_mut(|d| d.insert_temp(enabled_id(), on));
}

fn enabled(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(enabled_id()).unwrap_or(false))
}

/// Publish a hint for the field. Cheap when collection is off.
pub fn push(ctx: &egui::Context, hint: Hint) {
    if !enabled(ctx) {
        return;
    }
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Vec<Hint>>(store_id())
            .push(hint);
    });
}

/// Take everything published since the last call.
pub fn take(ctx: &egui::Context) -> Vec<Hint> {
    ctx.data_mut(|d| std::mem::take(d.get_temp_mut_or_default::<Vec<Hint>>(store_id())))
}

/// A stable fingerprint of a hint set, quantised to whole points so animation
/// jitter and reveal easing do not read as new geometry every frame.
pub fn fingerprint(hints: &[Hint]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let q = |v: f32| (v / 4.0).round() as i32;
    for hint in hints {
        match hint {
            Hint::Bar(r) | Hint::Frame(r) => {
                (matches!(hint, Hint::Bar(_)) as u8).hash(&mut h);
                (q(r.left()), q(r.top()), q(r.right()), q(r.bottom())).hash(&mut h);
            }
            Hint::Path(pts) => {
                2u8.hash(&mut h);
                for p in pts {
                    (q(p.x), q(p.y)).hash(&mut h);
                }
            }
            Hint::Circle { center, radius } => {
                3u8.hash(&mut h);
                (q(center.x), q(center.y), q(*radius)).hash(&mut h);
            }
            Hint::Point(p) => {
                4u8.hash(&mut h);
                (q(p.x), q(p.y)).hash(&mut h);
            }
        }
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_collect_only_when_enabled_and_are_taken_once() {
        let ctx = egui::Context::default();
        push(&ctx, Hint::Point(Pos2::new(1.0, 1.0)));
        assert!(take(&ctx).is_empty(), "disabled by default");
        set_enabled(&ctx, true);
        push(&ctx, Hint::Point(Pos2::new(1.0, 1.0)));
        push(
            &ctx,
            Hint::Frame(Rect::from_min_size(Pos2::ZERO, egui::vec2(10.0, 10.0))),
        );
        assert_eq!(take(&ctx).len(), 2);
        assert!(take(&ctx).is_empty(), "taken once");
    }

    #[test]
    fn fingerprint_ignores_sub_point_jitter() {
        let a = vec![Hint::Bar(Rect::from_min_size(
            Pos2::new(10.0, 10.0),
            egui::vec2(50.0, 100.0),
        ))];
        let b = vec![Hint::Bar(Rect::from_min_size(
            Pos2::new(10.4, 10.2),
            egui::vec2(50.0, 100.0),
        ))];
        let c = vec![Hint::Bar(Rect::from_min_size(
            Pos2::new(10.0, 60.0),
            egui::vec2(50.0, 100.0),
        ))];
        assert_eq!(fingerprint(&a), fingerprint(&b));
        assert_ne!(fingerprint(&a), fingerprint(&c));
    }
}
