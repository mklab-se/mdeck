//! `picture` checks: names that do not resolve, image files that are
//! missing, designs that never show one, and unreadable cloud files.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;
use crate::render;

/// Picture warnings: point cloud names that do not resolve, image files
/// that are missing, designs that leave no stage for a picture that would
/// otherwise show, and unreadable cloud files (reported once, on slide 0).
/// `deck` is the deck file: its folder and its generated point clouds are
/// searched, and image paths are relative to it. What the engine does not
/// show is the `engine` category's (one warning per picture, never two).
pub fn point_cloud_warnings(
    presentation: &parser::Presentation,
    deck: &std::path::Path,
    theme: &crate::theme::Theme,
) -> Vec<CheckWarning> {
    let mut out = Vec::new();
    let base = deck.parent().unwrap_or(std::path::Path::new("."));
    let mut lib = render::point_cloud::Library::for_deck(Some(base))
        .with_assets(crate::assets::point_cloud_dir(deck));
    // images are drawn by mdeck on any engine but a board, and clouds by the
    // engine or, on one without pictures, by mdeck as a stipple
    let shows = !theme.engine.is_board();
    for (i, slide) in presentation.slides.iter().enumerate() {
        let Some(name) = &slide.illustration else {
            continue;
        };
        let image = render::picture::is_image_path(name);
        let message = if image && !base.join(name).is_file() {
            format!("picture: no image file `{name}` (image paths are relative to the deck)")
        } else if image {
            match stage_problem(slide, name, theme).filter(|_| shows) {
                Some(m) => m,
                None => continue,
            }
        } else if let Err(e) = render::point_cloud::validate_name(name) {
            format!("picture: {e} (or name an image file: .png, .jpg, .jpeg, .webp or .svg)")
        } else if !lib.has(name) {
            format!(
                "no point cloud named `{name}` (run `mdeck point-cloud list`, or \
                 `mdeck ai point-cloud <deck>` to generate it)"
            )
        } else {
            match stage_problem(slide, name, theme).filter(|_| shows) {
                Some(m) => m,
                None => continue,
            }
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

/// A picture on a slide whose design leaves no stage for one (ENG-14).
fn stage_problem(slide: &parser::Slide, name: &str, theme: &crate::theme::Theme) -> Option<String> {
    (!render::design_has_stage(slide, theme)).then(|| {
        format!(
            "`{name}` is ignored: {} slides leave no stage for a picture in the {} design set",
            slide.design.name(),
            theme.arrangements.set
        )
    })
}

/// A deck folder that still keeps point clouds in v1's `illustrations/`,
/// which v2 no longer reads (CON-01).
fn v1_folder(base: &std::path::Path) -> Option<String> {
    let dir = base.join(render::point_cloud::V1_FOLDER);
    let has_clouds = std::fs::read_dir(&dir).ok()?.flatten().any(|e| {
        e.path()
            .extension()
            .is_some_and(|x| x == render::point_cloud::EXTENSION)
    });
    has_clouds.then(|| {
        format!(
            "`{}/` is a v1 folder; rename it to `{}/` (its point clouds are not read)",
            render::point_cloud::V1_FOLDER,
            render::point_cloud::FOLDER
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
        let cloud = render::point_cloud::Cloud {
            version: render::point_cloud::VERSION,
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
        // plain stipples clouds itself, so stages matter on it too
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

    /// PIC-04: the standard set opens a stage for a picture on copy
    /// designs, so only designs without one are reported there.
    #[test]
    fn the_standard_set_stages_a_picture_on_copy_slides() {
        let md = "# Deck\n<!-- picture: rocket -->\n\n---\n\n## Points\n<!-- picture: rocket -->\n\n- a\n\n---\n\n> Said.\n<!-- picture: rocket -->\n\n---\n\n## Code\n<!-- picture: rocket -->\n\n```rust\nfn main() {}\n```\n";
        let pres = parser::parse(md);
        let w = point_cloud_warnings(
            &pres,
            std::path::Path::new("/nonexistent/talk.md"),
            &crate::theme::Theme::dark(),
        );
        let slides: Vec<usize> = w.iter().map(|w| w.slide).collect();
        assert_eq!(slides, vec![4], "{w:?}");
        assert!(w[0].message.contains("code slides leave no stage"), "{w:?}");
    }

    #[test]
    fn an_image_picture_is_accepted_and_a_missing_one_reported() {
        let tmp = std::env::temp_dir().join(format!("mdeck-image-pic-{}", std::process::id()));
        std::fs::create_dir_all(tmp.join("images")).unwrap();
        std::fs::write(tmp.join("images/Team.jpg"), b"not decoded by the check").unwrap();
        let md = "# Deck\n\n---\n\n## Team\n<!-- picture: images/Team.jpg -->\n\n- a\n\n---\n\n## Gone\n<!-- picture: images/gone.png -->\n\n- a\n\n---\n\n## Code\n<!-- picture: images/Team.jpg -->\n\n```rust\nfn main() {}\n```\n";
        let pres = parser::parse(md);
        let mut theme = crate::theme::Theme::dark();
        theme.arrangements =
            crate::theme::arrangement::Arrangements::resolve("editorial", None).unwrap();
        let warnings = point_cloud_warnings(&pres, &tmp.join("talk.md"), &theme);
        let on = |n: usize| -> Vec<&str> {
            warnings
                .iter()
                .filter(|w| w.slide == n)
                .map(|w| w.message.as_str())
                .collect()
        };
        // an existing image is fine, even on plain, and never a bad cloud name
        assert!(on(2).is_empty(), "{warnings:?}");
        assert!(
            on(3)
                .iter()
                .any(|m| m.contains("no image file `images/gone.png`")),
            "{warnings:?}"
        );
        // an image shows on every engine, so a missing stage is reported
        assert!(
            on(4)
                .iter()
                .any(|m| m.contains("code slides leave no stage")),
            "{warnings:?}"
        );
        assert_eq!(on(4).len(), 1, "{warnings:?}");
        std::fs::remove_dir_all(&tmp).ok();
    }
}
