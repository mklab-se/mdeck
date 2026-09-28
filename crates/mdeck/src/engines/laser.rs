//! The laser engine: a beam from in front of the screen etches each picture
//! onto the slide. An illustration, a countdown digit or the end words are
//! ordered into a drawing path (nearest neighbour, the pen lifted on long
//! jumps), and the beam traces it in a second or two. Fresh marks burn
//! white-hot and cool through yellow and orange to a pale etched line; sparks
//! fly off the tip and smoke drifts up from it. On charts and diagrams the
//! beam traces what the renderers drew. Exports show the finished, cooled
//! etching.

use std::sync::Arc;

use eframe::egui::{self, Color32, Pos2, Rect};

use super::paint::{SPRITE_CORE, SPRITE_GLOW, Sprites, additive, mix, premul};
use super::stage::{FrameCx, Mask, Moment, Place, Stage};
use super::{Capabilities, Engine, EngineDef, hash01};
use crate::render::hints::Hint;
use crate::render::illustration::Library;
use crate::render::strokes::{Picture, plan, to_screen, toured};
use crate::theme::Theme;

/// Seconds into the end slide when the caption fades in: the words are
/// etched, held, and have faded.
pub const END_CAPTION_DELAY: f32 = 5.6;

pub static DEF: EngineDef = EngineDef {
    capabilities: Capabilities::PICTURES,
    create: || Box::new(Laser::new()),
    end_caption_delay: END_CAPTION_DELAY,
    medium: None,
    render_slide: None,
    problems: None,
};
/// The end words hold this long, then fade.
const END_WORDS: f32 = 3.8;
/// How long a mark stays hot (seconds, e-folding).
const HEAT: f32 = 0.32;
/// How long the faint glow of fresh marks lingers.
const LINGER: f32 = 2.6;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Look {
    Slide,
    Digit(u8),
    Burst,
    EndWords,
    EndOut,
}

type Key = (usize, Look, usize, u64, bool);

#[derive(Clone, Copy, Debug)]
struct Spark {
    pos: Pos2,
    vel: egui::Vec2,
    life: f32,
    max: f32,
}

#[derive(Clone, Copy, Debug)]
struct Puff {
    pos: Pos2,
    life: f32,
    max: f32,
    size: f32,
    drift: f32,
}

pub struct Laser {
    key: Option<Key>,
    now: f32,
    picture: Option<Picture>,
    /// The previous etching, fading out since the given time.
    fading: Option<(Picture, f32)>,
    /// The countdown's last digit flares and burns away (progress 0..1).
    flare: Option<f32>,
    sparks: Vec<Spark>,
    smoke: Vec<Puff>,
    /// Deterministic random stream for particles.
    rng: u32,
    sprites: Sprites,
}

