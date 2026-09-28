//! Pipe tables with a `|---|` separator row.

use crate::parser::{Block, Inline};

/// Whether a (trimmed) line looks like a table row: `| ... |`.
pub(super) fn is_table_line(trimmed: &str) -> bool {
    trimmed.starts_with('|') && trimmed.ends_with('|')
}

/// Parse a table starting at `lines[start]`. Returns `None` (consuming
/// nothing) when the lines don't form a table: fewer than two lines, or the
/// second line is not a `|---|` separator row.
pub(super) fn parse_table(lines: &[&str], start: usize) -> Option<(Block, usize)> {
    let mut table_lines: Vec<&str> = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with('|') {
            table_lines.push(trimmed);
            i += 1;
        } else if trimmed.is_empty() {
            i += 1;
            break;
        } else {
            break;
        }
    }

    if table_lines.len() < 2 || !is_table_separator(table_lines[1]) {
        return None;
    }

    // First line = headers
    let headers = parse_table_row(table_lines[0]);

    // Second line = separator (validated above)
    // Remaining lines = data rows
    let rows: Vec<Vec<Vec<Inline>>> = table_lines
        .iter()
        .skip(2)
        .map(|line| parse_table_row(line))
        .collect();

    Some((Block::Table { headers, rows }, i))
}

/// A separator row: every cell is `---`, `:--`, `--:` or `:-:` (1+ dashes).
fn is_table_separator(line: &str) -> bool {
    let cells = split_table_cells(line);
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let cell = cell.trim();
            let dashes = cell.trim_start_matches(':').trim_end_matches(':');
            !dashes.is_empty() && dashes.chars().all(|c| c == '-')
        })
}

fn parse_table_row(line: &str) -> Vec<Vec<Inline>> {
    split_table_cells(line)
        .iter()
        .map(|cell| crate::parser::inline::parse(cell.trim()))
        .collect()
}

/// Split a table row into raw cell strings, honouring `\|` (escaped pipe,
/// yielded as a literal `|`) and pipes inside backtick code spans. Leading and
/// trailing outer pipes are dropped.
fn split_table_cells(line: &str) -> Vec<String> {
    let line = line.trim();
    let chars: Vec<char> = line.chars().collect();
    let mut cells = Vec::new();
    let mut current = String::new();
    // Length of the backtick run that opened the current code span, if any
    let mut open_code: Option<usize> = None;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if chars.get(i + 1) == Some(&'|') => {
                current.push('|');
                i += 2;
            }
            '`' => {
                let run = chars[i..].iter().take_while(|&&c| c == '`').count();
                open_code = match open_code {
                    None => Some(run),
                    Some(n) if n == run => None,
                    other => other,
                };
                current.extend(std::iter::repeat_n('`', run));
                i += run;
            }
            '|' if open_code.is_none() => {
                cells.push(std::mem::take(&mut current));
                i += 1;
            }
            _ => {
                current.push(c);
                i += 1;
            }
        }
    }
    cells.push(current);

    // Drop the empty cells produced by the outer pipes
    if line.starts_with('|') && !cells.is_empty() {
        cells.remove(0);
    }
    if line.ends_with('|') && !line.ends_with("\\|") && cells.last().is_some_and(|c| c.is_empty()) {
        cells.pop();
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::blocks::parse;
    use crate::parser::blocks::tests::paragraph_text;
    use crate::parser::inlines_to_text;

    #[test]
    fn test_parse_table() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |";
        let blocks = parse(input);
        assert_eq!(blocks.len(), 1);
        if let Block::Table { headers, rows } = &blocks[0] {
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 1);
        } else {
            panic!("Expected Table");
        }
    }

    #[test]
    fn test_single_line_table_falls_back_to_paragraph() {
        let blocks = parse("| lonely |");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        assert_eq!(paragraph_text(&blocks[0]), "| lonely |");

        // Two pipe lines without a separator row are not a table either
        let blocks = parse("| a | b |\n| 1 | 2 |");
        assert!(blocks.iter().all(|b| matches!(b, Block::Paragraph { .. })));

        // But a valid table (with alignment colons) is
        let blocks = parse("| a | b |\n|:--|--:|\n| 1 | 2 |");
        assert!(matches!(blocks[0], Block::Table { .. }));
        assert!(is_table_separator("|---|:-:|"));
        assert!(is_table_separator("| --- | --- |"));
        assert!(!is_table_separator("| a | b |"));
        assert!(!is_table_separator("| --- | |"));
    }

    #[test]
    fn test_table_escaped_pipes_and_code_spans() {
        let input = "| Expr | Result |\n|---|---|\n| `a \\|\\| b` | x \\| y |\n| `c|d` | ``e|f`` |";
        let blocks = parse(input);
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        if let Block::Table { headers, rows } = &blocks[0] {
            assert_eq!(headers.len(), 2);
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].len(), 2);
            assert!(matches!(&rows[0][0][0], Inline::Code(s) if s == "a || b"));
            assert_eq!(inlines_to_text(&rows[0][1]), "x | y");
            assert_eq!(rows[1].len(), 2);
            assert!(matches!(&rows[1][0][0], Inline::Code(s) if s == "c|d"));
            assert!(matches!(&rows[1][1][0], Inline::Code(s) if s == "e|f"));
        } else {
            panic!("Expected Table");
        }
        assert_eq!(split_table_cells("| a | | b |"), vec![" a ", " ", " b "]);
        assert_eq!(split_table_cells("a | b"), vec!["a ", " b"]);
    }
}
