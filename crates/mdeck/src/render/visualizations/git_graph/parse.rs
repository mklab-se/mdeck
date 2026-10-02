//! The gitgraph source format: lanes, commits, branches, merges and tags.

use super::super::VizReveal;
use super::super::grammar::{Arrow, Problem, Source, relation, unquote};

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

const VERBS: &[&str] = &["lane", "commit", "branch", "merge", "tag"];

fn read(src: &Source) -> Vec<GitGraphItem> {
    src.check_settings(&[]);
    let mut items = Vec::new();
    for item in &src.items {
        item.check_attrs(src, &[]);
        let reveal = item.reveal;
        let Some((verb, rest)) = item.keyword(VERBS) else {
            src.problem(
                item.offset,
                format!(
                    "'{}' does not start with lane, commit, branch, merge or tag",
                    item.text
                ),
            );
            continue;
        };
        let link = || {
            let rel = relation(rest, &Arrow::FORWARD);
            if rel.is_none() {
                src.problem(
                    item.offset,
                    format!("{verb} needs 'source -> target', e.g. '{verb} main -> develop'"),
                );
            }
            rel
        };
        let named = |s: &str| {
            if s.is_empty() {
                src.problem(item.offset, format!("{verb} needs a branch name"));
            }
            !s.is_empty()
        };
        match verb {
            "lane" => {
                if named(rest) {
                    items.push(GitGraphItem::Lane {
                        name: rest.to_string(),
                    });
                }
            }
            "commit" => {
                let (branch, message) = match rest.split_once(": ") {
                    Some((b, m)) => (b.trim(), unquote(m)),
                    None => (rest, ""),
                };
                if named(branch) {
                    items.push(GitGraphItem::Commit {
                        branch: branch.to_string(),
                        message: message.to_string(),
                        reveal,
                    });
                }
            }
            "branch" => {
                if let Some(rel) = link() {
                    if rel.label.is_some() {
                        src.problem(item.offset, "a branch takes no label");
                    }
                    items.push(GitGraphItem::Branch {
                        source: rel.from.to_string(),
                        target: rel.to.to_string(),
                        reveal,
                    });
                }
            }
            "merge" => {
                if let Some(rel) = link() {
                    items.push(GitGraphItem::Merge {
                        source: rel.from.to_string(),
                        target: rel.to.to_string(),
                        label: rel.label.map(unquote).unwrap_or_default().to_string(),
                        reveal,
                    });
                }
            }
            _ => match rest.split_once(": ") {
                Some((branch, label)) if named(branch.trim()) => {
                    items.push(GitGraphItem::Tag {
                        branch: branch.trim().to_string(),
                        label: unquote(label).to_string(),
                        reveal,
                    });
                }
                Some(_) => {}
                None => src.problem(
                    item.offset,
                    "tag needs a branch and a label, e.g. 'tag main: \"v1.0\"'",
                ),
            },
        }
    }
    items
}

pub(super) fn parse_gitgraph(content: &str) -> Vec<GitGraphItem> {
    read(&Source::parse(content))
}

/// The problems in a `@gitgraph` block.
pub fn check(content: &str) -> Vec<Problem> {
    let src = Source::parse(content);
    read(&src);
    src.into_problems()
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
