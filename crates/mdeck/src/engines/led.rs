//! The LED engine: a fixed wall of RGB LEDs behind every slide. Unlit, the
//! lenses are barely visible; illustrations, countdown digits and the end
//! words appear by lighting LEDs, never by moving anything. A new picture
//! powers on in a sweep from its centre, each LED flickering as it strikes;
//! a lit picture shimmers slowly between the theme's colours. Title slides
//! get a chasing marquee border, other slides a faint aurora drifting across
//! the wall, away from the copy and out of the content's way.

use std::sync::Arc;

use eframe::egui::{self, Color32, Pos2, Rect};

use super::Engine;
use super::stage::{FrameCx, Moment, Place, Stage};
use crate::render::hints::Hint;
use crate::render::illustration::Library;
use crate::theme::Theme;

/// Seconds into the end slide when the caption fades in: the words have lit
/// and every LED has died out.
pub const END_CAPTION_DELAY: f32 = 5.4;

/// LED pitch in px on a 1920x1080 slide.
const PITCH: f32 = 13.5;
/// The end words stay lit this long, then the wall dies out.
const END_WORDS: f32 = 3.4;
/// Rise and fall rates of an LED toward its goal (per second).
const RISE: f32 = 16.0;
const FALL: f32 = 7.0;
/// How long a striking LED flickers.
const FLICKER: f32 = 0.16;

/// What the wall is showing, for deciding when to rebuild the picture.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Look {
    Slide,
    Digit(u8),
    Burst,
    EndWords,
    EndOut,
}

type Key = (usize, Look, usize, u64, bool);

#[derive(Clone, Copy, PartialEq, Debug)]
struct Grid {
    cols: usize,
    rows: usize,
    pitch: f32,
    origin: Pos2,
}

impl Grid {
    fn new(rect: Rect, scale: f32) -> Self {
        let pitch = (PITCH * scale).max(4.0);
        let cols = (rect.width() / pitch).floor().max(1.0) as usize;
        let rows = (rect.height() / pitch).floor().max(1.0) as usize;
        // centre the wall so the margins are even
        let origin = Pos2::new(
            rect.left() + (rect.width() - cols as f32 * pitch) / 2.0 + pitch / 2.0,
            rect.top() + (rect.height() - rows as f32 * pitch) / 2.0 + pitch / 2.0,
        );
        Grid {
            cols,
            rows,
            pitch,
            origin,
        }
    }

    fn len(&self) -> usize {
        self.cols * self.rows
    }

    fn centre(&self, i: usize) -> Pos2 {
        let (c, r) = (i % self.cols, i / self.cols);
        Pos2::new(
            self.origin.x + c as f32 * self.pitch,
            self.origin.y + r as f32 * self.pitch,
        )
    }

    /// Grid coordinates (fractional) of a point on screen.
    fn cell_of(&self, p: Pos2) -> (f32, f32) {
        (
            (p.x - self.origin.x) / self.pitch,
            (p.y - self.origin.y) / self.pitch,
        )
    }
}

pub struct Led {
    grid: Option<Grid>,
    /// The rect the grid was built for.
    rect: Rect,
    key: Option<Key>,
    /// Displayed brightness per LED (0..1, a strike may overshoot).
    level: Vec<f32>,
    /// What each LED is heading for now, and what it will head for once its
    /// wait runs out.
    goal: Vec<f32>,
    next: Vec<f32>,
    wait: Vec<f32>,
    /// Remaining flicker time of a striking LED.
    flick: Vec<f32>,
    /// Position in the palette (0..1) per LED, now and next.
    hue: Vec<f32>,
    next_hue: Vec<f32>,
    /// How much ambient light an LED may carry (0 over copy and content).
    zone: Vec<f32>,
    /// Seconds since the engine started, for shimmer and ambient motion.
    time: f32,
    /// Ambient strength for the current slide, and the marquee border.
    ambient: f32,
    border: bool,
    /// A reveal sweep running since this time.
    sweep: Option<f32>,
    reveal: (usize, usize),
    /// Burst ring progress (the countdown's last digit overloads the wall).
    burst: Option<f32>,
    sprites: Option<egui::TextureHandle>,
}

