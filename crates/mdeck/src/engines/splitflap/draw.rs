//! Drawing the board: the housing, the flaps with their hinge, characters
//! cut from the font atlas, and the fold of a turning flap. A turning cell
//! shows the next character's top half and the current one's bottom half,
//! with one flap falling in between: first the current top half folds down
//! over the hinge, then the next bottom half unfolds from it. Every piece is
//! a quad in one mesh, so a whole board costs a few thousand vertices.

use std::collections::HashMap;

use eframe::egui::{self, Color32, Pos2, Rect, Vec2, epaint};

use super::layout::{COLS, Cell, ROWS, SOLID, Style};
use crate::theme::Theme;

/// The atlas's white texel, for quads without a texture.
const WHITE: Rect = Rect {
    min: epaint::WHITE_UV,
    max: epaint::WHITE_UV,
};

/// A flap is this wide for its height.
const ASPECT: f32 = 0.70;
/// Gaps between flaps, as a share of the flap's width and height.
const GAP_X: f32 = 0.10;
const GAP_Y: f32 = 0.10;
/// Characters stand this share of the flap's height.
const GLYPH: f32 = 0.64;
/// Board faces are heavy: each character is drawn a second time this share
/// of its size to the right.
const EMBOLDEN: f32 = 0.028;

/// Where the board and its cells sit on the slide.
#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    /// The board's frame around the flaps.
    pub housing: Rect,
    origin: Pos2,
    pub cell: Vec2,
    pitch: Vec2,
}

impl Geometry {
    pub fn new(rect: Rect, scale: f32) -> Self {
        let (side, top, bottom) = (70.0 * scale, 64.0 * scale, 104.0 * scale);
        let avail = egui::vec2(rect.width() - 2.0 * side, rect.height() - top - bottom);
        let by_width = avail.x / (COLS as f32 * (1.0 + GAP_X) - GAP_X);
        let by_height = avail.y / (ROWS as f32 * (1.0 + GAP_Y) - GAP_Y) * ASPECT;
        let w = by_width.min(by_height).max(1.0);
        let h = w / ASPECT;
        let pitch = egui::vec2(w * (1.0 + GAP_X), h * (1.0 + GAP_Y));
        let size = egui::vec2(
            pitch.x * COLS as f32 - w * GAP_X,
            pitch.y * ROWS as f32 - h * GAP_Y,
        );
        let origin = Pos2::new(
            rect.center().x - size.x / 2.0,
            rect.top() + top + (avail.y - size.y) / 2.0,
        );
        Geometry {
            housing: Rect::from_min_size(origin, size).expand(w * 0.42),
            origin,
            cell: egui::vec2(w, h),
            pitch,
        }
    }

    pub fn cell_rect(&self, col: usize, row: usize) -> Rect {
        Rect::from_min_size(
            self.origin + egui::vec2(col as f32 * self.pitch.x, row as f32 * self.pitch.y),
            self.cell,
        )
    }

    /// The rect covering columns `c0..c1` and rows `r0..r1`.
    pub fn area(&self, c0: usize, r0: usize, c1: usize, r1: usize) -> Rect {
        self.cell_rect(c0, r0)
            .union(self.cell_rect(c1.saturating_sub(1), r1.saturating_sub(1)))
    }
}

/// The board's colours, from the theme.
pub struct Palette {
    flap_top: Color32,
    flap_bottom: Color32,
    housing: Color32,
    housing_edge: Color32,
    gap: Color32,
    pin: Color32,
    glyph: [Color32; 6],
    solid_on: Color32,
    solid_off: Color32,
}

