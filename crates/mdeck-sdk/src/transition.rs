//! Transitions: how one slide gives way to the next. See [`Transition`].

use crate::paint::{Painter, Rect, Vec2};
use crate::tokens::Tokens;

/// How one side (the leaving or the arriving slide) looks at a moment of
/// the transition. The host draws the slide moved by `offset`, scaled by
/// `scale` about its centre, at `opacity`.
///
/// Fields may be added in a 2.x release, so build a look from
/// [`SideLook::SHOWN`] or [`SideLook::HIDDEN`] with the `with_` methods:
///
/// ```
/// use mdeck_sdk::paint::Vec2;
/// use mdeck_sdk::transition::SideLook;
/// assert_eq!(SideLook::SHOWN.opacity, 1.0);
/// assert_eq!(SideLook::HIDDEN.opacity, 0.0);
/// let half = SideLook::SHOWN.with_opacity(0.5).with_offset(Vec2::new(10.0, 0.0)).with_scale(0.9);
/// assert_eq!((half.opacity, half.offset.x, half.scale), (0.5, 10.0, 0.9));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SideLook {
    /// Displacement in points.
    pub offset: Vec2,
    /// 0..1.
    pub opacity: f32,
    /// Size factor about the slide's centre (1: as is).
    pub scale: f32,
}

impl SideLook {
    /// In place, fully shown.
    pub const SHOWN: SideLook = SideLook {
        offset: Vec2::ZERO,
        opacity: 1.0,
        scale: 1.0,
    };
    /// In place, invisible.
    pub const HIDDEN: SideLook = SideLook {
        offset: Vec2::ZERO,
        opacity: 0.0,
        scale: 1.0,
    };

    /// Moved by `offset` points.
    ///
    /// See [`SideLook`] for an example.
    pub const fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// At `opacity` (0..1).
    ///
    /// See [`SideLook`] for an example.
    pub const fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Scaled by `scale` about the slide's centre.
    ///
    /// See [`SideLook`] for an example.
    pub const fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
}

/// A transition between two slides. A deck, a slide or a theme that names
/// it (`transition: drop`) changes slides through it: the host eases `t`
/// over [`Transition::duration`], draws the leaving and the arriving slide
/// as [`Transition::look`] says, then calls [`Transition::paint_over`].
///
/// ```
/// use mdeck_sdk::paint::{Rect, Vec2};
/// use mdeck_sdk::transition::{SideLook, Transition};
///
/// /// The new slide drops in from above.
/// struct Drop;
///
/// impl Transition for Drop {
///     fn name(&self) -> &str { "drop" }
///     fn summary(&self) -> &str { "The next slide drops in from above." }
///     fn look(&self, t: f32, _forward: bool, rect: Rect) -> (SideLook, SideLook) {
///         let to = SideLook::SHOWN.with_offset(Vec2::new(0.0, (t - 1.0) * rect.height()));
///         let from = SideLook::SHOWN.with_opacity(1.0 - t);
///         (from, to)
///     }
/// }
///
/// let (from, to) = Drop.look(1.0, true, Rect::ZERO);
/// assert_eq!((from.opacity, to.offset), (0.0, Vec2::ZERO));
/// ```
pub trait Transition: Send + Sync {
    /// The name decks use (`@transition: drop`).
    fn name(&self) -> &str;

    /// One sentence for `mdeck spec` and the docs.
    fn summary(&self) -> &str;

    /// Seconds the transition takes. The default is 0.5.
    fn duration(&self) -> f32 {
        0.5
    }

    /// How the leaving and the arriving slide look at `t` (0..1, already
    /// eased by the host), going `forward` (or back) over `rect`.
    fn look(&self, t: f32, forward: bool, rect: Rect) -> (SideLook, SideLook);

    /// Paint over both slides at `t` (a wipe line, a flash). The default
    /// paints nothing.
    fn paint_over(&self, _cx: &mut TransitionCx, _t: f32) {}
}

/// What a transition paints with in [`Transition::paint_over`].
pub struct TransitionCx<'a> {
    pub(crate) painter: Painter,
    pub(crate) tokens: &'a Tokens,
    pub(crate) rect: Rect,
    pub(crate) forward: bool,
}

impl<'a> TransitionCx<'a> {
    /// The painter, clipped to the slide.
    pub fn painter(&self) -> &Painter {
        &self.painter
    }

    /// The theme's colours.
    pub fn tokens(&self) -> &Tokens {
        self.tokens
    }

    /// The slide's rect.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Whether the presenter is moving forward.
    pub fn forward(&self) -> bool {
        self.forward
    }
}
