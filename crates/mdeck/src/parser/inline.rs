use super::Inline;

/// Parse inline formatting from a text string.
pub fn parse(text: &str) -> Vec<Inline> {
    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut current_text = String::new();

    while i < chars.len() {
        // Inline code: `code`
        if chars[i] == '`' {
            flush_text(&mut current_text, &mut result);
            if let Some((code, end)) = parse_inline_code(&chars, i) {
                result.push(Inline::Code(code));
                i = end;
                continue;
            }
        }

        // Bold: **text**
        if chars[i] == '*' && peek(&chars, i + 1) == Some('*') {
            flush_text(&mut current_text, &mut result);
            if let Some((children, end)) = parse_delimited(&chars, i, "**", "**") {
                let inner = parse(&children);
                result.push(Inline::Bold(inner));
                i = end;
                continue;
            }
        }

        // Strikethrough: ~~text~~
        if chars[i] == '~' && peek(&chars, i + 1) == Some('~') {
            flush_text(&mut current_text, &mut result);
            if let Some((children, end)) = parse_delimited(&chars, i, "~~", "~~") {
                let inner = parse(&children);
                result.push(Inline::Strikethrough(inner));
                i = end;
                continue;
            }
        }

        // Italic: *text* (single star, not followed by another star)
        if chars[i] == '*' && peek(&chars, i + 1) != Some('*') {
            flush_text(&mut current_text, &mut result);
            if let Some((children, end)) = parse_delimited(&chars, i, "*", "*") {
                let inner = parse(&children);
                result.push(Inline::Italic(inner));
                i = end;
                continue;
            }
        }

        // Link: [text](url)
        if chars[i] == '[' {
            flush_text(&mut current_text, &mut result);
            if let Some((link, end)) = parse_link(&chars, i) {
                result.push(link);
                i = end;
                continue;
            }
        }

        current_text.push(chars[i]);
        i += 1;
    }

    flush_text(&mut current_text, &mut result);
    result
}

fn flush_text(current: &mut String, result: &mut Vec<Inline>) {
    if !current.is_empty() {
        result.push(Inline::Text(std::mem::take(current)));
    }
}

fn peek(chars: &[char], index: usize) -> Option<char> {
    chars.get(index).copied()
}

fn parse_inline_code(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut i = start + 1;
    let mut code = String::new();
    while i < chars.len() {
        if chars[i] == '`' {
            return Some((code, i + 1));
        }
        code.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_delimited(
    chars: &[char],
    start: usize,
    open: &str,
    close: &str,
) -> Option<(String, usize)> {
    let open_chars: Vec<char> = open.chars().collect();
    let close_chars: Vec<char> = close.chars().collect();

    // Verify opening delimiter
    for (j, &oc) in open_chars.iter().enumerate() {
        if peek(chars, start + j) != Some(oc) {
            return None;
        }
    }

    let content_start = start + open_chars.len();
    let mut i = content_start;
    let mut depth = 0;
    let mut content = String::new();

    while i < chars.len() {
        // Check for closing delimiter
        if depth == 0 {
            let mut matches = true;
            for (j, &cc) in close_chars.iter().enumerate() {
                if peek(chars, i + j) != Some(cc) {
                    matches = false;
                    break;
                }
            }
            if matches && !content.is_empty() {
                return Some((content, i + close_chars.len()));
            }
        }

        if chars[i] == '`' {
            depth = if depth == 0 { 1 } else { 0 };
        }

        content.push(chars[i]);
        i += 1;
    }

    None
}

fn parse_link(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    // [text](url)
    if chars[start] != '[' {
        return None;
    }

    let mut i = start + 1;
    let mut text = String::new();

    // Find closing ]
    let mut bracket_depth = 1;
    while i < chars.len() && bracket_depth > 0 {
        if chars[i] == '[' {
            bracket_depth += 1;
        } else if chars[i] == ']' {
            bracket_depth -= 1;
            if bracket_depth == 0 {
                break;
            }
        }
        text.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != ']' {
        return None;
    }
    i += 1; // skip ]

    // Expect (
    if i >= chars.len() || chars[i] != '(' {
        return None;
    }
    i += 1;

    // Find closing )
    let mut url = String::new();
    let mut paren_depth = 1;
    while i < chars.len() && paren_depth > 0 {
        if chars[i] == '(' {
            paren_depth += 1;
        } else if chars[i] == ')' {
            paren_depth -= 1;
            if paren_depth == 0 {
                break;
            }
        }
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != ')' {
        return None;
    }
    i += 1; // skip )

    let text_inlines = parse(&text);
    Some((
        Inline::Link {
            text: text_inlines,
            url,
        },
        i,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let result = parse("Hello world");
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], Inline::Text(s) if s == "Hello world"));
    }

    #[test]
    fn test_bold() {
        let result = parse("Hello **world**");
        assert_eq!(result.len(), 2);
        assert!(matches!(&result[0], Inline::Text(s) if s == "Hello "));
        assert!(matches!(&result[1], Inline::Bold(_)));
    }

    #[test]
    fn test_italic() {
        let result = parse("Hello *world*");
        assert_eq!(result.len(), 2);
        assert!(matches!(&result[0], Inline::Text(s) if s == "Hello "));
        assert!(matches!(&result[1], Inline::Italic(_)));
    }

    #[test]
    fn test_inline_code() {
        let result = parse("Use `println!` here");
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[1], Inline::Code(s) if s == "println!"));
    }

    #[test]
    fn test_link() {
        let result = parse("Click [here](https://example.com)");
        assert_eq!(result.len(), 2);
        assert!(matches!(&result[1], Inline::Link { url, .. } if url == "https://example.com"));
    }

    #[test]
    fn test_strikethrough() {
        let result = parse("This is ~~deleted~~ text");
        assert_eq!(result.len(), 3);
        assert!(matches!(&result[1], Inline::Strikethrough(_)));
    }

    #[test]
    fn test_mixed_formatting() {
        let result = parse("**bold** and *italic*");
        assert!(result.len() >= 3);
        assert!(matches!(&result[0], Inline::Bold(_)));
        assert!(matches!(&result[2], Inline::Italic(_)));
    }
}
