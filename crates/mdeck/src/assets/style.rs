//! One style system for everything `mdeck ai` makes. A style is a prompt
//! that carries the look plus optional reference images. Each asset kind has
//! a default style:
//!
//! - images: [`crate::prompt::DEFAULT_IMAGE_STYLE`];
//! - icons: [`crate::prompt::DEFAULT_ICON_STYLE`];
//! - artworks: the style card of the engine's medium
//!   (`render::art::style`), which a theme's `art:` block can replace;
//! - point clouds: the particle prompt of `mdeck ai point-cloud`.
//!
//! Named styles live in the user config (`mdeck ai style`), each a prompt
//! with optional reference images. For images and icons the choice is
//! `--style` > the deck's `image-style` / `icon-style` > the config default >
//! the built-in default; a name is looked up, anything else is a literal
//! prompt. Every asset records the [`Style::id`] it was made in, so a change
//! of style makes it stale.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use crate::config::Config;
use crate::parser::PresentationMeta;

/// A reference image to send with a generation.
#[derive(Clone, Debug, PartialEq)]
pub enum Reference {
    /// A swatch bundled with mdeck.
    Bundled {
        name: &'static str,
        bytes: &'static [u8],
    },
    File(PathBuf),
}

/// A resolved style: its name (a named style, `default`, or `custom` for a
/// literal prompt), the prompt and the reference images.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub name: String,
    pub prompt: String,
    pub references: Vec<Reference>,
}

impl Style {
    pub fn new(name: &str, prompt: &str, references: Vec<PathBuf>) -> Self {
        Self {
            name: name.to_string(),
            prompt: prompt.to_string(),
            references: references.into_iter().map(Reference::File).collect(),
        }
    }

    /// A short, stable id: an asset made in one style goes stale in another.
    pub fn id(&self) -> String {
        let mut h = DefaultHasher::new();
        self.prompt.hash(&mut h);
        hash_references(&self.references, &mut h);
        format!("{}-{:08x}", slug(&self.name), h.finish() as u32)
    }

    /// The reference images as files an image client can send (bundled
    /// swatches are written to a temporary folder once).
    pub fn reference_files(references: &[Reference]) -> std::io::Result<Vec<PathBuf>> {
        let dir = std::env::temp_dir().join("mdeck-style-references");
        std::fs::create_dir_all(&dir)?;
        references
            .iter()
            .map(|r| match r {
                Reference::File(p) => Ok(p.clone()),
                Reference::Bundled { name, bytes } => {
                    let p = dir.join(name);
                    if !p.exists() {
                        std::fs::write(&p, bytes)?;
                    }
                    Ok(p)
                }
            })
            .collect()
    }
}

/// Feed reference images into a style hash: bundled ones by name and size,
/// files by path and contents (D20), so a swatch edited in place makes what
/// was made with it stale.
pub fn hash_references(references: &[Reference], h: &mut DefaultHasher) {
    for r in references {
        match r {
            Reference::Bundled { name, bytes } => {
                name.hash(h);
                bytes.len().hash(h);
            }
            Reference::File(p) => {
                p.hash(h);
                file_digest(p).hash(h);
            }
        }
    }
}

/// A hash of a file's contents, `None` when it cannot be read. Style ids
/// are asked for every frame, so the digest is kept per path and the file
/// is read again only when its size or modification time changes.
fn file_digest(path: &std::path::Path) -> Option<u64> {
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex};
    type Stamp = (u64, Option<std::time::SystemTime>);
    static DIGESTS: LazyLock<Mutex<HashMap<PathBuf, (Stamp, u64)>>> =
        LazyLock::new(Default::default);
    let meta = std::fs::metadata(path).ok()?;
    let stamp: Stamp = (meta.len(), meta.modified().ok());
    let mut digests = DIGESTS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((s, d)) = digests.get(path)
        && *s == stamp
    {
        return Some(*d);
    }
    let bytes = std::fs::read(path).ok()?;
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    let digest = h.finish();
    digests.insert(path.to_path_buf(), (stamp, digest));
    Some(digest)
}

fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "style".into() } else { s }
}

/// The styles a deck's images and icons are made in.
#[derive(Clone, Debug, PartialEq)]
pub struct Styles {
    pub image: Style,
    pub icon: Style,
}

/// Which style table a name is looked up in.
#[derive(Clone, Copy)]
enum Table {
    Image,
    Icon,
}

fn named(config: &Config, table: Table, name: &str) -> Option<Style> {
    let (prompt, refs) = match table {
        Table::Image => (
            config.get_style(name)?,
            config.style_references(name, false),
        ),
        Table::Icon => (
            config.get_icon_style(name)?,
            config.style_references(name, true),
        ),
    };
    Some(Style::new(name, prompt, refs.to_vec()))
}

/// A name looked up in `table`, or a literal prompt.
fn named_or_literal(config: &Config, table: Table, s: &str) -> Style {
    named(config, table, s).unwrap_or_else(|| Style::new("custom", s, Vec::new()))
}

