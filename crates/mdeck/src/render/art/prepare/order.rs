//! The time maps: when each pixel of a picture appears, for each
//! [`super::Strategy`].

use std::collections::VecDeque;

use image::RgbaImage;

use super::{STEPS, darkness};
use crate::engines::hash01;
use mdeck_sdk::paint::smoothstep;

const NEIGHBOURS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Moments along the ink: every connected stroke is traced from the end
/// nearest where the pen last lifted, the big shapes first and the details
/// after, and the faint edges around the ink arrive with it. The whole
/// drawing fills `0..span` of the reveal.
pub(super) fn draw_order(
    ink: &[f32],
    w: usize,
    h: usize,
    span: f32,
) -> (Vec<u16>, Vec<(f32, f32, f32)>) {
    let n = w * h;
    let (comp, comps) = strokes(ink, w, h);
    let mut when = vec![u16::MAX; n];
    let mut path = Vec::new();
    if comps.is_empty() {
        return (vec![0; n], path);
    }
    let order = tour(&comps, w);

    // Time: each stroke takes a share of the drawing that grows with its
    // ink, but small ones are not rushed to nothing.
    let mut queue = VecDeque::new();
    let weights: Vec<f32> = order
        .iter()
        .map(|&c| (comps[c].len() as f32).powf(0.75))
        .collect();
    let sum: f32 = weights.iter().sum();
    let mut clock = 0.0_f32;
    let mut pen = (0usize, 0usize);
    let mut dist = vec![u32::MAX; n];
    for (&c, wt) in order.iter().zip(&weights) {
        let dur = wt / sum;
        let members = &comps[c];
        // start at the member nearest the pen
        let start = *members
            .iter()
            .min_by_key(|&&i| {
                let (x, y) = (i as usize % w, i as usize / w);
                let dx = x as i64 - pen.0 as i64;
                let dy = y as i64 - pen.1 as i64;
                dx * dx + dy * dy
            })
            .expect("a stroke has pixels") as usize;
        dist[start] = 0;
        queue.clear();
        queue.push_back(start);
        let mut visited = Vec::with_capacity(members.len());
        let mut far = 0u32;
        while let Some(i) = queue.pop_front() {
            visited.push(i);
            far = far.max(dist[i]);
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let j = ny as usize * w + nx as usize;
                if comp[j] == c as u32 && dist[j] == u32::MAX {
                    dist[j] = dist[i] + 1;
                    queue.push_back(j);
                }
            }
        }
        let far = far.max(1) as f32;
        let samples = (visited.len() / 40).max(2);
        for (k, &i) in visited.iter().enumerate() {
            let t = (clock + dur * dist[i] as f32 / far) * span;
            when[i] = (t.clamp(0.0, 1.0) * STEPS) as u16;
            if k % samples == 0 || k + 1 == visited.len() {
                path.push((t, (i % w) as f32 / w as f32, (i / w) as f32 / h as f32));
            }
        }
        let last = *visited.last().expect("visited");
        pen = (last % w, last / w);
        clock += dur;
    }
    path.sort_by(|a, b| a.0.total_cmp(&b.0));

    // The soft edges around the ink arrive with the ink next to them.
    spread(&mut when, ink, w, h, 6);
    (when, path)
}

/// The connected strokes of ink: each pixel's stroke (`u32::MAX`: no ink)
/// and each stroke's pixels.
fn strokes(ink: &[f32], w: usize, h: usize) -> (Vec<u32>, Vec<Vec<u32>>) {
    const INK: f32 = 0.18;
    let n = w * h;
    let mut comp = vec![u32::MAX; n];
    let mut comps: Vec<Vec<u32>> = Vec::new();
    let mut queue = VecDeque::new();
    for start in 0..n {
        if comp[start] != u32::MAX || ink[start] < INK {
            continue;
        }
        let id = comps.len() as u32;
        let mut members = Vec::new();
        comp[start] = id;
        queue.push_back(start);
        while let Some(i) = queue.pop_front() {
            members.push(i as u32);
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let j = ny as usize * w + nx as usize;
                if comp[j] == u32::MAX && ink[j] >= INK {
                    comp[j] = id;
                    queue.push_back(j);
                }
            }
        }
        comps.push(members);
    }
    (comp, comps)
}

