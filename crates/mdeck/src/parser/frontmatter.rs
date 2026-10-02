use super::PresentationMeta;
use std::collections::HashMap;

/// Split off the frontmatter. Returns the metadata, the body, and the 1-based
/// line of the file the body starts on (1 without frontmatter).
pub fn extract(content: &str) -> (PresentationMeta, String, usize) {
    // Normalise line endings up front so byte offsets below are exact for
    // CRLF files, then strip a leading BOM.
    let normalized = content.replace("\r\n", "\n");
    let trimmed = normalized.trim_start_matches('\u{feff}');

    let Some(after_opening) = trimmed.strip_prefix("---\n") else {
        return (PresentationMeta::default(), trimmed.to_string(), 1);
    };

    // Find closing ---
    let Some((yaml_end, body_start)) = find_closing_delimiter(after_opening) else {
        return (PresentationMeta::default(), trimmed.to_string(), 1);
    };

    let yaml_str = &after_opening[..yaml_end];
    let body = after_opening.get(body_start..).unwrap_or("");
    // The opening `---` line, then every line up to and including the closing one.
    let line = 2 + after_opening[..body_start].matches('\n').count();

    let meta = parse_frontmatter(yaml_str);
    (meta, body.to_string(), line)
}

/// Locate the closing `---` line. Returns `(yaml_end, body_start)` byte
/// offsets into `s`: the YAML text is `s[..yaml_end]` and the document body
/// starts at `body_start` (just after the delimiter line).
fn find_closing_delimiter(s: &str) -> Option<(usize, usize)> {
    let mut offset = 0;
    for line in s.split_inclusive('\n') {
        if line.trim() == "---" {
            return Some((offset, offset + line.len()));
        }
        offset += line.len();
    }
    None
}

