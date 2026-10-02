//! The text of a node in a node-and-link visualization (`@flower`,
//! `@artifactflow`): `Name: what it does (icon: team)`.

use super::grammar::Item;

/// A node's name, its optional description, and its optional icon.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NodeText {
    pub name: String,
    pub detail: Option<String>,
    /// `Some("none")` hides a default icon.
    pub icon: Option<String>,
}

impl NodeText {
    /// `text` (`Name: detail`) with the `icon` attribute of `item`.
    pub fn from_item(text: &str, item: &Item) -> Self {
        let (name, detail) = match text.split_once(':') {
            Some((n, d)) => (n.trim(), Some(d.trim()).filter(|d| !d.is_empty())),
            None => (text.trim(), None),
        };
        NodeText {
            name: name.to_string(),
            detail: detail.map(str::to_string),
            icon: item
                .attr("icon")
                .map(str::to_ascii_lowercase)
                .filter(|i| !i.is_empty()),
        }
    }

    /// The icon to draw: the node's own, else `default`; none for `none`.
    pub fn icon_or<'a>(&'a self, default: Option<&'a str>) -> Option<&'a str> {
        match self.icon.as_deref() {
            Some("none") => None,
            Some(i) => Some(i),
            None => default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::grammar::Source;
    use super::*;

    fn node(line: &str) -> NodeText {
        let src = Source::parse(line);
        NodeText::from_item(src.items[0].text, &src.items[0])
    }

    #[test]
    fn parses_name_detail_and_icon() {
        assert_eq!(
            node("- Team 1: Builds features (icon: Team)"),
            NodeText {
                name: "Team 1".into(),
                detail: Some("Builds features".into()),
                icon: Some("team".into()),
            }
        );
        assert_eq!(
            node("- Platform"),
            NodeText {
                name: "Platform".into(),
                ..Default::default()
            }
        );
        let n = node("- Data (icon: none)");
        assert_eq!(n.name, "Data");
        assert_eq!(n.icon_or(Some("team")), None);
        assert_eq!(node("- Ops:").detail, None);
        assert_eq!(node("- Ops").icon_or(Some("team")), Some("team"));
    }
}
