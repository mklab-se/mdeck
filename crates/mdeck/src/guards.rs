//! Guard tests that keep the places naming the built-in engines in step:
//! each engine has a showcase theme and a sample deck (ENG-17), and the
//! build script, the cargo features and the CI matrix list the same engines.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    let p = repo().join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("reading {}: {e}", p.display()))
}

/// The quoted names in `["a", "b", ...]` that follows `start` in `text`.
fn quoted_list(text: &str, start: &str) -> BTreeSet<String> {
    let from = text.find(start).unwrap_or_else(|| panic!("no `{start}`"));
    let rest = &text[from + start.len()..];
    let open = rest.find('[').expect("a list");
    let close = rest[open..].find(']').expect("a closed list") + open;
    rest[open + 1..close]
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The engines `crates/mdeck/build.rs` turns `all_engines` on for.
fn build_rs_engines(text: &str) -> BTreeSet<String> {
    quoted_list(text, "const ENGINES: &[&str] =")
        .into_iter()
        .map(|s| s.to_lowercase())
        .collect()
}

/// The engine features: the default feature list of `crates/mdeck/Cargo.toml`
/// (every engine, and nothing else, is on by default).
fn cargo_engines(text: &str) -> BTreeSet<String> {
    quoted_list(text, "\ndefault =")
}

/// The feature sets in the CI workflow's engine matrix (without the empty
/// one, the build with no engine).
fn ci_engines(text: &str) -> BTreeSet<String> {
    let from = text.find("matrix:").expect("an engine matrix");
    let mut lines = text[from..].lines().skip(1);
    assert_eq!(
        lines.next().map(str::trim),
        Some("features:"),
        "the matrix lists features"
    );
    lines
        .map(str::trim)
        .take_while(|l| l.starts_with("- "))
        .map(|l| l.trim_start_matches("- ").trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn build_script_features_and_ci_name_the_same_engines() {
    let build = build_rs_engines(&read("crates/mdeck/build.rs"));
    let cargo = cargo_engines(&read("crates/mdeck/Cargo.toml"));
    let ci = ci_engines(&read(".github/workflows/ci.yml"));
    assert!(build.len() >= 9, "{build:?}");
    assert_eq!(build, cargo, "build.rs ENGINES vs Cargo.toml features");
    assert_eq!(build, ci, "build.rs ENGINES vs the CI engine matrix");
    // a full build registers exactly those engines, plus plain
    #[cfg(all_engines)]
    {
        let registered: BTreeSet<String> = crate::registry::get()
            .engines()
            .map(|d| d.name.to_string())
            .filter(|n| n != "plain")
            .collect();
        assert_eq!(build, registered, "build.rs ENGINES vs the registry");
    }
}

#[test]
fn the_parsers_read_what_they_should() {
    let build = "const ENGINES: &[&str] = &[\n    \"PARTICLES\",\n    \"LED\",\n];";
    assert_eq!(
        build_rs_engines(build),
        ["led", "particles"].map(String::from).into()
    );
    let cargo = "[features]\ndefault = [\n    \"led\",\n]\nled = []\n";
    assert_eq!(cargo_engines(cargo), ["led"].map(String::from).into());
    let ci = "    strategy:\n      matrix:\n        features:\n          - \"\"\n          - led\n    env:\n";
    assert_eq!(ci_engines(ci), ["led"].map(String::from).into());
}

#[test]
fn every_engine_has_a_showcase_theme_and_a_sample_deck() {
    // ENG-17: plain is the default engine, shown by the `dark` theme
    let themes: Vec<(&str, crate::theme::Theme)> = crate::theme::lookup::BUILTIN
        .iter()
        .map(|(name, _)| {
            let t =
                crate::theme::lookup::load_builtin(name).unwrap_or_else(|e| panic!("{name}: {e}"));
            (*name, t)
        })
        .collect();
    for engine in crate::registry::get().engines() {
        let name = engine.name;
        let showcase: Vec<&str> = themes
            .iter()
            .filter(|(_, t)| t.engine.name() == name)
            .map(|(n, _)| *n)
            .collect();
        assert!(
            !showcase.is_empty(),
            "engine `{name}` has no built-in theme that selects it"
        );
        if name == "plain" {
            assert!(showcase.contains(&"dark"), "{showcase:?}");
            continue;
        }
        let samples = repo().join("samples/engines");
        let deck = std::iter::once(name)
            .chain(showcase.iter().copied())
            .map(|n| samples.join(format!("{n}.md")))
            .find(|p| p.exists())
            .unwrap_or_else(|| {
                panic!("engine `{name}` has no samples/engines/<engine or theme>.md")
            });
        let text = std::fs::read_to_string(&deck).unwrap();
        assert!(
            showcase
                .iter()
                .any(|t| text.contains(&format!("theme: {t}\n"))),
            "{} does not use a showcase theme of `{name}` ({showcase:?})",
            deck.display()
        );
    }
}
