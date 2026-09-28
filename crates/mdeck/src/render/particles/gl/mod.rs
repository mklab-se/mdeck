//! Additive glow renderer for the particle field, with wakes.
//!
//! egui's own painter blends with premultiplied alpha, which cannot produce
//! the hot, saturating cores that overlapping lights have on the MKLab site
//! (canvas `globalCompositeOperation = 'lighter'`). This module draws the
//! particles in a small OpenGL pass through an `egui_glow` paint callback
//! with `SRC_ALPHA, ONE` blending: one soft radial sprite per particle,
//! shaped in the fragment shader to match the site's gradient sprites.
//!
//! **Wakes.** The site never clears its canvas fully; it paints 45% black
//! over the previous frame, so moving lights leave short trails. Here the
//! sprites are accumulated into an offscreen texture that is faded (not
//! cleared) every frame and then added onto the slide. Two textures
//! ping-pong; if the framebuffer cannot be created the sprites are drawn
//! straight to the screen, crisp and without trails.
//!
//! **Safety.** glow's calls are `unsafe` because they need a current GL
//! context and live GL object names. Every `unsafe fn` in this module and its
//! submodules ([`shaders`], [`objects`], [`passes`]) has one contract: call it
//! from inside the `egui_glow` paint callback (where egui has made its
//! context current), with objects created by [`objects::create_objects`] on
//! that same context. The callback is the only caller; each `SAFETY:` comment
//! points back to this contract ("the module contract").

use std::sync::{Arc, Mutex};

use eframe::egui;
use eframe::egui_glow;

use objects::{GlObjects, create_objects};
use passes::{draw_sprites, draw_with_wakes};

mod objects;
mod passes;
mod shaders;

/// One sprite to draw: centre in points, diameter in points, RGBA with alpha
/// already carrying the particle's brightness.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub rgba: [f32; 4],
}

#[derive(Default)]
pub struct GlowRenderer {
    objects: Arc<Mutex<Option<GlObjects>>>,
}

impl GlowRenderer {
    /// Queue a paint callback drawing `sprites` over `rect`, with wakes when
    /// `wakes` is set (never for stills). On a light background (`light`)
    /// the sprites blend normally instead of adding light, which would vanish
    /// into the page.
    pub fn paint(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        sprites: Vec<Sprite>,
        wakes: bool,
        light: bool,
    ) {
        if sprites.is_empty() && !wakes {
            return;
        }
        let objects = Arc::clone(&self.objects);
        let callback = egui::PaintCallback {
            rect,
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
                    if wakes && !light {
                        draw_with_wakes(gl, obj, &info, &sprites, gl_painter.intermediate_fbo());
                    } else {
                        draw_sprites(gl, obj, &info, &sprites, light);
                    }
                }
            })),
        };
        painter.add(callback);
    }
}
