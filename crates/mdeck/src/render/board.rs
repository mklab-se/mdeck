//! Drawing a slide with a board engine's design set (ENG-03): the board
//! draws every slide itself, text included. The set gets the slide as the
//! SDK's content model and borrows the deck's images and visuals through
//! [`DesignServices`].

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

/// Draw `slide` with the board's design `set` into `rect`.
pub fn render(
    set: &dyn DesignSet,
    cx: &BlockCx,
    slide: &Slide,
    rect: egui::Rect,
    slide_cx: &SlideContext,
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
    let design = h::with_engine_live(design, slide_cx.engine_drew);
    let design = h::with_deck(design, slide_cx.deck_title.clone(), slide_cx.count);
    let mut design = h::with_services(design, &mut services);
    set.render(&mut design, &content, h::rect(rect));
    for hint in h::take_design_hints(&mut design) {
        if let mdeck_sdk::geometry::Hint::Frame(r) = hint {
            super::hints::push(cx.ui.ctx(), super::hints::Hint::Frame(h::egui_rect(r)));
        }
    }
}