impl Laser {
    pub fn new() -> Self {
        Self {
            key: None,
            now: 0.0,
            picture: None,
            fading: None,
            flare: None,
            sparks: Vec::new(),
            smoke: Vec::new(),
            rng: 0x2545_F491,
            sprites: Sprites::new("mdeck-laser-sprites"),
        }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng & 0x00FF_FFFF) as f32 / 16_777_215.0
    }

    fn build(&self, cx: &FrameCx, stage: &Stage, look: Look) -> Option<Picture> {
        let rect = cx.rect;
        let aspect = rect.width() / rect.height();
        let place_mask = |mask: &Mask, h: f32| -> Place {
            let w = h * mask.1 / aspect;
            Place {
                u: 0.5 - w / 2.0,
                v: 0.47 - h / 2.0,
                w,
                h,
            }
        };
        let (strokes, duration, weight): (Vec<Vec<Pos2>>, f32, f32) = match (&stage.moment, look) {
            (Moment::Countdown { mask, .. }, _) => {
                let place = place_mask(mask, 0.56);
                (vec![toured(&mask.0, place, aspect)], 0.85, 1.0)
            }
            (Moment::End { words, .. }, Look::EndWords) => {
                let w = 0.60;
                let h = w / words.1 * aspect;
                let place = Place {
                    u: 0.5 - w / 2.0,
                    v: 0.47 - h / 2.0,
                    w,
                    h,
                };
                (vec![toured(&words.0, place, aspect)], 1.9, 1.0)
            }
            (Moment::Slide, _) => {
                let mut strokes = Vec::new();
                let mut weight = 1.0;
                if let Some(fig) = &stage.figure {
                    strokes.push(toured(&fig.cloud.points, fig.place, aspect));
                    if fig.backdrop {
                        weight = 0.45;
                    }
                }
                strokes.extend(hint_strokes(stage.hints, rect));
                if strokes.is_empty() {
                    return None;
                }
                let duration = if stage.figure.is_some() { 2.3 } else { 1.6 };
                (strokes, duration, weight)
            }
            _ => return None,
        };
        Some(plan(strokes, duration, weight, aspect, self.now))
    }

    fn emit(&mut self, tip: Pos2, rect: Rect, scale: f32, dt: f32) {
        let p = Pos2::new(
            rect.left() + tip.x * rect.width(),
            rect.top() + tip.y * rect.height(),
        );
        let sparks = (dt * 260.0).round() as usize + usize::from(self.rand() < dt * 260.0 % 1.0);
        for _ in 0..sparks {
            let a = self.rand() * std::f32::consts::TAU;
            let speed = (120.0 + 520.0 * self.rand()) * scale;
            let max = 0.18 + 0.4 * self.rand();
            self.sparks.push(Spark {
                pos: p,
                vel: egui::vec2(a.cos() * speed, a.sin() * speed - 180.0 * scale),
                life: max,
                max,
            });
        }
        if self.rand() < dt * 38.0 {
            let max = 1.8 + 1.4 * self.rand();
            let size = (10.0 + 12.0 * self.rand()) * scale;
            let drift = self.rand() * 6.0;
            self.smoke.push(Puff {
                pos: p,
                life: max,
                max,
                size,
                drift,
            });
        }
    }

    fn step_particles(&mut self, dt: f32, scale: f32) {
        let g = 900.0 * scale;
        for s in &mut self.sparks {
            s.vel.y += g * dt;
            s.vel *= 1.0 - 1.6 * dt;
            s.pos += s.vel * dt;
            s.life -= dt;
        }
        self.sparks.retain(|s| s.life > 0.0);
        for p in &mut self.smoke {
            p.pos.y -= (26.0 + 10.0 * (p.drift).sin()) * scale * dt;
            p.pos.x += (self.now * 0.9 + p.drift).sin() * 9.0 * scale * dt;
            p.size += 14.0 * scale * dt;
            p.life -= dt;
        }
        self.smoke.retain(|p| p.life > 0.0);
    }
}

