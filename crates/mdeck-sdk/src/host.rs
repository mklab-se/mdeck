//! The host side of the SDK: what mdeck itself uses to hand extensions
//! their painters and contexts. Hidden from the docs and **outside the
//! compatibility promise**: extensions never call it, and it changes
//! whenever mdeck's internals do.

use std::sync::Arc;

use crate::design::{DesignCx, DesignServices};
use crate::geometry::Hint;
use crate::paint::painter::FontFamilies;
use crate::paint::{Color, FontRole, FromEgui, Painter, Pos2, Rect, ToEgui, Vec2};
use crate::tokens::Tokens;
use crate::transition::TransitionCx;
use crate::visual::{ImageLoader, VisualCx};

pub use crate::paint::painter::Backend;

/// The egui font family names the host registered for each role. `None`
/// keeps egui's default proportional face (monospace for [`FontRole::Mono`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FontFamilyNames {
    /// Family for [`FontRole::Display`].
    pub display: Option<String>,
    /// Family for [`FontRole::Body`].
    pub body: Option<String>,
    /// Family for [`FontRole::Lead`].
    pub lead: Option<String>,
    /// Family for [`FontRole::Strong`].
    pub strong: Option<String>,
    /// Family for [`FontRole::Mono`].
    pub mono: Option<String>,
}

/// Tell the SDK which egui font family each role uses on `ctx`. Call it
/// whenever the theme's fonts change; painters made afterwards use it.
pub fn set_font_families(ctx: &egui::Context, names: FontFamilyNames) {
    let mut f = FontFamilies::default();
    for role in FontRole::ALL {
        f.names[FontFamilies::index(role)] = match role {
            FontRole::Display => names.display.clone(),
            FontRole::Body => names.body.clone(),
            FontRole::Lead => names.lead.clone(),
            FontRole::Strong => names.strong.clone(),
            FontRole::Mono => names.mono.clone(),
        };
    }
    ctx.data_mut(|d| d.insert_temp(FontFamilies::id(), Arc::new(f)));
}

/// An SDK painter over an egui painter.
pub fn painter(inner: egui::Painter, backend: Backend) -> Painter {
    Painter::new(inner, backend)
}

/// The egui painter behind an SDK painter.
pub fn egui_painter(p: &Painter) -> &egui::Painter {
    p.inner()
}

/// A visual's context. Take the geometry it published with [`take_visual_hints`].
pub fn visual_cx<'a>(
    painter: Painter,
    tokens: &'a Tokens,
    scale: f32,
    animate: bool,
    images: Option<&'a mut dyn ImageLoader>,
    ui: Option<&'a mut egui::Ui>,
) -> VisualCx<'a> {
    VisualCx {
        painter,
        tokens,
        scale,
        animate,
        published: Vec::new(),
        images,
        ui,
    }
}

/// The geometry a visual published.
pub fn take_visual_hints(cx: &mut VisualCx) -> Vec<Hint> {
    cx.take_published()
}

/// A design set's context.
#[allow(clippy::too_many_arguments)]
pub fn design_cx<'a>(
    painter: Painter,
    tokens: &'a Tokens,
    scale: f32,
    step: usize,
    index: usize,
    animate: bool,
    ui: Option<&'a mut egui::Ui>,
) -> DesignCx<'a> {
    DesignCx {
        painter,
        tokens,
        scale,
        step,
        index,
        animate,
        published: Vec::new(),
        services: None,
        engine_live: false,
        deck_title: None,
        count: 0,
        ui,
    }
}

/// Lend a design set the host's images and visuals.
pub fn with_services<'a>(
    mut cx: DesignCx<'a>,
    services: &'a mut dyn DesignServices,
) -> DesignCx<'a> {
    cx.services = Some(services);
    cx
}

/// Tell a design set the deck's title and slide count.
pub fn with_deck(mut cx: DesignCx<'_>, title: Option<String>, count: usize) -> DesignCx<'_> {
    cx.deck_title = title;
    cx.count = count;
    cx
}

/// Tell a board's design set whether its engine painted the slide live.
pub fn with_engine_live(mut cx: DesignCx<'_>, live: bool) -> DesignCx<'_> {
    cx.engine_live = live;
    cx
}

/// The geometry a design set published.
pub fn take_design_hints(cx: &mut DesignCx) -> Vec<Hint> {
    cx.take_published()
}

/// A transition's context.
pub fn transition_cx(
    painter: Painter,
    tokens: &Tokens,
    rect: Rect,
    forward: bool,
) -> TransitionCx<'_> {
    TransitionCx {
        painter,
        tokens,
        rect,
        forward,
    }
}

/// An egui texture as an SDK texture.
pub fn texture(handle: egui::TextureHandle) -> crate::paint::Texture {
    crate::paint::Texture {
        handle: handle.into(),
    }
}

/// SDK colour to egui.
pub fn color32(c: Color) -> egui::Color32 {
    c.eg()
}

/// egui colour to SDK.
pub fn color(c: egui::Color32) -> Color {
    c.sdk()
}

/// SDK rect to egui.
pub fn egui_rect(r: Rect) -> egui::Rect {
    r.eg()
}

/// egui rect to SDK.
pub fn rect(r: egui::Rect) -> Rect {
    r.sdk()
}

/// SDK position to egui.
pub fn egui_pos(p: Pos2) -> egui::Pos2 {
    p.eg()
}

/// egui position to SDK.
pub fn pos(p: egui::Pos2) -> Pos2 {
    p.sdk()
}

/// egui vector to SDK.
pub fn vec2(v: egui::Vec2) -> Vec2 {
    v.sdk()
}
