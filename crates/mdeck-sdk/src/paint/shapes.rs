//! Strokes, meshes, images, textures and fonts.

use super::color::{Color, smoothstep};
use super::geom::{Pos2, Rect, ToEgui};

/// A line's width (in points) and colour.
///
/// ```
/// use mdeck_sdk::paint::{Color, Stroke};
/// let s = Stroke::new(2.0, Color::WHITE);
/// assert_eq!(s.width, 2.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stroke {
    /// Width in points.
    pub width: f32,
    /// Colour (premultiplied).
    pub color: Color,
}

impl Stroke {
    /// No line.
    pub const NONE: Stroke = Stroke {
        width: 0.0,
        color: Color::TRANSPARENT,
    };

    /// A stroke of `width` points in `color`.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Color, Stroke};
    /// assert_eq!(Stroke::new(1.0, Color::BLACK).color, Color::BLACK);
    /// ```
    pub fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }
}

impl ToEgui for Stroke {
    type Out = egui::Stroke;
    fn eg(self) -> egui::Stroke {
        egui::Stroke::new(self.width, self.color.eg())
    }
}

/// One mesh vertex: position in points, texture coordinate (0..1) and a
/// colour that multiplies the texture.
///
/// ```
/// use mdeck_sdk::paint::{Color, Pos2, Vertex};
/// let v = Vertex { pos: Pos2::ZERO, uv: Pos2::ZERO, color: Color::WHITE };
/// assert_eq!(v.color, Color::WHITE);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    /// Position in points.
    pub pos: Pos2,
    /// Texture coordinate, 0..1 across the texture. Ignored without a texture.
    pub uv: Pos2,
    /// Colour (premultiplied); with alpha 0 it adds light.
    pub color: Color,
}

/// Triangles to draw in one call, optionally textured.
///
/// ```
/// use mdeck_sdk::paint::{Color, Mesh, Pos2, Rect, Vec2};
/// let mut m = Mesh::default();
/// m.add_rect(Rect::from_min_size(Pos2::ZERO, Vec2::splat(10.0)), Color::WHITE);
/// assert_eq!((m.vertices.len(), m.indices.len()), (4, 6));
/// ```
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    /// The vertices.
    pub vertices: Vec<Vertex>,
    /// Three indices into [`Mesh::vertices`] per triangle.
    pub indices: Vec<u32>,
    /// The texture sampled with each vertex's `uv`; `None` draws plain colour.
    pub texture: Option<Texture>,
}

impl Mesh {
    /// An empty mesh sampling `texture`.
    ///
    /// ```
    /// use mdeck_sdk::paint::Mesh;
    /// assert!(Mesh::default().is_empty());
    /// ```
    pub fn with_texture(texture: Texture) -> Self {
        Self {
            texture: Some(texture),
            ..Default::default()
        }
    }

    /// Whether there is nothing to draw.
    ///
    /// ```
    /// assert!(mdeck_sdk::paint::Mesh::default().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Append a vertex and return its index.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Color, Mesh, Pos2};
    /// let mut m = Mesh::default();
    /// assert_eq!(m.vertex(Pos2::ZERO, Pos2::ZERO, Color::WHITE), 0);
    /// ```
    pub fn vertex(&mut self, pos: Pos2, uv: Pos2, color: Color) -> u32 {
        self.vertices.push(Vertex { pos, uv, color });
        (self.vertices.len() - 1) as u32
    }

