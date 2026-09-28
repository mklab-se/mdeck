//! The laser engine: a beam from in front of the screen etches each picture
//! onto the slide. An illustration, a countdown digit or the end words are
//! ordered into a drawing path (nearest neighbour, the pen lifted on long
//! jumps), and the beam traces it in a second or two. Fresh marks burn
//! white-hot and cool through yellow and orange to a pale etched line; sparks
//! fly off the tip and smoke drifts up from it. On charts and diagrams the
//! beam traces what the renderers drew. Exports show the finished, cooled
//! etching.

use std::sync::Arc;

use eframe::egui::{self, Pos2, Rect};

use super::paint::{SPRITE_CORE, SPRITE_GLOW, Sprites, additive, mix, premul};
use super::stage::{FrameCx, Look, Moment, Stage};
use super::{Capabilities, Engine, EngineDef};
use crate::render::illustration::Library;
use crate::render::strokes::{Picture, to_screen};
use draw::{Ink, beam, etching, line, surface};
use sparks::{Puff, Spark};

mod draw;
mod picture;
mod sparks;

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

type Key = (usize, Look, usize, u64, bool);

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
}

impl Default for Laser {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Laser {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, _lib: &mut Library) {
        let look = stage.moment.look(END_WORDS);
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
