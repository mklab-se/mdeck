//! Style cards: what a medium's generated artwork looks like. A card is a
//! prompt that carries the look, plus small neutral swatches sent as
//! reference images where the image model supports them. A theme's `art:`
//! block can replace the prompt and the swatches.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use super::ArtKind;
use crate::theme::Theme;

/// A built-in style card.
pub struct Card {
    pub name: &'static str,
    pub prompt: &'static str,
    pub swatches: &'static [(&'static str, &'static [u8])],
}

/// Ink lines on white, shared by every line medium (the line engine's sheet and slate,
/// a pencil sketch): the engine draws the lines in its own medium.
pub const LINE: Card = Card {
    name: "line",
    prompt: "A precise line drawing in black ink on pure white paper: clean, confident \
        ink lines of even weight, hatching for shadows and cut surfaces, a few fine \
        construction lines, drawn with a ruling pen and a compass. Line work only: no \
        grey washes, no solid fills, no gradients. Pure white background. The drawing \
        sits in the middle of the sheet with generous empty margins. Absolutely no \
        text: no letters, no words, no numbers, no labels.",
    swatches: &[
        (
            "line-still-life.jpg",
            include_bytes!("../../../art/line/still-life.jpg"),
        ),
        (
            "line-street.jpg",
            include_bytes!("../../../art/line/street.jpg"),
        ),
    ],
};

/// The MKLab house style: graphite and ink on paper, old craft meeting
/// modern technology.
#[cfg(any(feature = "sketch", feature = "line"))]
pub const SKETCH: Card = Card {
    name: "sketch",
    prompt: "A detailed hand-drawn illustration in graphite pencil and black ink on plain \
        white paper. Fine, confident pen lines; dense cross-hatching and stippling for \
        shadow and texture; carefully drawn materials (wood grain, riveted iron, brass, \
        cloth). The world blends old-fashioned craftsmanship with modern technology: \
        nineteenth-century workshops, streets and people alongside computers, screens, \
        cables and robots. Strictly monochrome grayscale, no colour at all. The subject \
        sits on white paper with generous empty margins and fades out softly at the \
        edges, like a drawing on a sketchbook page. Absolutely no text: no letters, no \
        words, no numbers, no writing on screens or signs.",
    swatches: &[
        (
            "sketch-still-life.jpg",
            include_bytes!("../../../art/sketch/still-life.jpg"),
        ),
        (
            "sketch-street.jpg",
            include_bytes!("../../../art/sketch/street.jpg"),
        ),
    ],
};

/// Loose watercolour on cold-press paper.
#[cfg(feature = "watercolour")]
pub const WATERCOLOUR: Card = Card {
    name: "watercolour",
    prompt: "A loose, luminous watercolour painting on white cold-press paper: soft \
        washes that bleed into each other, visible paper grain, pigment granulation, a \
        few crisp wet edges and a light pencil underdrawing showing through. A limited, \
        harmonious palette with plenty of untouched white paper. The subject sits in the \
        middle with generous white margins and soft, fading edges. Absolutely no text: \
        no letters, no words, no numbers, no signatures.",
    swatches: &[
        (
            "watercolour-still-life.jpg",
            include_bytes!("../../../art/watercolour/still-life.jpg"),
        ),
        (
            "watercolour-street.jpg",
            include_bytes!("../../../art/watercolour/street.jpg"),
        ),
    ],
};

/// Black-and-white photographs, printed in the darkroom.
#[cfg(feature = "darkroom")]
pub const DARKROOM: Card = Card {
    name: "darkroom",
    prompt: "A black-and-white documentary photograph printed on fibre paper: natural \
        light, rich blacks and clean whites, fine film grain, a shallow depth of field, a \
        single clear subject in a simple composition. Strictly monochrome. No text, no \
        signs with words, no watermarks, no borders.",
    swatches: &[
        (
            "darkroom-still-life.jpg",
            include_bytes!("../../../art/darkroom/still-life.jpg"),
        ),
        (
            "darkroom-street.jpg",
            include_bytes!("../../../art/darkroom/street.jpg"),
        ),
    ],
};

