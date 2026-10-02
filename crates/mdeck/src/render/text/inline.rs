//! Inline runs (bold, links, code, math) laid out into jobs, and the blocks
//! that are a single run of them: headings, paragraphs and block quotes.

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Stroke};

use crate::parser::Inline;
use crate::render::TextCx;
use crate::theme::Theme;

/// Colours used when laying out inline runs. Derived from the theme and the
/// caller's base colour so that fade opacity carries through to every run.
#[derive(Clone)]
struct InlineStyle {
    /// Base text colour (already carries the fade opacity in its alpha).
    color: Color32,
    /// Colour for `**bold**` runs. Only a light face is bundled, so bold is
    /// emphasised with a brighter colour in addition to the size bump.
    strong: Color32,
    /// Colour for link text.
    link: Color32,
    /// Background tint for inline code.
    code_bg: Color32,
    /// Family for ordinary runs.
    body_family: FontFamily,
    /// Family for `**bold**` runs (a medium face where the theme bundles one).
    strong_family: FontFamily,
    /// Family for inline code.
    mono_family: FontFamily,
    /// Line height as a multiple of the font size, when the theme sets one.
    line_height: Option<f32>,
}

impl InlineStyle {
    fn new(theme: &Theme, color: Color32) -> Self {
        Self::with_body(theme, color, theme.body_family())
    }

    /// The same style with `body` as the family for ordinary runs.
    fn with_body(theme: &Theme, color: Color32, body: FontFamily) -> Self {
        let alpha = color.a();
        Self {
            color,
            strong: with_alpha(strong_color(theme), alpha),
            link: with_alpha(theme.accent, alpha),
            code_bg: with_alpha(theme.accent, (alpha as f32 * 0.12) as u8),
            body_family: body,
            strong_family: theme.strong_family(),
            mono_family: theme.mono_family(),
            line_height: theme.line_height,
        }
    }
}

fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Colour used to emphasise bold text: the theme's `colors.strong`, which
/// defaults to the heading colour when it is visibly brighter than body text
/// (dark themes) and to the accent otherwise (light themes).
pub fn strong_color(theme: &Theme) -> Color32 {
    theme.strong
}

/// Create a LayoutJob from inline elements.
pub fn inlines_to_job(
    inlines: &[Inline],
    font_size: f32,
    color: Color32,
    max_width: f32,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = max_width;
    let style = InlineStyle::new(theme, color);
    append_inlines(&mut job, inlines, font_size, &style, false, false);
    crate::render::math::finish(&mut job);
    job
}

fn append_inlines(
    job: &mut egui::text::LayoutJob,
    inlines: &[Inline],
    font_size: f32,
    style: &InlineStyle,
    bold: bool,
    italic: bool,
) {
    for inline in inlines {
        match inline {
            Inline::Text(s) => job.append(s, 0.0, text_format(font_size, style, bold, italic)),
            Inline::Bold(children) => {
                append_inlines(job, children, font_size, style, true, italic);
            }
            Inline::Italic(children) => {
                append_inlines(job, children, font_size, style, bold, true);
            }
            Inline::Strikethrough(children) => {
                let mut inner_job = egui::text::LayoutJob::default();
                append_inlines(&mut inner_job, children, font_size, style, bold, italic);
                // Apply strikethrough to all sections
                for section in &inner_job.sections {
                    let mut format = section.format.clone();
                    format.strikethrough = Stroke::new((font_size * 0.06).max(1.0), format.color);
                    job.append(
                        &inner_job.text[section.byte_range.start.0..section.byte_range.end.0],
                        0.0,
                        format,
                    );
                }
            }
            Inline::Code(s) => {
                let format = egui::text::TextFormat {
                    font_id: FontId::new(font_size * 0.85, style.mono_family.clone()),
                    color: style.color,
                    background: style.code_bg,
                    line_height: style.line_height.map(|lh| lh * font_size),
                    ..Default::default()
                };
                job.append(s, 0.0, format);
            }
            Inline::Link { text, .. } => {
                // Render link text in the theme accent colour
                let link_style = InlineStyle {
                    color: style.link,
                    strong: style.link,
                    ..style.clone()
                };
                append_inlines(job, text, font_size, &link_style, bold, italic);
            }
            Inline::Math { tex, display } => {
                let format = egui::text::TextFormat {
                    font_id: FontId::new(font_size, style.body_family.clone()),
                    color: style.color,
                    line_height: style.line_height.map(|lh| lh * font_size),
                    ..Default::default()
                };
                crate::render::math::append(job, tex, *display, &format);
            }
        }
    }
}

/// Format of a plain text run: bold runs take the strong family and colour,
/// a point larger unless the theme fixes the line height.
fn text_format(
    font_size: f32,
    style: &InlineStyle,
    bold: bool,
    italic: bool,
) -> egui::text::TextFormat {
    let (size, color, family) = if bold {
        let bump = if style.line_height.is_some() {
            0.0
        } else {
            1.0
        };
        (font_size + bump, style.strong, style.strong_family.clone())
    } else {
        (font_size, style.color, style.body_family.clone())
    };
    egui::text::TextFormat {
        font_id: FontId::new(size, family),
        color,
        italics: italic,
        line_height: style.line_height.map(|lh| lh * font_size),
        ..Default::default()
    }
}

