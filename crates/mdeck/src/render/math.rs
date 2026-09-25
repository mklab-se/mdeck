//! LaTeX math (spike for GitHub issue 11).
//!
//! RaTeX parses and lays out the formula and returns a flat display list:
//! glyphs from the KaTeX fonts at absolute em positions, rules, rectangles
//! and vector paths. mdeck paints that list itself with epaint, so formulas
//! stay sharp at any scale and in export.
//!
//! Inline math lives inside ordinary text jobs. [`append`] reserves the
//! formula's exact width with an invisible placeholder (a word joiner plus
//! invisible id digits, which epaint draws as nothing and never breaks), and
//! [`paint_galley`] finds placeholders in a laid-out galley and paints the
//! formulas on their baselines, in the colour of the surrounding text.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Stroke, pos2, vec2};
use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_types::display_item::{DisplayItem, DisplayList};
use ratex_types::math_style::MathStyle;
use ratex_types::path_command::PathCommand;

/// Formula size relative to the surrounding text (KaTeX uses 1.21).
pub const EM_SCALE: f32 = 1.15;

/// Starts every placeholder.
const MARK: char = '\u{2060}';
/// Invisible characters used as base-8 digits of the formula id.
const DIGITS: [char; 8] = [
    '\u{2061}', '\u{2062}', '\u{2063}', '\u{2064}', '\u{206A}', '\u{206B}', '\u{206C}', '\u{206D}',
];
const ID_DIGITS: usize = 4;
/// Ends every placeholder and carries the formula's width as its spacing.
/// epaint skips the spacing of a row's first glyph, and this one is never
/// first: the mark and the id always precede it.
const END: char = '\u{FEFF}';

/// A laid-out formula, or why it failed.
pub type Laid = Result<Arc<DisplayList>, String>;

#[derive(Default)]
struct Registry {
    ids: HashMap<(String, bool), usize>,
    formulas: Vec<Laid>,
}

static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(Default::default);

/// Lay out `tex` (display or inline style), cached for the process.
pub fn lay_out(tex: &str, display: bool) -> (usize, Laid) {
    let mut reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&id) = reg.ids.get(&(tex.to_string(), display)) {
        return (id, reg.formulas[id].clone());
    }
    let laid = ratex_parser::parse(tex)
        .map_err(|e| e.to_string())
        .map(|nodes| {
            let opts = LayoutOptions {
                style: if display {
                    MathStyle::Display
                } else {
                    MathStyle::Text
                },
                ..Default::default()
            };
            Arc::new(to_display_list(&layout(&nodes, &opts)))
        });
    let id = reg.formulas.len();
    reg.formulas.push(laid.clone());
    reg.ids.insert((tex.to_string(), display), id);
    (id, laid)
}

fn formula(id: usize) -> Option<Laid> {
    let reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    reg.formulas.get(id).cloned()
}

fn placeholder(id: usize) -> String {
    let mut s = String::from(MARK);
    let mut n = id;
    let mut digits = [DIGITS[0]; ID_DIGITS];
    for d in digits.iter_mut().rev() {
        *d = DIGITS[n % 8];
        n /= 8;
    }
    s.extend(digits);
    s
}

fn digit_value(c: char) -> Option<usize> {
    DIGITS.iter().position(|&d| d == c)
}

