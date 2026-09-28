//! Drawing the board: the housing, the flaps with their hinge, characters
//! cut from the font atlas, and the fold of a turning flap. A turning cell
//! shows the next character's top half and the current one's bottom half,
//! with one flap falling in between: first the current top half folds down
//! over the hinge, then the next bottom half unfolds from it. Every piece is
//! a quad in one mesh, so a whole board costs a few thousand vertices.

use std::collections::HashMap;

use eframe::egui::{self, Color32, Pos2, Rect, Vec2, epaint};

use super::flaps::Flaps;
use super::layout::{COLS, Cell, ROWS, Style};
use super::wheel::SOLID;
use crate::theme::Theme;

/// The atlas's white texel, for quads without a texture.
pub(super) const WHITE: Rect = Rect {
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
pub(super) const EMBOLDEN: f32 = 0.028;

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
    pub(super) gap: Color32,
    pub(super) pin: Color32,
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

    pub(super) fn glyph_color(&self, style: Style) -> Color32 {
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
    pub(super) fn face(&self, cell: Cell, top: bool) -> Color32 {
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
    pub(super) fn get(&mut self, ui: &egui::Ui, ch: char) -> Option<(Rect, Rect)> {
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

/// What the board shows: `views` holds one entry per cell, row by row;
/// cells under `hidden` (an image panel) keep only their housing; `labels`
/// go on the frame under it.
pub struct Scene<'a> {
    pub views: &'a [View],
    pub hidden: Option<Rect>,
    pub labels: Option<Labels<'a>>,
}

/// Paint the board: housing, every cell (turning ones mid-fold), and the
/// labels under it.
pub fn paint(
    ui: &egui::Ui,
    geo: &Geometry,
    theme: &Theme,
    scene: &Scene,
    opacity: f32,
    scale: f32,
) {
    let pal = Palette::of(theme);
    housing(ui.painter(), geo, &pal, opacity, scale);

    let mut flaps = Flaps {
        ui,
        cell_h: geo.cell.y,
        pal: &pal,
        glyphs: Glyphs::new(theme, geo.cell),
        mesh: egui::Mesh::with_texture(egui::TextureId::default()),
        opacity,
        r: geo.cell.x * 0.10,
        hinge: (geo.cell.y * 0.035).max(1.2),
        pin: geo.cell.x * 0.07,
    };
    for (i, view) in scene.views.iter().enumerate() {
        let cell = geo.cell_rect(i % COLS, i / COLS);
        if scene.hidden.is_some_and(|h| h.intersects(cell)) {
            continue;
        }
        flaps.cell(cell, view);
    }
    ui.painter().add(egui::Shape::mesh(flaps.mesh));

    if let Some(labels) = &scene.labels {
        draw_labels(ui.painter(), geo, theme, labels, opacity, scale);
    }
}

/// The housing: a dark frame with a hairline edge and a soft top light.
fn housing(painter: &egui::Painter, geo: &Geometry, pal: &Palette, opacity: f32, scale: f32) {
    let fade = |c: Color32| c.gamma_multiply(opacity);
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
}

/// The deck's title and the slide counter, spaced out on the frame.
fn draw_labels(
    painter: &egui::Painter,
    geo: &Geometry,
    theme: &Theme,
    labels: &Labels,
    opacity: f32,
    scale: f32,
) {
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
                color: theme.muted.gamma_multiply(opacity),
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
