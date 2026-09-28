mod code;
mod image;
mod lists;
mod tables;

use super::Block;
use code::{is_fence_start, parse_code_block};
use image::parse_image;
use lists::{is_list_start, is_ordered_list_start, parse_list};
use tables::{is_table_line, parse_table};

pub use super::directives::extract_directives;
pub(crate) use super::directives::parse_directive_line;

/// Parse a slide's content string into a Vec<Block>.
pub fn parse(content: &str) -> Vec<Block> {
    let lines: Vec<&str> = content.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let (block, end) = parse_block(&lines, i);
        blocks.extend(block);
        i = end;
    }
    blocks
}

/// Parse the block that starts at `lines[i]`. Returns it (`None` for blank
/// lines and comments, which hold no block) and the index of the first line
/// after it, which is always past `i`.
fn parse_block(lines: &[&str], i: usize) -> (Option<Block>, usize) {
    let trimmed = lines[i].trim();

    // Skip blank lines
    if trimmed.is_empty() {
        return (None, i + 1);
    }

    // Column separator: +++
    if trimmed == "+++" {
        return (Some(Block::ColumnSeparator), i + 1);
    }

    // HTML comment: <!-- ... --> (possibly spanning lines), never rendered
    if trimmed.starts_with("<!--") {
        return (None, skip_html_comment(lines, i));
    }

    // Horizontal rule: *** or ___
    if is_horizontal_rule(trimmed) {
        return (Some(Block::HorizontalRule), i + 1);
    }

    // Heading: # ...
    if let Some(heading) = parse_heading(trimmed) {
        return (Some(heading), i + 1);
    }

    // Fenced code block: ``` or ~~~
    if is_fence_start(trimmed) {
        let fence_char = if trimmed.starts_with("```") { '`' } else { '~' };
        let (block, end) = parse_code_block(lines, i, fence_char);
        return (Some(block), end);
    }

    // Image: ![alt](path)
    if let Some(img) = parse_image(trimmed) {
        return (Some(img), i + 1);
    }

    // Blockquote: > ...
    if is_blockquote_start(trimmed) {
        let (block, end) = parse_blockquote(lines, i);
        return (Some(block), end);
    }

    // Table: | ... |
    if is_table_line(trimmed)
        && let Some((table, end)) = parse_table(lines, i)
    {
        return (Some(table), end);
    }
    // Pipe lines that don't form a table fall through to a paragraph

    // Unordered list: - or + or *  (but not --- or ***)
    if is_list_start(trimmed) {
        let (block, end) = parse_list(lines, i, false);
        return (Some(block), end);
    }

    // Ordered list: 1. ...
    if is_ordered_list_start(trimmed) {
        let (block, end) = parse_list(lines, i, true);
        return (Some(block), end);
    }

    // Paragraph: collect consecutive non-blank, non-special lines.
    // parse_paragraph always consumes at least one line, so the loop
    // can never stall on a line no other branch accepted.
    let (block, end) = parse_paragraph(lines, i);
    (Some(block), end.max(i + 1))
}

fn is_blockquote_start(trimmed: &str) -> bool {
    trimmed.starts_with("> ") || trimmed == ">"
}

/// True if a (trimmed) line begins a block that a paragraph or a list item's
/// text must not swallow. Mirrors exactly what the main `parse` loop accepts,
/// so a line that isn't a real heading/image/etc. stays paragraph text.
fn is_block_start(trimmed: &str) -> bool {
    trimmed == "+++"
        || trimmed.starts_with("<!--")
        || is_horizontal_rule(trimmed)
        || parse_heading(trimmed).is_some()
        || is_fence_start(trimmed)
        || parse_image(trimmed).is_some()
        || is_blockquote_start(trimmed)
        || is_table_line(trimmed)
        || is_list_start(trimmed)
        || is_ordered_list_start(trimmed)
}

/// Skip an HTML comment starting at `lines[start]`. Returns the index of the
/// first line after the `-->` terminator (or `lines.len()` if unterminated).
fn skip_html_comment(lines: &[&str], start: usize) -> usize {
    let mut i = start;
    while i < lines.len() {
        // The opening `<!--` itself may be immediately followed by `-->`
        let hay = if i == start {
            lines[i].trim().get(4..).unwrap_or("")
        } else {
            lines[i]
        };
        i += 1;
        if hay.contains("-->") {
            break;
        }
    }
    i
}

fn is_horizontal_rule(line: &str) -> bool {
    let mut chars = line.chars().filter(|c| !c.is_whitespace());
    let Some(first) = chars.next() else {
        return false;
    };
    if first != '*' && first != '_' {
        return false;
    }
    let mut count = 1;
    for c in chars {
        if c != first {
            return false;
        }
        count += 1;
    }
    count >= 3
}

