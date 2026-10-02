//! Reading a thermal source into scalar values. A source is one of:
//!
//! - a **display image**: a grayscale export, brighter is hotter. Its values
//!   are relative intensities (0..1); nothing about it is a temperature.
//! - a **temperature-linear image**: the same, with `mapping: linear lo..hi`
//!   saying how gray levels map to a quantity.
//! - **temperature data**: a 16-bit (or 8-bit) grayscale PNG with a sidecar
//!   `<name>.yaml` giving the unit, scale and offset (`value = raw * scale +
//!   offset`), and optionally the codes for missing and clipped pixels.
//!
//! The values are kept apart from any display texture, at full precision.

use std::path::Path;

use serde::Deserialize;

use super::spec::{Polarity, Range, Unit};

/// What a source's values mean.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// Relative intensity 0..1; no unit.
    Display,
    /// A quantity, linear in the gray levels (`mapping:`): approximate.
    Linear(Range),
    /// A quantity from a data file and its sidecar: as exact as the file.
    Data(Unit),
}

/// Why a pixel has no trustworthy value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mark {
    None = 0,
    /// At or below the bottom of what the source can say.
    ClippedLow = 1,
    /// At or above the top.
    ClippedHigh = 2,
    /// No value (the sidecar's `nodata`).
    Missing = 3,
}

/// A decoded source.
#[derive(Debug, Clone)]
pub struct Source {
    pub width: usize,
    pub height: usize,
    /// Row-major values: intensity 0..1 (display) or the quantity.
    pub values: Vec<f32>,
    /// Per pixel, when any pixel is marked.
    pub marks: Option<Vec<Mark>>,
    pub kind: Kind,
    /// A colour image: shown as is, without palette, legend or threshold.
    pub chromatic: bool,
    /// The smallest and largest finite value.
    pub min: f32,
    pub max: f32,
    /// One gray level in the value's unit (the quantization step).
    pub step: f32,
}

impl Source {
    pub fn unit(&self) -> Option<&Unit> {
        match &self.kind {
            Kind::Display => None,
            Kind::Linear(r) => Some(&r.unit),
            Kind::Data(u) => Some(u),
        }
    }

    /// The value at a point of the image (fractions), averaged over a small
    /// neighbourhood, and its mark if the centre pixel has one.
    pub fn sample(&self, x: f32, y: f32) -> (f32, Mark) {
        let cx = ((x.clamp(0.0, 1.0) * self.width as f32) as usize).min(self.width - 1);
        let cy = ((y.clamp(0.0, 1.0) * self.height as f32) as usize).min(self.height - 1);
        let mark = self
            .marks
            .as_ref()
            .map_or(Mark::None, |m| m[cy * self.width + cx]);
        if mark != Mark::None {
            return (self.values[cy * self.width + cx], mark);
        }
        let (mut sum, mut n) = (0.0, 0);
        for yy in cy.saturating_sub(1)..=(cy + 1).min(self.height - 1) {
            for xx in cx.saturating_sub(1)..=(cx + 1).min(self.width - 1) {
                let i = yy * self.width + xx;
                let ok = self.marks.as_ref().is_none_or(|m| m[i] == Mark::None);
                if ok && self.values[i].is_finite() {
                    sum += self.values[i];
                    n += 1;
                }
            }
        }
        (if n > 0 { sum / n as f32 } else { f32::NAN }, mark)
    }
}

/// How to read a source file.
#[derive(Debug, Clone, PartialEq)]
pub enum Reading {
    /// `image:`, optionally mapped.
    Image {
        polarity: Polarity,
        mapping: Option<Range>,
    },
    /// `data:`, with its sidecar.
    Data,
}

/// The sidecar of a data file: `cabinet.thermal.png` → `cabinet.thermal.yaml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sidecar {
    pub unit: String,
    #[serde(default = "one")]
    pub scale: f32,
    #[serde(default)]
    pub offset: f32,
    /// The raw code of pixels without a value.
    pub nodata: Option<u32>,
    /// Raw codes at or below which the sensor clipped.
    pub clipped_low: Option<u32>,
    /// Raw codes at or above which the sensor clipped.
    pub clipped_high: Option<u32>,
}

fn one() -> f32 {
    1.0
}

/// Largest number of values kept per source; bigger images are sampled down
/// by a whole factor (still far finer than any slide shows).
const MAX_VALUES: usize = 12_000_000;
/// A pixel is coloured when two channels differ by more than this (of 255).
const CHROMA_TOLERANCE: u16 = 12;
/// An image is a colour image when more than this share of pixels is.
const CHROMA_SHARE: f32 = 0.005;

