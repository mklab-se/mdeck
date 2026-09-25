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

    for w in illustration_warnings(&presentation, &stories, base_path) {
        report.add(w);
    }
    if let Some(w) = cjk_font_warning(&presentation, render::fonts::cjk_coverage()) {
        report.add(w);
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

/// Illustration warnings: names that do not resolve, layouts that never show
/// one, illustrations shadowed by a story, story casts with unknown kinds,
/// and unreadable cloud files (reported once, on slide 0).
pub fn illustration_warnings(
    presentation: &parser::Presentation,
    stories: &[Option<render::story::sidecar::Resolved>],
    base: &std::path::Path,
) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    let mut lib = render::illustration::Library::for_deck(Some(base));
    for (i, slide) in presentation.slides.iter().enumerate() {
        if let Some(name) = &slide.illustration {
            if let Err(e) = render::illustration::validate_name(name) {
                out.push(CheckWarning {
                    slide: i + 1,
                    category: CheckCategory::Illustration,
                    message: format!("@illustration: {e}"),
                });
            } else if !lib.has(name) {
                out.push(CheckWarning {
                    slide: i + 1,
                    category: CheckCategory::Illustration,
                    message: format!(
                        "no illustration named `{name}` (run `mdeck illustration list`, or \
                         `mdeck illustration generate --name {name} --description \"...\"`)"
                    ),
                });
            } else if !render::ember::handles(slide) {
                out.push(CheckWarning {
                    slide: i + 1,
                    category: CheckCategory::Illustration,
                    message: format!(
                        "`{name}` is ignored: {} slides do not show an illustration",
                        format!("{:?}", slide.layout).to_lowercase()
                    ),
                });
            } else if stories.get(i).is_some_and(|r| r.is_some()) {
                out.push(CheckWarning {
                    slide: i + 1,
                    category: CheckCategory::Illustration,
                    message: format!(
                        "`{name}` is ignored because the slide plays a story (cast it as a story kind instead)"
                    ),
                });
            }
        }
        if let Some(r) = stories.get(i).and_then(|r| r.as_ref()) {
            let unknown = r.script.unknown_kinds(&mut lib);
            if !unknown.is_empty() {
                out.push(CheckWarning {
                    slide: i + 1,
                    category: CheckCategory::Story,
                    message: format!(
                        "story casts unknown kind(s): {} (see `mdeck illustration list`)",
                        unknown.join(", ")
                    ),
                });
            }
        }
    }
    for p in lib.take_problems() {
        out.push(CheckWarning {
            slide: 0,
            category: CheckCategory::Illustration,
            message: p,
        });
    }
    out
}

/// Chinese, Japanese or Korean text needs system CJK faces mdeck does not
/// bundle (GitHub issue 11). One warning, on the first slide whose text is
/// not covered by `covered` (the scripts the faces on this machine draw).
pub fn cjk_font_warning(
    presentation: &parser::Presentation,
    covered: render::fonts::Scripts,
) -> Option<CheckWarning> {
    use render::fonts::Scripts;
    let missing = |text: &str| Scripts::of(text).minus(covered);
    let title = presentation
        .meta
        .title
        .as_deref()
        .map(missing)
        .unwrap_or_default();
    let slides: Vec<(usize, Scripts)> = presentation
        .slides
        .iter()
        .enumerate()
        .map(|(i, s)| (i + 1, missing(&s.raw_source)))
        .filter(|(_, m)| !m.is_empty())
        .collect();
    let what = match (title.is_empty(), slides.len()) {
        (true, 0) => return None,
        (false, 0) => "the deck title uses".to_string(),
        (_, 1) => "1 slide uses".to_string(),
        (_, n) => format!("{n} slides use"),
    };
    let scripts = slides
        .iter()
        .fold(title, |acc, (_, m)| acc.union(*m))
        .names()
        .join(" and ");
    let hint = if cfg!(all(unix, not(target_os = "macos"))) {
        "install one (e.g. the `fonts-noto-cjk` package) "
    } else {
        "install one "
    };
    Some(CheckWarning {
        slide: if title.is_empty() { slides[0].0 } else { 1 },
        category: CheckCategory::Fonts,
        message: format!(
            "{what} {scripts} text but no system font covers it, so it will draw as boxes; \
             {hint}or set {}=/path/to/font.ttc",
            render::fonts::CJK_FONT_ENV
        ),
    })
}

