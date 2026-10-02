//! Generated assets: everything `mdeck ai` makes for a deck lives in one
//! folder next to it, `talk.assets/` for `talk.md`, recorded in one
//! [`manifest`] (`talk.assets/manifest.yaml`):
//!
//! - `artworks/`: a picture per slide for the art engines;
//! - `images/`: images for `![prompt](generate:)` placeholders;
//! - `icons/`: diagram icons for `(icon: generate:, prompt: "...")`;
//! - `point-clouds/`: point clouds for `@illustration` names.
//!
//! Generation never rewrites the deck: [`placeholders`] stay in the source
//! and are resolved through the manifest when the deck opens. One
//! [`style`] system covers every kind. Presenting and exporting never call
//! an AI.

pub mod manifest;
pub mod placeholders;
pub mod style;

use std::path::Path;

use crate::parser::Presentation;

/// Resolve a freshly parsed deck's placeholders through its manifest, in
/// memory. Returns a problem reading the manifest, if there was one (the
/// placeholders then stay placeholders).
pub fn resolve_placeholders(pres: &mut Presentation, deck: &Path) -> Option<String> {
    let (manifest, problem) = match manifest::load(deck) {
        Ok(m) => (m, None),
        Err(e) => (None, Some(e.to_string())),
    };
    if manifest.is_some() {
        let config = crate::config::Config::load_or_default();
        let styles = style::resolve(&config, &pres.meta, None);
        placeholders::apply(pres, deck, manifest.as_ref(), &styles);
    }
    problem
}

/// The folder `@illustration` point clouds generated for this deck go in.
pub fn point_cloud_dir(deck: &Path) -> std::path::PathBuf {
    manifest::folder_for(deck).join(manifest::Kind::PointCloud.folder())
}
