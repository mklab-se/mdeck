//! Packs (EXT-09 to EXT-11): data extensions that need no compiler. A pack
//! is a folder (or a zip of one) with a manifest, `mdeck-pack.yaml`, and any
//! of the folders in [`Folder`].
//!
//! Packs install into the user folder (`packs/<name>/` under `dirs::config_dir()/mdeck/`) or a
//! deck's own `packs/<name>/`. Lookups that take files from packs ask
//! [`theme_dirs`], [`point_cloud_dirs`] or [`dirs_for`]: deck packs first,
//! then user packs, each in name order. They come after the deck's and the
//! user's own folders and before the built-ins (THM-04).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

/// The manifest file at the root of every pack.
pub const MANIFEST: &str = "mdeck-pack.yaml";

/// `mdeck-pack.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Manifest {
    /// Lowercase letters, digits and hyphens; also the install folder's name.
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// The oldest mdeck the pack works with, e.g. `2.0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_mdeck: Option<String>,
}

/// What a pack may carry, one folder each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Folder {
    Themes,
    Designs,
    PointClouds,
    Styles,
    Fonts,
}

impl Folder {
    pub const ALL: [Folder; 5] = [
        Folder::Themes,
        Folder::Designs,
        Folder::PointClouds,
        Folder::Styles,
        Folder::Fonts,
    ];

    pub fn dir(self) -> &'static str {
        match self {
            Folder::Themes => "themes",
            Folder::Designs => "designs",
            Folder::PointClouds => "point-clouds",
            Folder::Styles => "styles",
            Folder::Fonts => "fonts",
        }
    }
}

/// Where a pack is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The deck's own `packs/` folder.
    Deck,
    /// The user's packs folder.
    User,
}

impl Scope {
    pub fn label(self) -> &'static str {
        match self {
            Scope::Deck => "deck",
            Scope::User => "user",
        }
    }
}

/// An installed pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub manifest: Manifest,
    pub dir: PathBuf,
    pub scope: Scope,
}

impl Installed {
    /// How many files each folder holds, for folders that hold any.
    pub fn contents(&self) -> Vec<(Folder, usize)> {
        Folder::ALL
            .iter()
            .filter_map(|&f| {
                let n = std::fs::read_dir(self.dir.join(f.dir()))
                    .ok()?
                    .flatten()
                    .filter(|e| e.path().is_file())
                    .count();
                (n > 0).then_some((f, n))
            })
            .collect()
    }
}

/// The user's packs folder.
pub fn user_root() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("mdeck").join("packs"))
}

/// A deck's packs folder.
pub fn deck_root(deck_dir: &Path) -> PathBuf {
    deck_dir.join("packs")
}

/// Every pack installed for a deck in `deck_dir` (its own first) and for the
/// user, each group in name order.
pub fn installed(deck_dir: Option<&Path>) -> Vec<Installed> {
    installed_in(deck_dir.map(deck_root).as_deref(), user_root().as_deref())
}

/// [`installed`] with explicit roots.
pub fn installed_in(deck_root: Option<&Path>, user_root: Option<&Path>) -> Vec<Installed> {
    let mut out = Vec::new();
    for (root, scope) in [(deck_root, Scope::Deck), (user_root, Scope::User)] {
        let Some(root) = root else { continue };
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        let mut found: Vec<Installed> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .filter_map(|dir| {
                let manifest = read_manifest(&dir).ok()?;
                Some(Installed {
                    manifest,
                    dir,
                    scope,
                })
            })
            .collect();
        found.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        out.extend(found);
    }
    out
}

/// The `folder` of every installed pack that has one, in lookup order.
pub fn dirs_for(deck_dir: Option<&Path>, folder: Folder) -> Vec<PathBuf> {
    pack_dirs(&installed(deck_dir), folder)
}

fn pack_dirs(packs: &[Installed], folder: Folder) -> Vec<PathBuf> {
    packs
        .iter()
        .map(|p| p.dir.join(folder.dir()))
        .filter(|d| d.is_dir())
        .collect()
}

/// Theme folders from packs, for the theme lookup (after the deck's and
/// the user's `themes/`, before the built-ins).
pub fn theme_dirs(deck_dir: Option<&Path>) -> Vec<PathBuf> {
    dirs_for(deck_dir, Folder::Themes)
}

/// Point cloud folders from packs, for the illustration lookup.
pub fn point_cloud_dirs(deck_dir: Option<&Path>) -> Vec<PathBuf> {
    dirs_for(deck_dir, Folder::PointClouds)
}

/// Read and validate a pack's manifest.
pub fn read_manifest(dir: &Path) -> Result<Manifest> {
    let path = dir.join(MANIFEST);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("{} has no {MANIFEST}", dir.display()))?;
    let manifest: Manifest =
        serde_norway::from_str(&text).map_err(|e| anyhow!("{}: {e}", path.display()))?;
    validate_name(&manifest.name).map_err(|e| anyhow!("{}: {e}", path.display()))?;
    Ok(manifest)
}

