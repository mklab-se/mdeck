//! Drawing the blueprint: the sheet, the title block, dimension lines, the
//! drafting machine and the pen.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

use super::super::hash01;
use super::super::paint::{SPRITE_CORE, SPRITE_GLOW, additive, mix, premul};
use super::super::stage::Stage;
use crate::render::strokes::{Picture, to_screen};
use crate::theme::Theme;

/// The sheet's colours.
pub(super) struct Ink {
    /// The sheet itself.
    pub(super) paper: Color32,
    /// Inked lines.
    pub(super) line: Color32,
    /// The grid, the border, construction lines.
    pub(super) faint: Color32,
    /// The pen's glint.
    pub(super) glint: Color32,
    pub(super) text: Color32,
    pub(super) muted: Color32,
}

impl Ink {
    pub(super) fn of(theme: &Theme) -> Self {
        Ink {
            paper: theme.background,
            line: theme.heading_color,
            faint: theme.rule,
            glint: theme.particle_light,
            text: theme.foreground,
            muted: theme.muted,
        }
    }
}

/// The blue sheet: cyanotype mottling, a fine grid with heavier lines
/// every fifth, and a ruled double border with zone ticks.
pub(super) fn sheet(
    painter: &egui::Painter,
    texture: egui::TextureId,
    rect: Rect,
    scale: f32,
    ink: &Ink,
    opacity: f32,
) {
    // the uneven wash of a real cyanotype
    let mut mesh = egui::Mesh::with_texture(texture);
    for k in 0..26u32 {
        let x = rect.left() + hash01(k * 5 + 1) * rect.width();
        let y = rect.top() + hash01(k * 5 + 2) * rect.height();
        let r = (260.0 + 420.0 * hash01(k * 5 + 3)) * scale;
        let light = hash01(k * 5 + 4) > 0.45;
        let c = if light {
            mix(ink.paper, Color32::WHITE, 0.5)
        } else {
            mix(ink.paper, Color32::BLACK, 0.6)
        };
        mesh.add_rect_with_uv(
            Rect::from_center_size(Pos2::new(x, y), egui::vec2(r * 2.0, r * 2.0)),
            SPRITE_GLOW,
            premul(c, 0.05 * opacity),
        );
    }
    // darker toward the edges
    for (cx, cy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        let r = rect.width() * 0.42;
        mesh.add_rect_with_uv(
            Rect::from_center_size(
                Pos2::new(
                    rect.left() + cx * rect.width(),
                    rect.top() + cy * rect.height(),
                ),
                egui::vec2(r * 2.0, r * 2.0),
            ),
            SPRITE_GLOW,
            premul(mix(ink.paper, Color32::BLACK, 0.7), 0.22 * opacity),
        );
    }
    painter.add(egui::Shape::mesh(mesh));

    let inset = 22.0 * scale;
    let frame = rect.shrink(inset);
    let step = 24.0 * scale;
    let cols = (frame.width() / step) as i32;
    let rows = (frame.height() / step) as i32;
    let grid = |major: bool| premul(ink.faint, if major { 0.42 } else { 0.16 } * opacity);
    for i in 1..cols {
        let x = frame.left() + i as f32 * step;
        painter.line_segment(
            [Pos2::new(x, frame.top()), Pos2::new(x, frame.bottom())],
            Stroke::new(1.0, grid(i % 5 == 0)),
        );
    }
    for j in 1..rows {
        let y = frame.top() + j as f32 * step;
        painter.line_segment(
            [Pos2::new(frame.left(), y), Pos2::new(frame.right(), y)],
            Stroke::new(1.0, grid(j % 5 == 0)),
        );
    }
    // the border: a heavy inner line and a fine outer one, zone ticks between
    let border = premul(ink.line, 0.55 * opacity);
    painter.rect_stroke(
        frame,
        0.0,
        Stroke::new(2.2 * scale, border),
        egui::StrokeKind::Middle,
    );
    let outer = frame.expand(8.0 * scale);
    painter.rect_stroke(
        outer,
        0.0,
        Stroke::new(1.0, premul(ink.line, 0.35 * opacity)),
        egui::StrokeKind::Middle,
    );
    let zones = 8;
    for z in 1..zones {
        let x = frame.left() + frame.width() * z as f32 / zones as f32;
        for (y0, y1) in [(outer.top(), frame.top()), (frame.bottom(), outer.bottom())] {
            painter.line_segment(
                [Pos2::new(x, y0), Pos2::new(x, y1)],
                Stroke::new(1.0, premul(ink.line, 0.35 * opacity)),
            );
        }
    }
    for z in 1..4 {
        let y = frame.top() + frame.height() * z as f32 / 4.0;
        for (x0, x1) in [(outer.left(), frame.left()), (frame.right(), outer.right())] {
            painter.line_segment(
                [Pos2::new(x0, y), Pos2::new(x1, y)],
                Stroke::new(1.0, premul(ink.line, 0.35 * opacity)),
            );
        }
    }
}