/// Read `path` as `reading` says.
pub fn load(path: &Path, reading: &Reading) -> anyhow::Result<Source> {
    use anyhow::{Context, anyhow};
    let img = image::open(path).with_context(|| path.display().to_string())?;
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w == 0 || h == 0 {
        return Err(anyhow!("{}: the image is empty", path.display()));
    }
    let chromatic = is_chromatic(&img);
    // gray codes, 16-bit, and how many codes there are
    let deep = matches!(
        img,
        image::DynamicImage::ImageLuma16(_)
            | image::DynamicImage::ImageLumaA16(_)
            | image::DynamicImage::ImageRgb16(_)
            | image::DynamicImage::ImageRgba16(_)
    );
    let luma = img.to_luma16();
    let max_code: u32 = if deep { 65535 } else { 255 };
    let shift = if deep { 0 } else { 8 };
    let factor = ((w * h) as f32 / MAX_VALUES as f32).sqrt().ceil().max(1.0) as usize;
    let (sw, sh) = (w.div_ceil(factor), h.div_ceil(factor));
    let code_at = |x: usize, y: usize| -> u32 {
        (luma.get_pixel((x * factor) as u32, (y * factor) as u32).0[0] >> shift) as u32
    };
    let mut values = Vec::with_capacity(sw * sh);
    let mut marks = vec![Mark::None; sw * sh];
    let mut any_mark = false;
    let (kind, step) = match reading {
        Reading::Image { polarity, mapping } => {
            for y in 0..sh {
                for x in 0..sw {
                    let mut c = code_at(x, y) as f32 / max_code as f32;
                    if *polarity == Polarity::BlackHot {
                        c = 1.0 - c;
                    }
                    let v = match mapping {
                        Some(r) => {
                            // the ends of a mapped range say "this or beyond"
                            if c <= 0.0 || c >= 1.0 {
                                marks[y * sw + x] = if c <= 0.0 {
                                    Mark::ClippedLow
                                } else {
                                    Mark::ClippedHigh
                                };
                                any_mark = true;
                            }
                            r.lo + c * r.span()
                        }
                        None => c,
                    };
                    values.push(v);
                }
            }
            match mapping {
                Some(r) => (Kind::Linear(r.clone()), r.span() / max_code as f32),
                None => (Kind::Display, 1.0 / max_code as f32),
            }
        }
        Reading::Data => {
            if chromatic {
                return Err(anyhow!(
                    "{}: a data file must be grayscale, not colour",
                    path.display()
                ));
            }
            let sidecar_path = path.with_extension("yaml");
            let text = std::fs::read_to_string(&sidecar_path).with_context(|| {
                format!(
                    "{}: the data file needs its sidecar {}",
                    path.display(),
                    sidecar_path.display()
                )
            })?;
            let sc: Sidecar = serde_norway::from_str(&text)
                .with_context(|| format!("{}", sidecar_path.display()))?;
            let unit = Unit::parse(&sc.unit)
                .ok_or_else(|| anyhow!("{}: unit is empty", sidecar_path.display()))?;
            for y in 0..sh {
                for x in 0..sw {
                    let raw = code_at(x, y);
                    let mark = if sc.nodata == Some(raw) {
                        Mark::Missing
                    } else if sc.clipped_low.is_some_and(|c| raw <= c) {
                        Mark::ClippedLow
                    } else if sc.clipped_high.is_some_and(|c| raw >= c) {
                        Mark::ClippedHigh
                    } else {
                        Mark::None
                    };
                    if mark != Mark::None {
                        marks[y * sw + x] = mark;
                        any_mark = true;
                    }
                    let v = if mark == Mark::Missing {
                        f32::NAN
                    } else {
                        raw as f32 * sc.scale + sc.offset
                    };
                    values.push(v);
                }
            }
            (Kind::Data(unit), sc.scale.abs())
        }
    };
    let (mut min, mut max) = (f32::INFINITY, f32::NEG_INFINITY);
    for (i, v) in values.iter().enumerate() {
        if v.is_finite() && (!any_mark || marks[i] == Mark::None) {
            min = min.min(*v);
            max = max.max(*v);
        }
    }
    if !min.is_finite() {
        (min, max) = (0.0, 1.0);
    }
    Ok(Source {
        width: sw,
        height: sh,
        values,
        marks: any_mark.then_some(marks),
        kind,
        chromatic,
        min,
        max,
        step,
    })
}

