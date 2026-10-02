//! Keeping a scene's clusters off the slide's copy: the scenes are
//! choreographed for the editorial layouts (copy on the left, the right
//! flank free), but a design set may centre the copy or run it wide. The
//! design publishes where its copy is ([`mdeck_sdk::geometry::Hint::Copy`]);
//! clusters that would land on it move to the nearest free side.

use crate::engines::particles::{Home, Scene};

/// The copy box in slide fractions: left, top, right, bottom.
pub type Box = [f32; 4];

/// Move every cluster of `scene` that would sit on `copy` to the nearest
/// side of it with room, and dim one that has nowhere to go. `aspect` is the
/// slide's width over height (a cluster's radius is in heights).
pub fn keep_clear(scene: &mut Scene, copy: Box, aspect: f32) {
    let [l, t, r, b] = copy;
    for g in &mut scene.groups {
        let Home::Cluster {
            u, v, r: radius, ..
        } = &mut g.home
        else {
            continue;
        };
        // the cluster's reach, as fractions of the slide's width and height
        let (mu, mv) = (*radius / aspect + 0.02, *radius + 0.03);
        let inside = *u > l - mu && *u < r + mu && *v > t - mv && *v < b + mv;
        if !inside {
            continue;
        }
        let options = [
            (l - mu, *v, l - mu >= mu),
            (r + mu, *v, r + mu <= 1.0 - mu),
            (*u, t - mv, t - mv >= mv),
            (*u, b + mv, b + mv <= 1.0 - mv),
        ];
        let best = options
            .iter()
            .filter(|o| o.2)
            .min_by(|a, b| {
                let d = |o: &&(f32, f32, bool)| ((o.0 - *u) * aspect).hypot(o.1 - *v);
                d(a).total_cmp(&d(b))
            })
            .map(|o| (o.0, o.1));
        match best {
            Some((nu, nv)) => (*u, *v) = (nu, nv),
            // copy wall to wall: the cluster stays, faint, behind it
            None => g.alpha = (g.alpha.0 * 0.25, g.alpha.1 * 0.25),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::particles::scenes::constellation;

    fn centres(scene: &Scene) -> Vec<(f32, f32, f32)> {
        scene
            .groups
            .iter()
            .filter_map(|g| match g.home {
                Home::Cluster { u, v, r, .. } => Some((u, v, r)),
                _ => None,
            })
            .collect()
    }

    // With `designs: standard` the title is centred, where the
    // constellation (made for the editorial spread) put bright clusters.
    #[test]
    fn clusters_leave_a_centred_title_alone() {
        let aspect = 16.0 / 9.0;
        let copy = [0.2, 0.38, 0.8, 0.62];
        for seed in 1..20 {
            let mut scene = constellation(seed);
            keep_clear(&mut scene, copy, aspect);
            for (u, v, r) in centres(&scene) {
                let on = u > copy[0] - r / aspect
                    && u < copy[2] + r / aspect
                    && v > copy[1] - r
                    && v < copy[3] + r;
                assert!(!on, "seed {seed}: cluster at ({u}, {v}) r {r}");
                assert!((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v));
            }
        }
    }

    #[test]
    fn copy_wall_to_wall_dims_what_cannot_move() {
        let mut scene = constellation(3);
        let before: Vec<(f32, f32)> = scene.groups.iter().map(|g| g.alpha).collect();
        keep_clear(&mut scene, [0.0, 0.0, 1.0, 1.0], 16.0 / 9.0);
        for (g, a) in scene.groups.iter().zip(before) {
            if matches!(g.home, Home::Cluster { .. }) {
                assert!(g.alpha.1 < a.1);
            }
        }
    }
}
