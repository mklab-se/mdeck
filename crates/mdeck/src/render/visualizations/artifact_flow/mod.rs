//! `@artifactflow`: how artifacts move from the teams that produce them,
//! through shared infrastructure, to the teams that consume them. Producers
//! and consumers are cards in titled panels on the left and right, services
//! larger cards in the middle, and every artifact an arrow between them
//! with its name (and an icon) on it.

mod card;
mod layout;
mod parse;
pub use parse::check;

use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use super::curve;
use super::{VIZ_FONT_MIN, VIZ_FONT_SECONDARY_LABEL, VIZ_FONT_TITLE, VizCtx};
use crate::theme::Theme;
use card::{Card, LabelBlock, centred, draw_card};
use parse::{Flow, Heading, Node, Role};

/// Default height when the slide does not give the chart one.
const DEFAULT_HEIGHT: f32 = 600.0;
/// Edges, px at 1920x1080.
const EDGE_STROKE: f32 = 3.0;

pub fn draw_artifact_flow(
    cx: &VizCtx,
    content: &str,
    pos: Pos2,
    max_width: f32,
    max_height: f32,
) -> f32 {
    let flow = parse::parse(content);
    if flow.nodes.is_empty() {
        return 0.0;
    }
    let height = if max_height > 0.0 {
        max_height
    } else {
        DEFAULT_HEIGHT * cx.scale
    };
    let rect = Rect::from_min_size(pos, Vec2::new(max_width, height));
    let scene = Scene::fit(cx, &flow, rect);
    scene.draw(cx, &flow);
    height
}

/// Everything laid out: the columns, the panels' headings, and a card per
/// node.
struct Scene {
    columns: [Option<Rect>; 3],
    /// Each column's panel: its heading and cards with padding around them.
    panels: [Option<Rect>; 3],
    cards: Vec<Rect>,
    k: f32,
}

impl Scene {
    /// Lay the flow out at the largest text size (down to the floor) at
    /// which every column's cards fit.
    fn fit(cx: &VizCtx, flow: &Flow, rect: Rect) -> Self {
        let present = Role::ALL.map(|r| flow.column(r).next().is_some());
        let columns = layout::columns(rect, present);
        let floor = VIZ_FONT_MIN / VIZ_FONT_SECONDARY_LABEL * 0.8;
        let mut k = 1.0_f32;
        loop {
            let (cards, panels, fits) = Self::cards(cx, flow, &columns, k);
            if fits || k <= floor {
                return Scene {
                    columns,
                    panels,
                    cards,
                    k,
                };
            }
            k = (k - 0.05).max(floor);
        }
    }

    /// The card rect of every node at text size `k`, each column's panel,
    /// and whether everything fits. A column's heading and cards form one
    /// block, centred in the column.
    #[allow(clippy::type_complexity)]
    fn cards(
        cx: &VizCtx,
        flow: &Flow,
        columns: &[Option<Rect>; 3],
        k: f32,
    ) -> (Vec<Rect>, [Option<Rect>; 3], bool) {
        let mut cards = vec![Rect::NOTHING; flow.nodes.len()];
        let mut panels = [None; 3];
        let mut fits = true;
        let pad = 20.0 * cx.scale * k;
        for role in Role::ALL {
            let c = role.column();
            let Some(col) = columns[c] else {
                continue;
            };
            let inner_w = col.width() - pad * 2.0;
            let head = flow.headings[c]
                .as_ref()
                .map_or(0.0, |h| heading_height(cx, h, inner_w, k) + pad);
            let gap = 24.0 * cx.scale * k;
            let nodes: Vec<(usize, &Node)> = flow.column(role).collect();
            let heights: Vec<f32> = nodes
                .iter()
                .map(|(_, n)| Card::new(cx, n, inner_w, k).height)
                .collect();
            let stack = heights.iter().sum::<f32>() + gap * heights.len().saturating_sub(1) as f32;
            let block = head + stack;
            fits &= block + pad * 2.0 <= col.height();
            let top = (col.center().y - block / 2.0).max(col.top() + pad);
            let area = Rect::from_min_size(
                Pos2::new(col.left() + pad, top + head),
                Vec2::new(inner_w, stack),
            );
            for ((i, _), r) in nodes.iter().zip(layout::stack(area, &heights, gap)) {
                cards[*i] = r;
            }
            panels[c] = Some(Rect::from_min_max(
                Pos2::new(col.left(), top - pad),
                Pos2::new(col.right(), top + block + pad),
            ));
        }
        (cards, panels, fits)
    }

