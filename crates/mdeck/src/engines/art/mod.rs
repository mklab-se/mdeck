//! What the art engines share: a generated picture being drawn in (the
//! reveal runs on the CPU into a texture, a frame at a time, and stops
//! uploading once the picture is finished), and the pen strokes a medium
//! draws when a slide has no artwork (its point cloud picture, the
//! countdown, the end words).

#![cfg_attr(
    not(all_engines),
    allow(dead_code, reason = "shared by the art engines, each using some of it")
)]

pub mod reveal;
pub mod strokes;
pub mod trace;

use std::sync::Arc;

use mdeck_sdk::cloud::Mask;
use mdeck_sdk::engine::Capabilities;
use mdeck_sdk::paint::{
    Color, ImageData, Mesh, Painter, Pos2, Rect, Texture, TextureFilter, sprite_sheet,
};
use mdeck_sdk::stage::{Artwork, Frame, Look, Moment, PictureSource, Place, Stage};

pub use reveal::{Reveal, Reveals};
use strokes::{Strokes, plan, plan_layers, to_screen, toured};
pub use trace::Fill;

/// What an art engine can do: show the slide's picture (its generated
/// artwork, else its point cloud as pen strokes), and draw the countdown
/// digits and the end words in its medium.
pub const CAPABILITIES: Capabilities = Capabilities::NONE
    .with_picture()
    .with_countdown()
    .with_ending();

/// The shared sprite sheet ([`sprite_sheet`]) as a texture, uploaded on
/// first use.
pub struct Sprites {
    name: &'static str,
    texture: Option<Texture>,
}

impl Sprites {
    /// `name`: the texture's name in debug tools.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            texture: None,
        }
    }

    /// The texture, uploading it the first time.
    pub fn get(&mut self, painter: &Painter) -> Texture {
        self.texture
            .get_or_insert_with(|| {
                painter.load_texture(self.name, &sprite_sheet(), TextureFilter::Linear)
            })
            .clone()
    }
}

/// A generated picture on the slide, drawn in over `duration` seconds.
pub struct Drawing {
    pub picture: Arc<Artwork>,
    pub place: Place,
    pub backdrop: bool,
    pub born: f32,
    pub duration: f32,
    texture: Option<Texture>,
    /// The moment the texture shows (negative: nothing uploaded yet).
    shown: f32,
}