/// Whether the visible pixels carry colour: more than a small share of
/// pixels has channels further apart than the tolerance. A grayscale
/// picture saved as RGB is not chromatic.
pub fn is_chromatic(img: &image::DynamicImage) -> bool {
    use image::GenericImageView;
    if !img.color().has_color() {
        return false;
    }
    let rgb = img.to_rgb8();
    let (w, h) = img.dimensions();
    // a sparse grid is enough to tell
    let stride = ((w as usize * h as usize) / 250_000).max(1);
    let (mut coloured, mut seen) = (0usize, 0usize);
    for (i, p) in rgb.pixels().enumerate().step_by(stride) {
        let _ = i;
        let [r, g, b] = p.0.map(u16::from);
        let spread = r.max(g).max(b) - r.min(g).min(b);
        if spread > CHROMA_TOLERANCE {
            coloured += 1;
        }
        seen += 1;
    }
    seen > 0 && coloured as f32 / seen as f32 > CHROMA_SHARE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("mdeck-thermal-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A horizontal ramp: 0 on the left to the top code on the right.
    fn ramp16(path: &Path, w: u32, h: u32) {
        let img = image::ImageBuffer::<image::Luma<u16>, Vec<u16>>::from_fn(w, h, |x, _| {
            image::Luma([(x as u64 * 65535 / (w as u64 - 1)) as u16])
        });
        img.save(path).unwrap();
    }

    #[test]
    fn a_known_scalar_ramp_reads_back_through_its_sidecar() {
        let d = tmp("ramp");
        let png = d.join("ramp.thermal.png");
        ramp16(&png, 101, 4);
        // 0..65535 -> 0..100 °C exactly: scale 100/65535
        std::fs::write(
            d.join("ramp.thermal.yaml"),
            format!("unit: °C\nscale: {}\noffset: 0\n", 100.0 / 65535.0),
        )
        .unwrap();
        let s = load(&png, &Reading::Data).unwrap();
        assert_eq!((s.width, s.height), (101, 4));
        assert_eq!(s.kind, Kind::Data(Unit::Celsius));
        for x in [0usize, 25, 50, 100] {
            let v = s.values[x];
            assert!((v - x as f32).abs() < 0.01, "pixel {x}: {v}");
        }
        assert!((s.min - 0.0).abs() < 1e-3 && (s.max - 100.0).abs() < 1e-3);
        // a sample in the middle of the ramp averages its neighbours
        let (v, mark) = s.sample(0.5, 0.5);
        assert_eq!(mark, Mark::None);
        assert!((v - 50.0).abs() < 1.0, "{v}");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn missing_and_clipped_codes_stay_marked() {
        let d = tmp("marks");
        let png = d.join("m.png");
        ramp16(&png, 11, 1);
        std::fs::write(
            d.join("m.yaml"),
            "unit: K\nscale: 0.01\nnodata: 0\nclipped_high: 65535\n",
        )
        .unwrap();
        let s = load(&png, &Reading::Data).unwrap();
        let marks = s.marks.as_ref().unwrap();
        assert_eq!(marks[0], Mark::Missing);
        assert!(s.values[0].is_nan());
        assert_eq!(marks[10], Mark::ClippedHigh);
        assert_eq!(marks[5], Mark::None);
        assert!(s.max < 655.35, "clipped pixels stay out of the range");
        assert_eq!(s.sample(0.0, 0.0).1, Mark::Missing);
        assert!(load(&d.join("nope.png"), &Reading::Data).is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_linear_mapping_and_polarity() {
        let d = tmp("linear");
        let png = d.join("l.png");
        image::GrayImage::from_fn(256, 1, |x, _| image::Luma([x as u8]))
            .save(&png)
            .unwrap();
        let mapping = Range::parse("18..92 °C").unwrap();
        let s = load(
            &png,
            &Reading::Image {
                polarity: Polarity::WhiteHot,
                mapping: Some(mapping.clone()),
            },
        )
        .unwrap();
        assert!((s.values[0] - 18.0).abs() < 1e-3 && (s.values[255] - 92.0).abs() < 1e-3);
        assert!((s.step - 74.0 / 255.0).abs() < 1e-4);
        let marks = s.marks.as_ref().unwrap();
        assert_eq!(
            (marks[0], marks[255]),
            (Mark::ClippedLow, Mark::ClippedHigh)
        );
        // black-hot inverts before the mapping
        let b = load(
            &png,
            &Reading::Image {
                polarity: Polarity::BlackHot,
                mapping: None,
            },
        )
        .unwrap();
        assert_eq!(b.kind, Kind::Display);
        assert!((b.values[0] - 1.0).abs() < 1e-6 && b.values[255].abs() < 1e-6);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn gray_saved_as_rgb_is_not_colour_but_a_photo_is() {
        let gray_rgb = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(64, 64, |x, y| {
            let v = ((x + y) * 2) as u8;
            image::Rgb([v, v, v.saturating_add(3)]) // within the tolerance
        }));
        assert!(!is_chromatic(&gray_rgb));
        let iron = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(64, 64, |x, _| {
            image::Rgb([(x * 4) as u8, 40, 120])
        }));
        assert!(is_chromatic(&iron));
        assert!(!is_chromatic(&image::DynamicImage::ImageLuma8(
            image::GrayImage::new(4, 4)
        )));
    }
}
