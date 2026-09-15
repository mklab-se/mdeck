//! Image to point cloud.
//!
//! The bright pixels of an image, ordered by greedy farthest-point sampling
//! weighted by brightness: the brightest pixel first, then repeatedly the
//! candidate farthest from everything chosen so far, favouring bright ones.
//! The order is the point: a prefix of any length is a well-spread sample of
//! the whole shape, so the field can draw a sketch with sixty particles and
//! a figure with six hundred from the same file.

use std::sync::Arc;

use image::{DynamicImage, GenericImageView};

use super::{Cloud, MAX_POINTS, VERSION};
use crate::render::particles::Rng;

/// Long side the image is reduced to before sampling.
const WORK_SIZE: u32 = 512;
/// Pixels dimmer than this fraction of the brightest are background.
const THRESHOLD: f32 = 0.35;
/// Most candidates the ordering considers (a brightness-weighted subsample).
const MAX_CANDIDATES: usize = 20_000;
/// Ordering stops when the next point would sit closer than this to a chosen
/// one, as a fraction of the long side of the working image.
const MIN_SPACING: f32 = 0.006;
/// Dots this far apart (working pixels) merge into one stroke.
const DILATE_RADIUS: i32 = 2;
/// Ring radius (working pixels) used to tell edge pixels from interior ones.
const EDGE_RADIUS: i32 = 4;
/// How much an edge pixel outranks an interior one in the ordering.
const EDGE_WEIGHT: f32 = 1.2;

/// Reduce `img` to a cloud named `name`.
pub fn convert(img: &DynamicImage, name: &str, description: &str) -> Result<Cloud, String> {
    let points = order_points(img).ok_or("the image has no bright pixels to trace")?;
    let (points, aspect) = normalise(points);
    let cloud = Cloud {
        version: VERSION,
        name: name.to_string(),
        description: description.to_string(),
        prompt: None,
        generated: None,
        aspect,
        points: Arc::new(points),
    };
    cloud.validate()?;
    Ok(cloud)
}

