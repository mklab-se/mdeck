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

/// The image options in an alt text (LANG-12, VIZ-11), in the settings
/// grammar: `@fill`, `@width: 60%` and `@height: 400px` (the space after
/// the colon is optional). Everything else written `@...` is reported:
/// placement (`@left`, `@right`, `@center`) belongs to the slide's design,
/// and `@fit` is what every image does anyway.
fn parse_image_alt(alt_full: &str) -> (String, ImageDirectives) {
    let mut directives = ImageDirectives::default();
    let mut alt_parts = Vec::new();

    let mut words = alt_full.split_whitespace().peekable();
    while let Some(word) = words.next() {
        let Some(option) = word.strip_prefix('@') else {
            alt_parts.push(word);
            continue;
        };
        let (key, value) = match option.split_once(':') {
            Some((k, v)) if v.trim().is_empty() => (k, words.next().unwrap_or("").to_string()),
            Some((k, v)) => (k, v.to_string()),
            None => (option, String::new()),
        };
        let size = |value: String, slot: &mut Option<String>, problems: &mut Vec<String>| {
            if valid_size(&value) {
                *slot = Some(value);
            } else {
                problems.push(format!(
                    "image option `@{key}: {value}` needs a size like 60% or 400px"
                ));
            }
        };
        match key {
            "fill" if value.is_empty() => directives.fill = true,
            "width" => size(value, &mut directives.width, &mut directives.problems),
            "height" => size(value, &mut directives.height, &mut directives.problems),
            "left" | "right" | "center" | "fit" => directives.problems.push(format!(
                "image option `@{key}` was removed in v2: an image's place is up to the slide's design"
            )),
            _ => directives.problems.push(format!(
                "`@{option}` is not an image option (@fill, @width: 60%, @height: 400px)"
            )),
        }
    }

    (alt_parts.join(" "), directives)
}

/// A size: a percentage or pixels (`60%`, `400px`, `400`).
fn valid_size(value: &str) -> bool {
    let v = value.trim();
    let number = v
        .strip_suffix('%')
        .or_else(|| v.strip_suffix("px"))
        .unwrap_or(v)
        .trim();
    number.parse::<f32>().is_ok_and(|n| n > 0.0)
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
    fn options_take_an_optional_space_and_unknown_ones_are_reported() {
        let blocks = parse("![Team @width: 60% @height:300px @fill](team.png)");
        let Block::Image {
            alt, directives, ..
        } = &blocks[0]
        else {
            panic!("Expected Image");
        };
        assert_eq!(alt, "Team");
        assert_eq!(directives.width.as_deref(), Some("60%"));
        assert_eq!(directives.height.as_deref(), Some("300px"));
        assert!(directives.fill && directives.problems.is_empty());

        let (alt, d) = parse_image_alt("A @left @fit @zoom @width: wide");
        assert_eq!(alt, "A");
        assert_eq!(d.problems.len(), 4, "{:?}", d.problems);
        assert!(d.problems[0].contains("removed in v2"));
        assert!(d.problems[2].contains("not an image option"));
        assert!(d.problems[3].contains("needs a size"));
        assert_eq!(d.width, None);
    }

    #[test]
    fn unknown_image_options_reach_the_slide_problems_on_their_line() {
        let pres = crate::parser::parse("# A\n\ntext\n\n![x @left](a.png)\n");
        let p = &pres.slides[0].problems;
        assert_eq!(p.len(), 1, "{p:?}");
        assert_eq!(p[0].line, 5);
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
