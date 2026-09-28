//! `S` while presenting: AI for the current slide in the background, a
//! picture on an art engine or a story on the particles engine, and what
//! happens when the result comes back.

use std::sync::mpsc;

use super::{PresentationApp, Toast, load_stories, slide_max_steps};
use crate::render;
use crate::render::story::sidecar as story_sidecar;

impl PresentationApp {
    /// Re-read the sidecar and rebuild per-slide stories and step counts.
    pub(super) fn reload_stories(&mut self) {
        self.stories = load_stories(&self.file_path, &self.presentation, true);
        self.max_steps = slide_max_steps(
            &self.presentation,
            &self.stories,
            self.theme.engine.plays_stories(),
        );
        for (i, r) in self.reveal_steps.iter_mut().enumerate() {
            *r = (*r).min(self.max_steps[i]);
        }
        self.story_version += 1;
    }

    /// `S`: AI for the current slide, in the background: a picture on an
    /// art engine, a story on the particles engine.
    pub(super) fn generate(&mut self) {
        if self.theme.engine.medium().is_some() {
            self.generate_art();
        } else {
            self.generate_story();
        }
    }

    /// Draw the current slide's picture for the art engine on screen.
    pub(super) fn generate_art(&mut self) {
        if self.art_rx.is_some() {
            self.toast = Some(Toast::new("A picture is already being drawn…".into()));
            return;
        }
        if !crate::commands::ai::has_capability("image") {
            self.toast = Some(Toast::new(
                "AI images are not configured: run `mdeck ai config`".into(),
            ));
            return;
        }
        let idx = self.current_slide;
        let Some(slide) = self.presentation.slides.get(idx) else {
            return;
        };
        if !render::art::wants_art(slide) {
            self.toast = Some(Toast::new(
                "This slide takes no art (its layout has no room, or it says @art: none)".into(),
            ));
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.art_rx = Some(rx);
        let deck = self.file_path.clone();
        let theme = self.theme.clone();
        std::thread::spawn(move || {
            let result = crate::commands::art::generate_one_blocking(&deck, idx, &theme)
                .map_err(|e| e.to_string());
            let _ = tx.send((idx, result));
        });
        self.toast = Some(Toast::new(format!(
            "Drawing a picture for slide {}…",
            idx + 1
        )));
    }

    pub(super) fn poll_art(&mut self) {
        let Some(rx) = &self.art_rx else {
            return;
        };
        match rx.try_recv() {
            Ok((idx, Ok(()))) => {
                self.art_rx = None;
                self.art.invalidate();
                self.art.sync(&self.presentation, &self.theme);
                self.art.preload();
                self.toast = Some(Toast::new(format!("Picture ready for slide {}", idx + 1)));
            }
            Ok((idx, Err(e))) => {
                self.art_rx = None;
                self.incident_log
                    .record("art_error", &format!("slide {}", idx + 1), &e);
                self.toast = Some(Toast::new(format!("Drawing failed: {e}")));
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.art_rx = None;
            }
        }
    }

    /// `S` on the particles engine: write a story for the current slide.
    pub(super) fn generate_story(&mut self) {
        if !self.theme.engine.plays_stories() {
            self.toast = Some(Toast::new(
                "S writes stories on the particles engine and draws pictures on art engines (Shift+T)".into(),
            ));
            return;
        }
        if self.story_rx.is_some() {
            self.toast = Some(Toast::new("A story is already being written…".into()));
            return;
        }
        if !crate::commands::ai::has_capability("chat") {
            self.toast = Some(Toast::new(
                "AI is not configured: run `mdeck ai enable`".into(),
            ));
            return;
        }
        let idx = self.current_slide;
        if self
            .stories
            .get(idx)
            .and_then(|r| r.as_ref())
            .is_some_and(|r| r.source == story_sidecar::Source::Pinned)
        {
            self.toast = Some(Toast::new(
                "This slide's story is pinned (hand-written)".into(),
            ));
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.story_rx = Some(rx);
        let deck = self.file_path.clone();
        std::thread::spawn(move || {
            let result = crate::commands::story::generate_one_blocking(&deck, idx)
                .map(|_| ())
                .map_err(|e| e.to_string());
            let _ = tx.send((idx, result));
        });
        self.toast = Some(Toast::new(format!(
            "Writing a story for slide {}…",
            idx + 1
        )));
    }

    pub(super) fn poll_story(&mut self) {
        let Some(rx) = &self.story_rx else {
            return;
        };
        match rx.try_recv() {
            Ok((idx, Ok(()))) => {
                self.story_rx = None;
                self.reload_stories();
                self.toast = Some(Toast::new(format!("Story ready for slide {}", idx + 1)));
            }
            Ok((idx, Err(e))) => {
                self.story_rx = None;
                self.incident_log
                    .record("story_error", &format!("slide {}", idx + 1), &e);
                self.toast = Some(Toast::new(format!("Story failed: {e}")));
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.story_rx = None;
            }
        }
    }
}
