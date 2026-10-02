//! Drawing a `@thermal` block: the picture (the visible photo, the thermal
//! image through a lens, or the thermal image), its legend, spots and
//! label. Every engine draws it the same way; an engine only paints around
//! it.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};

use super::compose::Look;
use super::lens::{Shape, coverage, textured_circle};
use super::palette::Palette;
use super::source::{Kind, Source};
use super::spec::{Spec, Threshold};
use super::state::State;
use super::{Library, palette_for};
use crate::render::BlockCx;
use crate::render::hints::{self, Hint};
use crate::theme::Theme;

/// Seconds for a threshold to change and a spot to fade in (the lens has
/// its own timings, in `lens`).
const FADE: f32 = 0.5;

/// Height when the slide does not give the block one.
const DEFAULT_HEIGHT: f32 = 620.0;

/// Draw `content` at `pos` within `max_width` by `max_height` (`0.0`: as
/// tall as it needs). Returns the height used.
pub fn draw(cx: &BlockCx, content: &str, pos: Pos2, max_width: f32, max_height: f32) -> f32 {
    let spec = Spec::parse(content);
    let lib = cx.image_cache.thermal();
    let scale = cx.scale;
    let height = if max_height > 0.0 {
        max_height
    } else {
        DEFAULT_HEIGHT * scale
    };
    let area = Rect::from_min_size(pos, Vec2::new(max_width, height));
    let source = match lib.source(&spec) {
        Some(Ok(s)) => s.clone(),
        Some(Err(e)) => {
            placeholder(cx, area, &format!("@thermal: {e}"));
            return height;
        }
        None => {
            placeholder(cx, area, "@thermal: no image: or data:");
            return height;
        }
    };
    let support = super::library::support_of(&source);
    let steps = spec.steps(&support);
    let state = State::at(&steps, cx.reveal_step);
    let palette = palette_for(cx.ui.ctx(), spec.palette);
    let window = window(&spec, &source);

    // layout: the picture, a legend on its right, a label and a note below
    let gap = 28.0 * scale;
    // the legend takes a share of a narrow block (a board's panel, a column)
    let legend_w = if source.chromatic {
        0.0
    } else {
        (max_width * 0.2).clamp(90.0 * scale, 150.0 * scale)
    };
    let text_h = 40.0 * scale;
    let avail = Vec2::new(
        (max_width - legend_w - if legend_w > 0.0 { gap } else { 0.0 }).max(10.0),
        (height - text_h).max(10.0),
    );
    let aspect = source.width as f32 / source.height as f32;
    let size = if avail.x / avail.y > aspect {
        Vec2::new(avail.y * aspect, avail.y)
    } else {
        Vec2::new(avail.x, avail.x / aspect)
    };
    let group_w = size.x + if legend_w > 0.0 { gap + legend_w } else { 0.0 };
    let left = area.left() + (max_width - group_w) / 2.0;
    let img = Rect::from_min_size(Pos2::new(left, area.top()), size);
    hints::push(
        cx.ui.ctx(),
        Hint::Frame(Rect::from_min_max(
            img.min,
            Pos2::new(left + group_w, img.bottom()),
        )),
    );

    let drawer = Drawer {
        cx,
        spec: &spec,
        source: &source,
        lib,
        img,
        palette,
        window,
        labels: Default::default(),
    };
    let opacity = cx.opacity;
    let painter = cx
        .ui
        .painter()
        .with_clip_rect(img.intersect(cx.ui.clip_rect()));

    // the base: the visible photo, or the thermal image dimmed to gray
    let thermal_from_start = !state.reveals;
    if !thermal_from_start {
        match &spec.visible {
            Some(v) => match cx.image_cache.get_or_load(cx.ui, v) {
                Some(tex) => image(
                    &painter,
                    tex.id(),
                    img,
                    Color32::from_white_alpha(alpha(opacity)),
                ),
                None => {
                    painter.rect_filled(img, 0.0, Color32::from_gray(10));
                }
            },
            None => drawer.thermal(&painter, img, Some(f32::INFINITY), 1.0, None),
        }
    }

    // the thermal layer: everywhere, through the lens, or opening up
    let threshold_now = state
        .threshold
        .as_ref()
        .and_then(|(t, _, _)| drawer.threshold(t));
    let (layer_alpha, shape) = coverage(cx, &state, img, thermal_from_start);
    if let Some(shape) = shape {
        // a changing threshold cross-fades from the one before
        let mut fade_from = None;
        if let Some((_, prev, step)) = &state.threshold {
            let p = progress(cx, *step, FADE);
            if p < 1.0 {
                fade_from = Some((prev.as_ref().and_then(|t| drawer.threshold(t)), p));
            }
        }
        match fade_from {
            Some((prev, p)) => {
                drawer.thermal(&painter, img, prev, layer_alpha, Some(shape));
                drawer.thermal(&painter, img, threshold_now, layer_alpha * p, Some(shape));
            }
            None => drawer.thermal(&painter, img, threshold_now, layer_alpha, Some(shape)),
        }
        if let Shape::Circle {
            center,
            radius,
            ring,
        } = shape
            && ring > 0.0
        {
            let a = ring * layer_alpha;
            let warm = Color32::from_rgb(255, 244, 220);
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(3.0 * scale, Theme::with_opacity(warm, opacity * a)),
            );
            painter.circle_stroke(
                center,
                radius + 5.0 * scale,
                Stroke::new(8.0 * scale, Theme::with_opacity(warm, opacity * a * 0.12)),
            );
        }
    }

    // the frame line, quiet
    cx.ui.painter().rect_stroke(
        img,
        0.0,
        Stroke::new(
            1.0 * scale,
            Theme::with_opacity(cx.theme.foreground, opacity * 0.12),
        ),
        egui::StrokeKind::Outside,
    );

    // the legend, once the thermal image shows
    if legend_w > 0.0 && state.thermal_visible() {
        let legend = Rect::from_min_size(
            Pos2::new(img.right() + gap, img.top()),
            Vec2::new(legend_w, img.height()),
        );
        drawer.legend(
            legend,
            threshold_now,
            layer_alpha.max(if thermal_from_start { 1.0 } else { 0.0 }),
        );
    }

    // spots, the label and the note about author-supplied values
    let mut authored = false;
    for spot in &state.spots {
        authored |= drawer.spot(spot, progress(cx, spot.step, FADE));
    }
    let mut y = img.bottom() + 10.0 * scale;
    let small = FontId::new(cx.theme.body_size * 0.5 * scale, cx.theme.mono_family());
    if let Some(label) = &spec.label {
        let g = cx.ui.painter().layout_no_wrap(
            label.clone(),
            FontId::new(cx.theme.body_size * 0.6 * scale, cx.theme.body_family()),
            fg(cx, 0.85),
        );
        let h = g.size().y;
        cx.ui
            .painter()
            .galley(Pos2::new(img.left(), y), g, fg(cx, 0.85));
        y += h + 4.0 * scale;
    }
    let mut notes = Vec::new();
    if authored {
        notes.push("† value supplied by the author".to_string());
    }
    if source.chromatic {
        notes.push("shown as exported (colour input)".to_string());
    }
    if !notes.is_empty() {
        let g = cx
            .ui
            .painter()
            .layout_no_wrap(notes.join("   "), small, fg(cx, 0.55));
        cx.ui
            .painter()
            .galley(Pos2::new(img.left(), y), g, fg(cx, 0.55));
    }
    height
}