impl Default for Laser {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Laser {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, _lib: &mut Library) {
        let look = match &stage.moment {
            Moment::Slide => Look::Slide,
            Moment::Countdown { digit, .. } => Look::Digit(*digit),
            Moment::Burst { .. } => Look::Burst,
            Moment::End { elapsed, .. } if *elapsed < END_WORDS => Look::EndWords,
            Moment::End { .. } => Look::EndOut,
        };
        let figure = stage
            .figure
            .as_ref()
            .map(|f| Arc::as_ptr(&f.cloud) as usize)
            .unwrap_or(0);
        let key = (stage.index, look, figure, stage.hints_key, stage.title);
        if self.key != Some(key) {
            match look {
                Look::Burst => self.flare = Some(0.0),
                Look::EndOut => {
                    if let Some(p) = self.picture.take() {
                        self.fading = Some((p, self.now));
                    }
                }
                _ => {
                    self.flare = None;
                    if let Some(p) = self.picture.take() {
                        self.fading = Some((p, self.now));
                    }
                    self.picture = self.build(cx, stage, look);
                }
            }
            self.key = Some(key);
        }
        if let Moment::Burst { progress } = stage.moment {
            self.flare = Some(progress);
        }
        if cx.still {
            // the finished, cooled etching
            self.fading = None;
            self.sparks.clear();
            self.smoke.clear();
            if let Some(p) = &mut self.picture {
                p.born = self.now - p.duration - 60.0;
            }
            return;
        }
        self.now += cx.dt;
        if let Some(p) = &self.picture
            && let Some((tip, on)) = p.tip(self.now - p.born)
            && on
        {
            self.emit(tip, cx.rect, cx.scale, cx.dt);
        }
        if self.flare.is_some_and(|f| f < 0.6) {
            // the burning digit throws sparks all along its strokes
            let points = self
                .picture
                .as_ref()
                .map(|p| p.points.clone())
                .unwrap_or_default();
            if !points.is_empty() {
                for _ in 0..6 {
                    let k = (self.rand() * points.len() as f32) as usize % points.len();
                    self.emit(points[k], cx.rect, cx.scale, cx.dt * 0.6);
                }
            }
        }
        self.step_particles(cx.dt, cx.scale);
        if self
            .fading
            .as_ref()
            .is_some_and(|(_, since)| self.now - since > 1.2)
        {
            self.fading = None;
        }
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, _stage: &Stage) {
        let texture = self.sprites.id(ui.ctx());
        let theme = cx.theme;
        let rect = cx.rect;
        let scale = cx.scale;
        let ink = Ink::of(theme);
        let mut mesh = egui::Mesh::with_texture(texture);
        surface(&mut mesh, rect, theme, scale, cx.opacity);

        if let Some((old, since)) = &self.fading {
            let fade = (1.0 - (self.now - since) / 0.9).clamp(0.0, 1.0) * cx.opacity;
            etching(&mut mesh, cx, old, self.now, &ink, fade, None);
        }
        let flare = self.flare;
        if let Some(p) = &self.picture {
            let fade = match flare {
                Some(f) => (1.0 - f * 1.4).clamp(0.0, 1.0),
                None => 1.0,
            } * cx.opacity;
            etching(&mut mesh, cx, p, self.now, &ink, fade, flare);
        }

        // smoke drifts up from where the beam has been
        for s in &self.smoke {
            let life = s.life / s.max;
            let a = 0.10 * life * (1.0 - life).min(0.3) / 0.3 * cx.opacity;
            mesh.add_rect_with_uv(
                Rect::from_center_size(s.pos, egui::vec2(s.size * 2.0, s.size * 2.0)),
                SPRITE_GLOW,
                premul(ink.smoke, a),
            );
        }

        // the beam, from in front of the screen to the tip
        if let Some(p) = &self.picture
            && let Some((tip, on)) = p.tip(self.now - p.born)
            && on
            && !cx.still
        {
            let tip = to_screen(tip, rect);
            let source = Pos2::new(
                rect.center().x + (tip.x - rect.center().x) * 0.22,
                rect.bottom() + rect.height() * 0.62,
            );
            let flicker = 0.85 + 0.15 * (self.now * 97.0).sin();
            beam(
                &mut mesh,
                source,
                tip,
                12.0 * scale,
                1.2 * scale,
                &ink,
                flicker * cx.opacity,
            );
            let glow = 30.0 * scale;
            mesh.add_rect_with_uv(
                Rect::from_center_size(tip, egui::vec2(glow * 2.0, glow * 2.0)),
                SPRITE_GLOW,
                additive(ink.hot, 0.9 * flicker * cx.opacity),
            );
            let core = 7.0 * scale;
            mesh.add_rect_with_uv(
                Rect::from_center_size(tip, egui::vec2(core * 2.0, core * 2.0)),
                SPRITE_CORE,
                additive(ink.white, flicker * cx.opacity),
            );
        }
        for s in &self.sparks {
            let life = s.life / s.max;
            let tail = s.pos - s.vel * 0.018;
            let c = mix(ink.hot, ink.white, life * life);
            line(
                &mut mesh,
                tail,
                s.pos,
                1.3 * scale,
                additive(c, life * cx.opacity),
            );
        }
        ui.painter().add(egui::Shape::mesh(mesh));
        let busy = !self.sparks.is_empty()
            || !self.smoke.is_empty()
            || self.fading.is_some()
            || self
                .picture
                .as_ref()
                .is_some_and(|p| self.now - p.born < p.duration + LINGER);
        if busy && !cx.still {
            ui.ctx().request_repaint();
        }
    }
}

/// The etching's colours: white-hot, hot, cooling, and the etched line.
struct Ink {
    white: Color32,
    hot: Color32,
    warm: Color32,
    etched: Color32,
    groove: Color32,
    smoke: Color32,
    light: bool,
}

