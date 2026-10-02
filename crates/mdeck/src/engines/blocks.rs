//! The blocks engine: pictures are built from falling blocks. An
//! illustration, a countdown digit or the end words are quantised to a grid,
//! the cells grouped into pieces of up to four, and the pieces drop from
//! above the slide, bottom row first, land with a small bounce and settle
//! into the picture. Leaving a slide, the stack flashes and clears row by
//! row, the way a completed line does. Exports show the settled stack.

use std::sync::Arc;

use mdeck_sdk::cloud::Mask;
use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Needs};
use mdeck_sdk::paint::{Color as Color32, Mesh, Painter, Pos2, Rect, Vec2, mix, premul};
use mdeck_sdk::stage::{Frame, Look, Moment, PictureSource, Place, Stage};
use mdeck_sdk::tokens::Tokens;

use crate::engines::hash01;

/// Seconds into the end slide when the caption fades in: the words have
/// landed, held, and cleared.
pub const END_CAPTION_DELAY: f32 = 5.4;

pub static DEF: EngineDef = EngineDef {
    name: "blocks",
    summary: "Pictures built from falling blocks that land, settle and clear row by row.",
    capabilities: Capabilities {
        picture: true,
        ..Capabilities::NONE
    },
    settings: &[],
    needs: Needs { page: false },
    ending_caption_delay: END_CAPTION_DELAY,
    create: |_| Box::new(Blocks::new()),
    board: None,
};
const END_WORDS: f32 = 3.6;
/// Gravity, px/s² on a 1920x1080 slide.
const GRAVITY: f32 = 7200.0;
/// How long a clear takes.
const CLEAR: f32 = 0.55;

type Key = (usize, Look, usize, bool);

/// One piece: a few neighbouring cells of one colour that fall together.
#[derive(Clone, Debug)]
struct Piece {
    /// Cells as (column, row) in the picture's grid.
    cells: Vec<(i32, i32)>,
    colour: usize,
    /// When it starts falling, from the picture's start.
    drop_at: f32,
}

/// A picture made of pieces on a grid.
#[derive(Clone, Debug)]
struct Stack {
    pieces: Vec<Piece>,
    /// The grid: top-left corner and cell size, in slide fractions of the
    /// slide's height (both axes), so blocks stay square.
    origin: Pos2,
    cell: f32,
    /// Brightness (a title's backdrop is dimmer).
    weight: f32,
    born: f32,
}

pub struct Blocks {
    key: Option<Key>,
    now: f32,
    stack: Option<Stack>,
    /// The previous stack, clearing since the given time.
    clearing: Option<(Stack, f32)>,
    /// The countdown's burst: blocks fly apart (progress 0..1).
    burst: Option<f32>,
    /// Something moved in the last paint (a falling piece, a clear, a burst).
    moving: bool,
}

impl Blocks {
    pub fn new() -> Self {
        Self {
            key: None,
            now: 0.0,
            stack: None,
            clearing: None,
            burst: None,
            moving: true,
        }
    }

    fn build(&self, cx: &Frame, stage: &Stage, look: Look) -> Option<Stack> {
        let aspect = cx.rect.width() / cx.rect.height();
        let mask_place = |mask: &Mask, h: f32| {
            let w = h * mask.aspect / aspect;
            Place {
                u: 0.5 - w / 2.0,
                v: 0.47 - h / 2.0,
                w,
                h,
            }
        };
        let (points, place, rows, spread, weight): (&[[f32; 2]], Place, usize, f32, f32) =
            match (&stage.moment, look) {
                (Moment::Countdown { mask, .. }, _) => {
                    (&mask.points, mask_place(mask, 0.56), 15, 0.45, 1.0)
                }
                (Moment::End { words, .. }, Look::EndWords) => {
                    let w = 0.64;
                    let h = w / words.aspect * aspect;
                    let place = Place {
                        u: 0.5 - w / 2.0,
                        v: 0.47 - h / 2.0,
                        w,
                        h,
                    };
                    (&words.points, place, 9, 1.1, 1.0)
                }
                (Moment::Slide, _) => {
                    let fig = stage.picture.as_ref()?;
                    let PictureSource::Cloud(cloud) = &fig.source else {
                        return None;
                    };
                    let weight = if fig.backdrop { 0.42 } else { 1.0 };
                    (&cloud.points, fig.place, 26, 1.7, weight)
                }
                _ => return None,
            };
        let build = Build {
            rows,
            spread,
            weight,
        };
        Some(stack(
            points,
            place,
            aspect,
            build,
            self.now,
            stage.index as u32,
        ))
    }
}

