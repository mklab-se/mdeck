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

use eframe::egui::{self, Color32, Pos2, Stroke, vec2};
use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_types::display_item::DisplayList;
use ratex_types::math_style::MathStyle;

mod paint;
pub use paint::paint_list;

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
    /// Each formula and whether it is display math.
    formulas: Vec<(Laid, bool)>,
}

static REGISTRY: LazyLock<Mutex<Registry>> = LazyLock::new(Default::default);

/// Lay out `tex` (display or inline style), cached for the process.
pub fn lay_out(tex: &str, display: bool) -> (usize, Laid) {
    let mut reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&id) = reg.ids.get(&(tex.to_string(), display)) {
        return (id, reg.formulas[id].0.clone());
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
    reg.formulas.push((laid.clone(), display));
    reg.ids.insert((tex.to_string(), display), id);
    (id, laid)
}

fn is_display(id: usize) -> bool {
    let reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    reg.formulas.get(id).is_some_and(|f| f.1)
}

fn formula(id: usize) -> Option<Laid> {
    let reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    reg.formulas.get(id).map(|f| f.0.clone())
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
    // A formula wider than the text column shrinks to fit rather than wrap.
    let column = job.wrap.max_width;
    let natural = dl.width as f32 * size * EM_SCALE;
    let em = if column.is_finite() && natural > column && dl.width > 0.0 {
        column / dl.width as f32
    } else {
        size * EM_SCALE
    };
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
        let text_baseline = text_baseline(&placed.row);
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
            let mut left = right - width;
            // Display formulas are centred in a left-aligned column.
            if is_display(id) && job.halign == egui::Align::LEFT && job.wrap.max_width.is_finite() {
                left = ((job.wrap.max_width - width) / 2.0).max(0.0) - placed.pos.x;
            }
            let origin = pos + placed.pos.to_vec2() + vec2(left, baseline);
            paint_list(painter, origin, em, tint.unwrap_or(format.color), &dl);
        }
    }
}

/// The baseline of a row's text, from a glyph that is not part of a formula
/// placeholder; `None` when the row holds only formulas.
fn text_baseline(row: &egui::epaint::text::Row) -> Option<f32> {
    row.glyphs
        .iter()
        .find(|g| g.chr != MARK && g.chr != END && digit_value(g.chr).is_none())
        .map(|g| g.pos.y)
}

/// Distance from a galley's top to the baseline its first line's text (or
/// formula) sits on.
pub fn first_baseline(galley: &egui::Galley) -> Option<f32> {
    let row = galley.rows.first()?;
    let y = text_baseline(&row.row).or_else(|| row.row.glyphs.first().map(|g| g.pos.y))?;
    Some(row.pos.y + y)
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
}
