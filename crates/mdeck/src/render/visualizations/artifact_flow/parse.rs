//! `@artifactflow` source: producers, services and consumers, the artifacts
//! that move between them, and optional column titles.

use super::super::assign_steps;
use super::super::grammar::{Arrow, Problem, Source};
use super::super::node_text::NodeText;

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
    Link(usize, String, String, Option<String>, Option<String>),
}

/// Parse an `@artifactflow` block. Without any `->` item, every producer
/// publishes to every service and every service feeds every consumer.
pub fn parse(content: &str) -> Flow {
    read(&Source::parse(content))
}

/// The problems in an `@artifactflow` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
}

fn read(src: &Source) -> Flow {
    src.check_settings(&["producers", "services", "consumers"]);
    let mut headings: [Option<Heading>; 3] =
        [Some(heading("Producers")), None, Some(heading("Consumers"))];
    for s in &src.settings {
        if let Some(col) = column_key(s.key) {
            headings[col] = match s.value {
                "" | "none" => None,
                v => Some(parse_heading(v)),
            };
        }
    }
    let mut lines = Vec::new();
    let mut reveals = Vec::new();
    for item in &src.items {
        if item.text.is_empty() {
            src.problem(item.offset, "an empty item");
            continue;
        }
        let line = if item.indent > 0 {
            item.check_attrs(src, &[]);
            Line::Item(item.text.to_string())
        } else if let Some((role, rest)) = item.keyword(&["producer", "service", "consumer"]) {
            item.check_attrs(src, &["icon"]);
            let role = match role {
                "producer" => Role::Producer,
                "service" => Role::Service,
                _ => Role::Consumer,
            };
            Line::Node(role, NodeText::from_item(rest, item))
        } else if let Some(rel) = item.relation(&Arrow::FORWARD) {
            item.check_attrs(src, &["icon"]);
            Line::Link(
                item.offset,
                rel.from.to_string(),
                rel.to.to_string(),
                rel.label.map(str::to_string),
                item.attr("icon").map(str::to_ascii_lowercase),
            )
        } else {
            src.problem(
                item.offset,
                format!(
                    "'{}' is not a producer, service, consumer or 'A -> B' artifact",
                    item.text
                ),
            );
            continue;
        };
        lines.push(line);
        reveals.push(item.reveal);
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
            Line::Link(offset, from, to, label, icon) => {
                links.push((offset, from, to, label, icon, step));
            }
        }
    }

    let find = |name: &str| {
        nodes
            .iter()
            .position(|n| n.text.name.eq_ignore_ascii_case(name))
    };
    let mut edges: Vec<Edge> = links
        .into_iter()
        .filter_map(|(offset, from, to, label, icon, step)| {
            for end in [&from, &to] {
                if find(end).is_none() {
                    src.problem(
                        offset,
                        format!("'{end}' is not a producer, service or consumer"),
                    );
                }
            }
            let (from, to) = (find(&from)?, find(&to)?);
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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
producers: Producing Teams | Build and publish artifacts
services: none
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
        assert_eq!(f.edges[1].step, 2, "* joins the previous step");
        assert_eq!(f.edges[1].label, None);
        assert_eq!(f.edges[2].step, 1, "waits for Product Team");
    }

    #[test]
    fn unknown_ends_settings_and_stray_lines_are_reported() {
        let lines: Vec<usize> = check(SAMPLE).iter().map(|p| p.offset).collect();
        assert_eq!(lines, [12]);
        let p = check("producer: A\n- producer B (shade: 1)\n- B to C\n");
        let lines: Vec<usize> = p.iter().map(|p| p.offset).collect();
        assert_eq!(lines, [0, 1, 2], "{p:?}");
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
