//! Drawing the board: the housing, the flaps with their hinge, characters
//! cut from the font atlas, and the fold of a turning flap. A turning cell
//! shows the next character's top half and the current one's bottom half,
//! with one flap falling in between: first the current top half folds down
//! over the hinge, then the next bottom half unfolds from it. Every piece is
//! a quad in one mesh, so a whole board costs a few thousand vertices.

use std::collections::HashMap;

use mdeck_sdk::paint::{Align2, Color, Font, Mesh, Painter, Pos2, Rect, Vec2};
use mdeck_sdk::tokens::Tokens;

use super::flaps::Flaps;
use super::layout::{COLS, Cell, ROWS, Style};
use super::wheel::SOLID;

/// The atlas's white texel, for quads without a texture.
pub(super) const WHITE: Rect = Rect {
    min: Pos2::ZERO,
    max: Pos2::ZERO,
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
        let avail = Vec2::new(rect.width() - 2.0 * side, rect.height() - top - bottom);
        let by_width = avail.x / (COLS as f32 * (1.0 + GAP_X) - GAP_X);
        let by_height = avail.y / (ROWS as f32 * (1.0 + GAP_Y) - GAP_Y) * ASPECT;
        let w = by_width.min(by_height).max(1.0);
        let h = w / ASPECT;
        let pitch = Vec2::new(w * (1.0 + GAP_X), h * (1.0 + GAP_Y));
        let size = Vec2::new(
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
            cell: Vec2::new(w, h),
            pitch,
        }
    }

    pub fn cell_rect(&self, col: usize, row: usize) -> Rect {
        Rect::from_min_size(
            self.origin + Vec2::new(col as f32 * self.pitch.x, row as f32 * self.pitch.y),
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
    flap_top: Color,
    flap_bottom: Color,
    housing: Color,
    housing_edge: Color,
    pub(super) gap: Color,
    pub(super) pin: Color,
    glyph: [Color; 6],
    solid_on: Color,
    solid_off: Color,
}

impl Palette {
    pub fn of(t: &Tokens) -> Self {
        let flap = t.code_background;
        let light = t.light;
        let housing = if light {
            mix(t.background, Color::BLACK, 0.08)
        } else {
            mix(t.background, Color::BLACK, 0.35)
        };
        Palette {
            flap_top: mix(flap, Color::WHITE, if light { 0.25 } else { 0.045 }),
            flap_bottom: mix(flap, Color::BLACK, if light { 0.02 } else { 0.10 }),
            housing,
            housing_edge: mix(housing, t.rule, 0.8),
            gap: mix(housing, Color::BLACK, 0.55),
            pin: mix(flap, t.muted, 0.45),
            glyph: [
                t.heading,
                t.accent,
                t.accent_soft,
                t.secondary,
                t.muted,
                t.accent,
            ],
            solid_on: t.accent,
            solid_off: mix(flap, t.muted, 0.28),
        }
    }

    pub(super) fn glyph_color(&self, style: Style) -> Color {
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
    pub(super) fn face(&self, cell: Cell, top: bool) -> Color {
        if cell.ch == SOLID {
            let c = if cell.style == Style::Dim {
                self.solid_off
            } else {
                self.solid_on
            };
            return if top { mix(c, Color::WHITE, 0.08) } else { c };
        }
        if top { self.flap_top } else { self.flap_bottom }
    }
}

/// Characters as quads cut from the font atlas, relative to a cell's
/// centre, looked up once per frame (the atlas may be rebuilt between).
pub struct Glyphs {
    font: Font,
    map: HashMap<char, Option<(Rect, Rect)>>,
}

impl Glyphs {
    pub fn new(cell: Vec2) -> Self {
        Glyphs {
            font: Font::display(cell.y * GLYPH),
            map: HashMap::new(),
        }
    }

    /// A mesh on the font atlas to draw the board into.
    pub(super) fn mesh(&self, painter: &Painter) -> Mesh {
        painter.glyph_mesh(Pos2::ZERO, "", self.font, Color::WHITE)
    }

    /// The character's quad (relative to the cell centre) and its uv rect.
    pub(super) fn get(&mut self, painter: &Painter, ch: char) -> Option<(Rect, Rect)> {
        if ch == ' ' || ch == SOLID {
            return None;
        }
        if let Some(hit) = self.map.get(&ch) {
            return *hit;
        }
        let text = ch.to_string();
        let size = painter.text_size(&text, self.font);
        // optical centring: capitals sit a touch low in the line box
        let origin = Pos2::new(-size.x / 2.0, -size.y / 2.0 - self.font.size * 0.04);
        let mesh = painter.glyph_mesh(origin, &text, self.font, Color::WHITE);
        let quad = (mesh.vertices.len() >= 4).then(|| {
            let v = &mesh.vertices;
            (
                Rect::from_min_max(v[0].pos, v[2].pos),
                Rect::from_min_max(v[0].uv, v[2].uv),
            )
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
/// labels under it. `opacity` fades what is drawn (on top of the painter's
/// own opacity).
pub fn paint(
    painter: &Painter,
    geo: &Geometry,
    tokens: &Tokens,
    scene: &Scene,
    opacity: f32,
    scale: f32,
) {
    let pal = Palette::of(tokens);
    housing(painter, geo, &pal, opacity, scale);

    let glyphs = Glyphs::new(geo.cell);
    let mesh = glyphs.mesh(painter);
    let mut flaps = Flaps {
        painter,
        cell_h: geo.cell.y,
        pal: &pal,
        glyphs,
        mesh,
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
    painter.mesh(flaps.mesh);

    if let Some(labels) = &scene.labels {
        draw_labels(painter, geo, tokens, labels, opacity, scale);
    }
}

/// The housing: a dark frame with a hairline edge and a soft top light.
fn housing(painter: &Painter, geo: &Geometry, pal: &Palette, opacity: f32, scale: f32) {
    let fade = |c: Color| c.gamma_multiply(opacity);
    let round = geo.cell.x * 0.22;
    painter.rect_filled(
        geo.housing.expand(2.0 * scale),
        round + 2.0,
        fade(pal.housing_edge),
    );
    painter.rect_filled(geo.housing, round, fade(pal.housing));
    let h = geo.housing;
    let bottom = h.top() + geo.cell.y * 0.35;
    let sheen = fade(Color::from_rgba_premultiplied(10, 10, 10, 10));
    let mut m = Mesh::default();
    let a = m.vertex(h.min, Pos2::ZERO, sheen);
    let b = m.vertex(Pos2::new(h.right(), h.top()), Pos2::ZERO, sheen);
    let c = m.vertex(Pos2::new(h.left(), bottom), Pos2::ZERO, Color::TRANSPARENT);
    let d = m.vertex(Pos2::new(h.right(), bottom), Pos2::ZERO, Color::TRANSPARENT);
    m.triangle(a, b, c);
    m.triangle(b, d, c);
    painter.mesh(m);
}

/// The deck's title and the slide counter, spaced out on the frame.
fn draw_labels(
    painter: &Painter,
    geo: &Geometry,
    tokens: &Tokens,
    labels: &Labels,
    opacity: f32,
    scale: f32,
) {
    let size = 15.0 * scale;
    let y = geo.housing.bottom() + 22.0 * scale;
    let font = Font::mono(size);
    let spacing = size * 0.22;
    let color = tokens.muted.gamma_multiply(opacity);
    for (text, right) in [(labels.left, false), (labels.right, true)] {
        if text.is_empty() {
            continue;
        }
        // spaced out: each character set on its own, the spacing between
        let chars: Vec<(String, f32)> = text
            .to_uppercase()
            .chars()
            .map(|c| {
                let s = c.to_string();
                let w = painter.text_size(&s, font).x;
                (s, w)
            })
            .collect();
        let width: f32 = chars.iter().map(|(_, w)| w + spacing).sum::<f32>() - spacing;
        let mut x = if right {
            geo.housing.right() - width
        } else {
            geo.housing.left()
        };
        for (s, w) in chars {
            painter.text(Pos2::new(x, y), Align2::LEFT_TOP, &s, font, color);
            x += w + spacing;
        }
    }
}

/// `a` toward `b` by `t` (0..1), alpha included.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::from_rgba_premultiplied(
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
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0));
        let g = Geometry::new(rect, 1.0);
        assert!(
            rect.contains(g.housing.min) && rect.contains(g.housing.max),
            "{:?}",
            g.housing
        );
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
