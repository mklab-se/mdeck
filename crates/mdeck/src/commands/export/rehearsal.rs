//! `--at` / `--moment`: stills of an engine's motion.

/// A moment outside the slides that `--moment` exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Moment {
    /// The opening countdown (its first digit, 3)
    Countdown,
    /// The countdown's 3
    #[value(name = "3")]
    Three,
    /// The countdown's 2
    #[value(name = "2")]
    Two,
    /// The countdown's 1
    #[value(name = "1")]
    One,
    /// The burst after the countdown's 1
    Burst,
    /// The end of the deck (the engine's end act)
    End,
}

/// Settings for looking at an engine's motion in export.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Rehearsal {
    /// Rehearse the engine from a cold start for this many seconds.
    pub(super) at: Option<f32>,
    pub(super) countdown: Option<(crate::engines::CountPhase, f32)>,
    pub(super) end: bool,
}

impl Rehearsal {
    pub(super) fn new(at: Option<f32>, moment: Option<Moment>) -> Self {
        use crate::engines::CountPhase;
        let countdown = match moment {
            Some(Moment::Countdown | Moment::Three) => Some((CountPhase::Digit(3), 0.5)),
            Some(Moment::Two) => Some((CountPhase::Digit(2), 0.5)),
            Some(Moment::One) => Some((CountPhase::Digit(1), 0.5)),
            Some(Moment::Burst) => Some((CountPhase::Burst, 0.0)),
            Some(Moment::End) | None => None,
        };
        Rehearsal {
            at,
            countdown,
            end: moment == Some(Moment::End),
        }
    }

    /// Whether the still shows a moment instead of a slide.
    pub(super) fn moment(&self) -> bool {
        self.end || self.countdown.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::CountPhase;

    #[test]
    fn moments_map_to_the_countdown_or_the_end() {
        let r = Rehearsal::new(Some(0.6), Some(Moment::Countdown));
        assert_eq!(r.at, Some(0.6));
        assert_eq!(r.countdown, Some((CountPhase::Digit(3), 0.5)));
        assert!(r.moment() && !r.end);
        assert_eq!(
            Rehearsal::new(None, Some(Moment::One)).countdown,
            Some((CountPhase::Digit(1), 0.5))
        );
        let end = Rehearsal::new(Some(2.0), Some(Moment::End));
        assert!(end.end && end.countdown.is_none());
        let plain = Rehearsal::new(Some(1.5), None);
        assert!(!plain.moment());
        assert_eq!(plain.at, Some(1.5));
    }
}
