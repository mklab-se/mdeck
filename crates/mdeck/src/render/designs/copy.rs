//! The copy stack: a design's text roles laid out top to bottom in a column,
//! as pieces the painter draws (and measurement adds up) one by one.

use std::sync::Arc;

use eframe::egui::{self, Color32, Pos2, Rect};

use super::parts::{Item, Role};
use super::style::{self, Resolved, fade, roman};
use crate::parser::{Block, Inline, ListItem, ListMarker};
use crate::render::SlideContext;
use crate::theme::Theme;
use crate::theme::arrangement::{Arrangement, Bar, Eyebrow, HAlign, RoleStyle};

/// What a layout pass works with.
pub struct Lay<'a> {
    pub ui: &'a egui::Ui,
    pub theme: &'a Theme,
    pub a: &'a Arrangement,
    pub scale: f32,
    pub deck: &'a SlideContext,
    /// The slide rect (inline visuals size against it).
    pub rect: Rect,
}

impl Lay<'_> {
    pub fn style(&self, role: Role) -> &RoleStyle {
        let r = &self.a.roles;
        match role {
            Role::Title => &r.title,
            Role::Subtitle => &r.subtitle,
            Role::Kicker => &r.kicker,
            Role::Heading => &r.heading,
            Role::Statement => &r.statement,
            Role::Lead => &r.lead,
            Role::Body | Role::Block => &r.body,
            Role::List => &r.list,
            Role::Quote => &r.quote,
            Role::Attribution => &r.attribution,
            Role::Caption => &r.caption,
        }
    }

    pub fn resolve(&self, style: &RoleStyle, level: u8) -> Resolved {
        style::resolve(
            self.theme,
            style,
            self.a.ornaments.emphasis,
            level,
            self.scale,
        )
    }

    /// The gap after an element in `style` whose size is `size` px.
    pub fn gap(&self, style: &RoleStyle, size: f32) -> f32 {
        self.theme.spacing.px(&style.gap, size / self.scale) * self.scale
    }

    pub fn space(&self, space: &crate::theme::arrangement::Space) -> f32 {
        self.theme.spacing.px(space, self.theme.body_size) * self.scale
    }
}

/// What stands before a list item.
#[derive(Clone)]
pub enum Mark {
    Glyph(Arc<egui::Galley>, Pos2),
    Dot {
        center: Pos2,
        radius: f32,
        color: Color32,
    },
    Task {
        rect: Rect,
        checked: bool,
    },
}

#[derive(Clone)]
pub enum Kind<'s> {
    Text {
        galley: Arc<egui::Galley>,
        /// Where the galley is painted.
        anchor: Pos2,
        mark: Option<Mark>,
        /// A bar down the left (x, colour, width).
        bar: Option<(f32, Color32, f32)>,
        /// An accent hairline under it (left, width).
        rule: Option<(Pos2, f32)>,
        /// The slide's title (an engine with a cold opening forms it).
        title: bool,
    },
    /// A block the block renderer draws in `rect`.
    Block { block: &'s Block, rect: Rect },
    /// The bar down the left of a quote with structure, spanning all of it.
    Bar { rect: Rect, color: Color32 },
}

#[derive(Clone)]
pub struct Piece<'s> {
    pub kind: Kind<'s>,
    /// Where it is drawn.
    pub bounds: Rect,
    /// Reveal step (0: always shown).
    pub step: usize,
    /// Its place in the entry stagger.
    pub nth: usize,
}

impl Piece<'_> {
    pub fn translate(&mut self, d: egui::Vec2) {
        self.bounds = self.bounds.translate(d);
        match &mut self.kind {
            Kind::Text {
                anchor,
                mark,
                bar,
                rule,
                ..
            } => {
                *anchor += d;
                if let Some((x, _, _)) = bar {
                    *x += d.x;
                }
                if let Some((p, _)) = rule {
                    *p += d;
                }
                match mark {
                    Some(Mark::Glyph(_, p)) => *p += d,
                    Some(Mark::Dot { center, .. }) => *center += d,
                    Some(Mark::Task { rect, .. }) => *rect = rect.translate(d),
                    None => {}
                }
            }
            Kind::Block { rect, .. } | Kind::Bar { rect, .. } => *rect = rect.translate(d),
        }
    }
}

