//! Headless rendering for extension tests: draw an engine frame (or any
//! painting) to an [`ImageData`] without a window or a GPU.
//!
//! [`Headless`] runs a real egui context, tessellates what was painted and
//! rasterises the triangles on the CPU, with premultiplied blending like
//! the GPU. Sprites ([`crate::paint::Painter::sprites`]) are drawn as
//! additive meshes, so they show, but without wakes. The result is close to
//! what mdeck shows, not identical (no multisampling, sRGB blending): use
//! it for "does it draw, and where" tests and tolerant golden comparisons
//! ([`mean_difference`]), not for pixel-exact ones.
//!
//! ```
//! use mdeck_sdk::paint::Color;
//! use mdeck_sdk::testing::Headless;
//!
//! let mut h = Headless::new(64, 36);
//! let img = h.paint(|p| p.circle_filled(p.clip_rect().center(), 10.0, Color::WHITE));
//! assert_eq!(img.size, [64, 36]);
//! assert_eq!(img.get(32, 18), Some(Color::WHITE));
//! assert_eq!(img.get(0, 0).unwrap().a(), 0);
//! ```

use std::collections::HashMap;

use crate::engine::Engine;
use crate::paint::painter::Backend;
use crate::paint::{ImageData, Painter, Pos2, Rect, Vec2};
use crate::stage::{Frame, Stage};

/// A headless egui context that renders to images. Keep one per test:
/// textures an engine loads stay valid across its frames.
pub struct Headless {
    ctx: egui::Context,
    width: usize,
    height: usize,
    textures: HashMap<egui::TextureId, Tex>,
}

struct Tex {
    size: [usize; 2],
    pixels: Vec<egui::Color32>,
}

