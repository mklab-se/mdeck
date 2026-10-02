//! Where the slides on screen are drawn: one slide, or two during a
//! transition (fade, slide, spatial, or the zoom into a thermal spot), and
//! the background images that move with them.

use eframe::egui;

use super::PresentationApp;
use crate::render::transition::TransitionKind;

/// One slide on screen: where, how opaque, its scroll, and how much it is
/// magnified (the zoom transition).
pub(super) struct Placement {
    pub index: usize,
    pub rect: egui::Rect,
    pub opacity: f32,
    pub scroll: f32,
    pub zoom: f32,
}

/// The zoom transition: the outgoing slide magnifies around the anchor and
/// fades; the incoming one settles from slightly magnified around the same
/// point. `anchor` is a fraction of the slide rect.
fn zoom_placements(
    rect: egui::Rect,
    (fx, fy): (f32, f32),
    (from, to): (usize, usize),
    from_scroll: f32,
    p: f32,
) -> Vec<Placement> {
    let anchor = rect.min + egui::vec2(fx * rect.width(), fy * rect.height());
    let around = |z: f32| {
        egui::Rect::from_min_max(
            anchor + (rect.min - anchor) * z,
            anchor + (rect.max - anchor) * z,
        )
    };
    let out_zoom = 1.0 + 2.4 * p;
    let in_zoom = 1.25 - 0.25 * p;
    vec![
        Placement {
            index: from,
            rect: around(out_zoom),
            opacity: (1.0 - p * 1.4).clamp(0.0, 1.0),
            scroll: from_scroll,
            zoom: out_zoom,
        },
        Placement {
            index: to,
            rect: around(in_zoom),
            opacity: ((p - 0.25) / 0.75).clamp(0.0, 1.0),
            scroll: 0.0,
            zoom: in_zoom,
        },
    ]
}

impl PresentationApp {
    /// Find where the spot a zoom transition targets was drawn, once; without
    /// it on the slide being left, the transition is an ordinary one.
    pub(super) fn resolve_zoom(&mut self, ctx: &egui::Context) {
        let rect = self.last_slide_rect;
        let Some(t) = self.transition.as_mut() else {
            return;
        };
        let Some(zoom) = t.zoom.as_mut() else {
            return;
        };
        if zoom.anchor.is_some() {
            return;
        }
        match crate::render::thermal::spot_anchor(ctx, &zoom.spot) {
            Some(p) if rect.width() > 0.0 && rect.contains(p) => {
                zoom.anchor = Some((
                    (p.x - rect.left()) / rect.width(),
                    (p.y - rect.top()) / rect.height(),
                ));
            }
            _ => t.zoom = None,
        }
    }

    /// Where each slide on screen is drawn in `rect`: the current one, or
    /// the outgoing and incoming slides mid-transition (outgoing first).
    pub(super) fn slide_placements(&self, rect: egui::Rect) -> Vec<Placement> {
        use crate::render::transition::TransitionDirection;
        let at = |index, rect, opacity, scroll| Placement {
            index,
            rect,
            opacity,
            scroll,
            zoom: 1.0,
        };
        let Some(t) = &self.transition else {
            return vec![at(self.current_slide, rect, 1.0, 0.0)];
        };
        let (from, to) = (t.from, t.to);
        let progress = t.progress();
        // The outgoing slide keeps its scroll position while it leaves
        let from_scroll = self.view(from).scroll;
        if let Some(anchor) = t.zoom.as_ref().and_then(|z| z.anchor) {
            return zoom_placements(rect, anchor, (from, to), from_scroll, progress);
        }
        let kind = self.drawn_transition(t.kind);
        let forward = t.direction == TransitionDirection::Forward;
        let spatial = t.spatial_direction(super::GridLayout::columns(self.slide_count()));
        let sides = crate::render::transition::sides(kind, progress, forward, rect, spatial);
        let leaving = sides.len() > 1;
        sides
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                let (index, scroll) = if leaving && i == 0 {
                    (from, from_scroll)
                } else {
                    (to, 0.0)
                };
                Placement {
                    index,
                    rect: s.rect,
                    opacity: s.opacity,
                    scroll,
                    zoom: s.zoom,
                }
            })
            .collect()
    }

    /// The transition as drawn: a board turns its own flaps from one slide
    /// to the next.
    pub(super) fn drawn_transition(&self, kind: TransitionKind) -> TransitionKind {
        if self.theme.engine.is_board() {
            TransitionKind::None
        } else {
            kind
        }
    }

    /// Let an extension transition paint over both slides.
    pub(super) fn paint_transition_over(&self, ui: &egui::Ui, rect: egui::Rect) {
        use crate::render::transition::TransitionDirection;
        let Some(t) = &self.transition else {
            return;
        };
        if t.zoom.as_ref().is_some_and(|z| z.anchor.is_some()) {
            return;
        }
        crate::render::transition::paint_over(
            self.drawn_transition(t.kind),
            ui,
            &self.theme,
            rect,
            t.progress(),
            t.direction == TransitionDirection::Forward,
        );
    }

    /// The background images of the slides on screen, moving with their
    /// slides and clipped to `rect` (the sheet on a page theme).
    pub(super) fn draw_backgrounds(&self, ui: &egui::Ui, rect: egui::Rect, scale: f32) {
        let painter = ui.painter().with_clip_rect(rect);
        let radius = self.theme.page.as_ref().map_or(0.0, |p| p.radius * scale);
        for p in self.slide_placements(rect) {
            self.deck.draw_background(
                &painter,
                p.rect,
                p.index,
                p.opacity,
                radius,
                !self.reduced_motion,
            );
        }
    }
}