/// Render a YAML scalar as the string a user would expect to see:
/// `title: 2026` → "2026", `draft: true` → "true". Non-scalars are ignored.
fn value_to_string(value: &serde_norway::Value) -> Option<String> {
    match value {
        serde_norway::Value::String(s) => Some(s.clone()),
        serde_norway::Value::Number(n) => Some(n.to_string()),
        serde_norway::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Deck keys are written `@theme: dark`, and `@` cannot start a plain YAML
/// key, so quote those keys (`"@theme": dark`) before handing the block to
/// the YAML parser.
fn quote_directive_keys(yaml_str: &str) -> String {
    let mut out = String::with_capacity(yaml_str.len() + 16);
    for line in yaml_str.lines() {
        match line.split_once(':') {
            Some((key, rest))
                if key.starts_with('@')
                    && key[1..]
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
            {
                out.push('"');
                out.push_str(key);
                out.push_str("\":");
                out.push_str(rest);
            }
            _ => out.push_str(line),
        }
        out.push('\n');
    }
    out
}

/// The frontmatter as `key → value` strings: parsed as YAML when it is YAML,
/// otherwise read line by line (`key: value`, quotes trimmed), so a stray
/// colon in a title does not lose the rest of the block.
fn frontmatter_pairs(yaml_str: &str) -> Vec<(String, String)> {
    let parsed: Result<HashMap<String, serde_norway::Value>, _> =
        serde_norway::from_str(&quote_directive_keys(yaml_str));
    match parsed {
        Ok(map) => map
            .into_iter()
            .filter_map(|(k, v)| value_to_string(&v).map(|v| (k, v)))
            .collect(),
        Err(_) => yaml_str
            .lines()
            .filter_map(|line| line.trim().split_once(':'))
            .map(|(k, v)| (k.trim().to_string(), v.trim().trim_matches('"').to_string()))
            .collect(),
    }
}

fn parse_frontmatter(yaml_str: &str) -> PresentationMeta {
    let mut meta = PresentationMeta::default();
    for (key, value) in frontmatter_pairs(yaml_str) {
        let text = Some(value.clone());
        match key.as_str() {
            "title" => meta.title = text,
            "author" => meta.author = text,
            "date" => meta.date = text,
            "@theme" => meta.theme = text,
            "@transition" => meta.transition = text,
            "@aspect" => meta.aspect = text,
            "@code-theme" => meta.code_theme = text,
            "@footer" => meta.footer = text,
            "@image-style" => meta.image_style = text,
            "@icon-style" => meta.icon_style = text,
            "@slide-level" => meta.slide_level = value.trim().parse().ok(),
            "@story" => meta.story = text,
            "@countdown" => meta.countdown = Some(parse_switch(&value)),
            "@engine" => meta.engine = text,
            "@logo" => meta.logo = text,
            "@logo-position" => meta.logo_position = text,
            "@logo-opacity" => meta.logo_opacity = text,
            "@logo-height" => meta.logo_height = text,
            "@background" => meta.background = text,
            "@background-opacity" => meta.background_opacity = text,
            "@art" => meta.art = text,
            _ => {}
        }
    }
    meta
}

/// `true`/`on`/`yes`/`1` → true; anything else → false.
fn parse_switch(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "on" | "yes" | "1"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_keys_are_read() {
        let (meta, _, _) = extract(
            "---\n@logo: brand/logo.svg\n@logo-position: bottom-left\n@logo-opacity: 40%\n@logo-height: 72\n---\n# A\n",
        );
        assert_eq!(meta.logo.as_deref(), Some("brand/logo.svg"));
        assert_eq!(meta.logo_position.as_deref(), Some("bottom-left"));
        assert_eq!(meta.logo_opacity.as_deref(), Some("40%"));
        assert_eq!(meta.logo_height.as_deref(), Some("72"));
    }

    #[test]
    fn engine_key_is_read() {
        let (meta, _, _) = extract("---\n@theme: dark\n@engine: particles\n---\n# A\n");
        assert_eq!(meta.engine.as_deref(), Some("particles"));
        let (meta, _, _) = extract("---\n@engine: particles\n: broken yaml [\n---\n# A\n");
        assert_eq!(meta.engine.as_deref(), Some("particles"), "manual fallback");
    }

    #[test]
    fn directive_keys_go_through_yaml() {
        // `@` keys used to make the whole block invalid YAML, so every deck
        // fell back to line splitting: comments and single quotes leaked in.
        let (meta, _, _) = extract(
            "---\ntitle: 'Quoted' # a comment\n@theme: dark # house style\n@footer: 'Acme, 2026'\n---\n# A\n",
        );
        assert_eq!(meta.title.as_deref(), Some("Quoted"));
        assert_eq!(meta.theme.as_deref(), Some("dark"));
        assert_eq!(meta.footer.as_deref(), Some("Acme, 2026"));
    }

    #[test]
    fn stray_colon_falls_back_to_lines() {
        let (meta, _, _) = extract("---\ntitle: Rust: the good parts\n@theme: light\n---\n# A\n");
        assert_eq!(meta.title.as_deref(), Some("Rust: the good parts"));
        assert_eq!(meta.theme.as_deref(), Some("light"));
    }

    #[test]
    fn countdown_switch_parses_common_spellings() {
        let (meta, _, _) = extract("---\n@countdown: false\n---\n# A\n");
        assert_eq!(meta.countdown, Some(false));
        let (meta, _, _) = extract("---\n@countdown: off\n---\n# A\n");
        assert_eq!(meta.countdown, Some(false));
        let (meta, _, _) = extract("---\n@countdown: true\n---\n# A\n");
        assert_eq!(meta.countdown, Some(true));
        let (meta, _, _) = extract("---\ntitle: x\n---\n# A\n");
        assert_eq!(meta.countdown, None);
    }

    #[test]
    fn test_extract_frontmatter() {
        let content = "---\ntitle: \"Hello\"\nauthor: \"Test\"\n@theme: dark\n---\n\n# Slide";
        let (meta, body, _) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("Hello"));
        assert_eq!(meta.author.as_deref(), Some("Test"));
        assert_eq!(meta.theme.as_deref(), Some("dark"));
        assert!(body.contains("# Slide"));
    }

    #[test]
    fn body_line_counts_the_frontmatter() {
        assert_eq!(extract("# A\n").2, 1);
        assert_eq!(extract("---\ntitle: x\n@theme: dark\n---\n\n# A").2, 5);
        assert_eq!(extract("\u{feff}---\r\ntitle: x\r\n---\r\n# A").2, 4);
        // No closing delimiter: no frontmatter, the body is the whole file.
        assert_eq!(extract("---\ntitle: x\n# A").2, 1);
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "# Just a slide\n\nSome content";
        let (meta, body, _) = extract(content);
        assert!(meta.title.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn test_frontmatter_with_all_fields() {
        let content = "---\ntitle: \"Test\"\nauthor: \"Author\"\ndate: 2026-02-28\n@theme: light\n@transition: fade\n@aspect: 16:9\n@footer: \"footer text\"\n---\nBody";
        let (meta, body, _) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("Test"));
        assert_eq!(meta.theme.as_deref(), Some("light"));
        assert_eq!(meta.transition.as_deref(), Some("fade"));
        assert_eq!(meta.aspect.as_deref(), Some("16:9"));
        assert_eq!(meta.footer.as_deref(), Some("footer text"));
        assert_eq!(body.trim(), "Body");
    }

    #[test]
    fn test_frontmatter_image_style() {
        let content = "---\ntitle: \"Test\"\n@image-style: Pixar\n@icon-style: minimal\n---\nBody";
        let (meta, body, _) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("Test"));
        assert_eq!(meta.image_style.as_deref(), Some("Pixar"));
        assert_eq!(meta.icon_style.as_deref(), Some("minimal"));
        assert_eq!(body.trim(), "Body");
    }

    #[test]
    fn test_frontmatter_date_not_string() {
        let content = "---\ntitle: \"Test\"\ndate: 2026-02-28\n---\nBody";
        let (meta, _body, _) = extract(content);
        assert_eq!(meta.date.as_deref(), Some("2026-02-28"));
    }

    #[test]
    fn test_frontmatter_numeric_scalars() {
        // `date: 2026` is a YAML number; it must show as "2026", not
        // `Number(2026)`, and a numeric title must not be dropped.
        let content = "---\ntitle: 2026\ndate: 2026\nauthor: 3.5\n@footer: true\n---\nBody";
        let (meta, _body, _) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("2026"));
        assert_eq!(meta.date.as_deref(), Some("2026"));
        assert_eq!(meta.author.as_deref(), Some("3.5"));
        assert_eq!(meta.footer.as_deref(), Some("true"));
    }

    #[test]
    fn test_frontmatter_crlf() {
        let content = "---\r\ntitle: \"Hello\"\r\nauthor: \"Me\"\r\n@theme: nord\r\n---\r\n\r\n# Slide\r\n\r\nText\r\n";
        let (meta, body, _) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("Hello"));
        assert_eq!(meta.author.as_deref(), Some("Me"));
        assert_eq!(meta.theme.as_deref(), Some("nord"));
        assert_eq!(body.trim(), "# Slide\n\nText");
        assert!(
            !body.contains("---"),
            "delimiter leaked into body: {body:?}"
        );
        assert!(!body.contains('\r'));
    }

    #[test]
    fn test_frontmatter_bom_crlf() {
        let content = "\u{feff}---\r\ntitle: \"BOM\"\r\n---\r\n# Slide\r\n";
        let (meta, body, _) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("BOM"));
        assert_eq!(body.trim(), "# Slide");
    }

    #[test]
    fn test_frontmatter_bom_no_frontmatter() {
        let content = "\u{feff}# Slide\r\nText";
        let (meta, body, _) = extract(content);
        assert!(meta.title.is_none());
        assert_eq!(body, "# Slide\nText");
    }

    #[test]
    fn test_frontmatter_unclosed() {
        let content = "---\ntitle: x\n# Slide";
        let (meta, body, _) = extract(content);
        assert!(meta.title.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn test_frontmatter_closing_at_end_of_file() {
        let content = "---\ntitle: x\n---";
        let (meta, body, _) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("x"));
        assert_eq!(body, "");
    }
}
