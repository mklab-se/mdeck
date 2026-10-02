//! The flaps of the board as quads in one mesh: each cell's resting
//! halves, its hinge and pins, the falling flap of a turning cell, and
//! the part of a character each half shows.

use mdeck_sdk::paint::{Color, Mesh, Painter, Pos2, Rect, Vec2};

use super::draw::{EMBOLDEN, Glyphs, Palette, View, WHITE, mix};
use super::layout::Cell;

/// The flaps of one board going into one mesh.
pub(super) struct Flaps<'a> {
    pub(super) painter: &'a Painter,
    /// A flap's height.
    pub(super) cell_h: f32,
    pub(super) pal: &'a Palette,
    pub(super) glyphs: Glyphs,
    /// The board's one mesh, on the font atlas.
    pub(super) mesh: Mesh,
    pub(super) opacity: f32,
    /// A flap half's corner radius, the hinge's height and a pin's width.
    pub(super) r: f32,
    pub(super) hinge: f32,
    pub(super) pin: f32,
}

impl Flaps<'_> {
    fn fade(&self, c: Color) -> Color {
        c.gamma_multiply(self.opacity)
    }

    /// One cell: its two resting halves, the hinge with its pins, and the
    /// falling flap while it turns.
    pub(super) fn cell(&mut self, cell: Rect, view: &View) {
        let (r, hinge) = (self.r, self.hinge);
        let mid = cell.center().y;
        let top = Rect::from_min_max(cell.min, Pos2::new(cell.right(), mid - hinge / 2.0));
        let bottom = Rect::from_min_max(Pos2::new(cell.left(), mid + hinge / 2.0), cell.max);
        let turning = view.t > 0.0;
        // static halves: the next character's top, the current one's bottom
        let upper = if turning { view.to } else { view.from };
        let lower = view.from;
        let (face_up, face_down) = (
            self.fade(self.pal.face(upper, true)),
            self.fade(self.pal.face(lower, false)),
        );
        rounded_half(&mut self.mesh, top, r, true, face_up);
        rounded_half(&mut self.mesh, bottom, r, false, face_down);
        self.glyph(cell, top, upper, self.opacity, None);
        self.glyph(cell, bottom, lower, self.opacity, None);
        // the hinge: a dark gap with a pin at each end
        let gap = self.fade(self.pal.gap);
        quad(
            &mut self.mesh,
            Rect::from_min_max(
                Pos2::new(cell.left(), mid - hinge / 2.0),
                Pos2::new(cell.right(), mid + hinge / 2.0),
            ),
            gap,
        );
        let pin = Vec2::new(self.pin, hinge * 2.2);
        let pin_color = self.fade(self.pal.pin);
        for x in [cell.left() + pin.x * 0.2, cell.right() - pin.x * 1.2] {
            quad(
                &mut self.mesh,
                Rect::from_min_size(Pos2::new(x, mid - pin.y / 2.0), pin),
                pin_color,
            );
        }
        if turning {
            self.falling(cell, top, bottom, view);
        }
    }

    /// The falling flap: the current top folds down, then the next bottom
    /// unfolds; it darkens as it turns edge-on.
    fn falling(&mut self, cell: Rect, top: Rect, bottom: Rect, view: &View) {
        let mid = cell.center().y;
        let angle = view.t * std::f32::consts::PI;
        let (half, face, first) = if view.t < 0.5 {
            (top, view.from, true)
        } else {
            (bottom, view.to, false)
        };
        let fold = angle.cos().abs();
        let widen = 0.10 * angle.sin();
        let shade = 1.0 - 0.45 * angle.sin();
        let cell_h = self.cell_h;
        let warp = |p: Pos2| -> Pos2 {
            let d = p.y - mid;
            let reach = (d.abs() / (cell_h / 2.0)).min(1.0);
            Pos2::new(
                cell.center().x + (p.x - cell.center().x) * (1.0 + widen * reach),
                mid + d * fold,
            )
        };
        let base = self.pal.face(face, first);
        let tint = self.fade(mix(Color::BLACK, base, shade));
        warped_quad(&mut self.mesh, half, WHITE, tint, &warp);
        self.glyph(cell, half, face, self.opacity * shade, Some(&warp));
    }

    /// The part of a cell's character inside `half`, optionally warped (a
    /// falling flap). Solid flaps and blanks draw nothing here.
    fn glyph(
        &mut self,
        cell: Rect,
        half: Rect,
        c: Cell,
        opacity: f32,
        warp: Option<&dyn Fn(Pos2) -> Pos2>,
    ) {
        // a list marker is a coloured bar, like a platform indicator, not the
        // font's small bullet; a hyphen is a bar on the upper flap, since
        // the font's thin one sits in the hinge and all but vanishes
        // ("SIGN UPS")
        let (rel, uv) = if let Some(bar) = bar_rect(c.ch, cell.size()) {
            (bar, WHITE)
        } else {
            match self.glyphs.get(self.painter, c.ch) {
                Some(q) => q,
                None => return,
            }
        };
        let quad_rect = rel.translate(cell.center().to_vec2());
        let clipped = quad_rect.intersect(half);
        if clipped.height() <= 0.0 || clipped.width() <= 0.0 {
            return;
        }
        // uv for the clipped part
        let fy = |y: f32| (y - quad_rect.top()) / quad_rect.height();
        let fx = |x: f32| (x - quad_rect.left()) / quad_rect.width();
        let uv_part = Rect::from_min_max(
            Pos2::new(
                uv.left() + uv.width() * fx(clipped.left()),
                uv.top() + uv.height() * fy(clipped.top()),
            ),
            Pos2::new(
                uv.left() + uv.width() * fx(clipped.right()),
                uv.top() + uv.height() * fy(clipped.bottom()),
            ),
        );
        let color = self
            .pal
            .glyph_color(c.style)
            .gamma_multiply(opacity.clamp(0.0, 1.0));
        let bold = if uv == WHITE {
            0.0
        } else {
            rel.height() * EMBOLDEN
        };
        for dx in [0.0, bold] {
            let part = clipped.translate(Vec2::new(dx, 0.0));
            match warp {
                Some(w) => warped_quad(&mut self.mesh, part, uv_part, color, w),
                None => self.mesh.add_rect_uv(part, uv_part, color),
            }
            if bold == 0.0 {
                break;
            }
        }
    }
}

