//! Revealing a generated picture on the CPU: which of an
//! [`Artwork`]'s pixels have arrived at a moment of the reveal, in the
//! medium's own way (soft edges, a faint first pass, grain on a rough
//! surface), and where the drawing hand is.

use mdeck_sdk::paint::{Color, smoothstep};
use mdeck_sdk::stage::{Artwork, Strategy};

use crate::engines::hash01;

/// Moments in an [`Artwork::when`] run over this many steps.
const STEPS: f32 = 65535.0;

/// How a medium reveals its pictures.
#[derive(Clone, Copy, Debug)]
pub struct Reveal {
    /// How long a pixel takes to arrive, as a share of the reveal.
    pub soft: f32,
    /// A faint first pass (construction lines, an underdrawing): its
    /// opacity, and how much faster than the ink it runs.
    pub ghost: f32,
    pub ghost_speed: f32,
    /// How much the medium breaks up on its surface (chalk on slate), 0..1.
    pub grain: f32,
}

/// Revealing an artwork on the CPU.
pub trait Reveals {
    /// The picture at `t` as premultiplied pixels (see [`Reveal`]).
    fn reveal(&self, t: f32, r: &Reveal) -> Vec<Color>;
    /// The finished picture, premultiplied.
    #[cfg(test)]
    fn finished(&self) -> Vec<Color>;
    /// Where the hand is at `t`, in 0..1 of the picture, while it draws.
    fn tip(&self, t: f32) -> Option<(f32, f32)>;
}

impl Reveals for Artwork {
    /// The picture at `t` (0: nothing yet, 1: finished) as premultiplied
    /// pixels. `soft` is how long a pixel takes to arrive (0..1 of the
    /// reveal), `ghost` the opacity of a faint first pass that runs ahead
    /// at `ghost_speed` (a draftsman's construction lines; 0 for none).
    fn reveal(&self, t: f32, r: &Reveal) -> Vec<Color> {
        let soft = r.soft.max(1e-3);
        let t_now = t * STEPS;
        let span = soft * STEPS;
        let g_now = t * r.ghost_speed * STEPS;
        let develop = self.strategy == Strategy::Develop;
        let w_px = self.width.max(1);
        self.rgba
            .iter()
            .zip(&self.when)
            .enumerate()
            .map(|(i, (px, &w))| {
                let w = w as f32;
                let mut k = ((t_now - w) / span).clamp(0.0, 1.0);
                if r.ghost > 0.0 {
                    k = k.max(r.ghost * ((g_now - w) / span).clamp(0.0, 1.0));
                }
                if develop {
                    // the print darkens into place instead of fading in
                    k = smoothstep(0.0, 1.0, k);
                }
                if r.grain > 0.0 {
                    // the medium skips on a rough surface, in clumps of a
                    // couple of pixels
                    let (x, y) = ((i % w_px) as u32, (i / w_px) as u32);
                    let clump = hash01((x / 2).wrapping_mul(7919) ^ (y / 2).wrapping_mul(104_729));
                    let fine = hash01(i as u32);
                    k *= 1.0 - r.grain * (0.65 * clump + 0.35 * fine);
                }
                let a = px[3] as f32 / 255.0 * k;
                Color::from_rgba_premultiplied(
                    (px[0] as f32 * a) as u8,
                    (px[1] as f32 * a) as u8,
                    (px[2] as f32 * a) as u8,
                    (a * 255.0) as u8,
                )
            })
            .collect()
    }

    #[cfg(test)]
    fn finished(&self) -> Vec<Color> {
        self.reveal(
            1.0 + 1e-3,
            &Reveal {
                soft: 1e-3,
                ghost: 0.0,
                ghost_speed: 1.0,
                grain: 0.0,
            },
        )
    }

    fn tip(&self, t: f32) -> Option<(f32, f32)> {
        if self.path.is_empty() || t <= 0.0 || t >= 1.0 {
            return None;
        }
        let i = self.path.partition_point(|p| p.0 <= t);
        if i == 0 {
            return Some((self.path[0].1, self.path[0].2));
        }
        if i >= self.path.len() {
            let p = self.path[self.path.len() - 1];
            return Some((p.1, p.2));
        }
        let (a, b) = (self.path[i - 1], self.path[i]);
        let f = ((t - a.0) / (b.0 - a.0).max(1e-6)).clamp(0.0, 1.0);
        Some((a.1 + (b.1 - a.1) * f, a.2 + (b.2 - a.2) * f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: Reveal = Reveal {
        soft: 0.02,
        ghost: 0.0,
        ghost_speed: 1.0,
        grain: 0.0,
    };

    /// A line of ink drawn left to right, the hand following it.
    fn line(strategy: Strategy) -> Artwork {
        let n = 64;
        Artwork {
            width: n,
            height: 1,
            rgba: vec![[255, 255, 255, 255]; n],
            when: (0..n).map(|i| (i * 65535 / n) as u16).collect(),
            path: vec![(0.0, 0.0, 0.5), (1.0, 1.0, 0.5)],
            strategy,
        }
    }

    fn ink(px: &[Color]) -> u64 {
        px.iter().map(|c| c.a() as u64).sum()
    }

    #[test]
    fn reveal_runs_from_nothing_to_the_picture() {
        let p = line(Strategy::Draw);
        let none = ink(&p.reveal(0.0, &PLAIN));
        let half = ink(&p.reveal(0.5, &PLAIN));
        let full = ink(&p.finished());
        assert_eq!(none, 0);
        assert!(half > 0 && half < full, "{half} {full}");
        // the hand is on the paper while it draws, and gone after
        let (u, v) = p.tip(0.5).expect("drawing");
        assert!((u - 0.5).abs() < 1e-4 && (v - 0.5).abs() < 1e-4);
        assert!(p.tip(1.0).is_none());
        assert!(p.tip(0.0).is_none());
    }

    #[test]
    fn every_strategy_finishes_as_the_picture() {
        for s in [
            Strategy::Draw,
            Strategy::Hatch,
            Strategy::Bloom,
            Strategy::Develop,
        ] {
            let p = line(s);
            for (px, out) in p.rgba.iter().zip(&p.finished()) {
                assert!(px[3].abs_diff(out.a()) <= 1, "{s:?}");
            }
        }
    }

    #[test]
    fn a_ghost_runs_ahead_faintly_and_grain_breaks_the_line() {
        let p = line(Strategy::Draw);
        let ghost = Reveal {
            ghost: 0.2,
            ghost_speed: 2.0,
            ..PLAIN
        };
        let early = p.reveal(0.3, &ghost);
        // past the ink, the ghost shows at its own faint opacity
        let ahead = early[35].a();
        assert!(ahead > 0 && ahead <= (0.2 * 255.0) as u8 + 1, "{ahead}");
        let chalk = Reveal {
            grain: 0.72,
            ..PLAIN
        };
        assert!(ink(&p.reveal(2.0, &chalk)) < ink(&p.finished()));
    }

    #[test]
    fn a_print_develops_eased() {
        let p = line(Strategy::Develop);
        let soft = Reveal { soft: 1.0, ..PLAIN };
        // a quarter of the way into a pixel's arrival is less than a quarter
        // of its ink
        let a = p.reveal(0.25, &soft)[0].a();
        assert!(a < 64, "{a}");
    }
}
