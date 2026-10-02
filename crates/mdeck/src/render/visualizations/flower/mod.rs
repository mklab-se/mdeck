//! `@flower`: a platform in the middle and the teams around it. Each petal
//! is a bulb in its own colour whose outline runs out of the centre, around
//! the bulb and back in with an arrow (the team takes from the platform and
//! gives back). Optional `A -> B` links curve between petals across the
//! flower.

mod layout;
mod parse;
pub use parse::check;
mod text;

use std::f32::consts::TAU;

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

use super::curve::{self, bezier};
use super::node_text::NodeText;
use super::{VIZ_FONT_VALUE_LABEL, VizCtx};
use crate::render::hints::{self, Hint};
use crate::theme::Theme;
use layout::{Layout, PetalBox, ellipse_radius};
use text::{Fonts, Kind, best_layout, draw_text_block, fonts};

/// Default height when the slide does not give the chart one.
const DEFAULT_HEIGHT: f32 = 640.0;
/// The petal outline and the centre ring, px at 1920x1080.
const STROKE: f32 = 4.0;
/// How far apart the stem's two ends sit on the centre ring, radians.
const STEM_SPREAD: f32 = 0.32;
/// The petals' icon when they do not name one.
const PETAL_ICON: &str = "team";

pub fn draw_flower(cx: &VizCtx, content: &str, pos: Pos2, max_width: f32, max_height: f32) -> f32 {
    let flower = parse::parse(content);
    if flower.petals.is_empty() && flower.center.is_none() {
        return 0.0;
    }
    let height = if max_height > 0.0 {
        max_height
    } else {
        DEFAULT_HEIGHT * cx.scale
    };
    let rect = Rect::from_min_size(pos, Vec2::new(max_width, height));
    let (layout, wide, k) = best_layout(cx, &flower, rect);
    // the text shrinks with its shapes, so it never spills out of them
    let k = k * layout.scale.min(1.0);
    let (center_fonts, petal_fonts) = (
        fonts(cx, Kind::Center, k, 1.0),
        fonts(cx, Kind::Petal, k, wide),
    );

    // petals that grow out of the centre as they are revealed
    let shown: Vec<Option<(PetalBox, f32)>> = flower
        .petals
        .iter()
        .zip(&layout.petals)
        .map(|(p, b)| (p.step <= cx.reveal_step).then(|| grown(&layout, *b, cx.anim(p.step))))
        .collect();

    let palette = cx.theme.edge_palette();
    let color = |i: usize| palette[i % palette.len()];

    for (i, petal) in shown.iter().enumerate() {
        if let Some((b, anim)) = petal {
            draw_petal_outline(cx, &layout, b, color(i), *anim);
        }
    }
    if let Some(center) = &flower.center
        && flower.center_step <= cx.reveal_step
    {
        draw_center(
            cx,
            &layout,
            center,
            &center_fonts,
            cx.anim(flower.center_step),
        );
    }
    for link in &flower.links {
        if let (Some((a, _)), Some((b, _))) = (shown[link.from], shown[link.to])
            && link.step <= cx.reveal_step
        {
            draw_link(
                cx,
                &layout,
                (&a, &b),
                link.label.as_deref(),
                cx.anim(link.step),
            );
        }
    }
    for (i, petal) in shown.iter().enumerate() {
        if let Some((b, anim)) = petal {
            let node = &flower.petals[i].text;
            let a = super::label_fade(*anim);
            draw_text_block(
                cx,
                node,
                node.icon_or(Some(PETAL_ICON)),
                b.bulb,
                &petal_fonts,
                a,
            );
        }
    }
    height
}

/// A petal `anim` of the way out of the centre: it moves out and grows.
fn grown(layout: &Layout, b: PetalBox, anim: f32) -> (PetalBox, f32) {
    let k = 0.35 + 0.65 * anim;
    (
        PetalBox {
            bulb: layout.center + (b.bulb - layout.center) * k,
            half: b.half * (0.4 + 0.6 * anim),
        },
        anim,
    )
}

