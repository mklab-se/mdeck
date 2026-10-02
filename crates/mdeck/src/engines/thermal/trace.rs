//! The heat trace: the presenter's pen strokes drawn in heat. A stroke
//! arrives white-hot, cools through the palette and is gone after
//! [`HEAT_TRACE_LIFE`]; a soft glow under it keeps it legible as an
//! annotation.

use mdeck_sdk::engine::Annotation;
use mdeck_sdk::paint::{Color, Painter, Stroke};

/// Seconds a heat-trace stroke takes to cool and fade away.
pub const HEAT_TRACE_LIFE: f32 = 4.0;

/// A heat-trace segment `age` seconds old: how hot (0..1, along the heat
/// palette, never below a dull red) and how opaque it is.
pub fn heat_trace_look(age: f32) -> (f32, f32) {
    let t = (age / HEAT_TRACE_LIFE).clamp(0.0, 1.0);
    let heat = 1.0 - 0.7 * t.powf(0.6);
    // full until 60% of its life, then fading to exactly nothing
    let alpha = ((1.0 - t) / 0.4).clamp(0.0, 1.0);
    (heat, alpha)
}

/// The age of point `i` of `stroke`: a finished stroke cools as one, the
/// stroke being drawn (age 0) is white-hot at its newest point while older
/// stretches already cool, a sixtieth of a second per point.
pub fn point_age(stroke: &Annotation, i: usize) -> f32 {
    if stroke.age > 0.0 {
        stroke.age
    } else {
        (stroke.points.len() - 1 - i) as f32 / 60.0
    }
}

/// `c` at `opacity` with normal blending, from its opaque channels.
fn with_opacity(c: Color, opacity: f32) -> Color {
    Color::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (opacity * 255.0) as u8)
}

/// Draw `strokes` in heat through `lut`. Returns whether any segment is
/// still visible.
pub fn draw(painter: &Painter, strokes: &[Annotation], lut: &[Color; 256]) -> bool {
    let mut alive = false;
    for stroke in strokes {
        let pts = &stroke.points;
        for i in 1..pts.len() {
            let (heat, alpha) = heat_trace_look(point_age(stroke, i));
            if alpha <= 0.0 {
                continue;
            }
            alive = true;
            let c = lut[(heat * 255.0) as usize];
            let seg = [pts[i - 1], pts[i]];
            painter.line_segment(
                seg,
                Stroke::new(stroke.width * 2.6, with_opacity(c, alpha * 0.25)),
            );
            painter.line_segment(seg, Stroke::new(stroke.width, with_opacity(c, alpha)));
        }
    }
    alive
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdeck_sdk::paint::Pos2;

    #[test]
    fn a_trace_arrives_white_hot_cools_and_fades_away() {
        let (h0, a0) = heat_trace_look(0.0);
        assert_eq!((h0, a0), (1.0, 1.0));
        let (h1, a1) = heat_trace_look(1.5);
        assert!(h1 < 0.8 && a1 == 1.0, "cooler, still fully there");
        let (h2, a2) = heat_trace_look(3.4);
        assert!(h2 < h1 && a2 < 1.0 && a2 > 0.0, "fading");
        assert_eq!(heat_trace_look(HEAT_TRACE_LIFE).1, 0.0, "gone");
        assert!(heat_trace_look(10.0).0 >= 0.3, "never black");
    }

    #[test]
    fn the_stroke_being_drawn_cools_along_its_length() {
        let mut s = Annotation {
            points: vec![Pos2::ZERO; 61],
            color: Color::WHITE,
            width: 6.0,
            age: 0.0,
        };
        assert_eq!(point_age(&s, 60), 0.0, "the newest point is white-hot");
        assert!((point_age(&s, 0) - 1.0).abs() < 1e-6);
        s.age = 2.0;
        assert_eq!(point_age(&s, 0), 2.0);
        assert_eq!(point_age(&s, 60), 2.0, "a finished stroke cools as one");
    }
}
