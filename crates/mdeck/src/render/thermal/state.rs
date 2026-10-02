//! Where a block is at a reveal step: which lens, how far revealed, which
//! threshold, which spots. A pure function of the step, so going back
//! rebuilds the earlier state exactly instead of replaying effects.

use super::spec::{Action, Line, Threshold};

/// A lens position: centre and radius as fractions of the image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LensAt {
    pub x: f32,
    pub y: f32,
    pub r: f32,
}

/// A spot to draw, and the step it appeared on.
#[derive(Debug, Clone, PartialEq)]
pub struct SpotAt {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub text: Option<String>,
    pub step: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    /// The lens now, the lens before it (where it glides from) and the step
    /// it moved on.
    pub lens: Option<(LensAt, Option<LensAt>, usize)>,
    /// The step the image was fully revealed on.
    pub revealed: Option<usize>,
    /// The threshold now, the one before (fading out) and its step.
    pub threshold: Option<(Threshold, Option<Threshold>, usize)>,
    pub spots: Vec<SpotAt>,
    /// The block reveals at all (has a lens or a reveal line). Without one
    /// the thermal image shows from the start.
    pub reveals: bool,
}

impl State {
    /// The state of a block whose kept lines and steps are `steps`, at
    /// reveal step `step`.
    pub fn at(steps: &[(&Line, usize)], step: usize) -> State {
        let mut st = State {
            reveals: steps
                .iter()
                .any(|(l, _)| matches!(l.action, Action::Lens { .. } | Action::Reveal)),
            ..State::default()
        };
        for (line, s) in steps.iter().filter(|(_, s)| *s <= step) {
            match &line.action {
                Action::Lens { x, y, r } => {
                    let at = LensAt {
                        x: *x,
                        y: *y,
                        r: *r,
                    };
                    let prev = st.lens.map(|(l, _, _)| l);
                    st.lens = Some((at, prev, *s));
                }
                Action::Reveal => {
                    st.revealed.get_or_insert(*s);
                }
                Action::Above(t) => {
                    let prev = st.threshold.take().map(|(t, _, _)| t);
                    st.threshold = Some((t.clone(), prev, *s));
                }
                Action::Spot { name, x, y, text } => st.spots.push(SpotAt {
                    name: name.clone(),
                    x: *x,
                    y: *y,
                    text: text.clone(),
                    step: *s,
                }),
            }
        }
        st
    }

    /// Whether the thermal image shows anywhere.
    pub fn thermal_visible(&self) -> bool {
        !self.reveals || self.revealed.is_some() || self.lens.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::super::spec::{Spec, Support};
    use super::*;

    const DECK: &str = "image: a.png\nvisible: b.jpg\n+ lens 20% 30% 10%\n+ lens 70% 40% 15%\n+ reveal\n+ above 85%\n+ above 60%\n* spot Hot 70% 40%\n";

    fn at(step: usize) -> State {
        let spec = Spec::parse(DECK);
        let steps = spec.steps(&Support::DISPLAY);
        State::at(&steps, step)
    }

    #[test]
    fn each_step_builds_on_the_last() {
        let s0 = at(0);
        assert!(
            s0.lens.is_none() && !s0.thermal_visible(),
            "the photo first"
        );
        let s1 = at(1);
        assert_eq!(s1.lens.unwrap().0.x, 0.2);
        assert!(
            s1.lens.unwrap().1.is_none(),
            "the first lens has nowhere to glide from"
        );
        let s2 = at(2);
        let (now, from, step) = s2.lens.unwrap();
        assert_eq!((now.x, from.unwrap().x, step), (0.7, 0.2, 2));
        let s3 = at(3);
        assert_eq!(s3.revealed, Some(3));
        let s5 = at(5);
        let (t, prev, step) = s5.threshold.clone().unwrap();
        assert_eq!(t, Threshold::Relative(0.6));
        assert_eq!(prev, Some(Threshold::Relative(0.85)));
        assert_eq!(step, 5);
        assert_eq!(s5.spots.len(), 1);
    }

    #[test]
    fn going_back_rebuilds_the_earlier_state_exactly() {
        // the state at a step never depends on where we came from
        for step in 0..6 {
            assert_eq!(at(step), at(step));
        }
        assert_eq!(at(5).spots[0].step, 5);
        assert!(at(4).spots.is_empty());
        assert!(at(2).threshold.is_none());
    }

    #[test]
    fn a_block_without_lens_or_reveal_shows_the_thermal_image_from_the_start() {
        let spec = Spec::parse("image: a.png\n+ above 80%\n");
        let s = State::at(&spec.steps(&Support::DISPLAY), 0);
        assert!(!s.reveals && s.thermal_visible());
    }
}
