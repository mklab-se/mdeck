//! What can stop a theme from loading. The texts are what users (and
//! `mdeck theme check`) see, so each variant spells out the key it is about.

use std::fmt;
use std::path::PathBuf;

/// Why a theme could not be read, merged or built.
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeError {
    /// The YAML does not match the theme file format.
    Parse(String),
    /// A file (a theme or a font) could not be read or parsed.
    File { path: String, message: String },
    /// A required key is unset in the theme and everything it extends.
    Missing { key: String },
    /// A key holds a value that does not mean anything for it.
    Invalid { key: String, reason: String },
    /// A number outside the range its key allows.
    OutOfRange {
        key: String,
        value: f32,
        lo: f32,
        hi: f32,
    },
    /// `colors.series` is an empty list.
    EmptySeries,
    /// An `extends` chain deeper than lookup follows (usually a cycle).
    Cycle { name: String },
    /// `extends` names a theme nothing defines.
    NoParent { name: String, parent: String },
    /// No theme has this name.
    Unknown {
        name: String,
        available: Vec<String>,
    },
    /// A theme named by path whose file does not exist.
    NotFound(PathBuf),
    /// No built-in theme has this name.
    NoBuiltin(String),
    /// A theme-relative path that is absolute.
    AbsolutePath { rel: String },
    /// A theme-relative path that does not exist.
    PathNotFound { rel: String, base: PathBuf },
    /// A theme-relative path that resolves outside the theme folder.
    PathEscapes { rel: String },
    /// The theme folder itself cannot be resolved.
    Io(String),
}

impl ThemeError {
    /// A value that is not valid for `key`, explained by `reason`.
    pub fn invalid(key: impl Into<String>, reason: impl Into<String>) -> Self {
        ThemeError::Invalid {
            key: key.into(),
            reason: reason.into(),
        }
    }

    /// `key` is unset everywhere along the `extends` chain.
    pub fn missing(key: impl Into<String>) -> Self {
        ThemeError::Missing { key: key.into() }
    }

    /// A problem reading or parsing the file at `path`.
    pub fn file(path: impl fmt::Display, message: impl fmt::Display) -> Self {
        ThemeError::File {
            path: path.to_string(),
            message: message.to_string(),
        }
    }
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThemeError::Parse(e) | ThemeError::Io(e) => f.write_str(e),
            ThemeError::File { path, message } => write!(f, "{path}: {message}"),
            ThemeError::Missing { key } => {
                write!(f, "{key} is not set (and nothing it extends sets it)")
            }
            ThemeError::Invalid { key, reason } => write!(f, "{key}: {reason}"),
            ThemeError::OutOfRange { key, value, lo, hi } => {
                write!(f, "{key}: {value} must be between {lo} and {hi}")
            }
            ThemeError::EmptySeries => f.write_str("colors.series needs at least one colour"),
            ThemeError::Cycle { name } => {
                write!(f, "theme '{name}' extends too deeply (is there a cycle?)")
            }
            ThemeError::NoParent { name, parent } => {
                write!(f, "theme '{name}' extends '{parent}', which does not exist")
            }
            ThemeError::Unknown { name, available } => write!(
                f,
                "unknown theme '{name}' (available: {})",
                available.join(", ")
            ),
            ThemeError::NotFound(path) => {
                write!(f, "theme file {} was not found", path.display())
            }
            ThemeError::NoBuiltin(name) => write!(f, "no built-in theme '{name}'"),
            ThemeError::AbsolutePath { rel } => {
                write!(f, "'{rel}' must be a path inside the theme folder")
            }
            ThemeError::PathNotFound { rel, base } => {
                write!(f, "'{rel}' was not found in {}", base.display())
            }
            ThemeError::PathEscapes { rel } => {
                write!(f, "'{rel}' points outside the theme folder")
            }
        }
    }
}

impl std::error::Error for ThemeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_key() {
        assert_eq!(
            ThemeError::missing("colors.accent").to_string(),
            "colors.accent is not set (and nothing it extends sets it)"
        );
        assert_eq!(
            ThemeError::invalid("countdown", "'loud' is not none, plain or burst").to_string(),
            "countdown: 'loud' is not none, plain or burst"
        );
        let range = ThemeError::OutOfRange {
            key: "text.line-height".into(),
            value: 9.0,
            lo: 0.5,
            hi: 4.0,
        };
        assert_eq!(
            range.to_string(),
            "text.line-height: 9 must be between 0.5 and 4"
        );
        assert_eq!(ThemeError::file("a.yaml", "bad").to_string(), "a.yaml: bad");
    }

    #[test]
    fn lookup_messages() {
        let unknown = ThemeError::Unknown {
            name: "x".into(),
            available: vec!["dark".into(), "light".into()],
        };
        assert_eq!(
            unknown.to_string(),
            "unknown theme 'x' (available: dark, light)"
        );
        assert!(
            ThemeError::Cycle { name: "c".into() }
                .to_string()
                .contains("cycle")
        );
        assert_eq!(
            ThemeError::NoParent {
                name: "e".into(),
                parent: "gone".into()
            }
            .to_string(),
            "theme 'e' extends 'gone', which does not exist"
        );
    }
}
