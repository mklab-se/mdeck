//! Paths a theme names (fonts, syntax files, logos, art references) are
//! relative to its folder and may not leave it.

use std::path::{Path, PathBuf};

use super::ThemeError;

/// Resolve a theme-relative path, refusing anything that leaves `base`.
pub fn confined_path(base: &Path, rel: &str) -> Result<PathBuf, ThemeError> {
    let p = Path::new(rel);
    if p.is_absolute() {
        return Err(ThemeError::AbsolutePath { rel: rel.into() });
    }
    let joined = base.join(p);
    let real = joined
        .canonicalize()
        .map_err(|_| ThemeError::PathNotFound {
            rel: rel.into(),
            base: base.to_path_buf(),
        })?;
    let root = base
        .canonicalize()
        .map_err(|e| ThemeError::Io(e.to_string()))?;
    if !real.starts_with(&root) {
        return Err(ThemeError::PathEscapes { rel: rel.into() });
    }
    Ok(real)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_cannot_leave_the_theme_folder() {
        let dir = std::env::temp_dir().join(format!("mdeck-confine-{}", std::process::id()));
        let inner = dir.join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(dir.join("secret.ttf"), b"x").unwrap();
        std::fs::write(inner.join("ok.ttf"), b"x").unwrap();
        assert!(confined_path(&inner, "ok.ttf").is_ok());
        let err = |rel: &str| confined_path(&inner, rel).unwrap_err().to_string();
        assert!(err("../secret.ttf").contains("outside"));
        assert!(err("/etc/hosts").contains("inside"));
        assert!(err("missing.ttf").contains("not found"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
