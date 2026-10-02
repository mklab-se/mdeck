//! The `slate` surface: a chalkboard. The slate with the ghosts of drawings
//! wiped off it, chalk lines, the stick of chalk and the dust it sheds.

use mdeck_sdk::paint::{
    Color, Mesh, Painter, Pos2, Rect, SPRITE_GLOW, Stroke, Texture, Vec2, mix, premul,
};
use mdeck_sdk::stage::Frame;
use mdeck_sdk::tokens::Tokens;

use super::Pace;
use crate::engines::art::strokes::Strokes;
use crate::engines::art::{Drawing, Hand, Reveal, Tip, across, drawn_segment};
use crate::engines::hash01;

/// Draw a picture in 3.6 s; nothing is ruled after it.
pub(super) const PACE: Pace = Pace {
    draw: 3.6,
    after: 0.0,
    fade: 0.7,
};

const REVEAL: Reveal = Reveal {
    soft: 0.02,
    ghost: 0.0,
    ghost_speed: 1.0,
    grain: 0.72,
};

/// Chalk: pictures and strokes in chalk white, the dust it sheds, and the
/// stick at the tip.
pub(super) struct Stick<'a> {
    pub(super) chalk: Chalk,
    pub(super) motes: &'a [Mote],
}

impl Hand for Stick<'_> {
    fn backdrop(&self) -> f32 {
        0.42
    }

    fn picture(
        &self,
        painter: &Painter,
        frame: &Frame,
        d: &mut Drawing,
        now: f32,
        k: f32,
        _: bool,
    ) {
        d.paint(
            painter,
            frame.rect,
            now,
            premul(self.chalk.white, k),
            REVEAL,
        );
    }

    fn strokes(&self, painter: &Painter, frame: &Frame, p: &Strokes, now: f32, k: f32, _: bool) {
        chalk_lines(painter, p, now, frame.rect, frame.scale, &self.chalk, k);
    }

    fn finish(&self, painter: &Painter, frame: &Frame, tip: Option<Tip>) {
        dust(painter, self.motes, &self.chalk, frame.opacity);
        if let Some(at) = tip.and_then(Tip::in_front) {
            stick(painter, at, frame.scale, &self.chalk, frame.opacity);
        }
    }

    fn busy(&self) -> bool {
        !self.motes.is_empty()
    }
}

/// The board's colours.
pub(super) struct Chalk {
    pub(super) slate: Color,
    /// White chalk.
    pub(super) white: Color,
    /// The coloured chalks, for the ghosts on the board.
    pub(super) colours: [Color; 3],
}

impl Chalk {
    pub(super) fn of(t: &Tokens) -> Self {
        Chalk {
            slate: t.background,
            white: t.heading,
            colours: [t.accent, t.accent_soft, t.secondary],
        }
    }
}

/// A mote of chalk dust falling from the stick.
#[derive(Clone, Copy, Debug)]
pub(super) struct Mote {
    pub(super) pos: Pos2,
    pub(super) vel: Vec2,
    pub(super) life: f32,
    pub(super) max: f32,
    pub(super) size: f32,
}

/// The slate: uneven, with soft clouds where it was wiped and the faint
/// ghosts of earlier drawings, always the same on every slide.
pub(super) fn slate(
    painter: &Painter,
    texture: &Texture,
    rect: Rect,
    scale: f32,
    c: &Chalk,
    opacity: f32,
) {
    let painter = &painter.with_clip(rect);
    let mut mesh = Mesh::with_texture(texture.clone());
    // wiped clouds of dust
    for k in 0..34u32 {
        let x = rect.left() + hash01(k * 5 + 101) * rect.width();
        let y = rect.top() + hash01(k * 5 + 102) * rect.height();
        let rx = (180.0 + 380.0 * hash01(k * 5 + 103)) * scale;
        let ry = rx * (0.35 + 0.4 * hash01(k * 5 + 104));
        mesh.add_rect_uv(
            Rect::from_center_size(Pos2::new(x, y), Vec2::new(rx * 2.0, ry * 2.0)),
            SPRITE_GLOW,
            premul(mix(c.slate, c.white, 0.6), 0.045 * opacity),
        );
    }
    // darker at the corners
    for (cx, cy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        let r = rect.width() * 0.4;
        mesh.add_rect_uv(
            Rect::from_center_size(
                Pos2::new(
                    rect.left() + cx * rect.width(),
                    rect.top() + cy * rect.height(),
                ),
                Vec2::new(r * 2.0, r * 2.0),
            ),
            SPRITE_GLOW,
            premul(mix(c.slate, Color::BLACK, 0.6), 0.25 * opacity),
        );
    }
    painter.mesh(mesh);
    // ghosts of wiped drawings: loose arcs and strokes, barely there
    for k in 0..16u32 {
        let cx = rect.left() + hash01(k * 7 + 301) * rect.width();
        let cy = rect.top() + hash01(k * 7 + 302) * rect.height();
        let r = (40.0 + 160.0 * hash01(k * 7 + 303)) * scale;
        let a0 = hash01(k * 7 + 304) * std::f32::consts::TAU;
        let sweep = 1.2 + 3.5 * hash01(k * 7 + 305);
        let colour = if k % 4 == 3 {
            c.colours[(k as usize / 4) % 3]
        } else {
            c.white
        };
        let points: Vec<Pos2> = (0..=28)
            .map(|i| {
                let a = a0 + sweep * i as f32 / 28.0;
                let wob = 1.0 + 0.08 * (a * 3.0 + k as f32).sin();
                Pos2::new(cx + a.cos() * r * wob, cy + a.sin() * r * 0.8 * wob)
            })
            .collect();
        painter.line(
            points,
            Stroke::new(9.0 * scale, premul(colour, 0.025 * opacity)),
        );
    }
}

