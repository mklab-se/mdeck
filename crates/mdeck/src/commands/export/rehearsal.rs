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
    /// The transition into the slide from the one before, halfway (or
    /// `--at` seconds in)
    Transition,
}

impl Moment {
    /// The file a moment exports to (`countdown.png`, `end.png`, ...).
    pub fn file_name(self) -> &'static str {
        match self {
            Moment::Countdown => "countdown.png",
            Moment::Three => "countdown-3.png",
            Moment::Two => "countdown-2.png",
            Moment::One => "countdown-1.png",
            Moment::Burst => "countdown-burst.png",
            Moment::End => "end.png",
            Moment::Transition => "transition.png",
        }
    }

    /// The one slide a moment is drawn on: `--slide` (or the first of
    /// `--range`) when given, else the first slide for the countdown, the
    /// last for the end and the second for a transition (into it from the
    /// first). `selected` is what `--slide` / `--range`
    /// chose, every slide when neither was given.
    pub fn target(self, selected: &[usize], chose: bool, count: usize) -> Vec<usize> {
        let slide = match (chose, self) {
            (true, _) => selected.first().copied().unwrap_or(0),
            (false, Moment::End) => count.saturating_sub(1),
            (false, Moment::Transition) => 1.min(count.saturating_sub(1)),
            (false, _) => 0,
        };
        vec![slide]
    }
}

/// How far into the burst `--moment burst` shows it when no `--at` is given:
/// the frame it starts, already blown apart (it clears the slide in a tenth
/// of a second).
const BURST_STILL_AT: f32 = 0.0;

/// Settings for looking at an engine's motion in export.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Rehearsal {
    /// Rehearse the engine from a cold start for this many seconds.
    pub(super) at: Option<f32>,
    pub(super) countdown: Option<(crate::engines::CountPhase, f32)>,
    pub(super) end: bool,
    /// The moment exported, which names the one file it writes.
    pub(super) moment: Option<Moment>,
    /// `--moment transition`: the transition into the slide, set once the
    /// slide is known.
    pub(super) transition: Option<crate::render::transition::TransitionKind>,
}

impl Rehearsal {
    pub(super) fn new(at: Option<f32>, moment: Option<Moment>) -> Self {
        use crate::engines::CountPhase;
        let countdown = match moment {
            Some(Moment::Countdown | Moment::Three) => Some((CountPhase::Digit(3), 0.5)),
            Some(Moment::Two) => Some((CountPhase::Digit(2), 0.5)),
            Some(Moment::One) => Some((CountPhase::Digit(1), 0.5)),
            Some(Moment::Burst) => Some((CountPhase::Burst, 0.0)),
            Some(Moment::End | Moment::Transition) | None => None,
        };
        // A settled burst has flown off the slide: without `--at`, show its
        // first frame.
        let at = match moment {
            Some(Moment::Burst) => at.or(Some(BURST_STILL_AT)),
            _ => at,
        };
        Rehearsal {
            at,
            countdown,
            end: moment == Some(Moment::End),
            moment,
            transition: None,
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
        assert_eq!(end.moment.map(Moment::file_name), Some("end.png"));
        // a burst is always rehearsed: settled, it is an empty slide
        assert_eq!(Rehearsal::new(None, Some(Moment::Burst)).at, Some(0.0));
        assert_eq!(Rehearsal::new(Some(0.8), Some(Moment::Burst)).at, Some(0.8));
        let plain = Rehearsal::new(Some(1.5), None);
        assert!(!plain.moment());
        assert_eq!(plain.at, Some(1.5));
    }

    // `--moment` wrote the same image once per slide; it is one image, on
    // the slide `--slide` names (or the countdown's first, the end's last).
    #[test]
    fn a_moment_is_one_image_on_one_slide() {
        let all: Vec<usize> = (0..8).collect();
        assert_eq!(Moment::Countdown.target(&all, false, 8), vec![0]);
        assert_eq!(Moment::End.target(&all, false, 8), vec![7]);
        assert_eq!(Moment::End.target(&[3], true, 8), vec![3]);
        assert_eq!(Moment::Burst.target(&[2, 3, 4], true, 8), vec![2]);
        assert_eq!(Moment::Countdown.file_name(), "countdown.png");
        // a transition is the change into the second slide, or `--slide`
        assert_eq!(Moment::Transition.target(&all, false, 8), vec![1]);
        assert_eq!(Moment::Transition.target(&all, false, 1), vec![0]);
        assert_eq!(Moment::Transition.target(&[5], true, 8), vec![5]);
        assert_eq!(Moment::Transition.file_name(), "transition.png");
        assert!(!Rehearsal::new(None, Some(Moment::Transition)).moment());
        assert_eq!(Moment::Two.file_name(), "countdown-2.png");
    }
}
