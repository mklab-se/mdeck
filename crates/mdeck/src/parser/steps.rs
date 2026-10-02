//! Steps: `+` items reveal one press at a time, numbered across the whole
//! slide in reading order, over lists and visuals alike.

use super::{Block, Chart, ListItem, ListMarker};

/// How many steps a visual's own `+` items take.
pub type VisualSteps<'a> = &'a dyn Fn(&Block) -> usize;

/// The steps a visual takes by its source alone (a thermal image's real
/// count depends on what its source can show; the deck renumbers with it).
pub fn default_visual_steps(block: &Block) -> usize {
    match block {
        Block::Diagram { content, .. } => crate::render::diagram::count_diagram_steps(content),
        Block::Chart {
            kind: Chart::Thermal,
            content,
            ..
        } => crate::render::thermal::default_steps(content),
        Block::Chart { content, .. } => crate::render::visualizations::count_viz_steps(content),
        _ => 0,
    }
}

/// Number the steps of a slide's blocks: each `+` item takes the next step
/// and its children appear with it (a nested `+` takes a step of its own);
/// each visual's steps follow the steps before it. Returns the slide's last
/// step. With `reveal` off every item shows from the start and visuals
/// show all their items.
pub fn number(blocks: &mut [Block], reveal: bool, visual_steps: VisualSteps) -> usize {
    let mut counter = 0;
    number_blocks(blocks, reveal, visual_steps, &mut counter);
    counter
}

fn number_blocks(
    blocks: &mut [Block],
    reveal: bool,
    visual_steps: VisualSteps,
    counter: &mut usize,
) {
    for block in blocks.iter_mut() {
        let own = default_or(visual_steps, block);
        match block {
            Block::List { items, .. } => number_items(items, reveal, 0, counter),
            Block::BlockQuote { blocks } | Block::Callout { blocks, .. } => {
                number_blocks(blocks, reveal, visual_steps, counter)
            }
            Block::Diagram { content, step_base }
            | Block::Chart {
                content, step_base, ..
            } => {
                *step_base = *counter;
                if reveal {
                    *counter += own;
                } else {
                    *content = without_steps(content);
                }
            }
            _ => {}
        }
    }
}

fn default_or(visual_steps: VisualSteps, block: &Block) -> usize {
    match block {
        Block::Diagram { .. } | Block::Chart { .. } => visual_steps(block),
        _ => 0,
    }
}

fn number_items(items: &mut [ListItem], reveal: bool, inherited: usize, counter: &mut usize) {
    for item in items {
        item.step = if reveal && item.marker == ListMarker::NextStep {
            *counter += 1;
            *counter
        } else {
            inherited
        };
        number_items(&mut item.children, reveal, item.step, counter);
    }
}

/// A visual's source with its `+` items made static (`reveal: none`).
fn without_steps(content: &str) -> String {
    content
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            match trimmed.strip_prefix("+ ") {
                Some(rest) => format!("{}- {rest}", &line[..line.len() - trimmed.len()]),
                None => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::blocks;

    fn steps(items: &[ListItem]) -> Vec<usize> {
        let mut out = Vec::new();
        for item in items {
            out.push(item.step);
            out.extend(steps(&item.children));
        }
        out
    }

    fn list(block: &Block) -> &[ListItem] {
        match block {
            Block::List { items, .. } => items,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn steps_follow_reading_order_across_lists() {
        // MD-18: the second list continues after the first.
        let mut b = blocks::parse("+ a\n- b\n+ c\n\ntext\n\n+ d\n+ e");
        assert_eq!(number(&mut b, true, &default_visual_steps), 4);
        assert_eq!(steps(list(&b[0])), [1, 0, 2]);
        assert_eq!(steps(list(&b[2])), [3, 4]);
    }

    #[test]
    fn children_reveal_with_their_parent() {
        // MD-17, D12: a `+` item's children come with it; a nested `+`
        // takes its own step, so no press reveals nothing.
        let mut b = blocks::parse("+ a\n  - a1\n  - a2\n+ b\n  + b1\n- c\n  + c1");
        assert_eq!(number(&mut b, true, &default_visual_steps), 4);
        assert_eq!(steps(list(&b[0])), [1, 1, 1, 2, 3, 0, 4]);
    }

    #[test]
    fn visuals_continue_the_count() {
        let mut b = blocks::parse("+ a\n\n```@bar\n+ x: 1\n+ y: 2\n```\n\n+ b");
        assert_eq!(number(&mut b, true, &default_visual_steps), 4);
        assert!(matches!(b[1], Block::Chart { step_base: 1, .. }));
        assert_eq!(steps(list(&b[2])), [4]);
    }

    #[test]
    fn reveal_none_shows_everything() {
        // MD-21
        let mut b = blocks::parse("+ a\n+ b\n\n```@bar\n+ x: 1\n```");
        assert_eq!(number(&mut b, false, &default_visual_steps), 0);
        assert_eq!(steps(list(&b[0])), [0, 0]);
        assert!(matches!(&b[1], Block::Chart { content, .. } if content == "- x: 1"));
    }
}
