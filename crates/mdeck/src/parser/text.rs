//! Plain text of inline runs.

use super::Inline;

/// Extract plain text from inline elements.
pub fn inlines_to_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(s) => text.push_str(s),
            Inline::Bold(children) | Inline::Italic(children) | Inline::Strikethrough(children) => {
                text.push_str(&inlines_to_text(children));
            }
            Inline::Code(s) => text.push_str(s),
            Inline::Link { text: t, .. } => text.push_str(&inlines_to_text(t)),
            Inline::Math { tex, .. } => text.push_str(tex),
        }
    }
    text
}

/// Visible length of an inline in characters (not bytes), so CJK and other
/// multibyte text is measured the same way as ASCII.
pub(super) fn inline_text_len(inline: &Inline) -> usize {
    match inline {
        Inline::Text(s) => s.chars().count(),
        Inline::Bold(children) | Inline::Italic(children) | Inline::Strikethrough(children) => {
            children.iter().map(inline_text_len).sum()
        }
        Inline::Code(s) => s.chars().count(),
        Inline::Link { text, .. } => text.iter().map(inline_text_len).sum(),
        // roughly what the formula occupies on the line
        Inline::Math { tex, .. } => tex.chars().count().div_ceil(2),
    }
}
