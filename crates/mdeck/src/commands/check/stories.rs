//! Stories and illustrations: sidecar entries that no longer match their
//! slide, and names that do not resolve or never show.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;
use crate::render;
use crate::render::story::sidecar::{self, Resolved, Source};

/// Story warnings (the sidecar cannot be read, broken inline scripts, entries
/// that no longer match their slide, stale stories) and each slide's story.
pub(super) fn story_warnings(
    deck: &std::path::Path,
    presentation: &parser::Presentation,
) -> (Vec<CheckWarning>, Vec<Option<Resolved>>) {
    let mut out = Vec::new();
    let loaded = match sidecar::load(deck) {
        Ok(sc) => sc,
        Err(e) => {
            out.push(CheckWarning {
                slide: 1,
                line: 0,
                category: CheckCategory::Story,
                message: format!("story sidecar could not be read: {e}"),
            });
            None
        }
    };
    let (stories, problems) = sidecar::resolve(presentation, loaded.as_ref());
    for p in problems {
        let slide = problem_slide(&p);
        out.push(CheckWarning {
            slide,
            line: slide_line(presentation, slide),
            category: CheckCategory::Story,
            message: p,
        });
    }
    for (i, r) in stories.iter().enumerate() {
        if let Some(r) = r
            && r.source == Source::Stale
        {
            out.push(CheckWarning {
                slide: i + 1,
                line: slide_line(presentation, i + 1),
                category: CheckCategory::Story,
                message: "story is stale (slide changed since it was written); run `mdeck ai story --stale`".into(),
            });
        }
    }
    (out, stories)
}

/// The first line of slide `number` (1-based), or 0 when there is none.
fn slide_line(presentation: &parser::Presentation, number: usize) -> usize {
    number
        .checked_sub(1)
        .and_then(|i| presentation.slides.get(i))
        .map_or(0, |s| s.line)
}

/// "slide N: ..." messages carry their own slide number; others go on slide 1.
fn problem_slide(message: &str) -> usize {
    message
        .strip_prefix("slide ")
        .and_then(|r| r.split(':').next())
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or(1)
}

/// Illustration warnings: names that do not resolve, layouts that never show
/// one, illustrations shadowed by a story, story casts with unknown kinds,
/// and unreadable cloud files (reported once, on slide 0).
pub fn illustration_warnings(
    presentation: &parser::Presentation,
    stories: &[Option<Resolved>],
    base: &std::path::Path,
) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    let mut lib = render::illustration::Library::for_deck(Some(base));
    for (i, slide) in presentation.slides.iter().enumerate() {
        let at = slide.directive_line("illustration");
        if let Some(name) = &slide.illustration {
            if let Err(e) = render::illustration::validate_name(name) {
                out.push(CheckWarning {
                    slide: i + 1,
                    line: at,
                    category: CheckCategory::Illustration,
                    message: format!("@illustration: {e}"),
                });
            } else if !lib.has(name) {
                out.push(CheckWarning {
                    slide: i + 1,
                    line: at,
                    category: CheckCategory::Illustration,
                    message: format!(
                        "no illustration named `{name}` (run `mdeck illustration list`, or \
                         `mdeck illustration generate --name {name} --description \"...\"`)"
                    ),
                });
            } else if !render::ember::handles(slide) {
                out.push(CheckWarning {
                    slide: i + 1,
                    line: at,
                    category: CheckCategory::Illustration,
                    message: format!(
                        "`{name}` is ignored: {} slides do not show an illustration",
                        format!("{:?}", slide.layout).to_lowercase()
                    ),
                });
            } else if stories.get(i).is_some_and(|r| r.is_some()) {
                out.push(CheckWarning {
                    slide: i + 1,
                    line: at,
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
                    line: slide.line,
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
            line: 0,
            category: CheckCategory::Illustration,
            message: p,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_name_their_slide() {
        assert_eq!(problem_slide("slide 7: bad beat"), 7);
        assert_eq!(problem_slide("something else"), 1);
    }

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
        let pres = parser::parse(md);
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
}
