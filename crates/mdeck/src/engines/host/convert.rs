//! mdeck's own types as the SDK's: the parsed slide as
//! [`mdeck_sdk::content::Slide`], the theme's colours as
//! [`mdeck_sdk::tokens::Tokens`], the geometry renderers publish as
//! [`mdeck_sdk::geometry::Hint`]. Extensions and the built-in engines only
//! ever see the SDK side.

use eframe::egui;
use mdeck_sdk::content as sdk;
use mdeck_sdk::geometry::Hint as SdkHint;
use mdeck_sdk::host as h;
use mdeck_sdk::paint::{Font, FontRole};
use mdeck_sdk::tokens::Tokens;

use crate::parser;
use crate::render::hints::Hint;
use crate::theme::Theme;

/// The slide as the SDK's content model.
pub fn slide(s: &parser::Slide) -> sdk::Slide {
    let mut out = sdk::Slide::new(s.design.name());
    out.directives = s
        .settings
        .iter()
        .map(|d| sdk::Directive::new(d.name.clone(), d.value.clone(), d.line))
        .collect();
    out.blocks = blocks(&s.blocks);
    out.raw_source = s.raw_source.clone();
    out.line = s.line;
    out.source_lines = s.source_lines.clone();
    out.notes = s.notes.clone();
    out.picture = s.illustration.clone();
    out.logo = s.logo.clone();
    out.art = s.art.clone();
    out
}

fn blocks(bs: &[parser::Block]) -> Vec<sdk::Block> {
    bs.iter().map(block).collect()
}

fn inlines(v: &[parser::Inline]) -> Vec<sdk::Inline> {
    v.iter().map(inline).collect()
}

fn block(b: &parser::Block) -> sdk::Block {
    use parser::Block as B;
    match b {
        B::Heading { level, inlines: i } => sdk::Block::heading(*level, inlines(i)),
        B::Paragraph { inlines: i } => sdk::Block::paragraph(inlines(i)),
        B::List {
            ordered,
            start,
            items,
        } => h::list(*ordered, *start, items.iter().map(item).collect()),
        B::Image {
            alt,
            path,
            directives,
        } => h::image(
            alt.clone(),
            path.clone(),
            directives.width.clone(),
            directives.height.clone(),
            directives.fill,
        ),
        B::CodeBlock {
            language,
            code,
            highlight_lines,
        } => h::code_block(language.clone(), code.clone(), highlight_lines.clone()),
        B::BlockQuote { blocks: inner } => sdk::Block::quote(blocks(inner)),
        B::Callout {
            kind,
            blocks: inner,
        } => sdk::Block::callout(callout(*kind), blocks(inner)),
        B::Table {
            headers,
            align,
            rows,
        } => h::table(
            headers.iter().map(|c| inlines(c)).collect(),
            align.iter().map(|a| self::align(*a)).collect(),
            rows.iter()
                .map(|r| r.iter().map(|c| inlines(c)).collect())
                .collect(),
        ),
        B::HorizontalRule => sdk::Block::HorizontalRule,
        B::Diagram { content, step_base } => {
            h::visual("architecture".into(), content.clone(), *step_base)
        }
        B::Chart {
            kind,
            content,
            step_base,
        } => h::visual(kind.tag().to_string(), content.clone(), *step_base),
        B::ColumnSeparator => sdk::Block::ColumnSeparator,
    }
}

fn callout(a: parser::Alert) -> sdk::CalloutKind {
    use parser::Alert as A;
    match a {
        A::Note => sdk::CalloutKind::Note,
        A::Tip => sdk::CalloutKind::Tip,
        A::Important => sdk::CalloutKind::Important,
        A::Warning => sdk::CalloutKind::Warning,
        A::Caution => sdk::CalloutKind::Caution,
    }
}

fn align(a: parser::Align) -> sdk::Align {
    match a {
        parser::Align::Left => sdk::Align::Left,
        parser::Align::Center => sdk::Align::Center,
        parser::Align::Right => sdk::Align::Right,
    }
}

fn item(i: &parser::ListItem) -> sdk::ListItem {
    let marker = match i.marker {
        parser::ListMarker::Static => sdk::ListMarker::Static,
        parser::ListMarker::NextStep => sdk::ListMarker::NextStep,
        parser::ListMarker::Ordered => sdk::ListMarker::Ordered,
    };
    let mut out = sdk::ListItem::new(marker, inlines(&i.inlines));
    out.children = i.children.iter().map(item).collect();
    out.step = i.step;
    out.checked = i.checked;
    out
}

fn inline(i: &parser::Inline) -> sdk::Inline {
    use parser::Inline as I;
    match i {
        I::Text(s) => sdk::Inline::Text(s.clone()),
        I::Bold(v) => sdk::Inline::Bold(inlines(v)),
        I::Italic(v) => sdk::Inline::Italic(inlines(v)),
        I::Strikethrough(v) => sdk::Inline::Strikethrough(inlines(v)),
        I::Code(s) => sdk::Inline::Code(s.clone()),
        I::Math { tex, display } => sdk::Inline::math(tex.clone(), *display),
        I::Link { text, url } => sdk::Inline::link(inlines(text), url.clone()),
    }
}

