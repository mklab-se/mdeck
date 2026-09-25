//! Content warnings: text the fonts on this machine cannot draw, and
//! formulas that do not parse.

use crate::check::{CheckCategory, CheckWarning};
use crate::parser;
use crate::render;

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

/// Formulas that do not parse, one warning each with RaTeX's reason. They
/// render as their source text.
pub fn math_warnings(presentation: &parser::Presentation) -> Vec<CheckWarning> {
    fn walk(inlines: &[parser::Inline], out: &mut Vec<(String, bool)>) {
        for inline in inlines {
            match inline {
                parser::Inline::Math { tex, display } => out.push((tex.clone(), *display)),
                parser::Inline::Bold(c)
                | parser::Inline::Italic(c)
                | parser::Inline::Strikethrough(c) => walk(c, out),
                parser::Inline::Link { text, .. } => walk(text, out),
                parser::Inline::Text(_) | parser::Inline::Code(_) => {}
            }
        }
    }
    fn items(list: &[parser::ListItem], out: &mut Vec<(String, bool)>) {
        for item in list {
            walk(&item.inlines, out);
            items(&item.children, out);
        }
    }
    let mut warnings = Vec::new();
    for (i, slide) in presentation.slides.iter().enumerate() {
        let mut found = Vec::new();
        for block in &slide.blocks {
            match block {
                parser::Block::Heading { inlines, .. }
                | parser::Block::Paragraph { inlines }
                | parser::Block::BlockQuote { inlines } => walk(inlines, &mut found),
                parser::Block::List { items: list, .. } => items(list, &mut found),
                parser::Block::Table { headers, rows } => {
                    for cell in headers.iter().chain(rows.iter().flatten()) {
                        walk(cell, &mut found);
                    }
                }
                _ => {}
            }
        }
        for (tex, display) in found {
            if let Err(e) = render::math::lay_out(&tex, display).1 {
                let delim = if display { "$$" } else { "$" };
                warnings.push(CheckWarning {
                    slide: i + 1,
                    category: CheckCategory::Math,
                    message: format!(
                        "formula `{delim}{tex}{delim}` does not parse ({e}); it shows as text"
                    ),
                });
            }
        }
    }
    warnings
}

/// Print the CJK font warning for a deck about to be presented or exported.
pub fn warn_missing_cjk_font(presentation: &parser::Presentation) {
    if let Some(w) = cjk_font_warning(presentation, render::fonts::cjk_coverage()) {
        use colored::Colorize;
        eprintln!("{} {}", "Warning:".yellow().bold(), w.message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn math_warnings_name_the_slide_and_skip_good_formulas() {
        let md = "# Fine $x^2$\n\n- ok $$\\frac{a}{b}$$\n\n---\n\n# Broken\n\n- item $\\frac{1}{$ here\n\n| a | b |\n|---|---|\n| $\\sqrt{$ | 2 |\n";
        let pres = parser::parse(md, std::path::Path::new("."));
        let w = math_warnings(&pres);
        assert_eq!(w.len(), 2, "{w:?}");
        assert!(
            w.iter()
                .all(|w| w.slide == 2 && w.category == CheckCategory::Math)
        );
        assert!(w[0].message.contains("\\frac{1}{"), "{}", w[0].message);
    }
}
