//! Picking up edits to the deck file while presenting.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::parser::{self, Presentation};
use crate::theme::lookup;
use crate::{deck, render};

use super::helpers::{find_matching_slide, hash_content};
use super::input::ActiveDraw;
use super::toast::Toast;
use super::{AppMode, PresentationApp, SlideView};

impl PresentationApp {
    pub(super) fn reload_presentation(&mut self) {
        let content = match std::fs::read_to_string(&self.deck.file) {
            Ok(c) => c,
            Err(e) => {
                self.incident_log.record(
                    "file_reload_error",
                    "failed to read presentation file for reload",
                    &format!("{e}\npath: {}", self.deck.file.display()),
                );
                self.toast = Some(Toast::new(format!("Reload error: {e}")));
                return;
            }
        };

        // Skip reload if file content hasn't actually changed (macOS FSEvents
        // can fire spuriously, and each reload resets per-slide state).
        let new_hash = hash_content(&content);
        if new_hash == self.last_content_hash {
            return;
        }
        self.last_content_hash = new_hash;

        let new_presentation = parser::parse(&content);

        if new_presentation.slides.is_empty() {
            self.toast = Some(Toast::new("Reload: no slides found".to_string()));
            return;
        }

        self.apply_reloaded(new_presentation);
    }

    /// Swap in a re-parsed presentation, preserving as much per-slide state
    /// (position, reveal progress, scroll) as still makes sense.
    pub(super) fn apply_reloaded(&mut self, new_presentation: Presentation) {
        // Preserve slide position
        let old_current = self.current_slide;
        let old_raw = self
            .deck
            .presentation
            .slides
            .get(old_current)
            .map(|s| s.raw_source.as_str());
        let old_reveal = self.view(old_current).reveal;
        let old_scroll = self.view(old_current).scroll_target;
        self.current_slide = find_matching_slide(old_raw, old_current, &new_presentation.slides);

        let slide_count = new_presentation.slides.len();

        // Recompute per-slide vectors, keeping the current slide's reveal
        // progress (clamped to the new step count) and scroll position.
        let engine_override = deck::deck_engine(self.cli_engine, &new_presentation, false);
        let new_theme = new_presentation.meta.theme.clone();
        self.deck.replace(new_presentation, &self.theme);
        self.views = vec![SlideView::default(); slide_count];
        let cur = self.current_slide;
        self.views[cur] = SlideView {
            reveal: old_reveal.min(self.deck.max_steps[cur]),
            revealed_at: None,
            scroll: old_scroll,
            scroll_target: old_scroll,
        };

        // Update the theme from new frontmatter (and pick up edits to the
        // theme file itself); `--theme` keeps its theme. The transition is
        // resolved from the deck on every slide change.
        if let Some(name) = self.cli_theme.clone() {
            let (theme, problems) = lookup::resolve_or_default(&self.themes, &name);
            deck::report_theme_problems(&problems);
            self.pending_theme = Some(theme);
        } else if let Some(name) = &new_theme {
            let (theme, problems) = lookup::resolve_or_default(&self.themes, name);
            deck::report_theme_problems(&problems);
            self.theme_key = name.trim().to_ascii_lowercase();
            self.pending_theme = Some(theme);
        } else if engine_override != self.engine_override {
            let (theme, _) = lookup::resolve_or_default(&self.themes, &self.theme_key);
            self.pending_theme = Some(theme);
        }
        self.engine_override = engine_override;

        self.jobs.precache_cancel.store(true, Ordering::Relaxed);
        render::diagram::clear_route_cache();
        render::visualizations::word_cloud::clear_cache();
        self.jobs.precache_cancel = Arc::new(AtomicBool::new(false));
        self.transition = None;
        self.pending_nav = None;
        self.leave_end_slide();
        self.ink.strokes.clear();
        self.ink.arrows.clear();
        self.ink.active = ActiveDraw::None;

        // Clamp grid selection (both the grid and its zoom animation carry one)
        match self.mode {
            AppMode::Grid { ref mut selected }
            | AppMode::OverviewTransition {
                ref mut selected, ..
            } => {
                *selected = (*selected).min(slide_count.saturating_sub(1));
            }
            AppMode::Presentation { .. } => {}
        }

        self.toast = Some(Toast::new("Presentation Change Detected".to_string()));

        self.spawn_diagram_precache();
    }

    /// Collect all diagram content from every slide and spawn a background thread
    /// to pre-compute their routing caches at reference resolution (1920x1080).
    pub(super) fn spawn_diagram_precache(&mut self) {
        let diagrams: Vec<(usize, String)> = self
            .deck
            .presentation
            .slides
            .iter()
            .enumerate()
            .flat_map(|(i, s)| {
                s.blocks.iter().filter_map(move |b| {
                    if let parser::Block::Diagram { content, .. } = b {
                        Some((i + 1, content.clone()))
                    } else {
                        None
                    }
                })
            })
            .collect();

        if diagrams.is_empty() {
            return;
        }

        let rx = render::diagram::precache_all_diagrams_with_report(
            diagrams,
            self.jobs.precache_cancel.clone(),
        );
        self.jobs.precache_report = Some(rx);
        self.jobs.report_printed = false;
    }
}
