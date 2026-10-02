//! The flower's text: the fonts for the centre and the petals, the sizes
//! their text asks for, the layout that needs the least shrinking, and
//! drawing a node's icon, name and description.

use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Vec2};

use super::super::node_text::NodeText;
use super::super::{VIZ_FONT_MIN, VIZ_FONT_PRIMARY_LABEL, VIZ_FONT_SECONDARY_LABEL, VizCtx};
use super::PETAL_ICON;
use super::layout::{self, Layout, Want};
use super::parse::Flower;
use crate::theme::Theme;

/// The centre or a petal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Kind {
    Center,
    Petal,
}

/// The fonts a node's text block is drawn with.
#[derive(Debug, Clone)]
pub(super) struct Fonts {
    name: FontId,
    detail: FontId,
    icon: f32,
    wrap: f32,
}

/// How much wider petal text may wrap, tried in turn: wider petals are
/// flatter, which a short slide needs.
const WIDER: [f32; 3] = [1.0, 1.4, 1.9];

/// The layout whose text needs the least shrinking: petal text wrapped
/// normally, or wider when the flower does not fit at full size otherwise.
pub(super) fn best_layout(cx: &VizCtx, flower: &Flower, rect: Rect) -> (Layout, f32, f32) {
    // (layout, wide, k), scored by the font size it ends up with
    let mut best: Option<(Layout, f32, f32)> = None;
    let score = |l: &Layout, k: f32| k * l.scale.min(1.0);
    for wide in WIDER {
        // shrink the text and lay it out again until the flower fits as is
        let mut k = 1.0_f32;
        loop {
            let l = layout::layout(flower.petals.len(), rect, want(cx, flower, wide, k));
            let next = (k * l.scale).max(FONT_FLOOR * 0.8);
            let fits = l.scale >= 0.999;
            if best
                .as_ref()
                .is_none_or(|(b, _, bk)| score(&l, k) > score(b, *bk))
            {
                best = Some((l, wide, k));
            }
            if fits || next >= k - 1e-3 {
                break;
            }
            k = next;
        }
        if best
            .as_ref()
            .is_some_and(|(b, _, bk)| b.scale >= 0.999 && *bk >= 0.999)
        {
            break;
        }
    }
    best.expect("WIDER is not empty")
}

/// The fonts for `kind` at `k` times full size, petal text wrapping `wide`
/// times its usual width.
pub(super) fn fonts(cx: &VizCtx, kind: Kind, k: f32, wide: f32) -> Fonts {
    let body = cx.theme.body_size * cx.scale * k;
    let (name, wrap, icon) = match kind {
        Kind::Center => (0.85, 7.0, 1.5),
        Kind::Petal => (VIZ_FONT_PRIMARY_LABEL, 7.5, 1.4),
    };
    Fonts {
        name: FontId::new(body * name, cx.theme.strong_family()),
        detail: FontId::new(body * VIZ_FONT_SECONDARY_LABEL, cx.theme.body_family()),
        icon: body * icon,
        wrap: body * wrap * wide,
    }
}

/// The sizes the text asks for at `k` times full font size: a centre circle around
/// its block, and one bulb size for every petal, around the largest block.
pub(super) fn want(cx: &VizCtx, flower: &Flower, wide: f32, k: f32) -> Want {
    let pad = 12.0 * cx.scale;
    let center_rows = flower.center.as_ref().map_or(Vec::new(), |c| {
        block_rows(cx, c, c.icon_or(None), &fonts(cx, Kind::Center, k, 1.0))
    });
    let radius = center_rows
        .iter()
        .flat_map(corners)
        .map(|p| p.to_vec2().length())
        .fold(0.0, f32::max)
        + pad;
    let radius = radius.max(cx.theme.body_size * cx.scale * 2.5 * k);
    let f = fonts(cx, Kind::Petal, k, wide);
    let rows: Vec<Rect> = flower
        .petals
        .iter()
        .flat_map(|p| block_rows(cx, &p.text, p.text.icon_or(Some(PETAL_ICON)), &f))
        .collect();
    let half = ellipse_around(&rows) + Vec2::splat(pad);
    Want { radius, half }
}

pub(super) fn corners(r: &Rect) -> [Pos2; 4] {
    [
        r.left_top(),
        r.right_top(),
        r.left_bottom(),
        r.right_bottom(),
    ]
}

