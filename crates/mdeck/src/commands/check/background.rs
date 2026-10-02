//! Background images: files that are missing, are not images or do not
//! decode, and opacities that do not parse.

use std::path::Path;

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;
use crate::render::background;

/// Background warnings, on the slide (or the frontmatter, slide 0) and line
/// they come from. `content` is the deck file, for frontmatter lines.
pub fn background_warnings(
    presentation: &parser::Presentation,
    base: &Path,
    content: &str,
) -> Vec<CheckWarning> {
    let (backgrounds, problems) = background::resolve(presentation, base);
    let warn = |slide, line, message| CheckWarning {
        slide,
        line,
        category: CheckCategory::Background,
        message,
    };
    let mut out: Vec<CheckWarning> = problems
        .into_iter()
        .map(|p| {
            let line = match p.slide {
                0 => frontmatter_line(content, &p.message),
                _ => p.line,
            };
            warn(p.slide, line, p.message)
        })
        .collect();
    // A file that exists can still fail to decode; say so where it is first used.
    for path in backgrounds.distinct_paths() {
        if let Err(e) = crate::render::image_cache::decode(&base.join(path)) {
            let first = (0..presentation.slides.len())
                .find(|&i| backgrounds.get(i).is_some_and(|b| b.path == path))
                .unwrap_or(0);
            let slide = &presentation.slides[first];
            let own =
                parser::setting(&slide.settings, "background").is_some_and(|v| v.trim() == path);
            let (n, line) = if own {
                (first + 1, slide.setting_line("background"))
            } else {
                (0, frontmatter_line(content, "background:"))
            };
            out.push(warn(n, line, format!("background: {e}")));
        }
    }
    out
}

/// The frontmatter line of the key a problem `message` starts with
/// (`background:` or `background-opacity:`), or 0.
fn frontmatter_line(content: &str, message: &str) -> usize {
    let key = if message.starts_with("background-opacity") {
        "background-opacity:"
    } else {
        "background:"
    };
    let mut lines = content.lines().enumerate();
    if lines.next().map(|(_, l)| l.trim()) != Some("---") {
        return 0;
    }
    lines
        .take_while(|(_, l)| l.trim() != "---")
        .find(|(_, l)| l.trim_start().starts_with(key))
        .map_or(0, |(i, _)| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warnings_name_the_frontmatter_or_slide_line() {
        let dir = std::env::temp_dir().join(format!("mdeck-check-bg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("broken.png"), b"not a png").unwrap();
        let md = "---\ntitle: T\nbackground: gone.jpg\nbackground-opacity: 300%\n---\n\n\
                  # One\n\n- a\n\n# Two\n<!-- background: broken.png -->\n\n- b\n";
        let pres = parser::parse(md);
        let w = background_warnings(&pres, &dir, md);
        let found: Vec<(usize, usize)> = w.iter().map(|w| (w.slide, w.line)).collect();
        assert_eq!(found, [(0, 4), (0, 3), (2, 12)], "{w:?}");
        assert!(w.iter().all(|w| w.category == CheckCategory::Background));
        assert!(w[1].message.contains("gone.jpg"));
        assert!(w[2].message.contains("broken.png"), "{}", w[2].message);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_working_background_is_quiet() {
        let dir = std::env::temp_dir().join(format!("mdeck-check-bg-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        image::RgbaImage::new(4, 4)
            .save(dir.join("ok.png"))
            .unwrap();
        let md = "---\nbackground: ok.png\n---\n\n# One\n\n- a\n";
        assert!(background_warnings(&parser::parse(md), &dir, md).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