impl Led {
    pub fn new() -> Self {
        Self {
            grid: None,
            rect: Rect::NOTHING,
            key: None,
            level: Vec::new(),
            goal: Vec::new(),
            next: Vec::new(),
            wait: Vec::new(),
            flick: Vec::new(),
            hue: Vec::new(),
            next_hue: Vec::new(),
            zone: Vec::new(),
            time: 0.0,
            ambient: 0.0,
            border: false,
            sweep: None,
            reveal: (usize::MAX, 0),
            burst: None,
            sprites: None,
        }
    }

    fn resize(&mut self, grid: Grid) {
        let n = grid.len();
        self.grid = Some(grid);
        for v in [
            &mut self.level,
            &mut self.goal,
            &mut self.next,
            &mut self.wait,
            &mut self.flick,
        ] {
            *v = vec![0.0; n];
        }
        self.hue = vec![0.5; n];
        self.next_hue = vec![0.5; n];
        self.zone = vec![1.0; n];
        self.key = None;
    }

    /// Build the picture for the stage: per-LED target brightness and hue,
    /// the ambient zone, and power-on delays.
    fn build(&mut self, cx: &FrameCx, stage: &Stage, look: Look) {
        let grid = self.grid.expect("grid built before the picture");
        let rect = cx.rect;
        let rect_aspect = rect.width() / rect.height();
        let n = grid.len();
        let mut density = vec![0.0f32; n];
        let mut weight = 1.0;
        let mut centre = rect.center();
        let mut brisk = false;
        self.ambient = 0.0;
        self.border = false;

        match (&stage.moment, look) {
            (Moment::Countdown { mask, .. }, _) => {
                let h = 0.60;
                let w = h * mask.1 / rect_aspect;
                let place = Place {
                    u: 0.5 - w / 2.0,
                    v: 0.47 - h / 2.0,
                    w,
                    h,
                };
                splat(&grid, rect, &mask.0, place, &mut density);
                centre = rect.center();
                brisk = true;
                self.ambient = 0.025;
            }
            (Moment::End { words, .. }, Look::EndWords) => {
                let w = 0.64;
                let h = w / words.1 * rect_aspect;
                let place = Place {
                    u: 0.5 - w / 2.0,
                    v: 0.47 - h / 2.0,
                    w,
                    h,
                };
                splat(&grid, rect, &words.0, place, &mut density);
                brisk = true;
            }
            (Moment::Slide, _) => {
                if let Some(fig) = &stage.figure {
                    splat(&grid, rect, &fig.cloud.points, fig.place, &mut density);
                    let p = fig.place;
                    centre = Pos2::new(
                        rect.left() + (p.u + p.w / 2.0) * rect.width(),
                        rect.top() + (p.v + p.h / 2.0) * rect.height(),
                    );
                    if fig.backdrop {
                        weight = 0.34;
                    }
                }
                self.border = stage.title;
                self.ambient = if stage.figure.is_some() { 0.05 } else { 0.11 };
            }
            _ => {}
        }

        // ambient zone: calm over the copy column and inside content frames
        for i in 0..n {
            let p = grid.centre(i);
            let x = (p.x - rect.left()) / rect.width();
            let y = (p.y - rect.top()) / rect.height();
            let mut z = if stage.title {
                // centred copy: keep the middle calm, light the edges
                let dx = (x - 0.5) / 0.42;
                let dy = (y - 0.5) / 0.36;
                smoothstep(0.75, 1.25, (dx * dx + dy * dy).sqrt())
            } else {
                smoothstep(0.36, 0.62, x)
            };
            for h in stage.hints {
                if let Hint::Frame(r) = h
                    && r.expand(grid.pitch).contains(p)
                {
                    z = 0.0;
                }
            }
            self.zone[i] = z;
        }

        // what the renderers drew lights the wall around it
        let hinted = if matches!(stage.moment, Moment::Slide) {
            hint_light(&grid, stage.hints)
        } else {
            vec![0.0; n]
        };

        // Brightness follows point density relative to the picture's own
        // dense parts, so strokes stay brighter than fills and a picture's
        // structure survives on a coarse grid.
        let mut dense: Vec<f32> = density.iter().copied().filter(|d| *d > 0.05).collect();
        let norm = if dense.is_empty() {
            1.0
        } else {
            let k = ((dense.len() as f32) * 0.9) as usize;
            let k = k.min(dense.len() - 1);
            dense.select_nth_unstable_by(k, f32::total_cmp);
            dense[k].max(0.3)
        };

        let (cx0, cy0) = grid.cell_of(centre);
        let reach = ((grid.cols * grid.cols + grid.rows * grid.rows) as f32).sqrt() * 0.5;
        let span = if brisk { 0.28 } else { 0.62 };
        let seed = (stage.index as f32 + 1.0) * 0.37;
        for i in 0..n {
            let rel = (density[i] / norm).min(1.0).powf(0.8);
            let fig = smoothstep(0.12, 1.0, rel) * weight;
            let t = fig.max(hinted[i]);
            let (c, r) = ((i % grid.cols) as f32, (i / grid.cols) as f32);
            let d = ((c - cx0).powi(2) + (r - cy0).powi(2)).sqrt() / reach;
            let jitter = hash01(i as u32 * 7 + 3);
            self.next[i] = t;
            self.wait[i] = if cx.still {
                0.0
            } else if t > self.goal[i] {
                d * span + jitter * 0.12
            } else {
                // what goes out, goes out quickly and unevenly
                jitter * if look == Look::EndOut { 1.3 } else { 0.18 }
            };
            // a slow diagonal field through the palette, different per picture;
            // light around drawn content takes the accent
            self.next_hue[i] = if hinted[i] > fig {
                0.0
            } else {
                0.5 + 0.5 * (c * 0.045 + r * 0.06 + seed).sin()
            };
        }
        if cx.still {
            self.goal.copy_from_slice(&self.next);
            self.level.copy_from_slice(&self.next);
            self.hue.copy_from_slice(&self.next_hue);
            self.flick.iter_mut().for_each(|f| *f = 0.0);
        }
    }