/// Append a formula to `job` in `format`'s size and colour. A formula that
/// does not parse is shown as its source.
///
/// Inline formulas reserve their width and extra headroom above the line;
/// they are painted on the surrounding text's baseline. Display formulas get
/// a line of their own, sized so the formula fits between its neighbours.
pub fn append(
    job: &mut egui::text::LayoutJob,
    tex: &str,
    display: bool,
    format: &egui::text::TextFormat,
) {
    let (id, laid) = lay_out(tex, display);
    let Ok(dl) = laid else {
        let delim = if display { "$$" } else { "$" };
        job.append(&format!("{delim}{tex}{delim}"), 0.0, format.clone());
        return;
    };
    let size = format.font_id.size;
    let em = size * EM_SCALE;
    let (h, d) = (dl.height as f32 * em, dl.depth as f32 * em);
    let text = placeholder(id);
    let mut f = format.clone();
    f.extra_letter_spacing = 0.0;
    f.background = Color32::TRANSPARENT;
    f.underline = Stroke::NONE;
    f.strikethrough = Stroke::NONE;
    let normal = format.line_height.unwrap_or(size * 1.2);
    if display {
        if !job.text.is_empty() && !job.text.ends_with('\n') {
            job.append("\n", 0.0, format.clone());
        }
        // Alone on its row, the placeholder's ascent is the baseline: make it
        // at least the formula's height, and the row tall enough for depth.
        let s_p = (h / 0.85).max(size);
        f.font_id.size = s_p;
        f.line_height = Some((s_p * 1.1 + d + size * 0.25).max(normal));
    } else {
        // Text is bottom-aligned in a taller row, so extra height goes above.
        let extra = (h - size * 0.9).max(0.0);
        f.line_height = Some(normal + extra);
    }
    let mut end = f.clone();
    end.extra_letter_spacing = dl.width as f32 * em;
    job.append(&text, 0.0, f);
    job.append(&END.to_string(), 0.0, end);
    if display {
        job.append("\n", 0.0, format.clone());
    }
}

/// Drop a trailing line break a display formula left at the end of `job`.
pub fn finish(job: &mut egui::text::LayoutJob) {
    if job.text.ends_with('\n')
        && let Some(last) = job.sections.last()
        && last.byte_range.end.0 == job.text.len()
        && last.byte_range.end.0 - last.byte_range.start.0 == 1
    {
        job.sections.pop();
        job.text.pop();
    }
}

/// Paint every formula placeholder in `galley`, drawn at `pos`.
///
/// epaint keeps a glyph's section private, so placeholders are matched to
/// their sections by order: the n-th placeholder glyph is the n-th section
/// whose text starts with the mark.
pub fn paint_galley(
    painter: &egui::Painter,
    pos: Pos2,
    galley: &egui::Galley,
    tint: Option<Color32>,
) {
    let job = &galley.job;
    if !job.text.contains(MARK) {
        return;
    }
    // (formula id, its format, the reserved width in points)
    let mut sections = job.sections.iter().enumerate().filter_map(|(k, sec)| {
        let text = &job.text[sec.byte_range.start.0..sec.byte_range.end.0];
        let mut chars = text.chars();
        (chars.next() == Some(MARK)).then(|| {
            let id = chars
                .take(ID_DIGITS)
                .map(|c| digit_value(c).unwrap_or(0))
                .fold(0, |acc, d| acc * 8 + d);
            let width = job
                .sections
                .get(k + 1)
                .map_or(0.0, |e| e.format.extra_letter_spacing);
            (id, &sec.format, width)
        })
    });
    for placed in &galley.rows {
        let glyphs = &placed.row.glyphs;
        // the row's text baseline, from a glyph that is not part of a placeholder
        let text_baseline = glyphs
            .iter()
            .find(|g| g.chr != MARK && g.chr != END && digit_value(g.chr).is_none())
            .map(|g| g.pos.y);
        for (i, g) in glyphs.iter().enumerate().filter(|(_, g)| g.chr == MARK) {
            let Some((id, format, width)) = sections.next() else {
                return;
            };
            let Some(Ok(dl)) = formula(id) else {
                continue;
            };
            if dl.width <= 0.0 || width <= 0.0 {
                continue;
            }
            let em = width / dl.width as f32;
            // the formula ends where the end character sits
            let right = glyphs
                .get(i + 1 + ID_DIGITS)
                .filter(|e| e.chr == END)
                .map_or(g.pos.x + width, |e| e.pos.x);
            let baseline = text_baseline.unwrap_or(g.pos.y);
            let origin = pos + placed.pos.to_vec2() + vec2(right - width, baseline);
            paint_list(painter, origin, em, tint.unwrap_or(format.color), &dl);
        }
    }
}

/// `painter.galley` plus the formulas inside it.
pub fn galley(painter: &egui::Painter, pos: Pos2, galley: Arc<egui::Galley>, fallback: Color32) {
    painter.galley(pos, galley.clone(), fallback);
    paint_galley(painter, pos, &galley, None);
}