impl Palette {
    pub fn of(theme: &Theme) -> Self {
        let flap = theme.code_background;
        let light = theme.is_light();
        let housing = if light {
            mix(theme.background, Color32::BLACK, 0.08)
        } else {
            mix(theme.background, Color32::BLACK, 0.35)
        };
        Palette {
            flap_top: mix(flap, Color32::WHITE, if light { 0.25 } else { 0.045 }),
            flap_bottom: mix(flap, Color32::BLACK, if light { 0.02 } else { 0.10 }),
            housing,
            housing_edge: mix(housing, theme.rule, 0.8),
            gap: mix(housing, Color32::BLACK, 0.55),
            pin: mix(flap, theme.muted, 0.45),
            glyph: [
                theme.heading_color,
                theme.accent,
                theme.accent_soft,
                theme.secondary,
                theme.muted,
                theme.accent,
            ],
            solid_on: theme.accent,
            solid_off: mix(flap, theme.muted, 0.28),
        }
    }

    fn glyph_color(&self, style: Style) -> Color32 {
        self.glyph[match style {
            Style::Normal => 0,
            Style::Heading => 1,
            Style::Strong => 2,
            Style::Code => 3,
            Style::Dim => 4,
            Style::Accent => 5,
        }]
    }

    /// The flap's own colour: a solid flap is coloured through.
    fn face(&self, cell: Cell, top: bool) -> Color32 {
        if cell.ch == SOLID {
            let c = if cell.style == Style::Dim {
                self.solid_off
            } else {
                self.solid_on
            };
            return if top { mix(c, Color32::WHITE, 0.08) } else { c };
        }
        if top { self.flap_top } else { self.flap_bottom }
    }
}

/// Characters as quads cut from egui's font atlas, relative to a cell's
/// centre, looked up once per frame (the atlas may be rebuilt between).
pub struct Glyphs {
    font: egui::FontId,
    map: HashMap<char, Option<(Rect, Rect)>>,
}

impl Glyphs {
    pub fn new(theme: &Theme, cell: Vec2) -> Self {
        Glyphs {
            font: egui::FontId::new(cell.y * GLYPH, theme.display_family()),
            map: HashMap::new(),
        }
    }

    /// The character's quad (relative to the cell centre) and its uv rect.
    fn get(&mut self, ui: &egui::Ui, ch: char) -> Option<(Rect, Rect)> {
        if ch == ' ' || ch == SOLID {
            return None;
        }
        if let Some(hit) = self.map.get(&ch) {
            return *hit;
        }
        let font = self.font.clone();
        let (galley, atlas) = ui.fonts_mut(|f| {
            (
                f.layout_no_wrap(ch.to_string(), font, Color32::WHITE),
                f.font_image_size(),
            )
        });
        let quad = galley.rows.first().and_then(|row| {
            let g = row.row.glyphs.first()?;
            if g.uv_rect.max[0] <= g.uv_rect.min[0] {
                return None;
            }
            let min = row.pos + g.pos.to_vec2() + g.uv_rect.offset;
            let rect = Rect::from_min_size(min, g.uv_rect.size)
                .translate(-galley.size() / 2.0)
                // optical centring: capitals sit a touch low in the line box
                .translate(egui::vec2(0.0, -self.font.size * 0.04));
            let (aw, ah) = (atlas[0] as f32, atlas[1] as f32);
            let uv = Rect::from_min_max(
                Pos2::new(g.uv_rect.min[0] as f32 / aw, g.uv_rect.min[1] as f32 / ah),
                Pos2::new(g.uv_rect.max[0] as f32 / aw, g.uv_rect.max[1] as f32 / ah),
            );
            Some((rect, uv))
        });
        self.map.insert(ch, quad);
        quad
    }
}

/// What one cell shows this frame: `from` turning into `to`, `t` through
/// the turn (0: `from` at rest).
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub from: Cell,
    pub to: Cell,
    pub t: f32,
}

/// The labels printed on the frame under the board.
pub struct Labels<'a> {
    pub left: &'a str,
    pub right: &'a str,
}

