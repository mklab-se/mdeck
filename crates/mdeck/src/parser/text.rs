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

/// Call `f` with every inline run in `blocks`: headings, paragraphs, list
/// items, table cells, and the blocks inside quotes and callouts.
pub fn for_each_inlines(blocks: &[super::Block], f: &mut impl FnMut(&[Inline])) {
    use super::{Block, ListItem};
    fn items(list: &[ListItem], f: &mut impl FnMut(&[Inline])) {
        for item in list {
            f(&item.inlines);
            items(&item.children, f);
        }
    }
    for block in blocks {
        match block {
            Block::Heading { inlines, .. } | Block::Paragraph { inlines } => f(inlines),
            Block::BlockQuote { blocks } | Block::Callout { blocks, .. } => {
                for_each_inlines(blocks, f)
            }
            Block::List { items: list, .. } => items(list, f),
            Block::Table { headers, rows, .. } => {
                for cell in headers.iter().chain(rows.iter().flatten()) {
                    f(cell);
                }
            }
            _ => {}
        }
    }
}
