//! The [`Painter`]: everything an extension draws goes through it.

use std::sync::Arc;

use super::color::Color;
use super::geom::{Align2, FromEgui, Pos2, Rect, ToEgui, Vec2};
use super::shapes::{Font, FontRole, ImageData, Mesh, Stroke, Texture, TextureFilter};
use super::sprites::{Sprite, SpriteBlend, SpriteLayer};
use crate::cloud::Mask;

/// Which egui font family each [`FontRole`] uses. The host registers the
/// theme's faces with egui and then tells the SDK their family names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FontFamilies {
    pub(crate) names: [Option<String>; 5],
}

impl FontFamilies {
    pub(crate) fn index(role: FontRole) -> usize {
        match role {
            FontRole::Display => 0,
            FontRole::Body => 1,
            FontRole::Lead => 2,
            FontRole::Strong => 3,
            FontRole::Mono => 4,
        }
    }

    pub(crate) fn id() -> egui::Id {
        egui::Id::new("mdeck-sdk-font-families")
    }
}

/// How a painter draws what egui cannot: the host's rendering backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backend {
    /// The glow (OpenGL) renderer: sprites use a GL paint callback.
    Glow,
    /// Anything else (headless rendering, tests): sprites become a mesh.
    Mesh,
}

/// The drawing surface handed to engines, visuals, design sets and
/// transitions. Coordinates are in points on the slide; multiply pixel
/// sizes by the frame's `scale` so drawing is resolution independent.
///
/// A painter is cheap to clone; [`Painter::with_clip`] and
/// [`Painter::with_opacity`] make derived painters for a part of the work.
///
/// ```no_run
/// use mdeck_sdk::paint::{Align2, Color, Font, Painter, Pos2, Stroke};
/// fn draw(p: &Painter) {
///     let r = p.clip_rect();
///     p.circle_filled(r.center(), 40.0, Color::from_rgb(255, 77, 28));
///     p.line_segment([r.min, r.max], Stroke::new(2.0, Color::WHITE));
///     p.text(r.center(), Align2::CENTER_CENTER, "Hello", Font::display(64.0), Color::WHITE);
/// }
/// ```
#[derive(Clone)]
pub struct Painter {
    inner: egui::Painter,
    backend: Backend,
    families: [egui::FontFamily; 5],
}

impl std::fmt::Debug for Painter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Painter")
            .field("clip_rect", &self.clip_rect())
            .field("backend", &self.backend)
            .finish()
    }
}

impl Painter {
    /// Wrap an egui painter. Font roles resolve through the families the
    /// host set; a family egui does not know falls back to egui's default
    /// proportional (or monospace) face instead of panicking.
    pub(crate) fn new(inner: egui::Painter, backend: Backend) -> Self {
        let ctx = inner.ctx();
        let set = ctx
            .data(|d| d.get_temp::<Arc<FontFamilies>>(FontFamilies::id()))
            .unwrap_or_default();
        let known = ctx.fonts(|f| f.families());
        let families = FontRole::ALL.map(|role| {
            let fallback = if role == FontRole::Mono {
                egui::FontFamily::Monospace
            } else {
                egui::FontFamily::Proportional
            };
            match &set.names[FontFamilies::index(role)] {
                Some(name) => {
                    let fam = egui::FontFamily::Name(name.as_str().into());
                    if known.contains(&fam) { fam } else { fallback }
                }
                None => fallback,
            }
        });
        Self {
            inner,
            backend,
            families,
        }
    }

    fn font_id(&self, font: Font) -> egui::FontId {
        egui::FontId::new(
            font.size.max(0.5),
            self.families[FontFamilies::index(font.role)].clone(),
        )
    }

    /// The area this painter draws into; anything outside is clipped.
    ///
    /// See [`Painter`] for an example.
    pub fn clip_rect(&self) -> Rect {
        self.inner.clip_rect().sdk()
    }

    /// A painter that clips to `rect` (intersected with the current clip).
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// let half = p.clip_rect().sub_rect(0.0, 0.0, 0.5, 1.0);
    /// let left = p.with_clip(half);
    /// assert!(left.clip_rect().width() <= p.clip_rect().width());
    /// # }
    /// ```
    pub fn with_clip(&self, rect: Rect) -> Painter {
        Painter {
            inner: self.inner.with_clip_rect(rect.eg()),
            backend: self.backend,
            families: self.families.clone(),
        }
    }