impl Default for Blocks {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Blocks {
    fn update(&mut self, cx: &Frame, stage: &Stage) {
        let look = stage.moment.look(END_WORDS);
        let figure = match stage.picture.as_ref().map(|p| &p.source) {
            Some(PictureSource::Cloud(c)) => Arc::as_ptr(c) as usize,
            _ => 0,
        };
        let key = (stage.index, look, figure, stage.title);
        if self.key != Some(key) {
            match look {
                Look::Burst => self.burst = Some(0.0),
                _ => {
                    self.burst = None;
                    if let Some(s) = self.stack.take() {
                        self.clearing = Some((s, self.now));
                    }
                    if look != Look::EndOut {
                        self.stack = self.build(cx, stage, look);
                    }
                }
            }
            self.key = Some(key);
        }
        if let Moment::Burst { progress } = stage.moment {
            self.burst = Some(progress);
        }
        if cx.still {
            self.clearing = None;
            if let Some(s) = &mut self.stack {
                s.born = self.now - 60.0;
            }
            return;
        }
        self.now += cx.dt;
        if self
            .clearing
            .as_ref()
            .is_some_and(|(_, since)| self.now - since > CLEAR + 0.3)
        {
            self.clearing = None;
        }
    }

    fn paint(&mut self, painter: &mut Painter, cx: &Frame, _stage: &Stage) {
        let rect = cx.rect;
        let palette = palette(cx.tokens);
        let mut mesh = Mesh::default();
        let mut moving = false;

        if let Some((old, since)) = &self.clearing {
            let t = self.now - since;
            for piece in &old.pieces {
                for &(c, r) in &piece.cells {
                    // rows clear from the top down, each flashing first
                    let row_delay = (r as f32 / 30.0).clamp(0.0, 1.0) * 0.25;
                    let u = ((t - row_delay) / CLEAR).clamp(0.0, 1.0);
                    if u >= 1.0 {
                        continue;
                    }
                    let flash = (1.0 - (u / 0.25)).clamp(0.0, 1.0);
                    let shrink = 1.0 - (((u - 0.25) / 0.75).clamp(0.0, 1.0)).powi(2);
                    let colour = mix(palette[piece.colour], Color32::WHITE, flash * 0.85);
                    let cell = cell_rect(old, c, r, rect);
                    block(
                        &mut mesh,
                        Rect::from_center_size(cell.center(), cell.size() * shrink),
                        colour,
                        old.weight * cx.opacity * (0.4 + 0.6 * shrink),
                    );
                }
            }
            moving = true;
        }

        if let Some(stack) = &self.stack {
            let t = self.now - stack.born;
            let unit = rect.height();
            let g = GRAVITY * cx.scale;
            for (k, piece) in stack.pieces.iter().enumerate() {
                let local = t - piece.drop_at;
                if local < 0.0 {
                    moving = true;
                    continue;
                }
                // fall from just above the slide to the resting place
                let top = piece.cells.iter().map(|c| c.1).min().unwrap_or(0);
                let rest_y = stack.origin.y * unit + top as f32 * stack.cell * unit;
                let fall = rest_y + stack.cell * unit * 4.0;
                let t_land = (2.0 * fall / g).sqrt();
                let (dy, squash) = if local < t_land {
                    moving = true;
                    (-fall + 0.5 * g * local * local, 1.0)
                } else {
                    let u = ((local - t_land) / 0.16).min(1.0);
                    if u < 1.0 {
                        moving = true;
                    }
                    let bounce = (u * std::f32::consts::PI).sin() * (1.0 - u);
                    (-bounce * stack.cell * unit * 0.22, 1.0 - 0.10 * bounce)
                };
                let mut offset = Vec2::new(0.0, dy);
                let mut fade = 1.0;
                if let Some(b) = self.burst {
                    // the digit bursts: every piece flies out from the centre
                    let centre = rect.center();
                    let p = cell_rect(stack, piece.cells[0].0, piece.cells[0].1, rect).center();
                    let dir = (p - centre).normalized();
                    let spin = hash01(k as u32 * 17) * 0.6 + 0.7;
                    offset += dir * b * b * rect.width() * spin;
                    fade = (1.0 - b * 1.3).clamp(0.0, 1.0);
                    moving = true;
                }
                for &(c, r) in &piece.cells {
                    let cell = cell_rect(stack, c, r, rect).translate(offset);
                    let h = cell.height() * squash;
                    let cell =
                        Rect::from_min_max(Pos2::new(cell.left(), cell.bottom() - h), cell.max);
                    block(
                        &mut mesh,
                        cell,
                        palette[piece.colour],
                        stack.weight * cx.opacity * fade,
                    );
                }
            }
        }
        painter.mesh(mesh);
        self.moving = moving && !cx.still;
    }

