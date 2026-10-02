//! Geometry hints from renderers to the particle field.
//!
//! On slides whose content fills the frame (charts, diagrams, images), the
//! Ember field serves what is drawn instead of decorating around it: embers
//! rise off bar tops, runners follow a line series or a routed edge, sparks
//! circle a pie, dust keeps clear of the content box. Renderers publish the
//! geometry they draw through [`push`]; the field reads it with [`take`] on
//! the next frame. Publishing is a no-op unless an engine switched it on
//! this frame, so a frame without one pays nothing and nothing piles up.

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
    /// A heading as laid out, at the place it settles: an engine that forms
    /// titles itself (the thermal cold opening) draws it from the glyphs.
    Text {
        galley: std::sync::Arc<egui::Galley>,
        pos: Pos2,
        /// The slide it belongs to (during a transition two slides draw).
        slide: usize,
    },
}

fn store_id() -> egui::Id {
    egui::Id::new("ember-hints")
}

fn enabled_id() -> egui::Id {
    egui::Id::new("ember-hints-enabled")
}

/// Turn collection on or off for the current frame. An engine turns it
/// on every frame it draws; the switch lapses by itself at the next frame,
/// so a frame no engine draws (the plain engine, the overview) collects
/// nothing (D15).
pub fn set_enabled(ctx: &egui::Context, on: bool) {
    let frame = on.then(|| ctx.cumulative_frame_nr());
    ctx.data_mut(|d| d.insert_temp(enabled_id(), frame));
}

/// Whether collection is on this frame.
pub fn enabled(ctx: &egui::Context) -> bool {
    let on = ctx.data(|d| d.get_temp::<Option<u64>>(enabled_id()).flatten());
    on == Some(ctx.cumulative_frame_nr())
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

/// Stop collecting and drop whatever was published.
pub fn clear(ctx: &egui::Context) {
    set_enabled(ctx, false);
    let _ = take(ctx);
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

    /// D15: collection switched on by an engine frame stays on for that
    /// frame only. A frame no engine draws (the plain engine, overview)
    /// must not keep collecting hints nobody takes.
    #[test]
    fn collection_lapses_on_a_frame_no_engine_turns_it_on() {
        let ctx = egui::Context::default();
        let frame = |f: &dyn Fn(&egui::Ui)| {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| f(ui));
            out.textures_delta.clear();
        };
        frame(&|ui| {
            set_enabled(ui.ctx(), true);
            push(ui.ctx(), Hint::Point(Pos2::new(1.0, 1.0)));
        });
        assert_eq!(take(&ctx).len(), 1, "collected on the engine's frame");
        for _ in 0..3 {
            frame(&|ui| {
                assert!(!enabled(ui.ctx()), "no engine drew this frame");
                push(ui.ctx(), Hint::Point(Pos2::new(1.0, 1.0)));
            });
        }
        assert!(take(&ctx).is_empty(), "nothing piles up without an engine");
    }

    #[test]
    fn clear_drops_what_was_published() {
        let ctx = egui::Context::default();
        set_enabled(&ctx, true);
        push(&ctx, Hint::Point(Pos2::new(1.0, 1.0)));
        clear(&ctx);
        assert!(!enabled(&ctx));
        assert!(take(&ctx).is_empty());
    }
}
