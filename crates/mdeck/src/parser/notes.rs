//! Speaker notes: everything after a `???` line.

use super::splitter;

/// Extract speaker notes from a raw slide string.
///
/// Notes are separated from slide content by a `???` line (three or more `?` characters).
/// The `???` separator is ignored inside fenced code blocks.
/// Returns `(content, Some(notes))` if a notes separator was found, or `(original, None)`.
pub(super) fn extract_notes(raw: &str) -> (String, Option<String>) {
    let mut fences = splitter::FenceTracker::new();

    let lines: Vec<&str> = raw.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let in_code_fence = fences.observe(line);

        // Check for notes separator (??? with 3+ question marks, outside code blocks)
        if !in_code_fence && trimmed.len() >= 3 && trimmed.chars().all(|c| c == '?') {
            let content = lines[..i].join("\n");
            let notes_text = if i + 1 < lines.len() {
                lines[i + 1..].join("\n").trim().to_string()
            } else {
                String::new()
            };
            let notes = if notes_text.is_empty() {
                None
            } else {
                Some(notes_text)
            };
            return (content, notes);
        }
    }

    (raw.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Layout, parse};

    #[test]
    fn extract_notes_splits_at_the_first_separator() {
        assert_eq!(
            extract_notes("a\n???\nb\n???\nc"),
            ("a".to_string(), Some("b\n???\nc".to_string()))
        );
        assert_eq!(extract_notes("a\n??"), ("a\n??".to_string(), None));
        assert_eq!(extract_notes("a\n???\n  "), ("a".to_string(), None));
    }

    #[test]
    fn test_notes_basic() {
        let content = "# My Slide\n\n- Point one\n- Point two\n\n???\n\nThis is the speaker note.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert_eq!(
            pres.slides[0].notes.as_deref(),
            Some("This is the speaker note.")
        );
        // Blocks should NOT contain notes content
        assert!(matches!(pres.slides[0].layout, Layout::Bullet));
    }

    #[test]
    fn test_notes_none() {
        let content = "# Simple Slide\n\nJust content, no notes.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(pres.slides[0].notes.is_none());
    }

    #[test]
    fn test_notes_multiline_with_formatting() {
        let content =
            "# Slide\n\n???\n\nFirst line of notes.\n\nSecond paragraph with **bold** text.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert_eq!(
            pres.slides[0].notes.as_deref(),
            Some("First line of notes.\n\nSecond paragraph with **bold** text.")
        );
    }

    #[test]
    fn test_notes_inside_code_block_ignored() {
        let content = "# Slide\n\n```\n???\nsome code\n```\n\nParagraph after code.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(
            pres.slides[0].notes.is_none(),
            "??? inside code block should not be treated as notes separator"
        );
    }

    #[test]
    fn test_notes_four_question_marks() {
        let content = "# Slide\n\n????\n\nNotes with four question marks.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert_eq!(
            pres.slides[0].notes.as_deref(),
            Some("Notes with four question marks.")
        );
    }

    #[test]
    fn test_notes_separator_as_first_line() {
        let content = "???\n\nOnly notes, no visible content.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert_eq!(
            pres.slides[0].notes.as_deref(),
            Some("Only notes, no visible content.")
        );
        assert!(pres.slides[0].blocks.is_empty());
    }

    #[test]
    fn test_notes_empty_after_separator() {
        let content = "# Slide\n\n???";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        // Empty notes after separator → None
        assert!(pres.slides[0].notes.is_none());
    }

    #[test]
    fn test_notes_across_multiple_slides() {
        let content =
            "# Slide 1\n\n???\n\nNotes for slide 1\n\n---\n\n# Slide 2\n\n???\n\nNotes for slide 2";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 2);
        assert_eq!(pres.slides[0].notes.as_deref(), Some("Notes for slide 1"));
        assert_eq!(pres.slides[1].notes.as_deref(), Some("Notes for slide 2"));
    }

    #[test]
    fn test_notes_layout_unaffected() {
        // Notes content should not affect layout classification
        let content =
            "# Title\n\nSubtitle\n\n???\n\n- This list in notes should not make it a Bullet layout";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(
            matches!(pres.slides[0].layout, Layout::Title),
            "Layout should be Title, got {:?}",
            pres.slides[0].layout
        );
    }

    #[test]
    fn test_notes_existing_samples_regression() {
        // Existing presentations should have no notes (no ??? separators)
        let content = include_str!("../../../../samples/poker-night.md");
        let pres = parse(content);
        for (i, slide) in pres.slides.iter().enumerate() {
            assert!(
                slide.notes.is_none(),
                "Slide {} in poker-night.md should have no notes",
                i
            );
        }
    }

    #[test]
    fn test_notes_inside_tilde_code_block_ignored() {
        let content = "# Slide\n\n~~~\n???\nsome code\n~~~\n\nParagraph after code.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(
            pres.slides[0].notes.is_none(),
            "??? inside tilde code block should not be treated as notes separator"
        );
    }

    #[test]
    fn test_notes_only_first_separator_counts() {
        let content = "# Slide\n\n???\n\nFirst notes section\n\n???\n\nSecond section";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        // Only the first ??? should split; everything after it is notes
        let notes = pres.slides[0].notes.as_deref().unwrap();
        assert!(notes.contains("First notes section"));
        assert!(notes.contains("???"));
        assert!(notes.contains("Second section"));
    }

    #[test]
    fn test_notes_separator_with_whitespace() {
        let content = "# Slide\n\n   ???   \n\nNotes with whitespace separator.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert_eq!(
            pres.slides[0].notes.as_deref(),
            Some("Notes with whitespace separator.")
        );
    }

    #[test]
    fn test_notes_separator_with_mixed_content_not_separator() {
        // "??? some text" should NOT be a notes separator: it has non-? chars
        let content = "# Slide\n\n??? some text here\n\nMore content.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(
            pres.slides[0].notes.is_none(),
            "??? followed by text should not be a notes separator"
        );
    }

    #[test]
    fn test_notes_two_question_marks_not_separator() {
        let content = "# Slide\n\n??\n\nNot notes.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(
            pres.slides[0].notes.is_none(),
            "?? (only 2 marks) should not trigger notes separator"
        );
    }

    #[test]
    fn test_notes_with_directives() {
        // Directives should still work when notes are present
        let content = "@layout: code\n# Code Example\n\n```rust\nfn main() {}\n```\n\n???\n\nExplain that this is a minimal Rust program.";
        let pres = parse(content);
        assert_eq!(pres.slides.len(), 1);
        assert!(
            matches!(pres.slides[0].layout, Layout::Code),
            "Layout should be Code when @layout directive is present, got {:?}",
            pres.slides[0].layout
        );
        assert_eq!(
            pres.slides[0].notes.as_deref(),
            Some("Explain that this is a minimal Rust program.")
        );
    }

    #[test]
    fn test_notes_sample_presentation_parses() {
        let content = include_str!("../../../../samples/features/notes.md");
        let pres = parse(content);
        assert!(
            pres.slides.len() >= 5,
            "Expected at least 5 slides in notes sample, got {}",
            pres.slides.len()
        );
        // Every slide in the notes sample should have notes
        for (i, slide) in pres.slides.iter().enumerate() {
            assert!(
                slide.notes.is_some(),
                "Slide {} in notes.md should have notes",
                i
            );
        }
    }
}
