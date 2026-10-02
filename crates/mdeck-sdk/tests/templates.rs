//! The scaffold templates for `mdeck sdk new <kind> <name>` (EXT-28).
//!
//! Building every template in a test would be slow, so each template is
//! instantiated once, as the workspace crates `examples/template-engine`,
//! `template-visual`, `template-design-set` and `template-transition`,
//! which CI builds, lints and tests (EXT-15). This test keeps those copies
//! identical to what the templates produce, and checks every template for
//! the files the scaffold promises and for unknown placeholders.

use std::path::{Path, PathBuf};

const KINDS: [&str; 4] = ["engine", "visual", "design-set", "transition"];
const FILES: [&str; 7] = [
    "Cargo.toml",
    ".gitignore",
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
/// as `Cargo.toml.tmpl`, since `cargo package` leaves out nested manifests,
/// and `.gitignore` as `gitignore.tmpl`.
fn source(kind: &str, file: &str) -> PathBuf {
    let stored = match file {
        "Cargo.toml" => "Cargo.toml.tmpl",
        ".gitignore" => "gitignore.tmpl",
        file => file,
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
                file == ".gitignore" || text.contains("{{name}}"),
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
fn no_template_stores_a_cargo_toml_or_gitignore() {
    // `cargo package` would leave the manifest out of the published SDK,
    // and honour the `.gitignore`.
    for kind in KINDS {
        for file in ["Cargo.toml", ".gitignore"] {
            assert!(!templates().join(kind).join(file).exists(), "{kind}/{file}");
        }
    }
}

#[test]
fn the_examples_are_the_instantiated_templates() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    if !examples.exists() {
        return; // outside the mdeck repository (a packaged crate)
    }
    for kind in KINDS {
        let name = format!("template-{kind}");
        let example = examples.join(&name);
        for file in [
            ".gitignore",
            "src/lib.rs",
            "tests/golden.rs",
            "theme.yaml",
            "deck.md",
            "README.md",
        ] {
            let template = std::fs::read_to_string(source(kind, file)).unwrap();
            let copy = std::fs::read_to_string(example.join(file)).unwrap_or_else(|e| {
                panic!("examples/{name}/{file}: {e}: instantiate the {kind} template there")
            });
            assert_eq!(
                instantiate(&template, &name),
                copy,
                "examples/{name}/{file} differs from templates/{kind}/{file}: \
                 regenerate the example from the template"
            );
        }
    }
}