pub fn validate_name(name: &str) -> Result<()> {
    let ok = name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        bail!(
            "pack name `{name}` must start with a letter and use only lowercase letters, digits and hyphens"
        );
    }
    Ok(())
}

/// Whether `running` is at least `minimum` (dotted numbers; missing parts
/// count as 0, anything after a `-` is ignored).
pub fn version_at_least(running: &str, minimum: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        v.trim()
            .trim_start_matches(['>', '=', 'v'])
            .split('-')
            .next()
            .unwrap_or("")
            .split('.')
            .map(|p| p.trim().parse().unwrap_or(0))
            .collect()
    }
    let (r, m) = (parts(running), parts(minimum));
    let n = r.len().max(m.len());
    let get = |v: &Vec<u64>, i: usize| v.get(i).copied().unwrap_or(0);
    for i in 0..n {
        match get(&r, i).cmp(&get(&m, i)) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => {}
        }
    }
    true
}

/// Where a pack comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    Dir(PathBuf),
    Zip(PathBuf),
    Git(String),
}

impl Origin {
    pub fn parse(arg: &str) -> Origin {
        let path = Path::new(arg);
        if path.exists() {
            if path.is_file() && path.extension().is_some_and(|e| e == "zip") {
                return Origin::Zip(path.to_path_buf());
            }
            return Origin::Dir(path.to_path_buf());
        }
        let git = arg.starts_with("https://")
            || arg.starts_with("http://")
            || arg.starts_with("ssh://")
            || arg.starts_with("git@")
            || arg.ends_with(".git");
        if git {
            Origin::Git(arg.to_string())
        } else {
            Origin::Dir(path.to_path_buf())
        }
    }
}

/// Install a pack from `origin` into `root` (a packs folder), replacing an
/// installed pack of the same name. Returns the installed pack.
pub fn install(origin: &Origin, root: &Path, scope: Scope) -> Result<Installed> {
    let staging = tempdir("pack")?;
    let result = install_via(origin, root, scope, &staging);
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn install_via(origin: &Origin, root: &Path, scope: Scope, staging: &Path) -> Result<Installed> {
    let source = match origin {
        Origin::Dir(dir) => {
            if !dir.is_dir() {
                bail!("{} is not a folder, a .zip or a git URL", dir.display());
            }
            dir.clone()
        }
        Origin::Zip(file) => {
            unzip(file, staging)?;
            pack_root(staging)?
        }
        Origin::Git(url) => {
            let target = staging.join("clone");
            let status = std::process::Command::new("git")
                .args(["clone", "--depth", "1", "--quiet", url])
                .arg(&target)
                .status()
                .map_err(|e| anyhow!("could not run git to clone {url}: {e}"))?;
            if !status.success() {
                bail!("git could not clone {url}");
            }
            pack_root(&target)?
        }
    };
    let manifest = read_manifest(&source)?;
    let running = env!("CARGO_PKG_VERSION");
    if let Some(min) = &manifest.min_mdeck
        && !version_at_least(running, min)
    {
        bail!(
            "pack `{}` needs mdeck {min} or later; this is mdeck {running}",
            manifest.name
        );
    }
    let dir = root.join(&manifest.name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
    }
    copy_tree(&source, &dir)?;
    Ok(Installed {
        manifest,
        dir,
        scope,
    })
}

/// Remove the pack `name` from `root`.
pub fn remove(root: &Path, name: &str) -> Result<PathBuf> {
    validate_name(name)?;
    let dir = root.join(name);
    if !dir.join(MANIFEST).is_file() {
        bail!("no pack named `{name}` in {}", root.display());
    }
    std::fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
    Ok(dir)
}

/// The folder holding the manifest: `dir` itself, or its only subfolder
/// (a zip of a folder, or a repository that keeps the pack one level down).
fn pack_root(dir: &Path) -> Result<PathBuf> {
    if dir.join(MANIFEST).is_file() {
        return Ok(dir.to_path_buf());
    }
    let subdirs: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !p.ends_with(".git") && !p.ends_with("__MACOSX"))
        .collect();
    if let [only] = subdirs.as_slice()
        && only.join(MANIFEST).is_file()
    {
        return Ok(only.clone());
    }
    bail!("no {MANIFEST} found: a pack has its manifest at the top")
}

fn unzip(file: &Path, into: &Path) -> Result<()> {
    let reader =
        std::fs::File::open(file).with_context(|| format!("opening {}", file.display()))?;
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| anyhow!("{}: {e}", file.display()))?;
    archive
        .extract(into)
        .map_err(|e| anyhow!("{}: {e}", file.display()))
}

/// Copy `from` to `to`, leaving out version control folders.
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).with_context(|| format!("creating {}", to.display()))?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let target = to.join(&name);
        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else {
            std::fs::copy(&path, &target).with_context(|| format!("copying {}", path.display()))?;
        }
    }
    Ok(())
}