    fn draw(&self, cx: &VizCtx, flow: &Flow) {
        let colors = column_colors(cx);
        // panels and headings
        for role in Role::ALL {
            let c = role.column();
            let Some(panel) = self.panels[c] else {
                continue;
            };
            if role != Role::Service {
                let fill = Theme::with_opacity(colors[c], cx.opacity * 0.07);
                cx.ui.painter().rect_filled(panel, 16.0 * cx.scale, fill);
            }
            if let Some(h) = &flow.headings[c] {
                draw_heading(cx, panel, h, self.k);
            }
        }
        self.draw_edges(cx, flow, &colors);
        for (i, node) in flow.nodes.iter().enumerate() {
            if node.step <= cx.reveal_step {
                let color = colors[node.role.column()];
                draw_card(cx, node, self.cards[i], color, self.k, cx.anim(node.step));
            }
        }
    }

    /// The edges under the cards, then their labels on top of everything
    /// in the gaps, pushed apart where they would touch.
    fn draw_edges(&self, cx: &VizCtx, flow: &Flow, colors: &[Color32; 3]) {
        let painter = cx.ui.painter();
        let routes = self.routes(flow);
        // how many edges meet each card on each side, to put a label at
        // the quieter end of its edge
        let mut crowd: std::collections::HashMap<(usize, bool), usize> = Default::default();
        for e in &flow.edges {
            let forward = flow.nodes[e.from].role.column() < flow.nodes[e.to].role.column();
            *crowd.entry((e.from, forward)).or_default() += 1;
            *crowd.entry((e.to, !forward)).or_default() += 1;
        }
        let mut labels: Vec<(Rect, LabelBlock, f32, usize)> = Vec::new();
        for (e, points) in flow.edges.iter().zip(&routes) {
            let Some(points) = points else { continue };
            if e.step > cx.reveal_step {
                continue;
            }
            let anim = cx.anim(e.step);
            let consumer = [e.from, e.to]
                .iter()
                .any(|&i| flow.nodes[i].role == Role::Consumer);
            let color = colors[if consumer { 2 } else { 0 }];
            let stroke = Stroke::new(
                EDGE_STROKE * cx.scale,
                Theme::with_opacity(color, cx.opacity * anim),
            );
            curve::draw_arrow(painter, &curve::trim(points, anim), stroke, 14.0 * cx.scale);
            if e.label.is_some() || e.icon.is_some() {
                let gap = self.gap_between(flow, e.from, e.to);
                let block = LabelBlock::new(
                    cx,
                    e.label.as_deref(),
                    e.icon.as_deref(),
                    gap.width(),
                    self.k,
                );
                // at the end where fewer edges meet, just above the curve
                // wherever the label spans it
                let forward = flow.nodes[e.from].role.column() < flow.nodes[e.to].role.column();
                let at_source = crowd[&(e.from, forward)] <= crowd[&(e.to, !forward)];
                let margin = 10.0 * cx.scale;
                let near_left = at_source == forward;
                let l = if near_left {
                    gap.left() + margin
                } else {
                    gap.right() - margin - block.size.x
                };
                let r = l + block.size.x;
                let top_of_curve = points
                    .iter()
                    .filter(|p| p.x >= l && p.x <= r)
                    .map(|p| p.y)
                    .fold(curve::point_at(points, 0.5).y, f32::min);
                let r = Rect::from_min_size(
                    Pos2::new(l, top_of_curve - 6.0 * cx.scale - block.size.y),
                    block.size,
                );
                labels.push((
                    r,
                    block,
                    super::label_fade(anim),
                    gap_index(flow, e.from, e.to),
                ));
            }
        }
        // keep labels in the same gap from touching
        for g in 0..2 {
            let mut group: Vec<usize> = (0..labels.len()).filter(|&i| labels[i].3 == g).collect();
            group.sort_by(|&a, &b| labels[a].0.top().total_cmp(&labels[b].0.top()));
            let mut rects: Vec<Rect> = group.iter().map(|&i| labels[i].0).collect();
            let within = self
                .columns
                .iter()
                .flatten()
                .next()
                .copied()
                .unwrap_or(Rect::NOTHING);
            layout::separate_up(&mut rects, 4.0 * cx.scale, within);
            for (&i, r) in group.iter().zip(rects) {
                labels[i].0 = r;
            }
        }
        for (r, block, alpha, _) in &labels {
            block.draw(cx, *r, *alpha);
        }
    }

