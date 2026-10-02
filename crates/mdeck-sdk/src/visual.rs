//! Visuals: the kinds of fenced block (```` ```@barchart ````) a deck can
//! hold. See [`Visual`].

use crate::geometry::Hint;
use crate::paint::Rect;
use crate::paint::{Painter, Texture};
use crate::problem::Problem;
use crate::tokens::Tokens;

/// A visual kind, drawn from the text of its fence.
///
/// ```
/// use mdeck_sdk::paint::{Color, Rect};
/// use mdeck_sdk::problem::Problem;
/// use mdeck_sdk::visual::{Visual, VisualCx};
///
/// /// ```@meter: one number from 0 to 100, drawn as a bar.
/// struct Meter;
///
/// impl Visual for Meter {
///     fn tag(&self) -> &str { "meter" }
///     fn summary(&self) -> &str { "A single value as a bar." }
///     fn check(&self, src: &str) -> Vec<Problem> {
///         match src.trim().parse::<f32>() {
///             Ok(_) => vec![],
///             Err(_) => vec![Problem::new("visual", "@meter needs a number")],
///         }
///     }
///     fn draw(&self, cx: &mut VisualCx, src: &str, rect: Rect, _step: usize) -> f32 {
///         let v = src.trim().parse::<f32>().unwrap_or(0.0).clamp(0.0, 100.0) / 100.0;
///         let h = 40.0 * cx.scale();
///         let track = Rect::from_min_size(rect.min, mdeck_sdk::paint::Vec2::new(rect.width(), h));
///         cx.painter().rect_filled(track, h / 2.0, cx.tokens().rule);
///         let bar = track.sub_rect(0.0, 0.0, v, 1.0);
///         cx.painter().rect_filled(bar, h / 2.0, cx.tokens().accent);
///         cx.publish(mdeck_sdk::geometry::Hint::Bar(bar));
///         h
///     }
/// }
/// ```
pub trait Visual: Send + Sync {
    /// The fence tag without `@` (`meter` for ```` ```@meter ````).
    fn tag(&self) -> &str;

    /// One sentence for `mdeck spec` and the docs.
    fn summary(&self) -> &str;

    /// What is wrong with `src`, for `mdeck --check`. The default finds nothing.
    fn check(&self, _src: &str) -> Vec<Problem> {
        Vec::new()
    }

    /// How many reveal steps the visual adds to its slide (0: none). The
    /// default is 0.
    fn steps(&self, _src: &str) -> usize {
        0
    }

    /// Draw `src` into `rect` at reveal `step` and return the height used
    /// in points (at most `rect.height()`).
    fn draw(&self, cx: &mut VisualCx, src: &str, rect: Rect, step: usize) -> f32;
}

/// Loads images a visual names, relative to the deck. The host implements it.
pub trait ImageLoader {
    /// The image at `path` as a texture, or `None` when it cannot be read.
    fn load(&mut self, painter: &Painter, path: &str) -> Option<Texture>;
}

/// What a visual draws with.
///
/// Made by the host for each draw. See [`Visual`] for an example.
pub struct VisualCx<'a> {
    pub(crate) painter: Painter,
    pub(crate) tokens: &'a Tokens,
    pub(crate) scale: f32,
    pub(crate) animate: bool,
    pub(crate) published: Vec<Hint>,
    pub(crate) images: Option<&'a mut dyn ImageLoader>,
    #[cfg_attr(not(feature = "unstable-egui"), allow(dead_code))]
    pub(crate) ui: Option<&'a mut egui::Ui>,
}

impl<'a> VisualCx<'a> {
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

    /// Whether to animate reveals (false for stills and reduced motion).
    pub fn animate(&self) -> bool {
        self.animate
    }

    /// Publish drawn geometry for the engine (see [`crate::geometry`]).
    pub fn publish(&mut self, hint: Hint) {
        self.published.push(hint);
    }

    /// The image at `path` (relative to the deck), loaded once and cached
    /// by the host.
    pub fn image(&mut self, path: &str) -> Option<Texture> {
        let painter = self.painter.clone();
        self.images.as_mut()?.load(&painter, path)
    }

    /// The raw egui `Ui` the slide is drawn in, when the host has one.
    ///
    /// **Unstable:** outside the compatibility promise (EXT-26); any egui
    /// release may break code that uses it. Every need for it is a request
    /// to extend [`crate::paint`].
    #[cfg(feature = "unstable-egui")]
    pub fn egui_ui(&mut self) -> Option<&mut egui::Ui> {
        self.ui.as_deref_mut()
    }

    /// The geometry published so far (the host takes it after drawing).
    pub(crate) fn take_published(&mut self) -> Vec<Hint> {
        std::mem::take(&mut self.published)
    }
}
