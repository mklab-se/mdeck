//! Drawing a slide with a code design set: a board engine's (ENG-03), which
//! draws every slide itself, text included, or one a theme's `designs:`
//! names (EXT-05). The set gets the slide as the SDK's content model and
//! borrows the deck's images and visuals through [`DesignServices`].

use eframe::egui;
use mdeck_sdk::design::{DesignServices, DesignSet};
use mdeck_sdk::host as h;
use mdeck_sdk::paint::{Painter, Rect, Texture};

use super::image_cache::ImageState;
use super::{BlockCx, SlideContext};
use crate::parser::{Chart, Slide};

/// The deck's images and visuals, lent to a design set.
struct Services<'a> {
    block: &'a BlockCx<'a>,
}

impl DesignServices for Services<'_> {
    fn image(&mut self, painter: &Painter, path: &str) -> Option<Texture> {
        let ctx = h::egui_painter(painter).ctx().clone();
        match self.block.image_cache.state(&ctx, path) {
            ImageState::Ready(handle) => Some(h::texture(handle)),
            ImageState::Loading | ImageState::Missing => None,
        }
    }

    fn visual(&mut self, _painter: &Painter, tag: &str, src: &str, rect: Rect, step: usize) -> f32 {
        let rect = h::egui_rect(rect);
        let block = BlockCx {
            reveal_step: step,
            ..*self.block
        };
        if tag == Chart::Thermal.tag() {
            return super::thermal::draw(&block, src, rect.min, rect.width(), rect.height());
        }
        let viz = super::visualizations::VizCtx {
            reveal_timestamp: None,
            ..block.viz()
        };
        super::visualizations::draw_tag(tag, src, &viz, rect.min, rect.width(), rect.height())
    }
}

/// Draw `slide` with the code design `set` into `rect`: a board engine's
/// own (`engine_live` when the board painted the slide live underneath) or
/// the one a theme's `designs:` names. The geometry it publishes reaches
/// the engine like the built-in designs' does.
pub fn render(
    set: &dyn DesignSet,
    cx: &BlockCx,
    slide: &Slide,
    rect: egui::Rect,
    slide_cx: &SlideContext,
    engine_live: bool,
) {
    let tokens = crate::engines::host::convert::tokens(cx.theme);
    h::set_font_families(
        cx.ui.ctx(),
        crate::engines::host::convert::font_families(cx.theme),
    );
    let painter = h::painter(cx.ui.painter().clone(), h::Backend::Glow).with_opacity(cx.opacity);
    let content = crate::engines::host::convert::slide(slide);
    let mut services = Services { block: cx };
    let design = h::design_cx(
        painter,
        &tokens,
        cx.scale,
        cx.reveal_step,
        slide_cx.index,
        slide_cx.animate,
        None,
    );
    let design = h::with_engine_live(design, engine_live);
    let design = h::with_deck(design, slide_cx.deck_title.clone(), slide_cx.count);
    let mut design = h::with_services(design, &mut services);
    set.render(&mut design, &content, h::rect(rect));
    for hint in h::take_design_hints(&mut design) {
        if let Some(hint) = egui_hint(hint) {
            super::hints::push(cx.ui.ctx(), hint);
        }
    }
}

/// The height `slide` needs in the code design `set` at `rect`'s width
/// (fully revealed) and the height there is, for scrolling overflow.
pub fn measure(
    set: &dyn DesignSet,
    ui: &egui::Ui,
    theme: &crate::theme::Theme,
    slide: &Slide,
    rect: egui::Rect,
    scale: f32,
    slide_cx: &SlideContext,
) -> (f32, f32) {
    let tokens = crate::engines::host::convert::tokens(theme);
    h::set_font_families(
        ui.ctx(),
        crate::engines::host::convert::font_families(theme),
    );
    let painter = h::painter(ui.painter().clone(), h::Backend::Glow);
    let content = crate::engines::host::convert::slide(slide);
    let design = h::design_cx(
        painter,
        &tokens,
        scale,
        usize::MAX,
        slide_cx.index,
        false,
        None,
    );
    let mut design = h::with_deck(design, slide_cx.deck_title.clone(), slide_cx.count);
    let needed = set.measure(&mut design, &content, h::rect(rect));
    let needed = if needed.is_finite() {
        needed.max(0.0)
    } else {
        0.0
    };
    (needed, rect.height())
}

/// A design's published geometry as the renderers' own hints (a heading's
/// text needs a laid-out galley, so it is left out).
fn egui_hint(hint: mdeck_sdk::geometry::Hint) -> Option<super::hints::Hint> {
    use super::hints::Hint;
    use mdeck_sdk::geometry::Hint as S;
    Some(match hint {
        S::Bar(r) => Hint::Bar(h::egui_rect(r)),
        S::Frame(r) => Hint::Frame(h::egui_rect(r)),
        S::Copy(r) => Hint::Copy(h::egui_rect(r)),
        S::Path(pts) => Hint::Path(pts.into_iter().map(h::egui_pos).collect()),
        S::Circle { center, radius } => Hint::Circle {
            center: h::egui_pos(center),
            radius,
        },
        S::Point(p) => Hint::Point(h::egui_pos(p)),
        _ => return None,
    })
}
