//! Deterministic random numbers shared by the core (point cloud conversion,
//! glyph masks). The particle field itself is the particles engine's.

mod rng;

pub use rng::Rng;