impl Ink {
    fn of(theme: &Theme) -> Self {
        let light = theme.is_light();
        Ink {
            white: theme.particle_light,
            hot: theme.accent,
            warm: theme.secondary,
            etched: if light {
                mix(theme.background, theme.heading_color, 0.7)
            } else {
                mix(
                    mix(theme.background, theme.heading_color, 0.80),
                    theme.accent_soft,
                    0.12,
                )
            },
            groove: mix(
                theme.background,
                Color32::BLACK,
                if light { 0.25 } else { 0.55 },
            ),
            smoke: if light {
                mix(theme.background, Color32::BLACK, 0.4)
            } else {
                mix(theme.background, theme.foreground, 0.55)
            },
            light,
        }
    }

    /// A mark's colour `heat` (1: just etched, 0: cold).
    fn at(&self, heat: f32) -> Color32 {
        if heat > 0.66 {
            mix(self.warm, self.white, (heat - 0.66) / 0.34)
        } else if heat > 0.33 {
            mix(self.hot, self.warm, (heat - 0.33) / 0.33)
        } else {
            mix(self.etched, self.hot, heat / 0.33)
        }
    }
}

/// Draw an etching as far as the beam has come: each mark cools with age,
/// rough edged, over a groove, with a lingering glow while fresh. `flare`
/// (the countdown's burst) heats everything again as it burns away.
fn etching(
    mesh: &mut egui::Mesh,
    cx: &FrameCx,
    pic: &Picture,
    now: f32,
    ink: &Ink,
    opacity: f32,
    flare: Option<f32>,
) {
    let (rect, scale) = (cx.rect, cx.scale);
    if opacity <= 0.0 || pic.points.len() < 2 {
        return;
    }
    let t = now - pic.born;
    let width = 2.8 * scale;
    let mut glows: Vec<(Pos2, Pos2, f32)> = Vec::new();
    for i in 1..pic.points.len() {
        if !pic.pen[i] || pic.at[i - 1] > t {
            continue;
        }
        let a = to_screen(pic.points[i - 1], rect);
        let mut b = to_screen(pic.points[i], rect);
        if pic.at[i] > t {
            // the segment being etched right now
            let f = (t - pic.at[i - 1]) / (pic.at[i] - pic.at[i - 1]).max(1e-4);
            b = a + (b - a) * f.clamp(0.0, 1.0);
        }
        let age = (t - pic.at[i]).max(0.0);
        let mut heat = (-age / HEAT).exp();
        if let Some(f) = flare {
            heat = heat.max((1.0 - f * 1.6).clamp(0.0, 1.0));
        }
        let color = ink.at(heat);
        let w = width * (1.0 + 0.7 * heat) * pic.weight.max(0.6);
        // groove under the mark, the mark, and a rough second pass beside it
        let n = (b - a).normalized().rot90();
        line(
            mesh,
            a + n * 0.7 * scale,
            b + n * 0.7 * scale,
            w,
            premul(ink.groove, 0.5 * opacity * pic.weight),
        );
        line(
            mesh,
            a,
            b,
            w,
            premul(color, opacity * (0.55 + 0.45 * pic.weight)),
        );
        let j = (hash01(i as u32 * 13) - 0.5) * 1.6 * scale;
        line(
            mesh,
            a + n * j,
            b - n * j,
            w * 0.55,
            premul(mix(color, ink.white, 0.15), 0.45 * opacity * pic.weight),
        );
        // a faint warmth stays in the engraving after the glow has gone
        let linger = (-age / LINGER).exp() * 0.5 + heat * 0.8 + 0.12;
        if linger > 0.03 && !ink.light {
            glows.push((a, b, linger));
        }
    }
    for (a, b, g) in glows {
        line(
            mesh,
            a,
            b,
            9.0 * scale,
            additive(ink.hot, 0.16 * g * opacity * pic.weight),
        );
    }
}

