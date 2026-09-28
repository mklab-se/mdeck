//! Bullet and numbered lists: markers, nesting by indent and continuation lines.

use super::is_block_start;
use crate::parser::{Block, ListItem, ListMarker};

pub(super) fn is_list_start(line: &str) -> bool {
    let mut chars = line.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some('-' | '+' | '*'), Some(' '))
    )
}

/// Whether `line` opens an ordered item. The same test `extract_ordered_item`
/// makes, so `parse_list` always takes the line it was handed (`1 . x` used to
/// pass here, fail there and stall the parser).
pub(super) fn is_ordered_list_start(line: &str) -> bool {
    extract_ordered_item(line).is_some()
}

pub(super) fn parse_list(lines: &[&str], start: usize, ordered: bool) -> (Block, usize) {
    let mut items: Vec<ListItem> = Vec::new();
    let mut i = start;
    // The first item's indent is the list's base level; deeper lines nest.
    let base_indent = line_indent(lines[start]);

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            // Check if next non-blank line continues the list
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                let next = lines[j].trim();
                if is_list_start(next) || is_ordered_list_start(next) {
                    i = j;
                    continue;
                }
            }
            break;
        }

        let indent = line_indent(line);

        if indent <= base_indent {
            // Top-level item
            let item = if ordered {
                extract_ordered_item(trimmed)
            } else {
                extract_unordered_item(trimmed)
            };
            let Some((text, marker)) = item else {
                break;
            };
            i += 1;
            let text = collect_item_text(lines, &mut i, text);
            // Collect nested items
            let (children, new_i) = collect_children(lines, i, base_indent);
            items.push(ListItem {
                marker,
                inlines: crate::parser::inline::parse(&text),
                children,
            });
            i = new_i;
        } else {
            // A deeper-indented item after a blank line: nest it under the
            // last top-level item.
            let Some((text, marker)) = extract_any_list_item(trimmed) else {
                break;
            };
            i += 1;
            let text = collect_item_text(lines, &mut i, text);
            let (children, new_i) = collect_children(lines, i, indent);
            let item = ListItem {
                marker,
                inlines: crate::parser::inline::parse(&text),
                children,
            };
            match items.last_mut() {
                Some(last) => last.children.push(item),
                None => items.push(item),
            }
            i = new_i;
        }
    }

    (Block::List { ordered, items }, i)
}

/// Gather a list item's text: the marker line's text plus any following
/// continuation lines (wrapped or lazily indented text that is neither blank,
/// another list item, nor the start of another block). `i` is advanced past
/// the consumed lines.
fn collect_item_text(lines: &[&str], i: &mut usize, first: &str) -> String {
    let mut text = first.trim().to_string();
    while *i < lines.len() {
        let trimmed = lines[*i].trim();
        if trimmed.is_empty() || is_block_start(trimmed) {
            break;
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(trimmed);
        *i += 1;
    }
    text
}

fn collect_children(lines: &[&str], start: usize, parent_indent: usize) -> (Vec<ListItem>, usize) {
    let mut children = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        let indent = line_indent(line);
        if indent <= parent_indent {
            break;
        }

        if let Some((text, marker)) = extract_any_list_item(trimmed) {
            i += 1;
            let text = collect_item_text(lines, &mut i, text);

            // Recursively collect deeper children
            let (sub_children, new_i) = collect_children(lines, i, indent);
            children.push(ListItem {
                marker,
                inlines: crate::parser::inline::parse(&text),
                children: sub_children,
            });
            i = new_i;
        } else {
            break;
        }
    }

    (children, i)
}

fn extract_unordered_item(line: &str) -> Option<(&str, ListMarker)> {
    if line.len() < 2 {
        return None;
    }
    let first = line.chars().next()?;
    let second = line.chars().nth(1)?;
    if second != ' ' {
        return None;
    }
    let marker = match first {
        '-' => ListMarker::Static,
        '+' => ListMarker::NextStep,
        '*' => ListMarker::WithPrev,
        _ => return None,
    };
    Some((&line[2..], marker))
}

