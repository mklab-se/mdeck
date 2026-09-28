use std::path::PathBuf;

use crate::check::{CheckCategory, CheckReport, CheckWarning};
use crate::parser;
use crate::render;

mod content;
mod directives;
pub use content::{cjk_font_warning, math_warnings, warn_missing_cjk_font};
pub use directives::directive_warnings;

pub fn run(file: PathBuf, verbose: u8, quiet: bool, engine: Option<String>) -> anyhow::Result<()> {
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
    for w in directive_warnings(&presentation) {
        report.add(w);
    }
    for w in math_warnings(&presentation) {
        report.add(w);
    }
    let defaults = crate::config::Config::load_or_default()
        .defaults
        .unwrap_or_default();
    for w in theme_warnings(&presentation, defaults.theme.as_deref(), base_path) {
        report.add(w);
    }
    let (theme, problems) = deck_theme(
        &presentation,
        defaults.theme.as_deref(),
        base_path,
        engine.as_deref(),
    )?;
    let kind = theme.engine;
    for message in problems {
        report.add(CheckWarning {
            slide: 0,
            category: CheckCategory::Engine,
            message,
        });
    }
    let with_story: Vec<bool> = stories.iter().map(Option::is_some).collect();
    for w in engine_warnings(&presentation, &with_story, kind) {
        report.add(w);
    }
    for w in art_warnings(&file, &presentation, &theme) {
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

/// The theme the deck runs in, on the engine it runs on (`--engine`,
/// `@engine`, then the theme's), and any problem with the deck's `@engine`.
/// An unknown `--engine` is an error.
pub fn deck_theme(
    presentation: &parser::Presentation,
    config_default: Option<&str>,
    base: &std::path::Path,
    cli: Option<&str>,
) -> anyhow::Result<(crate::theme::Theme, Vec<String>)> {
    use crate::theme::lookup;
    let (kind, problems) = crate::engines::choose(cli, presentation.meta.engine.as_deref())
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let name = lookup::select(presentation.meta.theme.as_deref(), config_default);
    let themes = lookup::Lookup::for_deck(Some(base));
    let (theme, _) = lookup::resolve_or_default(&themes, &name);
    Ok((crate::engines::with_engine(theme, kind), problems))
}

/// Generated art on an art engine: the sidecar cannot be read, pictures
/// are stale (per slide) or missing (one line for the deck).
pub fn art_warnings(
    deck: &std::path::Path,
    presentation: &parser::Presentation,
    theme: &crate::theme::Theme,
) -> Vec<CheckWarning> {
    use render::art::sidecar::Source;
    let Some(medium) = theme.engine.medium() else {
        return Vec::new();
    };
    let mut art = render::art::gallery::DeckArt::new(Some(deck), false);
    art.sync(presentation, theme);
    let mut out: Vec<CheckWarning> = art
        .problems()
        .iter()
        .map(|p| CheckWarning {
            slide: 0,
            category: CheckCategory::Art,
            message: format!("art sidecar could not be read: {p}"),
        })
        .collect();
    let mut missing = Vec::new();
    for (i, (slide, r)) in presentation.slides.iter().zip(art.resolved()).enumerate() {
        if !render::art::wants_art(slide) {
            continue;
        }
        match r {
            None => missing.push(i + 1),
            Some(r) if !r.file.exists() => out.push(CheckWarning {
                slide: i + 1,
                category: CheckCategory::Art,
                message: format!("art file {} is missing", r.file.display()),
            }),
            Some(r) if r.source == Source::Stale => out.push(CheckWarning {
                slide: i + 1,
                category: CheckCategory::Art,
                message: "art is stale (the slide changed since it was drawn); run `mdeck ai art --stale`".into(),
            }),
            _ => {}
        }
    }
    if !missing.is_empty() {
        let list: Vec<String> = missing.iter().map(|n| n.to_string()).collect();
        out.push(CheckWarning {
            slide: 0,
            category: CheckCategory::Art,
            message: format!(
                "{} slide{} no picture for the {} engine ({}); run `mdeck ai art {}` to draw {}",
                missing.len(),
                if missing.len() == 1 { " has" } else { "s have" },
                medium.name,
                list.join(", "),
                deck.file_name().unwrap_or_default().to_string_lossy(),
                if missing.len() == 1 { "it" } else { "them" },
            ),
        });
    }
    out
}

/// Content the deck's engine will not show, one warning per slide and thing.
pub fn engine_warnings(
    presentation: &parser::Presentation,
    with_story: &[bool],
    kind: crate::engines::EngineKind,
) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        let story = with_story.get(i).copied().unwrap_or(false);
        for message in crate::engines::unsupported(kind, slide, story) {
            out.push(CheckWarning {
                slide: i + 1,
                category: CheckCategory::Engine,
                message,
            });
        }
    }
    out
}

