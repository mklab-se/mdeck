//! Getting a picture ready to be drawn in: trimmed to its subject, made
//! transparent where it is bare paper, and given a time map, the moment
//! (0..1 of the reveal) each pixel appears. An engine reveals the picture by
//! showing the pixels whose moment has come ([`Prepared::coverage`]; the
//! art engines reveal in their own medium in `engines::art::reveal`), so the
//! finished frame is exactly the generated image.
//!
//! How the moments are laid out is the medium's [`Strategy`]:
//! - [`Strategy::Draw`]: along the ink, stroke by stroke, the way a pen would
//!   trace it (line art: sheet, slate, pencil).
//! - [`Strategy::Hatch`]: the outlines first, along the ink, then the tone
//!   from light to dark (a graphite sketch).
//! - [`Strategy::Bloom`]: washes spread outward from where the paint is
//!   heaviest (watercolour).
//! - [`Strategy::Develop`]: the whole print at once, shadows first
//!   (a photograph in the developer).

#![cfg_attr(
    not(all_engines),
    allow(
        dead_code,
        reason = "each art engine uses its own strategy; a build with fewer of them uses less"
    )
)]

use image::{GenericImageView, RgbaImage, imageops};

use super::ArtKind;
use order::{bloom_order, develop_order, draw_order, hatch_order};

mod order;

/// The longest side a prepared picture keeps: sharp on the stage of a
/// 4K slide, and quick to reveal on the CPU every frame.
pub const MAX_SIDE: u32 = 900;

/// Moments are stored in this many steps.
pub(super) const STEPS: f32 = 65535.0;

pub use mdeck_sdk::stage::Strategy;

/// A picture ready to be drawn in: the SDK's [`mdeck_sdk::stage::Artwork`],
/// which the host hands to art engines as the slide's picture.
pub use mdeck_sdk::stage::Artwork as Prepared;

