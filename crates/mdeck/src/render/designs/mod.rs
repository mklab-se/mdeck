//! Slide designs (DES-01..DES-17): one renderer, parameterised by the
//! theme's arrangement of the slide's design.
//!
//! A slide is laid out once into a [`Plan`] (copy pieces, plate items,
//! pillow and band); measuring for overflow reads the plan's height and
//! drawing paints the same plan, so the two can never disagree (DES-17).
//! Before a slide scrolls, code shrinks to [`CODE_FIT_FLOOR`] and then
//! prose to [`PROSE_FIT_FLOOR`] (DES-16).

pub mod copy;
pub mod motion;
pub mod parts;
pub mod pillow;
pub mod plate;
pub mod style;
#[cfg(test)]
mod tests;

use eframe::egui::{self, Pos2, Rect};

use crate::parser::{Block, Design, Slide};
use crate::render::{BlockCx, SlideContext};
use crate::theme::Theme;
use crate::theme::arrangement::{Arrangement, Frac, Place, VAlign};
use copy::{Kind, Lay, Mark, Piece, Stack};
use plate::Placed;

/// Code never shrinks below this fraction of the theme's code size; past it
/// the slide scrolls.
pub const CODE_FIT_FLOOR: f32 = 0.4;

/// Prose and lists never shrink below this fraction of their size; past it
/// the slide scrolls.
pub const PROSE_FIT_FLOOR: f32 = 0.8;

/// Short code grows toward this fraction of the body size when its lines
/// and the slide have room for it (a few lines should not sit small in a
/// large box).
pub const CODE_GROW_CEIL: f32 = 0.9;

/// A slide laid out.
pub struct Plan<'s> {
    pub pieces: Vec<Piece<'s>>,
    pub plate: Vec<Placed<'s>>,
    /// The stagger place of the plate.
    pub plate_nth: usize,
    /// Hairlines over columns (left end, width).
    pub rules: Vec<(Pos2, f32)>,
    /// Bounds of the copy (the pillow goes behind it).
    pub copy: Option<Rect>,
    /// A band behind the copy over a full-bleed image.
    pub band: Option<Rect>,
    /// Height of the content, and of the room it has.
    pub content: f32,
    pub available: f32,
    /// Top of the content.
    pub top: f32,
    /// Narrowest width code is laid out at.
    pub code_width: Option<f32>,
}

fn frac(rect: Rect, f: Frac) -> Rect {
    Rect::from_min_size(
        Pos2::new(
            rect.left() + f[0] * rect.width(),
            rect.top() + f[1] * rect.height(),
        ),
        egui::vec2(f[2] * rect.width(), f[3] * rect.height()),
    )
}

fn valign(v: VAlign, top: f32, room: f32, height: f32) -> f32 {
    if height >= room {
        return top;
    }
    match v {
        VAlign::Top => top,
        VAlign::Middle => top + (room - height) / 2.0,
        VAlign::Bottom => top + room - height,
    }
}

fn shift(pieces: &mut [Piece], d: egui::Vec2) {
    for p in pieces {
        p.translate(d);
    }
}

fn has_code(blocks: &[&Block]) -> bool {
    blocks.iter().any(|b| matches!(b, Block::CodeBlock { .. }))
}