/// Paint the board: housing, every cell (turning ones mid-fold), and the
/// labels under it. `views` holds one entry per cell, row by row; cells
/// listed in `hidden` (under an image panel) keep only their housing.
#[allow(clippy::too_many_arguments)]
pub fn paint(
    ui: &egui::Ui,
    geo: &Geometry,
    theme: &Theme,
    views: &[View],
    hidden: Option<Rect>,
    labels: Option<Labels>,
    opacity: f32,
    scale: f32,
) {
    let pal = Palette::of(theme);
    let painter = ui.painter();
    let fade = |c: Color32| c.gamma_multiply(opacity);

    // the housing: a dark frame with a hairline edge and a soft top light
    let round = geo.cell.x * 0.22;
    painter.rect_filled(
        geo.housing.expand(2.0 * scale),
        round + 2.0,
        fade(pal.housing_edge),
    );
    painter.rect_filled(geo.housing, round, fade(pal.housing));
    let sheen = Rect::from_min_max(
        geo.housing.left_top(),
        Pos2::new(geo.housing.right(), geo.housing.top() + geo.cell.y * 0.35),
    );
    let mut m = egui::Mesh::default();
    m.colored_vertex(sheen.left_top(), fade(Color32::from_white_alpha(10)));
    m.colored_vertex(sheen.right_top(), fade(Color32::from_white_alpha(10)));
    m.colored_vertex(sheen.left_bottom(), Color32::TRANSPARENT);
    m.colored_vertex(sheen.right_bottom(), Color32::TRANSPARENT);
    m.add_triangle(0, 1, 2);
    m.add_triangle(1, 3, 2);
    painter.add(egui::Shape::mesh(m));

    let mut glyphs = Glyphs::new(theme, geo.cell);
    let mut mesh = egui::Mesh::with_texture(egui::TextureId::default());
    let r = geo.cell.x * 0.10;
    let hinge = (geo.cell.y * 0.035).max(1.2);
    for (i, view) in views.iter().enumerate() {
        let (col, row) = (i % COLS, i / COLS);
        let cell = geo.cell_rect(col, row);
        if hidden.is_some_and(|h| h.intersects(cell)) {
            continue;
        }
        let mid = cell.center().y;
        let top = Rect::from_min_max(cell.left_top(), Pos2::new(cell.right(), mid - hinge / 2.0));
        let bottom = Rect::from_min_max(
            Pos2::new(cell.left(), mid + hinge / 2.0),
            cell.right_bottom(),
        );
        let turning = view.t > 0.0;
        // static halves: the next character's top, the current one's bottom
        let upper = if turning { view.to } else { view.from };
        let lower = view.from;
        rounded_half(&mut mesh, top, r, true, fade(pal.face(upper, true)));
        rounded_half(&mut mesh, bottom, r, false, fade(pal.face(lower, false)));
        glyph_half(
            &mut mesh,
            ui,
            &mut glyphs,
            &pal,
            cell,
            top,
            upper,
            opacity,
            None,
        );
        glyph_half(
            &mut mesh,
            ui,
            &mut glyphs,
            &pal,
            cell,
            bottom,
            lower,
            opacity,
            None,
        );
        // the hinge: a dark gap with a pin at each end
        quad(
            &mut mesh,
            Rect::from_min_max(
                Pos2::new(cell.left(), mid - hinge / 2.0),
                Pos2::new(cell.right(), mid + hinge / 2.0),
            ),
            fade(pal.gap),
        );
        let pin = egui::vec2(geo.cell.x * 0.07, hinge * 2.2);
        for x in [cell.left() + pin.x * 0.2, cell.right() - pin.x * 1.2] {
            quad(
                &mut mesh,
                Rect::from_min_size(Pos2::new(x, mid - pin.y / 2.0), pin),
                fade(pal.pin),
            );
        }
        if turning {
            // the falling flap: the current top folds down, then the next
            // bottom unfolds; it darkens as it turns edge-on
            let angle = view.t * std::f32::consts::PI;
            let (half, face, first) = if view.t < 0.5 {
                (top, view.from, true)
            } else {
                (bottom, view.to, false)
            };
            let fold = angle.cos().abs();
            let widen = 0.10 * angle.sin();
            let shade = 1.0 - 0.45 * angle.sin();
            let warp = |p: Pos2| -> Pos2 {
                let d = p.y - mid;
                let reach = (d.abs() / (geo.cell.y / 2.0)).min(1.0);
                Pos2::new(
                    cell.center().x + (p.x - cell.center().x) * (1.0 + widen * reach),
                    mid + d * fold,
                )
            };
            let base = pal.face(face, first);
            let tint = fade(mix(Color32::BLACK, base, shade));
            warped_quad(&mut mesh, half, WHITE, tint, &warp);
            glyph_half(
                &mut mesh,
                ui,
                &mut glyphs,
                &pal,
                cell,
                half,
                face,
                opacity * shade,
                Some(&warp),
            );
        }
    }
    painter.add(egui::Shape::mesh(mesh));

    if let Some(labels) = labels {
        let size = 15.0 * scale;
        let y = geo.housing.bottom() + 22.0 * scale;
        let font = egui::FontId::new(size, theme.mono_family());
        for (text, right) in [(labels.left, false), (labels.right, true)] {
            if text.is_empty() {
                continue;
            }
            let mut job = egui::text::LayoutJob::default();
            job.append(
                &text.to_uppercase(),
                0.0,
                egui::text::TextFormat {
                    font_id: font.clone(),
                    color: fade(theme.muted),
                    extra_letter_spacing: size * 0.22,
                    ..Default::default()
                },
            );
            let galley = painter.layout_job(job);
            let x = if right {
                geo.housing.right() - galley.size().x
            } else {
                geo.housing.left()
            };
            painter.galley(Pos2::new(x, y), galley, theme.muted);
        }
    }
}

