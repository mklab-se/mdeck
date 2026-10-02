//! Where the wall lights: a picture's points splatted onto the LEDs, the
//! light around what the slide's renderers drew, and the marquee ring.

use mdeck_sdk::geometry::Hint;
use mdeck_sdk::paint::{Pos2, Rect};
use mdeck_sdk::stage::Place;

use super::{Grid, PITCH};

/// Add each point's light to the LEDs around it (a small gaussian), so a
/// stroke of points lights a clean line of LEDs.
pub(super) fn splat(
    grid: &Grid,
    rect: Rect,
    points: &[[f32; 2]],
    place: Place,
    density: &mut [f32],
) {
    let sigma2 = 2.0 * 0.55f32.powi(2);
    for p in points {
        let x = rect.left() + (place.u + p[0] * place.w) * rect.width();
        let y = rect.top() + (place.v + p[1] * place.h) * rect.height();
        let (gx, gy) = grid.cell_of(Pos2::new(x, y));
        let (c0, r0) = (gx.round() as i64, gy.round() as i64);
        for r in (r0 - 2)..=(r0 + 2) {
            if r < 0 || r >= grid.rows as i64 {
                continue;
            }
            for c in (c0 - 2)..=(c0 + 2) {
                if c < 0 || c >= grid.cols as i64 {
                    continue;
                }
                let d2 = (c as f32 - gx).powi(2) + (r as f32 - gy).powi(2);
                density[r as usize * grid.cols + c as usize] += (-d2 / sigma2).exp();
            }
        }
    }
}

/// Light around the geometry a slide's renderers drew: a peak cap over each
/// bar like a level meter, a soft trail along lines and routed edges, a halo
/// ring around pies and donuts, a glint under each marked point.
pub(super) fn hint_light(grid: &Grid, hints: &[Hint]) -> Vec<f32> {
    let n = grid.len();
    let mut out = vec![0.0f32; n];
    let mut set = |c: i64, r: i64, v: f32| {
        if c >= 0 && r >= 0 && (c as usize) < grid.cols && (r as usize) < grid.rows {
            let i = r as usize * grid.cols + c as usize;
            out[i] = out[i].max(v);
        }
    };
    // Bars grow from a shared baseline: bottoms aligned means vertical bars.
    let bars: Vec<Rect> = hints
        .iter()
        .filter_map(|h| match h {
            Hint::Bar(b) => Some(*b),
            _ => None,
        })
        .collect();
    let aligned = |f: fn(&Rect) -> f32| {
        bars.iter()
            .filter(|a| bars.iter().filter(|b| (f(a) - f(b)).abs() < 2.0).count() * 2 > bars.len())
            .count()
    };
    let vertical = aligned(|r| r.bottom()) >= aligned(|r| r.left());
    for h in hints {
        match h {
            Hint::Bar(b) if vertical => {
                // a peak marker floating over the bar's value label, a dimmer
                // row above it, like a level meter's peak hold
                let (c0, r0) = grid.cell_of(b.min);
                let (c1, _) = grid.cell_of(Pos2::new(b.max.x, b.min.y));
                let row = (r0 - 0.5 - 38.0 / PITCH).floor() as i64;
                for c in (c0.ceil() as i64)..=(c1.floor() as i64) {
                    set(c, row, 0.95);
                    set(c, row - 1, 0.28);
                }
            }
            Hint::Bar(b) => {
                // horizontal: a marker past the bar's end and its value label
                let (c1, r0) = grid.cell_of(Pos2::new(b.max.x, b.min.y));
                let (_, r1) = grid.cell_of(b.max);
                let col = (c1 + 0.5 + 90.0 / PITCH).ceil() as i64;
                for r in (r0.ceil() as i64)..=(r1.floor() as i64) {
                    set(col, r, 0.95);
                    set(col + 1, r, 0.32);
                }
            }
            Hint::Path(points) => {
                for w in points.windows(2) {
                    let len = (w[1] - w[0]).length();
                    let steps = (len / (grid.pitch * 0.5)).ceil().max(1.0) as usize;
                    for k in 0..=steps {
                        let p = w[0] + (w[1] - w[0]) * (k as f32 / steps as f32);
                        let (gx, gy) = grid.cell_of(p);
                        for dr in -2..=2 {
                            for dc in -2..=2 {
                                let (c, r) = (gx.round() as i64 + dc, gy.round() as i64 + dr);
                                let d2 = (c as f32 - gx).powi(2) + (r as f32 - gy).powi(2);
                                set(c, r, 0.55 * (-d2 / 2.2).exp());
                            }
                        }
                    }
                }
            }
            Hint::Circle { center, radius } => {
                let ring = radius + grid.pitch * 1.6;
                let (gx, gy) = grid.cell_of(*center);
                let span = (ring / grid.pitch).ceil() as i64 + 2;
                for dr in -span..=span {
                    for dc in -span..=span {
                        let (c, r) = (gx.round() as i64 + dc, gy.round() as i64 + dr);
                        let d =
                            ((c as f32 - gx).powi(2) + (r as f32 - gy).powi(2)).sqrt() * grid.pitch;
                        let v = (-((d - ring) / (grid.pitch * 0.6)).powi(2)).exp();
                        set(c, r, 0.7 * v);
                    }
                }
            }
            Hint::Point(p) => {
                let (gx, gy) = grid.cell_of(*p);
                set(gx.round() as i64, gy.round() as i64 + 1, 0.7);
            }
            _ => {}
        }
    }
    out
}