/// Chalk lines as far as the chalk has come: a dusty, broken stroke.
pub(super) fn chalk_lines(
    painter: &Painter,
    pic: &Strokes,
    now: f32,
    rect: Rect,
    scale: f32,
    c: &Chalk,
    opacity: f32,
) {
    if opacity <= 0.0 || pic.points.len() < 2 {
        return;
    }
    let t = now - pic.born;
    let w = 3.4 * scale * pic.weight.max(0.6);
    for i in 1..pic.points.len() {
        let Some((a, b)) = drawn_segment(pic, i, t, rect) else {
            continue;
        };
        // chalk skips: some segments are faint, and a dusty halo sits around all
        let skip = if hash01(i as u32 * 13 + 7) < 0.18 {
            0.45
        } else {
            1.0
        };
        let k = opacity * pic.weight.max(0.4);
        painter.line_segment([a, b], Stroke::new(w * 2.4, premul(c.white, 0.07 * k)));
        painter.line_segment([a, b], Stroke::new(w, premul(c.white, 0.82 * skip * k)));
    }
}

/// A stick of chalk held to the board at `tip`, worn at its point.
pub(super) fn stick(painter: &Painter, tip: Pos2, scale: f32, c: &Chalk, opacity: f32) {
    if opacity <= 0.0 {
        return;
    }
    let d = Vec2::new(0.62, -0.78); // from the tip up and to the right
    let n = across(d);
    let w = 15.0 * scale;
    let len = 78.0 * scale;
    let end = tip + d * len;
    // shadow on the board
    let off = Vec2::new(12.0, 16.0) * scale;
    painter.convex_polygon(
        vec![
            tip + off * 0.2 + n * w * 0.4,
            end + off + n * w * 0.5,
            end + off - n * w * 0.5,
            tip + off * 0.2 - n * w * 0.4,
        ],
        Color::from_rgba_premultiplied(0, 0, 0, (70.0 * opacity) as u8),
        Stroke::NONE,
    );
    let body = mix(c.white, Color::from_rgb(236, 232, 220), 0.5);
    painter.convex_polygon(
        vec![
            tip + n * w * 0.36,
            end + n * w * 0.5,
            end - n * w * 0.5,
            tip - n * w * 0.36,
        ],
        premul(body, opacity),
        Stroke::NONE,
    );
    // shading along one side, a rounded end, the worn face at the tip
    painter.convex_polygon(
        vec![
            tip - n * w * 0.1,
            end - n * w * 0.15,
            end - n * w * 0.5,
            tip - n * w * 0.36,
        ],
        premul(mix(body, Color::BLACK, 0.18), opacity),
        Stroke::NONE,
    );
    painter.circle_filled(end, w * 0.5, premul(body, opacity));
    painter.circle_filled(tip, w * 0.36, premul(mix(body, Color::WHITE, 0.4), opacity));
}

/// Dust shed where the chalk meets the board.
pub(super) fn emit(motes: &mut Vec<Mote>, tip: Pos2, scale: f32, dt: f32, seed: &mut u32) {
    let mut rand = || {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 17;
        *seed ^= *seed << 5;
        (*seed & 0x00FF_FFFF) as f32 / 16_777_215.0
    };
    let n = (dt * 70.0) as usize + usize::from(rand() < (dt * 70.0).fract());
    for _ in 0..n {
        let max = 0.6 + 0.9 * rand();
        motes.push(Mote {
            pos: tip + Vec2::new(rand() - 0.5, rand() - 0.5) * 6.0 * scale,
            vel: Vec2::new((rand() - 0.5) * 40.0, 10.0 + 30.0 * rand()) * scale,
            life: max,
            max,
            size: (0.8 + 1.6 * rand()) * scale,
        });
    }
}

pub(super) fn step(motes: &mut Vec<Mote>, dt: f32, scale: f32) {
    for m in motes.iter_mut() {
        m.vel.y += 160.0 * scale * dt;
        m.vel.x *= 1.0 - 1.5 * dt;
        m.pos += m.vel * dt;
        m.life -= dt;
    }
    motes.retain(|m| m.life > 0.0);
}

pub(super) fn dust(painter: &Painter, motes: &[Mote], c: &Chalk, opacity: f32) {
    for m in motes {
        let k = (m.life / m.max).clamp(0.0, 1.0);
        painter.circle_filled(m.pos, m.size, premul(c.white, 0.55 * k * opacity));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chalk_dust_falls_and_settles() {
        let (mut motes, mut seed) = (Vec::new(), 7u32);
        emit(&mut motes, Pos2::new(100.0, 100.0), 1.0, 0.1, &mut seed);
        assert!(!motes.is_empty());
        let y0 = motes[0].pos.y;
        step(&mut motes, 0.1, 1.0);
        assert!(motes.iter().all(|m| m.pos.y > y0 - 10.0));
        for _ in 0..40 {
            step(&mut motes, 0.1, 1.0);
        }
        assert!(motes.is_empty(), "dust settles within its life");
    }
}