/// Theme warnings (on slide 0, since a theme belongs to the deck): the name
/// does not resolve, the file is invalid, something in it fell back, or
/// text is hard to read on its background.
pub fn theme_warnings(
    presentation: &parser::Presentation,
    config_default: Option<&str>,
    base: &std::path::Path,
) -> Vec<CheckWarning> {
    use crate::theme::lookup;
    let name = lookup::select(presentation.meta.theme.as_deref(), config_default);
    let themes = lookup::Lookup::for_deck(Some(base));
    let warn = |message: String| CheckWarning {
        slide: 0,
        category: CheckCategory::Theme,
        message,
    };
    let (theme, mut out) = match themes.load(&name) {
        Err(e) => (
            lookup::load_builtin(lookup::DEFAULT_THEME).expect("default theme"),
            vec![warn(format!(
                "{e}; the deck falls back to {}",
                lookup::DEFAULT_THEME
            ))],
        ),
        Ok(built) => {
            let w = built
                .warnings
                .iter()
                .cloned()
                .chain(crate::theme::validate::review(&built.theme))
                .map(|m| warn(format!("{name}: {m}")))
                .collect();
            (built.theme, w)
        }
    };
    // The deck's own logo keys, and whether the logo file can be drawn.
    let (logos, problems) = crate::render::logo::resolve_slides(&theme, presentation, base);
    out.extend(problems.into_iter().map(warn));
    for logo in logos.distinct() {
        if let Err(e) = crate::render::logo::load_image(&logo.path) {
            out.push(warn(format!("logo: {e}")));
        }
    }
    out
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

    #[test]
    fn theme_problems_are_reported_on_the_deck() {
        let dir = std::env::temp_dir().join(format!("mdeck-check-theme-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("themes")).unwrap();
        std::fs::write(
            dir.join("themes/murky.yaml"),
            "colors: { background: '#777777', text: '#808080' }\n",
        )
        .unwrap();
        std::fs::write(dir.join("themes/typo.yaml"), "colours: {}\n").unwrap();
        let deck = |theme: &str| parser::parse(&format!("---\n@theme: {theme}\n---\n# A\n"), &dir);

        assert!(theme_warnings(&deck("dark"), None, &dir).is_empty());
        let unknown = theme_warnings(&deck("solarized"), None, &dir);
        assert!(
            unknown[0].message.contains("unknown theme 'solarized'"),
            "{unknown:?}"
        );
        let typo = theme_warnings(&deck("typo"), None, &dir);
        assert!(typo[0].message.contains("colours"), "{typo:?}");
        let murky = theme_warnings(&deck("murky"), None, &dir);
        assert!(
            murky.iter().any(|w| w.message.contains("contrast")),
            "{murky:?}"
        );
        assert_eq!(murky[0].category, CheckCategory::Theme);
        // The config default applies when the deck names no theme.
        let plain = parser::parse("# A\n", &dir);
        assert!(!theme_warnings(&plain, Some("murky"), &dir).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

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
            logo: None,
            art: None,
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
