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
mod point_cloud;
mod requires;
mod settings;
mod theme;
mod thermal;
pub use assets::asset_warnings;
mod visuals;
pub use background::background_warnings;
pub use content::{cjk_font_warning, content_warnings, math_warnings, warn_missing_cjk_font};
pub use engine::{deck_theme, engine_warnings};
pub use point_cloud::point_cloud_warnings;
pub use settings::{fence_warnings, settings_warnings};
pub use theme::theme_warnings;
pub use thermal::thermal_warnings;

pub fn run(
    file: PathBuf,
    verbose: u8,
    quiet: bool,
    engine: Option<String>,
    theme: Option<String>,
) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(&file)?;
    let base_path = file.parent().unwrap_or(Path::new("."));
    let mut presentation = parser::parse(&content);
    // `--theme` beats the deck's `theme`, as it does when presenting
    override_theme(&mut presentation, theme.as_deref());

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

/// Check the deck in `theme` (the `--theme` flag) instead of its own.
fn override_theme(presentation: &mut parser::Presentation, theme: Option<&str>) {
    if let Some(name) = theme.map(str::trim).filter(|n| !n.is_empty()) {
        presentation.meta.theme = Some(name.to_string());
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
    add(crate::engines::settings_problems(&theme)
        .into_iter()
        .map(|message| CheckWarning {
            slide: 0,
            line: 0,
            category: CheckCategory::Engine,
            message: format!("theme {}: {message}", theme.name),
        })
        .collect());
    add(point_cloud_warnings(presentation, file, &theme));
    add(design_warnings(presentation));
    add(engine_warnings(presentation, theme.engine));
    add(engine::picture_stage_warnings(presentation, &theme));
    add(asset_warnings(file, presentation, &theme));
    Ok(report)
}

/// Slides whose `design` setting could not hold their content (DES-04):
/// the rest shows in the body, or the slide falls back to `content`.
fn design_warnings(presentation: &parser::Presentation) -> Vec<CheckWarning> {
    use parser::design::Reason;
    presentation
        .slides
        .iter()
        .enumerate()
        .filter_map(|(i, slide)| {
            let r = &slide.recognition;
            let message = match &r.reason {
                Reason::ChosenWithRest => format!(
                    "design: {} does not have a role for everything on this slide; the rest \
                     shows in its body, in reading order",
                    r.design.name()
                ),
                Reason::FellBack(_) => format!("this slide is drawn as {}", r.describe()),
                _ => return None,
            };
            Some(CheckWarning {
                slide: i + 1,
                line: slide.setting_line("design"),
                category: CheckCategory::Settings,
                message,
            })
        })
        .collect()
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
        "  slide {:>3}: {}, {} block{}, {} step{}",
        index + 1,
        slide.recognition.describe(),
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

    use crate::parser::{Block, Design, Slide};

    fn slide(blocks: Vec<Block>, design: Design, notes: Option<&str>) -> Slide {
        Slide {
            blocks,
            design,
            raw_source: String::new(),
            notes: notes.map(String::from),
            ..Default::default()
        }
    }

    // `--check --theme departures` checks the deck on the split-flap board:
    // a diagram it cannot show is reported, not silently dropped.
    #[cfg(feature = "splitflap")]
    #[test]
    fn the_theme_flag_checks_the_deck_in_that_theme() {
        let content = "---\ntheme: ember\n---\n# Map\n\n```@architecture\n- A (icon: api)\n- B (icon: database)\n- A -> B\n```\n";
        let mut p = parser::parse(content);
        let file = Path::new("/nonexistent/talk.md");
        let engine = |p: &parser::Presentation| {
            collect(file, content, p, Path::new("/nonexistent"), None)
                .unwrap()
                .warnings()
                .filter(|w| w.category == CheckCategory::Engine)
                .map(|w| w.message.clone())
                .collect::<Vec<_>>()
        };
        assert!(engine(&p).is_empty(), "{:?}", engine(&p));
        override_theme(&mut p, Some(" departures "));
        let w = engine(&p);
        assert!(
            w.iter().any(|m| m.contains("diagrams are not shown")),
            "{w:?}"
        );
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
    fn a_design_that_cannot_hold_the_slide_is_reported() {
        let p = parser::parse(
            "# A\n<!-- design: quote -->\n\n- one\n\n> q\n\n# B\n<!-- design: code -->\n\n- x\n\n# C\n<!-- design: points -->\n\n- y\n",
        );
        let w = design_warnings(&p);
        assert_eq!(w.len(), 2, "{w:?}");
        assert!(
            w[0].message.contains("shows in its body"),
            "{}",
            w[0].message
        );
        assert_eq!(w[0].line, 2);
        assert!(w[1].message.contains("no code block"), "{}", w[1].message);
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
        assert!(
            line.contains("slide   5: points (a heading + one list,"),
            "{line}"
        );
        assert!(line.contains("2 blocks"), "{line}");
        assert!(line.contains("2 steps"), "{line}");
        assert!(line.contains("\"Räksmörgås & friends\""), "{line}");
        assert!(line.ends_with("[notes]"), "{line}");
    }

    #[test]
    fn summary_without_heading_or_notes() {
        let s = slide(
            vec![Block::Paragraph { inlines: vec![] }],
            Design::Content,
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