/// Lay `slide` out in `rect` with the arrangement in `lay`.
fn plan<'s>(lay: &Lay, slide: &'s Slide, rect: Rect) -> Plan<'s> {
    let a = lay.a;
    let s = lay.scale;
    let parts = parts::of(slide.design, &slide.blocks);
    let region = if a.wide.is_some() && slide.blocks.iter().any(crate::render::is_wide_block) {
        a.wide.unwrap_or(a.copy)
    } else {
        a.copy
    };
    let r = frac(rect, region.region);

    // a full-bleed image: the copy in a band over it
    if slide.design == Design::Media
        && let [img @ Block::Image { directives, .. }] = parts.plate.as_slice()
        && directives.fill
    {
        return full_bleed(lay, &parts, img, rect);
    }

    let mut stack = Stack::new(r.left(), r.width(), region.align);
    stack.eyebrow(lay);
    stack.items(lay, &parts.copy);
    let copy_code = parts
        .copy
        .iter()
        .any(|i| matches!(i.block, Block::CodeBlock { .. }));
    let mut code_width = copy_code.then_some(r.width());

    let plate_spec = a
        .plate
        .as_ref()
        .filter(|_| !parts.plate.is_empty() || !parts.columns.is_empty());
    let Some(p) = plate_spec else {
        // no plate: everything in the copy
        stack.items(lay, &parts.footer);
        for col in &parts.columns {
            stack.items(lay, col);
        }
        stack.byline(lay);
        let h = stack.height();
        let top = valign(region.valign, r.top(), r.height(), h);
        let mut pieces = stack.pieces;
        shift(&mut pieces, egui::vec2(0.0, top));
        let copy = bounds(&pieces);
        renumber(&mut pieces);
        return Plan {
            pieces,
            plate: Vec::new(),
            plate_nth: 0,
            rules: Vec::new(),
            copy,
            band: None,
            content: h,
            available: r.height(),
            top,
            code_width,
        };
    };

    let pr = frac(rect, p.region);
    let gap = lay.space(&p.gap);
    if has_code(&parts.plate) {
        code_width = Some(code_width.map_or(pr.width(), |w| w.min(pr.width())));
    }

    match p.place {
        Place::Beside => {
            stack.items(lay, &parts.footer);
            let hc = stack.height();
            let natural = plate::natural_height(lay, &parts.plate, pr.width(), gap);
            let hp = natural.unwrap_or(pr.height());
            let content = hc.max(hp);
            let available = r.height().max(pr.height());
            let overflow = content > available;
            let ctop = if overflow {
                r.top()
            } else {
                valign(region.valign, r.top(), r.height(), hc)
            };
            let ptop = if overflow {
                pr.top()
            } else {
                valign(p.valign, pr.top(), pr.height(), hp)
            };
            let mut pieces = stack.pieces;
            shift(&mut pieces, egui::vec2(0.0, ctop));
            let copy = bounds(&pieces);
            let plate_rect =
                Rect::from_min_size(Pos2::new(pr.left(), ptop), egui::vec2(pr.width(), hp));
            let placed = plate::place(lay, &parts.plate, plate_rect, gap);
            let plate_nth = pieces.len();
            renumber(&mut pieces);
            Plan {
                pieces,
                plate: placed,
                plate_nth,
                rules: Vec::new(),
                copy,
                band: None,
                content,
                available,
                top: ctop.min(ptop),
                code_width,
            }
        }
        Place::Below => {
            let hc = stack.height();
            let head_gap = if stack.pieces.is_empty() {
                0.0
            } else {
                gap.max(stack.trailing_gap())
            };
            // the footer, in the copy's column
            let mut footer = Stack::new(r.left(), r.width(), region.align);
            footer.items(lay, &parts.footer);
            let hf = footer.height();
            let foot_gap = if footer.pieces.is_empty() { 0.0 } else { gap };
            // columns, or the plate's blocks
            let mut cols: Vec<Stack> = Vec::new();
            let rule_pad = if p.rule { 22.0 * s } else { 0.0 };
            let natural = if parts.columns.is_empty() {
                plate::natural_height(lay, &parts.plate, pr.width(), gap)
            } else {
                let n = parts.columns.len().max(1) as f32;
                let cw = (pr.width() - gap * (n - 1.0)) / n;
                for (i, col) in parts.columns.iter().enumerate() {
                    let mut st = Stack::new(
                        pr.left() + i as f32 * (cw + gap),
                        cw,
                        crate::theme::arrangement::HAlign::Left,
                    );
                    st.items(lay, col);
                    cols.push(st);
                }
                Some(cols.iter().map(Stack::height).fold(0.0, f32::max) + rule_pad)
            };
            let fixed = hc + head_gap + foot_gap + hf;
            let (hp, top) = match natural {
                Some(n) => {
                    let total = fixed + n;
                    (n, valign(region.valign, r.top(), r.height(), total))
                }
                None => {
                    let room = r.height() - fixed;
                    let hp = room.max(r.height() * 0.4);
                    (hp, r.top())
                }
            };
            let content = fixed + hp;
            let mut pieces = stack.pieces;
            shift(&mut pieces, egui::vec2(0.0, top));
            let copy = bounds(&pieces);
            let plate_top = top + hc + head_gap;
            let plate_nth = pieces.len();
            let mut rules = Vec::new();
            for mut st in cols {
                if p.rule {
                    rules.push((Pos2::new(st.x(), plate_top), st.width()));
                }
                shift(&mut st.pieces, egui::vec2(0.0, plate_top + rule_pad));
                pieces.extend(st.pieces);
            }
            let plate_rect =
                Rect::from_min_size(Pos2::new(pr.left(), plate_top), egui::vec2(pr.width(), hp));
            let placed = if parts.plate.is_empty() {
                Vec::new()
            } else {
                plate::place(lay, &parts.plate, plate_rect, gap)
            };
            let mut fpieces = footer.pieces;
            shift(&mut fpieces, egui::vec2(0.0, plate_top + hp + foot_gap));
            pieces.extend(fpieces);
            renumber(&mut pieces);
            Plan {
                pieces,
                plate: placed,
                plate_nth,
                rules,
                copy,
                band: None,
                content,
                available: r.height(),
                top,
                code_width,
            }
        }
    }
}