/// The title block in the bottom-right corner: the deck's title and the
/// sheet number, around the slide counter the editorial layouts draw.
pub(super) fn title_block(
    painter: &egui::Painter,
    theme: &Theme,
    rect: Rect,
    scale: f32,
    ink: &Ink,
    stage: &Stage,
    opacity: f32,
) {
    let frame = rect.shrink(22.0 * scale);
    let h = 58.0 * scale;
    let w = 540.0 * scale;
    let block = Rect::from_min_max(
        Pos2::new(frame.right() - w, frame.bottom() - h),
        frame.right_bottom(),
    );
    painter.rect_filled(block, 0.0, premul(ink.paper, 0.92 * opacity));
    let line = Stroke::new(1.4 * scale, premul(ink.line, 0.55 * opacity));
    painter.rect_stroke(block, 0.0, line, egui::StrokeKind::Middle);
    // the counter's cell on the right, a scale cell, the title on the left
    let counter_w = 150.0 * scale;
    let scale_w = 110.0 * scale;
    let x1 = block.right() - counter_w;
    let x0 = x1 - scale_w;
    for x in [x0, x1] {
        painter.line_segment(
            [Pos2::new(x, block.top()), Pos2::new(x, block.bottom())],
            line,
        );
    }
    let label = egui::FontId::new(11.0 * scale, theme.mono_family());
    let value = egui::FontId::new(17.0 * scale, theme.mono_family());
    let pad = 12.0 * scale;
    let label_color = premul(ink.muted, opacity);
    let value_color = premul(ink.text, opacity);
    for (x, text) in [(block.left(), "TITLE"), (x0, "SCALE"), (x1, "SHEET")] {
        painter.text(
            Pos2::new(x + pad, block.top() + 7.0 * scale),
            egui::Align2::LEFT_TOP,
            text,
            label.clone(),
            label_color,
        );
    }
    let title = stage.deck_title.unwrap_or("Untitled").to_uppercase();
    let max_chars = ((x0 - block.left() - 2.0 * pad) / (10.4 * scale)) as usize;
    let title = if title.chars().count() > max_chars {
        let cut: String = title.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{cut}…")
    } else {
        title
    };
    let base = block.bottom() - 9.0 * scale;
    painter.text(
        Pos2::new(block.left() + pad, base),
        egui::Align2::LEFT_BOTTOM,
        title,
        value.clone(),
        value_color,
    );
    painter.text(
        Pos2::new(x0 + pad, base),
        egui::Align2::LEFT_BOTTOM,
        "1 : 1",
        value.clone(),
        value_color,
    );
    painter.text(
        Pos2::new(x1 + pad, base),
        egui::Align2::LEFT_BOTTOM,
        format!("{:02} / {:02}", stage.index + 1, stage.count),
        value,
        value_color,
    );
}

/// Dimension lines ruled under and beside the finished drawing: extension
/// lines from its corners, arrowheads and end ticks, growing from the
/// middle as `t` runs 0..1.
pub(super) fn dimensions(
    painter: &egui::Painter,
    b: Rect,
    scale: f32,
    ink: &Ink,
    t: f32,
    opacity: f32,
) {
    if t <= 0.0 {
        return;
    }
    let k = t.clamp(0.0, 1.0);
    let k = 1.0 - (1.0 - k).powi(3);
    let color = premul(ink.line, 0.6 * opacity);
    let thin = Stroke::new(1.0 * scale.max(0.6), color);
    let gap = 26.0 * scale;
    let arrow = 10.0 * scale;
    // under the drawing
    let y = b.bottom() + gap;
    let (cx, half) = (b.center().x, b.width() / 2.0 * k);
    let (l, r) = (cx - half, cx + half);
    painter.line_segment([Pos2::new(l, y), Pos2::new(r, y)], thin);
    for (x, dir) in [(l, 1.0), (r, -1.0)] {
        head(painter, Pos2::new(x, y), egui::vec2(dir, 0.0), arrow, color);
    }
    if k > 0.98 {
        for x in [b.left(), b.right()] {
            painter.line_segment(
                [
                    Pos2::new(x, b.bottom() + 6.0 * scale),
                    Pos2::new(x, y + 8.0 * scale),
                ],
                thin,
            );
        }
    }
    // beside it, on the right
    let x = b.right() + gap;
    let (cy, half) = (b.center().y, b.height() / 2.0 * k);
    let (t0, t1) = (cy - half, cy + half);
    painter.line_segment([Pos2::new(x, t0), Pos2::new(x, t1)], thin);
    for (y, dir) in [(t0, 1.0), (t1, -1.0)] {
        head(painter, Pos2::new(x, y), egui::vec2(0.0, dir), arrow, color);
    }
    if k > 0.98 {
        for y in [b.top(), b.bottom()] {
            painter.line_segment(
                [
                    Pos2::new(b.right() + 6.0 * scale, y),
                    Pos2::new(x + 8.0 * scale, y),
                ],
                thin,
            );
        }
    }
}

