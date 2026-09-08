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

use std::sync::{Arc, Mutex};

use eframe::egui;
use eframe::egui_glow;
use eframe::glow::{self, HasContext};

/// One sprite to draw: centre in points, diameter in points, RGBA with alpha
/// already carrying the particle's brightness.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub rgba: [f32; 4],
}

/// How much of the previous frame survives into this one (per 60 Hz frame).
const WAKE_DECAY: f32 = 0.60;
/// Subtracted after decay so 8-bit residue never leaves permanent ghosts.
const WAKE_CUT: f32 = 0.012;

/// GPU objects, created lazily on first paint (the GL context only exists
/// inside the callback) and shared between frames.
struct GlObjects {
    sprites: glow::Program,
    sprites_vao: glow::VertexArray,
    sprites_vbo: glow::Buffer,
    u_screen: Option<glow::UniformLocation>,
    quad: glow::Program,
    quad_vao: glow::VertexArray,
    quad_vbo: glow::Buffer,
    u_tex: Option<glow::UniformLocation>,
    u_decay: Option<glow::UniformLocation>,
    u_cut: Option<glow::UniformLocation>,
    /// Wake buffers: framebuffer, colour texture, size in pixels. `None` when
    /// offscreen rendering is unavailable.
    wake: Option<WakeBuffers>,
    wake_failed: bool,
}

struct WakeBuffers {
    fbo: [glow::Framebuffer; 2],
    tex: [glow::Texture; 2],
    size: (i32, i32),
    /// Which buffer holds the previous frame.
    prev: usize,
}

#[derive(Default)]
pub struct GlowRenderer {
    objects: Arc<Mutex<Option<GlObjects>>>,
}

const SPRITE_VERT: &str = r#"
    uniform vec2 u_screen;      // viewport size in points
    attribute vec2 a_pos;       // sprite corner in points
    attribute vec2 a_uv;        // -1..1 across the sprite
    attribute vec4 a_color;
    varying vec2 v_uv;
    varying vec4 v_color;
    void main() {
        vec2 ndc = vec2(a_pos.x / u_screen.x * 2.0 - 1.0, 1.0 - a_pos.y / u_screen.y * 2.0);
        gl_Position = vec4(ndc, 0.0, 1.0);
        v_uv = a_uv;
        v_color = a_color;
    }
"#;

// Matches the site's sprite: full colour at the centre, half at 30% radius,
// transparent at the edge, with a smooth (not linear) falloff.
const SPRITE_FRAG: &str = r#"
    varying vec2 v_uv;
    varying vec4 v_color;
    void main() {
        float d = length(v_uv);
        if (d > 1.0) { discard; }
        float core = 1.0 - smoothstep(0.0, 0.36, d);
        float halo = 1.0 - smoothstep(0.30, 1.0, d);
        float a = core * 0.75 + halo * 0.55;
        gl_FragColor = vec4(v_color.rgb, v_color.a * a);
    }
"#;

const QUAD_VERT: &str = r#"
    attribute vec2 a_pos;       // NDC
    attribute vec2 a_uv;
    varying vec2 v_uv;
    void main() {
        gl_Position = vec4(a_pos, 0.0, 1.0);
        v_uv = a_uv;
    }
"#;

const QUAD_FRAG: &str = r#"
    uniform sampler2D u_tex;
    uniform float u_decay;
    uniform float u_cut;
    varying vec2 v_uv;
    void main() {
        vec3 c = texture2D(u_tex, v_uv).rgb;
        c = max(c * u_decay - vec3(u_cut), vec3(0.0));
        gl_FragColor = vec4(c, 1.0);
    }
"#;

impl GlowRenderer {
    /// Queue a paint callback drawing `sprites` over `rect`, with wakes when
    /// `wakes` is set (never for stills).
    pub fn paint(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        sprites: Vec<Sprite>,
        wakes: bool,
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
                    *guard = unsafe { create_objects(gl) };
                }
                let Some(obj) = guard.as_mut() else {
                    return;
                };
                unsafe {
                    if wakes {
                        draw_with_wakes(gl, obj, &info, &sprites, gl_painter.intermediate_fbo());
                    } else {
                        draw_sprites(gl, obj, &info, &sprites);
                    }
                }
            })),
        };
        painter.add(callback);
    }
}