/// The order the strokes are drawn in: big shapes first (the outline of the
/// subject), then the details, each tier toured nearest-next from where the
/// pen lifted.
fn tour(comps: &[Vec<u32>], w: usize) -> Vec<usize> {
    let total: usize = comps.iter().map(Vec::len).sum();
    let big = (total as f32 * 0.004).max(24.0) as usize;
    let centroid = |c: &Vec<u32>| {
        let (mut sx, mut sy) = (0.0, 0.0);
        for &i in c {
            sx += (i as usize % w) as f32;
            sy += (i as usize / w) as f32;
        }
        (sx / c.len() as f32, sy / c.len() as f32)
    };
    let centres: Vec<(f32, f32)> = comps.iter().map(centroid).collect();
    let mut order: Vec<usize> = Vec::with_capacity(comps.len());
    let mut pen = (0.0_f32, 0.0_f32);
    for tier in [true, false] {
        let mut left: Vec<usize> = (0..comps.len())
            .filter(|&c| (comps[c].len() >= big) == tier)
            .collect();
        while !left.is_empty() {
            let (k, _) = left
                .iter()
                .enumerate()
                .map(|(k, &c)| {
                    let d = (centres[c].0 - pen.0).powi(2) + (centres[c].1 - pen.1).powi(2);
                    (k, d)
                })
                .fold((0, f32::MAX), |a, b| if b.1 < a.1 { b } else { a });
            let c = left.swap_remove(k);
            pen = centres[c];
            order.push(c);
        }
    }
    order
}

/// Give unset pixels (`u16::MAX`) that carry any ink the moment of their
/// nearest set neighbour, `rounds` pixels out; the rest appear at the end.
fn spread(when: &mut [u16], ink: &[f32], w: usize, h: usize, rounds: usize) {
    for _ in 0..rounds {
        let snapshot = when.to_vec();
        let mut changed = false;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if snapshot[i] != u16::MAX || ink[i] <= 0.0 {
                    continue;
                }
                let mut best = u16::MAX;
                for (dx, dy) in NEIGHBOURS {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    best = best.min(snapshot[ny as usize * w + nx as usize]);
                }
                if best != u16::MAX {
                    when[i] = best;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    for v in when.iter_mut() {
        if *v == u16::MAX {
            *v = STEPS as u16;
        }
    }
}

/// A graphite sketch: the strong edges are drawn first along the ink (the
/// first half), then the tone is laid in stroke by stroke in diagonal bands
/// that sweep across the picture, the pencil zigzagging along the front.
pub(super) fn hatch_order(
    img: &RgbaImage,
    ink: &[f32],
    w: usize,
    h: usize,
) -> (Vec<u16>, Vec<(f32, f32, f32)>) {
    const START: f32 = 0.48;
    const SPAN: f32 = 0.47;
    const BANDS: usize = 64;
    let edges = edge_strength(img);
    let lines: Vec<f32> = edges
        .iter()
        .zip(ink)
        .map(|(e, i)| if *e > 0.2 && *i > 0.35 { *i } else { 0.0 })
        .collect();
    let (mut when, mut path) = draw_order(&lines, w, h, 0.5);

    // The sweep runs along (0.8, 0.6); strokes lie across it.
    let (ax, ay) = (0.8_f32, 0.6_f32);
    let reach = ax * w as f32 + ay * h as f32;
    let along = |x: usize, y: usize| (ax * x as f32 + ay * y as f32) / reach;
    let across = |x: usize, y: usize| -ay * x as f32 + ax * y as f32;
    // how far the tone reaches across each band, for the pencil's path
    let mut extent = vec![(f32::MAX, f32::MIN); BANDS];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if ink[i] < 0.08 {
                continue;
            }
            let b = ((along(x, y) * BANDS as f32) as usize).min(BANDS - 1);
            let c = across(x, y);
            extent[b].0 = extent[b].0.min(c);
            extent[b].1 = extent[b].1.max(c);
        }
    }
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if lines[i] >= 0.18 {
                continue;
            }
            // stroke by stroke (a few pixels wide), each a little ragged,
            // the darkest tone laid over the light
            let stroke = (along(x, y) * reach / 4.0).floor() as u32;
            let ragged =
                hash01(stroke.wrapping_mul(31) ^ (across(x, y) / 9.0) as i32 as u32) * 0.02;
            let t = START + SPAN * along(x, y) + ragged + 0.03 * ink[i];
            when[i] = (t.clamp(0.0, 1.0) * STEPS) as u16;
        }
    }

    // the pencil: back and forth along the front as it sweeps
    let steps = 240;
    for k in 0..=steps {
        let f = k as f32 / steps as f32;
        let b = ((f * BANDS as f32) as usize).min(BANDS - 1);
        let (lo, hi) = extent[b];
        if lo > hi {
            continue;
        }
        let zig = {
            let p = (f * 70.0).fract();
            if p < 0.5 { p * 2.0 } else { 2.0 - p * 2.0 }
        };
        let c = lo + (hi - lo) * zig;
        let a = f * reach;
        // back from (along, across) to pixels
        let x = ax * a - ay * c;
        let y = ay * a + ax * c;
        path.push((START + SPAN * f, x / w as f32, y / h as f32));
    }
    path.sort_by(|a, b| a.0.total_cmp(&b.0));
    (when, path)
}