    /// A painter whose every shape is drawn at `opacity` (0..1) times this
    /// one's.
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// let faint = p.with_opacity(0.3);
    /// assert!(faint.opacity() <= 0.3);
    /// # }
    /// ```
    pub fn with_opacity(&self, opacity: f32) -> Painter {
        let mut inner = self.inner.clone();
        inner.multiply_opacity(opacity.clamp(0.0, 1.0));
        Painter {
            inner,
            backend: self.backend,
            families: self.families.clone(),
        }
    }

    /// The opacity every shape is drawn at.
    ///
    /// See [`Painter::with_opacity`] for an example.
    pub fn opacity(&self) -> f32 {
        self.inner.opacity()
    }

    /// Physical pixels per point on the current display or export canvas.
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// assert!(p.pixels_per_point() > 0.0);
    /// # }
    /// ```
    pub fn pixels_per_point(&self) -> f32 {
        self.inner.pixels_per_point()
    }

    /// A filled circle.
    ///
    /// See [`Painter`] for an example.
    pub fn circle_filled(&self, center: Pos2, radius: f32, color: Color) {
        self.inner.circle_filled(center.eg(), radius, color.eg());
    }

    /// A circle outline.
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::{Color, Stroke};
    /// p.circle_stroke(p.clip_rect().center(), 30.0, Stroke::new(1.5, Color::WHITE));
    /// # }
    /// ```
    pub fn circle_stroke(&self, center: Pos2, radius: f32, stroke: Stroke) {
        self.inner.circle_stroke(center.eg(), radius, stroke.eg());
    }

    /// A straight line between two points.
    ///
    /// See [`Painter`] for an example.
    pub fn line_segment(&self, points: [Pos2; 2], stroke: Stroke) {
        self.inner
            .line_segment([points[0].eg(), points[1].eg()], stroke.eg());
    }

    /// An open polyline through `points`.
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::{Color, Pos2, Stroke};
    /// p.line(vec![Pos2::new(0.0, 0.0), Pos2::new(50.0, 20.0), Pos2::new(100.0, 0.0)],
    ///        Stroke::new(2.0, Color::WHITE));
    /// # }
    /// ```
    pub fn line(&self, points: Vec<Pos2>, stroke: Stroke) {
        if points.len() < 2 {
            return;
        }
        self.inner
            .line(points.into_iter().map(ToEgui::eg).collect(), stroke.eg());
    }

    /// A filled convex polygon with an outline (use [`Stroke::NONE`] for none).
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::{Color, Pos2, Stroke};
    /// let tri = vec![Pos2::new(0.0, 0.0), Pos2::new(40.0, 0.0), Pos2::new(20.0, 30.0)];
    /// p.convex_polygon(tri, Color::WHITE, Stroke::NONE);
    /// # }
    /// ```
    pub fn convex_polygon(&self, points: Vec<Pos2>, fill: Color, stroke: Stroke) {
        if points.len() < 3 {
            return;
        }
        self.inner.add(egui::Shape::convex_polygon(
            points.into_iter().map(ToEgui::eg).collect(),
            fill.eg(),
            stroke.eg(),
        ));
    }

    /// A filled rectangle with rounded corners (`corner_radius` in points; 0 for square).
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::Color;
    /// p.rect_filled(p.clip_rect().shrink(20.0), 8.0, Color::from_gray(30));
    /// # }
    /// ```
    pub fn rect_filled(&self, rect: Rect, corner_radius: f32, color: Color) {
        self.inner
            .rect_filled(rect.eg(), corner_radius_of(corner_radius), color.eg());
    }

    /// A rectangle outline, centred on the rectangle's edge.
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::{Color, Stroke};
    /// p.rect_stroke(p.clip_rect().shrink(20.0), 0.0, Stroke::new(1.0, Color::WHITE));
    /// # }
    /// ```
    pub fn rect_stroke(&self, rect: Rect, corner_radius: f32, stroke: Stroke) {
        self.inner.rect_stroke(
            rect.eg(),
            corner_radius_of(corner_radius),
            stroke.eg(),
            egui::StrokeKind::Middle,
        );
    }

    /// A triangle mesh, textured or plain.
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::{Color, Mesh};
    /// let mut m = Mesh::default();
    /// m.add_rect(p.clip_rect().shrink(100.0), Color::from_rgb(0, 40, 80));
    /// p.mesh(m);
    /// # }
    /// ```
    pub fn mesh(&self, mesh: Mesh) {
        if mesh.is_empty() {
            return;
        }
        self.inner.add(egui::Shape::mesh(mesh.to_egui()));
    }

