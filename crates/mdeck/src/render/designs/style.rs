//! Role styles as text: the font, size, colour, case and tracking an
//! arrangement gives a role, turned into layout jobs for markdown inlines.

use eframe::egui::{self, Color32, FontFamily, FontId};

use crate::parser::Inline;
use crate::theme::Theme;
use crate::theme::arrangement::{Case, Emphasis, Font, Ink, RoleStyle, Size};

/// A role style resolved against a theme at a scale.
#[derive(Clone)]
pub struct Resolved {
    pub size: f32,
    pub family: FontFamily,
    pub color: Color32,
    pub tracking: f32,
    pub line_height: Option<f32>,
    pub case: Case,
    pub italic: bool,
    pub emphasis: Emphasis,
}

pub fn family(theme: &Theme, font: Font) -> FontFamily {
    match font {
        Font::Display => theme.display_family(),
        Font::Body => theme.body_family(),
        Font::Lead => theme.lead_family(),
        Font::Strong => theme.strong_family(),
        Font::Mono => theme.mono_family(),
    }
}

pub fn ink(theme: &Theme, ink: Ink) -> Color32 {
    match ink {
        Ink::Text => theme.foreground,
        Ink::Heading => theme.heading_color,
        Ink::Muted => theme.muted,
        Ink::Strong => theme.strong,
        Ink::Accent => theme.accent,
        Ink::AccentSoft => theme.accent_soft,
        Ink::Secondary => theme.secondary,
        Ink::Bright => theme.bright_text(),
        Ink::Rule => theme.rule,
    }
}

/// px at 1920x1080 of `size` for a heading of `level` (0: not a heading).
pub fn size_px(theme: &Theme, size: &Size, level: u8) -> f32 {
    match size {
        Size::Px(p) => *p,
        Size::Token(t) => match t.as_str() {
            "h1" => theme.h1_size,
            "h2" => theme.h2_size,
            "h3" => theme.h3_size,
            "code" => theme.code_size,
            "level" if level > 0 => theme.heading_size(level),
            _ => theme.body_size,
        },
    }
}

pub fn resolve(
    theme: &Theme,
    style: &RoleStyle,
    emphasis: Emphasis,
    level: u8,
    scale: f32,
) -> Resolved {
    let c = ink(theme, style.color);
    Resolved {
        size: size_px(theme, &style.size, level) * style.scale * scale,
        family: family(theme, style.font),
        color: fade(c, style.opacity),
        tracking: style.tracking,
        line_height: style.line_height.or(theme.line_height),
        case: style.case,
        italic: style.italic,
        emphasis,
    }
}

pub fn fade(c: Color32, a: f32) -> Color32 {
    let a = a.clamp(0.0, 1.0) * c.a() as f32 / 255.0;
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a * 255.0) as u8)
}

fn cased(s: &str, case: Case) -> String {
    match case {
        Case::None => s.to_string(),
        Case::Upper => s.to_uppercase(),
        Case::Lower => s.to_lowercase(),
    }
}

/// A job for `inlines` in style `r`, wrapped at `width` and aligned by
/// `halign`. Bold runs take the strong face and colour, links the accent,
/// `*em*` slants or turns the soft accent as the arrangement says.
pub fn job(
    theme: &Theme,
    r: &Resolved,
    inlines: &[Inline],
    width: f32,
    halign: egui::Align,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width.max(1.0);
    job.halign = halign;
    let base = egui::text::TextFormat {
        font_id: FontId::new(r.size, r.family.clone()),
        color: r.color,
        extra_letter_spacing: r.size * r.tracking,
        line_height: r.line_height.map(|lh| lh * r.size),
        italics: r.italic,
        ..Default::default()
    };
    append(&mut job, inlines, &base, theme, r);
    crate::render::math::finish(&mut job);
    job
}