/// GLSL prefixes for this context. Desktop GL 3.2+ core profiles (macOS)
/// reject the legacy `attribute`/`varying` keywords and `#version 120`, so
/// the shaders are written in the legacy dialect and mapped with macros.
fn shader_prefixes(gl: &glow::Context) -> (String, String) {
    let lang = unsafe { gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION) };
    if lang.contains("ES") {
        (
            "#version 100\nprecision mediump float;\n".to_string(),
            "#version 100\nprecision mediump float;\n".to_string(),
        )
    } else {
        (
            "#version 140\n#define attribute in\n#define varying out\n".to_string(),
            "#version 140\n#define varying in\n#define texture2D texture\nout vec4 mdeck_frag;\n#define gl_FragColor mdeck_frag\n".to_string(),
        )
    }
}

unsafe fn compile(gl: &glow::Context, kind: u32, src: &str) -> Option<glow::Shader> {
    unsafe {
        let shader = gl.create_shader(kind).ok()?;
        gl.shader_source(shader, src);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            eprintln!(
                "mdeck: particle shader failed to compile: {}",
                gl.get_shader_info_log(shader)
            );
            gl.delete_shader(shader);
            return None;
        }
        Some(shader)
    }
}

unsafe fn link(
    gl: &glow::Context,
    vs_src: &str,
    fs_src: &str,
    attribs: &[&str],
) -> Option<glow::Program> {
    unsafe {
        let (vp, fp) = shader_prefixes(gl);
        let vs = compile(gl, glow::VERTEX_SHADER, &format!("{vp}{vs_src}"))?;
        let fs = compile(gl, glow::FRAGMENT_SHADER, &format!("{fp}{fs_src}"))?;
        let program = gl.create_program().ok()?;
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        for (i, name) in attribs.iter().enumerate() {
            gl.bind_attrib_location(program, i as u32, name);
        }
        gl.link_program(program);
        gl.detach_shader(program, vs);
        gl.detach_shader(program, fs);
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        if !gl.get_program_link_status(program) {
            eprintln!(
                "mdeck: particle shader failed to link: {}",
                gl.get_program_info_log(program)
            );
            gl.delete_program(program);
            return None;
        }
        Some(program)
    }
}

unsafe fn create_objects(gl: &glow::Context) -> Option<GlObjects> {
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
unsafe fn ensure_wake(gl: &glow::Context, obj: &mut GlObjects, w: i32, h: i32) -> bool {
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
            let (Ok(t), Ok(f)) = (gl.create_texture(), gl.create_framebuffer()) else {
                obj.wake_failed = true;
                return false;
            };
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
            let complete =
                gl.check_framebuffer_status(glow::FRAMEBUFFER) == glow::FRAMEBUFFER_COMPLETE;
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
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

/// Floats per sprite vertex: pos(2) + uv(2) + color(4).
const STRIDE: usize = 8;

/// Upload and draw the sprites additively into whatever framebuffer is bound,
/// mapping points to the viewport `info` describes.
unsafe fn draw_sprites(
    gl: &glow::Context,
    obj: &GlObjects,
    info: &egui::PaintCallbackInfo,
    sprites: &[Sprite],
) {
    // Six vertices (two triangles) per sprite.
    let mut data: Vec<f32> = Vec::with_capacity(sprites.len() * 6 * STRIDE);
    let origin = info.viewport.min;
    for s in sprites {
        if s.rgba[3] <= 0.002 {
            continue;
        }
        let h = s.size * 0.5;
        let (x, y) = (s.x - origin.x, s.y - origin.y);
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

    unsafe {
        gl.use_program(Some(obj.sprites));
        gl.uniform_2_f32(
            obj.u_screen.as_ref(),
            info.viewport.width(),
            info.viewport.height(),
        );

        gl.enable(glow::BLEND);
        gl.blend_equation(glow::FUNC_ADD);
        // Additive: light accumulates, overlaps saturate toward white.
        gl.blend_func(glow::SRC_ALPHA, glow::ONE);
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
unsafe fn draw_with_wakes(
    gl: &glow::Context,
    obj: &mut GlObjects,
    info: &egui::PaintCallbackInfo,
    sprites: &[Sprite],
    screen_fbo: Option<glow::Framebuffer>,
) {
    unsafe {
        let vp = info.viewport_in_pixels();
        if !ensure_wake(gl, obj, vp.width_px, vp.height_px) {
            draw_sprites(gl, obj, info, sprites);
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
        draw_sprites(gl, obj, info, sprites);

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
