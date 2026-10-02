//! Painting the wall: what each LED shows this frame (its level after
//! flicker, shimmer, the ambient aurora, a reveal sweep, the burst and the
//! marquee), then one mesh of lenses, bloom, glow and cores.

use mdeck_sdk::paint::{
    Color as Color32, Mesh, Pos2, Rect, SPRITE_CORE, SPRITE_GLOW, SPRITE_LENS, Texture, Vec2,
    additive, mix, premul,
};
use mdeck_sdk::tokens::Tokens;

use super::light::border_positions;
use super::{Grid, Led};
use crate::engines::hash01;

/// A lit LED: its index, level and colour.
type Lit = (usize, f32, Color32);

impl Led {
    /// Every LED bright enough to draw this frame. `centre`: where the
    /// countdown's burst starts.
    pub(super) fn shade(
        &self,
        grid: &Grid,
        palette: &Palette,
        sweep_x: Option<f32>,
        centre: Pos2,
    ) -> Vec<Lit> {
        let n = grid.len();
        let t = self.time;
        let mut lit: Vec<Lit> = Vec::with_capacity(n / 3);
        let border_ring = self.border.then(|| border_positions(grid));
        let reach = ((grid.cols * grid.cols + grid.rows * grid.rows) as f32).sqrt() * 0.5;
        for i in 0..n {
            let (c, r) = ((i % grid.cols) as f32, (i / grid.cols) as f32);
            let seed = hash01(i as u32);
            let mut level = self.level[i];
            if self.flick[i] > 0.0 {
                let step = ((self.flick[i] * 55.0) as i32 + (seed * 7.0) as i32) % 3;
                level *= if step == 0 { 0.15 } else { 1.0 };
            }
            // lit LEDs scintillate a little, like real ones
            level *= 0.93 + 0.07 * (t * 2.3 + seed * 40.0).sin();
            let mut hue = self.hue[i] + 0.10 * (t * 0.55 + c * 0.05 - r * 0.03).sin();
            let mut hot_ring = 0.0;

            // ambient aurora in the calm zone's complement
            let z = self.zone[i];
            if self.ambient > 0.0 && z > 0.0 {
                let a =
                    0.5 + 0.5 * (c * 0.085 + t * 0.32 + 1.6 * (r * 0.07 - t * 0.19).sin()).sin();
                let b = 0.5 + 0.5 * (r * 0.11 - t * 0.27 + 1.3 * (c * 0.05 + t * 0.13).sin()).sin();
                let amb = self.ambient * z * (a.powi(5) * 0.8 + b.powi(7) * 0.6);
                if amb > level * 0.5 {
                    hue = 0.15 + 0.7 * b;
                }
                level = level.max(amb + level * 0.5);
            }
            // a reveal: one bright band of light passes left to right
            if let Some(x) = sweep_x {
                let pos = x * (grid.cols as f32 + 12.0) - 6.0;
                let band = (-((c - pos) / 2.2).powi(2)).exp() * 0.30 * z.max(0.25);
                level = level.max(band);
            }
            // the countdown's burst: a white-hot ring runs out and takes everything with it
            if let Some(p) = self.burst {
                let (cx0, cy0) = grid.cell_of(centre);
                let d = ((c - cx0).powi(2) + (r - cy0).powi(2)).sqrt();
                let radius = p * reach * 1.25;
                let ring = (-((d - radius) / 3.5).powi(2)).exp();
                level = level * (1.0 - p).max(0.0) + ring * (1.0 - p * 0.6);
                if ring > 0.3 {
                    hue = 0.9;
                    hot_ring = ring;
                }
            }
            let mut color = palette.at(hue.clamp(0.0, 1.0));
            if hot_ring > 0.0 {
                color = mix(color, palette.white, hot_ring * 0.6);
            }
            if let Some(ring) = &border_ring
                && let Some(k) = ring.get(&i)
            {
                // a chasing marquee: every third bulb dark, running clockwise
                let phase = ((*k as f32) - t * 9.0).rem_euclid(3.0);
                let on = if phase < 2.0 { 0.85 } else { 0.08 };
                level = level.max(on);
                color = palette.bulb;
            }
            if level > 0.012 {
                // hot LEDs whiten toward their core
                let hot = (level - 0.75).max(0.0) * 1.2;
                lit.push((i, level, mix(color, palette.white, hot.min(0.55))));
            }
        }
        lit
    }
}

/// The wall into one mesh: every unlit lens, barely there, then bloom and
/// glow (additive), then crisp cores on top, so bright areas light the air
/// in front of the wall.
pub(super) fn wall(
    texture: Texture,
    grid: &Grid,
    lit: &[Lit],
    lens: Color32,
    opacity: f32,
) -> Mesh {
    let pitch = grid.pitch;
    let n = grid.len();
    let mut mesh = Mesh::with_texture(texture);
    mesh.indices.reserve(n * 2 * 3);
    mesh.vertices.reserve(n * 4 * 3);
    let mut sprite = |p: Pos2, r: f32, uv: Rect, color: Color32| {
        mesh.add_rect_uv(
            Rect::from_center_size(p, Vec2::new(r * 2.0, r * 2.0)),
            uv,
            color,
        );
    };

    let lens_r = pitch * 0.34;
    for i in 0..n {
        sprite(grid.centre(i), lens_r, SPRITE_LENS, premul(lens, opacity));
    }
    let bloom_r = pitch * 4.2;
    for &(i, level, color) in lit {
        if level > 0.5 && (i % 2 == (i / grid.cols) % 2) {
            let k = (level.min(1.2) - 0.4) * 0.07 * opacity;
            sprite(grid.centre(i), bloom_r, SPRITE_GLOW, additive(color, k));
        }
    }
    let glow_r = pitch * 1.8;
    for &(i, level, color) in lit {
        let g = (level.min(1.3) * 0.34 * opacity).min(1.0);
        sprite(grid.centre(i), glow_r, SPRITE_GLOW, additive(color, g));
    }
    let core_r = pitch * 0.40;
    for &(i, level, color) in lit {
        let k = (level.min(1.0) * opacity).min(1.0);
        sprite(grid.centre(i), core_r, SPRITE_CORE, premul(color, k));
    }
    mesh
}

/// The wall's colours, from the theme: a gradient through the accents, a
/// warm bulb colour for the marquee, and white for the hottest cores.
pub(super) struct Palette {
    stops: [Color32; 3],
    bulb: Color32,
    white: Color32,
}

impl Palette {
    pub(super) fn of(theme: &Tokens) -> Self {
        Palette {
            stops: [theme.accent, theme.accent_soft, theme.particle_cool],
            bulb: theme.secondary,
            white: theme.particle_light,
        }
    }

    fn at(&self, h: f32) -> Color32 {
        let h = h.clamp(0.0, 1.0) * 2.0;
        if h < 1.0 {
            mix(self.stops[0], self.stops[1], h)
        } else {
            mix(self.stops[1], self.stops[2], h - 1.0)
        }
    }
}

/// An unlit lens: the background, lifted a little (pressed in a little on a
/// light theme).
pub(super) fn lens_color(theme: &Tokens) -> Color32 {
    let bg = theme.background;
    let luma = (0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32) / 255.0;
    if luma > 0.5 {
        mix(bg, Color32::BLACK, 0.06)
    } else {
        mix(bg, Color32::WHITE, 0.075)
    }
}