    /// Every edge's curve from its source card's side to its target's, or
    /// `None` for an edge within one column. Edges meeting one side of a
    /// card share it, ordered by where their other end is so they never
    /// cross there.
    fn routes(&self, flow: &Flow) -> Vec<Option<Vec<Pos2>>> {
        use std::collections::HashMap;
        let col = |i: usize| flow.nodes[i].role.column();
        // (card, right side?) -> edges meeting it there
        let mut sides: HashMap<(usize, bool), Vec<usize>> = HashMap::new();
        for (k, e) in flow.edges.iter().enumerate() {
            if col(e.from) == col(e.to) {
                continue;
            }
            let forward = col(e.from) < col(e.to);
            sides.entry((e.from, forward)).or_default().push(k);
            sides.entry((e.to, !forward)).or_default().push(k);
        }
        let mut ends: HashMap<(usize, usize), Pos2> = HashMap::new();
        for ((card, right), mut edges) in sides {
            let other = |k: usize| {
                let e = &flow.edges[k];
                let o = if e.from == card { e.to } else { e.from };
                self.cards[o].center().y
            };
            edges.sort_by(|&a, &b| other(a).total_cmp(&other(b)));
            let r = self.cards[card];
            let x = if right { r.right() } else { r.left() };
            for (k, p) in edges
                .iter()
                .zip(layout::ports(x, r.top(), r.bottom(), edges.len()))
            {
                ends.insert((*k, card), p);
            }
        }
        flow.edges
            .iter()
            .enumerate()
            .map(|(k, e)| {
                let (a, b) = (*ends.get(&(k, e.from))?, *ends.get(&(k, e.to))?);
                Some(curve::sample(curve::horizontal_s(a, b), 40))
            })
            .collect()
    }

    /// The space between the columns an edge crosses (the first gap it
    /// crosses when it skips the middle).
    fn gap_between(&self, flow: &Flow, from: usize, to: usize) -> Rect {
        let (a, b) = (flow.nodes[from].role.column(), flow.nodes[to].role.column());
        let (lo, hi) = (a.min(b), a.max(b));
        let left = self.columns[lo];
        let right = (lo + 1..=hi).find_map(|c| self.columns[c]);
        match (left, right) {
            (Some(l), Some(r)) => Rect::from_min_max(
                Pos2::new(l.right(), l.top()),
                Pos2::new(r.left(), l.bottom()),
            ),
            _ => Rect::NOTHING,
        }
    }
}

/// Which gap (0 left, 1 right) an edge's label sits in.
fn gap_index(flow: &Flow, from: usize, to: usize) -> usize {
    let lo = flow.nodes[from]
        .role
        .column()
        .min(flow.nodes[to].role.column());
    lo.min(1)
}

/// Producers, services, consumers.
fn column_colors(cx: &VizCtx) -> [Color32; 3] {
    let p = cx.theme.edge_palette();
    [p[0], p[2 % p.len()], p[4 % p.len()]]
}

// ─── Headings ───────────────────────────────────────────────────────────────

fn heading_fonts(cx: &VizCtx, k: f32) -> (FontId, FontId) {
    let body = cx.theme.body_size * cx.scale * k;
    (
        FontId::new(body * VIZ_FONT_TITLE, cx.theme.strong_family()),
        FontId::new(body * VIZ_FONT_SECONDARY_LABEL, cx.theme.body_family()),
    )
}

fn heading_galleys(
    cx: &VizCtx,
    h: &Heading,
    width: f32,
    k: f32,
    alpha: f32,
) -> (Arc<egui::Galley>, Option<Arc<egui::Galley>>) {
    let (title, sub) = heading_fonts(cx, k);
    let title = centred(cx, &h.title, title, cx.fg(alpha), width);
    let sub = h
        .subtitle
        .as_deref()
        .map(|s| centred(cx, s, sub, cx.fg(0.7 * alpha), width));
    (title, sub)
}

/// The height of a heading's title and subtitle.
fn heading_height(cx: &VizCtx, h: &Heading, width: f32, k: f32) -> f32 {
    let (t, s) = heading_galleys(cx, h, width, k, 1.0);
    t.size().y + s.map_or(0.0, |s| s.size().y + 4.0 * cx.scale)
}

fn draw_heading(cx: &VizCtx, col: Rect, h: &Heading, k: f32) {
    let painter = cx.ui.painter();
    let pad = 20.0 * cx.scale * k;
    let inner = col.shrink(pad);
    let (t, s) = heading_galleys(cx, h, inner.width(), k, 1.0);
    let th = t.size().y;
    painter.galley(Pos2::new(inner.center().x, inner.top()), t, cx.fg(1.0));
    if let Some(s) = s {
        painter.galley(
            Pos2::new(inner.center().x, inner.top() + th + 4.0 * cx.scale),
            s,
            cx.fg(0.7),
        );
    }
}
