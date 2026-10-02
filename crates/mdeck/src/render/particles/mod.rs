//! The particle field behind Ember slides.
//!
//! A fixed pool of glowing particles morphs from scene to scene: each scene
//! assigns every particle a *home* (a point it drifts around), and particles
//! ease toward their homes, so a slide change is a migration rather than a
//! cut. Scenes are declarative data ([`Scene`]) built from slide content in
//! [`scenes`]; the field itself knows nothing about markdown.
//!
//! Rendering happens in two layers: hairline links between neighbouring
//! particles are drawn with the egui painter, and the glow sprites are drawn
//! additively by [`gl::GlowRenderer`].

// The field, its renderer and its scenes belong to the particles engine; the
// random numbers are shared (point cloud conversion).
#[cfg(feature = "particles")]
pub mod gl;
#[cfg(feature = "particles")]
pub mod scenes;

#[cfg(feature = "particles")]
mod field;
mod rng;
#[cfg(feature = "particles")]
mod scene;

#[cfg(feature = "particles")]
pub use field::Field;
pub use rng::Rng;
#[cfg(feature = "particles")]
pub use scene::{DEFAULT_TINTS, Drift, Group, Home, Palette, Scene, Tint};

/// Particles in the presentation window (the site uses 520 on desktop).
#[cfg(feature = "particles")]
pub const DEFAULT_COUNT: usize = 900;
