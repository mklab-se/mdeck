//! Moving through the deck: next and previous (reveal steps first), jumps,
//! and landing when a slide or grid transition completes.

use std::time::Instant;

use super::*;

impl PresentationApp {
    pub(super) fn navigate_forward(&mut self) {
        if self.transition.is_some() {
            self.pending_nav = Some(PendingNav::Forward);
            return;
        }

        // Already on end slide: nowhere to go
        if self.on_end_slide() {
            return;
        }

        let idx = self.current_slide;

        // If we have reveal steps remaining, reveal next item
        if self.views[idx].reveal < self.deck.max_steps[idx] {
            self.views[idx].reveal += 1;
            self.views[idx].revealed_at = Some(Instant::now());
            self.pending_reveal_scroll = true;
            return;
        }

        // On last real slide: transition to end slide
        if idx >= self.slide_count().saturating_sub(1) {
            self.views[idx].reset_scroll();
            self.mode = AppMode::Presentation { end: true };
            return;
        }

        // Scroll offsets are reset when the transition completes so the
        // outgoing slide keeps its scroll position while sliding out.
        self.transition = Some(ActiveTransition::new(
            idx,
            idx + 1,
            self.default_transition,
            TransitionDirection::Forward,
        ));
    }

    pub(super) fn navigate_backward(&mut self) {
        if self.transition.is_some() {
            self.pending_nav = Some(PendingNav::Backward);
            return;
        }

        // Coming back from end slide: return to last real slide
        if self.on_end_slide() {
            self.mode = AppMode::Presentation { end: false };
            return;
        }

        let idx = self.current_slide;

        // If we've revealed items, un-reveal. Only Next animates: clearing
        // the timestamp keeps what stays on screen from rising in again.
        if self.views[idx].reveal > 0 {
            self.views[idx].reveal -= 1;
            self.views[idx].revealed_at = None;
            return;
        }

        // Otherwise go to previous slide (fully revealed)
        if idx == 0 {
            return;
        }

        let prev = idx - 1;
        // Show previous slide fully revealed
        self.views[prev].reveal = self.deck.max_steps[prev];

        self.transition = Some(ActiveTransition::new(
            idx,
            prev,
            self.default_transition,
            TransitionDirection::Backward,
        ));
    }

    /// Jump directly to a slide (Home/End). The target is shown fully
    /// revealed and scrolled to the top, like `navigate_backward`.
    pub(super) fn jump_to_slide(&mut self, index: usize) {
        if index >= self.slide_count() || self.transition.is_some() {
            return;
        }
        self.leave_end_slide();
        self.views[index].reveal = self.deck.max_steps[index];
        let cur = self.current_slide;
        if index == cur {
            return;
        }
        self.views[index].reset_scroll();
        let direction = if index > cur {
            TransitionDirection::Forward
        } else {
            TransitionDirection::Backward
        };
        self.transition = Some(ActiveTransition::new(
            cur,
            index,
            self.default_transition,
            direction,
        ));
    }

    /// Finish a completed slide transition: land on the target slide, reset
    /// the outgoing slide's scroll, and apply any navigation queued meanwhile.
    pub(super) fn advance_transition(&mut self) {
        let Some(t) = self.transition.as_ref() else {
            return;
        };
        if !t.is_complete() {
            return;
        }
        let (from, to) = (t.from, t.to);
        self.transition = None;
        self.current_slide = to;
        if let Some(v) = self.views.get_mut(from) {
            v.reset_scroll();
        }
        if let Some(nav) = self.pending_nav.take() {
            match nav {
                PendingNav::Forward => self.navigate_forward(),
                PendingNav::Backward => self.navigate_backward(),
            }
        }
    }

    /// Finish a completed grid zoom animation.
    pub(super) fn advance_overview_transition(&mut self) {
        let AppMode::OverviewTransition { selected, entering } = self.mode else {
            return;
        };
        let Some(start) = self.overview_transition_start else {
            return;
        };
        if start.elapsed().as_secs_f32() < OVERVIEW_TRANSITION_DURATION {
            return;
        }
        let selected = selected.min(self.slide_count().saturating_sub(1));
        if entering {
            self.mode = AppMode::Grid { selected };
            // The hero slide zoomed out to its unscrolled cell; forget its scroll.
            let cur = self.current_slide;
            self.views[cur].reset_scroll();
        } else {
            self.current_slide = selected;
            // Leaving the grid behaves like navigating back: fully revealed, top.
            self.views[selected].reveal = self.deck.max_steps[selected];
            self.views[selected].reset_scroll();
            self.mode = AppMode::Presentation { end: false };
        }
        self.overview_transition_start = None;
    }
}
