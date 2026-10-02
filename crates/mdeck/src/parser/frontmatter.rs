use super::{PresentationMeta, Setting};
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

    let mut meta = parse_frontmatter(yaml_str);
    // The `---` line is line 1, so YAML line k is file line k + 2.
    for s in &mut meta.settings {
        s.line += 2;
    }
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
        // A list (`requires: [a, b]`) reads as `a, b`.
        serde_norway::Value::Sequence(items) => Some(
            items
                .iter()
                .filter_map(value_to_string)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        _ => None,
    }
}

/// The frontmatter's top-level keys as written, each with its value and its
/// 0-based line in `yaml_str`. Values come from the YAML parser when the
/// block is YAML; otherwise each line is read as `key: value` (quotes
/// trimmed), so a stray colon in a title does not lose the rest of the block.
fn frontmatter_pairs(yaml_str: &str) -> Vec<Setting> {
    let parsed: Result<HashMap<String, serde_norway::Value>, _> = serde_norway::from_str(yaml_str);
    let map = parsed.ok();
    yaml_str
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with([' ', '\t', '#', '-']))
        .filter_map(|(at, line)| {
            let (key, raw) = line.split_once(':')?;
            let key = key.trim().trim_matches(['"', '\'']).to_string();
            if key.is_empty() {
                return None;
            }
            let value = match &map {
                Some(map) => map.get(&key).and_then(value_to_string)?,
                None => raw.trim().trim_matches('"').to_string(),
            };
            Some(Setting {
                name: key,
                value,
                line: at,
            })
        })
        .collect()
}

fn parse_frontmatter(yaml_str: &str) -> PresentationMeta {
    let mut meta = PresentationMeta::default();
    let settings = frontmatter_pairs(yaml_str);
    for setting in &settings {
        let value = setting.value.trim().to_string();
        let text = Some(value.clone()).filter(|v| !v.is_empty());
        match setting.name.as_str() {
            "title" => meta.title = text,
            "author" => meta.author = text,
            "theme" => meta.theme = text,
            "engine" => meta.engine = text,
            "transition" => meta.transition = text,
            "slide-level" => meta.slide_level = value.parse().ok().filter(|l| (1..=6).contains(l)),
            "countdown" => meta.countdown = switch(&value, "on", "off"),
            "reveal" => meta.reveal = switch(&value, "steps", "none"),
            "footer" => meta.footer = text,
            "logo" => meta.logo = text,
            "logo-position" => meta.logo_position = text,
            "logo-opacity" => meta.logo_opacity = text,
            "logo-height" => meta.logo_height = text,
            "background" => meta.background = text,
            "background-opacity" => meta.background_opacity = text,
            "palette" => meta.palette = text,
            "art-world" => meta.art_world = text,
            "image-style" => meta.image_style = text,
            "icon-style" => meta.icon_style = text,
            // Unknown and v1 (`@theme`) keys are kept in `settings` for
            // `--check` and otherwise ignored.
            _ => {}
        }
    }
    meta.settings = settings;
    meta
}