/// The part of a cell's character inside `half`, optionally warped (a
/// falling flap). Solid flaps and blanks draw nothing here.
#[allow(clippy::too_many_arguments)]
fn glyph_half(
    mesh: &mut egui::Mesh,
    ui: &egui::Ui,
    glyphs: &mut Glyphs,
    pal: &Palette,
    cell: Rect,
    half: Rect,
    c: Cell,
    opacity: f32,
    warp: Option<&dyn Fn(Pos2) -> Pos2>,
) {
    // a list marker is a coloured bar, like a platform indicator, not the
    // font's small bullet
    let (rel, uv) = if c.ch == '•' {
        let size = egui::vec2(cell.width() * 0.26, cell.height() * 0.56);
        (Rect::from_center_size(Pos2::ZERO, size), WHITE)
    } else {
        match glyphs.get(ui, c.ch) {
            Some(q) => q,
            None => return,
        }
    };
    let quad_rect = rel.translate(cell.center().to_vec2());
    let clipped = quad_rect.intersect(half);
    if clipped.height() <= 0.0 || clipped.width() <= 0.0 {
        return;
    }
    // uv for the clipped part
    let fy = |y: f32| (y - quad_rect.top()) / quad_rect.height();
    let fx = |x: f32| (x - quad_rect.left()) / quad_rect.width();
    let uv_part = Rect::from_min_max(
        Pos2::new(
            uv.left() + uv.width() * fx(clipped.left()),
            uv.top() + uv.height() * fy(clipped.top()),
        ),
        Pos2::new(
            uv.left() + uv.width() * fx(clipped.right()),
            uv.top() + uv.height() * fy(clipped.bottom()),
        ),
    );
    let color = pal
        .glyph_color(c.style)
        .gamma_multiply(opacity.clamp(0.0, 1.0));
    let bold = if uv == WHITE {
        0.0
    } else {
        rel.height() * EMBOLDEN
    };
    for dx in [0.0, bold] {
        let part = clipped.translate(egui::vec2(dx, 0.0));
        match warp {
            Some(w) => warped_quad(mesh, part, uv_part, color, w),
            None => {
                mesh.add_rect_with_uv(part, uv_part, color);
            }
        }
        if bold == 0.0 {
            break;
        }
    }
}