    fn advance(&mut self, dt: f32) {
        let n = self.level.len();
        let rise = 1.0 - (-dt * RISE).exp();
        let fall = 1.0 - (-dt * FALL).exp();
        let tint = 1.0 - (-dt * 3.0).exp();
        for i in 0..n {
            if self.wait[i] > 0.0 {
                self.wait[i] -= dt;
                if self.wait[i] <= 0.0 {
                    // an LED that strikes from dark flickers first
                    if self.goal[i] < 0.08 && self.next[i] > 0.3 {
                        self.flick[i] = FLICKER;
                        self.level[i] = self.next[i] * 1.25;
                        self.hue[i] = self.next_hue[i];
                    }
                    self.goal[i] = self.next[i];
                }
            }
            let target = self.goal[i];
            let k = if target > self.level[i] { rise } else { fall };
            self.level[i] += (target - self.level[i]) * k;
            self.hue[i] += (self.next_hue[i] - self.hue[i]) * tint;
            if self.flick[i] > 0.0 {
                self.flick[i] -= dt;
            }
        }
    }

    fn sprites(&mut self, ctx: &egui::Context) -> egui::TextureId {
        self.sprites
            .get_or_insert_with(|| {
                ctx.load_texture(
                    "mdeck-led-sprites",
                    sprite_sheet(),
                    egui::TextureOptions::LINEAR,
                )
            })
            .id()
    }
}

impl Default for Led {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Led {
    fn update(&mut self, cx: &FrameCx, stage: &Stage, _lib: &mut Library) {
        if self.grid.is_none() || (self.rect.size() - cx.rect.size()).length() > 1.0 {
            self.resize(Grid::new(cx.rect, cx.scale));
        }
        if self.rect != cx.rect {
            // the wall follows the slide rect (translation only here)
            if let Some(g) = &mut self.grid {
                *g = Grid::new(cx.rect, cx.scale);
            }
            self.rect = cx.rect;
        }
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
            self.build(cx, stage, look);
            self.key = Some(key);
        }