impl std::fmt::Debug for Headless {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Headless")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl Headless {
    /// A canvas of `width` by `height` pixels (one pixel per point).
    ///
    /// See the [module docs](self) for an example.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            ctx: egui::Context::default(),
            width,
            height,
            textures: HashMap::new(),
        }
    }

    /// The canvas as a rect in points.
    ///
    /// ```
    /// let h = mdeck_sdk::testing::Headless::new(192, 108);
    /// assert_eq!(h.rect().width(), 192.0);
    /// ```
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(self.width as f32, self.height as f32))
    }

    /// Run `draw` with a painter over the canvas and return the pixels.
    /// The canvas starts transparent.
    ///
    /// See the [module docs](self) for an example.
    pub fn paint(&mut self, draw: impl FnOnce(&mut Painter)) -> ImageData {
        let rect = self.rect();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(self.width as f32, self.height as f32),
            )),
            ..Default::default()
        };
        let mut draw = Some(draw);
        let mut out = self.ctx.run_ui(input, |ui| {
            let layer =
                egui::LayerId::new(egui::Order::Foreground, egui::Id::new("mdeck-sdk-headless"));
            let inner = egui::Painter::new(ui.ctx().clone(), layer, crate::host::egui_rect(rect));
            let mut painter = Painter::new(inner, Backend::Mesh);
            if let Some(draw) = draw.take() {
                draw(&mut painter);
            }
        });
        for (id, deltas) in &out.textures_delta.set {
            for delta in deltas {
                self.apply(*id, delta);
            }
        }
        let freed = std::mem::take(&mut out.textures_delta.free);
        out.textures_delta.clear();
        let prims = self.ctx.tessellate(std::mem::take(&mut out.shapes), 1.0);
        let mut canvas = vec![egui::Color32::TRANSPARENT; self.width * self.height];
        for prim in prims {
            if let egui::epaint::Primitive::Mesh(mesh) = &prim.primitive {
                self.raster(&mut canvas, mesh, prim.clip_rect);
            }
        }
        for id in &freed {
            self.textures.remove(id);
        }
        ImageData {
            size: [self.width, self.height],
            pixels: canvas.into_iter().map(crate::host::color).collect(),
        }
    }

    /// Render one engine frame: fill the background with the theme's, run
    /// [`Engine::update`] and then [`Engine::paint`]. Build `frame` over
    /// [`Headless::rect`] (for example with [`Frame::new`]).
    ///
    /// ```
    /// use mdeck_sdk::engine::Engine;
    /// use mdeck_sdk::paint::Painter;
    /// use mdeck_sdk::stage::{Frame, Moment, Stage};
    /// use mdeck_sdk::testing::Headless;
    /// use mdeck_sdk::tokens::{EngineSettings, Tokens};
    ///
    /// struct Dot;
    /// impl Engine for Dot {
    ///     fn update(&mut self, _: &Frame, _: &Stage) {}
    ///     fn paint(&mut self, p: &mut Painter, f: &Frame, _: &Stage) {
    ///         p.circle_filled(f.rect.center(), 8.0, f.tokens.accent);
    ///     }
    /// }
    ///
    /// let mut h = Headless::new(64, 36);
    /// let (tokens, settings) = (Tokens::default(), EngineSettings::new());
    /// let frame = Frame::new(h.rect(), &tokens, &settings);
    /// let img = h.render_engine(&mut Dot, &frame, &Stage::new(Moment::Slide));
    /// assert_eq!(img.get(32, 18), Some(tokens.accent));
    /// assert_eq!(img.get(0, 0), Some(tokens.background));
    /// ```
    pub fn render_engine(
        &mut self,
        engine: &mut dyn Engine,
        frame: &Frame,
        stage: &Stage,
    ) -> ImageData {
        self.paint(|p| {
            p.rect_filled(frame.rect, 0.0, frame.tokens.background);
            engine.update(frame, stage);
            engine.paint(p, frame, stage);
        })
    }

    fn apply(&mut self, id: egui::TextureId, delta: &egui::epaint::ImageDelta) {
        let egui::ImageData::Color(img) = &delta.image;
        match delta.pos {
            None => {
                self.textures.insert(
                    id,
                    Tex {
                        size: img.size,
                        pixels: img.pixels.clone(),
                    },
                );
            }
            Some([x0, y0]) => {
                if let Some(t) = self.textures.get_mut(&id) {
                    for y in 0..img.size[1] {
                        for x in 0..img.size[0] {
                            let (tx, ty) = (x0 + x, y0 + y);
                            if tx < t.size[0] && ty < t.size[1] {
                                t.pixels[ty * t.size[0] + tx] = img.pixels[y * img.size[0] + x];
                            }
                        }
                    }
                }
            }
        }
    }

    fn raster(&self, canvas: &mut [egui::Color32], mesh: &egui::Mesh, clip: egui::Rect) {
        let tex = self.textures.get(&mesh.texture_id);
        let (w, h) = (self.width as i32, self.height as i32);
        let cx0 = clip.min.x.floor().max(0.0) as i32;
        let cy0 = clip.min.y.floor().max(0.0) as i32;
        let cx1 = (clip.max.x.ceil() as i32).min(w);
        let cy1 = (clip.max.y.ceil() as i32).min(h);
        for tri in mesh.indices.as_chunks::<3>().0 {
            let v = [
                &mesh.vertices[tri[0] as usize],
                &mesh.vertices[tri[1] as usize],
                &mesh.vertices[tri[2] as usize],
            ];
            let area = edge(v[0].pos, v[1].pos, v[2].pos);
            if area.abs() < 1e-8 {
                continue;
            }
            let minx = v.iter().map(|v| v.pos.x).fold(f32::MAX, f32::min).floor() as i32;
            let maxx = v.iter().map(|v| v.pos.x).fold(f32::MIN, f32::max).ceil() as i32;
            let miny = v.iter().map(|v| v.pos.y).fold(f32::MAX, f32::min).floor() as i32;
            let maxy = v.iter().map(|v| v.pos.y).fold(f32::MIN, f32::max).ceil() as i32;
            for y in miny.max(cy0)..maxy.min(cy1) {
                for x in minx.max(cx0)..maxx.min(cx1) {
                    let p = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
                    let w0 = edge(v[1].pos, v[2].pos, p) / area;
                    let w1 = edge(v[2].pos, v[0].pos, p) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let lerp = |f: fn(&egui::epaint::Vertex) -> f32| {
                        f(v[0]) * w0 + f(v[1]) * w1 + f(v[2]) * w2
                    };
                    let col = [
                        lerp(|v| v.color.r() as f32),
                        lerp(|v| v.color.g() as f32),
                        lerp(|v| v.color.b() as f32),
                        lerp(|v| v.color.a() as f32),
                    ];
                    let texel = match tex {
                        Some(t) => sample(t, lerp(|v| v.uv.x), lerp(|v| v.uv.y)),
                        None => [255.0; 4],
                    };
                    let src: [f32; 4] = std::array::from_fn(|i| col[i] * texel[i] / 255.0);
                    let dst = &mut canvas[(y * w + x) as usize];
                    let k = 1.0 - src[3] / 255.0;
                    let b = |s: f32, d: u8| (s + d as f32 * k).round().clamp(0.0, 255.0) as u8;
                    *dst = egui::Color32::from_rgba_premultiplied(
                        b(src[0], dst.r()),
                        b(src[1], dst.g()),
                        b(src[2], dst.b()),
                        b(src[3], dst.a()),
                    );
                }
            }
        }
    }
}

fn edge(a: egui::Pos2, b: egui::Pos2, p: egui::Pos2) -> f32 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

/// Bilinear sample at uv (0..1), premultiplied channels 0..255.
fn sample(t: &Tex, u: f32, v: f32) -> [f32; 4] {
    let (tw, th) = (t.size[0] as i32, t.size[1] as i32);
    if tw == 0 || th == 0 {
        return [0.0; 4];
    }
    let x = u * tw as f32 - 0.5;
    let y = v * th as f32 - 0.5;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |xi: i32, yi: i32| {
        let c = t.pixels[(yi.clamp(0, th - 1) * tw + xi.clamp(0, tw - 1)) as usize];
        [c.r() as f32, c.g() as f32, c.b() as f32, c.a() as f32]
    };
    let (x0, y0) = (x0 as i32, y0 as i32);
    let (a, b, c, d) = (
        at(x0, y0),
        at(x0 + 1, y0),
        at(x0, y0 + 1),
        at(x0 + 1, y0 + 1),
    );
    std::array::from_fn(|i| {
        let top = a[i] + (b[i] - a[i]) * fx;
        let bot = c[i] + (d[i] - c[i]) * fx;
        top + (bot - top) * fy
    })
}