/// Decode, trim, key out the paper and lay out the time map.
pub fn prepare(bytes: &[u8], kind: ArtKind, strategy: Strategy) -> anyhow::Result<Prepared> {
    let img = image::load_from_memory(bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(prepare_image(img.to_rgba8(), kind, strategy))
}

pub fn prepare_image(img: RgbaImage, kind: ArtKind, strategy: Strategy) -> Prepared {
    let paper = paper_colour(&img);
    let img = fit(trim(&img), MAX_SIDE);
    let (w, h) = (img.width() as usize, img.height() as usize);
    let rgba = key_paper(&img, paper, kind, strategy);
    let ink: Vec<f32> = rgba.iter().map(|p| p[3] as f32 / 255.0).collect();
    let (when, path) = match strategy {
        Strategy::Draw => draw_order(&ink, w, h, 1.0),
        Strategy::Hatch => hatch_order(&img, &ink, w, h),
        Strategy::Bloom => (bloom_order(&ink, w, h), Vec::new()),
        Strategy::Develop => (develop_order(&img), Vec::new()),
    };
    Prepared {
        width: w,
        height: h,
        rgba,
        when,
        path,
        strategy,
    }
}

/// Darkness of a pixel over white, 0..1.
pub(super) fn darkness(p: &image::Rgba<u8>) -> f32 {
    let l = 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
    let a = p[3] as f32 / 255.0;
    (1.0 - l / 255.0) * a
}

/// Crop to the subject: the box around everything darker than the paper,
/// with a little room. A photograph is kept whole.
fn trim(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if darkness(p) > 0.12 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return img.clone();
    }
    let pad = (w.max(h) as f32 * 0.02) as u32;
    let x0 = x0.saturating_sub(pad);
    let y0 = y0.saturating_sub(pad);
    let x1 = (x1 + pad).min(w - 1);
    let y1 = (y1 + pad).min(h - 1);
    img.view(x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image()
}

fn fit(img: RgbaImage, side: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w.max(h) <= side {
        return img;
    }
    let k = side as f32 / w.max(h) as f32;
    imageops::resize(
        &img,
        ((w as f32 * k).round() as u32).max(1),
        ((h as f32 * k).round() as u32).max(1),
        imageops::FilterType::Lanczos3,
    )
}

/// Make the paper transparent. Line art becomes white with its darkness as
/// alpha (the engine picks the ink); tonal art keeps its colours, unmixed
/// from white so it lies on any paper. A photograph stays opaque.
fn key_paper(img: &RgbaImage, paper: [f32; 3], kind: ArtKind, strategy: Strategy) -> Vec<[u8; 4]> {
    img.pixels()
        .map(|p| match (kind, strategy) {
            (_, Strategy::Develop) => [p[0], p[1], p[2], 255],
            (ArtKind::Line, _) => {
                // clean paper to nothing, full ink to full
                let lum = |c: [f32; 3]| 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
                let c = [p[0] as f32, p[1] as f32, p[2] as f32];
                let d = 1.0 - lum(c) / lum(paper).max(1.0);
                let d = ((d - 0.05) / 0.80).clamp(0.0, 1.0);
                [255, 255, 255, (d.powf(0.85) * 255.0) as u8]
            }
            (ArtKind::Tonal, _) => {
                // colour to alpha against the paper, so the paper vanishes
                // and what is on it lies on any sheet
                let c = [p[0] as f32, p[1] as f32, p[2] as f32];
                let a = (0..3)
                    .map(|k| (paper[k] - c[k]) / paper[k].max(1.0))
                    .fold(0.0_f32, f32::max);
                // what is barely off the paper is paper
                let a = a.clamp(0.0, 1.0);
                if a < 0.05 {
                    return [255, 255, 255, 0];
                }
                let un = |v: f32| ((v - (1.0 - a) * 255.0) / a).clamp(0.0, 255.0) as u8;
                [un(c[0]), un(c[1]), un(c[2]), (a * 255.0) as u8]
            }
        })
        .collect()
}

/// The paper's colour: the typical colour around the picture's edge,
/// assuming it is paper (a light edge); white when the edge is dark.
fn paper_colour(img: &RgbaImage) -> [f32; 3] {
    let (w, h) = img.dimensions();
    let mut ch: [Vec<u8>; 3] = Default::default();
    let mut take = |x: u32, y: u32| {
        let p = img.get_pixel(x, y);
        for k in 0..3 {
            ch[k].push(p[k]);
        }
    };
    for x in 0..w {
        take(x, 0);
        take(x, h - 1);
    }
    for y in 0..h {
        take(0, y);
        take(w - 1, y);
    }
    let mut out = [255.0; 3];
    for k in 0..3 {
        ch[k].sort_unstable();
        out[k] = ch[k][ch[k].len() * 6 / 10] as f32;
    }
    if out.iter().any(|v| *v < 190.0) {
        return [255.0; 3];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two strokes on white: a long horizontal line and a short dot.
    fn drawing() -> RgbaImage {
        let mut img = RgbaImage::from_pixel(200, 120, image::Rgba([255, 255, 255, 255]));
        for x in 20..180 {
            for y in 58..62 {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
            }
        }
        for x in 100..104 {
            for y in 100..104 {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
            }
        }
        img
    }

    #[test]
    fn trim_crops_to_the_subject() {
        let p = prepare_image(drawing(), ArtKind::Line, Strategy::Draw);
        // 160 x 46 of ink plus a little room
        assert!(p.width < 180 && p.width >= 160, "{}", p.width);
        assert!(p.height < 70 && p.height >= 46, "{}", p.height);
    }

    #[test]
    fn a_line_is_traced_from_one_end_to_the_other() {
        let p = prepare_image(drawing(), ArtKind::Line, Strategy::Draw);
        let row = p
            .rgba
            .iter()
            .position(|px| px[3] == 255)
            .map(|i| i / p.width)
            .expect("ink");
        let left = (0..p.width)
            .find(|&x| p.rgba[row * p.width + x][3] == 255)
            .unwrap();
        let right = (0..p.width)
            .rev()
            .find(|&x| p.rgba[row * p.width + x][3] == 255)
            .unwrap();
        let (a, b) = (p.when[row * p.width + left], p.when[row * p.width + right]);
        assert!(a.abs_diff(b) > 20_000, "a line takes time: {a} {b}");
        // bare paper is transparent, and the big stroke comes before the dot
        assert_eq!(p.rgba[0][3], 0);
        let dot = p.rgba.iter().rposition(|px| px[3] == 255).unwrap();
        assert!(p.when[dot] > a.min(b));
    }

    /// How much ink has arrived at `t` (the alpha the reveal shows).
    fn ink(p: &Prepared, t: f32) -> f32 {
        p.rgba
            .iter()
            .zip(p.coverage(t, 0.02))
            .map(|(px, k)| px[3] as f32 * k)
            .sum()
    }

    #[test]
    fn reveal_runs_from_nothing_to_the_picture() {
        let p = prepare_image(drawing(), ArtKind::Line, Strategy::Draw);
        let (none, half, full) = (ink(&p, 0.0), ink(&p, 0.5), ink(&p, 1.1));
        assert_eq!(none, 0.0);
        assert!(half > 0.0 && half < full, "{half} {full}");
        // the hand has a path to follow while it draws
        assert!(p.path.len() > 2);
        assert!(p.path.windows(2).all(|w| w[0].0 <= w[1].0));
    }

    #[test]
    fn every_strategy_finishes_as_the_picture() {
        let mut img = drawing();
        for x in 30..60 {
            for y in 20..40 {
                img.put_pixel(x, y, image::Rgba([120, 160, 200, 255]));
            }
        }
        for s in [
            Strategy::Draw,
            Strategy::Hatch,
            Strategy::Bloom,
            Strategy::Develop,
        ] {
            let kind = if s == Strategy::Draw {
                ArtKind::Line
            } else {
                ArtKind::Tonal
            };
            let p = prepare_image(img.clone(), kind, s);
            let done = p.coverage(1.0 + 1e-3, 1e-3);
            assert!(done.iter().all(|k| *k == 1.0), "{s:?}");
        }
    }

    #[test]
    fn tonal_art_unmixes_from_white() {
        let mut img = RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
        img.put_pixel(4, 4, image::Rgba([128, 128, 255, 255]));
        let px = key_paper(&img, [255.0; 3], ArtKind::Tonal, Strategy::Bloom);
        assert_eq!(px[0][3], 0);
        let p = px[4 * 8 + 4];
        // over white it gives back the colour
        let over =
            |c: u8| (c as f32 * p[3] as f32 / 255.0 + 255.0 * (1.0 - p[3] as f32 / 255.0)) as i32;
        assert!(
            (over(p[0]) - 128).abs() <= 3 && (over(p[2]) - 255).abs() <= 3,
            "{p:?}"
        );
    }
}
