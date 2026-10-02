//! `@artifactflow` source: producers, services and consumers, the artifacts
//! that move between them, and optional column titles.

use super::super::node_text::{NodeText, parse_link, take_icon};
use super::super::{assign_steps, header_directive, parse_reveal_prefix};

/// The three columns, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Producer,
    Service,
    Consumer,
}

impl Role {
    pub const ALL: [Role; 3] = [Role::Producer, Role::Service, Role::Consumer];

    pub fn column(self) -> usize {
        match self {
            Role::Producer => 0,
            Role::Service => 1,
            Role::Consumer => 2,
        }
    }

    /// The icon a node of this role shows when it names none.
    pub fn default_icon(self) -> &'static str {
        match self {
            Role::Service => "database",
            _ => "team",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub role: Role,
    pub text: NodeText,
    /// Indented `- item` lines under the node.
    pub items: Vec<String>,
    pub step: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub step: usize,
}

/// A column's title and subtitle (`# producers: Title | subtitle`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Heading {
    pub title: String,
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Per column; producers and consumers default to "Producers" and
    /// "Consumers", services to none.
    pub headings: [Option<Heading>; 3],
}

impl Flow {
    /// The nodes in `role`'s column, in source order, with their indices.
    pub fn column(&self, role: Role) -> impl Iterator<Item = (usize, &Node)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(move |(_, n)| n.role == role)
    }
}

enum Line {
    Node(Role, NodeText),
    Item(String),
    Link(String, String, Option<String>),
}

/// Parse an `@artifactflow` block. Without any `->` line, every producer
/// publishes to every service and every service feeds every consumer. A link
/// to an unknown name is left out; a line without a role keyword that is
/// not a link is ignored.
pub fn parse(content: &str) -> Flow {
    let mut headings: [Option<Heading>; 3] =
        [Some(heading("Producers")), None, Some(heading("Consumers"))];
    let mut lines = Vec::new();
    let mut reveals = Vec::new();
    for raw in content.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            if let Some((key, value)) = header_directive(trimmed)
                && let Some(col) = column_key(key)
            {
                headings[col] = match value {
                    "" | "none" => None,
                    v => Some(parse_heading(v)),
                };
            }
            continue;
        }
        let indented = raw.starts_with([' ', '\t']);
        let (text, reveal) = parse_reveal_prefix(trimmed);
        if text.is_empty() {
            continue;
        }
        let line = if indented {
            Line::Item(text.to_string())
        } else if let Some((role, rest)) = role_line(text) {
            Line::Node(role, NodeText::parse(rest))
        } else if let Some((from, to, label)) = parse_link(text) {
            Line::Link(from, to, label)
        } else {
            continue;
        };
        lines.push(line);
        reveals.push(reveal);
    }
    let steps = assign_steps(&reveals);

    let mut nodes: Vec<Node> = Vec::new();
    let mut links = Vec::new();
    for (line, step) in lines.into_iter().zip(steps) {
        match line {
            Line::Node(role, text) => nodes.push(Node {
                role,
                text,
                items: Vec::new(),
                step,
            }),
            Line::Item(item) => {
                if let Some(last) = nodes.last_mut() {
                    last.items.push(item);
                }
            }
            Line::Link(from, to, label) => links.push((from, to, label, step)),
        }
    }

    let find = |name: &str| {
        nodes
            .iter()
            .position(|n| n.text.name.eq_ignore_ascii_case(name))
    };
    let mut edges: Vec<Edge> = links
        .into_iter()
        .filter_map(|(from, to, label, step)| {
            let (from, to) = (find(&from)?, find(&to)?);
            let (label, icon) = match label.as_deref().map(take_icon) {
                Some((text, icon)) => (Some(text.to_string()).filter(|t| !t.is_empty()), icon),
                None => (None, None),
            };
            (from != to).then_some(Edge {
                from,
                to,
                label,
                icon,
                step,
            })
        })
        .collect();
    if edges.is_empty() {
        edges = default_edges(&nodes);
    }
    // an edge shows once both its ends have
    for e in &mut edges {
        e.step = e.step.max(nodes[e.from].step).max(nodes[e.to].step);
    }
    Flow {
        nodes,
        edges,
        headings,
    }
}

