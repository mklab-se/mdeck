//! Speaker notes: ```` ```@notes ```` fenced blocks, anywhere in a slide.

use super::splitter::FenceTracker;

/// Take a slide's ```` ```@notes ```` blocks out of `raw`. Returns the
/// content with those blocks' lines blanked (every other line keeps its
/// place) and the notes: each block's markdown, joined in order.
pub(super) fn extract_notes(raw: &str) -> (String, Option<String>) {
    let mut fences = FenceTracker::new();
    let mut content: Vec<&str> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in raw.lines() {
        let opening = !fences.is_open();
        let fenced = fences.observe(line);
        if let Some(block) = current.as_mut() {
            if fences.is_open() {
                block.push(line);
            } else {
                // the closing fence
                notes.push(dedent(block).trim().to_string());
                current = None;
            }
            content.push("");
        } else if fenced && opening && is_notes_fence(line) {
            current = Some(Vec::new());
            content.push("");
        } else {
            content.push(line);
        }
    }
    // An unclosed notes block runs to the end of the slide.
    if let Some(block) = current {
        notes.push(dedent(&block).trim().to_string());
    }
    let notes: Vec<String> = notes.into_iter().filter(|n| !n.is_empty()).collect();
    let notes = (!notes.is_empty()).then(|| notes.join("\n\n"));
    (content.join("\n"), notes)
}

/// Whether a fence opening line is tagged `@notes`.
fn is_notes_fence(line: &str) -> bool {
    let info = line.trim().trim_start_matches(['`', '~']).trim();
    info.split_whitespace().next() == Some("@notes")
}

/// Remove the indentation every non-blank line shares (a notes block
/// inside a list item is indented with it).
fn dedent(lines: &[&str]) -> String {
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|l| l.get(indent..).unwrap_or("").trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Layout, parse};

    #[test]
    fn notes_blocks_are_taken_out_and_joined() {
        let raw = "# A\n\n```@notes\nfirst\n```\n\ntext\n\n````@notes\n## Heading\n\n```rust\ncode\n```\n````";
        let (content, notes) = extract_notes(raw);
        assert_eq!(content.split('\n').count(), raw.split('\n').count());
        assert!(content.contains("text"));
        assert!(!content.contains("first"));
        assert_eq!(
            notes.as_deref(),
            Some("first\n\n## Heading\n\n```rust\ncode\n```")
        );
        assert_eq!(extract_notes("# A\n\n```@notes\n```").1, None);
    }

    #[test]
    fn headings_and_rules_in_notes_never_split_the_slide() {
        // D5: notes are inside a fence, so nothing in them starts a slide.
        let md = "# One\n\n- a\n\n```@notes\n# Not a slide\n\n---\n\nMore notes\n```\n\n# Two\n";
        let pres = parse(md);
        assert_eq!(pres.slides.len(), 2, "{:?}", pres.slides);
        assert_eq!(
            pres.slides[0].notes.as_deref(),
            Some("# Not a slide\n\n---\n\nMore notes")
        );
        assert!(matches!(pres.slides[0].layout, Layout::Bullet));
        assert!(pres.slides[1].notes.is_none());
    }

    #[test]
    fn notes_in_a_list_item_are_dedented() {
        let (_, notes) = extract_notes("- item\n\n  ```@notes\n  - one\n    - two\n  ```");
        assert_eq!(notes.as_deref(), Some("- one\n  - two"));
    }

    #[test]
    fn triple_question_marks_are_text() {
        let pres = parse("# Slide\n\n???\n\nNot notes.");
        assert!(pres.slides[0].notes.is_none());
        assert_eq!(pres.slides[0].blocks.len(), 3);
    }

    #[test]
    fn notes_sample_has_notes_on_every_slide() {
        let pres = parse(include_str!("../../../../samples/features/notes.md"));
        assert!(pres.slides.len() >= 5);
        for (i, slide) in pres.slides.iter().enumerate() {
            assert!(slide.notes.is_some(), "slide {} has no notes", i + 1);
        }
    }
}
