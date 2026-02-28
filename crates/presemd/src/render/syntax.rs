use std::sync::LazyLock;

use eframe::egui::{self, Color32, FontFamily, FontId};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use crate::theme::Theme;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Create a syntax-highlighted `LayoutJob` for a code block.
pub fn highlight_code(
    code: &str,
    language: Option<&str>,
    font_size: f32,
    opacity: f32,
    theme: &Theme,
    max_width: f32,
) -> egui::text::LayoutJob {
    let ss = &*SYNTAX_SET;
    let ts = &*THEME_SET;

    let syntax = language
        .and_then(|lang| ss.find_syntax_by_token(lang))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let theme_name = theme.syntect_theme_name();
    let syntect_theme = ts
        .themes
        .get(theme_name)
        .unwrap_or_else(|| ts.themes.values().next().unwrap());

    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = max_width;

    let mut highlighter = HighlightLines::new(syntax, syntect_theme);

    for line in code.lines() {
        let ranges = highlighter
            .highlight_line(line, ss)
            .unwrap_or_else(|_| vec![]);

        for (style, text) in ranges {
            let fg = Color32::from_rgba_unmultiplied(
                style.foreground.r,
                style.foreground.g,
                style.foreground.b,
                (opacity * style.foreground.a as f32 / 255.0 * 255.0) as u8,
            );
            let format = egui::text::TextFormat {
                font_id: FontId::new(font_size, FontFamily::Monospace),
                color: fg,
                ..Default::default()
            };
            job.append(text, 0.0, format);
        }

        // Add newline between lines
        let nl_format = egui::text::TextFormat {
            font_id: FontId::new(font_size, FontFamily::Monospace),
            color: Color32::TRANSPARENT,
            ..Default::default()
        };
        job.append("\n", 0.0, nl_format);
    }

    job
}