/// `painter.galley_with_override_text_color` plus the formulas inside it.
pub fn galley_tinted(painter: &egui::Painter, pos: Pos2, galley: Arc<egui::Galley>, tint: Color32) {
    painter.galley_with_override_text_color(pos, galley.clone(), tint);
    paint_galley(painter, pos, &galley, Some(tint));
}

/// Paint `dl` with its baseline-left corner at `baseline`, `em` pixels per em.
pub fn paint_list(
    painter: &egui::Painter,
    baseline: Pos2,
    em: f32,
    color: Color32,
    dl: &DisplayList,
) {
    let top = baseline.y - dl.height as f32 * em;
    let at = |x: f64, y: f64| pos2(baseline.x + x as f32 * em, top + y as f32 * em);
    for item in &dl.items {
        match item {
            DisplayItem::GlyphPath {
                x,
                y,
                scale,
                font,
                char_code,
                ..
            } => {
                let Some(ch) = char::from_u32(*char_code) else {
                    continue;
                };
                let family = katex_family(font);
                let size = em * *scale as f32;
                let galley =
                    painter.layout_no_wrap(ch.to_string(), FontId::new(size, family), color);
                let Some(row) = galley.rows.first() else {
                    continue;
                };
                let Some(glyph) = row.row.glyphs.first() else {
                    continue;
                };
                let ascent = row.pos.y + glyph.pos.y;
                let p = at(*x, *y);
                painter.galley(pos2(p.x, p.y - ascent), galley, color);
            }
            DisplayItem::Line {
                x,
                y,
                width,
                thickness,
                ..
            } => {
                let t = (*thickness as f32 * em).max(1.0);
                let p = at(*x, *y);
                let rect = egui::Rect::from_min_size(
                    pos2(p.x, p.y - t / 2.0),
                    vec2(*width as f32 * em, t),
                );
                painter.rect_filled(rect, 0.0, color);
            }
            DisplayItem::Rect {
                x,
                y,
                width,
                height,
                ..
            } => {
                let rect = egui::Rect::from_min_size(
                    at(*x, *y),
                    vec2(*width as f32 * em, *height as f32 * em),
                );
                painter.rect_filled(rect, 0.0, color);
            }
            DisplayItem::Path {
                x,
                y,
                commands,
                fill,
                ..
            } => {
                let origin = at(*x, *y);
                for poly in flatten(commands, origin, em) {
                    if *fill {
                        painter.add(egui::Shape::mesh(fill_mesh(&poly, color)));
                    } else {
                        painter.add(egui::Shape::line(
                            poly,
                            Stroke::new((em * 0.04).max(1.0), color),
                        ));
                    }
                }
            }
        }
    }
}

/// The registered family for a RaTeX font name; CJK and unknown faces fall
/// back to the proportional family (which ends in the system CJK faces).
fn katex_family(font: &str) -> FontFamily {
    if crate::render::fonts::KATEX_FACES
        .iter()
        .any(|(name, _)| *name == font)
    {
        FontFamily::Name(format!("katex-{font}").into())
    } else {
        FontFamily::Proportional
    }
}

