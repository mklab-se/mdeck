//! Paint helpers the engines share: colour mixing and blending for egui
//! meshes, `smoothstep`, and one small sprite sheet (a lens, a core and a
//! glow) that most engines draw their light with.

#![cfg_attr(
    not(all_engines),
    allow(dead_code, reason = "shared by several engines, each using some of it")
)]

use eframe::egui::{self, Color32, Pos2, Rect};

/// `a` toward `b` by `t` (0..1), opaque.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// `c` at opacity `a`, premultiplied (normal blending).
pub fn premul(c: Color32, a: f32) -> Color32 {
    let a = a.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        (c.r() as f32 * a) as u8,
        (c.g() as f32 * a) as u8,
        (c.b() as f32 * a) as u8,
        (a * 255.0) as u8,
    )
}

/// `c` scaled by `k` with zero alpha: added onto what is below (glow).
pub fn additive(c: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    Color32::from_rgba_premultiplied(
        (c.r() as f32 * k) as u8,
        (c.g() as f32 * k) as u8,
        (c.b() as f32 * k) as u8,
        0,
    )
}

/// 0 below `a`, 1 above `b`, and a smooth S between.
pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Sprite sheet: lens, core and glow side by side, white, premultiplied.
const SPRITE: usize = 64;
pub const SPRITE_LENS: Rect = Rect {
    min: Pos2 { x: 0.0, y: 0.0 },
    max: Pos2 {
        x: 1.0 / 3.0,
        y: 1.0,
    },
};
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
pub const SPRITE_GLOW: Rect = Rect {
    min: Pos2 {
        x: 2.0 / 3.0,
        y: 0.0,
    },
    max: Pos2 { x: 1.0, y: 1.0 },
};

pub fn sprite_sheet() -> egui::ColorImage {
    let w = SPRITE * 3;
    let mut pixels = vec![Color32::TRANSPARENT; w * SPRITE];
    for y in 0..SPRITE {
        for x in 0..SPRITE {
            let dx = (x as f32 + 0.5) / SPRITE as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / SPRITE as f32 * 2.0 - 1.0;
            let r = (dx * dx + dy * dy).sqrt();
            // lens: a flat disc with a faint rim, lit from the top left
            let edge = 1.0 - smoothstep(0.82, 1.0, r);
            let rim = smoothstep(0.55, 0.9, r) * 0.5 + 0.5;
            let light = 1.0 - 0.18 * (dx + dy).max(-1.0);
            let lens = (edge * rim * light).clamp(0.0, 1.0);
            // core: a bright, slightly soft dome
            let core = (1.0 - smoothstep(0.55, 1.0, r)) * (1.0 - 0.25 * r * r);
            // glow: a wide soft falloff
            let glow = (-(r * r) * 5.5).exp() * (1.0 - smoothstep(0.7, 1.0, r));
            for (k, v) in [lens, core, glow].into_iter().enumerate() {
                let v8 = (v.clamp(0.0, 1.0) * 255.0) as u8;
                pixels[y * w + k * SPRITE + x] = Color32::from_rgba_premultiplied(v8, v8, v8, v8);
            }
        }
    }
    egui::ColorImage::new([w, SPRITE], pixels)
}

/// The sprite sheet as a texture, uploaded on first use.
pub struct Sprites {
    name: &'static str,
    texture: Option<egui::TextureHandle>,
}

impl Sprites {
    /// `name`: the texture's name in egui.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            texture: None,
        }
    }

    pub fn id(&mut self, ctx: &egui::Context) -> egui::TextureId {
        self.texture
            .get_or_insert_with(|| {
                ctx.load_texture(self.name, sprite_sheet(), egui::TextureOptions::LINEAR)
            })
            .id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_is_clamped_and_symmetric() {
        assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
        assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
        assert_eq!(smoothstep(0.0, 1.0, 0.3), 0.3 * 0.3 * (3.0 - 2.0 * 0.3));
    }

    #[test]
    fn premultiplied_and_additive_colours() {
        let c = Color32::from_rgb(200, 100, 50);
        assert_eq!(premul(c, 1.0), c);
        assert_eq!(premul(c, 0.0), Color32::TRANSPARENT);
        assert_eq!(additive(c, 0.5).a(), 0);
        assert_eq!(mix(Color32::BLACK, Color32::WHITE, 1.0), Color32::WHITE);
    }
}
