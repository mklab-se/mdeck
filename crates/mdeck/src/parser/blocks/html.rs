//! Raw HTML lines (a README's centred logo and title, `<details>`): the
//! tags are dropped and their text kept, `<img>` becomes an image and
//! `<h1>`..`<h6>` a heading. Inline tags inside ordinary text are handled
//! by the inline parser.

use crate::parser::inline::{html_attr, html_tag};
use crate::parser::{Block, ImageDirectives, Inline};

/// Tags that start an HTML line of their own; a line opening with any other
/// tag (`<b>`, `<kbd>`) is ordinary text.
const BLOCK_TAGS: &[&str] = &[
    "article",
    "aside",
    "blockquote",
    "br",
    "center",
    "details",
    "div",
    "dl",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "img",
    "main",
    "nav",
    "p",
    "picture",
    "section",
    "source",
    "summary",
    "table",
    "tbody",
    "td",
    "th",
    "thead",
    "tr",
    "video",
];

/// Whether a (trimmed) line opens with an HTML block tag.
pub(super) fn is_html_line(trimmed: &str) -> bool {
    let chars: Vec<char> = trimmed.chars().collect();
    html_tag(&chars, 0).is_some_and(|(name, _, _)| BLOCK_TAGS.contains(&name.as_str()))
}

/// The blocks an HTML line shows: its images, a heading for `<hN>`, else
/// its text without the tags as a paragraph (nothing when only tags remain).
pub(super) fn parse_html_line(trimmed: &str) -> Vec<Block> {
    let chars: Vec<char> = trimmed.chars().collect();
    let mut images = Vec::new();
    let mut heading: Option<u8> = None;
    let mut bold = false;
    let mut text = String::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some((name, attrs, end)) = html_tag(&chars, i) {
            let closing = chars.get(i + 1) == Some(&'/');
            match name.as_str() {
                "img" => {
                    if let Some(src) = html_attr(&attrs, "src").filter(|s| !s.is_empty()) {
                        images.push(Block::Image {
                            alt: html_attr(&attrs, "alt").unwrap_or_default(),
                            path: src,
                            directives: ImageDirectives::default(),
                        });
                    }
                }
                "br" => text.push('\n'),
                "summary" if !closing => bold = true,
                h if !closing && h.len() == 2 && h.starts_with('h') => {
                    heading = h[1..].parse().ok();
                }
                _ => {}
            }
            i = end;
        } else {
            text.push(chars[i]);
            i += 1;
        }
    }
    let text = text.trim();
    if !images.is_empty() {
        return images;
    }
    if text.is_empty() {
        return Vec::new();
    }
    let inlines = crate::parser::inline::parse(text);
    vec![match heading {
        Some(level) => Block::Heading { level, inlines },
        None if bold => Block::Paragraph {
            inlines: vec![Inline::Bold(inlines)],
        },
        None => Block::Paragraph { inlines },
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::blocks::parse;
    use crate::parser::inlines_to_text;

    #[test]
    fn a_readme_header_presents_cleanly() {
        let md = "<p align=\"center\">\n  <img src=\"media/logo.png\" alt=\"mdeck\" width=\"200\">\n</p>\n<h1 align=\"center\">mdeck</h1>\n<p align=\"center\"><b>Markdown</b> slides</p>";
        let blocks = parse(md);
        assert_eq!(blocks.len(), 3, "{blocks:?}");
        assert!(
            matches!(&blocks[0], Block::Image { path, alt, .. } if path == "media/logo.png" && alt == "mdeck")
        );
        assert!(
            matches!(&blocks[1], Block::Heading { level: 1, inlines } if inlines_to_text(inlines) == "mdeck")
        );
        assert!(
            matches!(&blocks[2], Block::Paragraph { inlines } if inlines_to_text(inlines) == "Markdown slides")
        );
    }

    #[test]
    fn details_keep_their_text() {
        let blocks = parse("<details>\n<summary>More</summary>\n\nHidden text\n\n</details>");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert!(
            matches!(&blocks[0], Block::Paragraph { inlines } if matches!(inlines[0], Inline::Bold(_)))
        );
        assert!(!is_html_line("<b>bold</b> start"));
        assert!(is_html_line("<div>"));
    }
}
