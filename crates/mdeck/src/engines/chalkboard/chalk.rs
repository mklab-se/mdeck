//! Drawing the chalkboard: the slate with the ghosts of drawings wiped off
//! it, chalk lines, the stick of chalk and the dust it sheds.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, vec2};

use super::super::hash01;
use super::super::paint::{SPRITE_GLOW, mix, premul};
use crate::render::strokes::{Picture, to_screen};
use crate::theme::Theme;

/// The board's colours.
pub(super) struct Chalk {
    pub(super) slate: Color32,
    /// White chalk.
    pub(super) white: Color32,
    /// The coloured chalks, for the ghosts on the board.
    pub(super) colours: [Color32; 3],
}

impl Chalk {
    pub(super) fn of(theme: &Theme) -> Self {
        Chalk {
            slate: theme.background,
            white: theme.heading_color,
            colours: [theme.accent, theme.accent_soft, theme.secondary],
        }
    }
}

/// A mote of chalk dust falling from the stick.
#[derive(Clone, Copy, Debug)]
pub(super) struct Mote {
    pub(super) pos: Pos2,
    pub(super) vel: egui::Vec2,
    pub(super) life: f32,
    pub(super) max: f32,
    pub(super) size: f32,
}

/// The slate: uneven, with soft clouds where it was wiped and the faint
/// ghosts of earlier drawings, always the same on every slide.
pub(super) fn slate(
    painter: &egui::Painter,
    texture: egui::TextureId,
    rect: Rect,
    scale: f32,
    c: &Chalk,
    opacity: f32,
) {
    let painter = &painter.with_clip_rect(rect);
    let mut mesh = egui::Mesh::with_texture(texture);
    // wiped clouds of dust
    for k in 0..34u32 {
        let x = rect.left() + hash01(k * 5 + 101) * rect.width();
        let y = rect.top() + hash01(k * 5 + 102) * rect.height();
        let rx = (180.0 + 380.0 * hash01(k * 5 + 103)) * scale;
        let ry = rx * (0.35 + 0.4 * hash01(k * 5 + 104));
        mesh.add_rect_with_uv(
            Rect::from_center_size(Pos2::new(x, y), vec2(rx * 2.0, ry * 2.0)),
            SPRITE_GLOW,
            premul(mix(c.slate, c.white, 0.6), 0.045 * opacity),
        );
    }
    // darker at the corners
    for (cx, cy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        let r = rect.width() * 0.4;
        mesh.add_rect_with_uv(
            Rect::from_center_size(
                Pos2::new(
                    rect.left() + cx * rect.width(),
                    rect.top() + cy * rect.height(),
                ),
                vec2(r * 2.0, r * 2.0),
            ),
            SPRITE_GLOW,
            premul(mix(c.slate, Color32::BLACK, 0.6), 0.25 * opacity),
        );
    }
    painter.add(egui::Shape::mesh(mesh));
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
        painter.add(egui::Shape::line(
            points,
            Stroke::new(9.0 * scale, premul(colour, 0.025 * opacity)),
        ));
    }
}

/// Chalk lines as far as the chalk has come: a dusty, broken stroke.
pub(super) fn chalk_lines(
    painter: &egui::Painter,
    pic: &Picture,
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
    let mut shapes = Vec::new();
    for i in 1..pic.points.len() {
        if !pic.pen[i] || pic.at[i - 1] > t {
            continue;
        }
        let a = to_screen(pic.points[i - 1], rect);
        let mut b = to_screen(pic.points[i], rect);
        if pic.at[i] > t {
            let f = (t - pic.at[i - 1]) / (pic.at[i] - pic.at[i - 1]).max(1e-4);
            b = a + (b - a) * f.clamp(0.0, 1.0);
        }
        // chalk skips: some segments are faint, and a dusty halo sits around all
        let skip = if hash01(i as u32 * 13 + 7) < 0.18 {
            0.45
        } else {
            1.0
        };
        let k = opacity * pic.weight.max(0.4);
        shapes.push(egui::Shape::line_segment(
            [a, b],
            Stroke::new(w * 2.4, premul(c.white, 0.07 * k)),
        ));
        shapes.push(egui::Shape::line_segment(
            [a, b],
            Stroke::new(w, premul(c.white, 0.82 * skip * k)),
        ));
    }
    painter.extend(shapes);
}

/// A stick of chalk held to the board at `tip`, worn at its point.
pub(super) fn stick(painter: &egui::Painter, tip: Pos2, scale: f32, c: &Chalk, opacity: f32) {
    if opacity <= 0.0 {
        return;
    }
    let d = vec2(0.62, -0.78); // from the tip up and to the right
    let n = d.rot90();
    let w = 15.0 * scale;
    let len = 78.0 * scale;
    let end = tip + d * len;
    // shadow on the board
    let off = vec2(12.0, 16.0) * scale;
    painter.add(egui::Shape::convex_polygon(
        vec![
            tip + off * 0.2 + n * w * 0.4,
            end + off + n * w * 0.5,
            end + off - n * w * 0.5,
            tip + off * 0.2 - n * w * 0.4,
        ],
        Color32::from_black_alpha((70.0 * opacity) as u8),
        Stroke::NONE,
    ));
    let body = mix(c.white, Color32::from_rgb(236, 232, 220), 0.5);
    painter.add(egui::Shape::convex_polygon(
        vec![
            tip + n * w * 0.36,
            end + n * w * 0.5,
            end - n * w * 0.5,
            tip - n * w * 0.36,
        ],
        premul(body, opacity),
        Stroke::NONE,
    ));
    // shading along one side, a rounded end, the worn face at the tip
    painter.add(egui::Shape::convex_polygon(
        vec![
            tip - n * w * 0.1,
            end - n * w * 0.15,
            end - n * w * 0.5,
            tip - n * w * 0.36,
        ],
        premul(mix(body, Color32::BLACK, 0.18), opacity),
        Stroke::NONE,
    ));
    painter.circle_filled(end, w * 0.5, premul(body, opacity));
    painter.circle_filled(
        tip,
        w * 0.36,
        premul(mix(body, Color32::WHITE, 0.4), opacity),
    );
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
            pos: tip + vec2(rand() - 0.5, rand() - 0.5) * 6.0 * scale,
            vel: vec2((rand() - 0.5) * 40.0, 10.0 + 30.0 * rand()) * scale,
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

pub(super) fn dust(painter: &egui::Painter, motes: &[Mote], c: &Chalk, opacity: f32) {
    for m in motes {
        let k = (m.life / m.max).clamp(0.0, 1.0);
        painter.circle_filled(m.pos, m.size, premul(c.white, 0.55 * k * opacity));
    }
}
