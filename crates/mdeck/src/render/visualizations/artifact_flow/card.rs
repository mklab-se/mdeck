//! Cards, edge labels and the text helpers they share.

use std::sync::Arc;

use eframe::egui::{self, Align, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use super::super::{
    VIZ_CORNER_CARD, VIZ_FONT_PRIMARY_LABEL, VIZ_FONT_SECONDARY_LABEL, VIZ_FONT_VALUE_LABEL, VizCtx,
};
use super::parse::{Node, Role};
use crate::render::hints::{self, Hint};
use crate::theme::Theme;

// ─── Cards ──────────────────────────────────────────────────────────────────

/// A card's text laid out for a width: producers and consumers put the icon
/// beside the name; a service is larger and centred, its icon on top.
pub(super) struct Card {
    icon: Option<String>,
    icon_size: f32,
    name: Arc<egui::Galley>,
    detail: Option<Arc<egui::Galley>>,
    items: Vec<Arc<egui::Galley>>,
    pad: f32,
    line_gap: f32,
    centred: bool,
    pub(super) height: f32,
}

impl Card {
    pub(super) fn new(cx: &VizCtx, node: &Node, width: f32, k: f32) -> Self {
        let body = cx.theme.body_size * cx.scale * k;
        let service = node.role == Role::Service;
        let pad = 18.0 * cx.scale * k;
        let line_gap = 6.0 * cx.scale * k;
        let icon = node
            .text
            .icon_or(Some(node.role.default_icon()))
            .map(str::to_string);
        let icon_size = body * if service { 2.0 } else { 1.25 };
        let inner = width - pad * 2.0;
        let name_font = FontId::new(
            body * if service {
                0.85
            } else {
                VIZ_FONT_PRIMARY_LABEL
            },
            cx.theme.strong_family(),
        );
        let small = FontId::new(body * VIZ_FONT_SECONDARY_LABEL, cx.theme.body_family());
        let (name, detail) = if service {
            (
                centred(cx, &node.text.name, name_font, Color32::PLACEHOLDER, inner),
                node.text
                    .detail
                    .as_deref()
                    .map(|d| centred(cx, d, small.clone(), Color32::PLACEHOLDER, inner)),
            )
        } else {
            let beside = inner - icon.as_ref().map_or(0.0, |_| icon_size + pad * 0.6);
            (
                wrapped(cx, &node.text.name, name_font, beside),
                node.text
                    .detail
                    .as_deref()
                    .map(|d| wrapped(cx, d, small.clone(), inner)),
            )
        };
        let items: Vec<Arc<egui::Galley>> = node
            .items
            .iter()
            .map(|i| wrapped(cx, &format!("•  {i}"), small.clone(), inner))
            .collect();
        let mut height = pad * 2.0;
        height += if service {
            icon.as_ref().map_or(0.0, |_| icon_size + line_gap) + name.size().y
        } else {
            name.size()
                .y
                .max(if icon.is_some() { icon_size } else { 0.0 })
        };
        if let Some(d) = &detail {
            height += line_gap + d.size().y;
        }
        if !items.is_empty() {
            height += line_gap * 1.5
                + items
                    .iter()
                    .map(|g| g.size().y + line_gap * 0.5)
                    .sum::<f32>();
        }
        Card {
            icon,
            icon_size,
            name,
            detail,
            items,
            pad,
            line_gap,
            centred: service,
            height,
        }
    }
}

pub(super) fn draw_card(cx: &VizCtx, node: &Node, rect: Rect, color: Color32, k: f32, anim: f32) {
    let painter = cx.ui.painter();
    // slide in a little as it appears
    let rect = rect.translate(Vec2::new(0.0, (1.0 - anim) * 12.0 * cx.scale));
    let service = node.role == Role::Service;
    let radius = VIZ_CORNER_CARD * cx.scale;
    let a = cx.opacity * anim;
    painter.rect_filled(rect, radius, Theme::with_opacity(cx.theme.background, a));
    if service {
        painter.rect_filled(rect, radius, Theme::with_opacity(color, a * 0.12));
    }
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(
            if service { 3.0 } else { 2.0 } * cx.scale,
            Theme::with_opacity(color, a * if service { 0.9 } else { 0.6 }),
        ),
        egui::StrokeKind::Inside,
    );
    hints::push(cx.ui.ctx(), Hint::Frame(rect));

    let card = Card::new(cx, node, rect.width(), k);
    let fg = cx.fg(anim);
    let muted = cx.fg(0.75 * anim);
    let inner = rect.shrink(card.pad);
    let mut y = inner.top();
    if card.centred {
        if let Some(icon) = &card.icon {
            let at = Pos2::new(inner.center().x, y + card.icon_size / 2.0);
            crate::render::diagram::draw_icon(
                painter,
                icon,
                at,
                card.icon_size,
                Theme::with_opacity(color, a),
                card.icon_size / 36.0,
            );
            y += card.icon_size + card.line_gap;
        }
        let h = card.name.size().y;
        painter.galley(Pos2::new(inner.center().x, y), card.name.clone(), fg);
        y += h;
        if let Some(d) = &card.detail {
            y += card.line_gap;
            painter.galley(Pos2::new(inner.center().x, y), d.clone(), muted);
            y += d.size().y;
        }
    } else {
        let mut x = inner.left();
        let row = card.name.size().y.max(if card.icon.is_some() {
            card.icon_size
        } else {
            0.0
        });
        if let Some(icon) = &card.icon {
            let at = Pos2::new(x + card.icon_size / 2.0, y + row / 2.0);
            crate::render::diagram::draw_icon(
                painter,
                icon,
                at,
                card.icon_size,
                fg,
                card.icon_size / 36.0,
            );
            x += card.icon_size + card.pad * 0.6;
        }
        let h = card.name.size().y;
        painter.galley(Pos2::new(x, y + (row - h) / 2.0), card.name.clone(), fg);
        y += row;
        if let Some(d) = &card.detail {
            y += card.line_gap;
            painter.galley(Pos2::new(inner.left(), y), d.clone(), muted);
            y += d.size().y;
        }
    }
    if !card.items.is_empty() {
        y += card.line_gap * 1.5;
        // a service's list is a centred block of left-aligned lines
        let w = card.items.iter().map(|g| g.size().x).fold(0.0, f32::max);
        let x = if card.centred {
            inner.center().x - w / 2.0
        } else {
            inner.left()
        };
        for g in &card.items {
            let h = g.size().y;
            painter.galley(Pos2::new(x, y), g.clone(), muted);
            y += h + card.line_gap * 0.5;
        }
    }
}

