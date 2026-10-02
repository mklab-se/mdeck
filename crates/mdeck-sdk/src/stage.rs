//! What the core hands an engine each frame: the [`Stage`] (what is shown:
//! the moment, the slide, its picture, the drawn geometry) and the
//! [`Frame`] (where and how: the rect, the scale, the clock, the theme).

use std::sync::Arc;

use crate::cloud::{Cloud, Mask};
use crate::content::Slide;
use crate::geometry::Hint;
use crate::paint::{ImageData, Rect, smoothstep};
use crate::tokens::{EngineSettings, Tokens};

/// What the engine is showing.
///
/// ```
/// use mdeck_sdk::stage::{Look, Moment};
/// assert_eq!(Moment::Slide.look(2.0), Look::Slide);
/// assert_eq!(Moment::Burst { progress: 0.5 }.look(2.0), Look::Burst);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum Moment {
    /// A slide (see [`Stage::slide`]).
    Slide,
    /// The opening countdown shows `digit` (3, 2 or 1), with its glyph mask
    /// and progress through its hold (0..1).
    Countdown {
        /// 3, 2 or 1.
        digit: u8,
        /// The digit's glyph as points.
        mask: Mask,
        /// 0..1 through the digit's hold.
        progress: f32,
    },
    /// The last digit leaves and the first slide arrives (0..1).
    Burst {
        /// 0..1 through the burst.
        progress: f32,
    },
    /// The end slide, `elapsed` seconds after it was entered, with the
    /// words "THE END" as a mask.
    End {
        /// Seconds since the end slide was entered.
        elapsed: f32,
        /// The end words as points.
        words: Mask,
    },
}

/// What a moment shows without its masks and progress: what an engine keys
/// its picture on, to rebuild it only when this changes.
///
/// ```
/// use mdeck_sdk::stage::Look;
/// assert_ne!(Look::Digit(3), Look::Digit(2));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Look {
    /// A slide.
    Slide,
    /// A countdown digit.
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
    ///
    /// ```
    /// use mdeck_sdk::cloud::Mask;
    /// use mdeck_sdk::stage::{Look, Moment};
    /// let end = |t| Moment::End { elapsed: t, words: Mask::new(vec![], 5.0) };
    /// assert_eq!(end(1.0).look(3.0), Look::EndWords);
    /// assert_eq!(end(4.0).look(3.0), Look::EndOut);
    /// ```
    pub fn look(&self, end_words: f32) -> Look {
        match self {
            Moment::Slide => Look::Slide,
            Moment::Countdown { digit, .. } => Look::Digit(*digit),
            Moment::Burst { .. } => Look::Burst,
            Moment::End { elapsed, .. } if *elapsed < end_words => Look::EndWords,
            Moment::End { .. } => Look::EndOut,
        }
    }
}

/// A box in slide fractions: left, top, width and height (0..1 of the
/// slide's width and height). [`Rect::sub_rect`] turns it into points.
///
/// ```
/// use mdeck_sdk::stage::Place;
/// let p = Place { u: 0.5, v: 0.1, w: 0.4, h: 0.8 };
/// assert!(p.u + p.w <= 1.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// Left edge, 0..1 of the slide width.
    pub u: f32,
    /// Top edge, 0..1 of the slide height.
    pub v: f32,
    /// Width, 0..1 of the slide width.
    pub w: f32,
    /// Height, 0..1 of the slide height.
    pub h: f32,
}

impl Place {
    /// The place in points within `rect`.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
    /// use mdeck_sdk::stage::Place;
    /// let slide = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
    /// let r = Place { u: 0.5, v: 0.0, w: 0.5, h: 1.0 }.in_rect(slide);
    /// assert_eq!(r.left(), 960.0);
    /// ```
    pub fn in_rect(&self, rect: Rect) -> Rect {
        rect.sub_rect(self.u, self.v, self.w, self.h)
    }
}

/// How an [`Artwork`] is revealed: in strokes along a drawing path, in
/// hatching passes, blooming outward, or developing like a print.
///
/// ```
/// use mdeck_sdk::stage::Strategy;
/// assert_ne!(Strategy::Draw, Strategy::Develop);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Strategy {
    /// Lines appear along the drawing hand's path.
    Draw,
    /// Tone builds up in hatching passes.
    Hatch,
    /// Colour blooms outward from seeds (watercolour).
    Bloom,
    /// The whole picture darkens into place (a darkroom print).
    Develop,
}