    /// Append a triangle of three existing vertices.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Color, Mesh, Pos2};
    /// let mut m = Mesh::default();
    /// for _ in 0..3 { m.vertex(Pos2::ZERO, Pos2::ZERO, Color::WHITE); }
    /// m.triangle(0, 1, 2);
    /// assert_eq!(m.indices, vec![0, 1, 2]);
    /// ```
    pub fn triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }

    /// Append a rectangle in one colour, with uvs spanning the whole texture.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Color, Mesh, Pos2, Rect, Vec2};
    /// let mut m = Mesh::default();
    /// m.add_rect(Rect::from_min_size(Pos2::ZERO, Vec2::splat(1.0)), Color::WHITE);
    /// assert_eq!(m.indices.len(), 6);
    /// ```
    pub fn add_rect(&mut self, rect: Rect, color: Color) {
        self.add_rect_uv(
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            color,
        );
    }

    /// Append a rectangle sampling the `uv` part of the texture.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Color, Mesh, Pos2, Rect, Vec2, SPRITE_GLOW};
    /// let mut m = Mesh::default();
    /// m.add_rect_uv(Rect::from_min_size(Pos2::ZERO, Vec2::splat(8.0)), SPRITE_GLOW, Color::WHITE);
    /// assert_eq!(m.vertices[0].uv, SPRITE_GLOW.min);
    /// ```
    pub fn add_rect_uv(&mut self, rect: Rect, uv: Rect, color: Color) {
        let i = self.vertex(rect.min, uv.min, color);
        self.vertex(
            Pos2::new(rect.max.x, rect.min.y),
            Pos2::new(uv.max.x, uv.min.y),
            color,
        );
        self.vertex(rect.max, uv.max, color);
        self.vertex(
            Pos2::new(rect.min.x, rect.max.y),
            Pos2::new(uv.min.x, uv.max.y),
            color,
        );
        self.triangle(i, i + 1, i + 2);
        self.triangle(i, i + 2, i + 3);
    }

    pub(crate) fn to_egui(&self) -> egui::Mesh {
        let texture_id = self
            .texture
            .as_ref()
            .map_or(egui::TextureId::default(), |t| t.handle.id());
        let mut m = egui::Mesh::with_texture(texture_id);
        m.vertices = self
            .vertices
            .iter()
            .map(|v| egui::epaint::Vertex {
                pos: v.pos.eg(),
                // The font atlas's white pixel sits at uv (0, 0) for an untextured mesh.
                uv: if self.texture.is_some() {
                    v.uv.eg()
                } else {
                    egui::epaint::WHITE_UV
                },
                color: v.color.eg(),
            })
            .collect();
        m.indices = self.indices.clone();
        m
    }
}

/// An RGBA image in memory: `size` is `[width, height]` and `pixels` holds
/// `width * height` premultiplied colours, row by row from the top.
///
/// ```
/// use mdeck_sdk::paint::{Color, ImageData};
/// let img = ImageData::filled([2, 3], Color::WHITE);
/// assert_eq!(img.pixels.len(), 6);
/// assert_eq!(img.get(1, 2), Some(Color::WHITE));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImageData {
    /// `[width, height]` in pixels.
    pub size: [usize; 2],
    /// Premultiplied pixels, row-major from the top left.
    pub pixels: Vec<Color>,
}

impl ImageData {
    /// An image of `size` filled with `color`.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Color, ImageData};
    /// assert_eq!(ImageData::filled([1, 1], Color::BLACK).pixels, vec![Color::BLACK]);
    /// ```
    pub fn filled(size: [usize; 2], color: Color) -> Self {
        Self {
            size,
            pixels: vec![color; size[0] * size[1]],
        }
    }

    /// An image from straight (not premultiplied) RGBA bytes, four per pixel.
    /// Returns `None` when the byte count does not match the size.
    ///
    /// ```
    /// use mdeck_sdk::paint::ImageData;
    /// let img = ImageData::from_rgba_unmultiplied([1, 1], &[255, 0, 0, 255]).unwrap();
    /// assert_eq!(img.pixels[0].r(), 255);
    /// assert!(ImageData::from_rgba_unmultiplied([2, 2], &[0; 4]).is_none());
    /// ```
    pub fn from_rgba_unmultiplied(size: [usize; 2], rgba: &[u8]) -> Option<Self> {
        if rgba.len() != size[0] * size[1] * 4 {
            return None;
        }
        let pixels = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| Color::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();
        Some(Self { size, pixels })
    }

    /// Width in pixels.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::ImageData::filled([4, 2], Default::default()).width(), 4);
    /// ```
    pub fn width(&self) -> usize {
        self.size[0]
    }

    /// Height in pixels.
    ///
    /// ```
    /// assert_eq!(mdeck_sdk::paint::ImageData::filled([4, 2], Default::default()).height(), 2);
    /// ```
    pub fn height(&self) -> usize {
        self.size[1]
    }

    /// The pixel at `(x, y)`, if inside.
    ///
    /// ```
    /// use mdeck_sdk::paint::ImageData;
    /// assert_eq!(ImageData::filled([1, 1], Default::default()).get(5, 0), None);
    /// ```
    pub fn get(&self, x: usize, y: usize) -> Option<Color> {
        (x < self.size[0] && y < self.size[1]).then(|| self.pixels[y * self.size[0] + x])
    }

    pub(crate) fn to_egui(&self) -> egui::ColorImage {
        egui::ColorImage::new(self.size, self.pixels.iter().map(|c| c.eg()).collect())
    }
}