/// A media slide whose image fills the slide: the copy sits in a band at
/// the bottom.
fn full_bleed<'s>(lay: &Lay, parts: &parts::Parts<'s>, img: &'s Block, rect: Rect) -> Plan<'s> {
    let s = lay.scale;
    let pad = 60.0 * s;
    let mut stack = Stack::new(
        rect.left() + pad,
        rect.width() - 2.0 * pad,
        crate::theme::arrangement::HAlign::Left,
    );
    stack.items(lay, &parts.copy);
    stack.items(lay, &parts.footer);
    let h = stack.height();
    let band_h = h + 32.0 * s;
    let band = Rect::from_min_size(
        Pos2::new(rect.left(), rect.bottom() - band_h - 40.0 * s),
        egui::vec2(rect.width(), band_h),
    );
    let mut pieces = stack.pieces;
    shift(&mut pieces, egui::vec2(0.0, band.top() + 16.0 * s));
    renumber(&mut pieces);
    Plan {
        plate: vec![Placed {
            block: img,
            rect,
            caption: None,
        }],
        plate_nth: 0,
        rules: Vec::new(),
        copy: None,
        band: (!pieces.is_empty()).then_some(band),
        pieces,
        content: 0.0,
        available: rect.height(),
        top: rect.top(),
        code_width: None,
    }
}

fn bounds(pieces: &[Piece]) -> Option<Rect> {
    pieces.iter().map(|p| p.bounds).reduce(|a, b| a.union(b))
}

fn renumber(pieces: &mut [Piece]) {
    for (i, p) in pieces.iter_mut().enumerate() {
        p.nth = i;
    }
}

/// The theme `slide` is drawn with (sizes fitted so it fits, down to the
/// floors) and its plan.
pub fn layout<'s>(
    ui: &egui::Ui,
    slide: &'s Slide,
    theme: &Theme,
    rect: Rect,
    scale: f32,
    deck: &SlideContext,
) -> (Theme, Plan<'s>) {
    let build = |t: &Theme| {
        let lay = Lay {
            ui,
            theme: t,
            a: t.arrangement(slide.design),
            scale,
            deck,
            rect,
        };
        plan(&lay, slide, rect)
    };
    let mut t = theme.clone();
    let mut p = build(&t);
    let codes: Vec<&str> = slide
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::CodeBlock { code, .. } => Some(code.as_str()),
            _ => None,
        })
        .collect();
    let mut fc = 1.0_f32;
    let mut ft = 1.0_f32;
    // long lines shrink instead of wrapping
    if let Some(w) = p.code_width
        && !codes.is_empty()
    {
        let widest = codes
            .iter()
            .map(|c| crate::render::text::widest_code_line(ui, c, &t, scale))
            .fold(0.0, f32::max);
        let inner = w - 2.0 * crate::render::text::CODE_PADDING * scale;
        if widest > 0.0 {
            fc = grow_factor(theme, widest, inner);
            if (fc - 1.0).abs() > 1e-3 {
                t.code_size = theme.code_size * fc;
                p = build(&t);
            }
        }
    }
    for _ in 0..8 {
        if p.content <= p.available + 0.5 {
            break;
        }
        let ratio = p.available / p.content * 0.99;
        if !codes.is_empty() && fc > CODE_FIT_FLOOR + 1e-3 {
            fc = (fc * ratio).max(CODE_FIT_FLOOR);
            t.code_size = theme.code_size * fc;
        } else if ft > PROSE_FIT_FLOOR + 1e-3 {
            ft = (ft * ratio).max(PROSE_FIT_FLOOR);
            t.h1_size = theme.h1_size * ft;
            t.h2_size = theme.h2_size * ft;
            t.h3_size = theme.h3_size * ft;
            t.body_size = theme.body_size * ft;
        } else {
            break;
        }
        p = build(&t);
    }
    (t, p)
}

