//! The GLSL sources for the sprite and fade passes, and compiling them
//! into programs for whichever GL dialect the context speaks.
//!
//! **Safety.** Every `unsafe fn` here follows the contract in the
//! [parent module](super): call it inside the `egui_glow` paint callback with
//! that callback's current context and objects created on it.

use glow::{self, HasContext};

pub(super) const SPRITE_VERT: &str = r#"
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
pub(super) const SPRITE_FRAG: &str = r#"
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

pub(super) const QUAD_VERT: &str = r#"
    attribute vec2 a_pos;       // NDC
    attribute vec2 a_uv;
    varying vec2 v_uv;
    void main() {
        gl_Position = vec4(a_pos, 0.0, 1.0);
        v_uv = a_uv;
    }
"#;

pub(super) const QUAD_FRAG: &str = r#"
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

/// GLSL prefixes for this context. Desktop GL 3.2+ core profiles (macOS)
/// reject the legacy `attribute`/`varying` keywords and `#version 120`, so
/// the shaders are written in the legacy dialect and mapped with macros.
///
/// # Safety
/// See the module docs: call with the callback's current context.
unsafe fn shader_prefixes(gl: &glow::Context) -> (String, String) {
    // SAFETY: the caller holds the current context (module contract).
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
    // SAFETY: the caller upholds the module contract (current context).
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

pub(super) unsafe fn link(
    gl: &glow::Context,
    vs_src: &str,
    fs_src: &str,
    attribs: &[&str],
) -> Option<glow::Program> {
    // SAFETY: the caller upholds the module contract (current context).
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
