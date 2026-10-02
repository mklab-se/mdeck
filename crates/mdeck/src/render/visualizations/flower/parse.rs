//! `@flower` source: a `center`, `petal`s around it, and `A -> B` links
//! between petals.

use super::super::assign_steps;
use super::super::grammar::{Arrow, Problem, Source};
use super::super::node_text::NodeText;

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
    Link(usize, String, String, Option<String>),
}

/// Parse a `@flower` block. An item without a keyword is a petal; with two
/// `center` items the last wins. Every item's `+` / `*` marker counts toward
/// the steps.
pub fn parse(content: &str) -> Flower {
    read(&Source::parse(content))
}

/// The problems in a `@flower` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
}

fn read(src: &Source) -> Flower {
    src.check_settings(&[]);
    let mut lines = Vec::new();
    let mut reveals = Vec::new();
    for item in &src.items {
        if item.text.is_empty() {
            src.problem(item.offset, "an empty item");
            continue;
        }
        let keyword = item.keyword(&["center", "petal"]);
        let link = keyword
            .is_none()
            .then(|| item.relation(&Arrow::FORWARD))
            .flatten();
        let line = if let Some(rel) = link {
            item.check_attrs(src, &[]);
            Line::Link(
                item.offset,
                rel.from.to_string(),
                rel.to.to_string(),
                rel.label.map(str::to_string),
            )
        } else {
            item.check_attrs(src, &["icon"]);
            if item.keyword(&["centre"]).is_some() {
                src.problem(item.offset, "write 'center', not 'centre'");
            }
            let text = NodeText::from_item(keyword.map_or(item.text, |(_, rest)| rest), item);
            match keyword {
                Some(("center", _)) => Line::Center(text),
                _ => Line::Petal(text),
            }
        };
        lines.push(line);
        reveals.push(item.reveal);
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
            Line::Link(offset, from, to, label) => pending.push((offset, from, to, label, step)),
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
        .filter_map(|(offset, from, to, label, step)| {
            for end in [&from, &to] {
                if find(end).is_none() {
                    src.problem(offset, format!("'{end}' is not a petal"));
                }
            }
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
        assert_eq!(steps, [0, 1, 2, 2]);
        assert_eq!(f.petals[0].text.detail.as_deref(), Some("Takes the money"));
        assert_eq!(f.links.len(), 1, "a link to an unknown name is left out");
        assert_eq!(f.links[0].from, 0);
        assert_eq!(f.links[0].to, 1);
        assert_eq!(f.links[0].label.as_deref(), Some("uses"));
        assert_eq!(f.links[0].step, 1, "a link waits for its petals");
    }

    #[test]
    fn unknown_link_ends_and_attributes_are_reported() {
        let p = check("- A (colour: red)\n- B\n- A -> Nobody\n");
        let lines: Vec<usize> = p.iter().map(|p| p.offset).collect();
        assert_eq!(lines, [0, 2], "{p:?}");
    }

    #[test]
    fn the_centre_spelling_is_reported_and_a_flower_without_a_centre() {
        let f = parse("- centre Hub\n- A\n- B");
        assert!(f.center.is_none());
        assert_eq!(check("- centre Hub\n- A\n- B")[0].offset, 0);
        let f = parse("- A\n- B");
        assert!(f.center.is_none());
        assert_eq!(f.petals.len(), 2);
    }
}
