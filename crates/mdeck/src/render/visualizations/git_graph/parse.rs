//! The gitgraph source format: lanes, commits, branches, merges and tags.

use super::super::{VizReveal, parse_reveal_prefix};

// ─── Data model ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(super) enum GitGraphItem {
    /// Declare a branch lane (rendered as a dotted background line).
    Lane { name: String },
    /// A commit on a branch (dot on the lane).
    Commit {
        branch: String,
        message: String,
        reveal: VizReveal,
    },
    /// Fork: source branch creates target branch (S-curve, target becomes active).
    Branch {
        source: String,
        target: String,
        reveal: VizReveal,
    },
    /// Merge: source branch merges into target (S-curve, source becomes inactive).
    Merge {
        source: String,
        target: String,
        label: String,
        reveal: VizReveal,
    },
    /// Tag on a branch's latest commit.
    Tag {
        branch: String,
        label: String,
        reveal: VizReveal,
    },
}

// ─── Parsing ────────────────────────────────────────────────────────────────

pub(super) fn parse_gitgraph(content: &str) -> Vec<GitGraphItem> {
    let mut items = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (text, reveal) = parse_reveal_prefix(trimmed);
        if text.is_empty() {
            continue;
        }

        let lower = text.to_lowercase();

        if lower.starts_with("lane ") {
            let name = text["lane ".len()..].trim().to_string();
            items.push(GitGraphItem::Lane { name });
        } else if lower.starts_with("branch ") {
            let rest = &text["branch ".len()..];
            if let Some(arrow) = rest.find(" -> ") {
                let source = rest[..arrow].trim().to_string();
                let target = rest[arrow + " -> ".len()..].trim().to_string();
                items.push(GitGraphItem::Branch {
                    source,
                    target,
                    reveal,
                });
            }
        } else if lower.starts_with("commit ") {
            let rest = &text["commit ".len()..];
            let (branch, message) = if let Some(colon) = rest.find(": ") {
                (
                    rest[..colon].trim().to_string(),
                    rest[colon + 2..].trim().trim_matches('"').to_string(),
                )
            } else {
                (rest.trim().to_string(), String::new())
            };
            items.push(GitGraphItem::Commit {
                branch,
                message,
                reveal,
            });
        } else if lower.starts_with("merge ") {
            let rest = &text["merge ".len()..];
            if let Some(arrow) = rest.find(" -> ") {
                let source = rest[..arrow].trim().to_string();
                let after_arrow = &rest[arrow + " -> ".len()..];
                let (target, label) = if let Some(colon) = after_arrow.find(": ") {
                    (
                        after_arrow[..colon].trim().to_string(),
                        after_arrow[colon + 2..]
                            .trim()
                            .trim_matches('"')
                            .to_string(),
                    )
                } else {
                    (after_arrow.trim().to_string(), String::new())
                };
                items.push(GitGraphItem::Merge {
                    source,
                    target,
                    label,
                    reveal,
                });
            }
        } else if lower.starts_with("tag ") {
            let rest = &text["tag ".len()..];
            if let Some(colon) = rest.find(": ") {
                let branch = rest[..colon].trim().to_string();
                let label = rest[colon + 2..].trim().trim_matches('"').to_string();
                items.push(GitGraphItem::Tag {
                    branch,
                    label,
                    reveal,
                });
            }
        }
    }
    items
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lane() {
        let content = "- lane main\n- lane develop";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[0], GitGraphItem::Lane { name } if name == "main"));
        assert!(matches!(&items[1], GitGraphItem::Lane { name } if name == "develop"));
    }

    #[test]
    fn test_parse_branch_arrow_syntax() {
        let content = "- branch main -> develop";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 1);
        match &items[0] {
            GitGraphItem::Branch { source, target, .. } => {
                assert_eq!(source, "main");
                assert_eq!(target, "develop");
            }
            _ => panic!("Expected Branch"),
        }
    }

    #[test]
    fn test_parse_commit_with_message() {
        let content = "- commit develop: \"Initial setup\"";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 1);
        match &items[0] {
            GitGraphItem::Commit {
                branch, message, ..
            } => {
                assert_eq!(branch, "develop");
                assert_eq!(message, "Initial setup");
            }
            _ => panic!("Expected Commit"),
        }
    }

    #[test]
    fn test_parse_commit_without_message() {
        let content = "- commit feature";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 1);
        match &items[0] {
            GitGraphItem::Commit {
                branch, message, ..
            } => {
                assert_eq!(branch, "feature");
                assert!(message.is_empty());
            }
            _ => panic!("Expected Commit"),
        }
    }

    #[test]
    fn test_parse_merge_with_label() {
        let content = "- merge feature -> develop: \"PR #42\"";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 1);
        match &items[0] {
            GitGraphItem::Merge {
                source,
                target,
                label,
                ..
            } => {
                assert_eq!(source, "feature");
                assert_eq!(target, "develop");
                assert_eq!(label, "PR #42");
            }
            _ => panic!("Expected Merge"),
        }
    }

    #[test]
    fn test_parse_tag() {
        let content = "- tag main: \"v1.0\"";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 1);
        match &items[0] {
            GitGraphItem::Tag { branch, label, .. } => {
                assert_eq!(branch, "main");
                assert_eq!(label, "v1.0");
            }
            _ => panic!("Expected Tag"),
        }
    }

    #[test]
    fn test_parse_reveal_markers() {
        let content = "- lane main\n- commit main\n+ branch main -> develop\n* commit develop";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 4);
        // Lane is always static (override)
        match &items[2] {
            GitGraphItem::Branch { reveal, .. } => assert_eq!(*reveal, VizReveal::NextStep),
            _ => panic!(),
        }
        match &items[3] {
            GitGraphItem::Commit { reveal, .. } => assert_eq!(*reveal, VizReveal::Static),
            _ => panic!(),
        }
    }

    #[test]
    fn test_parse_full_gitflow() {
        let content = "\
- lane main
- lane hotfix
- lane release
- lane develop
- lane feature
- commit main
- branch main -> develop
+ branch develop -> feature
+ commit feature
+ commit feature
+ merge feature -> develop
+ branch develop -> release
+ commit release
+ merge release -> main: \"v1.0\"
* merge release -> develop
+ tag main: \"v1.0\"";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 16);
    }

    #[test]
    fn test_parse_ignores_comments_and_blanks() {
        let content = "# Comment\n\n- lane main\n# Another\n- commit main";
        let items = parse_gitgraph(content);
        assert_eq!(items.len(), 2);
    }
}
