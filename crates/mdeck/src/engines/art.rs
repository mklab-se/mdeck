//! What the art engines share: a generated picture being drawn in (the
//! reveal runs on the CPU into a texture, a frame at a time, and stops
//! uploading once the picture is finished), and the pen strokes a medium
//! draws when a slide has no picture (its `@illustration`, the countdown,
//! the end words).

use std::sync::Arc;

use eframe::egui::{self, Color32, Pos2, Rect};

use super::Capabilities;
use super::stage::{FrameCx, Mask, Moment, Place, Stage};
use crate::render::art::prepare::Prepared;
use crate::render::illustration::Library;
use crate::render::strokes::{Picture, plan, to_screen, toured};

/// What an art engine shows: the pictures most engines show, drawn from
/// generated art when a slide has it.
pub const CAPABILITIES: Capabilities = Capabilities {
    art: true,
    ..Capabilities::PICTURES
};

/// A generated picture on the slide, drawn in over `duration` seconds.
pub struct Drawing {
    pub picture: Arc<Prepared>,
    pub place: Place,
    pub backdrop: bool,
    pub born: f32,
    pub duration: f32,
    texture: Option<egui::TextureHandle>,
    /// The moment the texture shows (negative: nothing uploaded yet).
    shown: f32,
}

pub use crate::render::art::prepare::Reveal;

impl Drawing {
    pub fn new(
        picture: Arc<Prepared>,
        place: Place,
        backdrop: bool,
        born: f32,
        duration: f32,
    ) -> Self {
        Self {
            picture,
            place,
            backdrop,
            born,
            duration,
            texture: None,
            shown: -1.0,
        }
    }

    /// 0..1 through the reveal (1 and beyond: finished).
    pub fn progress(&self, now: f32) -> f32 {
        ((now - self.born) / self.duration.max(1e-3)).max(0.0)
    }

    /// Where the picture goes on screen.
    pub fn screen(&self, rect: Rect) -> Rect {
        to_rect(self.place, rect)
    }

    /// Where the drawing hand is at `now`, on screen.
    pub fn tip(&self, now: f32, rect: Rect) -> Option<Pos2> {
        let (u, v) = self.picture.tip(self.progress(now))?;
        let r = self.screen(rect);
        Some(Pos2::new(
            r.left() + u * r.width(),
            r.top() + v * r.height(),
        ))
    }

