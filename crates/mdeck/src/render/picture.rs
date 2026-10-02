//! Where a slide's picture goes: on the stage beside the copy, or large
//! and dim behind a title (D13). The design decides.
//!
//! A picture that names an image file (PIC-02, step 3) is content: mdeck
//! draws it here, on the design's stage, whatever the engine. Engines see
//! it as a `Frame` hint, so a picture engine can react to it. A point cloud
//! is the engine's to draw; on an engine without pictures mdeck stipples
//! it here instead.

use eframe::egui::{self, Color32, Pos2, Rect};
use mdeck_sdk::stage::Place;

use crate::parser::{Design, Slide};
use crate::render::designs::motion;
use crate::render::image_cache::ImageState;
use crate::render::{BlockCx, SlideContext};
use crate::theme::arrangement::Stage;

/// File extensions that make a `picture:` value an image file rather than
/// a point cloud name (cloud names have no dot).
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "svg"];

/// How strongly an image shows behind a title's copy.
const BACKDROP_OPACITY: f32 = 0.24;

/// How strongly a stippled point cloud shows behind a title's copy.
const CLOUD_BACKDROP_OPACITY: f32 = 0.3;

/// Seconds a stippled point cloud takes to come in when its slide is
/// entered, the most important points first.
const CLOUD_ENTRY_S: f32 = 1.1;

