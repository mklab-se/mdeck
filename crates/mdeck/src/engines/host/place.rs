//! Where a slide's picture goes: on the stage beside the copy, or large
//! and dim behind a title (D13). The design decides; until designs carry
//! arrangements (phase 3) the layout does.

use eframe::egui::{Pos2, Rect};
use mdeck_sdk::stage::Place;

use crate::parser::Layout;

/// Where a picture goes, as slide fractions. `aspect` is the picture's
/// height over width and `rect_aspect` the slide's width over height. On
/// the stage it fills the right-hand box beside the copy with breathing
/// room; as a backdrop it stands about four fifths of the slide tall,
/// centred.
pub fn figure_box(aspect: f32, layout: Layout, rect_aspect: f32, backdrop: bool) -> Place {
    if backdrop {
        let mut h = 0.80;
        let mut w = h / (aspect * rect_aspect);
        if w > 0.72 {
            w = 0.72;
            h = w * aspect * rect_aspect;
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
    let mut h = w * aspect * rect_aspect;
    if h > avail_h {
        h = avail_h;
        w = h / (aspect * rect_aspect);
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
                // h / w in pixels equals the picture's aspect
                let px = r.h / (r.w * 16.0 / 9.0);
                assert!((px - aspect).abs() < 1e-3, "{aspect}: {px}");
            }
        }
        // the stage is on the right, beside the copy
        assert!(figure_box(1.0, Layout::Bullet, 16.0 / 9.0, false).u > 0.5);
    }
}
