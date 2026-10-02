//! Published geometry: what a slide's visuals drew, so an engine can serve
//! it (embers off bar tops, runners along a line series, dust that keeps
//! clear of a card) instead of decorating around it.
//!
//! Visuals publish through [`crate::visual::VisualCx::publish`]; engines read
//! the last frame's set in [`crate::stage::Stage::geometry`].

use std::hash::{Hash, Hasher};

use crate::paint::{Color, Font, Pos2, Rect};

/// One piece of drawn geometry, in points.
///
/// New kinds may be added in a 2.x release: match with a wildcard arm.
///
/// ```
/// use mdeck_sdk::geometry::Hint;
/// use mdeck_sdk::paint::Pos2;
/// let h = Hint::Circle { center: Pos2::new(960.0, 540.0), radius: 200.0 };
/// assert!(matches!(h, Hint::Circle { .. }));
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Hint {
    /// A filled bar (vertical or horizontal).
    Bar(Rect),
    /// A polyline: a line series, a routed edge, a timeline axis. Direction
    /// matters: runners travel from the first point to the last.
    Path(Vec<Pos2>),
    /// A circle: a pie, a donut, a radar ring, a Venn set.
    Circle {
        /// Centre in points.
        center: Pos2,
        /// Radius in points.
        radius: f32,
    },
    /// A marked point: a scatter dot, a timeline event.
    Point(Pos2),
    /// A box the engine must stay out of: the content area, a node, a card.
    Frame(Rect),
    /// The slide's copy (headings, text, lists) as laid out by the active
    /// design. An engine that fills the slide keeps its brightest motion
    /// clear of it so the words stay readable; one that draws only behind
    /// visuals can ignore it.
    Copy(Rect),
    /// A heading as laid out, at the place it settles: an engine that forms
    /// titles itself draws it (for example with
    /// [`crate::paint::Painter::glyph_points`]). Build one with
    /// [`Hint::text`].
    #[non_exhaustive]
    Text {
        /// The heading's text.
        text: String,
        /// The font it is set in.
        font: Font,
        /// Its top-left corner in points.
        pos: Pos2,
        /// Its colour.
        color: Color,
        /// The slide it belongs to (during a transition two slides draw).
        slide: usize,
    },
}

impl Hint {
    /// A heading `text` in `font` and `color`, its top-left corner at
    /// `pos`, on slide `slide`.
    ///
    /// ```
    /// use mdeck_sdk::geometry::Hint;
    /// use mdeck_sdk::paint::{Color, Font, Pos2};
    /// let h = Hint::text("Hello", Font::display(96.0), Pos2::ZERO, Color::WHITE, 0);
    /// assert!(matches!(h, Hint::Text { .. }));
    /// ```
    pub fn text(
        text: impl Into<String>,
        font: Font,
        pos: Pos2,
        color: Color,
        slide: usize,
    ) -> Self {
        Hint::Text {
            text: text.into(),
            font,
            pos,
            color,
            slide,
        }
    }
}

/// A stable fingerprint of a hint set, quantised to 4-point steps so
/// animation jitter and reveal easing do not read as new geometry every
/// frame. Engines rebuild what they derive from geometry only when it changes.
///
/// ```
/// use mdeck_sdk::geometry::{fingerprint, Hint};
/// use mdeck_sdk::paint::Pos2;
/// let a = [Hint::Point(Pos2::new(10.0, 10.0))];
/// let b = [Hint::Point(Pos2::new(10.4, 10.2))];
/// assert_eq!(fingerprint(&a), fingerprint(&b));
/// ```
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
            Hint::Copy(r) => {
                6u8.hash(&mut h);
                (q(r.left()), q(r.top()), q(r.right()), q(r.bottom())).hash(&mut h);
            }
            Hint::Text {
                text,
                font,
                pos,
                slide,
                ..
            } => {
                5u8.hash(&mut h);
                slide.hash(&mut h);
                text.hash(&mut h);
                q(font.size).hash(&mut h);
                (q(pos.x), q(pos.y)).hash(&mut h);
            }
        }
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Vec2;

    #[test]
    fn fingerprint_ignores_sub_point_jitter_but_not_moves() {
        let bar = |y: f32| {
            [Hint::Bar(Rect::from_min_size(
                Pos2::new(10.0, y),
                Vec2::new(50.0, 100.0),
            ))]
        };
        assert_eq!(fingerprint(&bar(10.0)), fingerprint(&bar(10.2)));
        assert_ne!(fingerprint(&bar(10.0)), fingerprint(&bar(60.0)));
    }

    #[test]
    fn bars_and_frames_differ() {
        let r = Rect::from_min_size(Pos2::ZERO, Vec2::splat(10.0));
        assert_ne!(fingerprint(&[Hint::Bar(r)]), fingerprint(&[Hint::Frame(r)]));
    }
}
