//! mdeck's drawing interface (EXT-25).
//!
//! Extensions draw through [`Painter`] with mdeck's own types: [`Color`],
//! [`Pos2`], [`Vec2`], [`Rect`], [`Stroke`], [`Mesh`], [`Texture`],
//! [`ImageData`] and [`Font`]. No egui type appears here, so an egui upgrade
//! inside mdeck never breaks an extension.
//!
//! Colours are premultiplied: [`additive`] makes a colour that adds light
//! (alpha 0), [`premul`] one at an opacity with normal blending. For many
//! soft lights at once use [`Painter::sprites`], which adds light on the GPU.
//!
//! ```
//! use mdeck_sdk::paint::{mix, Color};
//! let ember = Color::from_rgb(255, 77, 28);
//! let dim = mix(Color::BLACK, ember, 0.25);
//! assert!(dim.r() < ember.r());
//! ```

mod color;
mod geom;
pub(crate) mod painter;
mod shapes;
mod sprites;

pub use color::{Color, additive, mix, premul, smoothstep};
pub use geom::{Align, Align2, Pos2, Rect, Vec2};
pub(crate) use geom::{FromEgui, ToEgui};
pub use painter::Painter;
pub use shapes::{
    Font, FontRole, ImageData, Mesh, SPRITE_CELL, SPRITE_CORE, SPRITE_GLOW, SPRITE_LENS, Stroke,
    Texture, TextureFilter, Vertex, sprite_sheet,
};
pub use sprites::{Sprite, SpriteBlend, SpriteLayer};
