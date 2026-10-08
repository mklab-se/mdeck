//! Where the tool keeps its files.
//!
//! Linux and macOS share one XDG layout, the same as Ailloy and every MKLab
//! tool: config in `$XDG_CONFIG_HOME/<tool>` (default `~/.config/<tool>`) and
//! cache in `$XDG_CACHE_HOME/<tool>` (default `~/.cache/<tool>`). Windows uses
//! its native folders (`%APPDATA%`, `%LOCALAPPDATA%`). Deliberately not
//! `dirs::config_dir()` on macOS, which is `~/Library/Application Support`.

use std::path::PathBuf;

/// Folder name under the config and cache roots.
pub const TOOL: &str = "mdeck";

/// The tool's config directory (holds `config.yaml` and the `themes`,
/// `designs`, `packs`, `illustrations` and `logs` folders).
#[cfg(not(windows))]
pub fn config_dir() -> Option<PathBuf> {
    xdg_dir(
        std::env::var_os("XDG_CONFIG_HOME"),
        dirs::home_dir(),
        ".config",
    )
}

/// The tool's config directory (holds `config.yaml`): `%APPDATA%\<tool>`.
#[cfg(windows)]
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(TOOL))
}

/// The tool's cache directory (custom builds, other disposable state).
#[cfg(not(windows))]
pub fn cache_dir() -> Option<PathBuf> {
    xdg_dir(
        std::env::var_os("XDG_CACHE_HOME"),
        dirs::home_dir(),
        ".cache",
    )
}

/// The tool's cache directory: `%LOCALAPPDATA%\<tool>`.
#[cfg(windows)]
pub fn cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join(TOOL))
}

/// `$XDG_*_HOME/<tool>` when that variable is an absolute path, else
/// `~/<fallback>/<tool>`.
#[cfg(not(windows))]
fn xdg_dir(
    xdg: Option<std::ffi::OsString>,
    home: Option<PathBuf>,
    fallback: &str,
) -> Option<PathBuf> {
    match xdg.map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => Some(dir.join(TOOL)),
        _ => home.map(|h| h.join(fallback).join(TOOL)),
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    #[test]
    fn an_absolute_xdg_variable_wins() {
        let dir = xdg_dir(Some("/xdg".into()), Some("/home/k".into()), ".config");
        assert_eq!(dir, Some(PathBuf::from("/xdg").join(TOOL)));
    }

    #[test]
    fn otherwise_the_dot_folder_in_home() {
        let config = xdg_dir(None, Some("/Users/k".into()), ".config");
        assert_eq!(config, Some(PathBuf::from("/Users/k/.config").join(TOOL)));
        let cache = xdg_dir(None, Some("/Users/k".into()), ".cache");
        assert_eq!(cache, Some(PathBuf::from("/Users/k/.cache").join(TOOL)));
    }

    #[test]
    fn empty_or_relative_xdg_variables_are_ignored() {
        // The XDG Base Directory spec says to ignore relative paths.
        for xdg in ["", "relative/dir"] {
            let dir = xdg_dir(Some(xdg.into()), Some("/home/k".into()), ".config");
            assert_eq!(
                dir,
                Some(PathBuf::from("/home/k/.config").join(TOOL)),
                "{xdg:?}"
            );
        }
    }

    #[test]
    fn no_home_and_no_xdg_is_none() {
        assert_eq!(xdg_dir(None, None, ".config"), None);
    }

    #[test]
    fn every_user_folder_lives_under_the_config_dir() {
        let config = config_dir().unwrap();
        assert_eq!(
            crate::config::Config::path().unwrap(),
            config.join("config.yaml")
        );
        assert_eq!(
            crate::theme::lookup::user_dir(),
            Some(config.join("themes"))
        );
        assert_eq!(
            crate::theme::lookup::user_designs_dir(),
            Some(config.join("designs"))
        );
        assert_eq!(
            crate::extensions::packs::user_root(),
            Some(config.join("packs"))
        );
        assert_eq!(
            crate::render::point_cloud::user_dir(),
            Some(config.join(crate::render::point_cloud::FOLDER))
        );
        assert_eq!(crate::incident_log::log_dir(), config.join("logs"));
    }

    #[test]
    fn custom_builds_live_under_the_cache_dir() {
        assert_eq!(
            crate::commands::build::build_root().unwrap(),
            cache_dir().unwrap().join("build")
        );
    }

    #[test]
    fn defaults_are_xdg_on_macos_too() {
        // With no XDG variables set, the folders are the dot folders in home.
        if std::env::var_os("XDG_CONFIG_HOME").is_none() {
            let home = dirs::home_dir().unwrap();
            assert_eq!(config_dir(), Some(home.join(".config").join(TOOL)));
        }
        if std::env::var_os("XDG_CACHE_HOME").is_none() {
            let home = dirs::home_dir().unwrap();
            assert_eq!(cache_dir(), Some(home.join(".cache").join(TOOL)));
        }
    }
}
