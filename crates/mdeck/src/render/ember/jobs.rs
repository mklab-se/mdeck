//! Text jobs in the Ember voice: the tracked monospace eyebrow, the serif
//! display face and the light grotesque lead.

use eframe::egui::{self, Color32};

use super::fade;
use crate::parser::Inline;
use crate::theme::Theme;

/// Tracked uppercase monospace label. `accent_prefix` is drawn in ember.
pub(super) fn eyebrow_job(
    theme: &Theme,
    accent_prefix: &str,
    rest: &str,
    size: f32,
    alpha: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font = egui::FontId::new(size, theme.mono_family());
    let format = |color: Color32| egui::text::TextFormat {
        font_id: font.clone(),
        color,
        extra_letter_spacing: size * 0.22,
        ..Default::default()
    };
    if !accent_prefix.is_empty() {
        job.append(
            &accent_prefix.to_uppercase(),
            0.0,
            format(fade(theme.accent, alpha)),
        );
    }
    job.append(&rest.to_uppercase(), 0.0, format(fade(theme.muted, alpha)));
    job
}

/// Display heading in the serif face; `*emphasis*` turns ember instead of italic.
pub fn display_job(
    inlines: &[Inline],
    size: f32,
    color: Color32,
    width: f32,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    let base = egui::text::TextFormat {
        font_id: egui::FontId::new(size, theme.display_family()),
        color,
        extra_letter_spacing: -size * 0.018,
        line_height: Some(size * 1.06),
        ..Default::default()
    };
    append_display(&mut job, inlines, &base, color, theme);
    crate::render::math::finish(&mut job);
    job
}

fn append_display(
    job: &mut egui::text::LayoutJob,
    inlines: &[Inline],
    base: &egui::text::TextFormat,
    color: Color32,
    theme: &Theme,
) {
    for inline in inlines {
        match inline {
            Inline::Text(s) => job.append(s, 0.0, base.clone()),
            Inline::Bold(children) | Inline::Strikethrough(children) => {
                append_display(job, children, base, color, theme)
            }
            Inline::Italic(children) => {
                let mut f = base.clone();
                f.color = fade(theme.accent_soft, color.a() as f32 / 255.0);
                append_display(job, children, &f, color, theme);
            }
            Inline::Code(s) => {
                let mut f = base.clone();
                f.font_id = egui::FontId::new(base.font_id.size * 0.8, theme.mono_family());
                job.append(s, 0.0, f);
            }
            Inline::Link { text, .. } => append_display(job, text, base, color, theme),
            Inline::Math { tex, display } => crate::render::math::append(job, tex, *display, base),
        }
    }
}

/// Body copy in the grotesque: light weight, relaxed leading, `**strong**`
/// brighter and medium, `*em*` in soft ember (never italic, as on the site).
pub(super) fn lead_job(
    inlines: &[Inline],
    size: f32,
    alpha: f32,
    width: f32,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    let base = egui::text::TextFormat {
        font_id: egui::FontId::new(size, theme.lead_family()),
        color: fade(theme.foreground, alpha),
        line_height: Some(size * 1.5),
        ..Default::default()
    };
    append_lead(&mut job, inlines, &base, alpha, theme);
    crate::render::math::finish(&mut job);
    job
}

/// A list item: the regular body face in a brighter ink than the light
/// paragraphs around it.
pub(super) fn item_job(
    inlines: &[Inline],
    size: f32,
    alpha: f32,
    width: f32,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    let base = egui::text::TextFormat {
        font_id: egui::FontId::new(size, theme.body_family()),
        color: fade(theme.bright_text(), alpha),
        line_height: Some(size * 1.5),
        ..Default::default()
    };
    append_lead(&mut job, inlines, &base, alpha, theme);
    crate::render::math::finish(&mut job);
    job
}

fn append_lead(
    job: &mut egui::text::LayoutJob,
    inlines: &[Inline],
    base: &egui::text::TextFormat,
    alpha: f32,
    theme: &Theme,
) {
    for inline in inlines {
        match inline {
            Inline::Text(s) => job.append(s, 0.0, base.clone()),
            Inline::Bold(children) => {
                let mut f = base.clone();
                f.font_id = egui::FontId::new(base.font_id.size, theme.strong_family());
                f.color = fade(theme.heading_color, alpha);
                append_lead(job, children, &f, alpha, theme);
            }
            Inline::Italic(children) => {
                let mut f = base.clone();
                f.color = fade(theme.accent_soft, alpha);
                append_lead(job, children, &f, alpha, theme);
            }
            Inline::Strikethrough(children) => {
                let mut f = base.clone();
                f.strikethrough = egui::Stroke::new(1.0, base.color);
                append_lead(job, children, &f, alpha, theme);
            }
            Inline::Code(s) => {
                let mut f = base.clone();
                f.font_id = egui::FontId::new(base.font_id.size * 0.86, theme.mono_family());
                f.color = fade(theme.bright_text(), alpha);
                f.background = fade(theme.accent, alpha * 0.10);
                job.append(s, 0.0, f);
            }
            Inline::Link { text, .. } => {
                let mut f = base.clone();
                f.color = fade(theme.accent_soft, alpha);
                f.underline = egui::Stroke::new(1.0, fade(theme.accent_soft, alpha * 0.5));
                append_lead(job, text, &f, alpha, theme);
            }
            Inline::Math { tex, display } => crate::render::math::append(job, tex, *display, base),
        }
    }
}