fn quad(mesh: &mut egui::Mesh, rect: Rect, color: Color32) {
    mesh.add_rect_with_uv(rect, WHITE, color);
}

fn warped_quad(
    mesh: &mut egui::Mesh,
    rect: Rect,
    uv: Rect,
    color: Color32,
    warp: &dyn Fn(Pos2) -> Pos2,
) {
    let base = mesh.vertices.len() as u32;
    for (p, t) in [
        (rect.left_top(), uv.left_top()),
        (rect.right_top(), uv.right_top()),
        (rect.left_bottom(), uv.left_bottom()),
        (rect.right_bottom(), uv.right_bottom()),
    ] {
        mesh.vertices.push(epaint::Vertex {
            pos: warp(p),
            uv: t,
            color,
        });
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 1, base + 3, base + 2);
}

/// Half a flap with its two outer corners rounded, lit from above: a fan of
/// triangles from the centre so the corners stay smooth.
fn rounded_half(mesh: &mut egui::Mesh, rect: Rect, r: f32, top: bool, color: Color32) {
    let r = r.min(rect.height() * 0.5).min(rect.width() * 0.5);
    let light = if top {
        mix(color, Color32::WHITE, 0.035)
    } else {
        color
    };
    let dark = if top {
        color
    } else {
        mix(color, Color32::BLACK, 0.18)
    };
    let shade = |p: Pos2| -> Color32 {
        let t = ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
        mix(light, dark, t)
    };
    let mut outline: Vec<Pos2> = Vec::new();
    let arc = |outline: &mut Vec<Pos2>, c: Pos2, a0: f32| {
        for k in 0..=5 {
            let a = a0 + k as f32 / 5.0 * std::f32::consts::FRAC_PI_2;
            outline.push(Pos2::new(c.x + r * a.cos(), c.y + r * a.sin()));
        }
    };
    use std::f32::consts::PI;
    if top {
        arc(&mut outline, Pos2::new(rect.left() + r, rect.top() + r), PI);
        arc(
            &mut outline,
            Pos2::new(rect.right() - r, rect.top() + r),
            1.5 * PI,
        );
        outline.push(rect.right_bottom());
        outline.push(rect.left_bottom());
    } else {
        outline.push(rect.left_top());
        outline.push(rect.right_top());
        arc(
            &mut outline,
            Pos2::new(rect.right() - r, rect.bottom() - r),
            0.0,
        );
        arc(
            &mut outline,
            Pos2::new(rect.left() + r, rect.bottom() - r),
            0.5 * PI,
        );
    }
    let base = mesh.vertices.len() as u32;
    let c = rect.center();
    mesh.vertices.push(epaint::Vertex {
        pos: c,
        uv: epaint::WHITE_UV,
        color: shade(c),
    });
    for p in &outline {
        mesh.vertices.push(epaint::Vertex {
            pos: *p,
            uv: epaint::WHITE_UV,
            color: shade(*p),
        });
    }
    let n = outline.len() as u32;
    for k in 0..n {
        mesh.add_triangle(base, base + 1 + k, base + 1 + (k + 1) % n);
    }
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_board_fits_the_slide_with_room_for_labels() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        let g = Geometry::new(rect, 1.0);
        assert!(rect.contains_rect(g.housing), "{:?}", g.housing);
        assert!(
            g.housing.bottom() < 1080.0 - 60.0,
            "labels fit under the board"
        );
        let last = g.cell_rect(COLS - 1, ROWS - 1);
        assert!((g.cell_rect(0, 0).left() - (1920.0 - last.right())).abs() < 0.5);
        assert!(
            g.cell.y > 60.0,
            "flaps are big enough to read: {:?}",
            g.cell
        );
        assert!((g.cell.x / g.cell.y - ASPECT).abs() < 1e-3);
    }
}
