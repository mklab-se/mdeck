//! The LED engine: a fixed wall of RGB LEDs behind every slide. Unlit, the
//! lenses are barely visible; illustrations, countdown digits and the end
//! words appear by lighting LEDs, never by moving anything. A new picture
//! powers on in a sweep from its centre, each LED flickering as it strikes;
//! a lit picture shimmers slowly between the theme's colours. Title slides
//! get a chasing marquee border, other slides a faint aurora drifting across
//! the wall, away from the copy and out of the content's way.

use std::sync::Arc;

use mdeck_sdk::cloud::Cloud;
use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Needs};
use mdeck_sdk::geometry::Hint;
use mdeck_sdk::paint::{Painter, Pos2, Rect, Texture, TextureFilter, smoothstep, sprite_sheet};
use mdeck_sdk::stage::{Frame, Look, Moment, Picture, PictureSource, Place, Stage};

use crate::engines::hash01;
use draw::{Palette, lens_color, wall};
use light::{hint_light, splat};

mod draw;
mod light;

/// Seconds into the end slide when the caption fades in: the words have lit
/// and every LED has died out.
pub const END_CAPTION_DELAY: f32 = 5.4;

pub static DEF: EngineDef = EngineDef {
    name: "led",
    summary: "A wall of RGB LEDs behind every slide; pictures light up, nothing moves.",
    capabilities: Capabilities {
        picture: true,
        ..Capabilities::NONE
    },
    settings: &[],
    needs: Needs { page: false },
    ending_caption_delay: END_CAPTION_DELAY,
    create: |_| Box::new(Led::new()),
    board: None,
};

/// LED pitch in px on a 1920x1080 slide.
const PITCH: f32 = 13.5;
/// The end words stay lit this long, then the wall dies out.
const END_WORDS: f32 = 3.4;
/// Rise and fall rates of an LED toward its goal (per second).
const RISE: f32 = 16.0;
const FALL: f32 = 7.0;
/// How long a striking LED flickers.
const FLICKER: f32 = 0.16;

type Key = (usize, Look, usize, u64, bool);

/// A picture on the wall, before it is lit.
struct Lighting {
    density: Vec<f32>,
    weight: f32,
    centre: Pos2,
    brisk: bool,
}

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
    /// The sprite sheet, uploaded on the first paint.
    sprites: Option<Texture>,
}