    /// Draw the `uv` part (0..1) of `texture` into `rect`, multiplied by `tint`
    /// ([`Color::WHITE`] draws it as it is).
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::{Color, ImageData, Pos2, Rect, TextureFilter};
    /// let tex = p.load_texture("swatch", &ImageData::filled([2, 2], Color::WHITE), TextureFilter::Nearest);
    /// let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    /// p.image(&tex, p.clip_rect().shrink(50.0), full, Color::WHITE);
    /// # }
    /// ```
    pub fn image(&self, texture: &Texture, rect: Rect, uv: Rect, tint: Color) {
        self.inner
            .image(texture.handle.id(), rect.eg(), uv.eg(), tint.eg());
    }

    /// Draw `text` on one line with its `align` point at `pos`, in the
    /// theme's face for `font.role`. Returns the rectangle the text covers.
    ///
    /// See [`Painter`] for an example.
    pub fn text(&self, pos: Pos2, align: Align2, text: &str, font: Font, color: Color) -> Rect {
        self.inner
            .text(pos.eg(), align.eg(), text, self.font_id(font), color.eg())
            .sdk()
    }

    /// The size `text` takes on one line in `font`, without drawing it.
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::Font;
    /// let size = p.text_size("Hello", Font::body(32.0));
    /// assert!(size.x > 0.0);
    /// # }
    /// ```
    pub fn text_size(&self, text: &str, font: Font) -> Vec2 {
        let id = self.font_id(font);
        self.inner
            .ctx()
            .fonts_mut(|f| f.layout_no_wrap(text.to_owned(), id, egui::Color32::WHITE))
            .size()
            .sdk()
    }

    /// The glyphs of `text` set on one line in `font` as quads cut from the
    /// font atlas, with the text's top-left corner at `pos`: a mesh on the
    /// atlas texture with four vertices and two triangles per visible glyph
    /// (in [`Mesh::add_rect_uv`]'s order: top-left, top-right, bottom-right,
    /// bottom-left), each coloured `color`. Nothing is drawn; pass the mesh
    /// (or quads cut, squashed or warped from it) to [`Painter::mesh`].
    ///
    /// The atlas's uv `(0, 0)` is a white texel, so plain quads with uv
    /// [`Pos2::ZERO`] can go into the same mesh: a split-flap board draws its
    /// flaps and the halves of their characters folding at the hinge as one
    /// mesh. The atlas may be rebuilt between frames, so build the mesh in
    /// the frame that draws it.
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::{Color, Font, Pos2};
    /// let mut m = p.glyph_mesh(Pos2::new(100.0, 100.0), "AB", Font::display(64.0), Color::WHITE);
    /// assert_eq!(m.vertices.len(), 8);
    /// // keep only the top half of each glyph: move the bottom vertices up
    /// for q in m.vertices.chunks_mut(4) {
    ///     let mid = (q[0].pos.y + q[2].pos.y) / 2.0;
    ///     let v_mid = (q[0].uv.y + q[2].uv.y) / 2.0;
    ///     for v in &mut q[2..] {
    ///         v.pos.y = mid;
    ///         v.uv.y = v_mid;
    ///     }
    /// }
    /// p.mesh(m);
    /// # }
    /// ```
    pub fn glyph_mesh(&self, pos: Pos2, text: &str, font: Font, color: Color) -> Mesh {
        let id = self.font_id(font);
        let (quads, atlas) = self.inner.ctx().fonts_mut(|f| {
            let galley = f.layout_no_wrap(text.to_owned(), id, egui::Color32::WHITE);
            let quads: Vec<(egui::Rect, [u16; 2], [u16; 2])> = galley
                .rows
                .iter()
                .flat_map(|row| {
                    row.glyphs
                        .iter()
                        .filter(|g| g.uv_rect.max[0] > g.uv_rect.min[0])
                        .map(move |g| {
                            let min = row.pos + g.pos.to_vec2() + g.uv_rect.offset;
                            (
                                egui::Rect::from_min_size(min, g.uv_rect.size),
                                g.uv_rect.min,
                                g.uv_rect.max,
                            )
                        })
                })
                .collect();
            (quads, f.font_image_size())
        });
        let mut mesh = Mesh::with_texture(Texture {
            handle: super::shapes::TextureRef::FontAtlas(atlas),
        });
        let (aw, ah) = (atlas[0].max(1) as f32, atlas[1].max(1) as f32);
        for (rect, min, max) in quads {
            let uv = Rect::from_min_max(
                Pos2::new(min[0] as f32 / aw, min[1] as f32 / ah),
                Pos2::new(max[0] as f32 / aw, max[1] as f32 / ah),
            );
            mesh.add_rect_uv(rect.sdk().translate(pos.to_vec2()), uv, color);
        }
        mesh
    }