fn parse_heading(line: &str) -> Option<Block> {
    if !line.starts_with('#') {
        return None;
    }

    let level = line.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }

    // `#` is ASCII so the byte index equals the char index here
    let rest = &line[level..];
    if !rest.starts_with(' ') && !rest.is_empty() {
        return None;
    }

    let text = strip_closing_hashes(rest.trim());
    let inlines = super::inline::parse(text);
    Some(Block::Heading {
        level: level as u8,
        inlines,
    })
}

/// Remove an optional closing `#` sequence: `## Head ##` → `Head`.
/// Per CommonMark the closing run must be preceded by a space (`# C#` keeps
/// its hash), or the heading text must consist solely of hashes (`# ##` → ``).
fn strip_closing_hashes(text: &str) -> &str {
    let stripped = text.trim_end_matches('#');
    if stripped.is_empty() || stripped.ends_with(' ') {
        stripped.trim_end()
    } else {
        text
    }
}

/// Setext heading underline: `===` (H1) or `---` (H2), at least 3 chars.
fn setext_level(trimmed: &str) -> Option<u8> {
    if trimmed.len() < 3 {
        return None;
    }
    if trimmed.chars().all(|c| c == '=') {
        Some(1)
    } else if trimmed.chars().all(|c| c == '-') {
        Some(2)
    } else {
        None
    }
}

fn parse_blockquote(lines: &[&str], start: usize) -> (Block, usize) {
    let mut quote_text = String::new();
    let mut i = start;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if let Some(rest) = trimmed.strip_prefix("> ") {
            if !quote_text.is_empty() {
                quote_text.push(' ');
            }
            quote_text.push_str(rest);
            i += 1;
        } else if trimmed == ">" {
            if !quote_text.is_empty() {
                quote_text.push(' ');
            }
            i += 1;
        } else {
            break;
        }
    }

    let inlines = super::inline::parse(&quote_text);
    (Block::BlockQuote { inlines }, i)
}

/// Parse a paragraph starting at `lines[start]`.
///
/// The first line is always consumed, even if it looks special but was
/// rejected by every other block parser (`#hashtag`, `![alt] text`), so the
/// caller can never stall. Subsequent lines join until a blank line or the
/// start of another block. A one-line paragraph followed by a `===`/`---`
/// underline becomes a setext heading.
fn parse_paragraph(lines: &[&str], start: usize) -> (Block, usize) {
    let mut text = String::new();
    let mut i = start;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        if i > start {
            // Setext heading: exactly one paragraph line + underline
            if i == start + 1
                && let Some(level) = setext_level(trimmed)
            {
                let inlines = super::inline::parse(&text);
                return (Block::Heading { level, inlines }, i + 1);
            }
            // Stop at blank lines or special block starts
            if trimmed.is_empty() || is_block_start(trimmed) {
                break;
            }
        }

        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(trimmed);
        i += 1;
    }

    let inlines = super::inline::parse(&text);
    (Block::Paragraph { inlines }, i)
}

#[cfg(test)]
mod tests {
    use super::lists::{is_list_start, line_indent};
    use super::*;
    use crate::parser::ListItem;