/// How a texture is sampled when drawn larger or smaller than its pixels.
///
/// ```
/// use mdeck_sdk::paint::TextureFilter;
/// assert_ne!(TextureFilter::Linear, TextureFilter::Nearest);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextureFilter {
    /// Smooth interpolation (photos, sprites).
    #[default]
    Linear,
    /// Hard pixels (pixel art, LED dots, data).
    Nearest,
}

impl ToEgui for TextureFilter {
    type Out = egui::TextureOptions;
    fn eg(self) -> egui::TextureOptions {
        match self {
            TextureFilter::Linear => egui::TextureOptions::LINEAR,
            TextureFilter::Nearest => egui::TextureOptions::NEAREST,
        }
    }
}

/// A texture uploaded to the GPU (opaque). Made by
/// [`crate::paint::Painter::load_texture`]; cloning shares it, and it is
/// freed when the last clone is dropped.
///
/// ```no_run
/// # fn demo(p: &mdeck_sdk::paint::Painter) {
/// use mdeck_sdk::paint::{Color, ImageData, TextureFilter};
/// let tex = p.load_texture("dot", &ImageData::filled([4, 4], Color::WHITE), TextureFilter::Linear);
/// assert_eq!(tex.size(), [4, 4]);
/// # }
/// ```
#[derive(Clone)]
pub struct Texture {
    pub(crate) handle: TextureRef,
}

/// What a [`Texture`] points at: a texture mdeck uploaded (freed with its
/// last clone), or egui's font atlas (owned by egui).
#[derive(Clone)]
pub(crate) enum TextureRef {
    Owned(egui::TextureHandle),
    FontAtlas([usize; 2]),
}

impl TextureRef {
    pub(crate) fn id(&self) -> egui::TextureId {
        match self {
            TextureRef::Owned(h) => h.id(),
            TextureRef::FontAtlas(_) => egui::TextureId::default(),
        }
    }

    pub(crate) fn size(&self) -> [usize; 2] {
        match self {
            TextureRef::Owned(h) => h.size(),
            TextureRef::FontAtlas(size) => *size,
        }
    }

    /// Replace the pixels; the font atlas is egui's and stays as it is.
    pub(crate) fn set(&mut self, image: impl Into<egui::ImageData>, options: egui::TextureOptions) {
        if let TextureRef::Owned(h) = self {
            h.set(image, options);
        }
    }
}

impl From<egui::TextureHandle> for TextureRef {
    fn from(h: egui::TextureHandle) -> Self {
        TextureRef::Owned(h)
    }
}

impl Texture {
    /// `[width, height]` in pixels.
    ///
    /// See [`Texture`] for an example.
    pub fn size(&self) -> [usize; 2] {
        self.handle.size()
    }
}

impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("size", &self.size())
            .finish()
    }
}

/// The typographic role of a font. The theme decides which face each role
/// uses; engines and visuals ask for a role, never a face.
///
/// ```
/// use mdeck_sdk::paint::FontRole;
/// assert_eq!(FontRole::ALL.len(), 5);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FontRole {
    /// Headings and display type (titles, countdown digits).
    Display,
    /// Running text.
    Body,
    /// A lead paragraph or subtitle.
    Lead,
    /// Emphasised body text.
    Strong,
    /// Code and fixed-width text.
    Mono,
}

impl FontRole {
    /// Every role.
    pub const ALL: [FontRole; 5] = [
        FontRole::Display,
        FontRole::Body,
        FontRole::Lead,
        FontRole::Strong,
        FontRole::Mono,
    ];
}

/// A font: a role and a size in points.
///
/// ```
/// use mdeck_sdk::paint::{Font, FontRole};
/// let f = Font::new(FontRole::Display, 72.0);
/// assert_eq!(f.size, 72.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Font {
    /// Which of the theme's faces.
    pub role: FontRole,
    /// Size in points.
    pub size: f32,
}

impl Font {
    /// A font of `role` at `size` points.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Font, FontRole};
    /// assert_eq!(Font::new(FontRole::Mono, 12.0).role, FontRole::Mono);
    /// ```
    pub const fn new(role: FontRole, size: f32) -> Self {
        Self { role, size }
    }

    /// Body text at `size` points.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Font, FontRole};
    /// assert_eq!(Font::body(20.0).role, FontRole::Body);
    /// ```
    pub const fn body(size: f32) -> Self {
        Self::new(FontRole::Body, size)
    }

