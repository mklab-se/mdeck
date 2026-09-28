//! A deck's art as the engine sees it: which picture each slide shows in
//! the current medium, loaded and prepared once. The presentation window
//! loads in the background (a slide shows its fallback until its picture is
//! ready); export loads before it draws.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eframe::egui;

use super::prepare::{Prepared, Strategy, prepare};
use super::sidecar::{self, Coverage, Resolved};
use super::style::Style;
use super::{ArtKind, Medium};
use crate::parser::Presentation;
use crate::theme::Theme;

enum Slot {
    Loading,
    Ready(Arc<Prepared>),
    Failed,
}

type Key = (PathBuf, ArtKind, Strategy);

pub struct DeckArt {
    deck: Option<PathBuf>,
    background: bool,
    /// The style and sidecar state `resolved` was worked out for.
    key: Option<String>,
    style: Option<Style>,
    strategy: Strategy,
    resolved: Vec<Option<Resolved>>,
    problems: Vec<String>,
    slots: Arc<Mutex<HashMap<Key, Slot>>>,
    repaint: Option<egui::Context>,
}

impl DeckArt {
    /// `background`: load pictures on worker threads (the presentation
    /// window) instead of on first use (export, check).
    pub fn new(deck: Option<&Path>, background: bool) -> Self {
        Self {
            deck: deck.map(Path::to_path_buf),
            background,
            key: None,
            style: None,
            strategy: Strategy::Draw,
            resolved: Vec::new(),
            problems: Vec::new(),
            slots: Arc::new(Mutex::new(HashMap::new())),
            repaint: None,
        }
    }

    /// Ask for a repaint on `ctx` when a picture finishes loading.
    pub fn repaint_on(&mut self, ctx: &egui::Context) {
        self.repaint = Some(ctx.clone());
    }

    /// Read the sidecar again on the next [`DeckArt::sync`] (the deck was
    /// reloaded or new art was generated).
    pub fn invalidate(&mut self) {
        self.key = None;
    }

    /// Bring the resolution up to date with the deck and the theme's
    /// medium. Cheap when nothing changed.
    pub fn sync(&mut self, presentation: &Presentation, theme: &Theme) {
        let Some(medium) = theme.engine.medium() else {
            if self.key.as_deref() != Some("") {
                self.key = Some(String::new());
                self.style = None;
                self.resolved.clear();
                self.problems.clear();
            }
            return;
        };
        let style = Style::for_medium(medium, theme);
        let key = format!("{}:{}", style.id(), presentation.slides.len());
        if self.key.as_deref() == Some(key.as_str()) {
            return;
        }
        self.problems.clear();
        let sc = match &self.deck {
            Some(deck) => sidecar::load(deck).unwrap_or_else(|e| {
                self.problems.push(e);
                None
            }),
            None => None,
        };
        self.resolved = match &self.deck {
            Some(deck) => sidecar::resolve(deck, presentation, sc.as_ref(), &style.id()),
            None => vec![None; presentation.slides.len()],
        };
        self.strategy = strategy_for(medium, style.kind);
        self.style = Some(style);
        self.key = Some(key);
    }

    /// Problems reading the sidecar, for the startup line and `--check`.
    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    pub fn resolved(&self) -> &[Option<Resolved>] {
        &self.resolved
    }

    /// How many slides take art and how many have none, or a stale picture.
    pub fn coverage(&self, presentation: &Presentation) -> Option<Coverage> {
        self.style
            .as_ref()
            .map(|_| sidecar::coverage(presentation, &self.resolved))
    }

    /// Slide `index`'s picture, when it has one and it is loaded. The first
    /// ask starts the load.
    pub fn picture(&mut self, index: usize) -> Option<Arc<Prepared>> {
        let style = self.style.as_ref()?;
        let resolved = self.resolved.get(index)?.as_ref()?;
        let key: Key = (resolved.file.clone(), style.kind, self.strategy);
        {
            let slots = self.slots.lock().ok()?;
            match slots.get(&key) {
                Some(Slot::Ready(p)) => return Some(p.clone()),
                Some(Slot::Loading) | Some(Slot::Failed) => return None,
                None => {}
            }
        }
        let load = {
            let key = key.clone();
            move || match std::fs::read(&key.0)
                .map_err(|e| e.to_string())
                .and_then(|bytes| prepare(&bytes, key.1, key.2))
            {
                Ok(p) => Slot::Ready(Arc::new(p)),
                Err(e) => {
                    eprintln!("art: {}: {e}", key.0.display());
                    Slot::Failed
                }
            }
        };
        if self.background {
            self.slots.lock().ok()?.insert(key.clone(), Slot::Loading);
            let slots = self.slots.clone();
            let repaint = self.repaint.clone();
            std::thread::spawn(move || {
                let slot = load();
                if let Ok(mut s) = slots.lock() {
                    s.insert(key, slot);
                }
                if let Some(ctx) = repaint {
                    ctx.request_repaint();
                }
            });
            None
        } else {
            let slot = load();
            let out = match &slot {
                Slot::Ready(p) => Some(p.clone()),
                _ => None,
            };
            self.slots.lock().ok()?.insert(key, slot);
            out
        }
    }

    /// Start loading every slide's picture (the window does this up front
    /// so the first pass through the deck never waits).
    pub fn preload(&mut self) {
        for i in 0..self.resolved.len() {
            let _ = self.picture(i);
        }
    }
}

/// How a medium draws pictures of `kind` in.
pub fn strategy_for(medium: &Medium, kind: ArtKind) -> Strategy {
    match kind {
        ArtKind::Line => Strategy::Draw,
        ArtKind::Tonal => medium.tonal_strategy,
    }
}