/// Whether a `picture:` value names an image file (`images/team.jpg`).
pub fn is_image_path(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Where the image picture of `slide` goes in `rect`, for an image of
/// `size` pixels, or `None` when the slide has no image picture or its
/// design no stage. Also says whether it is a title's backdrop.
pub fn image_rect(
    slide: &Slide,
    theme: &crate::theme::Theme,
    rect: Rect,
    size: [usize; 2],
) -> Option<(Rect, bool)> {
    slide.illustration.as_deref().filter(|n| is_image_path(n))?;
    if size[0] == 0 || size[1] == 0 {
        return None;
    }
    stage_rect(slide, theme, rect, size[1] as f32 / size[0] as f32)
}

/// Where a picture of `aspect` (height over width) stands on `slide` in
/// `rect`, and whether it is a title's backdrop; `None` when the design
/// has no stage.
fn stage_rect(
    slide: &Slide,
    theme: &crate::theme::Theme,
    rect: Rect,
    aspect: f32,
) -> Option<(Rect, bool)> {
    let stage = super::design_stage(slide, theme);
    if stage == Stage::None || aspect <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    let backdrop = stage == Stage::Backdrop;
    let p = figure_box(aspect, slide.design, rect.width() / rect.height(), backdrop);
    let r = Rect::from_min_size(
        Pos2::new(
            rect.left() + p.u * rect.width(),
            rect.top() + p.v * rect.height(),
        ),
        egui::vec2(p.w * rect.width(), p.h * rect.height()),
    );
    Some((r, backdrop))
}

/// Draw `slide`'s point cloud picture as a stipple of accent dots on the
/// stage, on an engine that does not draw pictures itself (plain), so a
/// `picture: rocket` shows on every theme. The most important points (the
/// outline) are strongest and come in first when the slide is entered;
/// behind a title the stipple is a dim backdrop.
pub fn draw_cloud(cx: &BlockCx, slide: &Slide, rect: Rect, deck: &SlideContext) {
    if cx.theme.engine.capabilities().picture {
        return;
    }
    let Some(name) = slide.illustration.as_deref().filter(|n| !is_image_path(n)) else {
        return;
    };
    let Some(cloud) = cx.image_cache.cloud(name) else {
        return;
    };
    let n = cloud.points.len();
    let Some((r, backdrop)) = stage_rect(slide, cx.theme, rect, cloud.aspect).filter(|_| n > 0)
    else {
        return;
    };
    let age = if deck.hold_copy {
        -1.0
    } else {
        motion::entry_age(cx.ui, deck.index, deck.animate, false)
    };
    if age < 0.0 {
        return;
    }
    // dots sized to the spacing of the points, within a readable range
    let spacing = (r.width() * r.height() / n as f32).sqrt();
    let radius = (spacing * 0.2).clamp(1.2 * cx.scale, 3.6 * cx.scale);
    let strength = cx.opacity
        * if backdrop {
            CLOUD_BACKDROP_OPACITY
        } else {
            1.0
        };
    let painter = cx.ui.painter();
    for (i, [x, y]) in cloud.points.iter().enumerate() {
        let k = i as f32 / n as f32;
        let appear = motion::ease_out((age - k * CLOUD_ENTRY_S * 0.6) / (CLOUD_ENTRY_S * 0.4));
        let a = strength * (1.0 - 0.45 * k) * appear;
        if a <= 0.004 {
            continue;
        }
        let p = Pos2::new(r.left() + x * r.width(), r.top() + y * r.height());
        painter.circle_filled(
            p,
            radius * 2.6,
            crate::theme::Theme::with_opacity(cx.theme.accent, a * 0.10),
        );
        painter.circle_filled(
            p,
            radius,
            crate::theme::Theme::with_opacity(cx.theme.accent, a),
        );
    }
}

/// Draw `slide`'s picture when it names an image file and the design has a
/// stage: framed beside the copy (the theme's corner radius, a soft shadow
/// and a hairline), or large and dim behind a title. Nothing while the
/// image loads; a missing file is `--check`'s to report.
pub fn draw_image(cx: &BlockCx, slide: &Slide, rect: Rect) {
    let Some(path) = slide.illustration.as_deref().filter(|n| is_image_path(n)) else {
        return;
    };
    let ImageState::Ready(texture) = cx.image_cache.state(cx.ui.ctx(), path) else {
        return;
    };
    let Some((r, backdrop)) = image_rect(slide, cx.theme, rect, texture.size()) else {
        return;
    };
    crate::render::hints::push(cx.ui.ctx(), crate::render::hints::Hint::Frame(r));
    let painter = cx.ui.painter();
    let radius = cx.theme.radius * cx.scale;
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    let alpha = |a: f32| (a.clamp(0.0, 1.0) * 255.0).round() as u8;
    if backdrop {
        let tint = Color32::from_white_alpha(alpha(cx.opacity * BACKDROP_OPACITY));
        painter
            .add(egui::epaint::RectShape::filled(r, radius, tint).with_texture(texture.id(), uv));
        return;
    }
    let shadow = egui::epaint::Shadow {
        offset: [0, (8.0 * cx.scale).min(127.0) as i8],
        blur: (32.0 * cx.scale).min(255.0) as u8,
        spread: 0,
        color: Color32::from_black_alpha(alpha(cx.opacity * 0.45)),
    };
    painter.add(shadow.as_shape(r, radius));
    let tint = Color32::from_white_alpha(alpha(cx.opacity));
    painter.add(egui::epaint::RectShape::filled(r, radius, tint).with_texture(texture.id(), uv));
    let line = crate::theme::Theme::with_opacity(cx.theme.foreground, 0.14 * cx.opacity);
    painter.rect_stroke(
        r,
        radius,
        egui::Stroke::new(cx.scale.max(0.5), line),
        egui::StrokeKind::Inside,
    );
}

/// Where a picture goes, as slide fractions. `aspect` is the picture's
/// height over width and `rect_aspect` the slide's width over height. On
/// the stage it fills the right-hand box beside the copy with breathing
/// room; as a backdrop it stands about four fifths of the slide tall,
/// centred.
pub fn figure_box(aspect: f32, design: Design, rect_aspect: f32, backdrop: bool) -> Place {
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
    let stage = stage_box(design);
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
pub fn stage_box(design: Design) -> Rect {
    match design {
        // Quotes run wider, so the stage is narrower.
        Design::Quote => Rect::from_min_max(Pos2::new(0.64, 0.10), Pos2::new(0.96, 0.90)),
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
                let r = figure_box(aspect, Design::Points, 16.0 / 9.0, backdrop);
                assert!(r.u >= 0.0 && r.u + r.w <= 1.0, "{aspect} {backdrop}: {r:?}");
                assert!(r.v >= 0.0 && r.v + r.h <= 1.0, "{aspect} {backdrop}: {r:?}");
                // h / w in pixels equals the picture's aspect
                let px = r.h / (r.w * 16.0 / 9.0);
                assert!((px - aspect).abs() < 1e-3, "{aspect}: {px}");
            }
        }
        // the stage is on the right, beside the copy
        assert!(figure_box(1.0, Design::Points, 16.0 / 9.0, false).u > 0.5);
    }

    #[test]
    fn a_picture_names_an_image_by_its_extension() {
        assert!(is_image_path("images/team.jpg"));
        assert!(is_image_path("Team.JPEG"));
        assert!(is_image_path("logo.svg"));
        assert!(!is_image_path("rocket"));
        assert!(!is_image_path("server-rack"));
        assert!(!is_image_path("notes.txt"));
    }

    #[test]
    fn an_image_picture_stands_on_the_stage_beside_the_copy() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        let p = crate::parser::parse(
            "# Deck\n\n---\n\n## Team\n<!-- picture: images/Team.jpg -->\n\n- one\n- two\n",
        );
        let slide = &p.slides[1];
        // the path keeps its case (file systems are case sensitive)
        assert_eq!(slide.illustration.as_deref(), Some("images/Team.jpg"));
        let mut editorial = crate::theme::Theme::dark();
        editorial.arrangements =
            crate::theme::arrangement::Arrangements::resolve("editorial", None).unwrap();
        let (r, backdrop) = image_rect(slide, &editorial, rect, [1200, 800]).expect("on the stage");
        assert!(!backdrop);
        assert!(r.left() > 0.5 * 1920.0, "{r:?}");
        assert!(rect.contains_rect(r), "{r:?}");
        assert!(((r.height() / r.width()) - 800.0 / 1200.0).abs() < 1e-3);
        // the first slide names no picture
        assert!(image_rect(&p.slides[0], &editorial, rect, [1200, 800]).is_none());
        // a title slide shows it behind the copy
        let t = crate::parser::parse("# Deck\n<!-- picture: hero.png -->\n");
        let (_, backdrop) = image_rect(&t.slides[0], &editorial, rect, [1200, 800]).unwrap();
        assert!(backdrop);
        // the standard set opens a stage for a slide that sets a picture,
        // and moves the copy into the left column
        let dark = crate::theme::Theme::dark();
        let (r, backdrop) = image_rect(slide, &dark, rect, [1200, 800]).expect("on the stage");
        assert!(!backdrop && r.left() > 0.5 * 1920.0, "{r:?}");
        let copy = dark.slide_arrangement(slide).copy.region;
        assert!(copy[0] + copy[2] <= 0.52, "{copy:?}");
        assert_eq!(
            dark.arrangement(slide.design).stage,
            crate::theme::arrangement::Stage::None
        );
        let (_, backdrop) = image_rect(&t.slides[0], &dark, rect, [1200, 800]).unwrap();
        assert!(backdrop);
        // a slide with code keeps its copy wide, and no stage
        let c =
            crate::parser::parse("## Code\n<!-- picture: a.png -->\n\n```rust\nfn x() {}\n```\n");
        assert!(image_rect(&c.slides[0], &dark, rect, [1200, 800]).is_none());
    }
}