/// A job for plain `text` in style `r` (eyebrows, numbers).
pub fn text_job(r: &Resolved, text: &str, halign: egui::Align) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob {
        halign,
        ..Default::default()
    };
    job.append(
        &cased(text, r.case),
        0.0,
        egui::text::TextFormat {
            font_id: FontId::new(r.size, r.family.clone()),
            color: r.color,
            extra_letter_spacing: r.size * r.tracking,
            ..Default::default()
        },
    );
    job
}

fn append(
    job: &mut egui::text::LayoutJob,
    inlines: &[Inline],
    base: &egui::text::TextFormat,
    theme: &Theme,
    r: &Resolved,
) {
    let alpha = base.color.a() as f32 / 255.0;
    for inline in inlines {
        match inline {
            Inline::Text(s) => job.append(&cased(s, r.case), 0.0, base.clone()),
            Inline::Bold(children) => {
                let mut f = base.clone();
                f.font_id = FontId::new(base.font_id.size, theme.strong_family());
                f.color = fade(theme.strong, alpha);
                append(job, children, &f, theme, r);
            }
            Inline::Italic(children) => {
                let mut f = base.clone();
                match r.emphasis {
                    Emphasis::Italic => f.italics = true,
                    Emphasis::Accent => f.color = fade(theme.accent_soft, alpha),
                }
                append(job, children, &f, theme, r);
            }
            Inline::Strikethrough(children) => {
                let mut f = base.clone();
                f.strikethrough =
                    egui::Stroke::new((base.font_id.size * 0.06).max(1.0), base.color);
                append(job, children, &f, theme, r);
            }
            Inline::Code(s) => {
                let mut f = base.clone();
                f.font_id = FontId::new(base.font_id.size * 0.85, theme.mono_family());
                f.extra_letter_spacing = 0.0;
                f.line_height = None;
                f.background = fade(theme.accent, alpha * 0.12);
                crate::render::text::append_code(job, s, f);
            }
            Inline::Link { text, .. } => {
                let mut f = base.clone();
                match r.emphasis {
                    Emphasis::Italic => f.color = fade(theme.accent, alpha),
                    Emphasis::Accent => {
                        f.color = fade(theme.accent_soft, alpha);
                        f.underline = egui::Stroke::new(1.0, fade(theme.accent_soft, alpha * 0.5));
                    }
                }
                append(job, text, &f, theme, r);
            }
            Inline::Math { tex, display } => crate::render::math::append(job, tex, *display, base),
        }
    }
}

/// Roman numeral of `n` (the editorial eyebrow).
pub fn roman(n: usize) -> String {
    const TABLE: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut n = n;
    let mut out = String::new();
    for (v, s) in TABLE {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roman_numerals() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(14), "XIV");
        assert_eq!(roman(40), "XL");
    }

    #[test]
    fn tokens_resolve_against_the_theme() {
        let t = Theme::dark();
        assert_eq!(size_px(&t, &Size::Token("h2".into()), 0), t.h2_size);
        assert_eq!(size_px(&t, &Size::Token("level".into()), 3), t.h3_size);
        assert_eq!(size_px(&t, &Size::Px(17.0), 1), 17.0);
        let style = RoleStyle {
            scale: 0.5,
            size: Size::Token("h1".into()),
            opacity: 0.5,
            color: Ink::Accent,
            ..Default::default()
        };
        let r = resolve(&t, &style, Emphasis::Italic, 0, 2.0);
        assert_eq!(r.size, t.h1_size);
        assert_eq!(r.color.a(), 127);
    }

    #[test]
    fn emphasis_and_case_follow_the_arrangement() {
        let t = Theme::dark();
        let mut r = resolve(&t, &RoleStyle::default(), Emphasis::Accent, 0, 1.0);
        r.case = Case::Upper;
        let j = job(
            &t,
            &r,
            &[
                Inline::Text("ab".into()),
                Inline::Italic(vec![Inline::Text("c".into())]),
            ],
            500.0,
            egui::Align::LEFT,
        );
        assert!(j.text.starts_with("ABC"));
        assert_eq!(j.sections[1].format.color, t.accent_soft);
        assert!(!j.sections[1].format.italics);
    }
}