/// Brightness (0..1) of every pixel of the working-size image, composited
/// over black, with its size.
fn luminance(img: &DynamicImage) -> (u32, u32, Vec<f32>) {
    let (w, h) = img.dimensions();
    let long = w.max(h).max(1);
    let img = if long > WORK_SIZE {
        let k = WORK_SIZE as f32 / long as f32;
        img.resize(
            ((w as f32 * k).round() as u32).max(1),
            ((h as f32 * k).round() as u32).max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        img.clone()
    };
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let lum = rgba
        .pixels()
        .map(|p| {
            let a = p[3] as f32 / 255.0;
            let r = p[0] as f32 / 255.0 * a;
            let g = p[1] as f32 / 255.0 * a;
            let b = p[2] as f32 / 255.0 * a;
            // perceived brightness with a little weight on the max channel so
            // saturated orange counts as bright as it looks
            (0.2126 * r + 0.7152 * g + 0.0722 * b) * 0.7 + r.max(g).max(b) * 0.3
        })
        .collect();
    (w, h, lum)
}

/// Grow every bright pixel into a disc of radius `r` (a max filter), so an
/// image drawn as separate dots becomes continuous strokes without dimming.
fn dilate(w: u32, h: u32, lum: &[f32], r: i32) -> Vec<f32> {
    let (wi, hi) = (w as i32, h as i32);
    let at = |x: i32, y: i32| lum[(y.clamp(0, hi - 1) * wi + x.clamp(0, wi - 1)) as usize];
    let mut tmp = vec![0.0f32; lum.len()];
    for y in 0..hi {
        for x in 0..wi {
            tmp[(y * wi + x) as usize] = (-r..=r).map(|d| at(x + d, y)).fold(0.0, f32::max);
        }
    }
    let at2 = |x: i32, y: i32| tmp[(y.clamp(0, hi - 1) * wi + x.clamp(0, wi - 1)) as usize];
    let mut out = vec![0.0f32; lum.len()];
    for y in 0..hi {
        for x in 0..wi {
            out[(y * wi + x) as usize] = (-r..=r).map(|d| at2(x, y + d)).fold(0.0, f32::max);
        }
    }
    out
}

/// Box-blur `lum` (radius `r`) so the shape becomes a solid mask instead of
/// the image's own particle texture.
fn blur(w: u32, h: u32, lum: &[f32], r: i32) -> Vec<f32> {
    let (wi, hi) = (w as i32, h as i32);
    let at = |x: i32, y: i32| lum[(y.clamp(0, hi - 1) * wi + x.clamp(0, wi - 1)) as usize];
    let mut tmp = vec![0.0f32; lum.len()];
    let n = (2 * r + 1) as f32;
    for y in 0..hi {
        for x in 0..wi {
            let s: f32 = (-r..=r).map(|d| at(x + d, y)).sum();
            tmp[(y * wi + x) as usize] = s / n;
        }
    }
    let at2 = |x: i32, y: i32| tmp[(y.clamp(0, hi - 1) * wi + x.clamp(0, wi - 1)) as usize];
    let mut out = vec![0.0f32; lum.len()];
    for y in 0..hi {
        for x in 0..wi {
            let s: f32 = (-r..=r).map(|d| at2(x, y + d)).sum();
            out[(y * wi + x) as usize] = s / n;
        }
    }
    out
}

/// Bright pixels in importance order, in working-image pixel coordinates.
fn order_points(img: &DynamicImage) -> Option<Vec<[f32; 2]>> {
    let (w, h, raw) = luminance(img);
    let lum = blur(w, h, &dilate(w, h, &raw, DILATE_RADIUS), 1);
    let peak = lum.iter().copied().fold(0.0f32, f32::max);
    if peak <= 0.0 {
        return None;
    }
    let cut = peak * THRESHOLD;
    let (wi, hi) = (w as i32, h as i32);
    let inside =
        |x: i32, y: i32| x >= 0 && y >= 0 && x < wi && y < hi && lum[(y * wi + x) as usize] >= cut;
    // How much of a ring around the pixel is background: 1 on the silhouette
    // edge, 0 deep inside. Edges are what a few particles should trace.
    let edge = |x: i32, y: i32| {
        let r = EDGE_RADIUS;
        let ring = [
            (r, 0),
            (-r, 0),
            (0, r),
            (0, -r),
            (r, r),
            (-r, r),
            (r, -r),
            (-r, -r),
        ];
        ring.iter()
            .filter(|(dx, dy)| !inside(x + dx, y + dy))
            .count() as f32
            / ring.len() as f32
    };
    let mut candidates: Vec<([f32; 2], f32)> = lum
        .iter()
        .enumerate()
        .filter(|&(_, &l)| l >= cut)
        .map(|(i, &l)| {
            let (x, y) = ((i as u32 % w) as i32, (i as u32 / w) as i32);
            let weight = 0.35 + 0.25 * (l / peak) + EDGE_WEIGHT * edge(x, y);
            ([x as f32 + 0.5, y as f32 + 0.5], weight)
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() > MAX_CANDIDATES {
        // weight-biased random subsample, deterministic
        let mut rng = Rng::new(0x1D_5EED);
        let keep = 2.0 * MAX_CANDIDATES as f32 / candidates.len() as f32;
        candidates.retain(|(_, b)| rng.unit() < keep * b);
        if candidates.len() > MAX_CANDIDATES {
            candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
            candidates.truncate(MAX_CANDIDATES);
        }
    }
    let min_spacing = MIN_SPACING * w.max(h) as f32;

    // Greedy farthest-point ordering, weighted.
    let n = candidates.len();
    let mut dist = vec![f32::INFINITY; n];
    let mut taken = vec![false; n];
    let mut out: Vec<[f32; 2]> = Vec::with_capacity(MAX_POINTS.min(n));
    let mut current = candidates
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.1.total_cmp(&b.1.1))
        .map(|(i, _)| i)?;
    loop {
        taken[current] = true;
        out.push(candidates[current].0);
        if out.len() >= MAX_POINTS || out.len() >= n {
            break;
        }
        let [cx, cy] = candidates[current].0;
        let mut best = None;
        let mut best_score = 0.0f32;
        let mut best_dist = 0.0f32;
        for i in 0..n {
            if taken[i] {
                continue;
            }
            let [x, y] = candidates[i].0;
            let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            if d < dist[i] {
                dist[i] = d;
            }
            let score = dist[i] * candidates[i].1;
            if score > best_score {
                best_score = score;
                best_dist = dist[i];
                best = Some(i);
            }
        }
        match best {
            Some(i) if best_dist >= min_spacing => current = i,
            _ => break,
        }
    }
    Some(out)
}

/// Fit points to their bounding box, returning unit-square points and the
/// box's height over width.
fn normalise(points: Vec<[f32; 2]>) -> (Vec<[f32; 2]>, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in &points {
        x0 = x0.min(p[0]);
        y0 = y0.min(p[1]);
        x1 = x1.max(p[0]);
        y1 = y1.max(p[1]);
    }
    let w = (x1 - x0).max(1.0);
    let h = (y1 - y0).max(1.0);
    let pts = points
        .into_iter()
        .map(|p| {
            [
                ((p[0] - x0) / w).clamp(0.0, 1.0),
                ((p[1] - y0) / h).clamp(0.0, 1.0),
            ]
        })
        .collect();
    (pts, h / w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    /// A bright ring on black, radius `r`, in a `size` square.
    fn ring(size: u32, r: f32) -> DynamicImage {
        let mut img = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 255]));
        let c = size as f32 / 2.0;
        for y in 0..size {
            for x in 0..size {
                let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
                if (d - r).abs() < 1.5 {
                    img.put_pixel(x, y, Rgba([255, 120, 40, 255]));
                }
            }
        }
        DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn a_ring_becomes_points_on_the_ring_with_square_aspect() {
        let cloud = convert(&ring(200, 60.0), "ring", "a ring").unwrap();
        assert!((cloud.aspect - 1.0).abs() < 0.05, "aspect {}", cloud.aspect);
        assert!(cloud.points.len() > 50);
        for p in cloud.points.iter() {
            let d = ((p[0] - 0.5).powi(2) + (p[1] - 0.5).powi(2)).sqrt();
            assert!((d - 0.5).abs() < 0.06, "point {p:?} is off the ring");
        }
    }

    #[test]
    fn the_first_points_are_spread_out() {
        let cloud = convert(&ring(200, 60.0), "ring", "").unwrap();
        let first: Vec<[f32; 2]> = cloud.points.iter().take(8).copied().collect();
        for i in 0..first.len() {
            for j in i + 1..first.len() {
                let d = ((first[i][0] - first[j][0]).powi(2) + (first[i][1] - first[j][1]).powi(2))
                    .sqrt();
                assert!(d > 0.3, "points {i} and {j} are only {d} apart");
            }
        }
    }

    #[test]
    fn conversion_is_deterministic_and_capped() {
        let a = convert(&ring(600, 250.0), "ring", "").unwrap();
        let b = convert(&ring(600, 250.0), "ring", "").unwrap();
        assert_eq!(a.points, b.points);
        assert!(a.points.len() <= MAX_POINTS);
    }

    #[test]
    fn tall_shapes_keep_their_aspect_and_transparent_pixels_are_black() {
        let mut img = RgbaImage::from_pixel(100, 300, Rgba([255, 255, 255, 0]));
        for y in 50..250 {
            for x in 40..60 {
                img.put_pixel(x, y, Rgba([255, 200, 100, 255]));
            }
        }
        let cloud = convert(&DynamicImage::ImageRgba8(img), "bar", "").unwrap();
        // dilation grows the 20 x 200 bar by DILATE_RADIUS on every side
        let grown = (200 + 2 * DILATE_RADIUS) as f32 / (20 + 2 * DILATE_RADIUS) as f32;
        assert!(
            (cloud.aspect - grown).abs() < 0.5,
            "aspect {} vs {grown}",
            cloud.aspect
        );
    }

    #[test]
    fn a_black_image_is_an_error() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 255])));
        assert!(convert(&img, "x", "").is_err());
    }
}
