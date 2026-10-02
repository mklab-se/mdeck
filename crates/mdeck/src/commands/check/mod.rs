//! `mdeck --check`: everything in a deck that will not come out the way its
//! author meant, per slide and category.

use std::path::{Path, PathBuf};

use crate::check::{CheckCategory, CheckReport, CheckWarning};
use crate::parser;
use crate::render;

mod assets;
mod background;
mod content;
mod engine;
mod illustration;
mod requires;
mod settings;
mod theme;
mod thermal;
pub use assets::asset_warnings;
mod visuals;
pub use background::background_warnings;
pub use content::{cjk_font_warning, content_warnings, math_warnings, warn_missing_cjk_font};
pub use engine::{deck_theme, engine_warnings};
pub use illustration::illustration_warnings;
pub use settings::{fence_warnings, settings_warnings};
pub use theme::theme_warnings;
pub use thermal::thermal_warnings;

pub fn run(file: PathBuf, verbose: u8, quiet: bool, engine: Option<String>) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(&file)?;
    let base_path = file.parent().unwrap_or(Path::new("."));
    let presentation = parser::parse(&content);

    if presentation.slides.is_empty() {
        anyhow::bail!("No slides found in {}", file.display());
    }

    if !quiet {
        let slide_count = presentation.slides.len();
        eprintln!(
            "Checking {} ({} slide{})...",
            file.file_name().unwrap_or_default().to_string_lossy(),
            slide_count,
            if slide_count == 1 { "" } else { "s" }
        );
    }

    // -v: per-slide overview (layout, block count, reveal steps, title)
    // and the settings that apply to each slide
    if verbose > 0 && !quiet {
        for (i, slide) in presentation.slides.iter().enumerate() {
            eprintln!("{}", slide_summary(i, slide));
            if let Some(settings) = applied_settings(&presentation.meta, slide) {
                eprintln!("{settings}");
            }
        }
        eprintln!();
    }

    let report = collect(&file, &content, &presentation, base_path, engine.as_deref())?;

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

/// Every check, in report order.
fn collect(
    file: &Path,
    content: &str,
    presentation: &parser::Presentation,
    base_path: &Path,
    engine: Option<&str>,
) -> anyhow::Result<CheckReport> {
    let mut report = CheckReport::new();
    let mut add = |warnings: Vec<CheckWarning>| {
        for w in warnings {
            report.add(w);
        }
    };

    add(diagram_warnings(presentation));
    add(illustration_warnings(presentation, file));
    add(
        cjk_font_warning(presentation, render::fonts::cjk_coverage())
            .into_iter()
            .collect(),
    );
    add(settings_warnings(presentation));
    add(fence_warnings(presentation));
    add(content_warnings(presentation));
    add(math_warnings(presentation));

    let defaults = crate::config::Config::load_or_default()
        .defaults
        .unwrap_or_default();
    add(theme_warnings(
        presentation,
        defaults.theme.as_deref(),
        base_path,
    ));
    add(background_warnings(presentation, base_path, content));
    add(thermal_warnings(presentation, base_path));
    add(visuals::visual_warnings(presentation));
    add(requires::requires_warnings(presentation, base_path));
    let (theme, problems) = deck_theme(presentation, defaults.theme.as_deref(), base_path, engine)?;
    add(problems
        .into_iter()
        .map(|message| CheckWarning {
            slide: 0,
            line: 0,
            category: CheckCategory::Engine,
            message,
        })
        .collect());
    add(engine_warnings(presentation, theme.engine));
    add(asset_warnings(file, presentation, &theme));
    Ok(report)
}

/// Routes the diagram layout could not draw cleanly.
fn diagram_warnings(presentation: &parser::Presentation) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        let mut fences = fence_lines(slide, "@architecture").into_iter();
        for block in &slide.blocks {
            if let parser::Block::Diagram { content, .. } = block {
                let line = fences.next().unwrap_or(slide.line);
                for message in render::diagram::check_diagram_routes(content) {
                    out.push(CheckWarning {
                        slide: i + 1,
                        line,
                        category: CheckCategory::DiagramRouting,
                        message,
                    });
                }
            }
        }
    }
    out
}

