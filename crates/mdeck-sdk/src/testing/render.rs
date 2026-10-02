//! Headless rendering of visuals, design sets and transitions, for their
//! tests (engines use [`Headless::render_engine`]).

use super::Headless;
use crate::content::Slide;
use crate::design::DesignSet;
use crate::geometry::Hint;
use crate::host;
use crate::paint::{ImageData, Rect};
use crate::tokens::Tokens;
use crate::transition::Transition;
use crate::visual::Visual;

/// What [`Headless::render_visual`] and [`Headless::render_design`] return.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Rendered {
    /// The canvas, with the theme's background under the drawing.
    pub image: ImageData,
    /// The geometry the extension published.
    pub hints: Vec<Hint>,
    /// The height the visual reported (a design set's
    /// [`DesignSet::measure`]).
    pub height: f32,
}

impl Headless {
    /// Draw `visual` from `src` at reveal `step` into the canvas inset by
    /// 5% on every side, over the theme's background, as a still (no
    /// animation, no image loader).
    ///
    /// ```
    /// use mdeck_sdk::geometry::Hint;
    /// use mdeck_sdk::paint::Rect;
    /// use mdeck_sdk::testing::Headless;
    /// use mdeck_sdk::tokens::Tokens;
    /// use mdeck_sdk::visual::{Visual, VisualCx};
    ///
    /// struct Block;
    /// impl Visual for Block {
    ///     fn tag(&self) -> &str { "block" }
    ///     fn summary(&self) -> &str { "A filled box." }
    ///     fn draw(&self, cx: &mut VisualCx, _src: &str, rect: Rect, _step: usize) -> f32 {
    ///         cx.painter().rect_filled(rect, 0.0, cx.tokens().accent);
    ///         cx.publish(Hint::Frame(rect));
    ///         rect.height()
    ///     }
    /// }
    ///
    /// let tokens = Tokens::default();
    /// let out = Headless::new(100, 60).render_visual(&Block, "", 0, &tokens);
    /// assert_eq!(out.image.get(50, 30), Some(tokens.accent));
    /// assert_eq!(out.image.get(1, 1), Some(tokens.background));
    /// assert_eq!(out.hints.len(), 1);
    /// ```
    pub fn render_visual(
        &mut self,
        visual: &dyn Visual,
        src: &str,
        step: usize,
        tokens: &Tokens,
    ) -> Rendered {
        let full = self.rect();
        let area = inset(full);
        let mut hints = Vec::new();
        let mut height = 0.0;
        let image = self.paint(|p| {
            p.rect_filled(full, 0.0, tokens.background);
            let mut cx = host::visual_cx(p.clone(), tokens, scale(full), false, None, None);
            height = visual.draw(&mut cx, src, area, step);
            hints = host::take_visual_hints(&mut cx);
        });
        Rendered {
            image,
            hints,
            height,
        }
    }

    /// Draw `slide` with design set `set` at reveal `step` over the whole
    /// canvas and the theme's background, as a still. `height` is what
    /// [`DesignSet::measure`] reports for the canvas.
    ///
    /// ```
    /// use mdeck_sdk::content::Slide;
    /// use mdeck_sdk::design::{DesignCx, DesignSet};
    /// use mdeck_sdk::paint::Rect;
    /// use mdeck_sdk::testing::Headless;
    /// use mdeck_sdk::tokens::Tokens;
    ///
    /// struct Bar;
    /// impl DesignSet for Bar {
    ///     fn name(&self) -> &str { "bar" }
    ///     fn render(&self, cx: &mut DesignCx, _: &Slide, rect: Rect) {
    ///         cx.painter().rect_filled(rect.sub_rect(0.0, 0.9, 1.0, 0.1), 0.0, cx.tokens().accent);
    ///     }
    /// }
    ///
    /// let tokens = Tokens::default();
    /// let out = Headless::new(100, 100).render_design(&Bar, &Slide::default(), 0, &tokens);
    /// assert_eq!(out.image.get(50, 95), Some(tokens.accent));
    /// assert_eq!(out.height, 100.0);
    /// ```
    pub fn render_design(
        &mut self,
        set: &dyn DesignSet,
        slide: &Slide,
        step: usize,
        tokens: &Tokens,
    ) -> Rendered {
        let full = self.rect();
        let mut hints = Vec::new();
        let mut height = 0.0;
        let image = self.paint(|p| {
            p.rect_filled(full, 0.0, tokens.background);
            let mut cx = host::design_cx(p.clone(), tokens, scale(full), step, 0, false, None);
            height = set.measure(&mut cx, slide, full);
            set.render(&mut cx, slide, full);
            hints = host::take_design_hints(&mut cx);
        });
        Rendered {
            image,
            hints,
            height,
        }
    }

    /// Paint `transition`'s [`Transition::paint_over`] at `t` (0..1) over the
    /// theme's background. The slides themselves are the host's: test
    /// [`Transition::look`] directly, it is a plain function.
    ///
    /// ```
    /// use mdeck_sdk::paint::Rect;
    /// use mdeck_sdk::testing::Headless;
    /// use mdeck_sdk::tokens::Tokens;
    /// use mdeck_sdk::transition::{SideLook, Transition, TransitionCx};
    ///
    /// struct Flash;
    /// impl Transition for Flash {
    ///     fn name(&self) -> &str { "flash" }
    ///     fn summary(&self) -> &str { "A white flash." }
    ///     fn look(&self, t: f32, _: bool, _: Rect) -> (SideLook, SideLook) {
    ///         (SideLook::SHOWN.with_opacity(1.0 - t), SideLook::SHOWN.with_opacity(t))
    ///     }
    ///     fn paint_over(&self, cx: &mut TransitionCx, _t: f32) {
    ///         cx.painter().rect_filled(cx.rect(), 0.0, mdeck_sdk::paint::Color::WHITE);
    ///     }
    /// }
    ///
    /// let img = Headless::new(10, 10).render_transition(&Flash, 0.5, true, &Tokens::default());
    /// assert_eq!(img.get(5, 5), Some(mdeck_sdk::paint::Color::WHITE));
    /// ```
    pub fn render_transition(
        &mut self,
        transition: &dyn Transition,
        t: f32,
        forward: bool,
        tokens: &Tokens,
    ) -> ImageData {
        let full = self.rect();
        self.paint(|p| {
            p.rect_filled(full, 0.0, tokens.background);
            let mut cx = host::transition_cx(p.clone(), tokens, full, forward);
            transition.paint_over(&mut cx, t);
        })
    }
}

fn scale(rect: Rect) -> f32 {
    (rect.width() / 1920.0).min(rect.height() / 1080.0)
}

fn inset(rect: Rect) -> Rect {
    rect.sub_rect(0.05, 0.05, 0.9, 0.9)
}