    #[test]
    fn test_parse_heading() {
        let blocks = parse("# Title");
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::Heading { level: 1, .. }));
    }

    #[test]
    fn test_parse_blockquote() {
        let blocks = parse("> This is a quote\n> with multiple lines");
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::BlockQuote { .. }));
    }

    #[test]
    fn test_parse_horizontal_rule() {
        let blocks = parse("Some text\n\n***\n\nMore text");
        assert!(blocks.iter().any(|b| matches!(b, Block::HorizontalRule)));
    }

    #[test]
    fn test_parse_column_separator() {
        let blocks = parse("Left content\n\n+++\n\nRight content");
        assert!(blocks.iter().any(|b| matches!(b, Block::ColumnSeparator)));
    }

    // --- Regression tests ---

    use crate::parser::inlines_to_text;

    pub(super) fn paragraph_text(block: &Block) -> String {
        match block {
            Block::Paragraph { inlines } => inlines_to_text(inlines),
            other => panic!("expected Paragraph, got {other:?}"),
        }
    }

    pub(super) fn heading(block: &Block) -> (u8, String) {
        match block {
            Block::Heading { level, inlines } => (*level, inlines_to_text(inlines)),
            other => panic!("expected Heading, got {other:?}"),
        }
    }

    pub(super) fn list_items(block: &Block) -> &Vec<ListItem> {
        match block {
            Block::List { items, .. } => items,
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn test_hash_lines_that_are_not_headings_do_not_hang() {
        // Each of these used to make `parse` loop forever: parse_heading
        // rejected the line and parse_paragraph refused to consume it.
        for input in ["#hashtag", "#include <stdio.h>", "#######", "#\tTitle", "#"] {
            let blocks = parse(input);
            assert_eq!(blocks.len(), 1, "input {input:?} → {blocks:?}");
        }
        assert_eq!(paragraph_text(&parse("#hashtag")[0]), "#hashtag");
        assert_eq!(paragraph_text(&parse("#######")[0]), "#######");
        assert_eq!(heading(&parse("#")[0]), (1, String::new()));

        // Inside a paragraph too
        let blocks = parse("Follow #rust on\n#mastodon today");
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            paragraph_text(&blocks[0]),
            "Follow #rust on #mastodon today"
        );
    }

    #[test]
    fn test_invalid_image_lines_do_not_hang() {
        for input in ["![alt] text", "![alt](path", "![", "![]"] {
            let blocks = parse(input);
            assert_eq!(blocks.len(), 1, "input {input:?} → {blocks:?}");
            assert!(matches!(blocks[0], Block::Paragraph { .. }));
        }
        let blocks = parse("line one\n![alt] text\nline three");
        assert_eq!(blocks.len(), 1);
        assert_eq!(
            paragraph_text(&blocks[0]),
            "line one ![alt] text line three"
        );
    }

    #[test]
    fn test_single_multibyte_char_lines_do_not_panic() {
        for input in ["🎉", "→", "é", "→ x", "- 🎉", "**", "*", "_", "|", "~"] {
            let blocks = parse(input);
            assert_eq!(blocks.len(), 1, "input {input:?} → {blocks:?}");
        }
        assert!(!is_list_start("→"));
        assert!(!is_horizontal_rule("→"));
        assert!(!is_horizontal_rule("**"));
        assert!(is_horizontal_rule("* * *"));
        assert!(is_horizontal_rule("___"));
        assert!(is_list_start("- 🎉"));
        let blocks = parse("- 🎉 party");
        let items = list_items(&blocks[0]);
        assert_eq!(inlines_to_text(&items[0].inlines), "🎉 party");
        // Indentation is counted in characters, not bytes
        assert_eq!(line_indent("\t\tx"), 2);
    }

    #[test]
    fn test_setext_headings() {
        let blocks = parse("Title\n=====\n\nSub\n-----\n\ntext");
        assert_eq!(blocks.len(), 3, "{blocks:?}");
        assert_eq!(heading(&blocks[0]), (1, "Title".to_string()));
        assert_eq!(heading(&blocks[1]), (2, "Sub".to_string()));
        assert_eq!(paragraph_text(&blocks[2]), "text");

        // Multi-line paragraphs are not turned into headings
        let blocks = parse("one\ntwo\n===");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        assert!(matches!(blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_closing_hashes() {
        assert_eq!(heading(&parse("## Head ##")[0]), (2, "Head".to_string()));
        assert_eq!(
            heading(&parse("# Title #####")[0]),
            (1, "Title".to_string())
        );
        assert_eq!(heading(&parse("# C#")[0]), (1, "C#".to_string()));
        assert_eq!(heading(&parse("# ##")[0]), (1, String::new()));
    }

    #[test]
    fn test_html_comments_are_skipped() {
        let blocks = parse("<!-- hidden -->\n# Title\n\n<!--\nmulti\nline\n-->\n\nText");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert!(matches!(blocks[0], Block::Heading { .. }));
        assert_eq!(paragraph_text(&blocks[1]), "Text");

        // Comment interrupting a paragraph
        let blocks = parse("before\n<!-- note -->\nafter");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert_eq!(paragraph_text(&blocks[0]), "before");
        assert_eq!(paragraph_text(&blocks[1]), "after");

        // Unterminated comment swallows the rest without hanging
        let blocks = parse("<!-- oops\nText");
        assert!(blocks.is_empty(), "{blocks:?}");

        // Comments before directives don't end the directive prelude
        let (dirs, content) = extract_directives("<!-- c -->\n@layout: quote\n> q");
        assert_eq!(dirs.len(), 1);
        assert!(content.contains("> q"));
    }

    #[test]
    fn test_paragraph_breaks_only_on_real_blocks() {
        // Lines resembling block starts but rejected by their parser stay in
        // the paragraph; real block starts still end it.
        let blocks = parse("text\n![bad] img\n#tag\n> quote");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert_eq!(paragraph_text(&blocks[0]), "text ![bad] img #tag");
        assert!(matches!(blocks[1], Block::BlockQuote { .. }));
    }

    #[test]
    fn parse_block_always_moves_forward() {
        let lines = [
            "", "+++", "<!-- a", "b -->", "# H", "```", "x", "```", "text", "more",
        ];
        let mut steps = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            let (block, end) = parse_block(&lines, i);
            assert!(end > i, "stalled at line {i}");
            steps.push((i, block.is_some()));
            i = end;
        }
        assert_eq!(
            steps,
            [
                (0, false),
                (1, true),
                (2, false),
                (4, true),
                (5, true),
                (8, true)
            ]
        );
    }
}
