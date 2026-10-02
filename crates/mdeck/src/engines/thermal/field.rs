//! The heat field: a grid of temperatures over the slide. Sources heat it
//! (quickly), it cools on its own (slowly), and heat spreads to the
//! neighbours. It is drawn through a palette in contour bands, transparent
//! where it is cold.

use mdeck_sdk::paint::{Color, ImageData};

/// How much larger the picture is than the grid (see [`Field::image`]).
pub const UPSAMPLE: usize = 3;

/// Cells across the slide; the height follows the slide's aspect.
pub const COLUMNS: usize = 320;

/// How fast a cell warms toward a hotter source and cools when the source
/// is gone or colder (per second, e-folding).
const HEAT_RATE: f32 = 7.0;
const COOL_RATE: f32 = 1.6;
/// How far heat spreads by default: blur passes per second (each pass
/// spreads it by about a cell).
pub const SPREAD: f32 = 40.0;

pub struct Field {
    pub w: usize,
    pub h: usize,
    pub heat: Vec<f32>,
    /// Where the sources want the field this frame (0..1).
    pub src: Vec<f32>,
    /// Cells that must stay dark (charts, images, diagrams).
    pub calm: Vec<bool>,
    /// Blur passes per second: how fast heat spreads (see [`SPREAD`]).
    pub spread: f32,
    tmp: Vec<f32>,
}

impl Field {
    pub fn new(aspect: f32) -> Self {
        let w = COLUMNS;
        let h = ((COLUMNS as f32 / aspect.max(0.1)).round() as usize).clamp(8, 1024);
        Field {
            w,
            h,
            heat: vec![0.0; w * h],
            src: vec![0.0; w * h],
            calm: vec![false; w * h],
            spread: SPREAD,
            tmp: vec![0.0; w * h],
        }
    }

    pub fn clear_sources(&mut self) {
        self.src.iter_mut().for_each(|v| *v = 0.0);
        self.calm.iter_mut().for_each(|v| *v = false);
    }

    /// Raise the source at a cell to at least `v`.
    pub fn source(&mut self, x: usize, y: usize, v: f32) {
        if x < self.w && y < self.h {
            let i = y * self.w + x;
            self.src[i] = self.src[i].max(v);
        }
    }

    /// A source disc at fractional cell coordinates.
    pub fn disc(&mut self, x: f32, y: f32, r: f32, v: f32) {
        let (x0, x1) = ((x - r).floor().max(0.0) as usize, (x + r).ceil() as usize);
        let (y0, y1) = ((y - r).floor().max(0.0) as usize, (y + r).ceil() as usize);
        for cy in y0..=y1.min(self.h.saturating_sub(1)) {
            for cx in x0..=x1.min(self.w.saturating_sub(1)) {
                let d = ((cx as f32 + 0.5 - x).powi(2) + (cy as f32 + 0.5 - y).powi(2)).sqrt();
                if d <= r {
                    self.source(cx, cy, v);
                }
            }
        }
    }

    /// Keep a box (in cells) dark.
    pub fn keep_calm(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let clamp = |v: f32, n: usize| (v.max(0.0) as usize).min(n);
        for y in clamp(y0, self.h)..clamp(y1.ceil(), self.h) {
            for x in clamp(x0, self.w)..clamp(x1.ceil(), self.w) {
                self.calm[y * self.w + x] = true;
            }
        }
    }

