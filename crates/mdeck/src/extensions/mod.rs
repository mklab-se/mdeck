//! What extends this mdeck: installed [`packs`], [`external`] visual
//! programs, and what the binary registers (built-ins, and the extension
//! crates of a custom build).
//!
//! [`provided`] is the seam to the registry: until phase 2b hands the
//! running `mdeck_sdk::registry::Registry` over, it lists the built-ins this
//! binary knows by name. Replace its body with a walk over the registry
//! (engines, visuals, transitions, design sets, themes with their origin).

pub mod external;
pub mod packs;

use std::path::Path;

/// The kinds of things an mdeck binary provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Engine,
    Visual,
    Transition,
    Theme,
}

impl Kind {
    pub fn plural(self) -> &'static str {
        match self {
            Kind::Engine => "Engines",
            Kind::Visual => "Visuals",
            Kind::Transition => "Transitions",
            Kind::Theme => "Themes",
        }
    }
}

/// One thing the binary provides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provided {
    pub kind: Kind,
    pub name: String,
    /// `built-in`, or the extension crate that registered it.
    pub origin: String,
}

/// Everything this binary provides, by kind and name.
///
/// Integration seam (phase 2b): build this from the registry `mdeck::run`
/// received, with each entry's origin, instead of the built-in lists.
pub fn provided() -> Vec<Provided> {
    let builtin = |kind: Kind, name: &str| Provided {
        kind,
        name: name.to_string(),
        origin: "built-in".to_string(),
    };
    let mut out = Vec::new();
    out.extend(
        crate::engines::EngineKind::ALL
            .iter()
            .map(|k| builtin(Kind::Engine, k.name())),
    );
    out.extend(
        crate::language::FENCES
            .iter()
            .filter(|f| f.kind == crate::language::FenceKind::Visual)
            .map(|f| builtin(Kind::Visual, f.tag)),
    );
    out.extend(
        crate::language::TRANSITIONS
            .iter()
            .map(|t| builtin(Kind::Transition, t)),
    );
    out.extend(
        crate::theme::lookup::BUILTIN
            .iter()
            .map(|(n, _)| builtin(Kind::Theme, n)),
    );
    out
}

/// The names `requires:` can be satisfied by: installed packs (the deck's
/// and the user's), extension crates in this binary, and everything the
/// binary provides by name.
pub fn installed_names(deck_dir: Option<&Path>) -> Vec<String> {
    let mut names: Vec<String> = packs::installed(deck_dir)
        .into_iter()
        .map(|p| p.manifest.name)
        .collect();
    for p in provided() {
        if p.origin != "built-in" {
            names.push(p.origin);
        }
        names.push(p.name.trim_start_matches('@').to_string());
    }
    names.sort();
    names.dedup();
    names
}

/// The names in a `requires:` value: `[a, b]`, `a, b` or `a b`.
pub fn parse_requires(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split([',', ' '])
        .map(|s| s.trim().trim_matches(['"', '\'']))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_values_parse_in_every_form() {
        assert_eq!(parse_requires("[acme-brand, glow]"), ["acme-brand", "glow"]);
        assert_eq!(parse_requires("acme-brand,glow"), ["acme-brand", "glow"]);
        assert_eq!(parse_requires("\"a\" 'b'"), ["a", "b"]);
        assert!(parse_requires("[]").is_empty());
    }

    #[test]
    fn the_builtins_are_provided() {
        let p = provided();
        assert!(
            p.iter()
                .any(|p| p.kind == Kind::Engine && p.name == "plain")
        );
        assert!(p.iter().any(|p| p.kind == Kind::Visual && p.name == "@bar"));
        assert!(
            p.iter()
                .any(|p| p.kind == Kind::Transition && p.name == "fade")
        );
        assert!(p.iter().any(|p| p.kind == Kind::Theme && p.name == "dark"));
        let names = installed_names(None);
        assert!(names.contains(&"plain".to_string()));
        assert!(names.contains(&"bar".to_string()));
    }
}