/// The mean absolute channel difference between two images of the same
/// size, 0 (identical) to 255; `None` when the sizes differ.
///
/// ```
/// use mdeck_sdk::paint::{Color, ImageData};
/// use mdeck_sdk::testing::mean_difference;
/// let a = ImageData::filled([2, 2], Color::BLACK);
/// let b = ImageData::filled([2, 2], Color::from_gray(10));
/// assert_eq!(mean_difference(&a, &a), Some(0.0));
/// assert_eq!(mean_difference(&a, &b), Some(7.5));
/// ```
pub fn mean_difference(a: &ImageData, b: &ImageData) -> Option<f32> {
    if a.size != b.size {
        return None;
    }
    let n = a.pixels.len().max(1) * 4;
    let sum: u64 = a
        .pixels
        .iter()
        .zip(&b.pixels)
        .flat_map(|(p, q)| {
            let (p, q) = (p.to_array(), q.to_array());
            (0..4).map(move |i| (p[i] as i32 - q[i] as i32).unsigned_abs() as u64)
        })
        .sum();
    Some(sum as f32 / n as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{Align2, Color, Font, Sprite, SpriteBlend, SpriteLayer, Stroke, additive};

    #[test]
    fn text_renders_ink_inside_its_rect() {
        let mut h = Headless::new(200, 80);
        let mut covered = Rect::ZERO;
        let img = h.paint(|p| {
            covered = p.text(
                Pos2::new(100.0, 40.0),
                Align2::CENTER_CENTER,
                "Hello",
                Font::body(32.0),
                Color::WHITE,
            );
        });
        let lit = img.pixels.iter().filter(|c| c.a() > 128).count();
        assert!(lit > 50, "only {lit} lit pixels");
        assert!(covered.width() > 40.0);
        // nothing outside the text's rect (with a pixel of slack)
        let r = covered.expand(1.0);
        for y in 0..80 {
            for x in 0..200 {
                if img.get(x, y).unwrap().a() > 0 {
                    assert!(
                        r.contains(Pos2::new(x as f32 + 0.5, y as f32 + 0.5)),
                        "{x},{y}"
                    );
                }
            }
        }
    }

    #[test]
    fn additive_sprites_add_light() {
        let mut h = Headless::new(40, 40);
        let layer = SpriteLayer::new();
        let one = |p: &mut Painter, n: usize| {
            p.rect_filled(p.clip_rect(), 0.0, Color::BLACK);
            let s = Sprite {
                center: Pos2::new(20.0, 20.0),
                size: 20.0,
                rgba: [1.0, 0.5, 0.25, 0.4],
            };
            p.sprites(&layer, vec![s; n], SpriteBlend::Additive);
        };
        let single = h.paint(|p| one(p, 1));
        let double = h.paint(|p| one(p, 2));
        let (a, b) = (single.get(20, 20).unwrap(), double.get(20, 20).unwrap());
        assert!(a.r() > 40, "{a:?}");
        assert!(b.r() > a.r() + 30, "light adds up: {a:?} then {b:?}");
        assert_eq!(b.a(), 255, "stays opaque over the background");
    }

    #[test]
    fn additive_mesh_colour_brightens_without_covering() {
        let mut h = Headless::new(10, 10);
        let img = h.paint(|p| {
            p.rect_filled(p.clip_rect(), 0.0, Color::from_gray(100));
            p.rect_filled(p.clip_rect(), 0.0, additive(Color::WHITE, 0.2));
        });
        let c = img.get(5, 5).unwrap();
        assert!((c.r() as i32 - 151).abs() <= 1, "{c:?}");
    }

    #[test]
    fn textures_persist_across_frames_and_lines_draw() {
        let mut h = Headless::new(20, 20);
        let mut tex = None;
        h.paint(|p| {
            tex = Some(p.load_texture(
                "red",
                &ImageData::filled([2, 2], Color::from_rgb(255, 0, 0)),
                crate::paint::TextureFilter::Nearest,
            ))
        });
        let tex = tex.unwrap();
        let img = h.paint(|p| {
            let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            p.image(&tex, p.clip_rect(), full, Color::WHITE);
            p.line_segment(
                [Pos2::new(0.0, 10.0), Pos2::new(20.0, 10.0)],
                Stroke::new(2.0, Color::from_rgb(0, 0, 255)),
            );
        });
        assert_eq!(img.get(3, 3), Some(Color::from_rgb(255, 0, 0)));
        assert!(img.get(10, 10).unwrap().b() > 200);
    }
}