/// A fresh, empty folder under the system temp folder.
pub(crate) fn tempdir(tag: &str) -> Result<PathBuf> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "mdeck-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_pack(dir: &Path, name: &str, min: Option<&str>) {
        std::fs::create_dir_all(dir.join("themes")).unwrap();
        std::fs::create_dir_all(dir.join("point-clouds")).unwrap();
        let min = min.map_or(String::new(), |m| format!("min-mdeck: \"{m}\"\n"));
        std::fs::write(
            dir.join(MANIFEST),
            format!("name: {name}\nversion: 1.0.0\ndescription: Test pack\n{min}"),
        )
        .unwrap();
        std::fs::write(dir.join("themes/acme.yaml"), "name: acme\n").unwrap();
    }

    #[test]
    fn versions_compare_numerically() {
        assert!(version_at_least("2.0.0", "2.0"));
        assert!(version_at_least("2.10.0", "2.9"));
        assert!(version_at_least("1.19.0", ">=1.2"));
        assert!(!version_at_least("1.19.0", "2.0"));
        assert!(!version_at_least("2.0.0", "2.0.1"));
        assert!(version_at_least("2.1.0-beta", "2.1"));
    }

    #[test]
    fn origins_are_recognised() {
        assert_eq!(
            Origin::parse("https://github.com/acme/pack"),
            Origin::Git("https://github.com/acme/pack".into())
        );
        assert_eq!(
            Origin::parse("git@github.com:acme/pack.git"),
            Origin::Git("git@github.com:acme/pack.git".into())
        );
        assert_eq!(
            Origin::parse("./no-such-pack"),
            Origin::Dir("./no-such-pack".into())
        );
    }

    #[test]
    fn install_list_and_remove_a_folder_pack() {
        let work = tempdir("pack-test").unwrap();
        let src = work.join("src");
        write_pack(&src, "acme-brand", Some("1.0"));
        let user = work.join("user");
        let deck = work.join("deck");

        let got = install(&Origin::Dir(src.clone()), &user, Scope::User).unwrap();
        assert_eq!(got.manifest.name, "acme-brand");
        assert!(user.join("acme-brand/themes/acme.yaml").is_file());
        assert_eq!(got.contents(), vec![(Folder::Themes, 1)]);

        write_pack(&src, "zeta", None);
        install(&Origin::Dir(src.clone()), &deck, Scope::Deck).unwrap();

        let all = installed_in(Some(&deck), Some(&user));
        let names: Vec<_> = all.iter().map(|p| p.manifest.name.as_str()).collect();
        assert_eq!(names, ["zeta", "acme-brand"], "deck packs come first");
        let themes = pack_dirs(&all, Folder::Themes);
        assert_eq!(
            themes,
            vec![deck.join("zeta/themes"), user.join("acme-brand/themes")]
        );
        assert_eq!(
            pack_dirs(&all, Folder::PointClouds).len(),
            2,
            "empty folders still count as folders"
        );
        assert!(pack_dirs(&all, Folder::Fonts).is_empty());

        // Reinstalling replaces.
        install(&Origin::Dir(src.clone()), &deck, Scope::Deck).unwrap();
        remove(&user, "acme-brand").unwrap();
        assert!(remove(&user, "acme-brand").is_err());
        assert_eq!(installed_in(None, Some(&user)).len(), 0);
        std::fs::remove_dir_all(&work).unwrap();
    }

    #[test]
    fn a_pack_for_a_newer_mdeck_is_refused() {
        let work = tempdir("pack-min").unwrap();
        write_pack(&work.join("src"), "future", Some("999.0"));
        let err = install(&Origin::Dir(work.join("src")), &work.join("u"), Scope::User)
            .unwrap_err()
            .to_string();
        assert!(err.contains("needs mdeck 999.0"), "{err}");
        std::fs::remove_dir_all(&work).unwrap();
    }

    #[test]
    fn a_zip_of_a_folder_installs() {
        use std::io::Write;
        let work = tempdir("pack-zip").unwrap();
        let file = work.join("pack.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&file).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("inner/mdeck-pack.yaml", opts).unwrap();
        zip.write_all(b"name: zipped\nversion: 0.1.0\n").unwrap();
        zip.start_file("inner/themes/z.yaml", opts).unwrap();
        zip.write_all(b"name: z\n").unwrap();
        zip.finish().unwrap();
        assert_eq!(
            Origin::parse(file.to_str().unwrap()),
            Origin::Zip(file.clone())
        );
        let got = install(&Origin::Zip(file), &work.join("u"), Scope::User).unwrap();
        assert_eq!(got.manifest.name, "zipped");
        assert!(work.join("u/zipped/themes/z.yaml").is_file());
        std::fs::remove_dir_all(&work).unwrap();
    }

    #[test]
    fn bad_manifests_are_explained() {
        let work = tempdir("pack-bad").unwrap();
        assert!(
            read_manifest(&work)
                .unwrap_err()
                .to_string()
                .contains("has no")
        );
        std::fs::write(work.join(MANIFEST), "name: Bad Name\nversion: 1\n").unwrap();
        assert!(read_manifest(&work).is_err());
        std::fs::write(work.join(MANIFEST), "name: ok\nversion: 1\nextra: x\n").unwrap();
        assert!(read_manifest(&work).is_err(), "unknown keys are refused");
        std::fs::remove_dir_all(&work).unwrap();
    }
}
