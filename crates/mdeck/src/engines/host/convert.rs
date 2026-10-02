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
    sdk::Slide {
        directives: s
            .settings
            .iter()
            .map(|d| sdk::Directive {
                name: d.name.clone(),
                value: d.value.clone(),
                line: d.line,
            })
            .collect(),
        blocks: blocks(&s.blocks),
        design: s.design.name().to_string(),
        raw_source: s.raw_source.clone(),
        line: s.line,
        source_lines: s.source_lines.clone(),
        notes: s.notes.clone(),
        illustration: s.illustration.clone(),
        logo: s.logo.clone(),
        art: s.art.clone(),
    }
}

fn blocks(bs: &[parser::Block]) -> Vec<sdk::Block> {
    bs.iter().map(block).collect()
}

/// The inlines of quoted blocks, joined by line breaks: the SDK's quote is
/// one run of text.
fn quoted(bs: &[parser::Block]) -> Vec<sdk::Inline> {
    let mut out = Vec::new();
    for b in bs {
        let part = match b {
            parser::Block::Paragraph { inlines } | parser::Block::Heading { inlines, .. } => {
                inlines.iter().map(inline).collect()
            }
            parser::Block::BlockQuote { blocks } | parser::Block::Callout { blocks, .. } => {
                quoted(blocks)
            }
            parser::Block::List { items, .. } => items
                .iter()
                .flat_map(|i| i.inlines.iter().map(inline))
                .collect(),
            _ => Vec::new(),
        };
        if part.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(sdk::Inline::Text("\n".into()));
        }
        out.extend(part);
    }
    out
}

fn block(b: &parser::Block) -> sdk::Block {
    use parser::Block as B;
    match b {
        B::Heading { level, inlines } => sdk::Block::Heading {
            level: *level,
            inlines: inlines.iter().map(inline).collect(),
        },
        B::Paragraph { inlines } => sdk::Block::Paragraph {
            inlines: inlines.iter().map(inline).collect(),
        },
        B::List { ordered, items, .. } => sdk::Block::List {
            ordered: *ordered,
            items: items.iter().map(item).collect(),
        },
        B::Image {
            alt,
            path,
            directives,
        } => sdk::Block::Image {
            alt: alt.clone(),
            path: path.clone(),
            directives: sdk::ImageDirectives {
                width: directives.width.clone(),
                height: directives.height.clone(),
                fill: directives.fill,
                fit: false,
                align: None,
            },
        },
        B::CodeBlock {
            language,
            code,
            highlight_lines,
        } => sdk::Block::CodeBlock {
            language: language.clone(),
            code: code.clone(),
            highlight_lines: highlight_lines.clone(),
        },
        B::BlockQuote { blocks } | B::Callout { blocks, .. } => sdk::Block::BlockQuote {
            inlines: quoted(blocks),
        },
        B::Table { headers, rows, .. } => sdk::Block::Table {
            headers: headers
                .iter()
                .map(|c| c.iter().map(inline).collect())
                .collect(),
            rows: rows
                .iter()
                .map(|r| r.iter().map(|c| c.iter().map(inline).collect()).collect())
                .collect(),
        },
        B::HorizontalRule => sdk::Block::HorizontalRule,
        B::Diagram { content, step_base } => sdk::Block::Visual {
            tag: "architecture".into(),
            content: content.clone(),
            step_base: *step_base,
        },
        B::Chart {
            kind,
            content,
            step_base,
        } => sdk::Block::Visual {
            tag: kind.tag().to_string(),
            content: content.clone(),
            step_base: *step_base,
        },
        B::ColumnSeparator => sdk::Block::ColumnSeparator,
    }
}

fn item(i: &parser::ListItem) -> sdk::ListItem {
    sdk::ListItem {
        marker: match i.marker {
            parser::ListMarker::Static => sdk::ListMarker::Static,
            parser::ListMarker::NextStep => sdk::ListMarker::NextStep,
            parser::ListMarker::Ordered => sdk::ListMarker::Ordered,
        },
        inlines: i.inlines.iter().map(inline).collect(),
        children: i.children.iter().map(item).collect(),
        step: i.step,
    }
}

fn inline(i: &parser::Inline) -> sdk::Inline {
    use parser::Inline as I;
    match i {
        I::Text(s) => sdk::Inline::Text(s.clone()),
        I::Bold(v) => sdk::Inline::Bold(v.iter().map(inline).collect()),
        I::Italic(v) => sdk::Inline::Italic(v.iter().map(inline).collect()),
        I::Strikethrough(v) => sdk::Inline::Strikethrough(v.iter().map(inline).collect()),
        I::Code(s) => sdk::Inline::Code(s.clone()),
        I::Math { tex, display } => sdk::Inline::Math {
            tex: tex.clone(),
            display: *display,
        },
        I::Link { text, url } => sdk::Inline::Link {
            text: text.iter().map(inline).collect(),
            url: url.clone(),
        },
    }
}

/// The theme's colours as tokens.
pub fn tokens(theme: &Theme) -> Tokens {
    let c = h::color;
    Tokens {
        background: c(theme.background),
        text: c(theme.foreground),
        heading: c(theme.heading_color),
        accent: c(theme.accent),
        accent_soft: c(theme.accent_soft),
        secondary: c(theme.secondary),
        muted: c(theme.muted),
        rule: c(theme.rule),
        code_background: c(theme.code_background),
        code_text: c(theme.code_foreground),
        positive: c(theme.positive),
        negative: c(theme.negative),
        series: theme.series.map(c),
        light: theme.is_light(),
        particle_light: c(theme.particle_light),
        particle_cool: c(theme.particle_cool),
    }
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
        Hint::Text { galley, pos, slide } => {
            let (font, color) = heading_font(galley, theme);
            SdkHint::Text {
                text: galley.text().to_string(),
                font,
                pos: h::pos(*pos),
                color,
                slide: *slide,
            }
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
                out.push(SdkHint::Text {
                    text: g.chr.to_string(),
                    font,
                    pos: h::pos(at),
                    color,
                    slide: *slide,
                });
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
