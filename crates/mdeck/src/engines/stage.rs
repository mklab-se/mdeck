//! What the core hands an engine each frame: the moment (a slide, the
//! countdown, the end), the slide's figure and where it goes, and the
//! geometry the slide's renderers drew.

#![cfg_attr(
    not(all_engines),
    allow(
        dead_code,
        reason = "the stage is all the host offers an engine; a build with fewer engines reads less of it"
    )
)]

use std::sync::Arc;

use eframe::egui::{Pos2, Rect};

use crate::parser::{Layout, Slide};
use crate::render::hints::Hint;
use crate::render::illustration::Cloud;
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

/// What a moment shows, without its masks and progress: what an engine keys
/// its picture on, to rebuild it only when this changes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Look {
    Slide,
    Digit(u8),
    /// The last digit leaves.
    Burst,
    /// The end slide while it shows the words.
    EndWords,
    /// The end slide after the words have gone.
    EndOut,
}

impl Moment {
    /// What this moment shows. The end shows its words for `end_words`
    /// seconds, then clears.
    pub fn look(&self, end_words: f32) -> Look {
        match self {
            Moment::Slide => Look::Slide,
            Moment::Countdown { digit, .. } => Look::Digit(*digit),
            Moment::Burst { .. } => Look::Burst,
            Moment::End { elapsed, .. } if *elapsed < end_words => Look::EndWords,
            Moment::End { .. } => Look::EndOut,
        }
    }

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

/// A slide's generated picture (`mdeck ai art`), loaded and prepared.
#[derive(Clone)]
pub struct Art {
    pub picture: Arc<crate::render::art::prepare::Prepared>,
    /// On a title slide the picture sits large and dim behind the copy.
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
    pub figure: Option<Figure>,
    /// The slide's generated picture, on engines that draw art.
    pub art: Option<Art>,
    /// Geometry the slide's renderers drew last frame (charts, diagrams,
    /// images), and its fingerprint (changes when the geometry does).
    pub hints: &'a [Hint],
    pub hints_key: u64,
    /// The deck's title and slide count, for engines that print them.
    pub deck_title: Option<&'a str>,
    pub count: usize,
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
    let stage = stage_box(layout);
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

/// The part of the slide a picture stands on beside the copy, as slide
/// fractions.
pub fn stage_box(layout: Layout) -> Rect {
    match layout {
        // Quotes run wider, so the stage is narrower.
        Layout::Quote => Rect::from_min_max(Pos2::new(0.64, 0.10), Pos2::new(0.96, 0.90)),
        // Copy sits left; the stage is the right half.
        _ => Rect::from_min_max(Pos2::new(0.52, 0.10), Pos2::new(0.96, 0.90)),
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