    /// Display type at `size` points.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Font, FontRole};
    /// assert_eq!(Font::display(80.0).role, FontRole::Display);
    /// ```
    pub const fn display(size: f32) -> Self {
        Self::new(FontRole::Display, size)
    }

    /// Monospace at `size` points.
    ///
    /// ```
    /// use mdeck_sdk::paint::{Font, FontRole};
    /// assert_eq!(Font::mono(14.0).role, FontRole::Mono);
    /// ```
    pub const fn mono(size: f32) -> Self {
        Self::new(FontRole::Mono, size)
    }
}

/// Side of one sprite cell in [`sprite_sheet`], in pixels.
pub const SPRITE_CELL: usize = 64;

/// The lens cell of [`sprite_sheet`] in uv coordinates: a flat disc with a
/// faint rim, lit from the top left.
pub const SPRITE_LENS: Rect = Rect {
    min: Pos2 { x: 0.0, y: 0.0 },
    max: Pos2 {
        x: 1.0 / 3.0,
        y: 1.0,
    },
};

/// The core cell of [`sprite_sheet`] in uv coordinates: a bright, slightly soft dome.
pub const SPRITE_CORE: Rect = Rect {
    min: Pos2 {
        x: 1.0 / 3.0,
        y: 0.0,
    },
    max: Pos2 {
        x: 2.0 / 3.0,
        y: 1.0,
    },
};

/// The glow cell of [`sprite_sheet`] in uv coordinates: a wide soft falloff.
pub const SPRITE_GLOW: Rect = Rect {
    min: Pos2 {
        x: 2.0 / 3.0,
        y: 0.0,
    },
    max: Pos2 { x: 1.0, y: 1.0 },
};

/// The shared sprite sheet: lens, core and glow side by side, white,
/// premultiplied. Load it once with [`crate::paint::Painter::load_texture`]
/// and draw light with [`Mesh::add_rect_uv`] and [`SPRITE_LENS`],
/// [`SPRITE_CORE`] or [`SPRITE_GLOW`].
///
/// ```
/// use mdeck_sdk::paint::{sprite_sheet, SPRITE_CELL};
/// let sheet = sprite_sheet();
/// assert_eq!(sheet.size, [SPRITE_CELL * 3, SPRITE_CELL]);
/// ```
pub fn sprite_sheet() -> ImageData {
    let w = SPRITE_CELL * 3;
    let mut pixels = vec![Color::TRANSPARENT; w * SPRITE_CELL];
    for y in 0..SPRITE_CELL {
        for x in 0..SPRITE_CELL {
            let dx = (x as f32 + 0.5) / SPRITE_CELL as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / SPRITE_CELL as f32 * 2.0 - 1.0;
            let r = (dx * dx + dy * dy).sqrt();
            let edge = 1.0 - smoothstep(0.82, 1.0, r);
            let rim = smoothstep(0.55, 0.9, r) * 0.5 + 0.5;
            let light = 1.0 - 0.18 * (dx + dy).max(-1.0);
            let lens = (edge * rim * light).clamp(0.0, 1.0);
            let core = (1.0 - smoothstep(0.55, 1.0, r)) * (1.0 - 0.25 * r * r);
            let glow = (-(r * r) * 5.5).exp() * (1.0 - smoothstep(0.7, 1.0, r));
            for (k, v) in [lens, core, glow].into_iter().enumerate() {
                let v8 = (v.clamp(0.0, 1.0) * 255.0) as u8;
                pixels[y * w + k * SPRITE_CELL + x] =
                    Color::from_rgba_premultiplied(v8, v8, v8, v8);
            }
        }
    }
    ImageData {
        size: [w, SPRITE_CELL],
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_sheet_cells_are_bright_in_the_middle_and_dark_at_the_corner() {
        let s = sprite_sheet();
        for cell in 0..3 {
            let mid = s
                .get(cell * SPRITE_CELL + SPRITE_CELL / 2, SPRITE_CELL / 2)
                .unwrap();
            let corner = s.get(cell * SPRITE_CELL, 0).unwrap();
            assert!(mid.a() > 100, "cell {cell}: {mid:?}");
            assert_eq!(corner.a(), 0, "cell {cell}");
        }
    }

    #[test]
    fn untextured_mesh_uses_the_white_uv() {
        let mut m = Mesh::default();
        m.add_rect(
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color::WHITE,
        );
        let e = m.to_egui();
        assert_eq!(e.texture_id, egui::TextureId::default());
        assert!(e.vertices.iter().all(|v| v.uv == egui::epaint::WHITE_UV));
    }
}
