//! EXT-24: no egui (or glow) type in the stable public API.
//!
//! A plain source scan: every `pub` item signature and `pub` field outside
//! the hidden `host` module must not name `egui` or `glow`, unless it sits
//! behind the `unstable-egui` feature. Trait impls cannot leak either: the
//! egui conversions are crate-private traits.

use std::path::Path;

fn files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The signature starting at line `i`: up to the first `{` or `;` (or
/// the end of a field line).
fn signature(lines: &[&str], i: usize) -> String {
    let mut sig = String::new();
    for line in &lines[i..] {
        sig.push_str(line);
        sig.push('\n');
        if line.contains('{') || line.contains(';') || line.trim_end().ends_with(',') {
            break;
        }
    }
    sig
}

#[test]
fn public_items_name_no_third_party_types() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut all = Vec::new();
    files(&src, &mut all);
    let mut leaks = Vec::new();
    for path in all {
        if path.ends_with("host.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        // stop at the unit tests
        let end = lines
            .iter()
            .position(|l| l.trim() == "#[cfg(test)]")
            .unwrap_or(lines.len());
        for i in 0..end {
            let t = lines[i].trim_start();
            if !t.starts_with("pub ") {
                continue;
            }
            // skip items gated on the unstable feature (attributes and docs above)
            let gated = lines[..i]
                .iter()
                .rev()
                .take_while(|l| {
                    let l = l.trim_start();
                    l.starts_with("#[") || l.starts_with("///")
                })
                .any(|l| l.contains("unstable-egui"));
            if gated {
                continue;
            }
            let sig = signature(&lines, i);
            if sig.contains("egui") || sig.contains("glow") {
                leaks.push(format!("{}:{}: {}", path.display(), i + 1, sig.trim()));
            }
        }
    }
    assert!(
        leaks.is_empty(),
        "third-party types in the public API:\n{}",
        leaks.join("\n")
    );
}

#[test]
fn version_matches_the_package() {
    assert_eq!(mdeck_sdk::VERSION, env!("CARGO_PKG_VERSION"));
}
