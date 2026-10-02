//! `requires:` (EXT-11): the packs and extensions a deck expects, and a
//! warning for each one this mdeck does not have.

use std::path::Path;

use crate::check::{CheckCategory, CheckWarning};
use crate::extensions;
use crate::parser;

pub fn requires_warnings(
    presentation: &parser::Presentation,
    deck_dir: &Path,
) -> Vec<CheckWarning> {
    let Some(setting) = presentation
        .meta
        .settings
        .iter()
        .rev()
        .find(|s| s.name == "requires")
    else {
        return Vec::new();
    };
    missing(
        &extensions::parse_requires(&setting.value),
        &extensions::installed_names(Some(deck_dir)),
    )
    .into_iter()
    .map(|name| CheckWarning {
        slide: 0,
        line: setting.line,
        category: CheckCategory::Extensions,
        message: format!(
            "this deck expects `{name}`, which is not installed: install the pack \
             (`mdeck pack install <path|git-url>`) or use an mdeck built with the \
             extension (`mdeck build --with <path|crate>`)"
        ),
    })
    .collect()
}

/// The required names that are not installed.
fn missing(required: &[String], installed: &[String]) -> Vec<String> {
    required
        .iter()
        .filter(|r| !installed.contains(r))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_missing_names_warn() {
        let installed = vec!["acme-brand".to_string(), "plain".to_string()];
        let required = extensions::parse_requires("[acme-brand, glow, plain]");
        assert_eq!(missing(&required, &installed), ["glow"]);
    }

    #[test]
    fn a_deck_requiring_an_unknown_pack_is_warned() {
        let pres = parser::parse("---\nrequires: [no-such-pack-xyz, plain]\n---\n\n# One\n");
        let w = requires_warnings(&pres, Path::new("/nonexistent-deck-dir"));
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].message.contains("`no-such-pack-xyz`"));
        assert_eq!(w[0].category, CheckCategory::Extensions);
        assert_eq!(w[0].line, 2);
    }

    #[test]
    fn no_requires_no_warnings() {
        let pres = parser::parse("# One\n");
        assert!(requires_warnings(&pres, Path::new(".")).is_empty());
    }
}
