//! Story scripts: a small declarative vocabulary that turns a slide into a
//! choreographed scene on the particle field.
//!
//! A script names a **cast** (people and props placed in stage cells), the
//! **flows** between them, and the **beats** the presenter releases with
//! Space. Authors write scripts by hand in a ```@scene fence, or let
//! `mdeck ai story` write them from an English ```@story hint. Either way the
//! model or the author states intent; geometry, colour, motion and label
//! placement are decided here, so a script can never draw off-brand.

mod labels;
mod script;
pub mod sidecar;
mod stage;
mod vocabulary;

#[cfg(test)]
mod fixtures;

// The labels are drawn only by the particles engine.
#[cfg(feature = "particles")]
pub use labels::draw_labels;
pub use labels::label_collisions;
pub use script::{Cell, Fill, FlowColor, Member, Script};
pub use stage::{Label, Staged, allowed, stage, stage_box};
pub use vocabulary::{is_figure, vocabulary};