/// Producers into every service, services out to every consumer; producers
/// straight to consumers when there is no service.
fn default_edges(nodes: &[Node]) -> Vec<Edge> {
    let of = |role: Role| -> Vec<usize> {
        nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.role == role)
            .map(|(i, _)| i)
            .collect()
    };
    let (producers, services, consumers) =
        (of(Role::Producer), of(Role::Service), of(Role::Consumer));
    let pairs = |a: &[usize], b: &[usize]| -> Vec<(usize, usize)> {
        a.iter()
            .flat_map(|&x| b.iter().map(move |&y| (x, y)))
            .collect()
    };
    let all = if services.is_empty() {
        pairs(&producers, &consumers)
    } else {
        let mut v = pairs(&producers, &services);
        v.extend(pairs(&services, &consumers));
        v
    };
    all.into_iter()
        .map(|(from, to)| Edge {
            from,
            to,
            label: None,
            icon: None,
            step: 0,
        })
        .collect()
}

fn heading(title: &str) -> Heading {
    Heading {
        title: title.to_string(),
        subtitle: None,
    }
}

/// `Title | subtitle`.
fn parse_heading(value: &str) -> Heading {
    match value.split_once('|') {
        Some((t, s)) => Heading {
            title: t.trim().to_string(),
            subtitle: Some(s.trim().to_string()).filter(|s| !s.is_empty()),
        },
        None => heading(value.trim()),
    }
}

fn column_key(key: &str) -> Option<usize> {
    match key.to_ascii_lowercase().as_str() {
        "producers" => Some(0),
        "services" => Some(1),
        "consumers" => Some(2),
        _ => None,
    }
}

/// `producer Build Team: ...` → the role and the rest.
fn role_line(text: &str) -> Option<(Role, &str)> {
    let (head, rest) = text.split_once(char::is_whitespace)?;
    let role = match head.to_ascii_lowercase().as_str() {
        "producer" => Role::Producer,
        "service" => Role::Service,
        "consumer" => Role::Consumer,
        _ => return None,
    };
    Some((role, rest.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# producers: Producing Teams | Build and publish artifacts
# services: none
- producer Build Team: Produces binaries and images
- producer Platform Team (icon: server)
- service Artifactory: Artifact repository
  - Container images
  - Libraries
- consumer Integration Team
+ consumer Product Team: Pulls approved artifacts
+ Build Team -> Artifactory: Container image v1.2.3 (icon: package)
* Platform Team -> Artifactory
- Artifactory -> Product Team: Pull package
- Artifactory -> Nobody: lost
";

    #[test]
    fn nodes_items_and_headings() {
        let f = parse(SAMPLE);
        let names: Vec<&str> = f.nodes.iter().map(|n| n.text.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Build Team",
                "Platform Team",
                "Artifactory",
                "Integration Team",
                "Product Team"
            ]
        );
        assert_eq!(f.nodes[1].text.icon.as_deref(), Some("server"));
        assert_eq!(f.nodes[2].role, Role::Service);
        assert_eq!(f.nodes[2].items, ["Container images", "Libraries"]);
        assert_eq!(f.nodes[4].step, 1);
        let producers = f.headings[0].as_ref().unwrap();
        assert_eq!(producers.title, "Producing Teams");
        assert_eq!(
            producers.subtitle.as_deref(),
            Some("Build and publish artifacts")
        );
        assert!(f.headings[1].is_none());
        assert_eq!(f.headings[2].as_ref().unwrap().title, "Consumers");
        assert_eq!(f.column(Role::Producer).count(), 2);
    }

    #[test]
    fn edges_carry_label_icon_and_step() {
        let f = parse(SAMPLE);
        assert_eq!(f.edges.len(), 3, "the link to an unknown name is left out");
        let e = &f.edges[0];
        assert_eq!((e.from, e.to), (0, 2));
        assert_eq!(e.label.as_deref(), Some("Container image v1.2.3"));
        assert_eq!(e.icon.as_deref(), Some("package"));
        assert_eq!(e.step, 2);
        assert_eq!(f.edges[1].step, 0, "`*` is static, like `-`");
        assert_eq!(f.edges[1].label, None);
        assert_eq!(f.edges[2].step, 1, "waits for Product Team");
    }

    #[test]
    fn without_links_everything_flows_through_the_services() {
        let f = parse("- producer A\n- producer B\n- service S\n- consumer C\n");
        let pairs: Vec<(usize, usize)> = f.edges.iter().map(|e| (e.from, e.to)).collect();
        assert_eq!(pairs, [(0, 2), (1, 2), (2, 3)]);
        let f = parse("- producer A\n- consumer C\n- consumer D\n");
        let pairs: Vec<(usize, usize)> = f.edges.iter().map(|e| (e.from, e.to)).collect();
        assert_eq!(pairs, [(0, 1), (0, 2)]);
    }
}
