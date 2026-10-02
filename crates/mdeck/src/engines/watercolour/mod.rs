//! The watercolour engine: every slide is a sheet of cold-press paper (the
//! theme's `page:`), and the slide's generated watercolour (`mdeck ai pictures`)
//! blooms onto it: a pale first wash over the whole picture, then the
//! colour spreading outward from where the paint is heaviest, the dark
//! accents dropped in last. Without art, the slide's point cloud picture is
//! drawn in ink with a loose wash beside it; the countdown and the end
//! words are painted the same way. Exports show the dry painting.

use mdeck_sdk::engine::{Capabilities, Engine, EngineDef, Medium, MediumKind, Needs};
use mdeck_sdk::paint::{Color, Painter, Rect, Stroke, Vec2, mix, premul};
use mdeck_sdk::stage::{Frame, Stage, Strategy};
use mdeck_sdk::tokens::Tokens;

use super::art::strokes::{Strokes, to_screen};
use super::art::{Canvas, Drawing, Hand, Reveal};
use super::hash01;

/// Watercolour asks for finished paintings and lets them bloom.
pub const MEDIUM: Medium = Medium {
    name: "watercolour",
    kind: MediumKind::Tonal,
    strategy: Strategy::Bloom,
};

pub static DEF: EngineDef = EngineDef {
    name: "watercolour",
    summary: "Generated paintings that bloom onto cold-press paper.",
    capabilities: Capabilities {
        medium: Some(MEDIUM),
        ..super::art::CAPABILITIES
    },
    settings: &[],
    needs: Needs { page: true },
    ending_caption_delay: 5.4,
    create: |_| Box::new(Watercolour::new()),
    board: None,
};

/// The end words hold this long, then fade.
const END_WORDS: f32 = 3.8;
/// Seconds for a painting to bloom.
const DRAW: f32 = 4.6;

/// Wet edges: pixels take a long while to arrive, and a pale first wash
/// runs well ahead of the colour.
const REVEAL: Reveal = Reveal {
    soft: 0.10,
    ghost: 0.12,
    ghost_speed: 3.0,
    grain: 0.0,
};

/// Line art on watercolour paper is an ink drawing.
const LINE_REVEAL: Reveal = Reveal {
    soft: 0.03,
    ghost: 0.0,
    ghost_speed: 1.0,
    grain: 0.0,
};

/// A point cloud without an artwork: its lines in ink and the shape laid in
/// with washes, the brush following loose diagonal strokes.
const FILL: crate::engines::art::Fill =
    crate::engines::art::Fill::Lines(Some(crate::engines::art::trace::Shading {
        spacing: 0.05,
        everywhere: true,
        cross: false,
    }));

pub struct Watercolour {
    canvas: Canvas,
}

impl Watercolour {
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(DRAW, 0.0, END_WORDS, 0.8).with_fill(FILL),
        }
    }
}

impl Default for Watercolour {
    fn default() -> Self {
        Self::new()
    }
}

/// The paints: an ink for lines and the wash colours.
struct Paint {
    ink: Color,
    washes: [Color; 3],
}

impl Paint {
    fn of(t: &Tokens) -> Self {
        Paint {
            ink: mix(t.heading, t.background, 0.1),
            washes: [t.accent, t.secondary, t.accent_soft],
        }
    }
}

impl Engine for Watercolour {
    fn update(&mut self, frame: &Frame, stage: &Stage) {
        self.canvas.update(frame, stage);
    }

    fn paint(&mut self, painter: &mut Painter, frame: &Frame, _stage: &Stage) {
        self.canvas.paint(painter, frame, &Paint::of(frame.tokens));
    }

    fn animating(&self) -> bool {
        self.canvas.moving
    }
}

/// Tonal paintings bloom in their own colours; line art is an ink drawing,
/// and pen strokes get a wash.
impl Hand for Paint {
    fn backdrop(&self) -> f32 {
        0.35
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
        let line = d.picture.strategy == Strategy::Draw;
        let (tint, reveal) = if line {
            (premul(self.ink, k), LINE_REVEAL)
        } else {
            (premul(Color::WHITE, k), REVEAL)
        };
        d.paint(painter, frame.rect, now, tint, reveal);
    }

    fn strokes(&self, painter: &Painter, frame: &Frame, p: &Strokes, now: f32, k: f32, _: bool) {
        ink_and_wash(painter, p, now, frame.rect, frame.scale, self, k);
    }
}

/// Pen strokes in ink, with a loose wash laid along them a moment later:
/// wide, pale and a little off the line, the way a quick sketch is washed.
fn ink_and_wash(
    painter: &Painter,
    pic: &Strokes,
    now: f32,
    rect: Rect,
    scale: f32,
    paint: &Paint,
    opacity: f32,
) {
    if opacity <= 0.0 || pic.points.len() < 2 {
        return;
    }
    let t = now - pic.born;
    let wash_t = t - 0.5;
    // a shaded drawing is washed where it is shaded; plain strokes along
    // their lines
    let shaded = pic.fill.iter().any(|f| *f);
    let mut washes = Vec::new();
    let mut lines = Vec::new();
    for i in 1..pic.points.len() {
        if !pic.pen[i] {
            continue;
        }
        let a = to_screen(pic.points[i - 1], rect);
        let b = to_screen(pic.points[i], rect);
        if pic.at[i - 1] <= wash_t && pic.fill[i] == shaded {
            let colour = paint.washes[(i / 40) % 3];
            let off = Vec2::new(
                hash01(i as u32 / 12) - 0.5,
                hash01(i as u32 / 12 + 99) - 0.5,
            ) * 14.0
                * scale;
            let (width, alpha) = if pic.fill[i] {
                (44.0, 0.075)
            } else {
                (30.0, 0.10)
            };
            washes.push((
                [a + off, b + off],
                Stroke::new(width * scale, premul(colour, alpha * opacity * pic.weight)),
            ));
        }
        // the shading is all wash, no ink
        if pic.at[i - 1] > t || pic.fill[i] {
            continue;
        }
        let mut b = b;
        if pic.at[i] > t {
            let f = (t - pic.at[i - 1]) / (pic.at[i] - pic.at[i - 1]).max(1e-4);
            b = a + (b - a) * f.clamp(0.0, 1.0);
        }
        lines.push((
            [a, b],
            Stroke::new(
                1.8 * scale,
                premul(paint.ink, 0.85 * opacity * pic.weight.max(0.4)),
            ),
        ));
    }
    for (seg, stroke) in washes.into_iter().chain(lines) {
        painter.line_segment(seg, stroke);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watercolour_asks_for_paintings_that_bloom() {
        assert_eq!(MEDIUM.kind, MediumKind::Tonal);
        assert_eq!(MEDIUM.strategy, Strategy::Bloom);
        assert_eq!(DEF.capabilities.medium.map(|m| m.name), Some("watercolour"));
    }
}
