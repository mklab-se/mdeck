//! Additive glow renderer for the particle field.
//!
//! egui's own painter blends with premultiplied alpha, which cannot produce
//! the hot, saturating cores that overlapping lights have on the MKLab site
//! (canvas `globalCompositeOperation = 'lighter'`). This module draws the
//! particles in a tiny OpenGL pass through an `egui_glow` paint callback with
//! `SRC_ALPHA, ONE` blending: one soft radial sprite per particle, shaped in
//! the fragment shader to match the site's pre-rendered gradient sprites.

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

/// GPU objects, created lazily on first paint (the GL context only exists
/// inside the callback) and shared between frames.
struct GlObjects {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    u_screen: Option<glow::UniformLocation>,
}

#[derive(Default)]
pub struct GlowRenderer {
    objects: Arc<Mutex<Option<GlObjects>>>,
}

const VERT: &str = r#"
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
const FRAG: &str = r#"
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

impl GlowRenderer {
    /// Queue a paint callback drawing `sprites` over `rect`.
    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, sprites: Vec<Sprite>) {
        if sprites.is_empty() {
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
                let Some(obj) = guard.as_ref() else {
                    return;
                };
                unsafe { draw(gl, obj, &info, &sprites) };
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
    let es = lang.contains("ES");
    if es {
        (
            "#version 100\nprecision mediump float;\n".to_string(),
            "#version 100\nprecision mediump float;\n".to_string(),
        )
    } else {
        (
            "#version 140\n#define attribute in\n#define varying out\n".to_string(),
            "#version 140\n#define varying in\nout vec4 mdeck_frag;\n#define gl_FragColor mdeck_frag\n".to_string(),
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

unsafe fn create_objects(gl: &glow::Context) -> Option<GlObjects> {
    unsafe {
        let (vp, fp) = shader_prefixes(gl);
        let vs = compile(gl, glow::VERTEX_SHADER, &format!("{vp}{VERT}"))?;
        let fs = compile(gl, glow::FRAGMENT_SHADER, &format!("{fp}{FRAG}"))?;
        let program = gl.create_program().ok()?;
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        gl.bind_attrib_location(program, 0, "a_pos");
        gl.bind_attrib_location(program, 1, "a_uv");
        gl.bind_attrib_location(program, 2, "a_color");
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
        let u_screen = gl.get_uniform_location(program, "u_screen");
        let vao = gl.create_vertex_array().ok()?;
        let vbo = gl.create_buffer().ok()?;
        Some(GlObjects {
            program,
            vao,
            vbo,
            u_screen,
        })
    }
}

/// Floats per vertex: pos(2) + uv(2) + color(4).
const STRIDE: usize = 8;

unsafe fn draw(
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
        gl.use_program(Some(obj.program));
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

        gl.bind_vertex_array(Some(obj.vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(obj.vbo));
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
