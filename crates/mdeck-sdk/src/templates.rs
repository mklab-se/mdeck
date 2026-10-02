//! The scaffold templates `mdeck sdk new <kind> <name>` writes (EXT-28),
//! embedded in the SDK so an installed mdeck carries them. Hidden: mdeck
//! uses it; extensions have no need to.

macro_rules! template {
    ($kind:literal) => {
        &[
            // Stored as `Cargo.toml.tmpl`: `cargo package` drops any
            // `Cargo.toml` below the crate root, taking it for a nested crate.
            template!(@file $kind, "Cargo.toml", "Cargo.toml.tmpl"),
            // Stored as `gitignore.tmpl`: `cargo package` would honour a
            // `.gitignore` and leave out the files it names, and a dotfile
            // is easy to lose.
            template!(@file $kind, ".gitignore", "gitignore.tmpl"),
            template!(@file $kind, "README.md"),
            template!(@file $kind, "deck.md"),
            template!(@file $kind, "theme.yaml"),
            template!(@file $kind, "src/lib.rs"),
            template!(@file $kind, "tests/golden.rs"),
        ]
    };
    (@file $kind:literal, $file:literal) => {
        template!(@file $kind, $file, $file)
    };
    (@file $kind:literal, $file:literal, $source:literal) => {
        (
            $file,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/templates/",
                $kind,
                "/",
                $source
            )),
        )
    };
}

const ENGINE: &[(&str, &str)] = template!("engine");
const VISUAL: &[(&str, &str)] = template!("visual");
const DESIGN_SET: &[(&str, &str)] = template!("design-set");
const TRANSITION: &[(&str, &str)] = template!("transition");

/// The files of the template for `kind` (`engine`, `visual`, `design-set`,
/// `transition`): path in the new crate and text, with `{{name}}`,
/// `{{crate_name}}` and `{{sdk_version}}` placeholders.
///
/// ```
/// let files = mdeck_sdk::templates::files("engine").unwrap();
/// assert!(files.iter().any(|(path, _)| *path == "src/lib.rs"));
/// assert!(mdeck_sdk::templates::files("widget").is_none());
/// ```
pub fn files(kind: &str) -> Option<&'static [(&'static str, &'static str)]> {
    Some(match kind {
        "engine" => ENGINE,
        "visual" => VISUAL,
        "design-set" => DESIGN_SET,
        "transition" => TRANSITION,
        _ => return None,
    })
}