/// The theme's colours as tokens.
pub fn tokens(theme: &Theme) -> Tokens {
    let c = h::color;
    let mut t = Tokens::default();
    t.background = c(theme.background);
    t.text = c(theme.foreground);
    t.heading = c(theme.heading_color);
    t.accent = c(theme.accent);
    t.accent_soft = c(theme.accent_soft);
    t.secondary = c(theme.secondary);
    t.muted = c(theme.muted);
    t.rule = c(theme.rule);
    t.code_background = c(theme.code_background);
    t.code_text = c(theme.code_foreground);
    t.positive = c(theme.positive);
    t.negative = c(theme.negative);
    t.series = theme.series.map(c);
    t.light = theme.is_light();
    t.particle_light = c(theme.particle_light);
    t.particle_cool = c(theme.particle_cool);
    t
}

/// The egui family names of the theme's faces, for the SDK's font roles.
pub fn font_families(theme: &Theme) -> h::FontFamilyNames {
    let name = |f: &egui::FontFamily| match f {
        egui::FontFamily::Name(n) => Some(n.to_string()),
        _ => None,
    };
    h::FontFamilyNames {
        display: name(&theme.fonts.display),
        body: name(&theme.fonts.body),
        lead: name(&theme.fonts.lead),
        strong: name(&theme.fonts.strong),
        mono: name(&theme.fonts.mono),
    }
}

/// A published hint as the SDK's (see [`hints`]).
pub fn hint(hint: &Hint, theme: &Theme) -> SdkHint {
    match hint {
        Hint::Bar(r) => SdkHint::Bar(h::rect(*r)),
        Hint::Path(pts) => SdkHint::Path(pts.iter().map(|p| h::pos(*p)).collect()),
        Hint::Circle { center, radius } => SdkHint::Circle {
            center: h::pos(*center),
            radius: *radius,
        },
        Hint::Point(p) => SdkHint::Point(h::pos(*p)),
        Hint::Frame(r) => SdkHint::Frame(h::rect(*r)),
        Hint::Copy(r) => SdkHint::Copy(h::rect(*r)),
        Hint::Text { galley, pos, slide } => {
            let (font, color) = heading_font(galley, theme);
            SdkHint::text(galley.text(), font, h::pos(*pos), color, *slide)
        }
    }
}

/// The published hints as the SDK's. A heading becomes one
/// [`SdkHint::Text`] per glyph, each placed where the galley put that
/// glyph: [`SdkHint::Text`] carries text and a font but not the layout
/// (wrapping, the display face's tight letter spacing, line height), so a
/// whole heading laid out again on one line would drift from the type the
/// slide draws. Glyph by glyph, an engine that rasterises the heading
/// (thermal's cold opening, [`mdeck_sdk::paint::Painter::glyph_ink`]) lands
/// on the drawn letters exactly.
pub fn hints(ctx: &egui::Context, hint: &Hint, theme: &Theme) -> Vec<SdkHint> {
    let Hint::Text { galley, pos, slide } = hint else {
        return vec![self::hint(hint, theme)];
    };
    let Some(format) = galley.job.sections.first().map(|s| &s.format) else {
        return vec![self::hint(hint, theme)];
    };
    let (font, color) = heading_font(galley, theme);
    let font_id = format.font_id.clone();
    let mut out = Vec::new();
    ctx.fonts_mut(|f| {
        for row in &galley.rows {
            for g in &row.glyphs {
                if g.uv_rect.is_nothing() || g.chr.is_whitespace() {
                    continue;
                }
                // where a glyph laid out alone sits in its own galley
                let alone =
                    f.layout_no_wrap(g.chr.to_string(), font_id.clone(), egui::Color32::WHITE);
                let Some((arow, ag)) = alone
                    .rows
                    .first()
                    .and_then(|r| r.glyphs.first().map(|g| (r, g)))
                else {
                    continue;
                };
                let at = *pos + row.pos.to_vec2() + g.pos.to_vec2()
                    - arow.pos.to_vec2()
                    - ag.pos.to_vec2();
                out.push(SdkHint::text(
                    g.chr.to_string(),
                    font,
                    h::pos(at),
                    color,
                    *slide,
                ));
            }
        }
    });
    out
}

/// A heading galley's face (as a role), size and colour, from its first
/// section.
fn heading_font(galley: &egui::Galley, theme: &Theme) -> (Font, mdeck_sdk::paint::Color) {
    let format = galley.job.sections.first().map(|s| &s.format);
    let size = format.map_or(theme.h1_size, |f| f.font_id.size);
    let family = format.map(|f| &f.font_id.family);
    let role = if family == Some(&theme.fonts.display) {
        FontRole::Display
    } else if family == Some(&theme.fonts.mono) {
        FontRole::Mono
    } else if family == Some(&theme.fonts.strong) {
        FontRole::Strong
    } else if family == Some(&theme.fonts.lead) {
        FontRole::Lead
    } else {
        FontRole::Body
    };
    let color = format.map_or(theme.heading_color, |f| f.color);
    (Font::new(role, size), h::color(color))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_parsed_slide_keeps_its_steps_and_visual_tags() {
        let pres = crate::parser::parse("# A\n\n- a\n+ b\n\n```@bar\nA: 1\n```\n");
        let s = slide(&pres.slides[0]);
        let sdk::Block::List { items, .. } = &s.blocks[1] else {
            panic!("{:?}", s.blocks);
        };
        assert_eq!((items[0].step, items[1].step), (0, 1));
        assert!(
            matches!(&s.blocks[2], sdk::Block::Visual { tag, .. } if tag == "bar"),
            "{:?}",
            s.blocks[2]
        );
        assert_eq!(s.title().as_deref(), Some("A"));
    }

    #[test]
    fn tokens_follow_the_theme() {
        let t = Theme::dark();
        let k = tokens(&t);
        assert_eq!(k.accent, h::color(t.accent));
        assert_eq!(k.light, t.is_light());
    }
}
