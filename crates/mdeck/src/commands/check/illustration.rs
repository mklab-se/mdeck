//! `picture` checks: names that do not resolve, layouts that never
//! show one, and unreadable cloud files.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;
use crate::render;

/// Illustration warnings: names that do not resolve, layouts that never show
/// one, and unreadable cloud files (reported once, on slide 0).
pub fn illustration_warnings(
    presentation: &parser::Presentation,
    base: &std::path::Path,
) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    let mut lib = render::illustration::Library::for_deck(Some(base));
    for (i, slide) in presentation.slides.iter().enumerate() {
        let Some(name) = &slide.illustration else {
            continue;
        };
        let message = if let Err(e) = render::illustration::validate_name(name) {
            format!("picture: {e}")
        } else if !lib.has(name) {
            format!(
                "no illustration named `{name}` (run `mdeck illustration list`, or \
                 `mdeck illustration generate --name {name} --description \"...\"`)"
            )
        } else if !render::ember::handles(slide) {
            format!(
                "`{name}` is ignored: {} slides do not show an illustration",
                format!("{:?}", slide.layout).to_lowercase()
            )
        } else {
            continue;
        };
        out.push(CheckWarning {
            slide: i + 1,
            line: slide.setting_line("picture"),
            category: CheckCategory::Illustration,
            message,
        });
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
    fn illustration_warnings_cover_missing_names_and_layouts() {
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
        let md = "\n## Fine\n<!-- picture: kettle -->\n\n- a\n\n---\n\n\n## Missing\n<!-- picture: nothing -->\n\n- a\n\n---\n\n\n## Code\n<!-- picture: kettle -->\n\n```rust\nfn main() {}\n```\n\n---\n\n\n## Bad\n<!-- picture: Bad Name -->\n\n- a\n\n---\n\n\n## Broken\n<!-- picture: broken -->\n\n- a\n";
        let pres = parser::parse(md);
        let warnings = illustration_warnings(&pres, &tmp);
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
        std::fs::remove_dir_all(&tmp).ok();
    }
}
