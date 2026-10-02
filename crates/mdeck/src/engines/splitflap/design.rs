//! The board's design set (ENG-03): the split-flap board draws every slide
//! itself. Without a live engine (thumbnails, the overview) the set draws
//! the board at rest; it always draws the slide's image (or thermal image)
//! in the panel, which the flaps leave free.

use mdeck_sdk::content::{Block, Slide};
use mdeck_sdk::design::{DesignCx, DesignSet};
use mdeck_sdk::geometry::Hint;
use mdeck_sdk::paint::{Color, Mesh, Pos2, Rect, Stroke, Texture, Vec2};
use mdeck_sdk::problem::Problem;

use super::draw::{self, Geometry, Labels, Scene, View};
use super::layout;
use super::panel_rect;
use super::writer::THERMAL;

/// The split-flap board's design set.
pub static BOARD: Board = Board;

/// See the module docs.
pub struct Board;

impl DesignSet for Board {
    fn name(&self) -> &str {
        "splitflap"
    }

    fn render(&self, cx: &mut DesignCx, slide: &Slide, rect: Rect) {
        let scale = cx.scale();
        let title = layout::is_title(slide, cx.index());
        let board = layout::lay_out(slide, title, cx.step());
        let geo = Geometry::new(rect, scale);
        if !cx.engine_live() {
            let views: Vec<View> = board
                .cells
                .iter()
                .map(|c| View {
                    from: *c,
                    to: *c,
                    t: 0.0,
                })
                .collect();
            let right = format!("{:02} / {:02}", cx.index() + 1, cx.count().max(1));
            let left = cx.deck_title().unwrap_or("").to_string();
            let scene = Scene {
                views: &views,
                hidden: board.image.map(|_| panel_rect(&geo)),
                labels: Some(Labels {
                    left: &left,
                    right: &right,
                }),
            };
            // the painter carries the slide's opacity already
            draw::paint(cx.painter(), &geo, cx.tokens(), &scene, 1.0, scale);
        }
        match board.image.and_then(|i| slide.blocks.get(i)) {
            Some(Block::Image { path, .. }) => panel_image(cx, path, panel_rect(&geo)),
            // the composed evidence view, annotations and all
            Some(Block::Visual {
                tag,
                content,
                step_base,
            }) if tag == THERMAL => {
                let panel = panel_rect(&geo).shrink(8.0 * scale);
                let step = cx.step().saturating_sub(*step_base);
                cx.visual(THERMAL, content, panel, step);
            }
            _ => {}
        }
    }

    fn unsupported(&self, slide: &Slide) -> Vec<Problem> {
        layout::problems(slide)
    }
}

/// The image fills the panel (cropped to cover it), with the flaps' corner
/// radius and a hairline frame; a panel without its image is the code
/// background.
fn panel_image(cx: &mut DesignCx, path: &str, panel: Rect) {
    let scale = cx.scale();
    let radius = 6.0 * scale;
    let tokens = cx.tokens().clone();
    match cx.image(path) {
        Some(texture) => {
            let [tw, th] = texture.size().map(|n| n.max(1) as f32);
            let k = (panel.width() / tw).max(panel.height() / th);
            let seen = Vec2::new(panel.width() / k / tw, panel.height() / k / th);
            let uv = Rect::from_center_size(Pos2::new(0.5, 0.5), seen);
            // on whole pixels, like any image rect, so the picture is sampled crisply
            let ppp = cx.painter().pixels_per_point();
            let snap = |p: Pos2| Pos2::new((p.x * ppp).round() / ppp, (p.y * ppp).round() / ppp);
            let crisp = Rect::from_min_max(snap(panel.min), snap(panel.max));
            cx.painter().mesh(rounded_image(texture, crisp, uv, radius));
            cx.publish(Hint::Frame(panel));
        }
        None => cx
            .painter()
            .rect_filled(panel, radius, tokens.code_background),
    }
    // a hairline just outside the panel
    let width = 1.0 * scale;
    cx.painter().rect_stroke(
        panel.expand(width / 2.0),
        radius + width / 2.0,
        Stroke::new(width, tokens.rule),
    );
}

/// `uv` of `texture` in `rect` with rounded corners: a fan from the centre.
fn rounded_image(texture: Texture, rect: Rect, uv: Rect, radius: f32) -> Mesh {
    use std::f32::consts::{FRAC_PI_2, PI};
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let mut outline = Vec::new();
    for (c, a0) in [
        (Pos2::new(rect.left() + r, rect.top() + r), PI),
        (Pos2::new(rect.right() - r, rect.top() + r), 1.5 * PI),
        (Pos2::new(rect.right() - r, rect.bottom() - r), 0.0),
        (Pos2::new(rect.left() + r, rect.bottom() - r), 0.5 * PI),
    ] {
        for k in 0..=6 {
            let a = a0 + k as f32 / 6.0 * FRAC_PI_2;
            outline.push(Pos2::new(c.x + r * a.cos(), c.y + r * a.sin()));
        }
    }
    let to_uv = |p: Pos2| {
        let (u, v) = rect.fraction_of(p);
        uv.lerp_inside(u, v)
    };
    let mut m = Mesh::with_texture(texture);
    let c = rect.center();
    let base = m.vertex(c, to_uv(c), Color::WHITE);
    for p in &outline {
        m.vertex(*p, to_uv(*p), Color::WHITE);
    }
    let n = outline.len() as u32;
    for k in 0..n {
        m.triangle(base, base + 1 + k, base + 1 + (k + 1) % n);
    }
    m
}