        // a reveal on the same slide sends a sweep of light across the wall
        if self.reveal.0 == stage.index && stage.reveal > self.reveal.1 && !cx.still {
            self.sweep = Some(self.time);
        }
        self.reveal = (stage.index, stage.reveal);
        self.burst = match stage.moment {
            Moment::Burst { progress } => Some(progress),
            _ => None,
        };
        if cx.still {
            self.time = 1.0;
            self.sweep = None;
            return;
        }
        self.time += cx.dt;
        self.advance(cx.dt);
    }

    fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, _stage: &Stage) {
        let Some(grid) = self.grid else {
            return;
        };
        let texture = self.sprites(ui.ctx());
        let theme = cx.theme;
        let palette = Palette::of(theme);
        let lens = lens_color(theme);
        let t = self.time;
        let pitch = grid.pitch;
        let n = grid.len();
        let opacity = cx.opacity;

        let mut mesh = egui::Mesh::with_texture(texture);
        mesh.reserve_triangles(n * 2 * 3);
        mesh.reserve_vertices(n * 4 * 3);

        // unlit lenses: every LED, barely there
        let lens_r = pitch * 0.34;
        for i in 0..n {
            let p = grid.centre(i);
            mesh.add_rect_with_uv(
                Rect::from_center_size(p, egui::vec2(lens_r * 2.0, lens_r * 2.0)),
                SPRITE_LENS,
                fade(lens, opacity),
            );
        }

        // what each LED shows this frame
        let mut lit: Vec<(usize, f32, Color32)> = Vec::with_capacity(n / 3);
        let border_ring = self.border.then(|| border_positions(&grid));
        let sweep_x = self.sweep.map(|s| (t - s) / 0.75);
        if sweep_x.is_some_and(|x| x > 1.3) {
            self.sweep = None;
        }
        let reach = ((grid.cols * grid.cols + grid.rows * grid.rows) as f32).sqrt() * 0.5;
        for i in 0..n {
            let (c, r) = ((i % grid.cols) as f32, (i / grid.cols) as f32);
            let seed = hash01(i as u32);
            let mut level = self.level[i];
            if self.flick[i] > 0.0 {
                let step = ((self.flick[i] * 55.0) as i32 + (seed * 7.0) as i32) % 3;
                level *= if step == 0 { 0.15 } else { 1.0 };
            }
            // lit LEDs scintillate a little, like real ones
            level *= 0.93 + 0.07 * (t * 2.3 + seed * 40.0).sin();
            let mut hue = self.hue[i] + 0.10 * (t * 0.55 + c * 0.05 - r * 0.03).sin();
            let mut hot_ring = 0.0;

            // ambient aurora in the calm zone's complement
            let z = self.zone[i];
            if self.ambient > 0.0 && z > 0.0 {
                let a =
                    0.5 + 0.5 * (c * 0.085 + t * 0.32 + 1.6 * (r * 0.07 - t * 0.19).sin()).sin();
                let b = 0.5 + 0.5 * (r * 0.11 - t * 0.27 + 1.3 * (c * 0.05 + t * 0.13).sin()).sin();
                let amb = self.ambient * z * (a.powi(5) * 0.8 + b.powi(7) * 0.6);
                if amb > level * 0.5 {
                    hue = 0.15 + 0.7 * b;
                }
                level = level.max(amb + level * 0.5);
            }
            // a reveal: one bright band of light passes left to right
            if let Some(x) = sweep_x {
                let pos = x * (grid.cols as f32 + 12.0) - 6.0;
                let band = (-((c - pos) / 2.2).powi(2)).exp() * 0.30 * z.max(0.25);
                level = level.max(band);
            }
            // the countdown's burst: a white-hot ring runs out and takes everything with it
            if let Some(p) = self.burst {
                let (cx0, cy0) = grid.cell_of(cx.rect.center());
                let d = ((c - cx0).powi(2) + (r - cy0).powi(2)).sqrt();
                let radius = p * reach * 1.25;
                let ring = (-((d - radius) / 3.5).powi(2)).exp();
                level = level * (1.0 - p).max(0.0) + ring * (1.0 - p * 0.6);
                if ring > 0.3 {
                    hue = 0.9;
                    hot_ring = ring;
                }
            }
            let mut color = palette.at(hue.clamp(0.0, 1.0));
            if hot_ring > 0.0 {
                color = mix(color, palette.white, hot_ring * 0.6);
            }
            if let Some(ring) = &border_ring
                && let Some(k) = ring.get(&i)
            {
                // a chasing marquee: every third bulb dark, running clockwise
                let phase = ((*k as f32) - t * 9.0).rem_euclid(3.0);
                let on = if phase < 2.0 { 0.85 } else { 0.08 };
                level = level.max(on);
                color = palette.bulb;
            }
            if level > 0.012 {
                // hot LEDs whiten toward their core
                let hot = (level - 0.75).max(0.0) * 1.2;
                lit.push((i, level, mix(color, palette.white, hot.min(0.55))));
            }
        }

        // bloom and glow first (additive), then crisp cores on top: bright
        // areas light the air in front of the wall
        let bloom_r = pitch * 4.2;
        for &(i, level, color) in &lit {
            if level > 0.5 && (i % 2 == (i / grid.cols) % 2) {
                let p = grid.centre(i);
                mesh.add_rect_with_uv(
                    Rect::from_center_size(p, egui::vec2(bloom_r * 2.0, bloom_r * 2.0)),
                    SPRITE_GLOW,
                    additive(color, (level.min(1.2) - 0.4) * 0.07 * opacity),
                );
            }
        }
        let glow_r = pitch * 1.8;
        for &(i, level, color) in &lit {
            let p = grid.centre(i);
            let g = (level.min(1.3) * 0.34 * opacity).min(1.0);
            mesh.add_rect_with_uv(
                Rect::from_center_size(p, egui::vec2(glow_r * 2.0, glow_r * 2.0)),
                SPRITE_GLOW,
                additive(color, g),
            );
        }
        let core_r = pitch * 0.40;
        for &(i, level, color) in &lit {
            let p = grid.centre(i);
            mesh.add_rect_with_uv(
                Rect::from_center_size(p, egui::vec2(core_r * 2.0, core_r * 2.0)),
                SPRITE_CORE,
                premul(color, (level.min(1.0) * opacity).min(1.0)),
            );
        }
        ui.painter().add(egui::Shape::mesh(mesh));
        if !cx.still {
            ui.ctx().request_repaint();
        }
    }
}

