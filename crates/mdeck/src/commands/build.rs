//! `mdeck build --with <path|crate[@version]>...` (EXT-14, EXT-17, D15): an
//! mdeck with extension crates built in, without touching mdeck's source.
//!
//! It generates a small cargo project in the cache folder
//! (`<cache>/mdeck/build/<hash>/`, one per set of inputs, so a rebuild with
//! the same extensions is incremental) whose `main` registers mdeck's
//! built-ins and then each extension, and hands the registry to mdeck:
//!
//! ```rust,ignore
//! let mut registry = mdeck_sdk::registry::Registry::new();
//! mdeck::builtins(&mut registry);
//! registry.set_origin("glow");
//! glow::register(&mut registry)?;
//! mdeck::run(registry)
//! ```
//!
//! mdeck comes from the source checkout this binary was built from (or
//! `--mdeck-path`, or `MDECK_SOURCE`), else from crates.io at exactly this
//! binary's version.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use colored::Colorize;

/// The environment variable that points `mdeck build` at an mdeck checkout.
pub const SOURCE_ENV: &str = "MDECK_SOURCE";

/// An extension to build in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Extension {
    /// A crate on disk: its folder and package name.
    Path { dir: PathBuf, package: String },
    /// A crate from crates.io, optionally at a version requirement.
    Crate {
        name: String,
        version: Option<String>,
    },
}

impl Extension {
    /// `./glow`, `../acme`, `/abs/path` (a folder with a `Cargo.toml`), or
    /// `name` / `name@1.2`.
    pub fn parse(arg: &str) -> Result<Self> {
        let path = Path::new(arg);
        let looks_like_path =
            path.exists() || arg.starts_with('.') || arg.contains('/') || arg.contains('\\');
        if looks_like_path {
            let manifest = path.join("Cargo.toml");
            let text = std::fs::read_to_string(&manifest).map_err(|_| {
                anyhow!("{arg}: no Cargo.toml there; --with takes a crate folder or a crate name")
            })?;
            let package = package_name(&text)
                .ok_or_else(|| anyhow!("{}: no [package] name", manifest.display()))?;
            let dir = std::fs::canonicalize(path)
                .with_context(|| format!("resolving {}", path.display()))?;
            return Ok(Extension::Path { dir, package });
        }
        let (name, version) = match arg.split_once('@') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (arg, None),
        };
        let ok = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !ok {
            bail!("`{arg}` is neither a crate folder nor a crate name");
        }
        Ok(Extension::Crate {
            name: name.to_string(),
            version,
        })
    }

    /// The package name.
    pub fn package(&self) -> &str {
        match self {
            Extension::Path { package, .. } => package,
            Extension::Crate { name, .. } => name,
        }
    }

    /// The name Rust code uses for the crate.
    pub fn ident(&self) -> String {
        self.package().replace('-', "_")
    }

    fn dependency(&self) -> String {
        match self {
            Extension::Path { dir, package } => {
                format!("{} = {{ path = {} }}", quote(package), path_literal(dir))
            }
            Extension::Crate { name, version } => format!(
                "{} = {}",
                quote(name),
                quote(version.as_deref().unwrap_or("*"))
            ),
        }
    }
}

/// The `name` in a Cargo.toml's `[package]` table.
fn package_name(cargo_toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in cargo_toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package
            && let Some(rest) = line.strip_prefix("name")
            && let Some(value) = rest.trim_start().strip_prefix('=')
        {
            return Some(value.trim().trim_matches(['"', '\'']).to_string());
        }
    }
    None
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A path as a TOML string (forward slashes work on every platform).
fn path_literal(p: &Path) -> String {
    quote(&p.to_string_lossy().replace('\\', "/"))
}

/// Where mdeck and mdeck-sdk come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdeckSource {
    /// An mdeck repository checkout (its root).
    Checkout(PathBuf),
    /// crates.io, at exactly this version.
    Published(String),
}

impl MdeckSource {
    /// `--mdeck-path`, then `MDECK_SOURCE`, then the checkout this binary was
    /// built from (when it is still there), then crates.io.
    pub fn detect(explicit: Option<PathBuf>) -> Result<Self> {
        let given = explicit.or_else(|| std::env::var_os(SOURCE_ENV).map(PathBuf::from));
        if let Some(root) = given {
            if !is_checkout(&root) {
                bail!(
                    "{} is not an mdeck checkout (no crates/mdeck and crates/mdeck-sdk)",
                    root.display()
                );
            }
            return Ok(MdeckSource::Checkout(std::fs::canonicalize(&root)?));
        }
        let built_from = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        if built_from.join(".git").exists() && is_checkout(&built_from) {
            return Ok(MdeckSource::Checkout(std::fs::canonicalize(&built_from)?));
        }
        Ok(MdeckSource::Published(
            env!("CARGO_PKG_VERSION").to_string(),
        ))
    }

