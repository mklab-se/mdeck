//! Entry motion: how long a slide has been on screen, and how far each
//! element of its copy has come in by the arrangement's `entry`.

use eframe::egui;

use crate::theme::arrangement::{Entry, Motion, RevealMotion};

pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[derive(Clone, Copy)]
struct Entered {
    at: f64,
    /// The frame the slide was last drawn in.
    last_frame: u64,
}

/// Seconds since slide `index` was (re)entered. A slide not drawn in the
/// previous frame counts as re-entered, so navigating back replays the
/// entry. Frames, not seconds: an idle window can go seconds between frames
/// without the slide leaving the screen. `hold` restarts the clock (while an
/// intro holds the copy back).
pub fn entry_age(ui: &egui::Ui, index: usize, animate: bool, hold: bool) -> f32 {
    if !animate {
        return 10.0;
    }
    let now = ui.input(|i| i.time);
    let frame = ui.ctx().cumulative_frame_nr();
    let id = egui::Id::new(("design-entry", index));
    let age = ui.ctx().data_mut(|d| {
        let e = d.get_temp_mut_or_insert_with(id, || Entered {
            at: now,
            last_frame: frame,
        });
        if hold || frame > e.last_frame + 1 {
            e.at = now;
        }
        e.last_frame = frame;
        (now - e.at) as f32
    });
    if age < 2.5 {
        ui.ctx().request_repaint();
    }
    age
}

/// How far the `nth` element has come in (0..1), `age` seconds after the
/// slide was entered.
pub fn entry_progress(entry: &Entry, age: f32, nth: usize) -> f32 {
    let dur = (entry.duration_ms / 1000.0).max(0.001);
    match entry.kind {
        Motion::None => 1.0,
        Motion::Fade | Motion::Rise => ease_out(age / dur),
        Motion::Stagger => ease_out((age - entry.step_ms / 1000.0 * nth as f32) / dur),
    }
}

/// Whether the entry moves elements up as they come in.
pub fn rises(entry: &Entry) -> bool {
    matches!(entry.kind, Motion::Rise | Motion::Stagger)
}

/// How far an element of reveal `step` has come in: an element revealed
/// with Next settles over `reveal-ms` from `reveal_age` seconds ago (the
/// app clears that time on Back, so nothing that stays animates again);
/// anything else follows the entry.
pub fn piece_progress(
    entry: &Entry,
    step: usize,
    reveal_step: usize,
    age: f32,
    reveal_age: Option<f32>,
    nth: usize,
) -> f32 {
    let dur = (entry.reveal_ms / 1000.0).max(0.001);
    match reveal_age {
        Some(r) if step > 0 && step == reveal_step && r < dur * 2.0 => ease_out(r / dur),
        _ => entry_progress(entry, age, nth),
    }
}

/// The offset of an element at progress `p` revealed by Next (`revealed`)
/// or entering, px at scale 1.
pub fn offset(entry: &Entry, p: f32, revealed: bool) -> egui::Vec2 {
    let left = 1.0 - p;
    if revealed {
        match entry.reveal {
            RevealMotion::Slide => egui::vec2(left * 24.0, 0.0),
            RevealMotion::Rise => egui::vec2(0.0, left * 14.0),
            RevealMotion::Fade => egui::Vec2::ZERO,
        }
    } else if rises(entry) {
        egui::vec2(0.0, left * entry.rise)
    } else {
        egui::Vec2::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stagger() -> Entry {
        Entry {
            kind: Motion::Stagger,
            step_ms: 120.0,
            duration_ms: 650.0,
            rise: 14.0,
            reveal: RevealMotion::Rise,
            reveal_ms: 550.0,
        }
    }

    #[test]
    fn stagger_orders_elements_and_completes() {
        let e = stagger();
        assert!(entry_progress(&e, 0.0, 0) < 0.01);
        assert!(entry_progress(&e, 0.3, 0) > entry_progress(&e, 0.3, 1));
        assert!((entry_progress(&e, 5.0, 3) - 1.0).abs() < 1e-6);
        let none = Entry::default();
        assert_eq!(entry_progress(&none, 0.0, 4), 1.0);
    }

    #[test]
    fn only_a_bullet_revealed_with_next_moves() {
        let e = stagger();
        assert!(piece_progress(&e, 3, 3, 20.0, Some(0.1), 4) < 0.5);
        assert!(piece_progress(&e, 3, 3, 9.5, Some(0.1), 4) < 0.5);
        assert_eq!(piece_progress(&e, 2, 3, 20.0, Some(0.1), 3), 1.0);
        // after Back the timestamp is cleared: nothing rises again
        assert_eq!(piece_progress(&e, 2, 2, 20.0, None, 3), 1.0);
    }

    #[test]
    fn an_idle_window_does_not_replay_the_entrance() {
        fn frame(ctx: &egui::Context, time: f64, draw: bool) -> Option<f32> {
            let mut age = None;
            let input = egui::RawInput {
                time: Some(time),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                if draw {
                    age = Some(entry_age(ui, 3, true, false));
                }
            });
            out.textures_delta.clear();
            age
        }
        let ctx = egui::Context::default();
        assert_eq!(frame(&ctx, 0.0, true), Some(0.0));
        let later = frame(&ctx, 5.0, true).unwrap();
        assert!(later > 4.9, "an idle gap replayed the entrance: {later}");
        frame(&ctx, 6.0, false);
        assert_eq!(frame(&ctx, 7.0, true), Some(0.0));
    }
}