    /// Upload `image` as a texture named `name` (the name shows in debug
    /// tools only).
    ///
    /// See [`Painter::image`] for an example.
    pub fn load_texture(&self, name: &str, image: &ImageData, filter: TextureFilter) -> Texture {
        Texture {
            handle: self
                .inner
                .ctx()
                .load_texture(name, image.to_egui(), filter.eg())
                .into(),
        }
    }

    /// Replace a texture's pixels (the size may change).
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::{Color, ImageData, TextureFilter};
    /// let mut tex = p.load_texture("live", &ImageData::filled([2, 2], Color::BLACK), TextureFilter::Linear);
    /// p.update_texture(&mut tex, &ImageData::filled([4, 4], Color::WHITE), TextureFilter::Linear);
    /// # }
    /// ```
    pub fn update_texture(&self, texture: &mut Texture, image: &ImageData, filter: TextureFilter) {
        texture.handle.set(image.to_egui(), filter.eg());
    }

    /// Draw soft round sprites (particles, embers, lights) with `blend`.
    /// `layer` keeps the GPU state between frames: an engine owns one per
    /// stream of sprites it draws.
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter, layer: &mdeck_sdk::paint::SpriteLayer) {
    /// use mdeck_sdk::paint::{Sprite, SpriteBlend};
    /// let c = p.clip_rect().center();
    /// p.sprites(layer, vec![Sprite { center: c, size: 24.0, rgba: [1.0, 0.4, 0.1, 0.9] }],
    ///           SpriteBlend::Additive);
    /// # }
    /// ```
    pub fn sprites(&self, layer: &SpriteLayer, sprites: Vec<Sprite>, blend: SpriteBlend) {
        match self.backend {
            Backend::Glow => layer.paint_gl(&self.inner, self.clip_rect(), sprites, blend),
            Backend::Mesh => {
                let opacity = self.opacity();
                let sprites: Vec<Sprite> = if opacity < 1.0 {
                    sprites
                        .into_iter()
                        .map(|mut s| {
                            s.rgba[3] *= opacity;
                            s
                        })
                        .collect()
                } else {
                    sprites
                };
                self.mesh(layer.mesh(self.inner.ctx(), &sprites, blend));
            }
        }
    }

    /// Sample up to `n` points inside the glyphs of `text` set in `font`,
    /// as a [`Mask`] in the unit square of the text's ink. The points are
    /// shuffled once (deterministically), so any prefix of them covers the
    /// whole text evenly: a group of `k` particles takes the first `k`.
    ///
    /// `font.size` sets the sampling resolution: 200 points or more gives
    /// dense masks for a countdown digit or a short word.
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::Font;
    /// let mask = p.glyph_points("3", Font::display(220.0), 2000);
    /// assert!(mask.points.len() <= 2000);
    /// assert!(mask.aspect > 0.0);
    /// # }
    /// ```
    pub fn glyph_points(&self, text: &str, font: Font, n: usize) -> Mask {
        let id = self.font_id(font);
        let (glyphs, image) = self.inner.ctx().fonts_mut(|f| {
            let galley = f.layout_no_wrap(text.to_string(), id, egui::Color32::WHITE);
            let glyphs: Vec<(egui::Pos2, egui::Vec2, [u16; 2], [u16; 2])> = galley
                .rows
                .iter()
                .flat_map(|r| r.glyphs.iter())
                .filter(|g| g.uv_rect.max[0] > g.uv_rect.min[0])
                .map(|g| {
                    (
                        g.pos + g.uv_rect.offset,
                        g.uv_rect.size,
                        g.uv_rect.min,
                        g.uv_rect.max,
                    )
                })
                .collect();
            (glyphs, f.image())
        });
        glyph_mask(&glyphs, &image, n)
    }

    /// Every pixel of ink in `text` set in `font` on one line whose coverage
    /// is at least `min_alpha`, as the pixel's top-left corner in points
    /// relative to the text's top-left (where [`Painter::text`] with
    /// [`Align2::LEFT_TOP`] puts it). Unlike [`Painter::glyph_points`] the
    /// points keep their place, so an engine can rasterise a heading
    /// published as [`crate::geometry::Hint::Text`] into a grid of its own
    /// (the thermal engine's cold opening).
    ///
    /// ```no_run
    /// # fn demo(p: &mdeck_sdk::paint::Painter) {
    /// use mdeck_sdk::paint::Font;
    /// let ink = p.glyph_ink("I", Font::display(100.0), 100);
    /// assert!(!ink.is_empty());
    /// assert!(p.glyph_ink(" ", Font::display(100.0), 100).is_empty());
    /// # }
    /// ```
    pub fn glyph_ink(&self, text: &str, font: Font, min_alpha: u8) -> Vec<Pos2> {
        let id = self.font_id(font);
        self.inner.ctx().fonts_mut(|f| {
            let galley = f.layout_no_wrap(text.to_string(), id, egui::Color32::WHITE);
            let atlas = f.image();
            let mut out = Vec::new();
            for row in &galley.rows {
                for g in &row.glyphs {
                    let (min, max) = (g.uv_rect.min, g.uv_rect.max);
                    if max[0] <= min[0] || max[1] <= min[1] {
                        continue;
                    }
                    let origin = row.pos + g.pos.to_vec2() + g.uv_rect.offset;
                    let size = g.uv_rect.size;
                    for ay in min[1]..max[1] {
                        for ax in min[0]..max[0] {
                            let (x, y) = (ax as usize, ay as usize);
                            if x >= atlas.size[0]
                                || y >= atlas.size[1]
                                || atlas[(x, y)].a() < min_alpha
                            {
                                continue;
                            }
                            out.push(Pos2::new(
                                origin.x + (ax - min[0]) as f32 / (max[0] - min[0]) as f32 * size.x,
                                origin.y + (ay - min[1]) as f32 / (max[1] - min[1]) as f32 * size.y,
                            ));
                        }
                    }
                }
            }
            out
        })
    }

    /// The egui painter behind this one.
    ///
    /// **Unstable:** outside the compatibility promise (EXT-26); any egui
    /// release may break code that uses it. Every need for it is a request
    /// to extend this painter.
    #[cfg(feature = "unstable-egui")]
    pub fn egui_painter(&self) -> &egui::Painter {
        &self.inner
    }

    pub(crate) fn inner(&self) -> &egui::Painter {
        &self.inner
    }
}