/// A generated picture prepared for an art engine: pixels plus, for every
/// pixel, when it appears during the reveal.
///
/// ```
/// use mdeck_sdk::stage::{Artwork, Strategy};
/// let art = Artwork {
///     width: 2, height: 1,
///     rgba: vec![[255, 255, 255, 255]; 2],
///     when: vec![0, 65535],
///     path: vec![],
///     strategy: Strategy::Draw,
/// };
/// assert_eq!(art.aspect(), 0.5);
/// assert_eq!(art.coverage(0.5, 0.0), [1.0, 0.0]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Artwork {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Straight (not premultiplied) RGBA, row by row. Line art is white
    /// with the ink's darkness as alpha, so the engine tints it with its own ink.
    pub rgba: Vec<[u8; 4]>,
    /// When each pixel appears, 0..=65535 over the reveal.
    pub when: Vec<u16>,
    /// Where the drawing hand is over the reveal, as `(t, u, v)` with `u, v`
    /// in 0..1 of the picture and `t` rising. Empty for [`Strategy::Develop`].
    pub path: Vec<(f32, f32, f32)>,
    /// How the picture is revealed.
    pub strategy: Strategy,
}

impl Artwork {
    /// Height over width.
    ///
    /// See [`Artwork`] for an example.
    pub fn aspect(&self) -> f32 {
        self.height as f32 / self.width.max(1) as f32
    }

    /// How far each pixel has arrived (0..1) at `t` (0: nothing yet, 1:
    /// finished), each taking `soft` (0..1 of the reveal) to arrive. An
    /// engine multiplies its pixels' alpha by this.
    ///
    /// See [`Artwork`] for an example.
    pub fn coverage(&self, t: f32, soft: f32) -> Vec<f32> {
        let steps = 65535.0;
        let span = soft.max(1e-3) * steps;
        let now = t * steps;
        let develop = self.strategy == Strategy::Develop;
        self.when
            .iter()
            .map(|&w| {
                let k = ((now - w as f32) / span).clamp(0.0, 1.0);
                if develop { smoothstep(0.0, 1.0, k) } else { k }
            })
            .collect()
    }
}

/// Where a slide's picture comes from.
///
/// ```
/// use std::sync::Arc;
/// use mdeck_sdk::{cloud::Cloud, stage::PictureSource};
/// let s = PictureSource::Cloud(Arc::new(Cloud::new("owl", vec![], 1.0)));
/// assert!(matches!(s, PictureSource::Cloud(_)));
/// ```
#[derive(Clone, Debug)]
pub enum PictureSource {
    /// A point cloud (`<!-- picture: name -->`).
    Cloud(Arc<Cloud>),
    /// A generated picture, prepared for the engine's medium.
    Artwork(Arc<Artwork>),
    /// A plain image. mdeck 2.0 does not send this: it draws a picture
    /// that names an image file on the stage itself, and engines see it as
    /// a [`crate::geometry::Hint::Frame`].
    Image(Arc<ImageData>),
}

/// The slide's picture, resolved and placed by the design.
///
/// ```
/// use std::sync::Arc;
/// use mdeck_sdk::{cloud::Cloud, stage::{Picture, PictureSource, Place}};
/// let p = Picture {
///     source: PictureSource::Cloud(Arc::new(Cloud::new("owl", vec![], 1.0))),
///     backdrop: false,
///     place: Place { u: 0.55, v: 0.1, w: 0.4, h: 0.8 },
/// };
/// assert!(!p.backdrop);
/// ```
#[derive(Clone, Debug)]
pub struct Picture {
    /// What to show.
    pub source: PictureSource,
    /// On a title slide the picture sits large and dim behind the centred
    /// copy; elsewhere it stands on the stage beside the copy.
    pub backdrop: bool,
    /// Where it goes.
    pub place: Place,
}