    fn dependency(&self, krate: &str) -> String {
        match self {
            MdeckSource::Checkout(root) => format!(
                "{krate} = {{ path = {} }}",
                path_literal(&root.join("crates").join(krate))
            ),
            MdeckSource::Published(v) => format!("{krate} = {}", quote(&format!("={v}"))),
        }
    }
}

fn is_checkout(root: &Path) -> bool {
    root.join("crates/mdeck/Cargo.toml").is_file()
        && root.join("crates/mdeck-sdk/Cargo.toml").is_file()
}

/// The generated cargo project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub cargo_toml: String,
    pub main_rs: String,
    pub bin: String,
}

impl Project {
    pub fn generate(extensions: &[Extension], source: &MdeckSource, bin: &str) -> Project {
        let mut toml = String::from(
            "# Generated by `mdeck build`; run it again rather than editing this file.\n\
             [package]\n\
             name = \"mdeck-custom-build\"\n\
             version = \"0.0.0\"\n\
             edition = \"2024\"\n\
             publish = false\n\n",
        );
        toml.push_str(&format!(
            "[[bin]]\nname = {}\npath = \"main.rs\"\n\n[dependencies]\n",
            quote(bin)
        ));
        toml.push_str(&source.dependency("mdeck"));
        toml.push('\n');
        toml.push_str(&source.dependency("mdeck-sdk"));
        toml.push('\n');
        for e in extensions {
            toml.push_str(&e.dependency());
            toml.push('\n');
        }
        if let MdeckSource::Checkout(_) = source {
            // Extensions depend on mdeck-sdk from crates.io; build them
            // against the checkout's copy so there is one SDK.
            toml.push_str("\n[patch.crates-io]\n");
            toml.push_str(&source.dependency("mdeck-sdk"));
            toml.push('\n');
        }
        toml.push_str("\n[profile.release]\nlto = \"thin\"\n\n# Not part of any surrounding workspace.\n[workspace]\n");

        let mut main = String::from(
            "//! Generated by `mdeck build`: mdeck with extensions built in.\n\n\
             fn main() -> impl std::process::Termination {\n    \
             let mut registry = ::mdeck_sdk::registry::Registry::new();\n    \
             ::mdeck::builtins(&mut registry);\n",
        );
        for e in extensions {
            main.push_str(&format!(
                "    registry.set_origin({name});\n    \
                 if let Err(e) = ::{ident}::register(&mut registry) {{\n        \
                 eprintln!(\"Error: the extension `{package}` could not register: {{e}}\");\n        \
                 std::process::exit(1);\n    }}\n",
                name = quote(e.package()),
                ident = e.ident(),
                package = e.package(),
            ));
        }
        main.push_str("    ::mdeck::run(registry)\n}\n");
        Project {
            cargo_toml: toml,
            main_rs: main,
            bin: bin.to_string(),
        }
    }

    /// A stable name for these inputs.
    pub fn hash(&self) -> String {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in self.cargo_toml.bytes().chain(self.main_rs.bytes()) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        format!("{h:016x}")
    }

    /// Write the project into `dir` (files that already match are left
    /// alone, so cargo's freshness checks hold).
    pub fn write(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        for (file, text) in [("Cargo.toml", &self.cargo_toml), ("main.rs", &self.main_rs)] {
            let path = dir.join(file);
            if std::fs::read_to_string(&path).ok().as_ref() != Some(text) {
                std::fs::write(&path, text)
                    .with_context(|| format!("writing {}", path.display()))?;
            }
        }
        Ok(())
    }
}

/// The cache folder custom builds live in.
pub fn build_root() -> Result<PathBuf> {
    dirs::cache_dir()
        .map(|d| d.join("mdeck").join("build"))
        .ok_or_else(|| anyhow!("could not find a cache folder for the build"))
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

fn require_cargo() -> Result<()> {
    let ok = Command::new(cargo())
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        bail!(
            "cargo was not found. `mdeck build` compiles mdeck with your extensions and needs \
             a Rust toolchain: install it from https://rustup.rs, then run this again"
        );
    }
    Ok(())
}