/// The smallest ellipse (by area, between a coin and a cigar) centred on
/// the origin that holds every corner of `rows`.
pub(super) fn ellipse_around(rows: &[Rect]) -> Vec2 {
    let points: Vec<Pos2> = rows.iter().flat_map(corners).collect();
    (0..=14)
        .map(|i| 1.2 + 0.1 * i as f32)
        .map(|aspect| {
            // semi-axes a and a / aspect: every (x/a)^2 + (y*aspect/a)^2 <= 1
            let a = points
                .iter()
                .map(|p| (p.x * p.x + (p.y * aspect).powi(2)).sqrt())
                .fold(0.0, f32::max);
            Vec2::new(a, a / aspect)
        })
        .min_by(|p, q| (p.x * p.y).total_cmp(&(q.x * q.y)))
        .unwrap_or(Vec2::ZERO)
}

/// The rows of a node's block (icon, name rows, detail rows) as drawn by
/// [`draw_text_block`] around the origin.
pub(super) fn block_rows(cx: &VizCtx, node: &NodeText, icon: Option<&str>, f: &Fonts) -> Vec<Rect> {
    let size = block_size(cx, node, icon, f);
    let mut y = -size.y / 2.0;
    let mut out = Vec::new();
    if icon.is_some() {
        out.push(Rect::from_min_size(
            Pos2::new(-f.icon / 2.0, y),
            Vec2::splat(f.icon),
        ));
        y += f.icon * (1.0 + ICON_GAP);
    }
    let (name, detail) = galleys(cx, node, f, Color32::WHITE);
    let at = |g: &egui::Galley, y: f32| {
        g.rows
            .iter()
            .map(|r| r.rect().translate(Vec2::new(0.0, y)))
            .collect::<Vec<_>>()
    };
    out.extend(at(&name, y));
    y += name.size().y + f.detail.size * LINE_GAP;
    if let Some(d) = detail {
        out.extend(at(&d, y));
    }
    out
}

/// The smallest the text may get: secondary text never goes below
/// `VIZ_FONT_MIN`.
const FONT_FLOOR: f32 = VIZ_FONT_MIN / VIZ_FONT_SECONDARY_LABEL;

const ICON_GAP: f32 = 0.2;
const LINE_GAP: f32 = 0.15;

/// The size of a node's block: icon, name, detail, stacked.
pub(super) fn block_size(cx: &VizCtx, node: &NodeText, icon: Option<&str>, f: &Fonts) -> Vec2 {
    let (name, detail) = galleys(cx, node, f, Color32::WHITE);
    let mut size = name.size();
    if let Some(d) = &detail {
        size.y += d.size().y + f.detail.size * LINE_GAP;
        size.x = size.x.max(d.size().x);
    }
    if icon.is_some() {
        size.y += f.icon * (1.0 + ICON_GAP);
        size.x = size.x.max(f.icon);
    }
    size
}

/// The name and the detail, wrapped and centred.
pub(super) fn galleys(
    cx: &VizCtx,
    node: &NodeText,
    f: &Fonts,
    color: Color32,
) -> (Arc<egui::Galley>, Option<Arc<egui::Galley>>) {
    let painter = cx.ui.painter();
    let centred = |text: &str, font: &FontId| {
        let mut job = egui::text::LayoutJob::simple(text.to_string(), font.clone(), color, f.wrap);
        job.halign = egui::Align::Center;
        painter.layout_job(job)
    };
    let name = centred(&node.name, &f.name);
    let detail = node.detail.as_deref().map(|d| centred(d, &f.detail));
    (name, detail)
}

/// Draw a node's icon, name and detail centred on `center`.
pub(super) fn draw_text_block(
    cx: &VizCtx,
    node: &NodeText,
    icon: Option<&str>,
    center: Pos2,
    f: &Fonts,
    alpha: f32,
) {
    if alpha <= 0.0 {
        return;
    }
    let painter = cx.ui.painter();
    let size = block_size(cx, node, icon, f);
    let mut y = center.y - size.y / 2.0;
    if let Some(icon) = icon {
        let at = Pos2::new(center.x, y + f.icon / 2.0);
        // the icons' line weight follows their size
        crate::render::diagram::draw_icon(
            painter,
            icon,
            at,
            f.icon,
            cx.fg(0.85 * alpha),
            f.icon / 36.0,
        );
        y += f.icon * (1.0 + ICON_GAP);
    }
    let (name, detail) = galleys(cx, node, f, cx.fg(alpha));
    let name_h = name.size().y;
    painter.galley(Pos2::new(center.x, y), name, cx.fg(alpha));
    y += name_h + f.detail.size * LINE_GAP;
    if let Some(d) = detail {
        let muted = Theme::with_opacity(cx.theme.foreground, cx.opacity * 0.75 * alpha);
        painter.galley(Pos2::new(center.x, y), d, muted);
    }
}