/// How much code whose widest line is `widest` (at the theme's code size)
/// is scaled to sit in `inner`: down to fit the width (not below
/// [`CODE_FIT_FLOOR`]), or up toward [`CODE_GROW_CEIL`] of the body size
/// while the lines still fit. Height is fitted afterwards.
pub fn grow_factor(theme: &Theme, widest: f32, inner: f32) -> f32 {
    if widest <= 0.0 || theme.code_size <= 0.0 {
        return 1.0;
    }
    let by_width = inner / widest;
    if by_width < 1.0 {
        return by_width.max(CODE_FIT_FLOOR);
    }
    let ceil = (theme.body_size * CODE_GROW_CEIL / theme.code_size).max(1.0);
    by_width.min(ceil)
}

/// `(content height, room)` of `slide` drawn in `rect`: the slide scrolls
/// when the first is larger.
pub fn measure(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: Rect,
    scale: f32,
    deck: &SlideContext,
) -> (f32, f32) {
    let (_, p) = layout(ui, slide, theme, rect, scale, deck);
    (p.content, p.available)
}

/// Bottom of what reveal step `step` shows, relative to the content's top
/// (for the auto-scroll to the item just revealed).
pub fn revealed_bottom(
    ui: &egui::Ui,
    slide: &Slide,
    theme: &Theme,
    rect: Rect,
    scale: f32,
    deck: &SlideContext,
    step: usize,
) -> Option<f32> {
    let (_, p) = layout(ui, slide, theme, rect, scale, deck);
    let pieces = p
        .pieces
        .iter()
        .filter(|x| x.step == step && step > 0)
        .map(|x| x.bounds.bottom());
    let plate = p
        .plate
        .iter()
        .filter(|x| {
            visual_base(x.block).is_some_and(|base| {
                step > base && step <= base + crate::parser::steps::default_visual_steps(x.block)
            })
        })
        .map(|x| x.rect.bottom());
    pieces
        .chain(plate)
        .fold(None, |m: Option<f32>, b| Some(m.map_or(b, |m| m.max(b))))
        .map(|b| b - p.top)
}

fn visual_base(block: &Block) -> Option<usize> {
    match block {
        Block::Chart { step_base, .. } | Block::Diagram { step_base, .. } => Some(*step_base),
        _ => None,
    }
}

/// Draw `slide` in `rect`.
pub fn render(cx: &BlockCx, slide: &Slide, rect: Rect, deck: &SlideContext) {
    let (fitted, plan) = layout(cx.ui, slide, cx.theme, rect, cx.scale, deck);
    let a = fitted.arrangement(slide.design);
    // an engine that forms headings itself (the thermal cold opening) gets
    // the title and section copy late, and the heading's layout as a hint
    let cold_open =
        cx.theme.copy_hold > 0.0 && matches!(slide.design, Design::Title | Design::Section);
    let mut age = if deck.hold_copy {
        motion::entry_age(cx.ui, deck.index, deck.animate, true);
        -1.0
    } else {
        motion::entry_age(cx.ui, deck.index, deck.animate, false)
    };
    if cold_open && age >= 0.0 && deck.animate {
        age -= cx.theme.copy_hold;
    }
    let cx = cx.with_theme(&fitted);
    paint(&cx, &plan, a, deck, (age, cold_open), rect);
}