/// Options for [`build`].
#[derive(Debug, Clone)]
pub struct BuildArgs {
    pub with: Vec<String>,
    pub out: Option<PathBuf>,
    pub name: String,
    pub mdeck_path: Option<PathBuf>,
    pub quiet: bool,
}

/// Where the binary goes: `--out` (a file, or a folder to put it in), else
/// `./target/release/<name>`.
pub fn output_path(out: Option<PathBuf>, bin: &str) -> PathBuf {
    let file = format!("{bin}{}", std::env::consts::EXE_SUFFIX);
    match out {
        Some(p) if p.is_dir() => p.join(file),
        Some(p) => p,
        None => Path::new("target").join("release").join(file),
    }
}

/// `mdeck build`: generate, compile, install. Returns the binary's path.
pub fn build(args: BuildArgs) -> Result<PathBuf> {
    if args.with.is_empty() {
        bail!("name at least one extension: `mdeck build --with <path|crate>`");
    }
    crate::commands::sdk::validate_name(&args.name)
        .map_err(|_| anyhow!("`{}` is not a usable binary name", args.name))?;
    let extensions = args
        .with
        .iter()
        .map(|a| Extension::parse(a))
        .collect::<Result<Vec<_>>>()?;
    let source = MdeckSource::detect(args.mdeck_path)?;
    let project = Project::generate(&extensions, &source, &args.name);
    require_cargo()?;

    let root = build_root()?;
    let dir = root.join(project.hash());
    project.write(&dir)?;
    let target = root.join("target");
    if !args.quiet {
        let names: Vec<&str> = extensions.iter().map(Extension::package).collect();
        eprintln!(
            "Building mdeck with {} ({})",
            names.join(", ").bold(),
            match &source {
                MdeckSource::Checkout(p) => format!("mdeck from {}", p.display()),
                MdeckSource::Published(v) => format!("mdeck {v} from crates.io"),
            }
        );
    }
    let mut cmd = Command::new(cargo());
    cmd.args(["build", "--release", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target);
    if args.quiet {
        cmd.arg("--quiet");
    }
    let status = cmd
        .status()
        .map_err(|e| anyhow!("could not run cargo: {e}"))?;
    if !status.success() {
        bail!(
            "the build failed (see cargo's output above); the generated project is in {}",
            dir.display()
        );
    }

    let built =
        target
            .join("release")
            .join(format!("{}{}", args.name, std::env::consts::EXE_SUFFIX));
    let out = output_path(args.out, &args.name);
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    // Unlink first: replacing a binary that is running (this one, say) in
    // place would corrupt it, while a new file leaves the old one intact.
    if out.exists() {
        std::fs::remove_file(&out).with_context(|| format!("replacing {}", out.display()))?;
    }
    std::fs::copy(&built, &out)
        .with_context(|| format!("copying {} to {}", built.display(), out.display()))?;
    Ok(out)
}

/// `mdeck build` from the command line: prints the binary's path.
pub fn run(args: BuildArgs) -> Result<()> {
    let out = build(args)?;
    println!("{}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkout() -> PathBuf {
        std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
    }

    #[test]
    fn extensions_parse_as_paths_or_crates() {
        let ambience = checkout().join("examples/engine-ambience");
        let e = Extension::parse(ambience.to_str().unwrap()).unwrap();
        assert_eq!(e.package(), "engine-ambience");
        assert_eq!(e.ident(), "engine_ambience");

        assert_eq!(
            Extension::parse("acme-brand-engines@1.2").unwrap(),
            Extension::Crate {
                name: "acme-brand-engines".into(),
                version: Some("1.2".into())
            }
        );
        assert_eq!(
            Extension::parse("glow").unwrap(),
            Extension::Crate {
                name: "glow".into(),
                version: None
            }
        );
        let err = Extension::parse("./no-such-extension")
            .unwrap_err()
            .to_string();
        assert!(err.contains("no Cargo.toml"), "{err}");
        assert!(Extension::parse("bad name!").is_err());
    }

    #[test]
    fn package_names_come_from_the_package_table() {
        let toml = "[workspace]\nname = \"nope\"\n\n[package]\nversion = \"1\"\nname = \"glow\"\n";
        assert_eq!(package_name(toml).as_deref(), Some("glow"));
        assert_eq!(package_name("[dependencies]\nname = \"x\"\n"), None);
    }

    #[test]
    fn a_checkout_build_uses_paths_and_patches_the_sdk() {
        let root = checkout();
        let source = MdeckSource::Checkout(root.clone());
        let exts = vec![
            Extension::Path {
                dir: root.join("examples/engine-ambience"),
                package: "engine-ambience".into(),
            },
            Extension::Crate {
                name: "acme".into(),
                version: Some("1.2".into()),
            },
        ];
        let p = Project::generate(&exts, &source, "mdeck");
        let root_str = root.to_string_lossy().replace('\\', "/");
        assert!(
            p.cargo_toml
                .contains(&format!("mdeck = {{ path = \"{root_str}/crates/mdeck\" }}"))
        );
        assert!(p.cargo_toml.contains("[patch.crates-io]"));
        assert!(p.cargo_toml.contains("\"acme\" = \"1.2\""));
        assert!(p.cargo_toml.contains("\"engine-ambience\" = { path ="));
        assert!(p.cargo_toml.contains("[workspace]"));
        assert!(
            p.cargo_toml
                .contains("name = \"mdeck\"\npath = \"main.rs\"")
        );

        let main = &p.main_rs;
        let builtins = main.find("::mdeck::builtins(&mut registry);").unwrap();
        let ambience = main
            .find("::engine_ambience::register(&mut registry)")
            .unwrap();
        let acme = main.find("::acme::register(&mut registry)").unwrap();
        let run = main.find("::mdeck::run(registry)").unwrap();
        assert!(builtins < ambience && ambience < acme && acme < run);
        assert!(main.contains("registry.set_origin(\"engine-ambience\");"));
    }

    #[test]
    fn a_published_build_pins_this_version() {
        let source = MdeckSource::Published("2.0.0".into());
        let p = Project::generate(&[], &source, "acme-deck");
        assert!(p.cargo_toml.contains("mdeck = \"=2.0.0\""));
        assert!(p.cargo_toml.contains("mdeck-sdk = \"=2.0.0\""));
        assert!(!p.cargo_toml.contains("[patch.crates-io]"));
        assert!(p.cargo_toml.contains("name = \"acme-deck\""));
    }

    #[test]
    fn the_hash_follows_the_inputs() {
        let source = MdeckSource::Published("2.0.0".into());
        let a = Project::generate(&[], &source, "mdeck");
        let b = Project::generate(&[], &source, "mdeck");
        let c = Project::generate(&[], &source, "other");
        assert_eq!(a.hash(), b.hash());
        assert_ne!(a.hash(), c.hash());
    }

    #[test]
    fn the_output_defaults_to_target_release() {
        let exe = std::env::consts::EXE_SUFFIX;
        assert_eq!(
            output_path(None, "mdeck"),
            PathBuf::from(format!("target/release/mdeck{exe}"))
        );
        assert_eq!(
            output_path(Some("bin/my-mdeck".into()), "mdeck"),
            PathBuf::from("bin/my-mdeck")
        );
        let dir = std::env::temp_dir();
        assert_eq!(
            output_path(Some(dir.clone()), "x"),
            dir.join(format!("x{exe}"))
        );
    }

    #[test]
    fn the_checkout_is_detected() {
        assert_eq!(
            MdeckSource::detect(Some(checkout())).unwrap(),
            MdeckSource::Checkout(checkout())
        );
        assert!(MdeckSource::detect(Some(std::env::temp_dir())).is_err());
    }

    /// The real thing: compiles an mdeck with the ambience tutorial engine.
    /// Slow, and needs `mdeck::builtins` and `mdeck::run` (phase 2b).
    #[test]
    #[ignore = "compiles mdeck in release mode; needs mdeck::run and mdeck::builtins"]
    fn builds_an_mdeck_with_the_ambience_engine() {
        let out = std::env::temp_dir().join(format!("mdeck-build-test-{}", std::process::id()));
        let path = build(BuildArgs {
            with: vec![
                checkout()
                    .join("examples/engine-ambience")
                    .to_string_lossy()
                    .to_string(),
            ],
            out: Some(out.clone()),
            name: "mdeck".into(),
            mdeck_path: Some(checkout()),
            quiet: true,
        })
        .unwrap();
        assert_eq!(path, out);
        let version = Command::new(&path).arg("--version").output().unwrap();
        assert!(version.status.success());
        std::fs::remove_file(&out).ok();
    }
}
