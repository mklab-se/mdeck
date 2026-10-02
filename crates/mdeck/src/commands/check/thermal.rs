//! `@thermal` blocks: unreadable sources, colour input, settings the source
//! cannot honour, and comparisons that cannot share a scale. The same
//! messages print when the deck is presented or exported.

use std::path::Path;

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;
use crate::render::thermal::Library;

pub fn thermal_warnings(presentation: &parser::Presentation, base: &Path) -> Vec<CheckWarning> {
    let mut lib = Library::new(base.to_path_buf());
    let mut out: Vec<CheckWarning> = lib
        .load(presentation)
        .into_iter()
        .map(|d| CheckWarning {
            slide: d.slide,
            line: d.line,
            category: CheckCategory::Thermal,
            message: d.message,
        })
        .collect();
    // the palette the deck asks for
    if let Some(p) = &presentation.meta.palette
        && crate::render::thermal::Palette::from_name(p).is_none()
    {
        out.push(CheckWarning {
            slide: 0,
            line: 0,
            category: CheckCategory::Thermal,
            message: format!(
                "@palette: '{p}' is not one of iron, white-hot, black-hot, rainbow, arctic, lava"
            ),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck_dir(tag: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("mdeck-check-thermal-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        image::GrayImage::from_fn(32, 16, |x, _| image::Luma([(x * 8) as u8]))
            .save(d.join("gray.png"))
            .unwrap();
        image::RgbImage::from_fn(32, 16, |x, _| image::Rgb([(x * 8) as u8, 30, 160]))
            .save(d.join("iron.png"))
            .unwrap();
        d
    }

    #[test]
    fn colour_input_and_unsupported_settings_are_reported_on_their_line() {
        let d = deck_dir("lines");
        let md = "---\n@palette: plasma\n---\n\n# A\n\n```@thermal\nimage: iron.png\n+ above 80%\n```\n\n# B\n\n```@thermal\nimage: gray.png\nwindow: 40..90 °C\n+ above 60 °C\n- spot Sp1 50% 50%: 86 °C\n```\n\n# C\n\n```@thermal\nimage: gone.png\n```\n";
        let pres = parser::parse(md);
        let w = thermal_warnings(&pres, &d);
        let found: Vec<(usize, usize)> = w.iter().map(|w| (w.slide, w.line)).collect();
        assert!(
            w[0].message.contains("unsupported chromatic input"),
            "{w:?}"
        );
        assert!(w[0].message.contains("1 threshold step left out"), "{w:?}");
        assert_eq!(found[0], (1, 8), "the image: line");
        let b: Vec<&CheckWarning> = w.iter().filter(|w| w.slide == 2).collect();
        assert!(
            b.iter()
                .any(|w| w.line == 16 && w.message.contains("window:")),
            "{b:?}"
        );
        assert!(
            b.iter()
                .any(|w| w.line == 17 && w.message.contains("above 60")),
            "{b:?}"
        );
        assert!(
            b.iter()
                .any(|w| w.line == 18 && w.message.contains("author-supplied")),
            "{b:?}"
        );
        assert!(
            w.iter()
                .any(|w| w.slide == 3 && w.message.contains("gone.png"))
        );
        assert!(
            w.iter()
                .any(|w| w.slide == 0 && w.message.contains("plasma"))
        );
        assert!(w.iter().all(|w| w.category == CheckCategory::Thermal));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_comparison_needs_mapped_sources() {
        let d = deck_dir("compare");
        let md = "# Before and after\n@thermal-window: 30..90 °C\n\n```@thermal\nimage: gray.png\nmapping: linear 20..100 °C\n```\n\n```@thermal\nimage: gray.png\n```\n";
        let pres = parser::parse(md);
        let w = thermal_warnings(&pres, &d);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].message.contains("cannot be compared"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_clean_block_is_quiet() {
        let d = deck_dir("clean");
        let md = "# A\n\n```@thermal\nimage: gray.png\npalette: arctic\n+ lens 50% 50% 20%\n+ reveal\n+ above 70%\n* spot Hotspot 90% 50%\n```\n";
        assert!(thermal_warnings(&parser::parse(md), &d).is_empty());
        std::fs::remove_dir_all(&d).ok();
    }
}