fn resolve_one(
    config: &Config,
    table: Table,
    cli: Option<&str>,
    deck: Option<&str>,
    config_default: Option<&str>,
    builtin: &str,
) -> Style {
    if let Some(s) = cli.or(deck) {
        return named_or_literal(config, table, s);
    }
    if let Some(name) = config_default
        && let Some(style) = named(config, table, name)
    {
        return style;
    }
    Style::new("default", builtin, Vec::new())
}

/// The deck's image and icon styles: `--style` > `image-style` /
/// `icon-style` > the config default > the built-in default. `--style`
/// applies to icons only when it names an icon style or is a literal prompt
/// (a name that only exists as an image style is left to images).
pub fn resolve(config: &Config, meta: &PresentationMeta, cli: Option<&str>) -> Styles {
    let defaults = config.defaults.as_ref();
    let image = resolve_one(
        config,
        Table::Image,
        cli,
        meta.image_style.as_deref(),
        defaults.and_then(|d| d.image_style.as_deref()),
        crate::prompt::DEFAULT_IMAGE_STYLE,
    );
    let icon_cli =
        cli.filter(|s| config.get_icon_style(s).is_some() || config.get_style(s).is_none());
    let icon = resolve_one(
        config,
        Table::Icon,
        icon_cli,
        meta.icon_style.as_deref(),
        defaults.and_then(|d| d.icon_style.as_deref()),
        crate::prompt::DEFAULT_ICON_STYLE,
    );
    Styles { image, icon }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        let mut c = Config::default();
        c.add_style("photo", "a photograph");
        c.add_style("both", "image both");
        c.add_icon_style("flat", "flat icon");
        c.add_icon_style("both", "icon both");
        c
    }

    #[test]
    fn the_cli_beats_the_deck_beats_the_config_beats_the_default() {
        let c = config();
        let mut meta = PresentationMeta::default();
        let s = resolve(&c, &meta, None);
        assert_eq!(s.image.prompt, crate::prompt::DEFAULT_IMAGE_STYLE);
        assert_eq!(s.icon.prompt, crate::prompt::DEFAULT_ICON_STYLE);
        assert_eq!(s.image.name, "default");
        meta.image_style = Some("photo".into());
        meta.icon_style = Some("thin lines".into());
        let s = resolve(&c, &meta, None);
        assert_eq!(
            (s.image.name.as_str(), s.image.prompt.as_str()),
            ("photo", "a photograph")
        );
        assert_eq!(
            (s.icon.name.as_str(), s.icon.prompt.as_str()),
            ("custom", "thin lines")
        );
        let s = resolve(&c, &meta, Some("watercolour"));
        assert_eq!(s.image.prompt, "watercolour");
        assert_eq!(s.icon.prompt, "watercolour", "a literal applies to both");
    }

    #[test]
    fn an_image_only_name_on_the_cli_leaves_icons_alone() {
        let c = config();
        let meta = PresentationMeta::default();
        let s = resolve(&c, &meta, Some("photo"));
        assert_eq!(s.image.prompt, "a photograph");
        assert_eq!(s.icon.name, "default");
        let s = resolve(&c, &meta, Some("both"));
        assert_eq!(s.image.prompt, "image both");
        assert_eq!(s.icon.prompt, "icon both");
        let s = resolve(&c, &meta, Some("flat"));
        assert_eq!(s.icon.prompt, "flat icon");
    }

    #[test]
    fn ids_follow_the_prompt_and_the_references() {
        let a = Style::new("photo", "a photograph", Vec::new());
        let b = Style::new("photo", "a photograph", vec![PathBuf::from("ref.png")]);
        let c = Style::new("photo", "a painting", Vec::new());
        assert!(a.id().starts_with("photo-"));
        assert_ne!(a.id(), b.id());
        assert_ne!(a.id(), c.id());
        assert_eq!(a.id(), Style::new("photo", "a photograph", Vec::new()).id());
        assert!(
            Style::new("My Style!", "x", Vec::new())
                .id()
                .starts_with("my-style-")
        );
    }

    /// D20: a reference image edited in place changes the id, so what was
    /// made with the old picture goes stale.
    #[test]
    fn ids_follow_a_reference_files_contents_not_only_its_path() {
        let dir = std::env::temp_dir().join(format!("mdeck-style-ref-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("swatch.png");
        std::fs::write(&file, b"first swatch").unwrap();
        let style = Style::new("look", "a look", vec![file.clone()]);
        let before = style.id();
        assert_eq!(before, style.id(), "stable while the file is unchanged");
        std::fs::write(&file, b"the swatch, repainted").unwrap();
        assert_ne!(before, style.id(), "an edited swatch is another style");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn named_styles_carry_their_references() {
        let mut c = config();
        c.set_style_references("photo", false, vec![PathBuf::from("look.png")]);
        let meta = PresentationMeta {
            image_style: Some("photo".into()),
            ..Default::default()
        };
        let s = resolve(&c, &meta, None);
        assert_eq!(s.image.references, vec![Reference::File("look.png".into())]);
    }
}
