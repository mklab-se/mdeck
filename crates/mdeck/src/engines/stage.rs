//! What the core hands an engine each frame: the moment (a slide, the
//! countdown, the end), the slide's figure and where it goes, and the
//! geometry the slide's renderers drew.

use std::sync::Arc;

use eframe::egui::Rect;

use crate::parser::{Layout, Slide};
use crate::render::hints::Hint;
use crate::render::illustration::Cloud;
use crate::render::story::Script;
use crate::theme::Theme;

/// Points in the unit square, with the shape's width / height.
pub type Mask = (Arc<Vec<[f32; 2]>>, f32);

/// Where the opening countdown is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CountPhase {
    /// Showing this digit (3, 2 or 1).
    Digit(u8),
    /// The last digit leaves and the first slide arrives.
    Burst,
}

/// What the engine is showing.
#[derive(Clone, Debug)]
pub enum Moment {
    /// A slide (see [`Stage::slide`]).
    Slide,
    /// A countdown digit, with its glyph mask and progress through its hold (0..1).
    Countdown {
        digit: u8,
        mask: Mask,
        progress: f32,
    },
    /// The last digit leaves (0..1).
    Burst { progress: f32 },
    /// The end slide, `elapsed` seconds after it was entered, with the words
    /// "THE END" as a mask.
    End { elapsed: f32, words: Mask },
}

impl Moment {
    /// The countdown phase, when this is part of the countdown.
    pub fn count_phase(&self) -> Option<(CountPhase, f32)> {
        match self {
            Moment::Countdown {
                digit, progress, ..
            } => Some((CountPhase::Digit(*digit), *progress)),
            Moment::Burst { progress } => Some((CountPhase::Burst, *progress)),
            _ => None,
        }
    }
}

/// A slide's `@illustration`, resolved.
#[derive(Clone, Debug)]
pub struct Figure {
    pub cloud: Arc<Cloud>,
    /// On a title slide the figure sits large and dim behind the centred
    /// copy; elsewhere it stands on the stage beside the copy.
    pub backdrop: bool,
    /// Where it goes, as slide fractions (see [`figure_box`]).
    pub place: Place,
}

/// A box in slide fractions: left, top, width and height (0..1 of the
/// slide's width and height).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub u: f32,
    pub v: f32,
    pub w: f32,
    pub h: f32,
}

/// Everything about the current slide an engine may use.
pub struct Stage<'a> {
    pub moment: Moment,
    /// 0-based slide index (the target slide during a transition).
    pub index: usize,
    /// Reveal step on that slide.
    pub reveal: usize,
    /// The slide, except during the countdown's burst and on the end slide.
    pub slide: Option<&'a Slide>,
    /// The first slide reads as a title page.
    pub title: bool,
    pub story: Option<&'a Script>,
    /// Changes whenever a story is regenerated.
    pub story_version: u64,
    pub figure: Option<Figure>,
    /// Geometry the slide's renderers drew last frame (charts, diagrams,
    /// images), and its fingerprint (changes when the geometry does).
    pub hints: &'a [Hint],
    pub hints_key: u64,
}

/// The frame being drawn.
pub struct FrameCx<'a> {
    /// The slide's rect on screen (or on the export canvas).
    pub rect: Rect,
    /// `min(w / 1920, h / 1080)`: multiply every pixel size by it.
    pub scale: f32,
    pub opacity: f32,
    /// Seconds since the last frame.
    pub dt: f32,
    /// Export: settle at once and paint the finished look.
    pub still: bool,
    pub theme: &'a Theme,
}

/// Where a figure goes, as slide fractions. `cloud_aspect` is the cloud's
/// height over width and `rect_aspect` the slide's width over height. On the
/// stage it fills the right-hand box beside the copy with breathing room; as
/// a backdrop it stands about four fifths of the slide tall, centred.
pub fn figure_box(cloud_aspect: f32, layout: Layout, rect_aspect: f32, backdrop: bool) -> Place {
    if backdrop {
        let mut h = 0.80;
        let mut w = h / (cloud_aspect * rect_aspect);
        if w > 0.72 {
            w = 0.72;
            h = w * cloud_aspect * rect_aspect;
        }
        return Place {
            u: 0.5 - w / 2.0,
            v: 0.5 - h / 2.0,
            w,
            h,
        };
    }
    let stage = crate::render::story::stage_box(layout);
    let avail_w = stage.width() * 0.82;
    let avail_h = stage.height() * 0.82;
    // heights are slide-height fractions: h = w * aspect * (W / H)
    let mut w = avail_w;
    let mut h = w * cloud_aspect * rect_aspect;
    if h > avail_h {
        h = avail_h;
        w = h / (cloud_aspect * rect_aspect);
    }
    let (cu, cv) = (stage.center().x, stage.center().y);
    Place {
        u: cu - w / 2.0,
        v: cv - h / 2.0,
        w,
        h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figure_boxes_stay_on_the_slide_and_keep_the_aspect() {
        for aspect in [0.3_f32, 1.0, 2.5] {
            for backdrop in [false, true] {
                let r = figure_box(aspect, Layout::Bullet, 16.0 / 9.0, backdrop);
                assert!(r.u >= 0.0 && r.u + r.w <= 1.0, "{aspect} {backdrop}: {r:?}");
                assert!(r.v >= 0.0 && r.v + r.h <= 1.0, "{aspect} {backdrop}: {r:?}");
                // h / w in pixels equals the cloud's aspect
                let px = r.h / (r.w * 16.0 / 9.0);
                assert!((px - aspect).abs() < 1e-3, "{aspect}: {px}");
            }
        }
        // the stage is on the right, beside the copy
        assert!(figure_box(1.0, Layout::Bullet, 16.0 / 9.0, false).u > 0.5);
    }
}