    /// Paint the picture as far as it has come, tinted by `tint` (line art
    /// is white, so the tint is its ink colour).
    pub fn paint(&mut self, ui: &egui::Ui, rect: Rect, now: f32, tint: Color32, reveal: Reveal) {
        let t = self.progress(now).min(1.0 + reveal.soft);
        let stale = self.texture.is_none() || (t - self.shown).abs() > 0.002;
        if stale {
            let pixels = self.picture.reveal(t, &reveal);
            let image = egui::ColorImage::new([self.picture.width, self.picture.height], pixels);
            match &mut self.texture {
                Some(tex) => tex.set(image, egui::TextureOptions::LINEAR),
                None => {
                    let name = format!("mdeck-art-{:p}", Arc::as_ptr(&self.picture));
                    self.texture = Some(ui.ctx().load_texture(
                        name,
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
            self.shown = t;
        }
        if let Some(tex) = &self.texture {
            let mut mesh = egui::Mesh::with_texture(tex.id());
            mesh.add_rect_with_uv(
                self.screen(rect),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                tint,
            );
            ui.painter().add(egui::Shape::mesh(mesh));
        }
    }
}

/// What a slide, the countdown or the end is showing.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Look {
    Slide,
    Digit(u8),
    Burst,
    EndWords,
    EndOut,
}

type Key = (usize, Look, usize, usize, bool);

/// The state every art engine keeps: the picture being drawn in (or the
/// pen strokes standing in for it), the one fading out, and the clock. An
/// engine calls [`Canvas::update`] from its own `update` and paints what
/// the canvas holds in its medium.
pub struct Canvas {
    pub now: f32,
    pub drawing: Option<Drawing>,
    /// The previous slide's picture, fading out since the given time.
    pub fading: Option<(Drawing, f32)>,
    pub strokes: Option<Picture>,
    pub fading_strokes: Option<(Picture, f32)>,
    /// The countdown's burst, 0..1, while it runs.
    pub burst: Option<f32>,
    /// Where the drawing hand is this frame (`None` in stills).
    pub tip: Option<Tip>,
    key: Option<Key>,
    /// Seconds to draw a picture in, and to finish around it after.
    draw: f32,
    after: f32,
    /// How long the end words hold.
    end_words: f32,
    /// How long the old picture takes to fade.
    fade: f32,
}

impl Canvas {
    pub fn new(draw: f32, after: f32, end_words: f32, fade: f32) -> Self {
        Self {
            now: 0.0,
            drawing: None,
            fading: None,
            strokes: None,
            fading_strokes: None,
            burst: None,
            tip: None,
            key: None,
            draw,
            after,
            end_words,
            fade,
        }
    }

    fn retire(&mut self) {
        if let Some(d) = self.drawing.take() {
            self.fading = Some((d, self.now));
        }
        if let Some(p) = self.strokes.take() {
            self.fading_strokes = Some((p, self.now));
        }
    }

    /// Follow the stage: a new slide, digit or end act starts a new picture
    /// (its generated art on a slide, else pen strokes), and the old one
    /// fades. `still` settles everything at once.
    pub fn update(&mut self, cx: &FrameCx, stage: &Stage, _lib: &mut Library) {
        let look = match &stage.moment {
            Moment::Slide => Look::Slide,
            Moment::Countdown { digit, .. } => Look::Digit(*digit),
            Moment::Burst { .. } => Look::Burst,
            Moment::End { elapsed, .. } if *elapsed < self.end_words => Look::EndWords,
            Moment::End { .. } => Look::EndOut,
        };
        let art = stage
            .art
            .as_ref()
            .map(|a| Arc::as_ptr(&a.picture) as usize)
            .unwrap_or(0);
        let figure = stage
            .figure
            .as_ref()
            .map(|f| Arc::as_ptr(&f.cloud) as usize)
            .unwrap_or(0);
        let key = (stage.index, look, art, figure, stage.title);
        if self.key != Some(key) {
            match look {
                Look::Burst => self.burst = Some(0.0),
                Look::EndOut => self.retire(),
                _ => {
                    self.burst = None;
                    self.retire();
                    if let (Look::Slide, Some(a)) = (look, &stage.art) {
                        self.drawing = Some(Drawing::new(
                            a.picture.clone(),
                            a.place,
                            a.backdrop,
                            self.now,
                            self.draw,
                        ));
                    } else {
                        self.strokes = fallback_strokes(cx, stage, self.now);
                    }
                }
            }
            self.key = Some(key);
        }
        if let Moment::Burst { progress } = stage.moment {
            self.burst = Some(progress);
        }
        if cx.still {
            self.fading = None;
            self.fading_strokes = None;
            if let Some(d) = &mut self.drawing {
                d.born = self.now - self.draw - self.after - 60.0;
            }
            if let Some(p) = &mut self.strokes {
                p.born = self.now - p.duration - 60.0;
            }
            self.tip = None;
            return;
        }
        self.now += cx.dt;
        let fade = self.fade;
        let now = self.now;
        if self
            .fading
            .as_ref()
            .is_some_and(|(_, since)| now - since > fade)
        {
            self.fading = None;
        }
        if self
            .fading_strokes
            .as_ref()
            .is_some_and(|(_, since)| now - since > fade)
        {
            self.fading_strokes = None;
        }
        self.tip = self.find_tip(cx.rect);
    }

    /// Where the hand is: on the picture being drawn in, or at the end of
    /// the pen strokes while the pen is down.
    fn find_tip(&self, rect: Rect) -> Option<Tip> {
        let now = self.now;
        if let Some(d) = &self.drawing
            && let Some(at) = d.tip(now, rect)
        {
            return Some(Tip::Picture {
                at,
                progress: d.progress(now),
                frame: d.screen(rect),
                backdrop: d.backdrop,
            });
        }
        let p = self.strokes.as_ref()?;
        let (tip, on) = p.tip(now - p.born)?;
        on.then(|| Tip::Pen {
            at: to_screen(tip, rect),
            progress: (now - p.born) / p.duration.max(0.1),
        })
    }

    /// Paint what the canvas holds in `hand`'s medium: the old picture and
    /// strokes fading, the current picture (dimmed behind a title) and
    /// strokes, then the hand's tool at the tip. Asks for another frame
    /// while anything moves.
    pub fn paint(&mut self, ui: &egui::Ui, cx: &FrameCx, hand: &impl Hand) {
        let now = self.now;
        let painter = ui.painter();
        let left = self
            .fading
            .as_ref()
            .map(|(_, since)| self.fade_left(*since));
        if let (Some((old, _)), Some(left)) = (&mut self.fading, left) {
            hand.picture(ui, cx, old, now, left * cx.opacity, false);
        }
        if let Some((old, since)) = &self.fading_strokes {
            let k = self.fade_left(*since) * cx.opacity;
            hand.strokes(painter, cx, old, now, k, false);
        }
        let burst = self.burst_left();
        if let Some(d) = &mut self.drawing {
            let k = if d.backdrop { hand.backdrop() } else { 1.0 } * cx.opacity;
            hand.picture(ui, cx, d, now, k, true);
        }
        if let Some(p) = &self.strokes {
            hand.strokes(painter, cx, p, now, burst * cx.opacity, true);
        }
        hand.finish(painter, cx, if cx.still { None } else { self.tip });
        if (self.busy() || hand.busy()) && !cx.still {
            ui.ctx().request_repaint();
        }
    }

    /// How far the old picture has faded (1: fully there, 0: gone).
    pub fn fade_left(&self, since: f32) -> f32 {
        (1.0 - (self.now - since) / (self.fade * 0.9)).clamp(0.0, 1.0)
    }

    /// The countdown's last digit fading as the first slide comes in.
    pub fn burst_left(&self) -> f32 {
        self.burst
            .map(|b| (1.0 - b * 1.5).clamp(0.0, 1.0))
            .unwrap_or(1.0)
    }

    /// Anything still moving: ask for another frame.
    pub fn busy(&self) -> bool {
        self.fading.is_some()
            || self.fading_strokes.is_some()
            || self
                .drawing
                .as_ref()
                .is_some_and(|d| self.now - d.born < self.draw + self.after + 0.2)
            || self
                .strokes
                .as_ref()
                .is_some_and(|p| self.now - p.born < p.duration + 0.2)
    }
}

/// Where the drawing hand is.
#[derive(Clone, Copy, Debug)]
pub enum Tip {
    /// On a generated picture being drawn in: `progress` through it (1 and
    /// beyond: finished) and the picture's `frame` on screen.
    Picture {
        at: Pos2,
        progress: f32,
        frame: Rect,
        backdrop: bool,
    },
    /// At the end of pen strokes: `progress` through them.
    Pen { at: Pos2, progress: f32 },
}

impl Tip {
    /// Where the tool goes, unless it is drawing a picture behind a title.
    pub fn in_front(self) -> Option<Pos2> {
        match self {
            Tip::Picture { backdrop: true, .. } => None,
            Tip::Picture { at, .. } | Tip::Pen { at, .. } => Some(at),
        }
    }
}

/// An art engine's medium, as [`Canvas::paint`] draws with it.
pub trait Hand {
    /// How strongly a picture behind a title shows (0..1).
    fn backdrop(&self) -> f32;

    /// Draw picture `d` as far as it has come at opacity `k`. `current`:
    /// the slide's own picture, not the old one fading out.
    fn picture(
        &self,
        ui: &egui::Ui,
        cx: &FrameCx,
        d: &mut Drawing,
        now: f32,
        k: f32,
        current: bool,
    );

    /// Draw pen strokes as far as they have come at opacity `k`.
    fn strokes(
        &self,
        painter: &egui::Painter,
        cx: &FrameCx,
        p: &Picture,
        now: f32,
        k: f32,
        current: bool,
    );

    /// Last, over everything: the tool at `tip` (`None` when nothing is
    /// being drawn, and in stills) and whatever else rides on top.
    fn finish(&self, _painter: &egui::Painter, _cx: &FrameCx, _tip: Option<Tip>) {}

    /// Something of the engine's own is still moving.
    fn busy(&self) -> bool {
        false
    }
}

/// A place in slide fractions, on screen.
pub fn to_rect(place: Place, rect: Rect) -> Rect {
    Rect::from_min_size(
        Pos2::new(
            rect.left() + place.u * rect.width(),
            rect.top() + place.v * rect.height(),
        ),
        egui::vec2(place.w * rect.width(), place.h * rect.height()),
    )
}

/// The pen strokes for a moment without a picture: the slide's
/// `@illustration`, a countdown digit or the end words. `None` when there is
/// nothing to draw.
pub fn fallback_strokes(cx: &FrameCx, stage: &Stage, now: f32) -> Option<Picture> {
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
    let (strokes, duration, weight) = match &stage.moment {
        Moment::Countdown { mask, .. } => (
            vec![toured(&mask.0, place_mask(mask, 0.56), aspect)],
            0.85,
            1.0,
        ),
        Moment::End { words, .. } => {
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
        Moment::Slide => {
            let fig = stage.figure.as_ref()?;
            let weight = if fig.backdrop { 0.4 } else { 1.0 };
            (
                vec![toured(&fig.cloud.points, fig.place, aspect)],
                2.4,
                weight,
            )
        }
        Moment::Burst { .. } => return None,
    };
    Some(plan(strokes, duration, weight, aspect, now))
}
