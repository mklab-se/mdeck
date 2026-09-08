use std::path::PathBuf;

use crate::check::{CheckCategory, CheckReport, CheckWarning};
use crate::parser;
use crate::render;

pub fn run(file: PathBuf, verbose: u8, quiet: bool) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(&file)?;
    let base_path = file.parent().unwrap_or(std::path::Path::new("."));
    let presentation = parser::parse(&content, base_path);

    if presentation.slides.is_empty() {
        anyhow::bail!("No slides found in {}", file.display());
    }

    let slide_count = presentation.slides.len();
    let file_name = file
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    if !quiet {
        eprintln!(
            "Checking {} ({} slide{})...",
            file_name,
            slide_count,
            if slide_count == 1 { "" } else { "s" }
        );
    }

    // -v: per-slide overview (layout, block count, reveal steps, title)
    if verbose > 0 && !quiet {
        for (i, slide) in presentation.slides.iter().enumerate() {
            eprintln!("{}", slide_summary(i, slide));
        }
        eprintln!();
    }

    let mut report = CheckReport::new();

    for (i, slide) in presentation.slides.iter().enumerate() {
        let slide_num = i + 1;
        for block in &slide.blocks {
            if let parser::Block::Diagram { content } = block {
                for warning_msg in render::diagram::check_diagram_routes(content) {
                    report.add(CheckWarning {
                        slide: slide_num,
                        category: CheckCategory::DiagramRouting,
                        message: warning_msg,
                    });
                }
            }
        }
    }

    // Ember stories: broken inline scripts and sidecar entries that no longer
    // match their slide.
    let sidecar = match render::story::sidecar::load(&file) {
        Ok(sc) => sc,
        Err(e) => {
            report.add(CheckWarning {
                slide: 1,
                category: CheckCategory::Story,
                message: format!("story sidecar could not be read: {e}"),
            });
            None
        }
    };
    let (stories, problems) = render::story::sidecar::resolve(&presentation, sidecar.as_ref());
    for p in problems {
        // "slide N: ..." messages carry their own slide number
        let slide = p
            .strip_prefix("slide ")
            .and_then(|r| r.split(':').next())
            .and_then(|n| n.trim().parse().ok())
            .unwrap_or(1);
        report.add(CheckWarning {
            slide,
            category: CheckCategory::Story,
            message: p,
        });
    }
    for (i, r) in stories.iter().enumerate() {
        if let Some(r) = r
            && r.source == render::story::sidecar::Source::Stale
        {
            report.add(CheckWarning {
                slide: i + 1,
                category: CheckCategory::Story,
                message: "story is stale (slide changed since it was written); run `mdeck ai story --stale`".into(),
            });
        }
    }

    if report.has_warnings() {
        if !quiet {
            report.print_detailed();
        }
        std::process::exit(1);
    } else {
        if !quiet {
            eprintln!("No issues found.");
        }
        Ok(())
    }
}

/// One-line description of a slide for `--check -v`.
fn slide_summary(index: usize, slide: &parser::Slide) -> String {
    let steps = parser::compute_max_steps(&slide.blocks);
    let title = slide
        .blocks
        .iter()
        .find_map(|b| match b {
            parser::Block::Heading { inlines, .. } => Some(parser::inlines_to_text(inlines)),
            _ => None,
        })
        .map(|t| crate::commands::util::truncate_chars(t.trim(), 48))
        .filter(|t| !t.is_empty());
    let mut line = format!(
        "  slide {:>3}: {:<11} {:>2} block{}, {} step{}",
        index + 1,
        format!("{:?}", slide.layout).to_lowercase(),
        slide.blocks.len(),
        if slide.blocks.len() == 1 { "" } else { "s" },
        steps,
        if steps == 1 { "" } else { "s" },
    );
    if let Some(title) = title {
        line.push_str(&format!("  \"{title}\""));
    }
    if slide.notes.is_some() {
        line.push_str("  [notes]");
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Block, Inline, Layout, ListItem, ListMarker, Slide};

    fn slide(blocks: Vec<Block>, layout: Layout, notes: Option<&str>) -> Slide {
        Slide {
            directives: vec![],
            blocks,
            layout,
            raw_source: String::new(),
            notes: notes.map(String::from),
            story_hint: None,
            scene_script: None,
        }
    }

    #[test]
    fn summary_includes_layout_blocks_steps_and_title() {
        let blocks = vec![
            Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Räksmörgås & friends".into())],
            },
            Block::List {
                ordered: false,
                items: vec![
                    ListItem {
                        marker: ListMarker::NextStep,
                        inlines: vec![],
                        children: vec![],
                    },
                    ListItem {
                        marker: ListMarker::NextStep,
                        inlines: vec![],
                        children: vec![],
                    },
                ],
            },
        ];
        let s = slide(blocks, Layout::Bullet, Some("remember to smile"));
        let line = slide_summary(4, &s);
        assert!(line.contains("slide   5:"), "{line}");
        assert!(line.contains("bullet"), "{line}");
        assert!(line.contains("2 blocks"), "{line}");
        assert!(line.contains("2 steps"), "{line}");
        assert!(line.contains("\"Räksmörgås & friends\""), "{line}");
        assert!(line.ends_with("[notes]"), "{line}");
    }

    #[test]
    fn summary_without_heading_or_notes() {
        let s = slide(
            vec![Block::Paragraph { inlines: vec![] }],
            Layout::Content,
            None,
        );
        let line = slide_summary(0, &s);
        assert!(line.contains("slide   1:"), "{line}");
        assert!(line.contains("1 block,"), "{line}");
        assert!(line.contains("0 steps"), "{line}");
        assert!(!line.contains('"'), "{line}");
        assert!(!line.contains("[notes]"), "{line}");
    }
}
