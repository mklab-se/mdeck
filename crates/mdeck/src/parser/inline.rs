use super::Inline;

/// Parse inline formatting from a text string.
///
/// Supported syntax (a pragmatic subset of CommonMark + GFM):
/// - `**bold**` / `__bold__`, `*italic*` / `_italic_`, `***bold italic***`
/// - `~~strikethrough~~`
/// - `` `code` ``, with longer backtick runs to embed backticks (``` ``a`b`` ```)
/// - `[text](url)`; inline `![alt](url)` renders as link text
/// - backslash escapes (`\*`, `\_`, `\[`, ...)
///
/// Emphasis delimiters must hug their content (`5 * 3 * 2` is plain text), and
/// `_` never opens or closes inside a word (`snake_case_name` stays literal).
pub fn parse(text: &str) -> Vec<Inline> {
    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut current_text = String::new();

    while i < chars.len() {
        let c = chars[i];

        // Backslash escape: `\*` → literal `*`
        if c == '\\' {
            if let Some(&next) = chars.get(i + 1)
                && next.is_ascii_punctuation()
            {
                current_text.push(next);
                i += 2;
                continue;
            }
            current_text.push(c);
            i += 1;
            continue;
        }

        // Code span: a run of N backticks closed by a run of exactly N
        if c == '`' {
            let n = run_len(&chars, i, '`');
            if let Some(end) = find_code_close(&chars, i + n, n) {
                flush_text(&mut current_text, &mut result);
                result.push(Inline::Code(code_span_content(&chars[i + n..end - n])));
                i = end;
            } else {
                // Unmatched backticks are literal
                current_text.extend(std::iter::repeat_n('`', n));
                i += n;
            }
            continue;
        }

        // Emphasis: * / _ (bold, italic, bold+italic)
        if (c == '*' || c == '_')
            && let Some((inline, end)) = try_emphasis(&chars, i, c)
        {
            flush_text(&mut current_text, &mut result);
            result.push(inline);
            i = end;
            continue;
        }

        // Strikethrough: ~~text~~
        if c == '~'
            && peek(&chars, i + 1) == Some('~')
            && run_len(&chars, i, '~') == 2
            && let Some(end) = find_closer(&chars, i + 2, '~', 2, true)
        {
            flush_text(&mut current_text, &mut result);
            let inner: String = chars[i + 2..end].iter().collect();
            result.push(Inline::Strikethrough(parse(&inner)));
            i = end + 2;
            continue;
        }

        // Link: [text](url)
        if c == '['
            && let Some((link, end)) = parse_link(&chars, i)
        {
            flush_text(&mut current_text, &mut result);
            result.push(link);
            i = end;
            continue;
        }

        // Inline image: ![alt](url) — no inline image rendering exists, so
        // show the alt text as a link rather than a stray `!` + link.
        if c == '!'
            && peek(&chars, i + 1) == Some('[')
            && let Some((link, end)) = parse_link(&chars, i + 1)
        {
            flush_text(&mut current_text, &mut result);
            result.push(link);
            i = end;
            continue;
        }

        current_text.push(c);
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

/// Length of the run of `ch` starting at `start`.
fn run_len(chars: &[char], start: usize, ch: char) -> usize {
    chars[start..].iter().take_while(|&&c| c == ch).count()
}

/// Find the closing backtick run of exactly `n` backticks at or after `from`.
/// Returns the index just past the closing run.
fn find_code_close(chars: &[char], from: usize, n: usize) -> Option<usize> {
    let mut j = from;
    while j < chars.len() {
        if chars[j] == '`' {
            let m = run_len(chars, j, '`');
            if m == n {
                return Some(j + m);
            }
            j += m;
        } else {
            j += 1;
        }
    }
    None
}

/// Code span content per CommonMark: strip one leading and one trailing space
/// when both are present and the content is not entirely spaces.
fn code_span_content(inner: &[char]) -> String {
    let s: String = inner.iter().collect();
    if s.len() >= 2 && s.starts_with(' ') && s.ends_with(' ') && !s.trim().is_empty() {
        s[1..s.len() - 1].to_string()
    } else {
        s
    }
}

/// Try to parse an emphasis span starting at `start` (which holds `delim`).
///
/// A run of 3+ delimiters is tried as bold+italic first, then bold, then
/// italic. Openers must be followed by non-whitespace; closers must be
/// preceded by non-whitespace. `_` additionally must not be intraword.
fn try_emphasis(chars: &[char], start: usize, delim: char) -> Option<(Inline, usize)> {
    let n = run_len(chars, start, delim);
    let intraword_ok = delim == '*';

    // `_` cannot open inside a word
    if !intraword_ok && start > 0 && chars[start - 1].is_alphanumeric() {
        return None;
    }

    let candidates: &[usize] = match n {
        1 => &[1],
        2 => &[2],
        _ => &[3, 2, 1],
    };

    for &len in candidates {
        let content_start = start + len;
        match chars.get(content_start) {
            Some(c) if !c.is_whitespace() => {}
            _ => continue,
        }
        if let Some(close) = find_closer(chars, content_start, delim, len, intraword_ok) {
            let content: String = chars[content_start..close].iter().collect();
            let inner = parse(&content);
            let inline = match len {
                3 => Inline::Bold(vec![Inline::Italic(inner)]),
                2 => Inline::Bold(inner),
                _ => Inline::Italic(inner),
            };
            return Some((inline, close + len));
        }
    }
    None
}

/// Find a closing delimiter run of exactly `len` × `delim` at or after `from`,
/// skipping escaped characters and code spans. The closer must be preceded by
/// non-whitespace; when `intraword_ok` is false it must also not be followed
/// by an alphanumeric character.
fn find_closer(
    chars: &[char],
    from: usize,
    delim: char,
    len: usize,
    intraword_ok: bool,
) -> Option<usize> {
    let mut j = from;
    while j < chars.len() {
        let c = chars[j];
        if c == '\\' {
            j += 2;
            continue;
        }
        if c == '`' {
            let m = run_len(chars, j, '`');
            j = find_code_close(chars, j + m, m).unwrap_or(j + m);
            continue;
        }
        if c == delim {
            let m = run_len(chars, j, delim);
            let preceded_ok = j > from && !chars[j - 1].is_whitespace();
            let followed_ok =
                intraword_ok || !chars.get(j + m).is_some_and(|c| c.is_alphanumeric());
            if m == len && preceded_ok && followed_ok {
                return Some(j);
            }
            j += m;
            continue;
        }
        j += 1;
    }
    None
}

fn parse_link(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    // [text](url)
    if chars.get(start) != Some(&'[') {
        return None;
    }

    let mut i = start + 1;
    let mut text = String::new();

    // Find closing ]
    let mut bracket_depth = 1;
    while i < chars.len() && bracket_depth > 0 {
        if chars[i] == '\\' && i + 1 < chars.len() {
            text.push(chars[i]);
            text.push(chars[i + 1]);
            i += 2;
            continue;
        }
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
    use crate::parser::inlines_to_text;

    fn text_of(inline: &Inline) -> String {
        inlines_to_text(std::slice::from_ref(inline))
    }

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

    // --- Regression tests for inline parser gaps ---

    #[test]
    fn test_underscore_italic_and_bold() {
        let result = parse("_italic_ and __bold__");
        assert_eq!(result.len(), 3, "{result:?}");
        assert!(matches!(&result[0], Inline::Italic(inner) if inlines_to_text(inner) == "italic"));
        assert!(matches!(&result[1], Inline::Text(s) if s == " and "));
        assert!(matches!(&result[2], Inline::Bold(inner) if inlines_to_text(inner) == "bold"));
    }

    #[test]
    fn test_underscore_intraword_is_literal() {
        let result = parse("use snake_case_name here");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Text(s) if s == "use snake_case_name here"));

        let result = parse("file_name_here.rs and __dunder__");
        assert!(matches!(&result[0], Inline::Text(s) if s == "file_name_here.rs and "));
        assert!(matches!(&result[1], Inline::Bold(_)));

        let result = parse("MY_CONST_VALUE");
        assert!(matches!(&result[0], Inline::Text(s) if s == "MY_CONST_VALUE"));
    }

    #[test]
    fn test_double_backtick_code_span() {
        let result = parse("``a`b``");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Code(s) if s == "a`b"));

        // Leading/trailing space stripped so a code span can start with a backtick
        let result = parse("`` `x` ``");
        assert!(matches!(&result[0], Inline::Code(s) if s == "`x`"));

        // Unmatched backticks are literal
        let result = parse("a `` b");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Text(s) if s == "a `` b"));
    }

    #[test]
    fn test_triple_star_bold_italic() {
        let result = parse("***both***");
        assert_eq!(result.len(), 1, "{result:?}");
        match &result[0] {
            Inline::Bold(inner) => {
                assert_eq!(inner.len(), 1);
                assert!(matches!(&inner[0], Inline::Italic(i) if inlines_to_text(i) == "both"));
            }
            other => panic!("expected Bold(Italic), got {other:?}"),
        }
    }

    #[test]
    fn test_spaced_stars_are_not_emphasis() {
        let result = parse("5 * 3 * 2");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Text(s) if s == "5 * 3 * 2"));

        let result = parse("a ** b ** c");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Text(s) if s == "a ** b ** c"));
    }

    #[test]
    fn test_closer_must_hug_content() {
        // `*foo *` is not italic (closer preceded by whitespace)
        let result = parse("*foo * bar");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Text(s) if s == "*foo * bar"));
    }

    #[test]
    fn test_backslash_escapes() {
        let result = parse(r"\*lit\*");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Text(s) if s == "*lit*"));

        let result = parse(r"\_not italic\_ and \[not a link\](x)");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Text(s) if s == "_not italic_ and [not a link](x)"));

        // Backslash before a non-punctuation char stays literal
        let result = parse(r"C:\path\to");
        assert!(matches!(&result[0], Inline::Text(s) if s == r"C:\path\to"));

        // Escaped delimiter inside emphasis does not close it
        let result = parse(r"*a\*b*");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Italic(inner) if inlines_to_text(inner) == "a*b"));
    }

    #[test]
    fn test_inline_image_renders_as_link_text() {
        let result = parse("see ![diagram](img.png) here");
        assert_eq!(result.len(), 3, "{result:?}");
        assert!(matches!(&result[0], Inline::Text(s) if s == "see "));
        assert!(
            matches!(&result[1], Inline::Link { text, url } if inlines_to_text(text) == "diagram" && url == "img.png")
        );
        assert!(matches!(&result[2], Inline::Text(s) if s == " here"));
        assert_eq!(inlines_to_text(&result), "see diagram here");
    }

    #[test]
    fn test_emphasis_skips_code_spans() {
        let result = parse("*a `*` b*");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Italic(_)));
        assert_eq!(text_of(&result[0]), "a * b");
    }

    #[test]
    fn test_nested_emphasis() {
        let result = parse("**bold with *italic* inside**");
        assert_eq!(result.len(), 1, "{result:?}");
        match &result[0] {
            Inline::Bold(inner) => {
                assert_eq!(inner.len(), 3);
                assert!(matches!(&inner[1], Inline::Italic(_)));
            }
            other => panic!("expected Bold, got {other:?}"),
        }
        let result = parse("*a **b** c*");
        assert_eq!(result.len(), 1, "{result:?}");
        assert!(matches!(&result[0], Inline::Italic(_)));
        assert_eq!(text_of(&result[0]), "a b c");
    }

    #[test]
    fn test_unclosed_delimiters_are_literal() {
        assert_eq!(inlines_to_text(&parse("**unclosed")), "**unclosed");
        assert_eq!(inlines_to_text(&parse("a * b")), "a * b");
        assert_eq!(inlines_to_text(&parse("~~x")), "~~x");
        assert_eq!(inlines_to_text(&parse("[x](")), "[x](");
        assert_eq!(inlines_to_text(&parse("![x] y")), "![x] y");
        // `**foo*` → literal `*` + italic foo
        let result = parse("**foo*");
        assert!(matches!(&result[0], Inline::Text(s) if s == "*"));
        assert!(matches!(&result[1], Inline::Italic(_)));
    }

    #[test]
    fn test_intraword_star_still_works() {
        let result = parse("2*3*4");
        assert_eq!(result.len(), 3, "{result:?}");
        assert!(matches!(&result[1], Inline::Italic(_)));
    }

    #[test]
    fn test_multibyte_text() {
        let result = parse("**héllo** → *wörld* 🎉");
        assert_eq!(inlines_to_text(&result), "héllo → wörld 🎉");
        assert!(matches!(&result[0], Inline::Bold(_)));
    }
}
