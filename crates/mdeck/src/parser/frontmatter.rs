use super::PresentationMeta;
use std::collections::HashMap;

pub fn extract(content: &str) -> (PresentationMeta, String) {
    // Normalise line endings up front so byte offsets below are exact for
    // CRLF files, then strip a leading BOM.
    let normalized = content.replace("\r\n", "\n");
    let trimmed = normalized.trim_start_matches('\u{feff}');

    let Some(after_opening) = trimmed.strip_prefix("---\n") else {
        return (PresentationMeta::default(), trimmed.to_string());
    };

    // Find closing ---
    let Some((yaml_end, body_start)) = find_closing_delimiter(after_opening) else {
        return (PresentationMeta::default(), trimmed.to_string());
    };

    let yaml_str = &after_opening[..yaml_end];
    let body = after_opening.get(body_start..).unwrap_or("");

    let meta = parse_frontmatter(yaml_str);
    (meta, body.to_string())
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
fn value_to_string(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn parse_frontmatter(yaml_str: &str) -> PresentationMeta {
    // Try to parse as YAML HashMap
    let map: HashMap<String, serde_yaml::Value> = match serde_yaml::from_str(yaml_str) {
        Ok(m) => m,
        Err(_) => return parse_frontmatter_manual(yaml_str),
    };

    PresentationMeta {
        title: get_string(&map, "title"),
        author: get_string(&map, "author"),
        date: get_string(&map, "date"),
        theme: get_string(&map, "@theme"),
        transition: get_string(&map, "@transition"),
        aspect: get_string(&map, "@aspect"),
        code_theme: get_string(&map, "@code-theme"),
        footer: get_string(&map, "@footer"),
        image_style: get_string(&map, "@image-style"),
        icon_style: get_string(&map, "@icon-style"),
        slide_level: get_u8(&map, "@slide-level"),
    }
}

fn get_string(map: &HashMap<String, serde_yaml::Value>, key: &str) -> Option<String> {
    map.get(key).and_then(value_to_string)
}

fn get_u8(map: &HashMap<String, serde_yaml::Value>, key: &str) -> Option<u8> {
    map.get(key).and_then(|v| match v {
        serde_yaml::Value::Number(n) => n.as_u64().and_then(|n| u8::try_from(n).ok()),
        serde_yaml::Value::String(s) => s.trim().parse().ok(),
        _ => None,
    })
}

/// Fallback: parse key: value lines manually
fn parse_frontmatter_manual(yaml_str: &str) -> PresentationMeta {
    let mut meta = PresentationMeta::default();
    for line in yaml_str.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match key {
                "title" => meta.title = Some(value.to_string()),
                "author" => meta.author = Some(value.to_string()),
                "date" => meta.date = Some(value.to_string()),
                "@theme" => meta.theme = Some(value.to_string()),
                "@transition" => meta.transition = Some(value.to_string()),
                "@aspect" => meta.aspect = Some(value.to_string()),
                "@code-theme" => meta.code_theme = Some(value.to_string()),
                "@footer" => meta.footer = Some(value.to_string()),
                "@image-style" => meta.image_style = Some(value.to_string()),
                "@icon-style" => meta.icon_style = Some(value.to_string()),
                "@slide-level" => meta.slide_level = value.parse().ok(),
                _ => {}
            }
        }
    }
    meta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter() {
        let content = "---\ntitle: \"Hello\"\nauthor: \"Test\"\n@theme: dark\n---\n\n# Slide";
        let (meta, body) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("Hello"));
        assert_eq!(meta.author.as_deref(), Some("Test"));
        assert_eq!(meta.theme.as_deref(), Some("dark"));
        assert!(body.contains("# Slide"));
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "# Just a slide\n\nSome content";
        let (meta, body) = extract(content);
        assert!(meta.title.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn test_frontmatter_with_all_fields() {
        let content = "---\ntitle: \"Test\"\nauthor: \"Author\"\ndate: 2026-02-28\n@theme: light\n@transition: fade\n@aspect: 16:9\n@footer: \"footer text\"\n---\nBody";
        let (meta, body) = extract(content);
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
        let (meta, body) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("Test"));
        assert_eq!(meta.image_style.as_deref(), Some("Pixar"));
        assert_eq!(meta.icon_style.as_deref(), Some("minimal"));
        assert_eq!(body.trim(), "Body");
    }

    #[test]
    fn test_frontmatter_date_not_string() {
        let content = "---\ntitle: \"Test\"\ndate: 2026-02-28\n---\nBody";
        let (meta, _body) = extract(content);
        assert_eq!(meta.date.as_deref(), Some("2026-02-28"));
    }

    #[test]
    fn test_frontmatter_numeric_scalars() {
        // `date: 2026` is a YAML number; it must show as "2026", not
        // `Number(2026)`, and a numeric title must not be dropped.
        let content = "---\ntitle: 2026\ndate: 2026\nauthor: 3.5\n@footer: true\n---\nBody";
        let (meta, _body) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("2026"));
        assert_eq!(meta.date.as_deref(), Some("2026"));
        assert_eq!(meta.author.as_deref(), Some("3.5"));
        assert_eq!(meta.footer.as_deref(), Some("true"));
    }

    #[test]
    fn test_frontmatter_crlf() {
        let content = "---\r\ntitle: \"Hello\"\r\nauthor: \"Me\"\r\n@theme: nord\r\n---\r\n\r\n# Slide\r\n\r\nText\r\n";
        let (meta, body) = extract(content);
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
        let (meta, body) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("BOM"));
        assert_eq!(body.trim(), "# Slide");
    }

    #[test]
    fn test_frontmatter_bom_no_frontmatter() {
        let content = "\u{feff}# Slide\r\nText";
        let (meta, body) = extract(content);
        assert!(meta.title.is_none());
        assert_eq!(body, "# Slide\nText");
    }

    #[test]
    fn test_frontmatter_unclosed() {
        let content = "---\ntitle: x\n# Slide";
        let (meta, body) = extract(content);
        assert!(meta.title.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn test_frontmatter_closing_at_end_of_file() {
        let content = "---\ntitle: x\n---";
        let (meta, body) = extract(content);
        assert_eq!(meta.title.as_deref(), Some("x"));
        assert_eq!(body, "");
    }
}
