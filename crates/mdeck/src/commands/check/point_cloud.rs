//! `picture` checks: names that do not resolve, layouts that never
//! show one, and unreadable cloud files.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;
use crate::render;

/// Point cloud warnings: names that do not resolve, layouts that never show
/// one, and unreadable cloud files (reported once, on slide 0). `deck` is
/// the deck file: its folder and its generated point clouds are searched.
pub fn point_cloud_warnings(
    presentation: &parser::Presentation,
    deck: &std::path::Path,
    theme: &crate::theme::Theme,
) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    let base = deck.parent().unwrap_or(std::path::Path::new("."));
    let mut lib = render::illustration::Library::for_deck(Some(base))
        .with_assets(crate::assets::point_cloud_dir(deck));
    for (i, slide) in presentation.slides.iter().enumerate() {
        let Some(name) = &slide.illustration else {
            continue;
        };
        let message = if let Err(e) = render::illustration::validate_name(name) {
            format!("picture: {e}")
        } else if !lib.has(name) {
            format!(
                "no point cloud named `{name}` (run `mdeck point-cloud list`, or \
                 `mdeck ai point-cloud <deck>` to generate it)"
            )
        } else if !render::design_has_stage(slide, theme) {
            format!(
                "`{name}` is ignored: {} slides leave no stage for a picture in the {} design set",
                slide.design.name(),
                theme.arrangements.set
            )
        } else {
            continue;
        };
        out.push(CheckWarning {
            slide: i + 1,
            line: slide.setting_line("picture"),
            category: CheckCategory::PointCloud,
            message,
            place: None,
        });
    }
    for message in v1_folder(base).into_iter().chain(lib.take_problems()) {
        out.push(CheckWarning {
            slide: 0,
            line: 0,
            category: CheckCategory::PointCloud,
            message,
            place: None,
        });
    }
    out
}

/// A deck folder that still keeps point clouds in v1's `illustrations/`,
/// which v2 no longer reads (CON-01).
fn v1_folder(base: &std::path::Path) -> Option<String> {
    let dir = base.join(render::illustration::V1_FOLDER);
    let has_clouds = std::fs::read_dir(&dir).ok()?.flatten().any(|e| {
        e.path()
            .extension()
            .is_some_and(|x| x == render::illustration::EXTENSION)
    });
    has_clouds.then(|| {
        format!(
            "`{}/` is a v1 folder; rename it to `{}/` (its point clouds are not read)",
            render::illustration::V1_FOLDER,
            render::illustration::FOLDER
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_v1_illustrations_folder_is_reported() {
        let tmp = std::env::temp_dir().join(format!("mdeck-v1-clouds-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("illustrations")).unwrap();
        let pres = parser::parse("# Hi\n");
        let theme = crate::theme::Theme::dark();
        let talk = tmp.join("talk.md");
        // a folder without point clouds is not a v1 library
        assert!(point_cloud_warnings(&pres, &talk, &theme).is_empty());
        std::fs::write(tmp.join("illustrations/kettle.mdpc"), "{}").unwrap();
        let messages: Vec<String> = point_cloud_warnings(&pres, &talk, &theme)
            .into_iter()
            .filter(|w| w.slide == 0)
            .map(|w| w.message)
            .collect();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("v1 folder; rename it to `point-clouds/`")),
            "{messages:?}"
        );
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn point_cloud_warnings_cover_missing_names_and_layouts() {
        let tmp = std::env::temp_dir().join(format!("mdeck-illu-check-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("point-clouds")).unwrap();
        let cloud = render::illustration::Cloud {
            version: render::illustration::VERSION,
            name: "kettle".into(),
            description: String::new(),
            prompt: None,
            generated: None,
            aspect: 1.0,
            points: std::sync::Arc::new(vec![[0.5, 0.5]]),
        };
        std::fs::write(tmp.join("point-clouds/kettle.mdpc"), cloud.to_json()).unwrap();
        std::fs::write(tmp.join("point-clouds/broken.mdpc"), "{").unwrap();
        let md = "\n## Fine\n<!-- picture: kettle -->\n\n- a\n\n---\n\n\n## Missing\n<!-- picture: nothing -->\n\n- a\n\n---\n\n\n## Code\n<!-- picture: kettle -->\n\n```rust\nfn main() {}\n```\n\n---\n\n\n## Bad\n<!-- picture: Bad Name -->\n\n- a\n\n---\n\n\n## Broken\n<!-- picture: broken -->\n\n- a\n";
        let pres = parser::parse(md);
        let mut theme = crate::theme::Theme::dark();
        theme.arrangements =
            crate::theme::arrangement::Arrangements::resolve("editorial", None).unwrap();
        let warnings = point_cloud_warnings(&pres, &tmp.join("talk.md"), &theme);
        let by_slide: Vec<(usize, String)> = warnings
            .iter()
            .map(|w| (w.slide, w.message.clone()))
            .collect();
        assert!(!by_slide.iter().any(|(s, _)| *s == 1), "{by_slide:?}");
        assert!(
            by_slide
                .iter()
                .any(|(s, m)| *s == 2 && m.contains("no point cloud named `nothing`")),
            "{by_slide:?}"
        );
        assert!(
            by_slide
                .iter()
                .any(|(s, m)| *s == 3 && m.contains("code slides leave no stage")),
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