/// Add each point's light to the LEDs around it (a small gaussian), so a
/// stroke of points lights a clean line of LEDs.
fn splat(grid: &Grid, rect: Rect, points: &[[f32; 2]], place: Place, density: &mut [f32]) {
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
fn hint_light(grid: &Grid, hints: &[Hint]) -> Vec<f32> {
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
                let (c0, r0) = grid.cell_of(b.left_top());
                let (c1, _) = grid.cell_of(b.right_top());
                let row = (r0 - 0.5 - 38.0 / PITCH).floor() as i64;
                for c in (c0.ceil() as i64)..=(c1.floor() as i64) {
                    set(c, row, 0.95);
                    set(c, row - 1, 0.28);
                }
            }
            Hint::Bar(b) => {
                // horizontal: a marker past the bar's end and its value label
                let (c1, r0) = grid.cell_of(b.right_top());
                let (_, r1) = grid.cell_of(b.right_bottom());
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
fn border_positions(grid: &Grid) -> std::collections::HashMap<usize, usize> {
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

/// The wall's colours, from the theme: a gradient through the accents, a
/// warm bulb colour for the marquee, and white for the hottest cores.
struct Palette {
    stops: [Color32; 3],
    bulb: Color32,
    white: Color32,
}

impl Palette {
    fn of(theme: &Theme) -> Self {
        Palette {
            stops: [theme.accent, theme.accent_soft, theme.particle_cool],
            bulb: theme.secondary,
            white: theme.particle_light,
        }
    }

    fn at(&self, h: f32) -> Color32 {
        let h = h.clamp(0.0, 1.0) * 2.0;
        if h < 1.0 {
            mix(self.stops[0], self.stops[1], h)
        } else {
            mix(self.stops[1], self.stops[2], h - 1.0)
        }
    }
}

/// An unlit lens: the background, lifted a little (pressed in a little on a
/// light theme).
fn lens_color(theme: &Theme) -> Color32 {
    let bg = theme.background;
    let luma = (0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32) / 255.0;
    if luma > 0.5 {
        mix(bg, Color32::BLACK, 0.06)
    } else {
        mix(bg, Color32::WHITE, 0.075)
    }
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

fn fade(c: Color32, a: f32) -> Color32 {
    premul(c, a)
}

/// `c` at opacity `a`, premultiplied (normal blending).
fn premul(c: Color32, a: f32) -> Color32 {
    let a = a.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        (c.r() as f32 * a) as u8,
        (c.g() as f32 * a) as u8,
        (c.b() as f32 * a) as u8,
        (a * 255.0) as u8,
    )
}

/// `c` scaled by `k` with zero alpha: added onto what is below (glow).
fn additive(c: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        (c.r() as f32 * k) as u8,
        (c.g() as f32 * k) as u8,
        (c.b() as f32 * k) as u8,
        0,
    )
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A stable pseudo-random number in 0..1 for an index.
fn hash01(i: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B1) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0x00FF_FFFF) as f32 / 16_777_215.0
}

/// Sprite sheet: lens, core and glow side by side, white, premultiplied.
const SPRITE: usize = 64;
const SPRITE_LENS: Rect = Rect {
    min: Pos2 { x: 0.0, y: 0.0 },
    max: Pos2 {
        x: 1.0 / 3.0,
        y: 1.0,
    },
};
const SPRITE_CORE: Rect = Rect {
    min: Pos2 {
        x: 1.0 / 3.0,
        y: 0.0,
    },
    max: Pos2 {
        x: 2.0 / 3.0,
        y: 1.0,
    },
};
const SPRITE_GLOW: Rect = Rect {
    min: Pos2 {
        x: 2.0 / 3.0,
        y: 0.0,
    },
    max: Pos2 { x: 1.0, y: 1.0 },
};

fn sprite_sheet() -> egui::ColorImage {
    let w = SPRITE * 3;
    let mut pixels = vec![Color32::TRANSPARENT; w * SPRITE];
    for y in 0..SPRITE {
        for x in 0..SPRITE {
            let dx = (x as f32 + 0.5) / SPRITE as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / SPRITE as f32 * 2.0 - 1.0;
            let r = (dx * dx + dy * dy).sqrt();
            // lens: a flat disc with a faint rim, lit from the top left
            let edge = 1.0 - smoothstep(0.82, 1.0, r);
            let rim = smoothstep(0.55, 0.9, r) * 0.5 + 0.5;
            let light = 1.0 - 0.18 * (dx + dy).max(-1.0);
            let lens = (edge * rim * light).clamp(0.0, 1.0);
            // core: a bright, slightly soft dome
            let core = (1.0 - smoothstep(0.55, 1.0, r)) * (1.0 - 0.25 * r * r);
            // glow: a wide soft falloff
            let glow = (-(r * r) * 5.5).exp() * (1.0 - smoothstep(0.7, 1.0, r));
            for (k, v) in [lens, core, glow].into_iter().enumerate() {
                let v8 = (v.clamp(0.0, 1.0) * 255.0) as u8;
                pixels[y * w + k * SPRITE + x] = Color32::from_rgba_premultiplied(v8, v8, v8, v8);
            }
        }
    }
    egui::ColorImage::new([w, SPRITE], pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_covers_the_slide_with_even_margins() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        let g = Grid::new(rect, 1.0);
        assert_eq!((g.cols, g.rows), (142, 80));
        let first = g.centre(0);
        let last = g.centre(g.len() - 1);
        assert!((first.x - (1920.0 - last.x)).abs() < 0.01);
        assert!((first.y - (1080.0 - last.y)).abs() < 0.01);
    }

    #[test]
    fn a_stroke_of_points_lights_a_line_of_leds() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
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
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(200.0, 120.0));
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