fn paint(
    cx: &BlockCx,
    plan: &Plan,
    a: &Arrangement,
    deck: &SlideContext,
    (age, cold_open): (f32, bool),
    slide: Rect,
) {
    let painter = cx.ui.painter();
    let theme = cx.theme;
    let s = cx.scale;
    if cold_open {
        for p in &plan.pieces {
            if let Kind::Text {
                galley,
                anchor,
                title: true,
                ..
            } = &p.kind
            {
                crate::render::hints::push(
                    cx.ui.ctx(),
                    crate::render::hints::Hint::Text {
                        galley: galley.clone(),
                        pos: *anchor,
                        slide: deck.index,
                    },
                );
            }
        }
    }
    // where the copy is, so an engine that fills the slide keeps clear of it
    if let Some(copy) = plan
        .pieces
        .iter()
        .filter(|p| matches!(p.kind, Kind::Text { .. }))
        .map(|p| p.bounds)
        .reduce(|a, b| a.union(b))
    {
        crate::render::hints::push(cx.ui.ctx(), crate::render::hints::Hint::Copy(copy));
    }
    // the plate first: a band and the copy may lie over it
    let entry = &a.entry;
    let reveal_age = cx.reveal_timestamp.map(|t| t.elapsed().as_secs_f32());
    // a lone image smaller than its plate sits where the plate's alignment
    // says, and what follows it (the caption) moves up to meet it
    let mut plate_shift = egui::Vec2::ZERO;
    let mut foot_shift = 0.0;
    if let ([item], Some(p)) = (plan.plate.as_slice(), a.plate.as_ref())
        && let Block::Image {
            path, directives, ..
        } = item.block
        && !directives.fill
        && let Some(f) = crate::render::text::image_rect_in(cx, path, directives, item.rect)
    {
        use crate::theme::arrangement::HAlign;
        let r = item.rect;
        let free = r.height() - f.height();
        let dx = match p.align {
            HAlign::Left => r.left() - f.left(),
            HAlign::Center => 0.0,
            HAlign::Right => r.right() - f.right(),
        };
        let top = match p.valign {
            VAlign::Top => r.top(),
            VAlign::Middle => r.top() + free / 2.0,
            VAlign::Bottom => r.bottom() - f.height(),
        };
        plate_shift = egui::vec2(dx, top - f.top());
        foot_shift = (top + f.height()) - r.bottom();
    }
    if age >= 0.0 {
        let pp = motion::entry_progress(entry, age, plan.plate_nth);
        let off = motion::offset(entry, pp, false) * s;
        let pcx = BlockCx {
            opacity: cx.opacity * pp,
            ..*cx
        };
        for item in &plan.plate {
            let rect = item.rect.translate(off + plate_shift);
            if matches!(item.block, Block::Chart { .. } | Block::Diagram { .. }) {
                crate::render::hints::push(cx.ui.ctx(), crate::render::hints::Hint::Frame(rect));
            }
            draw_boxed(&pcx, item.block, rect);
            if let Some((g, pos)) = &item.caption {
                crate::render::math::galley_faded(painter, *pos + off, g.clone(), pcx.opacity);
            }
        }
    }
    if let Some(band) = plan.band {
        painter.rect_filled(band, 0.0, style::fade(theme.background, cx.opacity * 0.6));
    }
    if a.ornaments.pillow
        && let Some(copy) = plan.copy
    {
        let alpha = if a.entry.kind == crate::theme::arrangement::Motion::None {
            cx.opacity
        } else {
            cx.opacity * motion::ease_out(age / 0.9)
        };
        pillow::pillow(painter, copy, alpha, theme);
    }
    if age < 0.0 {
        cx.ui.ctx().request_repaint();
        return;
    }
    for (pos, w) in &plan.rules {
        let p = motion::entry_progress(entry, age, plan.plate_nth);
        painter.line_segment(
            [*pos, *pos + egui::vec2(*w, 0.0)],
            egui::Stroke::new(
                1.0 * s.max(0.5),
                style::fade(theme.accent, cx.opacity * p * 0.7),
            ),
        );
    }
    for piece in &plan.pieces {
        if piece.step > cx.reveal_step {
            continue;
        }
        let p = motion::piece_progress(
            entry,
            piece.step,
            cx.reveal_step,
            age,
            reveal_age,
            piece.nth,
        );
        if p < 1.0 {
            cx.ui.ctx().request_repaint();
        }
        let revealed = piece.step > 0
            && piece.step == cx.reveal_step
            && reveal_age.is_some_and(|r| r < a.entry.reveal_ms / 500.0);
        let mut off = motion::offset(entry, p, revealed) * s;
        if piece.nth >= plan.plate_nth && !plan.plate.is_empty() {
            off.y += foot_shift;
        }
        let alpha = cx.opacity * p;
        match &piece.kind {
            Kind::Text {
                galley,
                anchor,
                mark,
                bar,
                rule,
                ..
            } => {
                crate::render::math::galley_faded(painter, *anchor + off, galley.clone(), alpha);
                match mark {
                    Some(Mark::Glyph(g, pos)) => {
                        crate::render::math::galley_faded(painter, *pos + off, g.clone(), alpha)
                    }
                    Some(Mark::Dot {
                        center,
                        radius,
                        color,
                    }) => {
                        painter.circle_filled(*center + off, *radius, style::fade(*color, alpha));
                    }
                    Some(Mark::Task { rect, checked }) => {
                        task_box(cx, rect.translate(off), *checked, alpha)
                    }
                    None => {}
                }
                if let Some((x, color, w)) = bar {
                    let b = piece.bounds.translate(off);
                    let r = Rect::from_min_max(
                        Pos2::new(*x - w / 2.0 + off.x, b.top() + 4.0 * s),
                        Pos2::new(*x + w / 2.0 + off.x, b.bottom() - 4.0 * s),
                    );
                    painter.rect_filled(r, w / 2.0, style::fade(*color, alpha));
                }
                if let Some((pos, w)) = rule {
                    painter.line_segment(
                        [*pos + off, *pos + off + egui::vec2(*w, 0.0)],
                        egui::Stroke::new(2.0 * s, style::fade(theme.accent, alpha)),
                    );
                }
            }
            Kind::Block { block, rect } => {
                let bcx = BlockCx {
                    opacity: alpha,
                    ..*cx
                };
                draw_boxed(&bcx, block, rect.translate(off));
            }
        }
    }
    if a.ornaments.begin_hint && deck.index == 0 && deck.animate {
        begin_hint(cx, a, age, slide);
    }
}

