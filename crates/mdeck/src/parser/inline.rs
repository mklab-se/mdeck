use super::Inline;

/// Parse inline formatting from a text string.
///
/// Supported syntax (a pragmatic subset of CommonMark + GFM):
/// - `**bold**` / `__bold__`, `*italic*` / `_italic_`, `***bold italic***`
/// - `~~strikethrough~~`
/// - `` `code` ``, with longer backtick runs to embed backticks (``` ``a`b`` ```)
/// - `[text](url)`, reference links `[text][label]`, `[text][]` and
///   `[label]` (resolved against the deck's definitions), autolinks
///   `<https://...>`; inline `![alt](url)` renders as link text
/// - footnote markers `[^1]` are hidden (their text goes to the notes)
/// - raw HTML tags are dropped and their text kept; `<br>` breaks the line
/// - backslash escapes (`\*`, `\_`, `\[`, `\$`, ...)
/// - `$math$` inline and `$$math$$` display LaTeX (rules in `parser::math`:
///   `$5 and $10` and `($K) and ($M)` stay text)
///
/// Emphasis delimiters must hug their content (`5 * 3 * 2` is plain text), and
/// `_` never opens or closes inside a word (`snake_case_name` stays literal).
pub fn parse(text: &str) -> Vec<Inline> {
    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut current_text = String::new();

    while i < chars.len() {
        if let Some(end) = try_hidden(&chars, i) {
            i = end;
        } else if let Some((inline, end)) = try_span(&chars, i) {
            flush_text(&mut current_text, &mut result);
            result.push(inline);
            i = end;
        } else {
            i = push_literal(&chars, i, &mut current_text);
        }
    }

    flush_text(&mut current_text, &mut result);
    result
}

/// The formatted span that starts at `chars[i]`, if one does, and the index
/// just past it.
fn try_span(chars: &[char], i: usize) -> Option<(Inline, usize)> {
    match chars[i] {
        // Math: $$display$$ or $inline$
        '$' => super::math::try_math(chars, i),
        // Code span: a run of N backticks closed by a run of exactly N
        '`' => try_code_span(chars, i),
        // Emphasis: * / _ (bold, italic, bold+italic)
        c @ ('*' | '_') => try_emphasis(chars, i, c),
        // Strikethrough: ~~text~~
        '~' => try_strikethrough(chars, i),
        // Link: [text](url), or a reference link
        '[' => parse_link(chars, i).or_else(|| reference_link(chars, i)),
        // Inline image: ![alt](url). No inline image rendering exists, so
        // show the alt text as a link rather than a stray `!` + link.
        '!' if peek(chars, i + 1) == Some('[') => {
            parse_link(chars, i + 1).or_else(|| reference_link(chars, i + 1))
        }
        // Autolink `<https://...>`, `<br>`, `<img>`
        '<' => autolink(chars, i).or_else(|| html_span(chars, i)),
        _ => None,
    }
}

/// Markup at `chars[i]` that shows nothing, and the index just past it: a
/// footnote marker `[^1]` (its text goes to the slide's notes) or an HTML
/// tag (its text is kept, the tag dropped).
fn try_hidden(chars: &[char], i: usize) -> Option<usize> {
    match chars[i] {
        '[' if peek(chars, i + 1) == Some('^') => {
            let close = (i + 2..chars.len()).find(|&j| chars[j] == ']')?;
            let id: String = chars[i + 2..close].iter().collect();
            if id.is_empty() || id.contains(char::is_whitespace) {
                return None;
            }
            super::references::use_footnote(&id);
            Some(close + 1)
        }
        '<' => {
            let (name, _, end) = html_tag(chars, i)?;
            (!matches!(name.as_str(), "br" | "img")).then_some(end)
        }
        _ => None,
    }
}