/// Flatten path commands (em units, relative to `origin`) into polygons.
fn flatten(commands: &[PathCommand], origin: Pos2, em: f32) -> Vec<Vec<Pos2>> {
    let p = |x: f64, y: f64| pos2(origin.x + x as f32 * em, origin.y + y as f32 * em);
    let mut polys: Vec<Vec<Pos2>> = Vec::new();
    let mut cur: Vec<Pos2> = Vec::new();
    for c in commands {
        match *c {
            PathCommand::MoveTo { x, y } => {
                if cur.len() > 2 {
                    polys.push(std::mem::take(&mut cur));
                }
                cur.clear();
                cur.push(p(x, y));
            }
            PathCommand::LineTo { x, y } => cur.push(p(x, y)),
            PathCommand::QuadTo { x1, y1, x, y } => {
                let a = *cur.last().unwrap_or(&p(x1, y1));
                let (b, e) = (p(x1, y1), p(x, y));
                for k in 1..=8 {
                    let t = k as f32 / 8.0;
                    let u = 1.0 - t;
                    cur.push(pos2(
                        u * u * a.x + 2.0 * u * t * b.x + t * t * e.x,
                        u * u * a.y + 2.0 * u * t * b.y + t * t * e.y,
                    ));
                }
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let a = *cur.last().unwrap_or(&p(x1, y1));
                let (b, c2, e) = (p(x1, y1), p(x2, y2), p(x, y));
                for k in 1..=10 {
                    let t = k as f32 / 10.0;
                    let u = 1.0 - t;
                    let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                    cur.push(pos2(
                        w0 * a.x + w1 * b.x + w2 * c2.x + w3 * e.x,
                        w0 * a.y + w1 * b.y + w2 * c2.y + w3 * e.y,
                    ));
                }
            }
            PathCommand::Close => {
                if cur.len() > 2 {
                    polys.push(std::mem::take(&mut cur));
                }
            }
        }
    }
    if cur.len() > 2 {
        polys.push(cur);
    }
    polys
}

/// Fill a simple (possibly concave) polygon by ear clipping.
fn fill_mesh(poly: &[Pos2], color: Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    for &p in poly {
        mesh.colored_vertex(p, color);
    }
    let area: f32 = (0..poly.len())
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let ccw = area > 0.0;
    let cross = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut idx: Vec<usize> = (0..poly.len()).collect();
    let mut guard = 0;
    while idx.len() > 3 && guard < poly.len() * poly.len() {
        guard += 1;
        let n = idx.len();
        let mut clipped = false;
        for k in 0..n {
            let (ia, ib, ic) = (idx[(k + n - 1) % n], idx[k], idx[(k + 1) % n]);
            let (a, b, c) = (poly[ia], poly[ib], poly[ic]);
            let turn = cross(a, b, c);
            if (turn > 0.0) != ccw || turn.abs() < 1e-6 {
                continue;
            }
            let inside = idx.iter().any(|&j| {
                if j == ia || j == ib || j == ic {
                    return false;
                }
                let p = poly[j];
                let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
                if ccw {
                    d1 > 0.0 && d2 > 0.0 && d3 > 0.0
                } else {
                    d1 < 0.0 && d2 < 0.0 && d3 < 0.0
                }
            });
            if inside {
                continue;
            }
            mesh.add_triangle(ia as u32, ib as u32, ic as u32);
            idx.remove(k);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if idx.len() == 3 {
        mesh.add_triangle(idx[0] as u32, idx[1] as u32, idx[2] as u32);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_round_trips_the_id() {
        for id in [0usize, 7, 8, 511, 4095] {
            let s = placeholder(id);
            let mut chars = s.chars();
            assert_eq!(chars.next(), Some(MARK));
            let back = chars.fold(0, |acc, c| acc * 8 + digit_value(c).unwrap());
            assert_eq!(back, id);
        }
    }

    #[test]
    fn common_formulas_lay_out_and_bad_ones_report() {
        for tex in [
            "x^2",
            r"\frac{-b \pm \sqrt{b^2-4ac}}{2a}",
            r"\sum_{i=1}^n i",
            r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
        ] {
            let (_, laid) = lay_out(tex, true);
            let dl = laid.unwrap_or_else(|e| panic!("{tex}: {e}"));
            assert!(dl.width > 0.0 && !dl.items.is_empty(), "{tex}");
        }
        assert!(lay_out(r"\frac{1}{", false).1.is_err());
        // cached: same id twice
        assert_eq!(lay_out("x^2", false).0, lay_out("x^2", false).0);
    }

    #[test]
    fn ear_clipping_fills_a_concave_polygon() {
        // an L shape: 6 vertices, 4 triangles
        let l = [
            pos2(0.0, 0.0),
            pos2(2.0, 0.0),
            pos2(2.0, 1.0),
            pos2(1.0, 1.0),
            pos2(1.0, 2.0),
            pos2(0.0, 2.0),
        ];
        let mesh = fill_mesh(&l, Color32::WHITE);
        assert_eq!(mesh.indices.len(), 4 * 3);
    }
}
