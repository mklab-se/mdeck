//! Painting a RaTeX display list with epaint: KaTeX-font glyphs, rules,
//! rectangles, and vector paths (flattened and ear-clipped, since epaint only
//! fills convex shapes).

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Stroke, pos2, vec2};
use ratex_types::display_item::{DisplayItem, DisplayList};
use ratex_types::path_command::PathCommand;

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
    use eframe::egui::pos2;

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