impl Drawing {
    pub fn new(
        picture: Arc<Artwork>,
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
        self.place.in_rect(rect)
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
    pub fn paint(&mut self, painter: &Painter, rect: Rect, now: f32, tint: Color, reveal: Reveal) {
        let t = self.progress(now).min(1.0 + reveal.soft);
        let stale = self.texture.is_none() || (t - self.shown).abs() > 0.002;
        if stale {
            let image = ImageData {
                size: [self.picture.width, self.picture.height],
                pixels: self.picture.reveal(t, &reveal),
            };
            match &mut self.texture {
                Some(tex) => painter.update_texture(tex, &image, TextureFilter::Linear),
                None => {
                    let name = format!("mdeck-art-{:p}", Arc::as_ptr(&self.picture));
                    self.texture = Some(painter.load_texture(&name, &image, TextureFilter::Linear));
                }
            }
            self.shown = t;
        }
        if let Some(tex) = &self.texture {
            let mut mesh = Mesh::with_texture(tex.clone());
            mesh.add_rect_uv(
                self.screen(rect),
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                tint,
            );
            painter.mesh(mesh);
        }
    }
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
    pub strokes: Option<Strokes>,
    pub fading_strokes: Option<(Strokes, f32)>,
    /// The countdown's burst, 0..1, while it runs.
    pub burst: Option<f32>,
    /// Where the drawing hand is this frame (`None` in stills).
    pub tip: Option<Tip>,
    /// Something moved in the last paint: the engine is animating.
    pub moving: bool,
    key: Option<Key>,
    /// Seconds to draw a picture in, and to finish around it after.
    draw: f32,
    after: f32,
    /// How long the end words hold.
    end_words: f32,
    /// How long the old picture takes to fade.
    fade: f32,
    /// How a point cloud picture is drawn without an artwork.
    fill: Fill,
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
            moving: true,
            key: None,
            draw,
            after,
            end_words,
            fade,
            fill: Fill::Tour,
        }
    }

    /// Draw point cloud pictures with `fill` instead of one pen tour.
    pub fn with_fill(mut self, fill: Fill) -> Self {
        self.fill = fill;
        self
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
    /// fades. A settled frame (a still, reduced motion) finishes everything
    /// at once.
    pub fn update(&mut self, frame: &Frame, stage: &Stage) {
        let look = stage.moment.look(self.end_words);
        let (art, cloud) = match stage.picture.as_ref().map(|p| &p.source) {
            Some(PictureSource::Artwork(a)) => (Arc::as_ptr(a) as usize, 0),
            Some(PictureSource::Cloud(c)) => (0, Arc::as_ptr(c) as usize),
            _ => (0, 0),
        };
        let key = (stage.index, look, art, cloud, stage.title);
        if self.key != Some(key) {
            match look {
                Look::Burst => self.burst = Some(0.0),
                Look::EndOut => self.retire(),
                _ => {
                    self.burst = None;
                    self.retire();
                    let artwork = stage.picture.as_ref().and_then(|p| match &p.source {
                        PictureSource::Artwork(a) => Some((a, p)),
                        _ => None,
                    });
                    if let (Look::Slide, Some((a, p))) = (look, artwork) {
                        self.drawing = Some(Drawing::new(
                            a.clone(),
                            p.place,
                            p.backdrop,
                            self.now,
                            self.draw,
                        ));
                    } else {
                        self.strokes = fallback_strokes(frame, stage, self.now, self.fill);
                    }
                }
            }
            self.key = Some(key);
        }
        if let Moment::Burst { progress, .. } = stage.moment {
            self.burst = Some(progress);
        }
        if frame.settled() {
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
        self.now += frame.dt;
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
        self.tip = self.find_tip(frame.rect);
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
    /// strokes, then the hand's tool at the tip. Notes in
    /// [`Canvas::moving`] whether anything still moves.
    pub fn paint(&mut self, painter: &Painter, frame: &Frame, hand: &impl Hand) {
        let now = self.now;
        let left = self
            .fading
            .as_ref()
            .map(|(_, since)| self.fade_left(*since));
        if let (Some((old, _)), Some(left)) = (&mut self.fading, left) {
            hand.picture(painter, frame, old, now, left * frame.opacity, false);
        }
        if let Some((old, since)) = &self.fading_strokes {
            let k = self.fade_left(*since) * frame.opacity;
            hand.strokes(painter, frame, old, now, k, false);
        }
        let burst = self.burst_left();
        if let Some(d) = &mut self.drawing {
            let k = if d.backdrop { hand.backdrop() } else { 1.0 } * frame.opacity;
            hand.picture(painter, frame, d, now, k, true);
        }
        if let Some(p) = &self.strokes {
            hand.strokes(painter, frame, p, now, burst * frame.opacity, true);
        }
        let settled = frame.settled();
        hand.finish(painter, frame, if settled { None } else { self.tip });
        self.moving = (self.busy() || hand.busy()) && !settled;
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
        painter: &Painter,
        frame: &Frame,
        d: &mut Drawing,
        now: f32,
        k: f32,
        current: bool,
    );

    /// Draw pen strokes as far as they have come at opacity `k`.
    fn strokes(
        &self,
        painter: &Painter,
        frame: &Frame,
        p: &Strokes,
        now: f32,
        k: f32,
        current: bool,
    );

    /// Last, over everything: the tool at `tip` (`None` when nothing is
    /// being drawn, and in stills) and whatever else rides on top.
    fn finish(&self, _painter: &Painter, _frame: &Frame, _tip: Option<Tip>) {}

    /// Something of the engine's own is still moving.
    fn busy(&self) -> bool {
        false
    }
}

/// The pen strokes for a moment without an artwork: the slide's point
/// cloud picture (drawn as `fill` says), a countdown digit or the end
/// words. `None` when there is nothing to draw.
pub fn fallback_strokes(frame: &Frame, stage: &Stage, now: f32, fill: Fill) -> Option<Strokes> {
    let rect = frame.rect;
    let aspect = rect.width() / rect.height();
    let place_mask = |mask: &Mask, h: f32| -> Place {
        let w = h * mask.aspect / aspect;
        Place {
            u: 0.5 - w / 2.0,
            v: 0.47 - h / 2.0,
            w,
            h,
        }
    };
    let (strokes, duration, weight) = match &stage.moment {
        Moment::Countdown { mask, .. } => (
            vec![toured(&mask.points, place_mask(mask, 0.56), aspect)],
            0.85,
            1.0,
        ),
        Moment::End { words, .. } => {
            let w = 0.60;
            let h = w / words.aspect * aspect;
            let place = Place {
                u: 0.5 - w / 2.0,
                v: 0.47 - h / 2.0,
                w,
                h,
            };
            (vec![toured(&words.points, place, aspect)], 1.9, 1.0)
        }
        Moment::Slide => {
            let pic = stage.picture.as_ref()?;
            let PictureSource::Cloud(cloud) = &pic.source else {
                return None;
            };
            let weight = if pic.backdrop { 0.4 } else { 1.0 };
            if let Fill::Lines(shading) = fill {
                let traced = trace::trace(&cloud.points, cloud.aspect, shading);
                let place = |l: Vec<[f32; 2]>| -> Vec<Pos2> {
                    l.into_iter()
                        .map(|p| {
                            let pl = pic.place;
                            Pos2::new(pl.u + p[0] * pl.w, pl.v + p[1] * pl.h)
                        })
                        .collect()
                };
                let lines = traced.lines.into_iter().map(place).collect();
                let hatch = traced.hatch.into_iter().map(place).collect();
                let duration = if shading.is_some() { 3.2 } else { 2.4 };
                return Some(plan_layers(lines, hatch, duration, weight, aspect, now));
            }
            (vec![toured(&cloud.points, pic.place, aspect)], 2.4, weight)
        }
        _ => return None,
    };
    Some(plan(strokes, duration, weight, aspect, now))
}

/// Segment `i` of `pic` (from point `i - 1` to point `i`) on screen, as far
/// as the hand has come `t` seconds into the drawing: `None` while the pen
/// is lifted or has not reached it, cut short while it is being drawn.
pub fn drawn_segment(pic: &Strokes, i: usize, t: f32, rect: Rect) -> Option<(Pos2, Pos2)> {
    if !pic.pen[i] || pic.at[i - 1] > t {
        return None;
    }
    let a = to_screen(pic.points[i - 1], rect);
    let mut b = to_screen(pic.points[i], rect);
    if pic.at[i] > t {
        let f = (t - pic.at[i - 1]) / (pic.at[i] - pic.at[i - 1]).max(1e-4);
        b = a + (b - a) * f.clamp(0.0, 1.0);
    }
    Some((a, b))
}

/// A vector turned a quarter the way the art engines' tools were drawn
/// (`(y, -x)`): the opposite of [`mdeck_sdk::paint::Vec2::rot90`].
pub fn across(v: mdeck_sdk::paint::Vec2) -> mdeck_sdk::paint::Vec2 {
    -v.rot90()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::cloud::Cloud;
    use mdeck_sdk::paint::Vec2;
    use mdeck_sdk::stage::Picture;
    use mdeck_sdk::tokens::{EngineSettings, Tokens};

    #[test]
    fn a_segment_is_cut_where_the_hand_is() {
        let pic = Strokes {
            points: vec![
                Pos2::new(0.0, 0.0),
                Pos2::new(1.0, 0.0),
                Pos2::new(1.0, 1.0),
            ],
            pen: vec![true, true, false],
            fill: vec![false; 3],
            at: vec![0.0, 1.0, 2.0],
            duration: 2.0,
            born: 0.0,
            weight: 1.0,
        };
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0));
        let (a, b) = drawn_segment(&pic, 1, 0.5, rect).expect("drawing");
        assert_eq!(a, to_screen(pic.points[0], rect));
        assert!((b.x - (a.x + to_screen(pic.points[1], rect).x) / 2.0).abs() < 1e-3);
        assert!(drawn_segment(&pic, 2, 5.0, rect).is_none(), "pen lifted");
    }

    #[test]
    fn across_turns_the_other_way_from_rot90() {
        assert_eq!(across(Vec2::new(1.0, 0.0)), Vec2::new(0.0, -1.0));
    }

    #[test]
    fn a_cloud_picture_is_drawn_in_pen_strokes_and_settles_in_stills() {
        let (tokens, settings) = (Tokens::default(), EngineSettings::new());
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        let mut frame = Frame::new(rect, &tokens, &settings);
        let cloud = Cloud::new(
            "dots",
            (0..40).map(|i| [i as f32 / 40.0, 0.5]).collect(),
            1.0,
        );
        let mut stage = Stage::new(Moment::Slide);
        stage.picture = Some(Picture::new(
            PictureSource::Cloud(Arc::new(cloud)),
            Place {
                u: 0.5,
                v: 0.1,
                w: 0.4,
                h: 0.8,
            },
        ));
        let mut canvas = Canvas::new(3.0, 0.0, 3.0, 0.5);
        canvas.update(&frame, &stage);
        assert!(canvas.strokes.is_some() && canvas.drawing.is_none());
        assert!(canvas.busy());
        frame.still = true;
        canvas.update(&frame, &stage);
        assert!(!canvas.busy(), "a still shows the finished drawing");
        assert!(canvas.tip.is_none());
        // no picture: nothing to draw on a slide
        assert!(fallback_strokes(&frame, &Stage::new(Moment::Slide), 0.0, Fill::Tour).is_none());
    }
}
