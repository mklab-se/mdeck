//! Design sets: code that arranges a slide's content. The standard and
//! editorial designs are data in the core; a board engine (one that draws
//! every slide itself, such as the split-flap board) supplies a
//! [`DesignSet`] in code.

use crate::content::Slide;
use crate::geometry::Hint;
use crate::paint::{Painter, Rect};
use crate::problem::Problem;
use crate::tokens::Tokens;

/// A set of designs implemented in code.
///
/// ```
/// use mdeck_sdk::content::Slide;
/// use mdeck_sdk::design::{DesignCx, DesignSet};
/// use mdeck_sdk::paint::{Align2, Font, Rect};
/// use mdeck_sdk::problem::Problem;
///
/// /// Every slide as its title, centred.
/// struct TitlesOnly;
///
/// impl DesignSet for TitlesOnly {
///     fn name(&self) -> &str { "titles-only" }
///     fn render(&self, cx: &mut DesignCx, slide: &Slide, rect: Rect) {
///         let title = slide.title().unwrap_or_default();
///         let font = Font::display(96.0 * cx.scale());
///         cx.painter().text(rect.center(), Align2::CENTER_CENTER, &title, font, cx.tokens().heading);
///     }
///     fn unsupported(&self, slide: &Slide) -> Vec<Problem> {
///         if slide.blocks.len() > 1 {
///             vec![Problem::new("design", "only the title is shown").at(slide.line)]
///         } else {
///             vec![]
///         }
///     }
/// }
/// ```
pub trait DesignSet: Send + Sync {
    /// The set's name.
    fn name(&self) -> &str;

    /// Draw `slide` into `rect` at the context's reveal step.
    fn render(&self, cx: &mut DesignCx, slide: &Slide, rect: Rect);

    /// The height `slide` needs at `rect`'s width, for scrolling overflow.
    /// The default is `rect.height()` (it always fits).
    fn measure(&self, _cx: &mut DesignCx, _slide: &Slide, rect: Rect) -> f32 {
        rect.height()
    }

    /// What of `slide` this set does not show, one problem per thing, for
    /// `mdeck --check`. The default reports nothing.
    fn unsupported(&self, _slide: &Slide) -> Vec<Problem> {
        Vec::new()
    }
}

/// What a design set draws with.
///
/// Made by the host for each slide. See [`DesignSet`] for an example.
pub struct DesignCx<'a> {
    pub(crate) painter: Painter,
    pub(crate) tokens: &'a Tokens,
    pub(crate) scale: f32,
    pub(crate) step: usize,
    pub(crate) index: usize,
    pub(crate) animate: bool,
    pub(crate) published: Vec<Hint>,
    #[cfg_attr(not(feature = "unstable-egui"), allow(dead_code))]
    pub(crate) ui: Option<&'a mut egui::Ui>,
}

impl<'a> DesignCx<'a> {
    /// The painter, clipped to the slide.
    pub fn painter(&self) -> &Painter {
        &self.painter
    }

    /// The theme's colours.
    pub fn tokens(&self) -> &Tokens {
        self.tokens
    }

    /// `min(w / 1920, h / 1080)`: multiply every pixel size by it.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// The reveal step on the slide.
    pub fn step(&self) -> usize {
        self.step
    }

    /// The slide's 0-based index in the deck.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Whether to animate (false for stills and reduced motion).
    pub fn animate(&self) -> bool {
        self.animate
    }

    /// Publish drawn geometry for the engine (see [`crate::geometry`]).
    pub fn publish(&mut self, hint: Hint) {
        self.published.push(hint);
    }

    /// The raw egui `Ui` the slide is drawn in, when the host has one.
    ///
    /// **Unstable:** outside the compatibility promise (EXT-26); any egui
    /// release may break code that uses it.
    #[cfg(feature = "unstable-egui")]
    pub fn egui_ui(&mut self) -> Option<&mut egui::Ui> {
        self.ui.as_deref_mut()
    }

    /// The geometry published so far (the host takes it after drawing).
    pub(crate) fn take_published(&mut self) -> Vec<Hint> {
        std::mem::take(&mut self.published)
    }
}
