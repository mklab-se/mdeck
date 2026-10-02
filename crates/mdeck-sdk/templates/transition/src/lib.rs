//! The `{{name}}` transition: the next slide rises into place while a thin
//! accent line sweeps across.
//!
//! Made with `mdeck sdk new transition {{name}}`. A deck selects it with
//! `transition: {{name}}` in its frontmatter, a theme with `transition: {{name}}`.

use mdeck_sdk::paint::{Pos2, Rect, Vec2, premul};
use mdeck_sdk::registry::{Registry, RegistryError};
use mdeck_sdk::transition::{SideLook, Transition, TransitionCx};

/// The entry point `mdeck build --with` calls: register what this crate brings.
pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.transition(Box::new(Rise))?;
    r.theme("{{name}}", include_str!("../theme.yaml"))
}

/// The transition. It holds no state: `look` is a plain function of `t`.
pub struct Rise;

impl Transition for Rise {
    /// The name decks and themes use.
    fn name(&self) -> &str {
        "{{name}}"
    }

    /// One sentence for `mdeck spec` and the docs.
    fn summary(&self) -> &str {
        "The next slide rises into place under a sweeping accent line."
    }

    /// Seconds it takes.
    fn duration(&self) -> f32 {
        0.6
    }

    /// How the leaving and the arriving slide look at `t` (0..1, already
    /// eased). The host draws both; going back plays the motion reversed.
    fn look(&self, t: f32, forward: bool, rect: Rect) -> (SideLook, SideLook) {
        let lift = 0.08 * rect.height();
        let dir = if forward { 1.0 } else { -1.0 };
        let from = SideLook {
            offset: Vec2::new(0.0, -dir * lift * t),
            opacity: 1.0 - t,
            scale: 1.0 - 0.02 * t,
        };
        let to = SideLook {
            offset: Vec2::new(0.0, dir * lift * (1.0 - t)),
            opacity: t,
            scale: 1.0,
        };
        (from, to)
    }

    /// Paint over both slides: the accent line, brightest mid-way.
    fn paint_over(&self, cx: &mut TransitionCx, t: f32) {
        let rect = cx.rect();
        let x = if cx.forward() { t } else { 1.0 - t };
        let alpha = (t * std::f32::consts::PI).sin();
        let h = (rect.height() / 1080.0) * 6.0;
        let line = Rect::from_center_size(
            Pos2::new(rect.left() + x * rect.width(), rect.center().y),
            Vec2::new(rect.width() * 0.35, h),
        );
        cx.painter()
            .rect_filled(line, h / 2.0, premul(cx.tokens().accent, alpha));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slide() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0))
    }

    #[test]
    fn starts_on_the_old_slide_and_ends_on_the_new() {
        let (from, to) = Rise.look(0.0, true, slide());
        assert_eq!((from.opacity, to.opacity), (1.0, 0.0));
        let (from, to) = Rise.look(1.0, true, slide());
        assert_eq!((from.opacity, to), (0.0, SideLook::SHOWN));
    }

    #[test]
    fn going_back_mirrors_the_motion() {
        let (_, fwd) = Rise.look(0.5, true, slide());
        let (_, back) = Rise.look(0.5, false, slide());
        assert_eq!(fwd.offset.y, -back.offset.y);
    }
}