/// The bar a flap draws for `ch` instead of the font's glyph, relative to
/// the cell's centre: the list marker, and the hyphen just above the hinge.
fn bar_rect(ch: char, cell: Vec2) -> Option<Rect> {
    match ch {
        '•' => Some(Rect::from_center_size(
            Pos2::ZERO,
            Vec2::new(cell.x * 0.26, cell.y * 0.56),
        )),
        '-' => Some(Rect::from_center_size(
            Pos2::new(0.0, -cell.y * 0.1),
            Vec2::new(cell.x * 0.42, cell.y * 0.09),
        )),
        _ => None,
    }
}

fn quad(mesh: &mut Mesh, rect: Rect, color: Color) {
    mesh.add_rect_uv(rect, WHITE, color);
}

fn warped_quad(mesh: &mut Mesh, rect: Rect, uv: Rect, color: Color, warp: &dyn Fn(Pos2) -> Pos2) {
    let corner = |r: Rect, right: bool, bottom: bool| {
        Pos2::new(
            if right { r.right() } else { r.left() },
            if bottom { r.bottom() } else { r.top() },
        )
    };
    let mut ids = [0u32; 4];
    for (k, (right, bottom)) in [(false, false), (true, false), (false, true), (true, true)]
        .into_iter()
        .enumerate()
    {
        ids[k] = mesh.vertex(
            warp(corner(rect, right, bottom)),
            corner(uv, right, bottom),
            color,
        );
    }
    mesh.triangle(ids[0], ids[1], ids[2]);
    mesh.triangle(ids[1], ids[3], ids[2]);
}

/// Half a flap with its two outer corners rounded, lit from above: a fan of
/// triangles from the centre so the corners stay smooth.
fn rounded_half(mesh: &mut Mesh, rect: Rect, r: f32, top: bool, color: Color) {
    let r = r.min(rect.height() * 0.5).min(rect.width() * 0.5);
    let light = if top {
        mix(color, Color::WHITE, 0.035)
    } else {
        color
    };
    let dark = if top {
        color
    } else {
        mix(color, Color::BLACK, 0.18)
    };
    let shade = |p: Pos2| -> Color {
        let t = ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
        mix(light, dark, t)
    };
    let mut outline: Vec<Pos2> = Vec::new();
    let arc = |outline: &mut Vec<Pos2>, c: Pos2, a0: f32| {
        for k in 0..=5 {
            let a = a0 + k as f32 / 5.0 * std::f32::consts::FRAC_PI_2;
            outline.push(Pos2::new(c.x + r * a.cos(), c.y + r * a.sin()));
        }
    };
    use std::f32::consts::PI;
    if top {
        arc(&mut outline, Pos2::new(rect.left() + r, rect.top() + r), PI);
        arc(
            &mut outline,
            Pos2::new(rect.right() - r, rect.top() + r),
            1.5 * PI,
        );
        outline.push(rect.max);
        outline.push(Pos2::new(rect.left(), rect.bottom()));
    } else {
        outline.push(rect.min);
        outline.push(Pos2::new(rect.right(), rect.top()));
        arc(
            &mut outline,
            Pos2::new(rect.right() - r, rect.bottom() - r),
            0.0,
        );
        arc(
            &mut outline,
            Pos2::new(rect.left() + r, rect.bottom() - r),
            0.5 * PI,
        );
    }
    let c = rect.center();
    let base = mesh.vertex(c, Pos2::ZERO, shade(c));
    for p in &outline {
        mesh.vertex(*p, Pos2::ZERO, shade(*p));
    }
    let n = outline.len() as u32;
    for k in 0..n {
        mesh.triangle(base, base + 1 + k, base + 1 + (k + 1) % n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hyphen_is_a_bar_that_shows_past_the_hinge() {
        // the hinge is 3.5% of the cell's height (draw.rs); the font's
        // hyphen was hidden behind it, so "SIGN-UPS" read "SIGN UPS"
        let cell = Vec2::new(40.0, 60.0);
        let bar = bar_rect('-', cell).expect("the hyphen is drawn as a bar");
        let hinge = cell.y * 0.035;
        assert!(bar.bottom() < -hinge, "clear of the hinge: {bar:?}");
        assert!(bar.height() > hinge * 1.5, "{bar:?}");
        assert!(
            bar.width() > bar.height() * 2.0,
            "a hyphen is wide, not a dot"
        );
        assert!(bar_rect('A', cell).is_none());
        assert!(super::super::wheel::wheel_index('-').is_some());
    }
}
