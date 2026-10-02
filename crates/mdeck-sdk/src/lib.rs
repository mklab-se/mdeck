//! # mdeck-sdk
//!
//! The SDK for [mdeck](https://github.com/mklab-se/mdeck) extensions: the
//! interfaces for engines, visual kinds, design sets and transitions, and
//! the types they receive (EXT-12).
//!
//! An extension is an ordinary Rust crate that depends on `mdeck-sdk` and
//! exposes a [`registry::Register`] function. Everything it touches is an
//! mdeck type: no egui (or other third-party) type is part of the stable
//! surface (EXT-24), and drawing goes through [`paint::Painter`] (EXT-25).
//! The `unstable-egui` cargo feature opens raw egui access for what the
//! painter does not offer yet; it is outside the compatibility promise.
//!
//! The SDK is versioned in lockstep with mdeck: SDK 2.x works with mdeck 2.x.
//!
//! ## Modules
//!
//! - [`paint`]: colours, geometry, meshes, textures, fonts and the [`paint::Painter`];
//! - [`tokens`]: the theme's colours and the engine's typed settings;
//! - [`content`]: the slide content model;
//! - [`cloud`]: point clouds and masks;
//! - [`geometry`]: geometry visuals publish for engines;
//! - [`stage`]: what an engine is shown each frame;
//! - [`engine`], [`visual`], [`design`], [`transition`]: the extension traits;
//! - [`registry`]: where extensions register what they bring;
//! - [`problem`]: what extensions report to `mdeck --check`;
//! - [`testing`]: headless rendering for extension tests.
//!
//! ```
//! use mdeck_sdk::registry::{Registry, RegistryError};
//!
//! pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
//!     r.theme("midnight", "name: midnight\nengine: plain\n")
//! }
//!
//! let mut r = Registry::new();
//! r.set_origin("midnight-pack");
//! register(&mut r).unwrap();
//! assert!(mdeck_sdk::VERSION.contains('.'));
//! ```

#![deny(missing_docs)]

pub mod cloud;
pub mod content;
pub mod design;
pub mod engine;
pub mod geometry;
#[doc(hidden)]
pub mod host;
pub mod paint;
pub mod problem;
pub mod registry;
pub mod stage;
#[doc(hidden)]
pub mod templates;
pub mod testing;
pub mod tokens;
pub mod transition;
pub mod visual;

/// The SDK's version, equal to the mdeck version it ships with.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