/// Watercolour: washes spread outward from a few pools where the paint is
/// heaviest, with ragged wet edges, the darkest paint settling last.
pub(super) fn bloom_order(ink: &[f32], w: usize, h: usize) -> Vec<u16> {
    // the pools: the heaviest cells of a coarse grid, a few of them
    const GRID: usize = 8;
    const POOLS: usize = 5;
    let (cw, ch) = (w.div_ceil(GRID), h.div_ceil(GRID));
    let mut cells: Vec<(f32, usize, usize)> = Vec::new();
    for gy in 0..GRID {
        for gx in 0..GRID {
            let (mut sum, mut n, mut best, mut at) = (0.0, 0usize, -1.0, (0, 0));
            for y in gy * ch..((gy + 1) * ch).min(h) {
                for x in gx * cw..((gx + 1) * cw).min(w) {
                    let v = ink[y * w + x];
                    sum += v;
                    n += 1;
                    if v > best {
                        best = v;
                        at = (x, y);
                    }
                }
            }
            if n > 0 {
                cells.push((sum / n as f32, at.0, at.1));
            }
        }
    }
    cells.sort_by(|a, b| b.0.total_cmp(&a.0));
    let pools: Vec<(f32, f32)> = cells
        .iter()
        .take(POOLS)
        .map(|c| (c.1 as f32, c.2 as f32))
        .collect();
    let mut dist = vec![0.0_f32; w * h];
    let mut far = 1.0_f32;
    for y in 0..h {
        for x in 0..w {
            let d = pools
                .iter()
                .map(|p| ((x as f32 - p.0).powi(2) + (y as f32 - p.1).powi(2)).sqrt())
                .fold(f32::MAX, f32::min);
            dist[y * w + x] = d;
            far = far.max(d);
        }
    }
    (0..w * h)
        .map(|i| {
            let wet = 0.82 * dist[i] / far + 0.16 * value_noise(i % w, i / w, 40.0);
            // the darkest paint settles a touch later than the wash around it
            let t = wet * 0.9 + 0.08 * ink[i];
            (t.clamp(0.0, 1.0) * STEPS) as u16
        })
        .collect()
}

/// A print in the developer: the shadows come up first, the highlights last.
pub(super) fn develop_order(img: &RgbaImage) -> Vec<u16> {
    img.pixels()
        .map(|p| {
            let t = 0.15 + 0.85 * (1.0 - darkness(p));
            (t.clamp(0.0, 1.0) * 0.7 * STEPS) as u16
        })
        .collect()
}

/// Sobel edge strength, 0..1.
fn edge_strength(img: &RgbaImage) -> Vec<f32> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    // blurred first, so the grain of the shading is not taken for an edge
    let soft = image::imageops::blur(img, 1.6);
    let l: Vec<f32> = soft.pixels().map(darkness).collect();
    let at = |x: i32, y: i32| {
        l[y.clamp(0, h as i32 - 1) as usize * w + x.clamp(0, w as i32 - 1) as usize]
    };
    let mut out = vec![0.0; w * h];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let gx = at(x + 1, y - 1) + 2.0 * at(x + 1, y) + at(x + 1, y + 1)
                - at(x - 1, y - 1)
                - 2.0 * at(x - 1, y)
                - at(x - 1, y + 1);
            let gy = at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1)
                - at(x - 1, y - 1)
                - 2.0 * at(x, y - 1)
                - at(x + 1, y - 1);
            out[y as usize * w + x as usize] = ((gx * gx + gy * gy).sqrt() / 2.0).min(1.0);
        }
    }
    out
}

/// Smooth noise in 0..1 with features about `cell` pixels across.
fn value_noise(x: usize, y: usize, cell: f32) -> f32 {
    let (fx, fy) = (x as f32 / cell, y as f32 / cell);
    let (ix, iy) = (fx.floor() as u32, fy.floor() as u32);
    let (tx, ty) = (fx.fract(), fy.fract());
    let s = |v: f32| smoothstep(0.0, 1.0, v);
    let c = |a: u32, b: u32| hash01(a.wrapping_mul(7919) ^ b.wrapping_mul(104_729));
    let top = c(ix, iy) + (c(ix + 1, iy) - c(ix, iy)) * s(tx);
    let bottom = c(ix, iy + 1) + (c(ix + 1, iy + 1) - c(ix, iy + 1)) * s(tx);
    top + (bottom - top) * s(ty)
}
