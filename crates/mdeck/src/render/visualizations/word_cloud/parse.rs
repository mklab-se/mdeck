//! Parsing: one word per item, `- Word (size: N)`, with reveal markers.

use crate::render::visualizations::grammar::{Problem, Source};
use crate::render::visualizations::{VizReveal, parse_value};

/// The size of a word without `(size: N)`.
const DEFAULT_SIZE: f32 = 20.0;

#[derive(Debug, Clone)]
pub(super) struct WordEntry {
    pub(super) text: String,
    pub(super) size: f32,
    pub(super) reveal: VizReveal,
}

fn read(src: &Source) -> Vec<WordEntry> {
    src.check_settings(&[]);
    let mut entries = Vec::new();
    for item in &src.items {
        item.check_attrs(src, &["size"]);
        if item.text.is_empty() {
            src.problem(item.offset, "an empty word");
            continue;
        }
        let size = match item.attr("size") {
            None => DEFAULT_SIZE,
            Some(s) => match parse_value(s).filter(|v| *v > 0.0) {
                Some(v) => v,
                None => {
                    src.problem(item.offset, format!("size: '{s}' is not a positive number"));
                    DEFAULT_SIZE
                }
            },
        };
        entries.push(WordEntry {
            text: item.text.to_string(),
            size,
            reveal: item.reveal,
        });
    }
    entries
}

pub(super) fn parse_word_cloud(content: &str) -> Vec<WordEntry> {
    read(&Source::parse(content))
}

/// The problems in a `@wordcloud` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
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
    fn bad_sizes_and_attributes_are_reported() {
        let content = "- A (size: big)\n- B (weight: 3)\n- C# (size: 12.5)";
        let entries = parse_word_cloud(content);
        assert_eq!(entries[0].size, DEFAULT_SIZE);
        assert_eq!(entries[2].text, "C#");
        assert_eq!(entries[2].size, 12.5);
        let lines: Vec<usize> = check(content).iter().map(|p| p.offset).collect();
        assert_eq!(lines, [0, 1]);
    }
}
