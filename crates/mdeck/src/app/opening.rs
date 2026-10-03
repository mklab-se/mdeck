//! The window's first moment. macOS slides a window into its fullscreen
//! space after it opens, and the window reports its final size from the
//! first frame, so the first slide was drawn stretched and cut while the
//! window grew. Instead the window stays black until the slide is over and
//! the theme's fonts are in, then the deck fades in. The window's position
//! tells when the slide is over: it reports where it opened until then, and
//! the screen's corner after.

use std::time::{Duration, Instant};

/// The longest a fullscreen start waits for the window to move into place
/// (the slide takes about half a second).
#[cfg(target_os = "macos")]
const HOLD_FULLSCREEN: Duration = Duration::from_millis(1200);
/// Elsewhere fullscreen is not animated: a few frames are enough.
#[cfg(not(target_os = "macos"))]
const HOLD_FULLSCREEN: Duration = Duration::from_millis(100);
/// A window is shown at its size: hold only for the first frames.
const HOLD_WINDOWED: Duration = Duration::from_millis(100);
/// The fade from black to the deck.
const FADE: Duration = Duration::from_millis(450);

/// Where the opening is: black, fading in, or over.
#[derive(Debug)]
pub(super) struct Opening {
    /// The hold ends early when the window moves (a fullscreen start on
    /// macOS); otherwise it lasts `hold`.
    until_moved: bool,
    /// Where the window was on the first frame: left and top.
    origin: Option<[f32; 2]>,
    hold: Duration,
    fade: Duration,
    /// The first frame.
    first: Option<Instant>,
    /// When the deck started fading in.
    shown: Option<Instant>,
}

impl Opening {
    /// The opening for a fullscreen or windowed start; with reduced motion
    /// the deck appears without a fade.
    pub(super) fn new(windowed: bool, reduced_motion: bool) -> Self {
        Self {
            until_moved: !windowed && cfg!(target_os = "macos"),
            origin: None,
            hold: if windowed {
                HOLD_WINDOWED
            } else {
                HOLD_FULLSCREEN
            },
            fade: if reduced_motion { Duration::ZERO } else { FADE },
            first: None,
            shown: None,
        }
    }

    /// The deck's opacity at `now`: 0 while the window is held black, then
    /// rising to 1 over the fade. `ready` says the window can draw the deck
    /// as it should look (the theme's fonts are in); the hold lasts until
    /// it is. `origin` is the window's top-left corner, when known. The
    /// first call starts the clock.
    pub(super) fn opacity(&mut self, now: Instant, ready: bool, origin: Option<[f32; 2]>) -> f32 {
        let first = *self.first.get_or_insert(now);
        if self.origin.is_none() {
            self.origin = origin;
        }
        if self.shown.is_none() {
            let moved = origin.is_some() && origin != self.origin;
            let held = now.saturating_duration_since(first) < self.hold;
            let waiting = if self.until_moved {
                held && !moved
            } else {
                held
            };
            if !ready || waiting {
                return 0.0;
            }
            self.shown = Some(now);
        }
        let since = now.saturating_duration_since(self.shown.unwrap_or(now));
        if since >= self.fade {
            return 1.0;
        }
        let t = since.as_secs_f32() / self.fade.as_secs_f32();
        // ease out: quick to show, gentle to settle
        1.0 - (1.0 - t) * (1.0 - t)
    }

    /// Whether the deck is showing (fading in or fully): clocks that belong
    /// to the deck, like the countdown's, start then.
    pub(super) fn shown(&self) -> bool {
        self.shown.is_some()
    }

    /// Whether the opening is still running at `now` (held or fading).
    pub(super) fn running(&self, now: Instant) -> bool {
        self.shown
            .is_none_or(|s| now.saturating_duration_since(s) < self.fade)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    /// A fullscreen start that waits for the window to move, as on macOS.
    fn sliding() -> Opening {
        Opening {
            until_moved: true,
            hold: ms(1200),
            ..Opening::new(false, false)
        }
    }

    const OPENED: Option<[f32; 2]> = Some([1612.0, 33.0]);
    const PLACED: Option<[f32; 2]> = Some([0.0, 0.0]);

    #[test]
    fn black_until_the_window_moves_into_place_then_fades_in() {
        let t0 = Instant::now();
        let mut o = sliding();
        assert_eq!(o.opacity(t0, true, OPENED), 0.0);
        assert_eq!(o.opacity(t0 + ms(400), true, OPENED), 0.0, "still sliding");
        assert!(!o.shown());
        let placed = t0 + ms(480);
        assert_eq!(o.opacity(placed, true, PLACED), 0.0, "the fade starts at 0");
        assert!(o.shown());
        let mid = o.opacity(placed + FADE / 2, true, PLACED);
        assert!(mid > 0.5 && mid < 1.0, "{mid}");
        assert!(o.running(placed + FADE / 2));
        assert_eq!(o.opacity(placed + FADE, true, PLACED), 1.0);
        assert!(!o.running(placed + FADE));
    }

    #[test]
    fn a_window_that_never_moves_shows_after_the_longest_hold() {
        let t0 = Instant::now();
        let mut o = sliding();
        o.opacity(t0, true, OPENED);
        assert_eq!(o.opacity(t0 + ms(1100), true, OPENED), 0.0);
        o.opacity(t0 + ms(1200), true, OPENED);
        assert!(o.shown());
        // no position reported at all: the same
        let mut o = sliding();
        o.opacity(t0, true, None);
        assert_eq!(o.opacity(t0 + ms(1100), true, None), 0.0);
        o.opacity(t0 + ms(1200), true, None);
        assert!(o.shown());
    }

    #[test]
    fn a_window_shows_after_its_first_frames() {
        let t0 = Instant::now();
        let mut o = Opening::new(true, false);
        o.opacity(t0, true, OPENED);
        assert!(!o.shown());
        o.opacity(t0 + HOLD_WINDOWED, true, OPENED);
        assert!(o.shown());
    }

    #[test]
    fn waits_for_the_fonts() {
        let t0 = Instant::now();
        let mut o = sliding();
        o.opacity(t0, false, OPENED);
        assert_eq!(
            o.opacity(t0 + ms(600), false, PLACED),
            0.0,
            "fonts not in yet"
        );
        assert!(!o.shown());
        let ready = t0 + ms(650);
        assert_eq!(o.opacity(ready, true, PLACED), 0.0);
        assert_eq!(o.opacity(ready + FADE, true, PLACED), 1.0);
    }

    #[test]
    fn once_shown_it_stays_shown() {
        let t0 = Instant::now();
        let mut o = Opening::new(true, false);
        o.opacity(t0, true, OPENED);
        o.opacity(t0 + HOLD_WINDOWED, true, OPENED);
        // a later theme switch waits for its fonts, the deck stays up
        assert_eq!(o.opacity(t0 + HOLD_WINDOWED + FADE, false, OPENED), 1.0);
    }

    #[test]
    fn reduced_motion_shows_the_deck_without_a_fade() {
        let t0 = Instant::now();
        let mut o = Opening {
            fade: Duration::ZERO,
            ..sliding()
        };
        o.opacity(t0, true, OPENED);
        assert_eq!(o.opacity(t0 + ms(500), true, PLACED), 1.0);
        assert!(!o.running(t0 + ms(500)));
        assert_eq!(Opening::new(false, true).fade, Duration::ZERO);
    }
}