/// A tapered beam from `from` (wide, faint) to `to` (thin, bright).
fn beam(mesh: &mut egui::Mesh, from: Pos2, to: Pos2, w0: f32, w1: f32, ink: &Ink, k: f32) {
    let n = (to - from).normalized().rot90();
    let base = mesh.vertices.len() as u32;
    let uv = SOLID_UV;
    for (p, w, c) in [
        (from, w0, additive(ink.hot, 0.05 * k)),
        (to, w1 * 3.0, additive(ink.hot, 0.55 * k)),
    ] {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p + n * w,
            uv,
            color: c,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p - n * w,
            uv,
            color: c,
        });
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 1, base + 3, base + 2);
    // the white core, brightest at the tip
    let base = mesh.vertices.len() as u32;
    for (p, w, c) in [
        (from, w1 * 0.8, additive(ink.white, 0.0)),
        (to, w1, additive(ink.white, 0.9 * k)),
    ] {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p + n * w,
            uv,
            color: c,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p - n * w,
            uv,
            color: c,
        });
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 1, base + 3, base + 2);
}

/// The middle of the sprite sheet's core: a white texel, for untextured quads.
const SOLID_UV: Pos2 = Pos2 { x: 0.5, y: 0.5 };

fn line(mesh: &mut egui::Mesh, a: Pos2, b: Pos2, w: f32, color: Color32) {
    let d = b - a;
    if d.length() < 0.01 {
        return;
    }
    let n = d.normalized().rot90() * (w / 2.0);
    let base = mesh.vertices.len() as u32;
    for p in [a + n, a - n, b + n, b - n] {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p,
            uv: SOLID_UV,
            color,
        });
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 1, base + 3, base + 2);
}

/// The surface being etched: a fine, even grain, the same on every slide.
fn surface(mesh: &mut egui::Mesh, rect: Rect, theme: &Theme, scale: f32, opacity: f32) {
    let grain = if theme.is_light() {
        mix(theme.background, Color32::BLACK, 0.5)
    } else {
        mix(theme.background, Color32::WHITE, 0.5)
    };
    for k in 0..900u32 {
        let x = hash01(k * 3 + 1);
        let y = hash01(k * 3 + 2);
        let a = 0.025 + 0.05 * hash01(k * 3 + 3);
        let r = (0.6 + 0.9 * hash01(k * 7 + 5)) * scale;
        let p = Pos2::new(
            rect.left() + x * rect.width(),
            rect.top() + y * rect.height(),
        );
        mesh.add_rect_with_uv(
            Rect::from_center_size(p, egui::vec2(r * 2.0, r * 2.0)),
            SPRITE_CORE,
            premul(grain, a * opacity),
        );
    }
}

/// What the renderers drew, as strokes for the beam: lines and edges,
/// circles, and the tops of bars.
fn hint_strokes(hints: &[Hint], rect: Rect) -> Vec<Vec<Pos2>> {
    let frac = |p: Pos2| {
        Pos2::new(
            (p.x - rect.left()) / rect.width(),
            (p.y - rect.top()) / rect.height(),
        )
    };
    let mut out = Vec::new();
    for h in hints {
        match h {
            Hint::Path(points) if points.len() > 1 => {
                // densify so the beam's pace follows the line's length
                let mut s = Vec::new();
                for w in points.windows(2) {
                    let steps = ((w[1] - w[0]).length() / 6.0).ceil().max(1.0) as usize;
                    for k in 0..steps {
                        s.push(frac(w[0] + (w[1] - w[0]) * (k as f32 / steps as f32)));
                    }
                }
                s.push(frac(*points.last().expect("two points")));
                out.push(s);
            }
            Hint::Circle { center, radius } => {
                let r = radius + 10.0;
                out.push(
                    (0..=96)
                        .map(|k| {
                            let a = k as f32 / 96.0 * std::f32::consts::TAU;
                            frac(*center + egui::vec2(a.cos() * r, a.sin() * r))
                        })
                        .collect(),
                );
            }
            Hint::Bar(b) => {
                let y = b.top() - 6.0;
                let steps = (b.width() / 6.0).ceil().max(1.0) as usize;
                out.push(
                    (0..=steps)
                        .map(|k| frac(Pos2::new(b.left() + b.width() * k as f32 / steps as f32, y)))
                        .collect(),
                );
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_cool_from_white_to_the_etched_colour() {
        let ink = Ink::of(&Theme::dark());
        assert_eq!(ink.at(1.0), ink.white);
        assert_eq!(ink.at(0.0), ink.etched);
    }
}
