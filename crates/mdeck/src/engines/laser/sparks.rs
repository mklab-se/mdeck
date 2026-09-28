//! Sparks thrown off the beam's tip and the smoke drifting up from it.

use eframe::egui::{self, Pos2, Rect};

use super::Laser;

#[derive(Clone, Copy, Debug)]
pub(super) struct Spark {
    pub(super) pos: Pos2,
    pub(super) vel: egui::Vec2,
    pub(super) life: f32,
    pub(super) max: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Puff {
    pub(super) pos: Pos2,
    pub(super) life: f32,
    pub(super) max: f32,
    pub(super) size: f32,
    pub(super) drift: f32,
}

impl Laser {
    pub(super) fn emit(&mut self, tip: Pos2, rect: Rect, scale: f32, dt: f32) {
        let p = Pos2::new(
            rect.left() + tip.x * rect.width(),
            rect.top() + tip.y * rect.height(),
        );
        let sparks = (dt * 260.0).round() as usize + usize::from(self.rand() < dt * 260.0 % 1.0);
        for _ in 0..sparks {
            let a = self.rand() * std::f32::consts::TAU;
            let speed = (120.0 + 520.0 * self.rand()) * scale;
            let max = 0.18 + 0.4 * self.rand();
            self.sparks.push(Spark {
                pos: p,
                vel: egui::vec2(a.cos() * speed, a.sin() * speed - 180.0 * scale),
                life: max,
                max,
            });
        }
        if self.rand() < dt * 38.0 {
            let max = 1.8 + 1.4 * self.rand();
            let size = (10.0 + 12.0 * self.rand()) * scale;
            let drift = self.rand() * 6.0;
            self.smoke.push(Puff {
                pos: p,
                life: max,
                max,
                size,
                drift,
            });
        }
    }

    pub(super) fn step_particles(&mut self, dt: f32, scale: f32) {
        let g = 900.0 * scale;
        for s in &mut self.sparks {
            s.vel.y += g * dt;
            s.vel *= 1.0 - 1.6 * dt;
            s.pos += s.vel * dt;
            s.life -= dt;
        }
        self.sparks.retain(|s| s.life > 0.0);
        for p in &mut self.smoke {
            p.pos.y -= (26.0 + 10.0 * (p.drift).sin()) * scale * dt;
            p.pos.x += (self.now * 0.9 + p.drift).sin() * 9.0 * scale * dt;
            p.size += 14.0 * scale * dt;
            p.life -= dt;
        }
        self.smoke.retain(|p| p.life > 0.0);
    }
}