/// A reference image to send with a generation.
#[derive(Clone, Debug)]
pub enum Reference {
    Bundled {
        name: &'static str,
        bytes: &'static [u8],
    },
    File(PathBuf),
}

/// The style a deck's art is generated in: the medium's card, or what the
/// theme's `art:` block says instead.
#[derive(Clone, Debug)]
pub struct Style {
    /// The card it came from (or `theme` when the theme wrote the prompt).
    pub name: String,
    pub kind: ArtKind,
    pub prompt: String,
    pub references: Vec<Reference>,
}

impl Style {
    /// The style a medium's art is made in under `theme`.
    pub fn for_medium(medium: &super::Medium, theme: &Theme) -> Style {
        Style::resolve(medium.kind, medium.tonal, theme)
    }

    /// The style for a medium that asks for `default_kind`, with `tonal` as
    /// its own card (line art is shared by every medium), under `theme`.
    pub fn resolve(default_kind: ArtKind, tonal: &'static Card, theme: &Theme) -> Style {
        let kind = theme.art.kind.unwrap_or(default_kind);
        let card = match kind {
            ArtKind::Line => &LINE,
            ArtKind::Tonal => tonal,
        };
        let mut style = Style {
            name: card.name.to_string(),
            kind,
            prompt: card.prompt.to_string(),
            references: card
                .swatches
                .iter()
                .map(|(name, bytes)| Reference::Bundled { name, bytes })
                .collect(),
        };
        if let Some(prompt) = &theme.art.style {
            style.name = "theme".into();
            style.prompt = prompt.clone();
        }
        if !theme.art.references.is_empty() {
            style.references = theme
                .art
                .references
                .iter()
                .cloned()
                .map(Reference::File)
                .collect();
        }
        style
    }

    /// A short, stable id: art generated in one style is not reused for
    /// another.
    pub fn id(&self) -> String {
        let mut h = DefaultHasher::new();
        self.kind.name().hash(&mut h);
        self.prompt.hash(&mut h);
        for r in &self.references {
            match r {
                Reference::Bundled { name, bytes } => {
                    name.hash(&mut h);
                    bytes.len().hash(&mut h);
                }
                Reference::File(p) => p.hash(&mut h),
            }
        }
        format!("{}-{:08x}", self.name, h.finish() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "sketch", feature = "watercolour"))]
    #[test]
    fn line_art_is_shared_and_ids_follow_the_style() {
        let theme = Theme::light();
        let a = Style::resolve(ArtKind::Line, &SKETCH, &theme);
        let b = Style::resolve(ArtKind::Line, &WATERCOLOUR, &theme);
        assert_eq!(a.id(), b.id(), "every line medium shares the line art");
        let c = Style::resolve(ArtKind::Tonal, &SKETCH, &theme);
        assert_ne!(a.id(), c.id());
        assert!(c.id().starts_with("sketch-"));
        let mut t = Theme::light();
        t.art.style = Some("pixel art".into());
        let d = Style::resolve(ArtKind::Tonal, &SKETCH, &t);
        assert_eq!(d.prompt, "pixel art");
        assert!(d.id().starts_with("theme-"));
    }

    #[test]
    fn every_card_forbids_text_and_has_swatches() {
        let cards = [
            Some(&LINE),
            #[cfg(any(feature = "sketch", feature = "line"))]
            Some(&SKETCH),
            #[cfg(feature = "watercolour")]
            Some(&WATERCOLOUR),
            #[cfg(feature = "darkroom")]
            Some(&DARKROOM),
        ];
        for card in cards.into_iter().flatten() {
            assert!(card.prompt.contains("no text") || card.prompt.contains("No text"));
            assert_eq!(card.swatches.len(), 2, "{}", card.name);
            for (_, bytes) in card.swatches {
                assert!(image::load_from_memory(bytes).is_ok(), "{}", card.name);
            }
        }
    }
}
