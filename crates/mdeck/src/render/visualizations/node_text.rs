//! The text of a node in a node-and-link visualization (`@flower`,
//! `@artifactflow`): `Name: what it does (icon: team)`.

/// A node's name, its optional description, and its optional icon.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NodeText {
    pub name: String,
    pub detail: Option<String>,
    /// `Some("none")` hides a default icon.
    pub icon: Option<String>,
}

impl NodeText {
    /// Parse `Name: detail (icon: x)`; the detail and the icon are optional.
    pub fn parse(text: &str) -> Self {
        let (text, icon) = take_icon(text.trim());
        let (name, detail) = match text.split_once(':') {
            Some((n, d)) => (n.trim(), Some(d.trim()).filter(|d| !d.is_empty())),
            None => (text.trim(), None),
        };
        NodeText {
            name: name.to_string(),
            detail: detail.map(str::to_string),
            icon,
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

/// Split a trailing `(icon: name)` off `text`.
pub fn take_icon(text: &str) -> (&str, Option<String>) {
    let t = text.trim_end();
    if let Some(open) = t.rfind("(icon:")
        && t.ends_with(')')
    {
        let name = t[open + 6..t.len() - 1].trim().to_ascii_lowercase();
        if !name.is_empty() {
            return (t[..open].trim_end(), Some(name));
        }
    }
    (t, None)
}

/// Split `A -> B: label` into its ends and optional label; `None` when the
/// line is not a link.
pub fn parse_link(text: &str) -> Option<(String, String, Option<String>)> {
    let (from, rest) = text.split_once("->")?;
    let (to, label) = match rest.split_once(':') {
        Some((t, l)) => (t, Some(l.trim()).filter(|l| !l.is_empty())),
        None => (rest, None),
    };
    let (from, to) = (from.trim(), to.trim());
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some((from.to_string(), to.to_string(), label.map(str::to_string)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_detail_and_icon() {
        assert_eq!(
            NodeText::parse("Team 1: Builds features (icon: Team)"),
            NodeText {
                name: "Team 1".into(),
                detail: Some("Builds features".into()),
                icon: Some("team".into()),
            }
        );
        assert_eq!(
            NodeText::parse("Platform"),
            NodeText {
                name: "Platform".into(),
                ..Default::default()
            }
        );
        let n = NodeText::parse("Data (icon: none)");
        assert_eq!(n.name, "Data");
        assert_eq!(n.icon_or(Some("team")), None);
        assert_eq!(NodeText::parse("Ops:").detail, None);
        assert_eq!(NodeText::parse("Ops").icon_or(Some("team")), Some("team"));
    }

    #[test]
    fn links_split_on_the_arrow_and_the_first_colon() {
        assert_eq!(
            parse_link("Build Team -> Artifactory: Image: v1.2 (icon: package)"),
            Some((
                "Build Team".into(),
                "Artifactory".into(),
                Some("Image: v1.2 (icon: package)".into())
            ))
        );
        assert_eq!(parse_link("A -> B"), Some(("A".into(), "B".into(), None)));
        assert_eq!(parse_link("A -> "), None);
        assert_eq!(parse_link("no link here"), None);
    }
}