/// The HTML tags whose markup is dropped (and text kept). Others, such as
/// `Vec<T>` in prose, stay text.
const HTML_TAGS: &[&str] = &[
    "a",
    "abbr",
    "article",
    "aside",
    "b",
    "big",
    "blockquote",
    "br",
    "center",
    "cite",
    "code",
    "dd",
    "del",
    "details",
    "dfn",
    "div",
    "dl",
    "dt",
    "em",
    "figcaption",
    "figure",
    "font",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "i",
    "img",
    "ins",
    "kbd",
    "li",
    "main",
    "mark",
    "nav",
    "ol",
    "p",
    "picture",
    "pre",
    "q",
    "s",
    "samp",
    "section",
    "small",
    "source",
    "span",
    "strike",
    "strong",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "th",
    "thead",
    "tr",
    "tt",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

/// An HTML tag at `chars[i]`: its lowercase name, its attribute text and
/// the index just past its `>`.
pub(super) fn html_tag(chars: &[char], i: usize) -> Option<(String, String, usize)> {
    if chars.get(i) != Some(&'<') {
        return None;
    }
    let mut j = i + 1;
    if chars.get(j) == Some(&'/') {
        j += 1;
    }
    let name_start = j;
    while j < chars.len() && chars[j].is_ascii_alphanumeric() {
        j += 1;
    }
    let name: String = chars[name_start..j]
        .iter()
        .collect::<String>()
        .to_lowercase();
    if !HTML_TAGS.contains(&name.as_str()) {
        return None;
    }
    // After the name: whitespace and attributes, `/`, or the end.
    if !matches!(chars.get(j), Some(c) if c.is_whitespace() || *c == '>' || *c == '/') {
        return None;
    }
    let close = (j..chars.len()).find(|&k| chars[k] == '>')?;
    let attrs: String = chars[j..close].iter().collect();
    Some((name, attrs, close + 1))
}

/// The value of attribute `name` in an HTML tag's attribute text.
pub(super) fn html_attr(attrs: &str, name: &str) -> Option<String> {
    let lower = attrs.to_ascii_lowercase();
    let mut from = 0;
    while let Some(pos) = lower[from..].find(name) {
        let at = from + pos;
        let before_ok = at == 0 || !lower.as_bytes()[at - 1].is_ascii_alphanumeric();
        let rest = attrs[at + name.len()..].trim_start();
        if before_ok && let Some(value) = rest.strip_prefix('=') {
            let value = value.trim_start();
            return Some(match value.chars().next() {
                Some(q @ ('"' | '\'')) => value[1..].split(q).next().unwrap_or("").to_string(),
                _ => value
                    .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .next()
                    .unwrap_or("")
                    .to_string(),
            });
        }
        from = at + name.len();
    }
    None
}

/// `<br>` as a line break, `<img alt=... src=...>` as its alt text.
fn html_span(chars: &[char], i: usize) -> Option<(Inline, usize)> {
    let (name, attrs, end) = html_tag(chars, i)?;
    match name.as_str() {
        "br" => Some((Inline::Text("\n".into()), end)),
        "img" => {
            let alt = html_attr(&attrs, "alt").unwrap_or_default();
            let url = html_attr(&attrs, "src").unwrap_or_default();
            Some((
                Inline::Link {
                    text: parse(&alt),
                    url,
                },
                end,
            ))
        }
        _ => None,
    }
}

/// `<https://example.com>` or `<name@example.com>`: a link showing its target.
fn autolink(chars: &[char], i: usize) -> Option<(Inline, usize)> {
    let close = (i + 1..chars.len())
        .find(|&j| chars[j] == '>' || chars[j] == '<' || chars[j].is_whitespace())?;
    if chars[close] != '>' {
        return None;
    }
    let target: String = chars[i + 1..close].iter().collect();
    let scheme = target.split_once(':').map(|(s, _)| s);
    let is_uri = scheme.is_some_and(|s| {
        (2..=32).contains(&s.len())
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
    });
    let is_email = !is_uri
        && target
            .split_once('@')
            .is_some_and(|(a, b)| !a.is_empty() && b.contains('.') && !b.starts_with('.'));
    if !is_uri && !is_email {
        return None;
    }
    let url = if is_email {
        format!("mailto:{target}")
    } else {
        target.clone()
    };
    let shown = target
        .strip_prefix("mailto:")
        .unwrap_or(&target)
        .to_string();
    Some((
        Inline::Link {
            text: vec![Inline::Text(shown)],
            url,
        },
        close + 1,
    ))
}

/// `[text][label]`, `[text][]` or `[label]`, when the label is defined.
fn reference_link(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    let close = bracket_close(chars, start)?;
    let text: String = chars[start + 1..close].iter().collect();
    let (label, end) = if chars.get(close + 1) == Some(&'[') {
        let label_close = bracket_close(chars, close + 1)?;
        let label: String = chars[close + 2..label_close].iter().collect();
        (
            if label.trim().is_empty() {
                text.clone()
            } else {
                label
            },
            label_close + 1,
        )
    } else {
        (text.clone(), close + 1)
    };
    let url = super::references::resolve(&label)?;
    Some((
        Inline::Link {
            text: parse(&text),
            url,
        },
        end,
    ))
}

/// The index of the `]` closing the `[` at `start`.
fn bracket_close(chars: &[char], start: usize) -> Option<usize> {
    let mut depth = 0;
    let mut i = start;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Append the literal text at `chars[i]` to `out` and return the index after
/// it: an escaped character without its backslash, a whole run of unmatched
/// backticks, or a single character.
fn push_literal(chars: &[char], i: usize, out: &mut String) -> usize {
    match chars[i] {
        // Backslash escape: `\*` → literal `*`
        '\\' if chars.get(i + 1).is_some_and(|c| c.is_ascii_punctuation()) => {
            out.push(chars[i + 1]);
            i + 2
        }
        // Unmatched backticks are literal, the whole run at once so a shorter
        // run inside it can't close as a code span
        '`' => {
            let n = run_len(chars, i, '`');
            out.extend(std::iter::repeat_n('`', n));
            i + n
        }
        c => {
            out.push(c);
            i + 1
        }
    }
}

fn try_code_span(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    let n = run_len(chars, start, '`');
    let end = find_code_close(chars, start + n, n)?;
    let content = code_span_content(&chars[start + n..end - n]);
    Some((Inline::Code(content), end))
}

fn try_strikethrough(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    if run_len(chars, start, '~') != 2 {
        return None;
    }
    let end = find_closer(chars, start + 2, '~', 2, true)?;
    let inner: String = chars[start + 2..end].iter().collect();
    Some((Inline::Strikethrough(parse(&inner)), end + 2))
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

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn try_span_reports_where_each_span_ends() {
        let cases = [
            ("`a` x", 3),
            ("**b** x", 5),
            ("~~c~~ x", 5),
            ("[d](e) x", 6),
            ("![d](e) x", 7),
            ("$x$ y", 3),
        ];
        for (text, end) in cases {
            let (_, got) = try_span(&chars(text), 0).unwrap_or_else(|| panic!("{text:?}"));
            assert_eq!(got, end, "{text:?}");
        }
        for text in ["plain", "~~~x~~~", "! [x](y)", "`open", "\\*"] {
            assert!(try_span(&chars(text), 0).is_none(), "{text:?}");
        }
    }

    #[test]
    fn push_literal_consumes_escapes_and_backtick_runs() {
        let mut out = String::new();
        assert_eq!(push_literal(&chars("\\*x"), 0, &mut out), 2);
        assert_eq!(push_literal(&chars("\\a"), 0, &mut out), 1);
        assert_eq!(push_literal(&chars("```x"), 0, &mut out), 3);
        assert_eq!(push_literal(&chars("é"), 0, &mut out), 1);
        assert_eq!(out, "*\\```é");
    }

    #[test]
    fn html_tags_are_dropped_and_text_kept() {
        let r = parse("Press <kbd>Ctrl</kbd> + <b>C</b>");
        assert_eq!(inlines_to_text(&r), "Press Ctrl + C");
        let r = parse("one<br>two<br/>three");
        assert_eq!(inlines_to_text(&r), "one\ntwo\nthree");
        // Not HTML: stays text.
        assert_eq!(
            inlines_to_text(&parse("Vec<T> and a < b")),
            "Vec<T> and a < b"
        );
        let r = parse(r#"<img src="logo.png" alt="Logo" width=40>"#);
        assert!(matches!(&r[0], Inline::Link { url, .. } if url == "logo.png"));
        assert_eq!(
            html_attr(r#" src='a.png' alt=b"#, "alt").as_deref(),
            Some("b")
        );
    }

    #[test]
    fn autolinks_and_reference_links_are_links() {
        let r = parse("See <https://mdeck.dev> or <me@x.org>.");
        assert!(matches!(&r[1], Inline::Link { url, .. } if url == "https://mdeck.dev"));
        assert!(matches!(&r[3], Inline::Link { url, .. } if url == "mailto:me@x.org"));
        let (_, defs) = super::super::references::collect("[docs]: https://d.dev");
        super::super::references::with(defs, || {
            for md in ["[the docs][docs]", "[Docs][]", "[docs]"] {
                let r = parse(md);
                assert!(
                    matches!(&r[0], Inline::Link { url, .. } if url == "https://d.dev"),
                    "{md}: {r:?}"
                );
            }
            // Undefined labels stay text.
            assert_eq!(inlines_to_text(&parse("[x][nope] [y]")), "[x][nope] [y]");
        });
    }

    #[test]
    fn footnote_markers_are_hidden() {
        let r = parse("Fines rose[^1] sharply.");
        assert_eq!(inlines_to_text(&r), "Fines rose sharply.");
        let _ = super::super::references::take_footnotes();
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
