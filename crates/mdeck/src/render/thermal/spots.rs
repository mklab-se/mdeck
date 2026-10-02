//! The spots of a `@thermal` block: crosshairs, labels that keep out of
//! each other's way, measured or author-supplied values, and where each was
//! drawn (for the zoom transition).

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use super::draw::Drawer;
use super::legend::format_value;
use super::source::{Kind, Mark};
use super::state::SpotAt;
use crate::theme::Theme;

impl Drawer<'_> {
    /// A spot: a crosshair, its name and its value. Returns whether the
    /// value was supplied by the author (marked †).
    pub(super) fn spot(&self, spot: &SpotAt, p: f32) -> bool {
        let cx = self.cx;
        let scale = cx.scale;
        let painter = cx.ui.painter();
        let a = p * cx.opacity;
        let at = Pos2::new(
            self.img.left() + spot.x * self.img.width(),
            self.img.top() + spot.y * self.img.height(),
        );
        remember_spot(cx.ui.ctx(), &spot.name, at);
        let white = Color32::WHITE;
        let r = 16.0 * scale;
        let stroke = Stroke::new(2.5 * scale, Theme::with_opacity(white, a));
        // a dark halo under the crosshair keeps it readable on any colour
        let halo = Stroke::new(5.5 * scale, Theme::with_opacity(Color32::BLACK, a * 0.45));
        for s in [halo, stroke] {
            painter.circle_stroke(at, r, s);
            for d in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                painter.line_segment([at + d * r * 0.45, at + d * r * 1.6], s);
            }
        }
        let (value, authored) = self.spot_value(spot);
        let text = match &value {
            Some(v) => format!("{}  {}", spot.name, v),
            None => spot.name.clone(),
        };
        let g = painter.layout_no_wrap(
            text,
            FontId::new(cx.theme.body_size * 0.55 * scale, cx.theme.mono_family()),
            Theme::with_opacity(white, a),
        );
        let pad = Vec2::new(10.0, 5.0) * scale;
        let mut box_min = at + Vec2::new(r * 1.8, -r * 1.8 - g.size().y);
        // keep the label inside the image
        if box_min.x + g.size().x + pad.x * 2.0 > self.img.right() {
            box_min.x = at.x - r * 1.8 - g.size().x - pad.x * 2.0;
        }
        box_min.y = box_min.y.max(self.img.top() + 4.0 * scale);
        let mut bg = Rect::from_min_size(box_min, g.size() + pad * 2.0);
        // step clear of labels already drawn: below them, else above
        let mut labels = self.labels.borrow_mut();
        for _ in 0..labels.len() {
            let Some(hit) = labels.iter().find(|l| l.expand(2.0 * scale).intersects(bg)) else {
                break;
            };
            let below = bg.translate(Vec2::new(0.0, hit.bottom() + 4.0 * scale - bg.top()));
            bg = if below.bottom() <= self.img.bottom() {
                below
            } else {
                bg.translate(Vec2::new(0.0, hit.top() - 4.0 * scale - bg.bottom()))
            };
        }
        labels.push(bg);
        painter.rect_filled(
            bg,
            4.0 * scale,
            Theme::with_opacity(Color32::from_gray(8), a * 0.85),
        );
        painter.galley(bg.min + pad, g, Theme::with_opacity(white, a));
        authored
    }

    /// A spot's value: the author's text (marked †) or, from a mapped or
    /// data source, the measured value (≈ for a linear mapping).
    fn spot_value(&self, spot: &SpotAt) -> (Option<String>, bool) {
        if let Some(t) = &spot.text {
            return (Some(format!("{t}†")), true);
        }
        let Some(unit) = self.source.unit() else {
            return (None, false);
        };
        let (v, mark) = self.source.sample(spot.x, spot.y);
        let span = self.window.1 - self.window.0;
        let text = match mark {
            Mark::Missing => "no data".to_string(),
            Mark::ClippedHigh => format!("≥ {}", format_value(v, span, unit)),
            Mark::ClippedLow => format!("≤ {}", format_value(v, span, unit)),
            Mark::None if !v.is_finite() => "no data".to_string(),
            Mark::None => {
                let approx = matches!(self.source.kind, Kind::Linear(_));
                let decimals = if self.source.step < 0.2 { 1 } else { 0 };
                format!(
                    "{}{:.*} {}",
                    if approx { "≈ " } else { "" },
                    decimals,
                    v,
                    unit.symbol()
                )
            }
        };
        (Some(text), false)
    }
}

/// Where each spot was last drawn (screen) and in which frame, for the
/// zoom transition.
fn remember_spot(ctx: &egui::Context, name: &str, at: Pos2) {
    let id = egui::Id::new(("thermal-spot", name.to_ascii_lowercase()));
    let frame = ctx.cumulative_frame_nr();
    ctx.data_mut(|d| d.insert_temp(id, (at, frame)));
}

/// Where the spot `name` was drawn in the last frame or two, if it was:
/// an older position belongs to another slide.
pub fn spot_anchor(ctx: &egui::Context, name: &str) -> Option<Pos2> {
    let id = egui::Id::new(("thermal-spot", name.to_ascii_lowercase()));
    let now = ctx.cumulative_frame_nr();
    ctx.data(|d| d.get_temp::<(Pos2, u64)>(id))
        .filter(|(_, frame)| now.saturating_sub(*frame) <= 2)
        .map(|(p, _)| p)
}
