//! What the theme says about its engine beyond the name. Two seams that the
//! v2 theme schema (phase 3) replaces: the engine's settings and whether
//! copy slides use the editorial arrangement.

use mdeck_sdk::tokens::{EngineSettings, Value};

use super::Theme;
use super::file::ThemeFile;

/// The engine's settings (THM-11, ENG-11). Interim: built from the theme
/// keys that configure engines today, each only when the theme file sets
/// it: `surface:` (the line engine's `surface`) and `heat:` (the thermal
/// engine's `palette` and `drift`). Phase 3 reads the theme's `engine:`
/// block here instead.
pub fn engine_settings(theme: &Theme) -> EngineSettings {
    EngineSettings::from_pairs(theme.engine_keys.clone())
}

/// The engine keys a theme file sets, for [`engine_settings`].
pub(super) fn engine_keys(f: &ThemeFile) -> Vec<(String, Value)> {
    let mut keys = Vec::new();
    if let Some(s) = &f.surface {
        keys.push(("surface".to_string(), Value::String(s.trim().to_string())));
    }
    if let Some(p) = &f.heat.palette {
        keys.push(("palette".to_string(), Value::String(p.trim().to_string())));
    }
    if let Some(d) = f.heat.drift {
        keys.push(("drift".to_string(), Value::Bool(d)));
    }
    keys
}

/// Whether copy slides use the editorial arrangement (eyebrow, copy
/// column, stage, counter chrome). Interim: every engine but plain and the
/// boards; phase 3 makes it the theme's design set.
pub fn uses_editorial(theme: &Theme) -> bool {
    theme.engine.paints() && !theme.engine.is_board()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_keys_the_file_sets_become_settings() {
        let f = ThemeFile::parse("surface: slate\nheat: { drift: true }\n").unwrap();
        let keys = engine_keys(&f);
        assert_eq!(
            keys,
            vec![
                ("surface".to_string(), Value::String("slate".into())),
                ("drift".to_string(), Value::Bool(true)),
            ]
        );
        assert!(engine_keys(&ThemeFile::parse("{}").unwrap()).is_empty());
    }

    #[test]
    fn plain_themes_are_not_editorial() {
        assert!(!uses_editorial(&Theme::dark()));
    }
}
