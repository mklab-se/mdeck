//! `@flower` source: a `center`, `petal`s around it, and `A -> B` links
//! between petals.

use super::super::node_text::{NodeText, parse_link};
use super::super::{assign_steps, parse_reveal_prefix};

#[derive(Debug, Clone, PartialEq)]
pub struct Flower {
    pub center: Option<NodeText>,
    pub center_step: usize,
    pub petals: Vec<Petal>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Petal {
    pub text: NodeText,
    pub step: usize,
}

/// A link from one petal to another (indices into `petals`).
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
    pub step: usize,
}

enum Line {
    Center(NodeText),
    Petal(NodeText),
    Link(String, String, Option<String>),
}

/// Parse a `@flower` block. A line without a keyword is a petal; a link
/// to a name that is not a petal is left out; with two `center` lines the
/// last wins. Every line's `+` / `*` marker counts toward the steps.
pub fn parse(content: &str) -> Flower {
    let mut lines = Vec::new();
    let mut reveals = Vec::new();
    for raw in content.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (text, reveal) = parse_reveal_prefix(trimmed);
        if text.is_empty() {
            continue;
        }
        let line = if let Some(rest) = keyword(text, &["center", "centre"]) {
            Line::Center(NodeText::parse(rest))
        } else if let Some(rest) = keyword(text, &["petal"]) {
            Line::Petal(NodeText::parse(rest))
        } else if let Some((from, to, label)) = parse_link(text) {
            Line::Link(from, to, label)
        } else {
            Line::Petal(NodeText::parse(text))
        };
        lines.push(line);
        reveals.push(reveal);
    }
    let steps = assign_steps(&reveals);

    let mut flower = Flower {
        center: None,
        center_step: 0,
        petals: Vec::new(),
        links: Vec::new(),
    };
    let mut pending = Vec::new();
    for (line, step) in lines.into_iter().zip(steps) {
        match line {
            Line::Center(text) => {
                flower.center = Some(text);
                flower.center_step = step;
            }
            Line::Petal(text) => flower.petals.push(Petal { text, step }),
            Line::Link(from, to, label) => pending.push((from, to, label, step)),
        }
    }
    let find = |name: &str| {
        flower
            .petals
            .iter()
            .position(|p| p.text.name.eq_ignore_ascii_case(name))
    };
    let links: Vec<Link> = pending
        .into_iter()
        .filter_map(|(from, to, label, step)| {
            let (from, to) = (find(&from)?, find(&to)?);
            (from != to).then_some(Link {
                from,
                to,
                label,
                step,
            })
        })
        .collect();
    // a link shows once both its petals have
    flower.links = links
        .into_iter()
        .map(|l| Link {
            step: l
                .step
                .max(flower.petals[l.from].step)
                .max(flower.petals[l.to].step),
            ..l
        })
        .collect();
    flower
}

/// `text` without a leading `word ` (any of `words`, any case).
fn keyword<'a>(text: &'a str, words: &[&str]) -> Option<&'a str> {
    let (head, rest) = text.split_once(char::is_whitespace)?;
    words
        .iter()
        .any(|w| head.eq_ignore_ascii_case(w))
        .then_some(rest.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_petals_and_links() {
        let f = parse(
            "# a comment\n\
             - center Development Platform: Shared capabilities (icon: database)\n\
             - petal Payments: Takes the money\n\
             + petal Identity\n\
             + Data Team\n\
             * Mobile\n\
             - Payments -> Identity: uses\n\
             - Payments -> Nobody\n",
        );
        let center = f.center.unwrap();
        assert_eq!(center.name, "Development Platform");
        assert_eq!(center.icon.as_deref(), Some("database"));
        let names: Vec<&str> = f.petals.iter().map(|p| p.text.name.as_str()).collect();
        assert_eq!(names, ["Payments", "Identity", "Data Team", "Mobile"]);
        let steps: Vec<usize> = f.petals.iter().map(|p| p.step).collect();
        assert_eq!(steps, [0, 1, 2, 0], "`*` is static, like `-`");
        assert_eq!(f.petals[0].text.detail.as_deref(), Some("Takes the money"));
        assert_eq!(f.links.len(), 1, "a link to an unknown name is left out");
        assert_eq!(f.links[0].from, 0);
        assert_eq!(f.links[0].to, 1);
        assert_eq!(f.links[0].label.as_deref(), Some("uses"));
        assert_eq!(f.links[0].step, 1, "a link waits for its petals");
    }

    #[test]
    fn the_centre_spelling_and_a_flower_without_a_centre() {
        let f = parse("- centre Hub\n- A\n- B");
        assert_eq!(f.center.unwrap().name, "Hub");
        let f = parse("- A\n- B");
        assert!(f.center.is_none());
        assert_eq!(f.petals.len(), 2);
    }
}
