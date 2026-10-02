//! Parsing: one word per line, `Word (size: N)`, with reveal markers.

use crate::render::visualizations::{VizReveal, parse_reveal_prefix};

#[derive(Debug, Clone)]
pub(super) struct WordEntry {
    pub(super) text: String,
    pub(super) size: f32,
    pub(super) reveal: VizReveal,
}

pub(super) fn parse_word_cloud(content: &str) -> Vec<WordEntry> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (text, reveal) = parse_reveal_prefix(trimmed);
        if text.is_empty() {
            continue;
        }

        // Parse "Word (size: N)" or "Word"
        let (label, size) = if let Some(paren_start) = text.find('(') {
            let before = text[..paren_start].trim();
            let meta = &text[paren_start..];
            let size = parse_size_meta(meta).unwrap_or(20.0);
            (before.to_string(), size)
        } else {
            (text.to_string(), 20.0)
        };

        if !label.is_empty() {
            entries.push(WordEntry {
                text: label,
                size,
                reveal,
            });
        }
    }
    entries
}

fn parse_size_meta(meta: &str) -> Option<f32> {
    let inner = meta.trim_start_matches('(').trim_end_matches(')');
    for part in inner.split(',') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("size:")
            && let Ok(s) = val.trim().parse::<f32>()
        {
            return Some(s);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_word_cloud_basic() {
        let content = "- Data Science (size: 40)\n- AI (size: 50)";
        let entries = parse_word_cloud(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "Data Science");
        assert_eq!(entries[0].size, 40.0);
        assert_eq!(entries[1].text, "AI");
        assert_eq!(entries[1].size, 50.0);
    }

    #[test]
    fn test_parse_word_cloud_no_size() {
        let content = "- Hello World";
        let entries = parse_word_cloud(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Hello World");
        assert_eq!(entries[0].size, 20.0);
    }

    #[test]
    fn test_parse_word_cloud_reveal_markers() {
        let content = "- Static\n+ Step1\n* WithPrev";
        let entries = parse_word_cloud(content);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].reveal, VizReveal::Static);
        assert_eq!(entries[1].reveal, VizReveal::NextStep);
        assert_eq!(entries[2].reveal, VizReveal::Static);
    }

    #[test]
    fn test_parse_word_cloud_skips_comments() {
        let content = "# comment\n- Word (size: 30)";
        let entries = parse_word_cloud(content);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_parse_size_meta() {
        assert_eq!(parse_size_meta("(size: 40)"), Some(40.0));
        assert_eq!(parse_size_meta("(size: 12.5)"), Some(12.5));
        assert_eq!(parse_size_meta("(invalid)"), None);
    }
}