/// Pieces stacked from y = 0 in a column at `x` of `width`.
pub struct Stack<'s> {
    pub pieces: Vec<Piece<'s>>,
    y: f32,
    pending: f32,
    x: f32,
    width: f32,
    align: HAlign,
    last_quote_right: Option<f32>,
}

impl<'s> Stack<'s> {
    pub fn new(x: f32, width: f32, align: HAlign) -> Self {
        Stack {
            pieces: Vec::new(),
            y: 0.0,
            pending: 0.0,
            x,
            width,
            align,
            last_quote_right: None,
        }
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    /// The gap the last element asks for after it.
    pub fn trailing_gap(&self) -> f32 {
        self.pending
    }

    /// Height of the stack (no trailing gap).
    pub fn height(&self) -> f32 {
        self.y
    }

    fn top(&mut self) -> f32 {
        self.y += self.pending;
        self.pending = 0.0;
        self.y
    }

    fn push(&mut self, piece: Piece<'s>, gap: f32) {
        self.y = self.y.max(piece.bounds.bottom());
        self.pending = gap;
        self.pieces.push(piece);
    }

    /// Where a galley of `align` is painted so it sits in the column.
    fn anchor_x(&self, align: HAlign) -> f32 {
        match align {
            HAlign::Left => self.x,
            HAlign::Center => self.x + self.width / 2.0,
            HAlign::Right => self.x + self.width,
        }
    }

    /// A text piece in `role` (heading level `level`).
    fn text(&mut self, lay: &Lay, role: Role, level: u8, inlines: &[Inline], step: usize) -> Rect {
        let style = lay.style(role);
        let r = lay.resolve(style, level);
        let align = style.align.unwrap_or(self.align);
        let job = style::job(lay.theme, &r, inlines, self.width, h(align));
        let galley = lay.ui.painter().layout_job(job);
        let y = self.top();
        let mut x = self.anchor_x(align);
        if role == Role::Attribution
            && align == HAlign::Right
            && let Some(right) = self.last_quote_right
        {
            x = right;
        }
        let anchor = Pos2::new(x, y);
        let bounds = galley.rect.translate(anchor.to_vec2());
        let o = &lay.a.ornaments;
        let bar = (role == Role::Quote && o.quote_bar == Bar::Left).then(|| {
            (
                bounds.left() - 24.0 * lay.scale,
                style::ink(lay.theme, o.bar_color),
                o.bar_width * lay.scale,
            )
        });
        let title = role == Role::Title;
        let rule = (title && o.title_rule).then(|| {
            let w = 64.0 * lay.scale;
            let left = match align {
                HAlign::Left => bounds.left(),
                HAlign::Center => bounds.center().x - w / 2.0,
                HAlign::Right => bounds.right() - w,
            };
            (Pos2::new(left, bounds.bottom() + 14.0 * lay.scale), w)
        });
        if role == Role::Quote {
            self.last_quote_right = Some(bounds.right());
        }
        let extra = if rule.is_some() {
            28.0 * lay.scale
        } else {
            0.0
        };
        let gap = lay.gap(style, r.size) + extra;
        let nth = self.pieces.len();
        self.push(
            Piece {
                kind: Kind::Text {
                    galley,
                    anchor,
                    mark: None,
                    bar,
                    rule,
                    title,
                },
                bounds,
                step,
                nth,
            },
            gap,
        );
        bounds
    }

    /// The eyebrow over the copy.
    pub fn eyebrow(&mut self, lay: &Lay) {
        let (accent, rest) = match lay.a.eyebrow {
            Eyebrow::None => return,
            Eyebrow::Numeral => numeral_eyebrow(lay.deck),
            Eyebrow::Deck => (String::new(), deck_eyebrow(lay.deck)),
        };
        let style = &lay.a.roles.eyebrow;
        let r = lay.resolve(style, 0);
        let align = style.align.unwrap_or(self.align);
        let mut job = style::text_job(&r, &rest, h(align));
        if !accent.is_empty() {
            let mut prefix = style::text_job(&r, &accent, h(align));
            for s in &mut prefix.sections {
                s.format.color = style::ink(lay.theme, crate::theme::arrangement::Ink::Accent);
            }
            let offset = prefix.text.len();
            prefix.text.push_str(&job.text);
            for mut s in job.sections {
                s.byte_range = (s.byte_range.start + offset)..(s.byte_range.end + offset);
                prefix.sections.push(s);
            }
            job = prefix;
        }
        let galley = lay.ui.painter().layout_job(job);
        let y = self.top();
        let anchor = Pos2::new(self.anchor_x(align), y);
        let bounds = galley.rect.translate(anchor.to_vec2());
        let gap = lay.gap(style, r.size);
        let nth = self.pieces.len();
        self.push(
            Piece {
                kind: Kind::Text {
                    galley,
                    anchor,
                    mark: None,
                    bar: None,
                    rule: None,
                    title: false,
                },
                bounds,
                step: 0,
                nth,
            },
            gap,
        );
    }

    /// The deck's author under a title page.
    pub fn byline(&mut self, lay: &Lay) {
        if !lay.a.byline || lay.deck.index != 0 || lay.a.eyebrow == Eyebrow::Deck {
            return;
        }
        let Some(author) = lay.deck.author.clone() else {
            return;
        };
        // the byline sits a little apart from what is above it
        self.pending = self.pending.max(28.0 * lay.scale);
        let style = &lay.a.roles.byline;
        let r = lay.resolve(style, 0);
        let align = style.align.unwrap_or(self.align);
        let galley = lay
            .ui
            .painter()
            .layout_job(style::text_job(&r, &author, h(align)));
        let y = self.top();
        let anchor = Pos2::new(self.anchor_x(align), y);
        let bounds = galley.rect.translate(anchor.to_vec2());
        let nth = self.pieces.len();
        self.push(
            Piece {
                kind: Kind::Text {
                    galley,
                    anchor,
                    mark: None,
                    bar: None,
                    rule: None,
                    title: false,
                },
                bounds,
                step: 0,
                nth,
            },
            0.0,
        );
    }

    /// Every item, in order.
    pub fn items(&mut self, lay: &Lay, items: &[Item<'s>]) {
        for item in items {
            self.item(lay, item);
        }
    }

    pub fn item(&mut self, lay: &Lay, item: &Item<'s>) {
        match (item.role, item.block) {
            (
                Role::List,
                Block::List {
                    ordered,
                    start,
                    items,
                },
            ) => {
                self.list(lay, items, *ordered, *start, 0);
                self.pending = lay.gap(&lay.a.roles.list, lay.theme.body_size * lay.scale);
            }
            (Role::Block, block) => self.block(lay, block),
            (role, Block::Heading { level, inlines }) => {
                self.text(lay, role, *level, inlines, 0);
            }
            (Role::Quote, _) if item.quote.is_some_and(Block::quote_is_structured) => {
                self.structured_quote(lay, item.quote.unwrap_or_default());
            }
            (Role::Quote, _) => {
                let inlines = item.inlines.clone().unwrap_or_default();
                let inlines = if lay.a.ornaments.quote_marks {
                    with_quotes(&inlines)
                } else {
                    inlines
                };
                self.text(lay, Role::Quote, 0, &inlines, 0);
            }
            (Role::Attribution, block) => {
                let inlines = item.inlines.clone().or_else(|| match block {
                    Block::Paragraph { inlines } => Some(inlines.clone()),
                    _ => None,
                });
                let inlines = attribution(
                    &inlines.unwrap_or_default(),
                    lay.a.ornaments.attribution_dash,
                );
                self.text(lay, Role::Attribution, 0, &inlines, 0);
            }
            (role, Block::Paragraph { inlines }) => {
                self.text(lay, role, 0, inlines, 0);
            }
            (_, block) => self.block(lay, block),
        }
    }

    /// A quote that holds a list or a nested quote (MD-12): its runs of
    /// paragraphs as quote text, its lists as lists, and each nested quote
    /// indented with a bar of its own. Left-aligned, since a list or a
    /// nested quote only reads as one from a common left edge.
    fn structured_quote(&mut self, lay: &Lay, blocks: &'s [Block]) {
        let align = self.align;
        self.align = HAlign::Left;
        // marks only where they pair up: the quote opens and closes in text
        let text =
            |b: Option<&Block>| matches!(b, Some(Block::Paragraph { .. } | Block::Heading { .. }));
        let marks = lay.a.ornaments.quote_marks && text(blocks.first()) && text(blocks.last());
        self.quote_level(lay, blocks, 0, marks);
        self.align = align;
        self.last_quote_right = None;
        let style = &lay.a.roles.quote;
        let size = lay.resolve(style, 0).size;
        self.pending = self.pending.max(lay.gap(style, size));
    }

    fn quote_level(&mut self, lay: &Lay, blocks: &'s [Block], depth: usize, marks: bool) {
        let s = lay.scale;
        let o = &lay.a.ornaments;
        let first = self.pieces.len();
        let top = self.top();
        let mut i = 0;
        while i < blocks.len() {
            let block = &blocks[i];
            match block {
                Block::Paragraph { .. } | Block::Heading { .. } => {
                    // a run of paragraphs reads as one passage
                    let mut run: Vec<Inline> = Vec::new();
                    let from = i;
                    while let Some(Block::Paragraph { inlines } | Block::Heading { inlines, .. }) =
                        blocks.get(i)
                    {
                        if !run.is_empty() {
                            run.push(Inline::Text("\n".into()));
                        }
                        run.extend(inlines.iter().cloned());
                        i += 1;
                    }
                    let open = marks && depth == 0 && from == 0;
                    let close = marks && depth == 0 && i == blocks.len();
                    let run = quote_marks(&run, open, close);
                    self.text(lay, Role::Quote, 0, &run, 0);
                    if let Some(Piece {
                        kind: Kind::Text { bar, .. },
                        ..
                    }) = self.pieces.last_mut()
                    {
                        *bar = None;
                    }
                    continue;
                }
                Block::List {
                    ordered,
                    start,
                    items,
                } => {
                    self.list(lay, items, *ordered, *start, 0);
                    self.pending = lay.gap(&lay.a.roles.list, lay.theme.body_size * s);
                }
                Block::BlockQuote { blocks: inner } => {
                    let nest = QUOTE_NEST * s;
                    self.x += nest;
                    self.width -= nest;
                    self.quote_level(lay, inner, depth + 1, marks);
                    self.x -= nest;
                    self.width += nest;
                }
                other => self.block(lay, other),
            }
            i += 1;
        }
        if depth == 0 && o.quote_bar != Bar::Left {
            return;
        }
        let bottom = self.y;
        if bottom <= top {
            return;
        }
        let w = o.bar_width.max(1.5) * s;
        let x = self.x - 24.0 * s;
        let rect = Rect::from_min_max(
            Pos2::new(x - w / 2.0, top + 4.0 * s),
            Pos2::new(x + w / 2.0, bottom - 4.0 * s),
        );
        let nth = self.pieces.get(first).map_or(self.pieces.len(), |p| p.nth);
        self.pieces.push(Piece {
            kind: Kind::Bar {
                rect,
                color: style::ink(lay.theme, o.bar_color),
            },
            bounds: rect,
            step: 0,
            nth,
        });
    }

    fn list(&mut self, lay: &Lay, items: &[ListItem], ordered: bool, start: u32, level: usize) {
        let o = &lay.a.ornaments;
        let s = lay.scale;
        let style = if level == 0 {
            &lay.a.roles.list
        } else {
            &lay.a.roles.nested
        };
        let r = lay.resolve(style, 0);
        let indent = o.indent * s;
        let text_x = self.x + indent + o.nested_indent * s * level as f32;
        let text_w = (self.x + self.width - text_x).max(200.0 * s);
        let item_gap = lay.space(&o.item_gap) * if level == 0 { 1.0 } else { 0.75 };
        let marker_ink = style::ink(lay.theme, o.bullet_color);
        for (i, item) in items.iter().enumerate() {
            let job = style::job(lay.theme, &r, &item.inlines, text_w, egui::Align::LEFT);
            let galley = lay.ui.painter().layout_job(job);
            let y = self.top();
            let anchor = Pos2::new(text_x, y);
            let first_row = galley
                .rows
                .first()
                .map_or(r.size, |row| row.rect().height());
            let numbered = (ordered || item.marker == ListMarker::Ordered) && o.numbering;
            let mark = if let Some(checked) = item.checked {
                let side = r.size * 0.62;
                Some(Mark::Task {
                    rect: Rect::from_min_size(
                        Pos2::new(text_x - indent, y + (first_row - side) / 2.0),
                        egui::vec2(side, side),
                    ),
                    checked,
                })
            } else if numbered || o.bullet != "dot" {
                let text = if numbered {
                    format!("{}.", start as usize + i)
                } else {
                    o.bullet.clone()
                };
                let mut mr = r.clone();
                mr.color = fade(marker_ink, r.color.a() as f32 / 255.0);
                mr.case = crate::theme::arrangement::Case::None;
                let marker =
                    lay.ui
                        .painter()
                        .layout_job(style::text_job(&mr, &text, egui::Align::LEFT));
                let dy = crate::render::math::first_baseline(&galley)
                    .zip(crate::render::math::first_baseline(&marker))
                    .map_or(0.0, |(t, m)| (t - m).max(0.0));
                // right-aligned in the gutter, a little apart from the text
                let x = text_x - 0.4 * r.size - marker.rect.width();
                Some(Mark::Glyph(marker, Pos2::new(x, y + dy)))
            } else {
                Some(Mark::Dot {
                    center: Pos2::new(text_x - indent * 0.53, y + first_row * 0.55),
                    radius: if level == 0 { 3.2 } else { 2.4 } * s,
                    color: marker_ink,
                })
            };
            let bounds = galley.rect.translate(anchor.to_vec2());
            let nth = self.pieces.len();
            self.push(
                Piece {
                    kind: Kind::Text {
                        galley,
                        anchor,
                        mark,
                        bar: None,
                        rule: None,
                        title: false,
                    },
                    bounds,
                    step: item.step,
                    nth,
                },
                item_gap,
            );
            if !item.children.is_empty() {
                let child_ordered = item
                    .children
                    .first()
                    .is_some_and(|c| c.marker == ListMarker::Ordered);
                self.list(lay, &item.children, child_ordered, 1, level + 1);
                self.pending = item_gap;
            }
        }
    }

    /// A block drawn whole: code, a table, a callout, an inline image or
    /// visual (sized to the column), a rule.
    fn block(&mut self, lay: &Lay, block: &'s Block) {
        let w = self.width;
        let s = lay.scale;
        let height = match block {
            Block::Image { .. } => (w * 0.6).min(400.0 * s),
            Block::Chart { .. } | Block::Diagram { .. } => {
                (w * 0.5625).min(lay.rect.height() * 0.6)
            }
            _ => crate::render::text::measure_single_block_height(lay.ui, block, lay.theme, w, s),
        };
        let y = self.top();
        let rect = Rect::from_min_size(Pos2::new(self.x, y), egui::vec2(w, height));
        let gap = crate::render::text::block_spacing(block, lay.theme, s);
        let nth = self.pieces.len();
        self.push(
            Piece {
                kind: Kind::Block { block, rect },
                bounds: rect,
                step: 0,
                nth,
            },
            gap,
        );
    }
}

pub fn h(align: HAlign) -> egui::Align {
    match align {
        HAlign::Left => egui::Align::LEFT,
        HAlign::Center => egui::Align::Center,
        HAlign::Right => egui::Align::RIGHT,
    }
}

/// `II · Deck title`: the numeral (in the accent) and the rest.
pub fn numeral_eyebrow(deck: &SlideContext) -> (String, String) {
    let numeral = roman(deck.index.max(1));
    let rest = deck
        .deck_title
        .clone()
        .or_else(|| deck.author.clone())
        .unwrap_or_default();
    if rest.is_empty() {
        (numeral, String::new())
    } else {
        (numeral, format!(" · {rest}"))
    }
}

/// Over a title page: author and deck title, whichever are set.
pub fn deck_eyebrow(deck: &SlideContext) -> String {
    match (&deck.author, &deck.deck_title) {
        (Some(a), Some(t)) if a != t => format!("{a} · {t}"),
        (Some(a), _) => a.clone(),
        (None, Some(t)) => t.clone(),
        (None, None) => "mdeck".to_string(),
    }
}

/// How far a nested quote's text sits in from its parent's, at 1920x1080.
const QUOTE_NEST: f32 = 48.0;

/// Curly quotation marks around `inlines`, unless they are there already.
pub fn with_quotes(inlines: &[Inline]) -> Vec<Inline> {
    quote_marks(inlines, true, true)
}

/// An opening mark before `inlines` (`open`) and a closing one after them
/// (`close`), each unless it is there already.
fn quote_marks(inlines: &[Inline], open: bool, close: bool) -> Vec<Inline> {
    let first = !open
        || matches!(inlines.first(), Some(Inline::Text(s)) if {
            let t = s.trim_start();
            t.starts_with('\u{201C}') || t.starts_with('"')
        });
    let last = !close
        || matches!(inlines.last(), Some(Inline::Text(s)) if {
            let t = s.trim_end();
            t.ends_with('\u{201D}') || t.ends_with('"')
        });
    if first && last {
        return inlines.to_vec();
    }
    let mut out = Vec::with_capacity(inlines.len() + 2);
    if !first {
        out.push(Inline::Text("\u{201C}".into()));
    }
    out.extend(inlines.iter().cloned());
    if !last {
        out.push(Inline::Text("\u{201D}".into()));
    }
    out
}

/// An attribution without its leading `--`, `---` or dash; with `dash` it
/// gets a typographic dash in front instead.
pub fn attribution(inlines: &[Inline], dash: bool) -> Vec<Inline> {
    let mut out = inlines.to_vec();
    if let Some(Inline::Text(s)) = out.first_mut() {
        let t = s.trim_start();
        let rest = t
            .strip_prefix("---")
            .or_else(|| t.strip_prefix("--"))
            .or_else(|| t.strip_prefix('\u{2014}'))
            .map(str::trim_start)
            .unwrap_or(t);
        *s = if dash {
            format!("\u{2014} {rest}")
        } else {
            rest.to_string()
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eyebrows_name_the_deck() {
        let mut deck = SlideContext {
            index: 2,
            author: Some("Ada".into()),
            ..Default::default()
        };
        assert_eq!(numeral_eyebrow(&deck), ("II".into(), " · Ada".into()));
        deck.deck_title = Some("Engines".into());
        assert_eq!(numeral_eyebrow(&deck), ("II".into(), " · Engines".into()));
        assert_eq!(deck_eyebrow(&deck), "Ada · Engines");
        assert_eq!(deck_eyebrow(&SlideContext::default()), "mdeck");
    }

    #[test]
    fn quotes_and_attributions() {
        let t = |s: &str| Inline::Text(s.into());
        assert_eq!(with_quotes(&[t("hi")]).len(), 3);
        assert_eq!(with_quotes(&[t("\"hi\"")]).len(), 1);
        let a = attribution(&[t("-- Alan Kay")], false);
        assert!(matches!(&a[0], Inline::Text(s) if s == "Alan Kay"));
        let a = attribution(&[t("--- Ada")], true);
        assert!(matches!(&a[0], Inline::Text(s) if s == "\u{2014} Ada"));
    }
}