/// Print the CJK font warning for a deck about to be presented or exported.
pub fn warn_missing_cjk_font(presentation: &parser::Presentation) {
    if let Some(w) = cjk_font_warning(presentation, render::fonts::cjk_coverage()) {
        use colored::Colorize;
        eprintln!("{} {}", "Warning:".yellow().bold(), w.message);
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

    #[test]
    fn illustration_warnings_cover_missing_names_layouts_and_stories() {
        let tmp = std::env::temp_dir().join(format!("mdeck-illu-check-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("illustrations")).unwrap();
        let cloud = render::illustration::Cloud {
            version: render::illustration::VERSION,
            name: "kettle".into(),
            description: String::new(),
            prompt: None,
            generated: None,
            aspect: 1.0,
            points: std::sync::Arc::new(vec![[0.5, 0.5]]),
        };
        std::fs::write(tmp.join("illustrations/kettle.mdpc"), cloud.to_json()).unwrap();
        std::fs::write(tmp.join("illustrations/broken.mdpc"), "{").unwrap();
        let md = "@illustration: kettle\n\n## Fine\n\n- a\n\n---\n\n@illustration: nothing\n\n## Missing\n\n- a\n\n---\n\n@illustration: kettle\n\n## Code\n\n```rust\nfn main() {}\n```\n\n---\n\n@illustration: Bad Name\n\n## Bad\n\n- a\n\n---\n\n@illustration: broken\n\n## Broken\n\n- a\n";
        let pres = parser::parse(md, &tmp);
        let stories: Vec<Option<render::story::sidecar::Resolved>> = vec![None; pres.slides.len()];
        let warnings = illustration_warnings(&pres, &stories, &tmp);
        let by_slide: Vec<(usize, String)> = warnings
            .iter()
            .map(|w| (w.slide, w.message.clone()))
            .collect();
        assert!(!by_slide.iter().any(|(s, _)| *s == 1), "{by_slide:?}");
        assert!(
            by_slide
                .iter()
                .any(|(s, m)| *s == 2 && m.contains("no illustration named `nothing`")),
            "{by_slide:?}"
        );
        assert!(
            by_slide
                .iter()
                .any(|(s, m)| *s == 3 && m.contains("code slides do not show")),
            "{by_slide:?}"
        );
        assert!(
            by_slide
                .iter()
                .any(|(s, m)| *s == 4 && m.contains("lowercase")),
            "{by_slide:?}"
        );
        assert!(
            by_slide
                .iter()
                .any(|(s, m)| *s == 0 && m.contains("broken.mdpc")),
            "{by_slide:?}"
        );

        // a story on the slide shadows the illustration; an unknown cast kind is reported
        let script = render::story::Script::parse("cast:\n  - { id: a, kind: kettle, cell: left }\n  - { id: b, kind: zeppelin, cell: right }\n").unwrap();
        let mut stories = stories;
        stories[0] = Some(render::story::sidecar::Resolved {
            script,
            source: render::story::sidecar::Source::Sidecar,
        });
        let warnings = illustration_warnings(&pres, &stories, &tmp);
        assert!(
            warnings
                .iter()
                .any(|w| w.slide == 1 && w.message.contains("plays a story"))
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.slide == 1 && w.message.contains("zeppelin"))
        );
        std::fs::remove_dir_all(&tmp).ok();
    }
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
            illustration: None,
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

    #[test]
    fn cjk_warning_names_the_first_slide_and_counts_the_rest_when_no_font() {
        let md = "---\ntitle: Plain\n---\n\n# Hello\n\n- one\n\n---\n\n# 中文标题\n\n- 第一点\n\n---\n\n# Also\n\n- 日本語\n";
        let pres = parser::parse(md, std::path::Path::new("."));
        let w = cjk_font_warning(&pres, render::fonts::Scripts::NONE)
            .expect("CJK text without a font warns");
        assert_eq!(w.slide, 2);
        assert_eq!(w.category, CheckCategory::Fonts);
        assert!(
            w.message.contains("2 slides use Chinese text"),
            "{}",
            w.message
        );
        assert!(w.message.contains("MDECK_CJK_FONT"), "{}", w.message);

        // a Chinese-only font leaves the kana slide uncovered, and the
        // warning moves to it and names only what is missing
        let md = "# 中文\n\n---\n\n# ひらがな と 漢字\n";
        let pres = parser::parse(md, std::path::Path::new("."));
        let han_only = render::fonts::Scripts {
            han: true,
            ..render::fonts::Scripts::NONE
        };
        let w = cjk_font_warning(&pres, han_only).unwrap();
        assert_eq!(w.slide, 2);
        assert!(
            w.message.contains("1 slide uses Japanese kana text"),
            "{}",
            w.message
        );
    }

    #[test]
    fn cjk_warning_covers_the_frontmatter_title() {
        let md = "---\ntitle: 中文测试\n---\n\n# Hello\n\n- one\n";
        let pres = parser::parse(md, std::path::Path::new("."));
        let w = cjk_font_warning(&pres, render::fonts::Scripts::NONE)
            .expect("a CJK title without a font warns");
        assert_eq!(w.slide, 1);
        assert!(
            w.message.starts_with("the deck title uses Chinese"),
            "{}",
            w.message
        );
    }

    #[test]
    fn cjk_warning_is_silent_with_a_font_or_without_cjk_text() {
        let cjk = parser::parse("# 中文", std::path::Path::new("."));
        assert!(cjk_font_warning(&cjk, render::fonts::Scripts::ALL).is_none());
        let plain = parser::parse("# Hello\n\n- Räksmörgås ①", std::path::Path::new("."));
        assert!(cjk_font_warning(&plain, render::fonts::Scripts::NONE).is_none());
    }
}