/// Everything about what is shown that an engine may use.
///
/// ```
/// use mdeck_sdk::stage::{Moment, Stage};
/// let s = Stage::new(Moment::Slide);
/// assert_eq!(s.index, 0);
/// assert!(s.slide.is_none());
/// ```
#[derive(Clone, Debug)]
pub struct Stage<'a> {
    /// The moment.
    pub moment: Moment,
    /// 0-based slide index (the target slide during a transition).
    pub index: usize,
    /// Reveal step on that slide.
    pub step: usize,
    /// The slide, except during the countdown's burst and on the end slide.
    pub slide: Option<&'a Slide>,
    /// The slide reads as a title page.
    pub title: bool,
    /// The slide's picture, on engines that show pictures.
    pub picture: Option<Picture>,
    /// Geometry the slide's visuals drew last frame.
    pub geometry: &'a [Hint],
    /// The geometry's [`crate::geometry::fingerprint`]: changes when it does.
    pub geometry_key: u64,
    /// The deck's title, for engines that print it.
    pub deck_title: Option<&'a str>,
    /// The number of slides in the deck.
    pub count: usize,
}

impl<'a> Stage<'a> {
    /// A stage at `moment` with nothing else set: slide 0 of 0, no slide,
    /// no picture, no geometry. Handy in tests.
    ///
    /// See [`Stage`] for an example.
    pub fn new(moment: Moment) -> Self {
        Self {
            moment,
            index: 0,
            step: 0,
            slide: None,
            title: false,
            picture: None,
            geometry: &[],
            geometry_key: 0,
            deck_title: None,
            count: 0,
        }
    }
}

/// The frame being drawn.
///
/// ```
/// use mdeck_sdk::paint::{Pos2, Rect, Vec2};
/// use mdeck_sdk::stage::Frame;
/// use mdeck_sdk::tokens::{EngineSettings, Tokens};
/// let (tokens, settings) = (Tokens::default(), EngineSettings::new());
/// let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(960.0, 540.0));
/// let f = Frame::new(rect, &tokens, &settings);
/// assert_eq!(f.scale, 0.5);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    /// The slide's rect on screen (or on the export canvas).
    pub rect: Rect,
    /// `min(w / 1920, h / 1080)`: multiply every pixel size by it.
    pub scale: f32,
    /// The slide's opacity (0..1) during transitions.
    pub opacity: f32,
    /// Seconds since the last frame.
    pub dt: f32,
    /// Export: settle at once and paint the finished look (ENG-07).
    pub still: bool,
    /// Show settled states, no motion (ENG-09).
    pub reduced_motion: bool,
    /// The theme's colours.
    pub tokens: &'a Tokens,
    /// The theme's `engine:` settings.
    pub settings: &'a EngineSettings,
}

impl<'a> Frame<'a> {
    /// A frame over `rect` at full opacity, one 60 Hz frame long, live
    /// (not a still), with the scale derived from `rect`.
    ///
    /// See [`Frame`] for an example.
    pub fn new(rect: Rect, tokens: &'a Tokens, settings: &'a EngineSettings) -> Self {
        Self {
            rect,
            scale: (rect.width() / 1920.0).min(rect.height() / 1080.0),
            opacity: 1.0,
            dt: 1.0 / 60.0,
            still: false,
            reduced_motion: false,
            tokens,
            settings,
        }
    }

    /// Whether to show settled states (a still or reduced motion).
    ///
    /// ```
    /// # use mdeck_sdk::{paint::Rect, stage::Frame, tokens::{EngineSettings, Tokens}};
    /// # let (t, s) = (Tokens::default(), EngineSettings::new());
    /// let mut f = Frame::new(Rect::ZERO, &t, &s);
    /// f.reduced_motion = true;
    /// assert!(f.settled());
    /// ```
    pub fn settled(&self) -> bool {
        self.still || self.reduced_motion
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn develop_coverage_is_eased() {
        let mut art = Artwork {
            width: 1,
            height: 1,
            rgba: vec![[0; 4]],
            when: vec![0],
            path: vec![],
            strategy: Strategy::Draw,
        };
        let linear = art.coverage(0.25, 0.5)[0];
        art.strategy = Strategy::Develop;
        let eased = art.coverage(0.25, 0.5)[0];
        assert!((linear - 0.5).abs() < 1e-4);
        assert!((eased - 0.5).abs() < 1e-4, "smoothstep(0.5) is 0.5");
        assert!(art.coverage(0.1, 0.5)[0] < 0.2);
    }
}