    fn animating(&self) -> bool {
        self.moving
    }
}

/// The screen rect of grid cell (`c`, `r`), with a small gap around it.
fn cell_rect(stack: &Stack, c: i32, r: i32, rect: Rect) -> Rect {
    let unit = rect.height();
    let s = stack.cell * unit;
    let min = Pos2::new(
        rect.left() + stack.origin.x * rect.width() + c as f32 * s,
        rect.top() + stack.origin.y * unit + r as f32 * s,
    );
    Rect::from_min_size(min, Vec2::new(s, s)).shrink(s * 0.05)
}

/// A bevelled block: a light top and left edge, a dark bottom and right,
/// the face slightly lit from above.
fn block(mesh: &mut Mesh, r: Rect, colour: Color32, opacity: f32) {
    if opacity <= 0.0 || r.width() <= 0.5 || r.height() <= 0.5 {
        return;
    }
    let bevel = (r.width().min(r.height()) * 0.16).max(1.0);
    let inner = r.shrink(bevel);
    let light = premul(mix(colour, Color32::WHITE, 0.42), opacity);
    let side = premul(mix(colour, Color32::WHITE, 0.12), opacity);
    let dark = premul(mix(colour, Color32::BLACK, 0.42), opacity);
    let shade = premul(mix(colour, Color32::BLACK, 0.22), opacity);
    let face_top = premul(mix(colour, Color32::WHITE, 0.06), opacity);
    let face_bottom = premul(mix(colour, Color32::BLACK, 0.06), opacity);
    // four trapezoids around the face
    let trap = |mesh: &mut Mesh, pts: [Pos2; 4], c: Color32| {
        let base = mesh.vertices.len() as u32;
        for p in pts {
            mesh.vertex(p, Pos2::ZERO, c);
        }
        mesh.triangle(base, base + 1, base + 2);
        mesh.triangle(base, base + 2, base + 3);
    };
    let (lt, rt, rb, lb) = corners(r);
    let (ilt, irt, irb, ilb) = corners(inner);
    trap(mesh, [lt, rt, irt, ilt], light);
    trap(mesh, [lt, ilt, ilb, lb], side);
    trap(mesh, [irt, rt, rb, irb], shade);
    trap(mesh, [ilb, irb, rb, lb], dark);
    let base = mesh.vertex(ilt, Pos2::ZERO, face_top);
    mesh.vertex(irt, Pos2::ZERO, face_top);
    mesh.vertex(irb, Pos2::ZERO, face_bottom);
    mesh.vertex(ilb, Pos2::ZERO, face_bottom);
    mesh.triangle(base, base + 1, base + 2);
    mesh.triangle(base, base + 2, base + 3);
}

/// A rect's corners: left top, right top, right bottom, left bottom.
fn corners(r: Rect) -> (Pos2, Pos2, Pos2, Pos2) {
    (
        r.min,
        Pos2::new(r.max.x, r.min.y),
        r.max,
        Pos2::new(r.min.x, r.max.y),
    )
}

/// The block colours, from the theme.
fn palette(theme: &Tokens) -> [Color32; 5] {
    [
        theme.accent,
        theme.secondary,
        theme.particle_cool,
        theme.accent_soft,
        theme.series[4],
    ]
}

/// Quantise `points` placed in `place` to a grid `rows` cells tall, group
/// the filled cells into pieces of up to four and time their drops over
/// `spread` seconds, bottom row first.
/// How a picture is stacked: its grid `rows` tall, its drops spread over
/// `spread` seconds, and how strongly it shows (`weight`).
#[derive(Clone, Copy)]
struct Build {
    rows: usize,
    spread: f32,
    weight: f32,
}

fn stack(
    points: &[[f32; 2]],
    place: Place,
    aspect: f32,
    build: Build,
    born: f32,
    seed: u32,
) -> Stack {
    let Build {
        rows,
        spread,
        weight,
    } = build;
    // cell size in slide-height fractions; columns follow the picture's width
    let cell = place.h / rows as f32;
    let cols = ((place.w * aspect) / cell).ceil().max(1.0) as usize;
    let mut count = vec![0u32; rows * cols];
    for p in points {
        let c = ((p[0] * place.w * aspect) / cell).floor() as usize;
        let r = ((p[1] * place.h) / cell).floor() as usize;
        if c < cols && r < rows {
            count[r * cols + c] += 1;
        }
    }
    // a cell is filled when it holds a fair share of points
    let mut filled_counts: Vec<u32> = count.iter().copied().filter(|n| *n > 0).collect();
    filled_counts.sort_unstable();
    let median = filled_counts
        .get(filled_counts.len() / 2)
        .copied()
        .unwrap_or(1);
    let threshold = (median as f32 * 0.45).ceil().max(1.0) as u32;
    let filled: Vec<bool> = count.iter().map(|n| *n >= threshold).collect();

    // group into pieces: bottom rows first, each piece growing into free
    // neighbours up to a size between two and four
    let mut owner = vec![usize::MAX; rows * cols];
    let mut pieces: Vec<Piece> = Vec::new();
    for r in (0..rows).rev() {
        for c in 0..cols {
            let i = r * cols + c;
            if !filled[i] || owner[i] != usize::MAX {
                continue;
            }
            let id = pieces.len();
            let target = 2 + (hash01(seed.wrapping_mul(977) + i as u32) * 3.0) as usize;
            let mut cells = vec![(c as i32, r as i32)];
            owner[i] = id;
            let mut k = 0;
            while cells.len() < target && k < cells.len() {
                let (cc, rr) = cells[k];
                for (dc, dr) in [(1, 0), (0, -1), (-1, 0), (0, 1)] {
                    let (nc, nr) = (cc + dc, rr + dr);
                    if nc < 0 || nr < 0 || nc >= cols as i32 || nr >= rows as i32 {
                        continue;
                    }
                    let j = nr as usize * cols + nc as usize;
                    if filled[j] && owner[j] == usize::MAX && cells.len() < target {
                        owner[j] = id;
                        cells.push((nc, nr));
                    }
                }
                k += 1;
            }
            let colour = (hash01(seed.wrapping_add(id as u32 * 31)) * 5.0) as usize % 5;
            pieces.push(Piece {
                cells,
                colour,
                drop_at: 0.0,
            });
        }
    }
    // pieces are already bottom-up; spread their drops, a little uneven
    let n = pieces.len().max(1) as f32;
    for (k, p) in pieces.iter_mut().enumerate() {
        p.drop_at = (k as f32 / n) * spread + hash01(k as u32 * 7 + seed) * 0.06;
    }
    // the grid's origin in slide fractions: x in slide widths, y in heights
    let width_frac = cols as f32 * cell / aspect;
    let origin = Pos2::new(place.u + (place.w - width_frac) / 2.0, place.v);
    Stack {
        pieces,
        origin: Pos2::new(origin.x, origin.y),
        cell,
        weight,
        born,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RING: Build = Build {
        rows: 20,
        spread: 1.5,
        weight: 1.0,
    };

    fn square_ring() -> Vec<[f32; 2]> {
        let mut pts = Vec::new();
        for k in 0..200 {
            let t = k as f32 / 199.0;
            pts.extend([[t, 0.0], [t, 0.999], [0.0, t], [0.999, t]]);
        }
        pts
    }

    #[test]
    fn a_ring_becomes_a_ring_of_pieces_dropped_bottom_first() {
        let place = Place {
            u: 0.55,
            v: 0.1,
            w: 0.3,
            h: 0.8,
        };
        let s = stack(&square_ring(), place, 16.0 / 9.0, RING, 0.0, 1);
        let cells: Vec<(i32, i32)> = s.pieces.iter().flat_map(|p| p.cells.clone()).collect();
        assert!(cells.iter().any(|c| c.1 == 0) && cells.iter().any(|c| c.1 == 19));
        // hollow: the middle stays empty
        assert!(!cells.contains(&(5, 10)));
        // every piece is 1 to 4 cells, and the first drops are low rows
        assert!(s.pieces.iter().all(|p| (1..=4).contains(&p.cells.len())));
        let first_row = s.pieces[0].cells.iter().map(|c| c.1).max().unwrap();
        assert_eq!(first_row, 19);
        let last = s.pieces.last().unwrap();
        assert!(last.drop_at > s.pieces[0].drop_at);
        assert!(last.drop_at <= 1.5 + 0.06);
    }

    #[test]
    fn blocks_are_square_on_screen() {
        let place = Place {
            u: 0.55,
            v: 0.1,
            w: 0.3,
            h: 0.8,
        };
        let s = stack(&square_ring(), place, 16.0 / 9.0, RING, 0.0, 1);
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        let r = cell_rect(&s, 3, 4, rect);
        assert!((r.width() - r.height()).abs() < 0.01, "{r:?}");
    }
}
