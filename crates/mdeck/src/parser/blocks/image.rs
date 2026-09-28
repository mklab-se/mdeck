//! Image lines: `![alt @fill](path "title")`.

use crate::parser::{Block, ImageDirectives};

pub(super) fn parse_image(line: &str) -> Option<Block> {
    // ![alt](path)
    if !line.starts_with("![") {
        return None;
    }

    let close_bracket = line.find("](")?;
    let alt_full = &line[2..close_bracket];

    let paren_start = close_bracket + 2;
    let paren_end = line[paren_start..].find(')')? + paren_start;
    let path = image_path(&line[paren_start..paren_end]);

    // Extract directives from alt text
    let (alt, directives) = parse_image_alt(alt_full);

    Some(Block::Image {
        alt,
        path,
        directives,
    })
}

/// Extract the path from the inside of an image's parentheses.
///
/// Drops an optional title (`img.png "Title"` / `img.png 'Title'` /
/// `img.png (Title)`) and unwraps `<angle brackets>`. Unquoted paths that
/// simply contain spaces are kept whole.
fn image_path(inner: &str) -> String {
    let inner = inner.trim();
    if let Some(rest) = inner.strip_prefix('<')
        && let Some(end) = rest.find('>')
    {
        return rest[..end].to_string();
    }
    let mut parts = inner.splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or("");
    let rest = parts.next().map(str::trim_start).unwrap_or("");
    if rest.starts_with('"') || rest.starts_with('\'') || rest.starts_with('(') {
        first.to_string()
    } else {
        inner.to_string()
    }
}

fn parse_image_alt(alt_full: &str) -> (String, ImageDirectives) {
    let mut directives = ImageDirectives::default();
    let mut alt_parts = Vec::new();

    for word in alt_full.split_whitespace() {
        if let Some(directive) = word.strip_prefix('@') {
            if directive == "fill" {
                directives.fill = true;
            } else if directive == "fit" {
                directives.fit = true;
            } else if directive == "left" {
                directives.align = Some("left".to_string());
            } else if directive == "right" {
                directives.align = Some("right".to_string());
            } else if directive == "center" {
                directives.align = Some("center".to_string());
            } else if let Some(val) = directive.strip_prefix("width:") {
                directives.width = Some(val.to_string());
            } else if let Some(val) = directive.strip_prefix("height:") {
                directives.height = Some(val.to_string());
            }
        } else {
            alt_parts.push(word);
        }
    }

    (alt_parts.join(" "), directives)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::blocks::parse;

    #[test]
    fn test_parse_image() {
        let blocks = parse("![Photo @fill](photo.jpg)");
        assert_eq!(blocks.len(), 1);
        if let Block::Image {
            alt,
            path,
            directives,
        } = &blocks[0]
        {
            assert_eq!(alt, "Photo");
            assert_eq!(path, "photo.jpg");
            assert!(directives.fill);
        } else {
            panic!("Expected Image");
        }
    }

    #[test]
    fn test_parse_image_width() {
        let blocks = parse("![Diagram @width:80%](diagram.png)");
        assert_eq!(blocks.len(), 1);
        if let Block::Image { directives, .. } = &blocks[0] {
            assert_eq!(directives.width.as_deref(), Some("80%"));
        } else {
            panic!("Expected Image");
        }
    }

    #[test]
    fn test_image_title_and_angle_brackets() {
        let blocks = parse("![alt](img.png \"Title\")");
        if let Block::Image { path, .. } = &blocks[0] {
            assert_eq!(path, "img.png");
        } else {
            panic!("Expected Image");
        }
        assert_eq!(image_path("img.png 'Title'"), "img.png");
        assert_eq!(image_path("img.png (Title)"), "img.png");
        assert_eq!(image_path("<my photo.png>"), "my photo.png");
        assert_eq!(image_path("<my photo.png> \"t\""), "my photo.png");
        // Unquoted paths with spaces are kept whole
        assert_eq!(image_path("my photo.png"), "my photo.png");
    }
}
