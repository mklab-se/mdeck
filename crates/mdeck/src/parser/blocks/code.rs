//! Fenced blocks: code with its language and highlighted lines, and the
//! `@` fences that hold diagrams and charts.

use crate::parser::{Block, Chart};

/// Whether a (trimmed) line opens a ``` or ~~~ fence.
pub(super) fn is_fence_start(trimmed: &str) -> bool {
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

pub(super) fn parse_code_block(lines: &[&str], start: usize, fence_char: char) -> (Block, usize) {
    let opening = lines[start].trim();
    let fence_prefix: String = opening.chars().take_while(|&c| c == fence_char).collect();
    let fence_len = fence_prefix.len();

    // Parse language and highlight spec from opening line
    let after_fence = &opening[fence_len..];
    let (language, highlight_lines, viz_kind) = parse_code_info(after_fence.trim());

    let mut code_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        // Check for closing fence
        let closing_count = trimmed.chars().take_while(|&c| c == fence_char).count();
        if closing_count >= fence_len
            && trimmed
                .chars()
                .skip(closing_count)
                .all(|c| c.is_whitespace())
        {
            i += 1;
            break;
        }
        code_lines.push(lines[i]);
        i += 1;
    }

    let code = code_lines.join("\n");

    let block = match viz_kind {
        VizKind::Diagram => Block::Diagram {
            content: code,
            step_base: 0,
        },
        VizKind::Chart(kind) => Block::Chart {
            kind,
            content: code,
            step_base: 0,
        },
        VizKind::None => Block::CodeBlock {
            language,
            code,
            highlight_lines,
        },
    };
    (block, i)
}

/// Which visualization type a code block represents (if any).
#[derive(Debug, Clone, Copy, PartialEq)]
enum VizKind {
    None,
    Diagram,
    Chart(Chart),
}

fn parse_code_info(info: &str) -> (Option<String>, Vec<usize>, VizKind) {
    if info.is_empty() {
        return (None, vec![], VizKind::None);
    }

    // Check for visualization language tags
    if info.split_whitespace().next() == Some("@architecture") {
        return (None, vec![], VizKind::Diagram);
    }
    if let Some(chart) = Chart::from_info(info) {
        return (None, vec![], VizKind::Chart(chart));
    }

    // Parse language and optional highlight spec.
    // The language is always the first whitespace-separated token, so
    // `rust title=x {1}` → "rust".
    let (lang_part, highlight_part) = if let Some(brace_start) = info.find('{') {
        let rest = &info[brace_start..];
        let highlight = if let Some(brace_end) = rest.find('}') {
            parse_highlight_spec(&rest[1..brace_end])
        } else {
            vec![]
        };
        (&info[..brace_start], highlight)
    } else {
        (info, vec![])
    };
    let lang_part = lang_part.split_whitespace().next().unwrap_or("");

    let language = if lang_part.is_empty() {
        None
    } else {
        Some(lang_part.to_string())
    };

    (language, highlight_part, VizKind::None)
}

/// Upper bound on the number of lines a single highlight range may span, so a
/// typo like `{1-99999999999}` can't allocate billions of entries.
const MAX_HIGHLIGHT_RANGE: usize = 10_000;

fn parse_highlight_spec(spec: &str) -> Vec<usize> {
    let mut lines = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if let Some((start, end)) = part.split_once('-') {
            if let (Ok(s), Ok(e)) = (start.trim().parse::<usize>(), end.trim().parse::<usize>()) {
                if s > e {
                    continue;
                }
                let e = e.min(s.saturating_add(MAX_HIGHLIGHT_RANGE));
                lines.extend(s..=e);
            }
        } else if let Ok(n) = part.parse::<usize>() {
            lines.push(n);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::blocks::parse;

    #[test]
    fn test_parse_code_block() {
        let blocks = parse("```rust\nfn main() {}\n```");
        assert_eq!(blocks.len(), 1);
        if let Block::CodeBlock { language, code, .. } = &blocks[0] {
            assert_eq!(language.as_deref(), Some("rust"));
            assert_eq!(code, "fn main() {}");
        } else {
            panic!("Expected CodeBlock");
        }
    }

    #[test]
    fn test_parse_diagram_block() {
        let blocks = parse("```@architecture\n- A -> B: hello\n```");
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], Block::Diagram { .. }));
    }

    #[test]
    fn test_highlight_spec() {
        let result = parse_highlight_spec("3,5-7");
        assert_eq!(result, vec![3, 5, 6, 7]);
    }

    #[test]
    fn test_highlight_spec_is_bounded() {
        let result = parse_highlight_spec("1-99999999999");
        assert_eq!(result.len(), MAX_HIGHLIGHT_RANGE + 1);
        assert_eq!(result[0], 1);
        // Reversed ranges are rejected, single lines and sane ranges still work
        assert_eq!(parse_highlight_spec("9-3"), Vec::<usize>::new());
        assert_eq!(parse_highlight_spec("3,5-7,x"), vec![3, 5, 6, 7]);
        assert_eq!(
            parse_highlight_spec("18446744073709551615-18446744073709551615").len(),
            1
        );
        let blocks = parse("```rust {1-99999999999}\nfn main() {}\n```");
        if let Block::CodeBlock {
            highlight_lines, ..
        } = &blocks[0]
        {
            assert_eq!(highlight_lines.len(), MAX_HIGHLIGHT_RANGE + 1);
        } else {
            panic!("Expected CodeBlock");
        }
    }

    #[test]
    fn test_code_info_language_is_first_token() {
        let blocks = parse("```rust title=x {1}\nfn main() {}\n```");
        if let Block::CodeBlock {
            language,
            highlight_lines,
            ..
        } = &blocks[0]
        {
            assert_eq!(language.as_deref(), Some("rust"));
            assert_eq!(highlight_lines, &vec![1]);
        } else {
            panic!("Expected CodeBlock");
        }
        let (lang, hl, _) = parse_code_info("python linenos");
        assert_eq!(lang.as_deref(), Some("python"));
        assert!(hl.is_empty());
        let (lang, hl, _) = parse_code_info("{2}");
        assert!(lang.is_none());
        assert_eq!(hl, vec![2]);
        let (lang, _, _) = parse_code_info("rust{3}");
        assert_eq!(lang.as_deref(), Some("rust"));
    }
}