/// A slim filled arrowhead at `p`, pointing along `dir`.
pub(super) fn head(painter: &egui::Painter, p: Pos2, dir: egui::Vec2, len: f32, color: Color32) {
    let n = dir.rot90();
    painter.add(egui::Shape::convex_polygon(
        vec![
            p,
            p + dir * len + n * len * 0.28,
            p + dir * len - n * len * 0.28,
        ],
        color,
        Stroke::NONE,
    ));
}

/// The drafting machine: a horizontal and a vertical rule through the pen,
/// across the drawing's box, and the pen's glint.
pub(super) fn crosshair(
    painter: &egui::Painter,
    texture: egui::TextureId,
    b: Rect,
    tip: Pos2,
    scale: f32,
    ink: &Ink,
    opacity: f32,
) {
    let c = premul(ink.faint, 0.9 * opacity);
    let reach = 30.0 * scale;
    painter.line_segment(
        [
            Pos2::new(b.left() - reach, tip.y),
            Pos2::new(b.right() + reach, tip.y),
        ],
        Stroke::new(1.0, c),
    );
    painter.line_segment(
        [
            Pos2::new(tip.x, b.top() - reach),
            Pos2::new(tip.x, b.bottom() + reach),
        ],
        Stroke::new(1.0, c),
    );
    pen_tip(painter, texture, tip, scale, ink, opacity);
}

pub(super) fn pen_tip(
    painter: &egui::Painter,
    texture: egui::TextureId,
    tip: Pos2,
    scale: f32,
    ink: &Ink,
    opacity: f32,
) {
    let mut mesh = egui::Mesh::with_texture(texture);
    let glow = 22.0 * scale;
    mesh.add_rect_with_uv(
        Rect::from_center_size(tip, egui::vec2(glow * 2.0, glow * 2.0)),
        SPRITE_GLOW,
        additive(ink.glint, 0.45 * opacity),
    );
    let core = 4.0 * scale;
    mesh.add_rect_with_uv(
        Rect::from_center_size(tip, egui::vec2(core * 2.0, core * 2.0)),
        SPRITE_CORE,
        premul(ink.glint, opacity),
    );
    painter.add(egui::Shape::mesh(mesh));
    painter.circle_stroke(
        tip,
        9.0 * scale,
        Stroke::new(1.0, premul(ink.line, 0.5 * opacity)),
    );
}

/// Pen strokes as far as the pen has come: a faint construction pass that
/// runs ahead, and the inked line with a little bleed into the paper.
pub(super) fn pen_lines(
    painter: &egui::Painter,
    pic: &Picture,
    now: f32,
    rect: Rect,
    scale: f32,
    ink: &Ink,
    opacity: f32,
) {
    if opacity <= 0.0 || pic.points.len() < 2 {
        return;
    }
    let t = now - pic.born;
    let ghost_t = t * 2.2;
    let w = 2.2 * scale * pic.weight.max(0.6);
    let mut ghost = Vec::new();
    let mut inked = Vec::new();
    let mut bleed = Vec::new();
    for i in 1..pic.points.len() {
        if !pic.pen[i] {
            continue;
        }
        let a = to_screen(pic.points[i - 1], rect);
        let b = to_screen(pic.points[i], rect);
        if pic.at[i - 1] <= ghost_t {
            ghost.push(egui::Shape::line_segment(
                [a, b],
                Stroke::new(1.0, premul(ink.faint, 0.9 * opacity)),
            ));
        }
        if pic.at[i - 1] > t {
            continue;
        }
        let mut b = b;
        if pic.at[i] > t {
            let f = (t - pic.at[i - 1]) / (pic.at[i] - pic.at[i - 1]).max(1e-4);
            b = a + (b - a) * f.clamp(0.0, 1.0);
        }
        bleed.push(egui::Shape::line_segment(
            [a, b],
            Stroke::new(w * 2.6, premul(ink.line, 0.10 * opacity * pic.weight)),
        ));
        inked.push(egui::Shape::line_segment(
            [a, b],
            Stroke::new(w, premul(ink.line, 0.92 * opacity * pic.weight.max(0.4))),
        ));
    }
    painter.extend(ghost);
    painter.extend(bleed);
    painter.extend(inked);
}