/// The bulb's tint, and its outline: out of the centre, around the bulb,
/// and back into the centre with an arrow.
fn draw_petal_outline(cx: &VizCtx, layout: &Layout, b: &PetalBox, color: Color32, anim: f32) {
    let painter = cx.ui.painter();
    let fill = Theme::with_opacity(color, cx.opacity * 0.12 * anim);
    let bounds = Rect::from_center_size(b.bulb, b.half * 2.0);
    painter.add(egui::Shape::ellipse_filled(b.bulb, b.half, fill));
    hints::push(cx.ui.ctx(), Hint::Frame(bounds));

    let stroke = Stroke::new(
        STROKE * cx.scale,
        Theme::with_opacity(color, cx.opacity * anim),
    );
    let path = petal_path(layout, b, cx.scale);
    let (out, back) = path.split_at(path.len() - STEM_SAMPLES - 1);
    painter.add(egui::Shape::line(out.to_vec(), stroke));
    let mut back = back.to_vec();
    back.insert(0, out[out.len() - 1]);
    curve::draw_arrow(painter, &back, stroke, 16.0 * cx.scale);
}

const STEM_SAMPLES: usize = 16;
const ARC_SAMPLES: usize = 72;

/// The petal outline as a polyline: the stem out of the centre ring, the
/// bulb's far side, and the stem back to the ring. The last
/// `STEM_SAMPLES + 1` points are the way back.
fn petal_path(layout: &Layout, b: &PetalBox, scale: f32) -> Vec<Pos2> {
    let c = layout.center;
    let ring = layout.radius + 10.0 * scale;
    let (a, h) = (b.half.x, b.half.y);
    let point = |phi: f32| b.bulb + Vec2::new(a * phi.cos(), h * phi.sin());
    let tangent = |phi: f32| Vec2::new(-a * phi.sin(), h * phi.cos()).normalized();
    // the bulb's outline opens toward the centre
    let to_center = c - b.bulb;
    let psi = (to_center.y / h).atan2(to_center.x / a);
    let open = 0.55;
    let (phi_out, phi_in) = (psi + open, psi + TAU - open);
    let (e_out, e_in) = (point(phi_out), point(phi_in));
    // the stem's ends on the ring, each on its own side
    let toward = (b.bulb - c).angle();
    let on_ring = |t: f32| c + Vec2::angled(toward + t) * ring;
    // leave on the side of the axis the outline heads for
    let axis = b.bulb - c;
    let side = (axis.x * (e_out - c).y - axis.y * (e_out - c).x).signum();
    let (s_out, s_in) = (on_ring(side * STEM_SPREAD), on_ring(-side * STEM_SPREAD));
    let reach = (to_center.length() - ellipse_radius(b.half, to_center) - ring).max(0.0) * 0.55;
    let out = curve::sample(
        [
            s_out,
            s_out + (s_out - c).normalized() * reach,
            e_out - tangent(phi_out) * reach,
            e_out,
        ],
        STEM_SAMPLES,
    );
    let arc = (1..ARC_SAMPLES)
        .map(|i| point(phi_out + (phi_in - phi_out) * i as f32 / ARC_SAMPLES as f32));
    let back = curve::sample(
        [
            e_in,
            e_in + tangent(phi_in) * reach,
            s_in + (s_in - c).normalized() * reach,
            s_in,
        ],
        STEM_SAMPLES,
    );
    out.into_iter().chain(arc).chain(back).collect()
}

