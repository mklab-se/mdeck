//! Checks on a resolved theme that are advice, not errors: text that is
//! hard to read on its background.

use super::{Theme, contrast};

/// Minimum contrast ratios (WCAG 2: 4.5 for body text, 3 for large text).
const BODY_TEXT: f32 = 4.5;
const LARGE_TEXT: f32 = 3.0;

/// Readability problems in `theme`, one line each.
pub fn review(theme: &Theme) -> Vec<String> {
    let mut out = Vec::new();
    let mut pair = |what: &str, fg, bg, min: f32| {
        let r = contrast(fg, bg);
        if r < min {
            out.push(format!(
                "{what} has a contrast of {r:.1}:1 (at least {min}:1 reads comfortably)"
            ));
        }
    };
    pair(
        "colors.text on colors.background",
        theme.foreground,
        theme.background,
        BODY_TEXT,
    );
    pair(
        "colors.heading on colors.background",
        theme.heading_color,
        theme.background,
        LARGE_TEXT,
    );
    pair(
        "colors.accent on colors.background",
        theme.accent,
        theme.background,
        LARGE_TEXT,
    );
    pair(
        "colors.code-text on colors.code-background",
        theme.code_foreground,
        theme.code_background,
        BODY_TEXT,
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::file::ThemeFile;
    use crate::theme::lookup::{BUILTIN, builtin_file, load_builtin};

    #[test]
    fn builtins_read_comfortably() {
        for (name, _) in BUILTIN.iter().copied() {
            let t = load_builtin(name).unwrap();
            assert!(review(&t).is_empty(), "{name}: {:?}", review(&t));
        }
    }

    #[test]
    fn grey_on_grey_is_flagged() {
        let f = ThemeFile::parse("colors: { background: '#777777', text: '#888888' }")
            .unwrap()
            .over(&builtin_file("dark").unwrap());
        let t = Theme::build("x", &f).unwrap().theme;
        let r = review(&t);
        assert!(r.iter().any(|l| l.starts_with("colors.text")), "{r:?}");
    }
}
