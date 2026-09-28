//! GPU objects: the sprite and fade programs with their buffers, created
//! lazily on the first paint, and the ping-pong wake buffers.
//!
//! **Safety.** Every `unsafe fn` here follows the contract in the
//! [parent module](super): call it inside the `egui_glow` paint callback with
//! that callback's current context and objects created on it.

use eframe::glow::{self, HasContext};

use super::shaders::{QUAD_FRAG, QUAD_VERT, SPRITE_FRAG, SPRITE_VERT, link};

/// GPU objects, created lazily on first paint (the GL context only exists
/// inside the callback) and shared between frames.
pub(super) struct GlObjects {
    pub(super) sprites: glow::Program,
    pub(super) sprites_vao: glow::VertexArray,
    pub(super) sprites_vbo: glow::Buffer,
    pub(super) u_screen: Option<glow::UniformLocation>,
    pub(super) quad: glow::Program,
    pub(super) quad_vao: glow::VertexArray,
    pub(super) quad_vbo: glow::Buffer,
    pub(super) u_tex: Option<glow::UniformLocation>,
    pub(super) u_decay: Option<glow::UniformLocation>,
    pub(super) u_cut: Option<glow::UniformLocation>,
    /// Wake buffers: framebuffer, colour texture, size in pixels. `None` when
    /// offscreen rendering is unavailable.
    pub(super) wake: Option<WakeBuffers>,
    pub(super) wake_failed: bool,
}

pub(super) struct WakeBuffers {
    pub(super) fbo: [glow::Framebuffer; 2],
    pub(super) tex: [glow::Texture; 2],
    pub(super) size: (i32, i32),
    /// Which buffer holds the previous frame.
    pub(super) prev: usize,
}

pub(super) unsafe fn create_objects(gl: &glow::Context) -> Option<GlObjects> {
    // SAFETY: the caller upholds the module contract (current context).
    unsafe {
        let sprites = link(gl, SPRITE_VERT, SPRITE_FRAG, &["a_pos", "a_uv", "a_color"])?;
        let u_screen = gl.get_uniform_location(sprites, "u_screen");
        let sprites_vao = gl.create_vertex_array().ok()?;
        let sprites_vbo = gl.create_buffer().ok()?;

        let quad = link(gl, QUAD_VERT, QUAD_FRAG, &["a_pos", "a_uv"])?;
        let u_tex = gl.get_uniform_location(quad, "u_tex");
        let u_decay = gl.get_uniform_location(quad, "u_decay");
        let u_cut = gl.get_uniform_location(quad, "u_cut");
        let quad_vao = gl.create_vertex_array().ok()?;
        let quad_vbo = gl.create_buffer().ok()?;
        // full-screen quad: pos.xy, uv.xy
        let verts: [f32; 24] = [
            -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 0.0, 0.0,
            1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 1.0,
        ];
        gl.bind_vertex_array(Some(quad_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(quad_vbo));
        // SAFETY: `verts` is a live, initialised `[f32; 24]`; viewing its
        // bytes as `u8` for exactly `size_of_val` bytes is always valid.
        let bytes: &[u8] =
            std::slice::from_raw_parts(verts.as_ptr() as *const u8, std::mem::size_of_val(&verts));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        gl.bind_vertex_array(None);

        Some(GlObjects {
            sprites,
            sprites_vao,
            sprites_vbo,
            u_screen,
            quad,
            quad_vao,
            quad_vbo,
            u_tex,
            u_decay,
            u_cut,
            wake: None,
            wake_failed: false,
        })
    }
}

/// Create (or resize) the two wake buffers for a viewport of `w × h` pixels.
pub(super) unsafe fn ensure_wake(gl: &glow::Context, obj: &mut GlObjects, w: i32, h: i32) -> bool {
    // SAFETY: the caller upholds the module contract (current context).
    unsafe {
        if obj.wake_failed || w <= 0 || h <= 0 {
            return false;
        }
        if let Some(wk) = &obj.wake
            && wk.size == (w, h)
        {
            return true;
        }
        if let Some(old) = obj.wake.take() {
            for f in old.fbo {
                gl.delete_framebuffer(f);
            }
            for t in old.tex {
                gl.delete_texture(t);
            }
        }
        let mut fbo = Vec::new();
        let mut tex = Vec::new();
        for _ in 0..2 {
            let (t, f) = match (gl.create_texture(), gl.create_framebuffer()) {
                (Ok(t), Ok(f)) => (t, f),
                (t, f) => {
                    // Do not leak whichever half (or earlier pair) succeeded.
                    t.into_iter()
                        .chain(tex.drain(..))
                        .for_each(|t| gl.delete_texture(t));
                    f.into_iter()
                        .chain(fbo.drain(..))
                        .for_each(|f| gl.delete_framebuffer(f));
                    obj.wake_failed = true;
                    return false;
                }
            };
            let complete = attach_wake_texture(gl, t, f, w, h);
            if !complete {
                obj.wake_failed = true;
                eprintln!("mdeck: wake framebuffer incomplete; drawing the field without trails");
                return false;
            }
            fbo.push(f);
            tex.push(t);
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
        obj.wake = Some(WakeBuffers {
            fbo: [fbo[0], fbo[1]],
            tex: [tex[0], tex[1]],
            size: (w, h),
            prev: 0,
        });
        true
    }
}

/// Give texture `t` a `w × h` RGBA8 image, attach it to framebuffer `f`
/// (which stays bound) and clear it to black. Returns whether `f` is complete.
unsafe fn attach_wake_texture(
    gl: &glow::Context,
    t: glow::Texture,
    f: glow::Framebuffer,
    w: i32,
    h: i32,
) -> bool {
    // SAFETY: the caller upholds the module contract (current context), and
    // `t` and `f` were just created on it.
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(t));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            w,
            h,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(f));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(t),
            0,
        );
        let complete = gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        complete
    }
}