fn corner_radius_of(r: f32) -> egui::CornerRadius {
    egui::CornerRadius::same(r.clamp(0.0, 255.0).round() as u8)
}

type Glyph = (egui::Pos2, egui::Vec2, [u16; 2], [u16; 2]);

/// Sample glyph coverage from the font atlas `image` into mask points.
fn glyph_mask(glyphs: &[Glyph], image: &egui::ColorImage, n: usize) -> Mask {
    if glyphs.is_empty() || n == 0 {
        return Mask::new(Vec::new(), 0.6);
    }
    let (mut bx0, mut by0, mut bx1, mut by1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (pos, size, _, _) in glyphs {
        bx0 = bx0.min(pos.x);
        by0 = by0.min(pos.y);
        bx1 = bx1.max(pos.x + size.x);
        by1 = by1.max(pos.y + size.y);
    }
    let (bw, bh) = ((bx1 - bx0).max(1.0), (by1 - by0).max(1.0));
    let mut pts = Vec::new();
    for (pos, size, min, max) in glyphs {
        let (x0, y0, x1, y1) = (
            min[0] as usize,
            min[1] as usize,
            max[0] as usize,
            max[1] as usize,
        );
        let (gw, gh) = ((x1 - x0).max(1), (y1 - y0).max(1));
        let step = (bh as usize / 90).max(1);
        for y in (y0..y1.min(image.size[1])).step_by(step) {
            for x in (x0..x1.min(image.size[0])).step_by(step) {
                if image[(x, y)].a() > 110 {
                    let px = pos.x + (x - x0) as f32 / gw as f32 * size.x;
                    let py = pos.y + (y - y0) as f32 / gh as f32 * size.y;
                    pts.push([(px - bx0) / bw, (py - by0) / bh]);
                }
            }
        }
    }
    // Masks are consumed in order, and scanline order would light the top
    // of the glyph first.
    shuffle(&mut pts, 0x6C7F);
    pts.truncate(n);
    Mask::new(pts, bw / bh)
}

/// Deterministic Fisher-Yates (xorshift64*).
pub(crate) fn shuffle(pts: &mut [[f32; 2]], seed: u64) {
    let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut unit = || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        (s.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 24) as f32
    };
    for i in (1..pts.len()).rev() {
        let j = (unit() * (i + 1) as f32) as usize;
        pts.swap(i, j.min(i));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_painter(f: impl FnOnce(&Painter)) {
        let ctx = egui::Context::default();
        let mut f = Some(f);
        let mut out = ctx.run_ui(Default::default(), |ui| {
            let p = Painter::new(ui.painter().clone(), Backend::Mesh);
            if let Some(f) = f.take() {
                f(&p);
            }
        });
        out.textures_delta.clear();
    }

    #[test]
    fn glyph_points_are_dense_unit_square_and_capped() {
        with_painter(|p| {
            let mask = p.glyph_points("8", Font::display(220.0), 100_000);
            assert!(mask.points.len() > 300, "{}", mask.points.len());
            assert!(mask.aspect > 0.3 && mask.aspect < 1.0, "{}", mask.aspect);
            assert!(
                mask.points
                    .iter()
                    .all(|q| (0.0..=1.0).contains(&q[0]) && (0.0..=1.0).contains(&q[1]))
            );
            let few = p.glyph_points("8", Font::display(220.0), 50);
            assert_eq!(few.points.len(), 50);
            assert!(
                p.glyph_points("", Font::display(220.0), 50)
                    .points
                    .is_empty()
            );
        });
    }

    #[test]
    fn glyph_ink_keeps_its_place_inside_the_text() {
        with_painter(|p| {
            let font = Font::display(100.0);
            let ink = p.glyph_ink("Hi", font, 100);
            assert!(ink.len() > 500, "{}", ink.len());
            let size = p.text_size("Hi", font);
            assert!(
                ink.iter()
                    .all(|q| q.x >= 0.0 && q.y >= 0.0 && q.x <= size.x && q.y <= size.y)
            );
            // a space has no ink, and a stricter threshold keeps less
            assert!(p.glyph_ink(" ", font, 100).is_empty());
            assert!(p.glyph_ink("Hi", font, 250).len() < ink.len());
        });
    }

    #[test]
    fn unknown_font_family_falls_back_instead_of_panicking() {
        let ctx = egui::Context::default();
        crate::host::set_font_families(
            &ctx,
            crate::host::FontFamilyNames {
                display: Some("no-such-face".into()),
                ..Default::default()
            },
        );
        let mut out = ctx.run_ui(Default::default(), |ui| {
            let p = Painter::new(ui.painter().clone(), Backend::Mesh);
            assert!(p.text_size("Hi", Font::display(40.0)).x > 0.0);
        });
        out.textures_delta.clear();
    }

    #[test]
    fn glyph_mesh_cuts_quads_from_the_atlas() {
        with_painter(|p| {
            let m = p.glyph_mesh(
                Pos2::new(10.0, 20.0),
                "A B",
                Font::display(40.0),
                Color::WHITE,
            );
            assert_eq!(m.vertices.len(), 8, "the space has no quad");
            assert_eq!(m.indices.len(), 12);
            let tex = m.texture.as_ref().expect("on the atlas");
            assert!(tex.size()[0] > 0);
            assert!(
                m.vertices
                    .iter()
                    .all(|v| v.pos.x >= 10.0 && v.pos.y >= 20.0)
            );
            assert!(
                m.vertices
                    .iter()
                    .all(|v| (0.0..=1.0).contains(&v.uv.x) && (0.0..=1.0).contains(&v.uv.y))
            );
            assert!(m.to_egui().texture_id == egui::TextureId::default());
            assert!(
                p.glyph_mesh(Pos2::ZERO, "", Font::display(40.0), Color::WHITE)
                    .is_empty()
            );
        });
    }

    #[test]
    fn derived_painters_clip_and_fade() {
        with_painter(|p| {
            let small = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::splat(20.0));
            assert_eq!(p.with_clip(small).clip_rect(), small);
            assert!((p.with_opacity(0.5).with_opacity(0.5).opacity() - 0.25).abs() < 1e-6);
        });
    }

    #[test]
    fn shuffle_is_deterministic_and_a_permutation() {
        let mut a: Vec<[f32; 2]> = (0..50).map(|i| [i as f32, 0.0]).collect();
        let mut b = a.clone();
        shuffle(&mut a, 7);
        shuffle(&mut b, 7);
        assert_eq!(a, b);
        let mut xs: Vec<i32> = a.iter().map(|p| p[0] as i32).collect();
        xs.sort();
        assert_eq!(xs, (0..50).collect::<Vec<_>>());
    }
}
