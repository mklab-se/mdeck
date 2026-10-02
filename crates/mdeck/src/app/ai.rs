//! `S` while presenting: AI for the current slide in the background, a
//! picture on an art engine, and what happens when the result comes back.

use std::sync::mpsc;

use super::{PresentationApp, Toast};
use crate::render;

impl PresentationApp {
    /// `S`: AI for the current slide, in the background: a picture on an
    /// art engine.
    pub(super) fn generate(&mut self) {
        if self.theme.engine.medium().is_some() {
            self.generate_art();
        } else {
            self.toast = Some(Toast::new(
                "S draws pictures on art engines (Shift+T)".into(),
            ));
        }
    }

    /// Draw the current slide's picture for the art engine on screen.
    pub(super) fn generate_art(&mut self) {
        if self.jobs.art.is_some() {
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
        let Some(slide) = self.deck.presentation.slides.get(idx) else {
            return;
        };
        if !render::art::wants_art(slide) {
            self.toast = Some(Toast::new(
                "This slide takes no art (its layout has no room, or it says @art: none)".into(),
            ));
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.jobs.art = Some(rx);
        let deck = self.deck.file.clone();
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
        let Some(rx) = &self.jobs.art else {
            return;
        };
        match rx.try_recv() {
            Ok((idx, Ok(()))) => {
                self.jobs.art = None;
                self.deck.art.invalidate();
                self.deck.art.sync(&self.deck.presentation, &self.theme);
                self.deck.art.preload();
                self.toast = Some(Toast::new(format!("Picture ready for slide {}", idx + 1)));
            }
            Ok((idx, Err(e))) => {
                self.jobs.art = None;
                self.incident_log
                    .record("art_error", &format!("slide {}", idx + 1), &e);
                self.toast = Some(Toast::new(format!("Drawing failed: {e}")));
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.jobs.art = None;
            }
        }
    }
}