/// `Some(true)` for `on`, `Some(false)` for `off`, `None` for anything else
/// (which `--check` reports).
fn switch(value: &str, on: &str, off: &str) -> Option<bool> {
    let v = value.trim();
    if v == on {
        Some(true)
    } else if v == off {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_keys_are_read() {
        let (meta, _, _) = extract(
            "---\ntitle: 'Quoted' # a comment\ntheme: dark # house style\nfooter: 'Acme, 2026'\nengine: particles\nlogo: brand/logo.svg\nlogo-position: bottom-left\nlogo-opacity: 40%\nlogo-height: 72\nart-world: a harbour town\nslide-level: 3\n---\n# A\n",
        );
        assert_eq!(meta.title.as_deref(), Some("Quoted"));
        assert_eq!(meta.theme.as_deref(), Some("dark"));
        assert_eq!(meta.footer.as_deref(), Some("Acme, 2026"));
        assert_eq!(meta.engine.as_deref(), Some("particles"));
        assert_eq!(meta.logo.as_deref(), Some("brand/logo.svg"));
        assert_eq!(meta.logo_position.as_deref(), Some("bottom-left"));
        assert_eq!(meta.logo_opacity.as_deref(), Some("40%"));
        assert_eq!(meta.logo_height.as_deref(), Some("72"));
        assert_eq!(meta.art_world.as_deref(), Some("a harbour town"));
        assert_eq!(meta.slide_level, Some(3));
    }

    #[test]
    fn v1_keys_are_not_honoured_but_kept_for_check() {
        // D2: `@theme` is v1 syntax; it no longer sets the theme.
        let (meta, _, _) = extract("---\ntitle: T\n@theme: dark\n@thme: x\n---\n# A\n");
        assert_eq!(meta.title.as_deref(), Some("T"), "the rest still reads");
        assert_eq!(meta.theme, None);
        let keys: Vec<(&str, usize)> = meta
            .settings
            .iter()
            .map(|s| (s.name.as_str(), s.line))
            .collect();
        assert_eq!(keys, [("title", 2), ("@theme", 3), ("@thme", 4)]);
    }

    #[test]
    fn stray_colon_falls_back_to_lines() {
        let (meta, _, _) = extract("---\ntitle: Rust: the good parts\ntheme: light\n---\n# A\n");
        assert_eq!(meta.title.as_deref(), Some("Rust: the good parts"));
        assert_eq!(meta.theme.as_deref(), Some("light"));
    }

    #[test]
    fn switches_read_their_two_words() {
        let read = |fm: &str| extract(&format!("---\n{fm}\n---\n# A\n")).0;
        assert_eq!(read("countdown: off").countdown, Some(false));
        assert_eq!(read("countdown: on").countdown, Some(true));
        assert_eq!(read("countdown: maybe").countdown, None);
        assert_eq!(read("title: x").countdown, None);
        assert_eq!(read("reveal: none").reveal, Some(false));
        assert_eq!(read("reveal: steps").reveal, Some(true));
    }

    #[test]
    fn test_extract_frontmatter() {
        let content = "---\ntitle: \"Hello\"\nauthor: \"Test\"\ntheme: dark\n---\n\n# Slide";
        let (meta, body, _) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("Hello"));
        assert_eq!(meta.author.as_deref(), Some("Test"));
        assert_eq!(meta.theme.as_deref(), Some("dark"));
        assert!(body.contains("# Slide"));
    }

    #[test]
    fn body_line_counts_the_frontmatter() {
        assert_eq!(extract("# A\n").2, 1);
        assert_eq!(extract("---\ntitle: x\ntheme: dark\n---\n\n# A").2, 5);
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
    fn test_frontmatter_styles() {
        let content = "---\ntitle: \"Test\"\nimage-style: Pixar\nicon-style: minimal\n---\nBody";
        let (meta, body, _) = extract(content);
        assert_eq!(meta.image_style.as_deref(), Some("Pixar"));
        assert_eq!(meta.icon_style.as_deref(), Some("minimal"));
        assert_eq!(body.trim(), "Body");
    }

    #[test]
    fn test_frontmatter_numeric_scalars() {
        // `title: 2026` is a YAML number; it must show as "2026".
        let content = "---\ntitle: 2026\nauthor: 3.5\nfooter: true\n---\nBody";
        let (meta, _body, _) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("2026"));
        assert_eq!(meta.author.as_deref(), Some("3.5"));
        assert_eq!(meta.footer.as_deref(), Some("true"));
    }

    #[test]
    fn test_frontmatter_crlf() {
        let content = "---\r\ntitle: \"Hello\"\r\nauthor: \"Me\"\r\ntheme: nord\r\n---\r\n\r\n# Slide\r\n\r\nText\r\n";
        let (meta, body, _) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("Hello"));
        assert_eq!(meta.theme.as_deref(), Some("nord"));
        assert_eq!(body.trim(), "# Slide\n\nText");
        assert!(!body.contains('\r'));
    }

    #[test]
    fn test_frontmatter_bom() {
        let (meta, body, _) = extract("\u{feff}---\r\ntitle: \"BOM\"\r\n---\r\n# Slide\r\n");
        assert_eq!(meta.title.as_deref(), Some("BOM"));
        assert_eq!(body.trim(), "# Slide");
        let (meta, body, _) = extract("\u{feff}# Slide\r\nText");
        assert!(meta.title.is_none());
        assert_eq!(body, "# Slide\nText");
    }

    #[test]
    fn test_frontmatter_unclosed_and_empty_body() {
        let content = "---\ntitle: x\n# Slide";
        let (meta, body, _) = extract(content);
        assert!(meta.title.is_none());
        assert_eq!(body, content);
        let (meta, body, _) = extract("---\ntitle: x\n---");
        assert_eq!(meta.title.as_deref(), Some("x"));
        assert_eq!(body, "");
    }
}
