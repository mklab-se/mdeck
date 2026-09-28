//! Generated artwork for the art engines (sketch, blueprint, chalkboard,
//! watercolour, darkroom). `mdeck ai art` makes a picture per slide with the
//! configured image model, in the style of the deck's medium ([`style`]),
//! and records it in a sidecar next to the deck ([`sidecar`]). Presenting
//! never calls the AI: [`gallery::Gallery`] loads what is cached, and
//! [`prepare`] works out the order an engine draws each picture in.

pub mod gallery;
mod loader;
pub mod prepare;
pub mod sidecar;
pub mod style;

use crate::parser::Slide;

/// What kind of picture a medium asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtKind {
    /// Black ink lines on white, drawn by the engine in its own medium.
    Line,
    /// A picture in the medium itself (graphite, watercolour, photograph).
    Tonal,
}

impl ArtKind {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim() {
            "line" => Some(ArtKind::Line),
            "tonal" => Some(ArtKind::Tonal),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ArtKind::Line => "line",
            ArtKind::Tonal => "tonal",
        }
    }
}

/// An art engine's medium: the kind of picture it asks for by default,
/// its own card for tonal pictures, and how it draws those in (line art is
/// always drawn along the ink).
pub struct Medium {
    pub name: &'static str,
    pub kind: ArtKind,
    pub tonal: &'static style::Card,
    pub tonal_strategy: prepare::Strategy,
}

/// Whether a slide gets a picture: its layout has a stage for one (the
/// editorial layouts: title, section, quote, bullet and copy slides) and it
/// did not say `@art: none`.
pub fn wants_art(slide: &Slide) -> bool {
    slide.art.as_deref().map(str::trim) != Some("none") && crate::render::ember::handles(slide)
}

/// The scene a slide's own `@art:` describes, if it does.
pub fn slide_scene(slide: &Slide) -> Option<&str> {
    slide
        .art
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "none")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn copy_slides_want_art_unless_they_say_none() {
        let md = "# Title\n\nA talk\n\n# Copy\n\n- one\n\n# No art\n@art: none\n\n- two\n\n# Code\n\n```rust\nfn main() {}\n```\n\n# Scene\n@art: a lighthouse at dawn\n\n- three\n";
        let pres = crate::parser::parse(md, Path::new("."));
        let wants: Vec<bool> = pres.slides.iter().map(wants_art).collect();
        assert_eq!(wants, [true, true, false, false, true]);
        assert_eq!(slide_scene(&pres.slides[4]), Some("a lighthouse at dawn"));
        assert_eq!(slide_scene(&pres.slides[1]), None);
    }
}