/// The outermost ring of LEDs, numbered clockwise from the top left, for the
/// marquee border on title slides.
pub(super) fn border_positions(grid: &Grid) -> std::collections::HashMap<usize, usize> {
    let (w, h) = (grid.cols, grid.rows);
    let mut ring = std::collections::HashMap::new();
    let mut k = 0;
    let inset = 1;
    if w < 2 * inset + 2 || h < 2 * inset + 2 {
        return ring;
    }
    let (l, rgt, top, bot) = (inset, w - 1 - inset, inset, h - 1 - inset);
    for c in l..rgt {
        ring.insert(top * w + c, k);
        k += 1;
    }
    for r in top..bot {
        ring.insert(r * w + rgt, k);
        k += 1;
    }
    for c in (l + 1..=rgt).rev() {
        ring.insert(bot * w + c, k);
        k += 1;
    }
    for r in (top + 1..=bot).rev() {
        ring.insert(r * w + l, k);
        k += 1;
    }
    ring
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::paint::Vec2;

    #[test]
    fn a_stroke_of_points_lights_a_line_of_leds() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        let g = Grid::new(rect, 1.0);
        let mut d = vec![0.0; g.len()];
        // a horizontal stroke across the middle of the unit square
        let pts: Vec<[f32; 2]> = (0..200).map(|k| [k as f32 / 199.0, 0.5]).collect();
        let place = Place {
            u: 0.25,
            v: 0.25,
            w: 0.5,
            h: 0.5,
        };
        splat(&g, rect, &pts, place, &mut d);
        let (_, row) = g.cell_of(Pos2::new(960.0, 540.0));
        let row = row.round() as usize;
        let on = d[row * g.cols + g.cols / 2];
        let off = d[(row + 4) * g.cols + g.cols / 2];
        assert!(on > 1.0, "the stroke's row is lit: {on}");
        assert!(off < 0.01, "four rows away is dark: {off}");
    }

    #[test]
    fn the_marquee_border_is_one_closed_ring() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 120.0));
        let g = Grid::new(rect, 1.0);
        let ring = border_positions(&g);
        let expected = 2 * (g.cols - 2) + 2 * (g.rows - 2) - 4;
        assert_eq!(ring.len(), expected);
        let mut ks: Vec<usize> = ring.values().copied().collect();
        ks.sort_unstable();
        assert!(
            ks.windows(2).all(|w| w[1] == w[0] + 1),
            "numbered without gaps"
        );
    }
}
