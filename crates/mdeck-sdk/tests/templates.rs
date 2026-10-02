//! The scaffold templates for `mdeck sdk new <kind> <name>` (EXT-28).
//!
//! Building every template in a test would be slow, so the engine template
//! is instantiated once, as the workspace crate `examples/template-engine`,
//! which CI builds, lints and tests. This test keeps that copy identical to
//! what the template produces, and checks every template for the files the
//! scaffold promises and for unknown placeholders.

use std::path::{Path, PathBuf};

const KINDS: [&str; 4] = ["engine", "visual", "design-set", "transition"];
const FILES: [&str; 6] = [
    "Cargo.toml",
    "README.md",
    "deck.md",
    "theme.yaml",
    "src/lib.rs",
    "tests/golden.rs",
];
const PLACEHOLDERS: [&str; 3] = ["{{name}}", "{{crate_name}}", "{{sdk_version}}"];

fn templates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("templates")
}

/// Where `file` of the template for `kind` is stored. `Cargo.toml` is kept
/// as `Cargo.toml.tmpl`, since `cargo package` leaves out nested manifests.
fn source(kind: &str, file: &str) -> PathBuf {
    let stored = if file == "Cargo.toml" {
        "Cargo.toml.tmpl"
    } else {
        file
    };
    templates().join(kind).join(stored)
}

/// What `mdeck sdk new` writes for `name`.
fn instantiate(text: &str, name: &str) -> String {
    text.replace("{{name}}", name)
        .replace("{{crate_name}}", &name.replace('-', "_"))
        .replace("{{sdk_version}}", env!("CARGO_PKG_VERSION"))
}

#[test]
fn every_template_has_the_promised_files() {
    for kind in KINDS {
        for file in FILES {
            let path = source(kind, file);
            assert!(path.is_file(), "missing {}", path.display());
        }
    }
}

#[test]
fn templates_use_only_known_placeholders() {
    for kind in KINDS {
        for file in FILES {
            let text = std::fs::read_to_string(source(kind, file)).unwrap();
            let mut rest = text.as_str();
            while let Some(i) = rest.find("{{") {
                let end = rest[i..].find("}}").map_or(rest.len(), |j| i + j + 2);
                let token = &rest[i..end];
                assert!(
                    PLACEHOLDERS.contains(&token),
                    "{kind}/{file}: unknown placeholder {token}"
                );
                rest = &rest[end..];
            }
            assert!(
                text.contains("{{name}}"),
                "{kind}/{file} never uses the name"
            );
        }
    }
}

#[test]
fn templates_avoid_em_dashes() {
    for kind in KINDS {
        for file in FILES {
            let text = std::fs::read_to_string(source(kind, file)).unwrap();
            assert!(!text.contains('\u{2014}'), "{kind}/{file} has an em-dash");
        }
    }
}

#[test]
fn files_match_the_template_folders() {
    for kind in KINDS {
        let files = mdeck_sdk::templates::files(kind).unwrap();
        let paths: Vec<&str> = files.iter().map(|(path, _)| *path).collect();
        assert_eq!(paths, FILES, "{kind}");
        for (path, text) in files {
            let stored = std::fs::read_to_string(source(kind, path)).unwrap();
            assert_eq!(*text, stored, "{kind}/{path}");
        }
    }
}

#[test]
fn no_template_stores_a_cargo_toml() {
    // `cargo package` would leave it out of the published SDK.
    for kind in KINDS {
        assert!(
            !templates().join(kind).join("Cargo.toml").exists(),
            "{kind}"
        );
    }
}

#[test]
fn the_engine_example_is_the_instantiated_template() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/template-engine");
    if !example.exists() {
        return; // outside the mdeck repository (a packaged crate)
    }
    for file in [
        "src/lib.rs",
        "tests/golden.rs",
        "theme.yaml",
        "deck.md",
        "README.md",
    ] {
        let template = std::fs::read_to_string(templates().join("engine").join(file)).unwrap();
        let copy = std::fs::read_to_string(example.join(file)).unwrap();
        assert_eq!(
            instantiate(&template, "template-engine"),
            copy,
            "examples/template-engine/{file} differs from templates/engine/{file}: \
             regenerate the example from the template"
        );
    }
}
