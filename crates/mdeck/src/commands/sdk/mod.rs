//! `mdeck sdk new <kind> <name>` (EXT-28): a new extension crate from the
//! scaffold templates in `crates/mdeck-sdk/templates/`, embedded in the SDK
//! so the command works from any installed mdeck.

pub mod preview;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use colored::Colorize;

/// What a scaffold makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TemplateKind {
    /// An engine: a living layer under (or around) every slide
    Engine,
    /// A visual: a fence tag drawn on the slide
    Visual,
    /// A design set: the designs slides are laid out with
    DesignSet,
    /// A transition between slides
    Transition,
}

impl TemplateKind {
    pub fn name(self) -> &'static str {
        match self {
            TemplateKind::Engine => "engine",
            TemplateKind::Visual => "visual",
            TemplateKind::DesignSet => "design-set",
            TemplateKind::Transition => "transition",
        }
    }

    /// The template's files: path in the crate and text (embedded in the
    /// SDK, see `mdeck_sdk::templates`).
    pub fn files(self) -> &'static [(&'static str, &'static str)] {
        mdeck_sdk::templates::files(self.name()).expect("every kind has a template")
    }
}

/// A template's text for the extension `name`.
pub fn instantiate(text: &str, name: &str) -> String {
    text.replace("{{name}}", name)
        .replace("{{crate_name}}", &name.replace('-', "_"))
        .replace("{{sdk_version}}", env!("CARGO_PKG_VERSION"))
}

/// An extension name: a crate name and a name in the registry.
pub fn validate_name(name: &str) -> Result<()> {
    let ok = name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.ends_with('-');
    if !ok {
        bail!(
            "`{name}` is not a usable name: start with a letter, use lowercase letters, digits and hyphens"
        );
    }
    Ok(())
}

/// Write the `kind` scaffold for `name` into `dir`, which must not exist or
/// be empty. Returns the files written.
pub fn create(kind: TemplateKind, name: &str, dir: &Path) -> Result<Vec<PathBuf>> {
    validate_name(name)?;
    if dir.exists() {
        let empty = std::fs::read_dir(dir)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if !empty {
            bail!(
                "{} already exists and is not empty; nothing was written",
                dir.display()
            );
        }
    }
    let mut written = Vec::new();
    for (file, text) in kind.files() {
        let path = dir.join(file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&path, instantiate(text, name))
            .with_context(|| format!("writing {}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

/// `mdeck sdk new`.
pub fn run_new(kind: TemplateKind, name: &str, dir: Option<PathBuf>, quiet: bool) -> Result<()> {
    let dir = dir.unwrap_or_else(|| PathBuf::from(name));
    create(kind, name, &dir)?;
    if !quiet {
        println!(
            "Created the {} `{}` in {}",
            kind.name(),
            name.bold(),
            dir.display()
        );
        println!();
        println!("  cd {}", dir.display());
        println!("  cargo test                 # records tests/golden/{name}.png");
        println!("  mdeck build --with .       # an mdeck with it built in");
        println!("  ./target/release/mdeck deck.md");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_checked() {
        assert!(validate_name("glow").is_ok());
        assert!(validate_name("acme-roadmap2").is_ok());
        for bad in ["", "Glow", "2glow", "glow_", "glow-", "a b"] {
            assert!(validate_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn every_kind_instantiates_without_placeholders() {
        for kind in TemplateKind::value_variants() {
            assert_eq!(kind.files().len(), 7);
            for (file, text) in kind.files() {
                let out = instantiate(text, "my-ext");
                assert!(!out.contains("{{"), "{}/{file}", kind.name());
            }
        }
    }

    #[test]
    fn the_engine_scaffold_matches_the_example() {
        let dir = crate::extensions::packs::tempdir("sdk-new").unwrap();
        let target = dir.join("template-engine");
        let files = create(TemplateKind::Engine, "template-engine", &target).unwrap();
        assert_eq!(files.len(), 7);
        let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/template-engine");
        // Cargo.toml differs on purpose: the example is a workspace member.
        for file in [
            ".gitignore",
            "README.md",
            "deck.md",
            "theme.yaml",
            "src/lib.rs",
            "tests/golden.rs",
        ] {
            assert_eq!(
                std::fs::read_to_string(target.join(file)).unwrap(),
                std::fs::read_to_string(example.join(file)).unwrap(),
                "{file}"
            );
        }
        let cargo = std::fs::read_to_string(target.join("Cargo.toml")).unwrap();
        assert!(cargo.contains("name = \"template-engine\""));
        assert!(cargo.contains(&format!("mdeck-sdk = \"{}\"", env!("CARGO_PKG_VERSION"))));

        // Refuses to overwrite.
        let err = create(TemplateKind::Engine, "template-engine", &target)
            .unwrap_err()
            .to_string();
        assert!(err.contains("not empty"), "{err}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