/// Measure the height of inlines laid out at `max_width` without painting.
pub(super) fn measure_inlines(
    ui: &egui::Ui,
    inlines: &[Inline],
    font_size: f32,
    max_width: f32,
    theme: &Theme,
) -> f32 {
    let job = inlines_to_job(inlines, font_size, theme.foreground, max_width, theme);
    ui.painter().layout_job(job).rect.height()
}

/// Layout and paint inlines, returning the height used.
pub fn draw_inlines(
    cx: &TextCx,
    inlines: &[Inline],
    pos: Pos2,
    font_size: f32,
    color: Color32,
    max_width: f32,
) -> f32 {
    let job = inlines_to_job(inlines, font_size, color, max_width, cx.theme);
    let galley = cx.ui.painter().layout_job(job);
    let height = galley.rect.height();
    crate::render::math::galley(cx.ui.painter(), pos, galley, color);
    height
}

/// Layout job for a heading: the theme's display face (Ember's serif) at the
/// heading size. Drawing and measuring both go through here.
pub fn heading_job(
    inlines: &[Inline],
    level: u8,
    theme: &Theme,
    color: Color32,
    max_width: f32,
    scale: f32,
) -> egui::text::LayoutJob {
    let size = theme.heading_size(level) * scale;
    if crate::theme::uses_editorial(theme) {
        return crate::render::ember::display_job(inlines, size, color, max_width, theme);
    }
    display_inlines_job(inlines, size, color, max_width, theme)
}

/// Headline text (title, section and quote headings) in the theme's
/// display face; runs otherwise styled like body text.
pub fn display_inlines_job(
    inlines: &[Inline],
    size: f32,
    color: Color32,
    max_width: f32,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = max_width;
    let style = InlineStyle::with_body(theme, color, theme.display_family());
    append_inlines(&mut job, inlines, size, &style, false, false);
    crate::render::math::finish(&mut job);
    job
}

/// Draw a heading block. Returns height used.
pub fn draw_heading(cx: &TextCx, inlines: &[Inline], level: u8, pos: Pos2, max_width: f32) -> f32 {
    let color = Theme::with_opacity(cx.theme.heading_color, cx.opacity);
    let job = heading_job(inlines, level, cx.theme, color, max_width, cx.scale);
    let galley = cx.ui.painter().layout_job(job);
    let height = galley.rect.height();
    crate::render::math::galley(cx.ui.painter(), pos, galley, color);
    height
}

/// Draw a paragraph. Returns height used.
pub fn draw_paragraph(cx: &TextCx, inlines: &[Inline], pos: Pos2, max_width: f32) -> f32 {
    let color = Theme::with_opacity(cx.theme.foreground, cx.opacity);
    let size = cx.theme.body_size * cx.scale;
    draw_inlines(cx, inlines, pos, size, color, max_width)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Vec<Inline> {
        vec![Inline::Text(s.to_string())]
    }

    #[test]
    fn bold_and_links_use_theme_colours_with_incoming_alpha() {
        let theme = Theme::dark();
        let faded = Color32::from_rgba_unmultiplied(200, 200, 200, 128);
        let inlines = vec![
            Inline::Bold(text("bold")),
            Inline::Link {
                text: text("link"),
                url: "https://example.com".into(),
            },
            Inline::Code("code".into()),
        ];
        let job = inlines_to_job(&inlines, 20.0, faded, 1000.0, &theme);
        let bold = &job.sections[0].format;
        assert_eq!(bold.color, with_alpha(theme.heading_color, 128));
        assert_eq!(bold.font_id.size, 21.0);
        let link = &job.sections[1].format;
        assert_eq!(link.color, with_alpha(theme.accent, 128));
        let code = &job.sections[2].format;
        assert_eq!(
            code.background,
            with_alpha(theme.accent, (128.0 * 0.12) as u8)
        );
    }

    #[test]
    fn strong_colour_uses_heading_only_when_visibly_brighter() {
        // Dark: white headings vs grey body, clearly brighter.
        assert_eq!(strong_color(&Theme::dark()), Theme::dark().heading_color);
        // Light and nord: heading and body colours are nearly identical, so
        // the accent is used to make bold visible.
        assert_eq!(strong_color(&Theme::light()), Theme::light().accent);
        assert_eq!(strong_color(&Theme::nord()), Theme::nord().accent);
    }

    #[test]
    fn bold_runs_bump_the_size_only_without_a_fixed_line_height() {
        let theme = Theme::dark();
        let mut style = InlineStyle::new(&theme, Color32::WHITE);
        style.line_height = None;
        let plain = text_format(20.0, &style, false, true);
        assert_eq!(plain.font_id.size, 20.0);
        assert!(plain.italics);
        assert_eq!(plain.color, style.color);
        let bold = text_format(20.0, &style, true, false);
        assert_eq!(bold.font_id.size, 21.0);
        assert_eq!(bold.color, style.strong);
        assert_eq!(bold.font_id.family, style.strong_family);

        style.line_height = Some(1.5);
        let bold = text_format(20.0, &style, true, false);
        assert_eq!(bold.font_id.size, 20.0);
        assert_eq!(bold.line_height, Some(30.0));
    }
}
