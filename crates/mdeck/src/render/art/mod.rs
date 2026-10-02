//! Generated artwork for the art engines (line, sketch,
//! watercolour, darkroom). `mdeck ai pictures` makes a picture per slide
//! with the configured image model, in the style of the deck's medium
//! ([`style`]), and records it in the deck's asset manifest ([`resolve`],
//! [`crate::assets::manifest`]). Presenting
//! never calls the AI: [`gallery::Gallery`] loads what is cached, and
//! [`prepare`] works out the order an engine draws each picture in.

pub mod gallery;
mod loader;
pub mod prepare;
pub mod resolve;
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

/// Whether a slide gets a generated picture: its layout has a stage for one
/// (the editorial layouts: title, section, quote, bullet and copy slides),
/// it did not say `picture: none`, and it did not choose a point cloud for
/// its picture without a `picture-prompt` (phase 2 makes the picture one
/// source: an artwork when there is one, else the point cloud).
pub fn wants_art(slide: &Slide) -> bool {
    let art = slide.art.as_deref().map(str::trim);
    art != Some("none")
        && (slide.illustration.is_none() || art.is_some())
        && crate::render::ember::handles(slide)
}

/// The scene a slide's own `picture-prompt` describes, if it does.
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

    #[test]
    fn copy_slides_want_art_unless_they_say_none() {
        let md = "# Title\n\nA talk\n\n# Copy\n\n- one\n\n# No art\n<!-- picture: none -->\n\n- two\n\n# Code\n\n```rust\nfn main() {}\n```\n\n# Scene\n<!-- picture-prompt: a lighthouse at dawn -->\n\n- three\n\n# Cloud\n<!-- picture: gear -->\n\n- four\n";
        let pres = crate::parser::parse(md);
        let wants: Vec<bool> = pres.slides.iter().map(wants_art).collect();
        assert_eq!(wants, [true, true, false, false, true, false]);
        assert_eq!(slide_scene(&pres.slides[4]), Some("a lighthouse at dawn"));
        assert_eq!(slide_scene(&pres.slides[1]), None);
    }
}