impl Led {
    pub fn new() -> Self {
        Self {
            grid: None,
            rect: Rect::ZERO,
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

    /// What the stage shows as point density per LED, how strongly it
    /// lights (`weight`), where its power-on sweep starts and whether it
    /// comes up briskly. Sets the slide's ambient light and marquee.
    fn picture(&mut self, grid: &Grid, cx: &Frame, stage: &Stage, look: Look) -> Lighting {
        let rect = cx.rect;
        let rect_aspect = rect.width() / rect.height();
        let mut density = vec![0.0f32; grid.len()];
        let mut weight = 1.0;
        let mut centre = rect.center();
        let mut brisk = false;
        self.ambient = 0.0;
        self.border = false;

        match (&stage.moment, look) {
            (Moment::Countdown { mask, .. }, _) => {
                let h = 0.60;
                let w = h * mask.aspect / rect_aspect;
                let place = Place {
                    u: 0.5 - w / 2.0,
                    v: 0.47 - h / 2.0,
                    w,
                    h,
                };
                splat(grid, rect, &mask.points, place, &mut density);
                centre = rect.center();
                brisk = true;
                self.ambient = 0.025;
            }
            (Moment::End { words, .. }, Look::EndWords) => {
                let w = 0.64;
                let h = w / words.aspect * rect_aspect;
                let place = Place {
                    u: 0.5 - w / 2.0,
                    v: 0.47 - h / 2.0,
                    w,
                    h,
                };
                splat(grid, rect, &words.points, place, &mut density);
                brisk = true;
            }
            (Moment::Slide, _) => {
                let figure = cloud(stage);
                if let Some((fig, cloud)) = figure {
                    splat(grid, rect, &cloud.points, fig.place, &mut density);
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
                self.ambient = if figure.is_some() { 0.05 } else { 0.11 };
            }
            _ => {}
        }
        Lighting {
            density,
            weight,
            centre,
            brisk,
        }
    }

    /// The ambient zone: calm over the copy column and inside content frames.
    fn calm(&mut self, grid: &Grid, rect: Rect, stage: &Stage) {
        for i in 0..grid.len() {
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
            for h in stage.geometry {
                if let Hint::Frame(r) = h
                    && r.expand(grid.pitch).contains(p)
                {
                    z = 0.0;
                }
            }
            self.zone[i] = z;
        }
    }

    /// Build the picture for the stage: per-LED target brightness and hue,
    /// the ambient zone, and power-on delays.
    fn build(&mut self, cx: &Frame, stage: &Stage, look: Look) {
        let grid = self.grid.expect("grid built before the picture");
        let n = grid.len();
        let Lighting {
            density,
            weight,
            centre,
            brisk,
        } = self.picture(&grid, cx, stage, look);
        self.calm(&grid, cx.rect, stage);

        // what the renderers drew lights the wall around it
        let hinted = if matches!(stage.moment, Moment::Slide) {
            hint_light(&grid, stage.geometry)
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
}

impl Default for Led {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for Led {
    fn update(&mut self, cx: &Frame, stage: &Stage) {
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
        let look = stage.moment.look(END_WORDS);
        let figure = cloud(stage)
            .map(|(_, c)| Arc::as_ptr(c) as usize)
            .unwrap_or(0);
        let key = (stage.index, look, figure, stage.geometry_key, stage.title);
        if self.key != Some(key) {
            self.build(cx, stage, look);
            self.key = Some(key);
        }

        // a reveal on the same slide sends a sweep of light across the wall
        if self.reveal.0 == stage.index && stage.step > self.reveal.1 && !cx.still {
            self.sweep = Some(self.time);
        }
        self.reveal = (stage.index, stage.step);
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

    fn paint(&mut self, painter: &mut Painter, cx: &Frame, _stage: &Stage) {
        let Some(grid) = self.grid else {
            return;
        };
        let texture = self
            .sprites
            .get_or_insert_with(|| {
                painter.load_texture("mdeck-led-sprites", &sprite_sheet(), TextureFilter::Linear)
            })
            .clone();
        let palette = Palette::of(cx.tokens);
        let sweep_x = self.sweep.map(|s| (self.time - s) / 0.75);
        if sweep_x.is_some_and(|x| x > 1.3) {
            self.sweep = None;
        }
        let lit = self.shade(&grid, &palette, sweep_x, cx.rect.center());
        let mesh = wall(texture, &grid, &lit, lens_color(cx.tokens), cx.opacity);
        painter.mesh(mesh);
    }

    /// The wall always shimmers.
    fn animating(&self) -> bool {
        true
    }
}

/// The slide's picture when it is a point cloud, with the cloud.
fn cloud<'s>(stage: &'s Stage) -> Option<(&'s Picture, &'s Arc<Cloud>)> {
    let pic = stage.picture.as_ref()?;
    match &pic.source {
        PictureSource::Cloud(c) => Some((pic, c)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_covers_the_slide_with_even_margins() {
        let rect = Rect::from_min_size(Pos2::ZERO, mdeck_sdk::paint::Vec2::new(1920.0, 1080.0));
        let g = Grid::new(rect, 1.0);
        assert_eq!((g.cols, g.rows), (142, 80));
        let first = g.centre(0);
        let last = g.centre(g.len() - 1);
        assert!((first.x - (1920.0 - last.x)).abs() < 0.01);
        assert!((first.y - (1080.0 - last.y)).abs() < 0.01);
    }
}
