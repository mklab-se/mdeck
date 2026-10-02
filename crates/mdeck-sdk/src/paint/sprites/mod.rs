//! Additive glow sprites, with wakes.
//!
//! egui's own painter blends with premultiplied alpha, which cannot produce
//! the hot, saturating cores that overlapping lights have when light is
//! *added* (canvas `globalCompositeOperation = 'lighter'`). On the GL
//! backend this module draws sprites in a small OpenGL pass through an
//! `egui_glow` paint callback with `SRC_ALPHA, ONE` blending: one soft radial
//! sprite each, shaped in the fragment shader.
//!
//! **Wakes.** With [`SpriteBlend::AdditiveWakes`] the sprites are accumulated
//! into an offscreen texture that is faded (not cleared) every frame and then
//! added onto the slide, so moving lights leave short trails. Two textures
//! ping-pong; if the framebuffer cannot be created the sprites are drawn
//! straight to the screen, crisp and without trails.
//!
//! **Without GL** (headless rendering, tests) the same sprites are drawn as
//! a textured mesh whose colours carry zero alpha, which premultiplied
//! blending turns into addition. Wakes need GL and are left out there.
//!
//! **Safety.** glow's calls are `unsafe` because they need a current GL
//! context and live GL object names. Every `unsafe fn` in this module and its
//! submodules ([`shaders`], [`objects`], [`passes`]) has one contract: call it
//! from inside the `egui_glow` paint callback (where egui has made its
//! context current), with objects created by [`objects::create_objects`] on
//! that same context. The callback is the only caller; each `SAFETY:` comment
//! points back to this contract ("the module contract").

use std::sync::{Arc, Mutex};

use objects::{GlObjects, create_objects};
use passes::{draw_sprites, draw_with_wakes};

use super::color::{Color, smoothstep};
use super::geom::{Pos2, Rect, ToEgui};
use super::shapes::{ImageData, Mesh, Texture, TextureFilter};

mod objects;
mod passes;
mod shaders;

/// One sprite: centre and diameter in points, and a linear RGBA colour
/// (0..1) whose alpha carries the sprite's brightness.
///
/// ```
/// use mdeck_sdk::paint::{Pos2, Sprite};
/// let s = Sprite { center: Pos2::new(100.0, 100.0), size: 12.0, rgba: [1.0, 0.5, 0.2, 0.8] };
/// assert_eq!(s.size, 12.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    /// Centre in points.
    pub center: Pos2,
    /// Diameter in points.
    pub size: f32,
    /// Red, green, blue (0..1, not premultiplied) and brightness (0..1).
    pub rgba: [f32; 4],
}

/// How sprites combine with what is below them.
///
/// ```
/// use mdeck_sdk::paint::SpriteBlend;
/// assert_ne!(SpriteBlend::Additive, SpriteBlend::Normal);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpriteBlend {
    /// Light accumulates: overlaps saturate toward white. For dark slides.
    Additive,
    /// Additive, and moving sprites leave short fading trails (GL only;
    /// elsewhere like [`SpriteBlend::Additive`]). Never use it for stills.
    AdditiveWakes,
    /// Normal (ink) blending. For light slides, where added light would
    /// vanish into the page.
    Normal,
}

/// The state behind a stream of sprite frames: GPU objects and wake
/// buffers on the GL backend, the sprite texture elsewhere. An engine keeps
/// one per layer it draws and passes it to [`crate::paint::Painter::sprites`]
/// every frame.
///
/// ```
/// let layer = mdeck_sdk::paint::SpriteLayer::new();
/// # drop(layer);
/// ```
#[derive(Default)]
pub struct SpriteLayer {
    objects: Arc<Mutex<Option<GlObjects>>>,
    texture: Mutex<Option<Texture>>,
}

impl SpriteLayer {
    /// A new layer; GPU objects are created on first use.
    ///
    /// See [`SpriteLayer`] for an example.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue the GL callback drawing `sprites` over `rect`.
    pub(crate) fn paint_gl(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        sprites: Vec<Sprite>,
        blend: SpriteBlend,
    ) {
        let wakes = blend == SpriteBlend::AdditiveWakes;
        let light = blend == SpriteBlend::Normal;
        if sprites.is_empty() && !wakes {
            return;
        }
        let objects = Arc::clone(&self.objects);
        let callback = egui::PaintCallback {
            rect: rect.eg(),
            callback: Arc::new(egui_glow::CallbackFn::new(move |info, gl_painter| {
                let gl = gl_painter.gl();
                let mut guard = objects.lock().unwrap_or_else(|p| p.into_inner());
                if guard.is_none() {
                    // SAFETY: inside the egui_glow callback, so `gl` is current.
                    *guard = unsafe { create_objects(gl) };
                }
                let Some(obj) = guard.as_mut() else {
                    return;
                };
                // SAFETY: inside the egui_glow callback, and `obj` was created
                // on this context above.
                unsafe {
                    if wakes {
                        draw_with_wakes(gl, obj, &info, &sprites, gl_painter.intermediate_fbo());
                    } else {
                        draw_sprites(gl, obj, &info, &sprites, light);
                    }
                }
            })),
        };
        painter.add(callback);
    }