/// The deck file line of each code fence in a slide whose info string starts
/// with `tag`, in order (so the nth is the nth such block).
fn fence_lines(slide: &parser::Slide, tag: &str) -> Vec<usize> {
    let mut fences = parser::splitter::FenceTracker::new();
    let mut out = Vec::new();
    for (offset, line) in slide.raw_source.lines().enumerate() {
        let opening = !fences.is_open();
        if fences.observe(line) && opening {
            let info = line.trim().trim_start_matches(['`', '~']).trim_start();
            if info.starts_with(tag) {
                out.push(slide.line_at(offset));
            }
        }
    }
    out
}

/// The settings that apply to a slide, for `--check -v`: its own, then
/// the deck's values of settings a slide can override that it does not.
/// `None` when nothing applies.
fn applied_settings(meta: &parser::PresentationMeta, slide: &parser::Slide) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for s in &slide.settings {
        if crate::language::setting(&s.name).is_some_and(|d| d.scope.in_slide())
            && parser::setting(&slide.settings, &s.name) == Some(s.value.trim())
            && !parts.iter().any(|p| p.starts_with(&format!("{}:", s.name)))
        {
            parts.push(format!("{}: {}", s.name, s.value.trim()));
        }
    }
    for s in &meta.settings {
        let shared = crate::language::setting(&s.name)
            .is_some_and(|d| d.scope == crate::language::Scope::Both);
        if shared && parser::setting(&slide.settings, &s.name).is_none() {
            parts.push(format!("{}: {} (deck)", s.name, s.value.trim()));
        }
    }
    (!parts.is_empty()).then(|| format!("               {}", parts.join(", ")))
}

/// One-line description of a slide for `--check -v`.
fn slide_summary(index: usize, slide: &parser::Slide) -> String {
    let steps = slide.steps;
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

    use crate::parser::{Block, Layout, Slide};

    fn slide(blocks: Vec<Block>, layout: Layout, notes: Option<&str>) -> Slide {
        Slide {
            blocks,
            layout,
            raw_source: String::new(),
            notes: notes.map(String::from),
            ..Default::default()
        }
    }

    #[test]
    fn verbose_lists_the_settings_that_apply() {
        let p = parser::parse(
            "---\ntransition: fade\nlogo: a.png\ntheme: dark\n---\n# A\n<!-- design: quote\nlogo: none -->\n\n> q\n\n# B\n\nx\n",
        );
        let a = applied_settings(&p.meta, &p.slides[0]).unwrap();
        assert_eq!(
            a.trim(),
            "design: quote, logo: none, transition: fade (deck)"
        );
        let b = applied_settings(&p.meta, &p.slides[1]).unwrap();
        assert_eq!(b.trim(), "transition: fade (deck), logo: a.png (deck)");
        let none = parser::parse("# A\n");
        assert_eq!(applied_settings(&none.meta, &none.slides[0]), None);
    }

    #[test]
    fn fence_lines_find_each_tagged_fence() {
        let md = "---\ntitle: T\n---\n# A\n\n```@architecture\na -> b\n```\n\n```text\n```@architecture\n```\n\n~~~ @architecture\nc\n~~~\n";
        let p = parser::parse(md);
        assert_eq!(fence_lines(&p.slides[0], "@architecture"), [6, 14]);
        assert_eq!(fence_lines(&p.slides[0], "@bar"), Vec::<usize>::new());
    }

    #[test]
    fn summary_includes_layout_blocks_steps_and_title() {
        let s = &parser::parse(
            "# Räksmörgås & friends\n\n+ a\n+ b\n\n```@notes\nremember to smile\n```",
        )
        .slides[0];
        let line = slide_summary(4, s);
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