fn extract_ordered_item(line: &str) -> Option<(&str, ListMarker)> {
    let dot_pos = line.find(". ")?;
    if dot_pos == 0 || !line[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((&line[dot_pos + 2..], ListMarker::Ordered))
}

fn extract_any_list_item(line: &str) -> Option<(&str, ListMarker)> {
    extract_unordered_item(line).or_else(|| extract_ordered_item(line))
}

pub(super) fn line_indent(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::blocks::parse;
    use crate::parser::blocks::tests::{list_items, paragraph_text};
    use crate::parser::inlines_to_text;

    #[test]
    fn test_parse_unordered_list() {
        let blocks = parse("- First\n- Second\n- Third");
        assert_eq!(blocks.len(), 1);
        if let Block::List { ordered, items } = &blocks[0] {
            assert!(!ordered);
            assert_eq!(items.len(), 3);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_parse_list_markers() {
        let blocks = parse("- Static\n+ Next\n* WithPrev");
        assert_eq!(blocks.len(), 1);
        if let Block::List { items, .. } = &blocks[0] {
            assert_eq!(items[0].marker, ListMarker::Static);
            assert_eq!(items[1].marker, ListMarker::NextStep);
            assert_eq!(items[2].marker, ListMarker::WithPrev);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_nested_list() {
        let blocks = parse("- Parent\n  - Child\n    - Grandchild");
        assert_eq!(blocks.len(), 1);
        if let Block::List { items, .. } = &blocks[0] {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].children.len(), 1);
            assert_eq!(items[0].children[0].children.len(), 1);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_list_lazy_continuation_lines() {
        let blocks = parse("- First item that is long\n  and wraps here\n- Second");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        let items = list_items(&blocks[0]);
        assert_eq!(items.len(), 2);
        assert_eq!(
            inlines_to_text(&items[0].inlines),
            "First item that is long and wraps here"
        );
        assert_eq!(inlines_to_text(&items[1].inlines), "Second");

        // Unindented continuation
        let blocks = parse("- First\ncontinues\n- Second");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        let items = list_items(&blocks[0]);
        assert_eq!(inlines_to_text(&items[0].inlines), "First continues");

        // Ordered lists
        let blocks = parse("1. Step one\n   more detail\n2. Step two\nlazy");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        let items = list_items(&blocks[0]);
        assert_eq!(items.len(), 2);
        assert_eq!(inlines_to_text(&items[0].inlines), "Step one more detail");
        assert_eq!(inlines_to_text(&items[1].inlines), "Step two lazy");

        // Nested item continuation attaches to the nested item
        let blocks = parse("- Parent\n  - Child text\n    wrapped\n  - Sibling");
        let items = list_items(&blocks[0]);
        assert_eq!(items[0].children.len(), 2);
        assert_eq!(
            inlines_to_text(&items[0].children[0].inlines),
            "Child text wrapped"
        );

        // A real block start still ends the list
        let blocks = parse("- Item\n# Heading");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert!(matches!(blocks[1], Block::Heading { .. }));
        let blocks = parse("- Item\n\nParagraph");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
    }

    #[test]
    fn test_spaced_ordered_marker_is_text() {
        // `1 . x` looked like an ordered item to the dispatcher but not to
        // parse_list, which then consumed nothing and the loop never ended.
        let blocks = parse("# A\n\n1 . x");
        assert_eq!(blocks.len(), 2, "{blocks:?}");
        assert_eq!(paragraph_text(&blocks[1]), "1 . x");
        let blocks = parse("text\n12  . more");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        assert!(!is_ordered_list_start("1 . x"));
        assert!(is_ordered_list_start("10. x"));
    }

    #[test]
    fn test_indented_list_keeps_items() {
        // A list whose first item is indented used to drop all its items
        let blocks = parse("  - a\n  - b\n    - c");
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        let items = list_items(&blocks[0]);
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].children.len(), 1);
    }
}
