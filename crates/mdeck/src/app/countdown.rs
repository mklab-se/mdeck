//! The 3-2-1 opener before the first slide.

use eframe::egui;
use std::time::{Duration, Instant};

use super::*;

/// Each countdown digit holds for this long.
const COUNTDOWN_DIGIT: Duration = Duration::from_millis(1100);
/// Time for the particles to gather into the first digit before its second
/// starts counting (the clock starts on the first drawn frame).
const COUNTDOWN_LEAD: Duration = Duration::from_millis(450);
/// The 3 holds this much longer than the other digits: it is the one the
/// audience has to find on a screen that was black a moment ago.
const COUNTDOWN_FIRST_EXTRA: Duration = Duration::from_millis(600);
/// Ember's final burst, after the "1".
const COUNTDOWN_BURST: Duration = Duration::from_millis(1000);
/// The 3-2-1 opener. Ember forms the digits out of particles and bursts;
/// Nord shows plain numerals. Any key or click cancels it.
pub(super) struct Countdown {
    /// Set on the first frame that draws it, not when the app is created:
    /// shader compilation and font atlas building would eat the first digit.
    pub(super) start: Option<Instant>,
    burst: bool,
}

/// What the countdown is showing right now.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum CountdownPhase {
    /// A digit and how far through its second we are (0..1).
    Digit(u8, f32),
    /// The burst and its progress (0..1).
    Burst(f32),
    Done,
}

impl Countdown {
    pub(super) fn phase(&self, now: Instant) -> CountdownPhase {
        let Some(start) = self.start else {
            return CountdownPhase::Digit(3, 0.0);
        };
        let t = now.saturating_duration_since(start).as_secs_f32() - COUNTDOWN_LEAD.as_secs_f32();
        if t < 0.0 {
            return CountdownPhase::Digit(3, 0.0);
        }
        let d = COUNTDOWN_DIGIT.as_secs_f32();
        let first = d + COUNTDOWN_FIRST_EXTRA.as_secs_f32();
        if t < first {
            return CountdownPhase::Digit(3, t / first);
        }
        let t2 = t - first;
        if t2 < 2.0 * d {
            let n = (t2 / d).floor();
            return CountdownPhase::Digit(2 - n as u8, (t2 - n * d) / d);
        }
        if self.burst {
            let b = (t2 - 2.0 * d) / COUNTDOWN_BURST.as_secs_f32();
            if b < 1.0 {
                return CountdownPhase::Burst(b);
            }
        }
        CountdownPhase::Done
    }
}

impl PresentationApp {
    /// Start the opening countdown if the theme has one and the deck did not
    /// turn it off (`@countdown: false`).
    pub(super) fn start_countdown(&mut self) {
        if self.deck.presentation.meta.countdown == Some(false) {
            return;
        }
        let burst = match self.theme.countdown {
            ThemeCountdown::Burst => true,
            ThemeCountdown::Plain => false,
            ThemeCountdown::None => return,
        };
        self.countdown = Some(Countdown { start: None, burst });
    }

    /// Draw Nord's plain numeral for the current countdown phase.
    pub(super) fn draw_countdown_numeral(&self, ui: &egui::Ui, rect: egui::Rect, scale: f32) {
        let Some(cd) = &self.countdown else {
            return;
        };
        let CountdownPhase::Digit(d, p) = cd.phase(Instant::now()) else {
            return;
        };
        // fade in over the first quarter, out over the last quarter, settle in size
        let fade_in = (p / 0.25).clamp(0.0, 1.0);
        let fade_out = ((1.0 - p) / 0.25).clamp(0.0, 1.0);
        let alpha = fade_in.min(fade_out);
        let size =
            420.0 * scale * (1.06 - 0.06 * render::transition::ease_in_out(p.min(0.5) * 2.0));
        let color = Theme::with_opacity(self.theme.heading_color, alpha);
        let galley = ui.painter().layout_no_wrap(
            d.to_string(),
            egui::FontId::new(size, self.theme.display_family()),
            color,
        );
        let pos = egui::pos2(
            rect.center().x - galley.rect.width() / 2.0,
            rect.center().y - galley.rect.height() / 2.0,
        );
        ui.painter().galley(pos, galley, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_phases_run_three_two_one_then_burst_then_done() {
        let start = Instant::now();
        let lead = COUNTDOWN_LEAD.as_secs_f32();
        let d = COUNTDOWN_DIGIT.as_secs_f32();
        let three = d + COUNTDOWN_FIRST_EXTRA.as_secs_f32();
        let cd = Countdown {
            start: Some(start),
            burst: true,
        };
        let at = |secs: f32| start + Duration::from_secs_f32(lead + secs);
        // before the clock has started, and during the lead, the 3 is forming
        let unstarted = Countdown {
            start: None,
            burst: true,
        };
        assert_eq!(unstarted.phase(start), CountdownPhase::Digit(3, 0.0));
        assert_eq!(cd.phase(start), CountdownPhase::Digit(3, 0.0));
        assert!(matches!(cd.phase(at(0.1)), CountdownPhase::Digit(3, _)));
        // the 3 holds longer than a plain digit
        assert!(matches!(cd.phase(at(d + 0.2)), CountdownPhase::Digit(3, _)));
        assert!(matches!(
            cd.phase(at(three + d * 0.5)),
            CountdownPhase::Digit(2, _)
        ));
        assert!(matches!(
            cd.phase(at(three + d * 1.9)),
            CountdownPhase::Digit(1, _)
        ));
        assert!(matches!(
            cd.phase(at(three + d * 2.0 + 0.3)),
            CountdownPhase::Burst(_)
        ));
        assert_eq!(cd.phase(at(three + d * 2.0 + 1.2)), CountdownPhase::Done);
        let plain = Countdown {
            start: Some(start),
            burst: false,
        };
        assert_eq!(plain.phase(at(three + d * 2.0 + 0.1)), CountdownPhase::Done);
    }
}
