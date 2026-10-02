use eframe::egui;

use crate::deck::SlideFrame;
use crate::render;
use crate::theme::Theme;

use super::PresentationApp;
use super::keys::{SCROLL_SMOOTH_RATE, scroll_target_to_show, smooth_factor};
use super::overlays::draw_fade_gradient;

/// One slide on screen: where, how opaque, and its scroll.
pub(super) struct Placement {
    pub index: usize,
    pub rect: egui::Rect,
    pub opacity: f32,
    pub scroll: f32,
}

impl PresentationApp {
    /// Draw a slide at its current reveal step, scrolled by `scroll` pixels
    /// and clipped to `rect` when scrolled.
    pub(super) fn draw_slide(
        &self,
        ui: &mut egui::Ui,
        index: usize,
        rect: egui::Rect,
        opacity: f32,
        scale: f32,
        scroll: f32,
    ) {
        if index >= self.deck.presentation.slides.len() {
            return;
        }
        let cx = self.slide_context(index);
        let frame = |rect| SlideFrame {
            rect,
            opacity,
            reveal: self.view(index).reveal,
            reveal_timestamp: self.view(index).revealed_at,
            scale,
        };
        if scroll.abs() < 0.5 {
            self.deck
                .draw_slide(ui, &self.theme, index, frame(rect), &cx);
            return;
        }
        // Scrolled: render into a child clipped to the slide rect so content
        // above/below the viewport never bleeds into other elements.
        // (`max_rect` only affects layout; clipping must be set explicitly.)
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .id_salt(("scrolled_slide", index)),
        );
        child.shrink_clip_rect(rect);
        let scrolled = rect.translate(egui::vec2(0.0, -scroll));
        self.deck
            .draw_slide(&child, &self.theme, index, frame(scrolled), &cx);
    }

    /// Facts about the deck shown by the Ember eyebrow and chrome.
    pub(super) fn slide_context(&self, index: usize) -> render::SlideContext {
        render::SlideContext {
            index,
            count: self.slide_count(),
            deck_title: self.deck.presentation.meta.title.clone(),
            author: self.deck.presentation.meta.author.clone(),
            hold_copy: self.countdown_running(),
            animate: true,
            beats: self
                .story(index)
                .filter(|s| s.beats.len() > 1)
                .map(|s| (self.view(index).reveal, s.beats.len())),
            say: self
                .story(index)
                .and_then(|s| s.line(self.view(index).reveal).map(str::to_string)),
            engine_drew: self.theme.engine.paints()
                && matches!(self.mode, super::AppMode::Presentation { .. }),
        }
    }

    /// Bottom edge (relative to the content top) of the lowest element revealed
    /// by the current reveal step of slide `idx`, using the same measurements
    /// as the overflow detection. `None` when nothing new is below step 0.
    fn revealed_content_bottom(
        &self,
        ui: &egui::Ui,
        idx: usize,
        rect: egui::Rect,
        scale: f32,
    ) -> Option<f32> {
        let slide = &self.deck.presentation.slides[idx];
        let step = self.view(idx).reveal;
        let padding = 80.0 * scale;
        let content_width = match slide.layout {
            crate::parser::Layout::Code => rect.width() * 0.75,
            _ => rect.width() - padding * 2.0,
        };
        let heights: Vec<f32> = slide
            .blocks
            .iter()
            .map(|b| {
                render::text::measure_single_block_height(ui, b, &self.theme, content_width, scale)
            })
            .collect();
        let item_height = self.theme.body_size * scale + 8.0 * scale;
        super::helpers::revealed_bottom(&slide.blocks, &heights, step, item_height, 20.0 * scale)
    }

    /// Draw a slide at full reveal (all steps visible). Used by grid view.
    pub(super) fn draw_slide_fully_revealed(
        &self,
        ui: &egui::Ui,
        index: usize,
        rect: egui::Rect,
        opacity: f32,
        scale: f32,
    ) {
        if index < self.deck.presentation.slides.len() {
            let reveal = self.deck.max_steps.get(index).copied().unwrap_or(0);
            let mut cx = self.slide_context(index);
            cx.hold_copy = false;
            cx.animate = false;
            cx.engine_drew = false;
            let frame = SlideFrame {
                rect,
                opacity,
                reveal,
                reveal_timestamp: None,
                scale,
            };
            self.deck
                .draw_background(ui.painter(), rect, index, opacity, 4.0 * scale, false);
            self.deck.draw_slide(ui, &self.theme, index, frame, &cx);
        }
    }

    pub(super) fn draw_presentation_with_scroll(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rect: egui::Rect,
        scale: f32,
    ) {
        // Cache slide rect for mouse coordinate conversion
        self.last_slide_rect = rect;

        // During transitions the outgoing slide keeps its scroll (see draw_presentation)
        if self.transition.is_some() {
            self.draw_presentation(ui, ctx, rect, scale);
            self.draw_annotations(ui, scale);
            return;
        }

        let idx = self.current_slide;
        let slide = &self.deck.presentation.slides[idx];
        let (content_height, available_height) =
            render::measure_slide_content_height(ui, slide, &self.theme, rect, scale);
        let overflow = content_height - available_height;

        if overflow <= 0.0 {
            // No overflow: render normally, reset scroll
            self.views[idx].reset_scroll();
            self.pending_reveal_scroll = false;
            self.draw_presentation(ui, ctx, rect, scale);
            self.draw_annotations(ui, scale);
            return;
        }

        // A reveal step was just added: scroll so the newly revealed element is visible
        if self.pending_reveal_scroll {
            self.pending_reveal_scroll = false;
            if let Some(bottom) = self.revealed_content_bottom(ui, idx, rect, scale)
                && let Some(target) = scroll_target_to_show(
                    bottom,
                    self.views[idx].scroll_target,
                    available_height,
                    overflow,
                    40.0 * scale,
                )
            {
                self.views[idx].scroll_target = target;
            }
        }

        let scroll_offset = self.animate_slide_scroll(ctx, idx, overflow);

        // Render slide clipped to the slide rect so content doesn't bleed outside
        self.draw_slide(ui, idx, rect, 1.0, scale, scroll_offset);

        self.draw_scroll_hints(ui, rect, scale, scroll_offset, overflow);

        // Draw annotations on top of slide content
        self.draw_annotations(ui, scale);

        // Footer, counter, FPS
        self.draw_presentation_chrome(ui, rect, scale);
    }

    /// Ease slide `idx`'s scroll toward its target, clamped to `overflow`
    /// (frame-rate independent). Returns the offset to draw with.
    fn animate_slide_scroll(&mut self, ctx: &egui::Context, idx: usize, overflow: f32) -> f32 {
        // Clamp target
        self.views[idx].scroll_target = self.views[idx].scroll_target.clamp(0.0, overflow);

        // Animate: ease current offset toward target (frame-rate independent)
        let target = self.views[idx].scroll_target;
        let current = self.views[idx].scroll;
        let diff = target - current;
        if diff.abs() < 0.5 {
            self.views[idx].scroll = target;
        } else {
            let dt = ctx.input(|i| i.stable_dt);
            self.views[idx].scroll = current + diff * smooth_factor(dt, SCROLL_SMOOTH_RATE);
            ctx.request_repaint();
        }
        self.views[idx].scroll
    }

    /// Fades at the edges that have more content past them, and the arrows
    /// that say so.
    fn draw_scroll_hints(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        scale: f32,
        scroll_offset: f32,
        overflow: f32,
    ) {
        // Draw fade-out gradient at bottom
        let fade_h = 80.0 * scale;
        if scroll_offset < overflow - 0.5 {
            draw_fade_gradient(ui, rect, fade_h, &self.theme, false);
        }
        // Draw fade-in gradient at top when scrolled
        if scroll_offset > 0.5 {
            draw_fade_gradient(ui, rect, fade_h, &self.theme, true);
        }

        // Draw scroll indicators
        let indicator_color = Theme::with_opacity(self.theme.foreground, 0.35);
        let indicator_font = egui::FontId::proportional(self.theme.body_size * 0.4 * scale);
        if scroll_offset < overflow - 0.5 {
            let galley = ui.painter().layout_no_wrap(
                "\u{25BC}".to_string(),
                indicator_font.clone(),
                indicator_color,
            );
            let pos = egui::pos2(
                rect.center().x - galley.rect.width() / 2.0,
                rect.bottom() - 40.0 * scale,
            );
            ui.painter().galley(pos, galley, indicator_color);
        }
        if scroll_offset > 0.5 {
            let galley = ui.painter().layout_no_wrap(
                "\u{25B2}".to_string(),
                indicator_font,
                indicator_color,
            );
            let pos = egui::pos2(
                rect.center().x - galley.rect.width() / 2.0,
                rect.top() + 10.0 * scale,
            );
            ui.painter().galley(pos, galley, indicator_color);
        }
    }

    pub(super) fn draw_presentation(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rect: egui::Rect,
        scale: f32,
    ) {
        for p in self.slide_placements(rect) {
            self.draw_slide(ui, p.index, p.rect, p.opacity, scale, p.scroll);
        }
        if self.transition.is_some() {
            ctx.request_repaint();
        }
        self.draw_presentation_chrome(ui, rect, scale);
    }

    /// Where each slide on screen is drawn in `rect`: the current one, or
    /// the outgoing and incoming slides mid-transition (outgoing first).
    pub(super) fn slide_placements(&self, rect: egui::Rect) -> Vec<Placement> {
        use crate::render::transition::{TransitionDirection, TransitionKind};
        let at = |index, rect, opacity, scroll| Placement {
            index,
            rect,
            opacity,
            scroll,
        };
        let Some(t) = &self.transition else {
            return vec![at(self.current_slide, rect, 1.0, 0.0)];
        };
        let (from, to) = (t.from, t.to);
        let progress = t.progress();
        // The outgoing slide keeps its scroll position while it leaves
        let from_scroll = self.view(from).scroll;
        // a board turns its own flaps from one slide to the next
        let kind = if self.theme.engine.is_board() {
            TransitionKind::None
        } else {
            t.kind
        };
        let (from_rect, to_rect) = match kind {
            TransitionKind::Fade => {
                return vec![
                    at(from, rect, 1.0 - progress, from_scroll),
                    at(to, rect, progress, 0.0),
                ];
            }
            TransitionKind::None => return vec![at(to, rect, 1.0, 0.0)],
            TransitionKind::SlideHorizontal => {
                let w = rect.width();
                let sign = match t.direction {
                    TransitionDirection::Forward => -1.0,
                    TransitionDirection::Backward => 1.0,
                };
                let from_offset = sign * progress * w;
                let to_offset = from_offset - sign * w;
                (
                    rect.translate(egui::vec2(from_offset, 0.0)),
                    rect.translate(egui::vec2(to_offset, 0.0)),
                )
            }
            TransitionKind::Spatial => {
                let (dx, dy) = t.spatial_direction(super::GridLayout::columns(self.slide_count()));
                let (w, h) = (rect.width(), rect.height());
                (
                    rect.translate(egui::vec2(-dx * progress * w, -dy * progress * h)),
                    rect.translate(egui::vec2(
                        dx * (1.0 - progress) * w,
                        dy * (1.0 - progress) * h,
                    )),
                )
            }
        };
        vec![
            at(from, from_rect, 1.0, from_scroll),
            at(to, to_rect, 1.0, 0.0),
        ]
    }

    /// The background images of the slides on screen, moving with their
    /// slides and clipped to `rect` (the sheet on a page theme).
    pub(super) fn draw_backgrounds(&self, ui: &egui::Ui, rect: egui::Rect, scale: f32) {
        let painter = ui.painter().with_clip_rect(rect);
        let radius = self.theme.page.as_ref().map_or(0.0, |p| p.radius * scale);
        for p in self.slide_placements(rect) {
            self.deck
                .draw_background(&painter, p.rect, p.index, p.opacity, radius, true);
        }
    }

    pub(super) fn draw_presentation_chrome(&self, ui: &egui::Ui, rect: egui::Rect, scale: f32) {
        // The logo stays put while slides move under it.
        if !self.countdown_running() {
            self.deck
                .draw_logo(ui.painter(), rect, self.current_slide, scale);
        }
        if self.theme.engine.capabilities().editorial {
            if !self.countdown_running() {
                render::ember::draw_chrome(
                    ui.painter(),
                    &self.theme,
                    rect,
                    &self.slide_context(self.current_slide),
                    scale,
                );
            }
            if self.show_hud {
                if let Some(line) = &self.slide_context(self.current_slide).say {
                    render::ember::draw_say_line(ui.painter(), &self.theme, rect, line, scale);
                }
                let fps_text = format!("{:.0} fps", self.fps.per_second);
                let fps_color = Theme::with_opacity(self.theme.foreground, 0.3);
                let fps_galley = ui.painter().layout_no_wrap(
                    fps_text,
                    egui::FontId::new(14.0 * scale, self.theme.mono_family()),
                    fps_color,
                );
                let fps_pos = egui::pos2(
                    rect.right() - fps_galley.rect.width() - 12.0 * scale,
                    rect.top() + 10.0 * scale,
                );
                ui.painter().galley(fps_pos, fps_galley, fps_color);
            }
            return;
        }
        // A board prints its own labels under the board.
        if self.theme.engine.is_board() {
            if self.show_hud {
                let fps_text = format!("{:.0} fps", self.fps.per_second);
                let fps_color = Theme::with_opacity(self.theme.foreground, 0.3);
                let fps_galley = ui.painter().layout_no_wrap(
                    fps_text,
                    egui::FontId::monospace(14.0 * scale),
                    fps_color,
                );
                let fps_pos = egui::pos2(
                    rect.right() - fps_galley.rect.width() - 12.0 * scale,
                    rect.top() + 10.0 * scale,
                );
                ui.painter().galley(fps_pos, fps_galley, fps_color);
            }
            return;
        }
        // Footer
        if let Some(ref footer) = self.deck.presentation.meta.footer {
            let footer_color = Theme::with_opacity(self.theme.foreground, 0.4);
            let galley = ui.painter().layout_no_wrap(
                footer.clone(),
                egui::FontId::proportional(14.0 * scale),
                footer_color,
            );
            let pos = egui::pos2(
                rect.center().x - galley.rect.width() / 2.0,
                rect.bottom() - 30.0 * scale,
            );
            ui.painter().galley(pos, galley, footer_color);
        }

        // Slide counter
        let counter_text = format!("{} / {}", self.current_slide + 1, self.slide_count());
        let counter_color = Theme::with_opacity(self.theme.foreground, 0.3);
        let counter_galley = ui.painter().layout_no_wrap(
            counter_text,
            egui::FontId::monospace(14.0 * scale),
            counter_color,
        );
        let counter_pos = egui::pos2(
            rect.right() - counter_galley.rect.width() - 16.0 * scale,
            rect.bottom() - 30.0 * scale,
        );
        ui.painter()
            .galley(counter_pos, counter_galley, counter_color);

        // FPS overlay: presenter-only, shown with the HUD (H) so the audience never sees it
        if !self.show_hud {
            return;
        }
        let fps_text = format!("{:.0} fps", self.fps.per_second);
        let fps_color = Theme::with_opacity(self.theme.foreground, 0.3);
        let fps_galley =
            ui.painter()
                .layout_no_wrap(fps_text, egui::FontId::monospace(14.0 * scale), fps_color);
        let fps_pos = egui::pos2(
            rect.right() - fps_galley.rect.width() - 12.0 * scale,
            rect.top() + 10.0 * scale,
        );
        ui.painter().galley(fps_pos, fps_galley, fps_color);
    }
}