// ─── Edge labels ────────────────────────────────────────────────────────────

/// An edge's label: an optional icon beside wrapped, left-aligned text.
pub(super) struct LabelBlock {
    icon: Option<String>,
    icon_size: f32,
    text: Option<Arc<egui::Galley>>,
    gap: f32,
    pub(super) size: Vec2,
}

impl LabelBlock {
    pub(super) fn new(
        cx: &VizCtx,
        label: Option<&str>,
        icon: Option<&str>,
        width: f32,
        k: f32,
    ) -> Self {
        let body = cx.theme.body_size * cx.scale * k;
        let icon_size = body * 1.1;
        let gap = 8.0 * cx.scale;
        let room = width - 16.0 * cx.scale - icon.map_or(0.0, |_| icon_size + gap);
        let text = label.map(|l| {
            wrapped(
                cx,
                l,
                FontId::new(body * VIZ_FONT_VALUE_LABEL, cx.theme.body_family()),
                room.max(body * 3.0),
            )
        });
        let tw = text.as_ref().map_or(Vec2::ZERO, |t| t.size());
        let size = Vec2::new(
            tw.x + icon.map_or(0.0, |_| icon_size + if text.is_some() { gap } else { 0.0 }),
            tw.y.max(if icon.is_some() { icon_size } else { 0.0 }),
        ) + Vec2::new(12.0, 6.0) * cx.scale;
        LabelBlock {
            icon: icon.map(str::to_string),
            icon_size,
            text,
            gap,
            size,
        }
    }

    pub(super) fn draw(&self, cx: &VizCtx, rect: Rect, alpha: f32) {
        if alpha <= 0.0 {
            return;
        }
        let painter = cx.ui.painter();
        let halo = Theme::with_opacity(cx.theme.background, cx.opacity * 0.9 * alpha);
        painter.rect_filled(rect, 8.0 * cx.scale, halo);
        let inner = rect.shrink2(Vec2::new(6.0, 3.0) * cx.scale);
        let mut x = inner.left();
        if let Some(icon) = &self.icon {
            let at = Pos2::new(x + self.icon_size / 2.0, inner.center().y);
            crate::render::diagram::draw_icon(
                painter,
                icon,
                at,
                self.icon_size,
                cx.fg(0.8 * alpha),
                self.icon_size / 36.0,
            );
            x += self.icon_size + self.gap;
        }
        if let Some(t) = &self.text {
            let h = t.size().y;
            painter.galley(
                Pos2::new(x, inner.center().y - h / 2.0),
                t.clone(),
                cx.fg(0.9 * alpha),
            );
        }
    }
}

// ─── Text ───────────────────────────────────────────────────────────────────

pub(super) fn wrapped(cx: &VizCtx, text: &str, font: FontId, width: f32) -> Arc<egui::Galley> {
    cx.ui
        .painter()
        .layout(text.to_string(), font, Color32::PLACEHOLDER, width.max(1.0))
}

/// Wrapped and centred on the x it is painted at.
pub(super) fn centred(
    cx: &VizCtx,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text.to_string(), font, color, width.max(1.0));
    job.halign = Align::Center;
    cx.ui.painter().layout_job(job)
}
