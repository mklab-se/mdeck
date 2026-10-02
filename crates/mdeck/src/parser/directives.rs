//! `@name: value` directives: the known names, and taking them out of a slide.

use super::Directive;

/// Directives that apply to the slide they are written in. These are honoured
/// anywhere at the top level of a slide, not only at its start.
pub const SLIDE_DIRECTIVES: &[&str] = &[
    "layout",
    "illustration",
    "logo",
    "art",
    "background",
    "background-opacity",
    "thermal-window",
    "zoom",
    "class",
];

/// Directives that only mean something in the frontmatter. Written inside a
/// slide they are removed from its content and ignored (`--check` says so).
pub const GLOBAL_DIRECTIVES: &[&str] = &[
    "theme",
    "engine",
    "transition",
    "aspect",
    "code-theme",
    "footer",
    "image-style",
    "icon-style",
    "slide-level",
    "countdown",
    "logo-position",
    "logo-opacity",
    "logo-height",
    "palette",
];

/// A directive name mdeck knows, at slide or deck scope.
pub fn is_known_directive(name: &str) -> bool {
    SLIDE_DIRECTIVES.contains(&name) || GLOBAL_DIRECTIVES.contains(&name)
}

/// The value of the slide directive `name`; when it is written twice, the last wins.
pub fn directive<'a>(directives: &'a [Directive], name: &str) -> Option<&'a str> {
    directives
        .iter()
        .rev()
        .find(|d| d.name == name)
        .map(|d| d.value.as_str())
}

/// Extract a slide's `@name: value` directives. Returns (directives, remaining content).
/// Each directive's `line` is its 0-based line in `raw`.
///
/// Any directive lines at the start of the slide are taken, known or not. After
/// that, a line holding only a known directive ([`is_known_directive`])
/// is taken wherever it stands at the top level of the slide (column 0, outside
/// code fences), so `@illustration: x` written under the heading works. Its line
/// becomes blank, keeping the blocks around it apart. Unknown names past the
/// start stay text, so prose such as `@team: see you at five` is never swallowed.
pub fn extract_directives(raw: &str) -> (Vec<Directive>, String) {
    let mut directives = Vec::new();
    let mut remaining_lines = Vec::new();
    let mut past_directives = false;
    let mut fences = super::splitter::FenceTracker::new();

    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if !past_directives {
            if trimmed.is_empty() {
                continue;
            }
            // Single-line HTML comments may precede directives
            if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
                continue;
            }
            if let Some(directive) = parse_directive_line(trimmed) {
                directives.push(Directive {
                    line: index,
                    ..directive
                });
                continue;
            }
            past_directives = true;
        }
        let in_fence = fences.observe(line);
        if !in_fence
            && line.starts_with('@')
            && let Some(directive) = parse_directive_line(trimmed)
            && is_known_directive(&directive.name)
        {
            directives.push(Directive {
                line: index,
                ..directive
            });
            remaining_lines.push("");
            continue;
        }
        remaining_lines.push(line);
    }

    (directives, remaining_lines.join("\n"))
}

pub(crate) fn parse_directive_line(line: &str) -> Option<Directive> {
    if !line.starts_with('@') {
        return None;
    }
    let after_at = &line[1..];
    let colon_pos = after_at.find(':')?;
    let name = after_at[..colon_pos].trim().to_string();
    let value = after_at[colon_pos + 1..].trim().to_string();

    // Validate: name should be word characters and hyphens
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }

    Some(Directive {
        name,
        value,
        line: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_directives() {
        let raw = "@layout: two-column\n@theme: dark\n\n# Title\n\nContent";
        let (dirs, content) = extract_directives(raw);
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].name, "layout");
        assert_eq!(dirs[0].value, "two-column");
        assert_eq!(dirs[1].name, "theme");
        assert_eq!(dirs[1].value, "dark");
        assert!(content.contains("# Title"));
    }

    #[test]
    fn comments_before_directives_keep_the_prelude_open() {
        let (dirs, content) = extract_directives("<!-- c -->\n@layout: quote\n> q");
        assert_eq!(dirs.len(), 1);
        assert!(content.contains("> q"));
    }

    #[test]
    fn directive_lines_need_a_word_name() {
        let d = parse_directive_line("@logo-height: 72 ").unwrap();
        assert_eq!((d.name.as_str(), d.value.as_str()), ("logo-height", "72"));
        for line in ["layout: x", "@: x", "@two words: x", "@layout"] {
            assert!(parse_directive_line(line).is_none(), "{line:?}");
        }
    }

    #[test]
    fn last_directive_wins_and_names_are_known() {
        let (dirs, _) = extract_directives("@layout: code\n@layout: quote\n# A");
        assert_eq!(directive(&dirs, "layout"), Some("quote"));
        assert_eq!(directive(&dirs, "logo"), None);
        assert!(is_known_directive("illustration"));
        assert!(is_known_directive("slide-level"));
        assert!(!is_known_directive("team"));
    }
}