/// "Space to begin" under the first slide, live only.
fn begin_hint(cx: &BlockCx, a: &Arrangement, age: f32, slide: Rect) {
    let p = motion::entry_progress(&a.entry, age, 6);
    let lay_style = &a.roles.eyebrow;
    let mut r = style::resolve(cx.theme, lay_style, a.ornaments.emphasis, 0, cx.scale);
    r.size *= 0.85;
    let g = cx
        .ui
        .painter()
        .layout_job(style::text_job(&r, "Space to begin", egui::Align::Center));
    crate::render::math::galley_faded(
        cx.ui.painter(),
        Pos2::new(slide.center().x, slide.bottom() - 62.0 * cx.scale),
        g,
        cx.opacity * p * 0.9,
    );
    if p < 1.0 {
        cx.ui.ctx().request_repaint();
    }
}

fn task_box(cx: &BlockCx, rect: Rect, checked: bool, alpha: f32) {
    let side = rect.width();
    let accent = style::fade(cx.theme.accent, alpha);
    let painter = cx.ui.painter();
    let rounding = side * 0.22;
    if checked {
        painter.rect_filled(rect, rounding, accent);
        let ink = style::fade(cx.theme.background, alpha);
        let stroke = egui::Stroke::new((side * 0.14).max(1.0), ink);
        let p = |x: f32, y: f32| Pos2::new(rect.left() + x * side, rect.top() + y * side);
        painter.line_segment([p(0.22, 0.52), p(0.42, 0.72)], stroke);
        painter.line_segment([p(0.42, 0.72), p(0.78, 0.30)], stroke);
    } else {
        painter.rect_stroke(
            rect,
            rounding,
            egui::Stroke::new((side * 0.1).max(1.0), accent),
            egui::StrokeKind::Inside,
        );
    }
}

/// Draw `block` in `rect`: images fit it, visuals fill it, anything else
/// draws at its width.
fn draw_boxed(cx: &BlockCx, block: &Block, rect: Rect) {
    match block {
        Block::Image {
            alt,
            path,
            directives,
        } => {
            crate::render::text::draw_image_in_area(cx, path, alt, directives, rect);
        }
        Block::Chart {
            kind: crate::parser::Chart::Thermal,
            content,
            step_base,
        } => {
            crate::render::thermal::draw(
                &cx.after_steps(*step_base),
                content,
                rect.min,
                rect.width(),
                rect.height(),
            );
        }
        Block::Chart {
            kind,
            content,
            step_base,
        } => {
            crate::render::visualizations::draw(
                *kind,
                content,
                &cx.after_steps(*step_base).viz(),
                rect.min,
                rect.width(),
                rect.height(),
            );
        }
        Block::Diagram { content, step_base } => {
            crate::render::diagram::draw_diagram_sized(
                &cx.after_steps(*step_base),
                content,
                rect.min,
                rect.width(),
                rect.height(),
            );
        }
        _ => {
            crate::render::text::draw_block(cx, block, rect.min, rect.width());
        }
    }
}
