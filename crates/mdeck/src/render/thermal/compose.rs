//! Turning a source's values into a picture: the palette over a window,
//! everything below a threshold in gray, and hatching where the source has
//! no trustworthy value. Pure, so it can be tested and cached.

use eframe::egui::{Color32, ColorImage};

use super::palette::Palette;
use super::source::{Mark, Source};

/// Longest side of a composed picture: sharp on a 4K slide.
pub const MAX_SIDE: usize = 2048;

/// How a source is shown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub palette: Palette,
    /// The values shown from the palette's cold end to its hot end, in the
    /// source's own unit (relative 0..1 for display images).
    pub window: (f32, f32),
    /// Colour only values at or above this; the rest is gray.
    pub threshold: Option<f32>,
}

impl Look {
    /// A key for caching the composed picture.
    pub fn key(&self) -> (super::Palette, u32, u32, Option<u32>) {
        (
            self.palette,
            self.window.0.to_bits(),
            self.window.1.to_bits(),
            self.threshold.map(f32::to_bits),
        )
    }

    /// Where `value` falls in the window, 0..1.
    pub fn t(&self, value: f32) -> f32 {
        let (lo, hi) = self.window;
        ((value - lo) / (hi - lo).max(f32::EPSILON)).clamp(0.0, 1.0)
    }
}

/// Gray for what is below the threshold: the same lightness order, dimmed,
/// so the coloured part reads as the subject.
fn below(t: f32) -> Color32 {
    let v = (24.0 + t * 120.0) as u8;
    Color32::from_gray(v)
}

/// Diagonal stripes `period` pixels apart: a third of each takes `b`.
fn hatch(x: usize, y: usize, period: usize, a: Color32, b: Color32) -> Color32 {
    if (x + y) % period < period / 3 { b } else { a }
}

/// Compose `source` with `look` at no more than [`MAX_SIDE`] on a side.
pub fn compose(source: &Source, look: &Look) -> ColorImage {
    let factor = (source.width.max(source.height) as f32 / MAX_SIDE as f32)
        .ceil()
        .max(1.0) as usize;
    let (w, h) = (
        source.width.div_ceil(factor),
        source.height.div_ceil(factor),
    );
    let lut = look.palette.lut();
    // stripes stay visible however small the picture is drawn
    let period = (w.max(h) / 90).max(6);
    let color = |t: f32| lut[(t * 255.0).round() as usize];
    let mut pixels = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let i = (y * factor).min(source.height - 1) * source.width
                + (x * factor).min(source.width - 1);
            let v = source.values[i];
            let mark = source.marks.as_ref().map_or(Mark::None, |m| m[i]);
            let c = match mark {
                Mark::Missing => hatch(
                    x,
                    y,
                    period,
                    Color32::from_gray(64),
                    Color32::from_gray(140),
                ),
                Mark::ClippedHigh => hatch(x, y, period, color(1.0), Color32::BLACK),
                Mark::ClippedLow => hatch(x, y, period, color(0.0), Color32::from_gray(150)),
                Mark::None if !v.is_finite() => Color32::from_gray(64),
                Mark::None => {
                    let t = look.t(v);
                    match look.threshold {
                        Some(th) if v < th => below(t),
                        _ => color(t),
                    }
                }
            };
            pixels.push(c);
        }
    }
    ColorImage::new([w, h], pixels)
}

/// Whether any pixel of `source` is marked as clipped or missing.
pub fn has_marks(source: &Source) -> bool {
    source
        .marks
        .as_ref()
        .is_some_and(|m| m.iter().any(|&k| k != Mark::None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::thermal::source::Kind;

    fn ramp(w: usize) -> Source {
        Source {
            width: w,
            height: 1,
            values: (0..w).map(|x| x as f32 / (w - 1) as f32).collect(),
            marks: None,
            kind: Kind::Display,
            chromatic: false,
            min: 0.0,
            max: 1.0,
            step: 1.0 / 255.0,
        }
    }

    #[test]
    fn the_palette_runs_across_the_window() {
        let s = ramp(256);
        let look = Look {
            palette: Palette::WhiteHot,
            window: (0.0, 1.0),
            threshold: None,
        };
        let img = compose(&s, &look);
        assert_eq!(img.size, [256, 1]);
        assert_eq!(img.pixels[0], Color32::BLACK);
        assert_eq!(img.pixels[255], Color32::WHITE);
        // a narrower window saturates both ends
        let narrow = compose(
            &s,
            &Look {
                window: (0.25, 0.75),
                ..look
            },
        );
        assert_eq!(narrow.pixels[10], Color32::BLACK);
        assert_eq!(narrow.pixels[250], Color32::WHITE);
        let mid = narrow.pixels[128];
        assert!(
            mid.r() == mid.g() && (125..=132).contains(&mid.r()),
            "{mid:?}"
        );
    }

    #[test]
    fn below_the_threshold_is_gray_and_above_is_coloured() {
        let s = ramp(101);
        let img = compose(
            &s,
            &Look {
                palette: Palette::Iron,
                window: (0.0, 1.0),
                threshold: Some(0.85),
            },
        );
        let p = |x: usize| img.pixels[x];
        let gray = |c: Color32| c.r() == c.g() && c.g() == c.b();
        assert!(gray(p(50)) && gray(p(84)), "below is gray");
        assert_eq!(p(90), Palette::Iron.at(0.9), "above keeps the palette");
    }

    #[test]
    fn marked_pixels_are_hatched_never_plain() {
        let mut s = ramp(12);
        let mut marks = vec![Mark::None; 12];
        marks[11] = Mark::ClippedHigh;
        marks[0] = Mark::Missing;
        s.marks = Some(marks);
        let img = compose(
            &s,
            &Look {
                palette: Palette::Iron,
                window: (0.0, 1.0),
                threshold: None,
            },
        );
        // the missing pixel is gray, the clipped one striped against the top colour
        assert!(
            img.pixels[0] == Color32::from_gray(64) || img.pixels[0] == Color32::from_gray(140)
        );
        assert!(img.pixels[11] == Color32::WHITE || img.pixels[11] == Color32::BLACK);
        assert!(has_marks(&s));
    }

    #[test]
    fn big_sources_are_sampled_down_to_the_cap() {
        let s = Source {
            width: 5000,
            height: 10,
            ..ramp(5000)
        };
        let s = Source {
            values: vec![0.5; 5000 * 10],
            ..s
        };
        let img = compose(
            &s,
            &Look {
                palette: Palette::Iron,
                window: (0.0, 1.0),
                threshold: None,
            },
        );
        assert!(img.size[0] <= MAX_SIDE);
    }
}
