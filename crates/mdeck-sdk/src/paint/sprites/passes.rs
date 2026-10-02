//! The draw passes: sprites into whatever framebuffer is bound, the
//! full-screen fade quad, and the wake sequence that combines them.
//!
//! **Safety.** Every `unsafe fn` here follows the contract in the
//! [parent module](super): call it inside the `egui_glow` paint callback with
//! that callback's current context and objects created on it.

use glow::{self, HasContext};

use super::Sprite;
use super::objects::{GlObjects, ensure_wake};

/// How much of the previous frame survives into this one (per 60 Hz frame).
const WAKE_DECAY: f32 = 0.60;
/// Subtracted after decay so 8-bit residue never leaves permanent ghosts.
const WAKE_CUT: f32 = 0.012;

/// Floats per sprite vertex: pos(2) + uv(2) + color(4).
const STRIDE: usize = 8;

/// Upload and draw the sprites additively into whatever framebuffer is bound,
/// mapping points to the viewport `info` describes.
pub(super) unsafe fn draw_sprites(
    gl: &glow::Context,
    obj: &GlObjects,
    info: &egui::PaintCallbackInfo,
    sprites: &[Sprite],
    light: bool,
) {
    // Six vertices (two triangles) per sprite.
    let mut data: Vec<f32> = Vec::with_capacity(sprites.len() * 6 * STRIDE);
    let origin = info.viewport.min;
    for s in sprites {
        if s.rgba[3] <= 0.002 {
            continue;
        }
        let h = s.size * 0.5;
        let (x, y) = (s.center.x - origin.x, s.center.y - origin.y);
        let corners = [
            (x - h, y - h, -1.0, -1.0),
            (x + h, y - h, 1.0, -1.0),
            (x + h, y + h, 1.0, 1.0),
            (x - h, y - h, -1.0, -1.0),
            (x + h, y + h, 1.0, 1.0),
            (x - h, y + h, -1.0, 1.0),
        ];
        for (px, py, u, v) in corners {
            data.extend_from_slice(&[px, py, u, v, s.rgba[0], s.rgba[1], s.rgba[2], s.rgba[3]]);
        }
    }
    if data.is_empty() {
        return;
    }

    // SAFETY: the caller upholds the module contract (current context).
    unsafe {
        gl.use_program(Some(obj.sprites));
        gl.uniform_2_f32(
            obj.u_screen.as_ref(),
            info.viewport.width(),
            info.viewport.height(),
        );

        gl.enable(glow::BLEND);
        gl.blend_equation(glow::FUNC_ADD);
        // Additive: light accumulates, overlaps saturate toward white. On a
        // light page, ink instead: normal blending.
        if light {
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        } else {
            gl.blend_func(glow::SRC_ALPHA, glow::ONE);
        }
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::CULL_FACE);

        gl.bind_vertex_array(Some(obj.sprites_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(obj.sprites_vbo));
        let bytes: &[u8] = std::slice::from_raw_parts(
            data.as_ptr() as *const u8,
            data.len() * std::mem::size_of::<f32>(),
        );
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STREAM_DRAW);

        let stride = (STRIDE * std::mem::size_of::<f32>()) as i32;
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);
        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_pointer_f32(2, 4, glow::FLOAT, false, stride, 16);

        gl.draw_arrays(glow::TRIANGLES, 0, (data.len() / STRIDE) as i32);

        gl.disable_vertex_attrib_array(0);
        gl.disable_vertex_attrib_array(1);
        gl.disable_vertex_attrib_array(2);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        gl.bind_vertex_array(None);
        gl.use_program(None);
    }
}

/// Draw the full-screen quad sampling `tex` with the given decay and cut.
unsafe fn draw_quad(gl: &glow::Context, obj: &GlObjects, tex: glow::Texture, decay: f32, cut: f32) {
    // SAFETY: the caller upholds the module contract (current context).
    unsafe {
        gl.use_program(Some(obj.quad));
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.uniform_1_i32(obj.u_tex.as_ref(), 0);
        gl.uniform_1_f32(obj.u_decay.as_ref(), decay);
        gl.uniform_1_f32(obj.u_cut.as_ref(), cut);
        gl.bind_vertex_array(Some(obj.quad_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(obj.quad_vbo));
        let stride = (4 * std::mem::size_of::<f32>()) as i32;
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);
        gl.draw_arrays(glow::TRIANGLES, 0, 6);
        gl.disable_vertex_attrib_array(0);
        gl.disable_vertex_attrib_array(1);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        gl.bind_vertex_array(None);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.use_program(None);
    }
}

/// Fade last frame's light into the current wake buffer, add this frame's
/// sprites, then add the buffer onto the slide.
pub(super) unsafe fn draw_with_wakes(
    gl: &glow::Context,
    obj: &mut GlObjects,
    info: &egui::PaintCallbackInfo,
    sprites: &[Sprite],
    screen_fbo: Option<glow::Framebuffer>,
) {
    // SAFETY: the caller upholds the module contract (current context).
    unsafe {
        let vp = info.viewport_in_pixels();
        if !ensure_wake(gl, obj, vp.width_px, vp.height_px) {
            draw_sprites(gl, obj, info, sprites, false);
            return;
        }
        let (fbo, tex, prev) = {
            let wk = obj.wake.as_ref().expect("ensured");
            (wk.fbo, wk.tex, wk.prev)
        };
        let cur = 1 - prev;

        // 1. into the current buffer: faded previous frame, then new sprites
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo[cur]));
        gl.viewport(0, 0, vp.width_px, vp.height_px);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        draw_quad(gl, obj, tex[prev], WAKE_DECAY, WAKE_CUT);
        draw_sprites(gl, obj, info, sprites, false);

        // 2. back to the screen: add the buffer over the slide background
        gl.bind_framebuffer(glow::FRAMEBUFFER, screen_fbo);
        gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
        gl.enable(glow::BLEND);
        gl.blend_equation(glow::FUNC_ADD);
        gl.blend_func(glow::ONE, glow::ONE);
        draw_quad(gl, obj, tex[cur], 1.0, 0.0);

        if let Some(wk) = obj.wake.as_mut() {
            wk.prev = cur;
        }
    }
}