/// Progress (0..1) of something that appeared on `step`, over `seconds`;
/// settled when it is not the newest step or nothing animates.
pub(super) fn progress(cx: &BlockCx, step: usize, seconds: f32) -> f32 {
    if step != cx.reveal_step || step == 0 {
        return 1.0;
    }
    let Some(ts) = cx.reveal_timestamp else {
        return 1.0;
    };
    let t = ts.elapsed().as_secs_f32() / seconds;
    if t < 1.0 {
        cx.ui.ctx().request_repaint();
    }
    t.clamp(0.0, 1.0)
}

/// The foreground at `a` of the block's opacity.
pub(super) fn fg(cx: &BlockCx, a: f32) -> Color32 {
    Theme::with_opacity(cx.theme.foreground, cx.opacity * a)
}

pub(super) fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn alpha(a: f32) -> u8 {
    (a.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn image(painter: &egui::Painter, tex: egui::TextureId, rect: Rect, tint: Color32) {
    painter.image(
        tex,
        rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        tint,
    );
}

fn placeholder(cx: &BlockCx, area: Rect, message: &str) {
    let r = area.shrink(20.0 * cx.scale);
    cx.ui.painter().rect_filled(
        r,
        8.0 * cx.scale,
        Theme::with_opacity(cx.theme.code_background, cx.opacity),
    );
    let g = cx.ui.painter().layout(
        message.to_string(),
        FontId::new(cx.theme.body_size * 0.5 * cx.scale, cx.theme.mono_family()),
        fg(cx, 0.7),
        r.width() - 40.0 * cx.scale,
    );
    cx.ui
        .painter()
        .galley(r.center() - g.size() / 2.0, g, fg(cx, 0.7));
}

/// The window (in the source's unit) the palette spans: the block's (or
/// the slide's) when it can apply, else the mapping's range, the data's
/// extent, or 0..1 for a display image.
pub fn window(spec: &Spec, source: &Source) -> (f32, f32) {
    let fallback = match &source.kind {
        Kind::Display => (0.0, 1.0),
        Kind::Linear(r) => (r.lo, r.hi),
        Kind::Data(_) => (source.min, source.max.max(source.min + f32::EPSILON)),
    };
    match (&spec.window, source.unit()) {
        (Some(w), Some(unit)) => w.to_unit(unit).map_or(fallback, |w| (w.lo, w.hi)),
        _ => fallback,
    }
}

pub(super) struct Drawer<'a> {
    pub(super) cx: &'a BlockCx<'a>,
    pub(super) spec: &'a Spec,
    pub(super) source: &'a Source,
    pub(super) lib: &'a Library,
    pub(super) img: Rect,
    pub(super) palette: Palette,
    pub(super) window: (f32, f32),
    /// Spot labels drawn so far, so the next one keeps out of their way.
    pub(super) labels: std::cell::RefCell<Vec<Rect>>,
}

impl Drawer<'_> {
    /// A threshold in the source's unit (relative ones over the window).
    fn threshold(&self, t: &Threshold) -> Option<f32> {
        match t {
            Threshold::Relative(f) => Some(self.window.0 + f * (self.window.1 - self.window.0)),
            Threshold::Value(v, unit) => unit.convert(*v, self.source.unit()?),
        }
    }

    /// Paint the thermal layer (thresholded) over `img`, all of it or a
    /// circle of it.
    fn thermal(
        &self,
        painter: &egui::Painter,
        img: Rect,
        threshold: Option<f32>,
        a: f32,
        shape: Option<Shape>,
    ) {
        let tint = Color32::from_white_alpha(alpha(a * self.cx.opacity));
        let tex = if self.source.chromatic {
            // colour input is shown as exported
            match self.spec.image.as_deref() {
                Some(p) => match self.cx.image_cache.get_or_load(self.cx.ui, p) {
                    Some(t) => t,
                    None => return,
                },
                None => return,
            }
        } else {
            let look = Look {
                palette: self.palette,
                window: self.window,
                threshold,
            };
            match self
                .lib
                .texture(self.cx.ui.ctx(), self.spec, self.source, &look)
            {
                Some(t) => t,
                None => return,
            }
        };
        match shape.unwrap_or(Shape::Full) {
            Shape::Full => image(painter, tex.id(), img, tint),
            Shape::Circle { center, radius, .. } => {
                textured_circle(painter, tex.id(), img, center, radius, tint)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::thermal::spec::{Range, Unit};

    fn source(kind: Kind) -> Source {
        Source {
            width: 2,
            height: 1,
            values: vec![20.0, 80.0],
            marks: None,
            kind,
            chromatic: false,
            min: 20.0,
            max: 80.0,
            step: 0.1,
        }
    }

    #[test]
    fn the_window_needs_a_unit_and_converts_into_it() {
        let mut spec = Spec::parse("image: a.png\nwindow: 40..90 °C\n");
        let display = source(Kind::Display);
        assert_eq!(
            window(&spec, &display),
            (0.0, 1.0),
            "ignored without a mapping"
        );
        let linear = source(Kind::Linear(Range::parse("10..110 °C").unwrap()));
        assert_eq!(window(&spec, &linear), (40.0, 90.0));
        let data_f = source(Kind::Data(Unit::Fahrenheit));
        let (lo, hi) = window(&spec, &data_f);
        assert!((lo - 104.0).abs() < 0.01 && (hi - 194.0).abs() < 0.01);
        spec.window = None;
        assert_eq!(window(&spec, &linear), (10.0, 110.0), "the mapping's range");
        assert_eq!(
            window(&spec, &source(Kind::Data(Unit::Celsius))),
            (20.0, 80.0),
            "the data's extent"
        );
    }
}