    /// The sprites as one mesh, for backends without GL.
    pub(crate) fn mesh(&self, ctx: &egui::Context, sprites: &[Sprite], blend: SpriteBlend) -> Mesh {
        let texture = self
            .texture
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_or_insert_with(|| Texture {
                handle: ctx.load_texture(
                    "mdeck-sdk-sprite",
                    sprite_falloff().to_egui(),
                    TextureFilter::Linear.eg(),
                ),
            })
            .clone();
        sprite_mesh(texture, sprites, blend)
    }
}

/// Sprites as quads over the falloff texture. Additive sprites carry zero
/// alpha, which premultiplied blending turns into addition.
pub(crate) fn sprite_mesh(texture: Texture, sprites: &[Sprite], blend: SpriteBlend) -> Mesh {
    let mut mesh = Mesh::with_texture(texture);
    let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    for s in sprites {
        let [r, g, b, a] = s.rgba.map(|c| c.clamp(0.0, 1.0));
        if a <= 0.002 {
            continue;
        }
        let q = |c: f32| (c * a * 255.0).round() as u8;
        let alpha = if blend == SpriteBlend::Normal {
            (a * 255.0).round() as u8
        } else {
            0
        };
        let color = Color::from_rgba_premultiplied(q(r), q(g), q(b), alpha);
        let half = s.size * 0.5;
        let rect = Rect::from_min_max(
            Pos2::new(s.center.x - half, s.center.y - half),
            Pos2::new(s.center.x + half, s.center.y + half),
        );
        mesh.add_rect_uv(rect, full, color);
    }
    mesh
}

/// The sprite fragment shader's falloff as a white premultiplied texture:
/// full at the centre, half by 30% of the radius, clear at the edge.
pub(crate) fn sprite_falloff() -> ImageData {
    const N: usize = 64;
    let mut pixels = Vec::with_capacity(N * N);
    for y in 0..N {
        for x in 0..N {
            let u = (x as f32 + 0.5) / N as f32 * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / N as f32 * 2.0 - 1.0;
            let d = (u * u + v * v).sqrt();
            let a = if d > 1.0 {
                0.0
            } else {
                let core = 1.0 - smoothstep(0.0, 0.36, d);
                let halo = 1.0 - smoothstep(0.30, 1.0, d);
                (core * 0.75 + halo * 0.55).min(1.0)
            };
            let v8 = (a * 255.0).round() as u8;
            pixels.push(Color::from_rgba_premultiplied(v8, v8, v8, v8));
        }
    }
    ImageData {
        size: [N, N],
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tex() -> Texture {
        let ctx = egui::Context::default();
        Texture {
            handle: ctx.load_texture("t", sprite_falloff().to_egui(), Default::default()),
        }
    }

    #[test]
    fn additive_sprites_carry_zero_alpha_and_normal_ones_do_not() {
        let s = Sprite {
            center: Pos2::new(10.0, 10.0),
            size: 4.0,
            rgba: [1.0, 0.5, 0.0, 0.5],
        };
        let add = sprite_mesh(tex(), &[s], SpriteBlend::Additive);
        assert_eq!(add.vertices.len(), 4);
        assert!(add.vertices.iter().all(|v| v.color.a() == 0));
        assert_eq!(add.vertices[0].color.r(), 128);
        let normal = sprite_mesh(tex(), &[s], SpriteBlend::Normal);
        assert!(normal.vertices.iter().all(|v| v.color.a() == 128));
        assert_eq!(normal.vertices[0].pos, Pos2::new(8.0, 8.0));
    }

    #[test]
    fn invisible_sprites_are_skipped() {
        let s = Sprite {
            center: Pos2::ZERO,
            size: 4.0,
            rgba: [1.0, 1.0, 1.0, 0.0],
        };
        assert!(sprite_mesh(tex(), &[s], SpriteBlend::Additive).is_empty());
    }

    #[test]
    fn falloff_is_bright_at_the_centre_and_clear_outside() {
        let f = sprite_falloff();
        assert!(f.get(32, 32).unwrap().a() > 240);
        assert_eq!(f.get(0, 0).unwrap().a(), 0);
    }
}