    /// Advance by `dt` seconds. Returns how much the field changed (the
    /// largest change of any cell), so a still field can stop asking for
    /// frames.
    pub fn step(&mut self, dt: f32) -> f32 {
        let dt = dt.clamp(0.0, 0.1);
        // measured over the whole step: at rest, heating and spreading
        // balance, each moving heat while the field as a whole stands still
        let before = self.heat.clone();
        let warm = 1.0 - (-HEAT_RATE * dt).exp();
        let cool = 1.0 - (-COOL_RATE * dt).exp();
        for i in 0..self.heat.len() {
            let (h, s) = (self.heat[i], if self.calm[i] { 0.0 } else { self.src[i] });
            let k = if self.calm[i] {
                1.0 - (-8.0 * dt).exp()
            } else if s > h {
                warm
            } else {
                cool
            };
            self.heat[i] = h + (s - h) * k;
        }
        // spread: separable 1-2-1 blur passes, as many as the time asks
        // for (a fraction of one mixed in), so any frame rate spreads alike
        let passes = self.spread * dt;
        let whole = passes.floor() as usize;
        for k in 0..=whole {
            let mix = if k < whole {
                1.0
            } else {
                passes - whole as f32
            };
            if mix <= 0.0 {
                break;
            }
            self.blur(mix);
        }
        self.heat
            .iter()
            .zip(&before)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f32::max)
    }

    /// One blur pass mixed in by `mix`.
    fn blur(&mut self, mix: f32) {
        let (w, h) = (self.w, self.h);
        for y in 0..h {
            for x in 0..w {
                let l = self.heat[y * w + x.saturating_sub(1)];
                let c = self.heat[y * w + x];
                let r = self.heat[y * w + (x + 1).min(w - 1)];
                self.tmp[y * w + x] = (l + 2.0 * c + r) * 0.25;
            }
        }
        for y in 0..h {
            for x in 0..w {
                let u = self.tmp[y.saturating_sub(1) * w + x];
                let c = self.tmp[y * w + x];
                let d = self.tmp[(y + 1).min(h - 1) * w + x];
                let i = y * w + x;
                let blurred = (u + 2.0 * c + d) * 0.25;
                let next = self.heat[i] + (blurred - self.heat[i]) * mix;
                self.heat[i] = if self.calm[i] { next * 0.5 } else { next };
            }
        }
    }

    /// Run until the field stops changing (export, reduced motion).
    pub fn settle(&mut self) {
        for _ in 0..600 {
            if self.step(1.0 / 30.0) < 1e-4 {
                break;
            }
        }
    }

    /// The field as a picture: the palette in `bands` contour steps (0 for
    /// smooth), transparent where it is cold. The field is interpolated to
    /// [`UPSAMPLE`] times its size before it is cut into bands, so contour
    /// edges are smooth curves, not the grid's steps.
    pub fn image(&self, lut: &[Color; 256], bands: usize) -> ImageData {
        let (w, h) = (self.w * UPSAMPLE, self.h * UPSAMPLE);
        let mut pixels = Vec::with_capacity(w * h);
        for y in 0..h {
            let fy = ((y as f32 + 0.5) / UPSAMPLE as f32 - 0.5).clamp(0.0, (self.h - 1) as f32);
            let (y0, ty) = (fy.floor() as usize, fy.fract());
            let y1 = (y0 + 1).min(self.h - 1);
            for x in 0..w {
                let fx = ((x as f32 + 0.5) / UPSAMPLE as f32 - 0.5).clamp(0.0, (self.w - 1) as f32);
                let (x0, tx) = (fx.floor() as usize, fx.fract());
                let x1 = (x0 + 1).min(self.w - 1);
                let at = |xx: usize, yy: usize| self.heat[yy * self.w + xx];
                let top = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * tx;
                let bottom = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * tx;
                let t = (top + (bottom - top) * ty).clamp(0.0, 1.0);
                let tq = if bands > 0 {
                    (t * bands as f32).floor() / bands as f32
                } else {
                    t
                };
                let c = lut[(tq * 255.0) as usize];
                let a = smooth(0.03, 0.3, t);
                pixels.push(Color::from_rgba_unmultiplied(
                    c.r(),
                    c.g(),
                    c.b(),
                    (a * 255.0) as u8,
                ));
            }
        }
        ImageData {
            size: [w, h],
            pixels,
        }
    }

    /// The hottest cell.
    #[cfg(test)]
    pub fn peak(&self) -> f32 {
        self.heat.iter().copied().fold(0.0, f32::max)
    }
}

pub fn smooth(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_heats_its_cell_and_the_heat_spreads_then_cools() {
        let mut f = Field::new(16.0 / 9.0);
        assert_eq!((f.w, f.h), (320, 180));
        let (x, y) = (100, 100);
        // a small hot body (a single cell gives most of its heat away)
        f.disc(x as f32 + 0.5, y as f32 + 0.5, 2.5, 1.0);
        for _ in 0..30 {
            f.step(1.0 / 60.0);
        }
        let at = |f: &Field, x: usize, y: usize| f.heat[y * f.w + x];
        assert!(at(&f, x, y) > 0.2, "the source warms its cell");
        assert!(at(&f, x + 5, y) > 0.0, "heat spreads beyond the body");
        assert!(at(&f, x + 5, y) < at(&f, x, y));
        let hot = at(&f, x, y);
        f.clear_sources();
        for _ in 0..60 {
            f.step(1.0 / 60.0);
        }
        assert!(at(&f, x, y) < hot * 0.5, "it cools without its source");
    }

    #[test]
    fn calm_boxes_stay_dark_and_a_still_field_settles() {
        let mut f = Field::new(2.0);
        f.disc(50.0, 50.0, 6.0, 1.0);
        f.keep_calm(40.0, 40.0, 60.0, 60.0);
        f.settle();
        assert!(
            f.peak() < 0.05,
            "a calm box keeps its sources dark: {}",
            f.peak()
        );
        let mut g = Field::new(2.0);
        g.disc(50.0, 50.0, 6.0, 1.0);
        g.settle();
        assert!(g.peak() > 0.8);
        assert!(g.step(1.0 / 30.0) < 1e-3, "settled");
    }

    #[test]
    fn cold_is_transparent_and_bands_quantise() {
        let mut f = Field::new(1.0);
        f.heat.iter_mut().for_each(|v| *v = 0.55);
        f.heat[0] = 0.0;
        let lut = crate::engines::heat_palette::Palette::WhiteHot.lut();
        let img = f.image(&lut, 4);
        assert_eq!(img.size, [f.w * UPSAMPLE, f.h * UPSAMPLE]);
        assert_eq!(img.pixels[0].a(), 0, "the cold corner");
        let warm = img.pixels[img.size[0] * 30 + 30];
        assert_eq!(warm.a(), 255);
        // 0.55 in 4 bands is 0.5
        let expected = lut[127].r();
        assert!((warm.r() as i32 - expected as i32).abs() <= 1);
    }
}