/// A link between two petals: a curve between their facing edges that
/// keeps clear of the centre, bowing outward when the petals face each
/// other across it.
fn draw_link(
    cx: &VizCtx,
    layout: &Layout,
    (a, b): (&PetalBox, &PetalBox),
    label: Option<&str>,
    anim: f32,
) {
    let painter = cx.ui.painter();
    let c = link_curve(layout, a, b, 8.0 * cx.scale);
    let points = curve::trim(&curve::sample(c, 40), anim);
    let color = cx.fg(0.55 * anim);
    curve::draw_arrow(
        painter,
        &points,
        Stroke::new(2.5 * cx.scale, color),
        12.0 * cx.scale,
    );
    if let Some(label) = label {
        let a = super::label_fade(anim);
        let font = cx.font(VIZ_FONT_VALUE_LABEL);
        let galley = painter.layout_no_wrap(label.to_string(), font, cx.fg(0.85 * a));
        let r = Rect::from_center_size(
            bezier(c, 0.5),
            galley.size() + Vec2::new(16.0, 6.0) * cx.scale,
        );
        let halo = Theme::with_opacity(cx.theme.background, cx.opacity * 0.92 * a);
        painter.rect_filled(r, r.height() / 2.0, halo);
        painter.galley(r.center() - galley.size() / 2.0, galley, cx.fg(0.85 * a));
    }
}

/// The curve of a link from bulb `a` to bulb `b`, `gap` off their outlines.
/// Its apex keeps a third of the centre's radius clear of the centre.
fn link_curve(layout: &Layout, a: &PetalBox, b: &PetalBox, gap: f32) -> [Pos2; 4] {
    let center = layout.center;
    let mid = a.bulb + (b.bulb - a.bulb) * 0.5;
    let away = mid - center;
    let dir = if away.length() > layout.radius * 0.5 {
        away.normalized()
    } else {
        // facing each other across the centre: go round one side
        let ab = (b.bulb - a.bulb).normalized();
        Vec2::new(ab.y, -ab.x)
    };
    // a quadratic's apex is halfway to its control point
    let apex = (away.dot(dir)).max(layout.radius * 1.35);
    let control = center + dir * (2.0 * apex - away.dot(dir).max(0.0));
    let start = a.edge_toward(control - a.bulb);
    let end = b.edge_toward(control - b.bulb);
    let start = start + (control - start).normalized() * gap;
    let end = end + (control - end).normalized() * gap;
    [
        start,
        start + (control - start) * (2.0 / 3.0),
        end + (control - end) * (2.0 / 3.0),
        end,
    ]
}

/// The platform: a ring in the theme's accent with its name inside.
fn draw_center(cx: &VizCtx, layout: &Layout, node: &NodeText, fonts: &Fonts, anim: f32) {
    let painter = cx.ui.painter();
    let accent = cx.theme.accent;
    let r = layout.radius * (0.6 + 0.4 * anim);
    painter.circle_filled(layout.center, r, cx.theme.background);
    painter.circle_filled(
        layout.center,
        r,
        Theme::with_opacity(accent, cx.opacity * 0.14 * anim),
    );
    painter.circle_stroke(
        layout.center,
        r,
        Stroke::new(
            STROKE * cx.scale,
            Theme::with_opacity(accent, cx.opacity * anim),
        ),
    );
    hints::push(
        cx.ui.ctx(),
        Hint::Frame(Rect::from_center_size(layout.center, Vec2::splat(r * 2.0))),
    );
    draw_text_block(cx, node, node.icon_or(None), layout.center, fonts, anim);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_petal_path_leaves_and_returns_to_the_ring() {
        let want = layout::Want {
            radius: 120.0,
            half: Vec2::new(150.0, 100.0),
        };
        let l = layout::layout(
            5,
            Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 800.0)),
            want,
        );
        for b in &l.petals {
            let path = petal_path(&l, b, 1.0);
            let ring = l.radius + 10.0;
            let first = path[0].distance(l.center);
            let last = path[path.len() - 1].distance(l.center);
            assert!((first - ring).abs() < 0.01 && (last - ring).abs() < 0.01);
            // it reaches round the far side of the bulb (an ellipse's
            // farthest point from outside need not lie on the axis)
            let far = path
                .iter()
                .map(|p| p.distance(l.center))
                .fold(0.0, f32::max);
            let axis = (b.bulb - l.center).length();
            let outer = axis + ellipse_radius(b.half, b.bulb - l.center);
            assert!(far >= outer - 1.0, "{far} < {outer}");
            assert!(far <= axis + b.half.max_elem() + 1.0, "{far}");
        }
    }
}
