//! A deck's art as the engine sees it: which picture each slide shows in
//! the current medium, loaded and prepared once. The presentation window
//! loads in the background (a slide shows its fallback until its picture is
//! ready, a few worker threads at a time); export loads before it draws.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eframe::egui;

use super::loader::Pool;
use super::prepare::{Prepared, Strategy, prepare};
use super::resolve::{self, Coverage, Resolved};
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

/// Threads loading pictures in the window.
const WORKERS: usize = 3;

pub struct DeckArt {
    deck: Option<PathBuf>,
    background: bool,
    /// The style and manifest state `resolved` was worked out for.
    key: Option<String>,
    style: Option<Style>,
    strategy: Strategy,
    resolved: Vec<Option<Resolved>>,
    problems: Vec<String>,
    slots: Arc<Mutex<HashMap<Key, Slot>>>,
    repaint: Arc<Mutex<Option<egui::Context>>>,
    /// The workers, started with the first background load.
    pool: Option<Pool<Key>>,
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
            repaint: Arc::new(Mutex::new(None)),
            pool: None,
        }
    }

    /// Ask for a repaint on `ctx` when a picture finishes loading.
    pub fn repaint_on(&mut self, ctx: &egui::Context) {
        if let Ok(mut r) = self.repaint.lock()
            && r.is_none()
        {
            *r = Some(ctx.clone());
        }
    }

    /// Read the manifest again on the next [`DeckArt::sync`] (the deck was
    /// reloaded or new art was generated).
    pub fn invalidate(&mut self) {
        self.key = None;
    }

    /// Bring the resolution up to date with the deck and the theme's
    /// medium. Cheap when nothing changed.
    pub fn sync(&mut self, presentation: &Presentation, theme: &Theme) {
        let Some(medium) = theme.engine.medium().map(super::Medium::of) else {
            if self.key.as_deref() != Some("") {
                self.key = Some(String::new());
                self.style = None;
                self.resolved.clear();
                self.problems.clear();
            }
            return;
        };
        let style = Style::for_medium(&medium, theme);
        let key = format!("{}:{}", style.id(), presentation.slides.len());
        if self.key.as_deref() == Some(key.as_str()) {
            return;
        }
        self.problems.clear();
        let sc = match &self.deck {
            Some(deck) => crate::assets::manifest::load(deck).unwrap_or_else(|e| {
                self.problems.push(e.to_string());
                None
            }),
            None => None,
        };
        self.resolved = match &self.deck {
            Some(deck) => resolve::resolve(deck, presentation, sc.as_ref(), &style.id()),
            None => vec![None; presentation.slides.len()],
        };
        self.strategy = strategy_for(&medium, style.kind);
        self.style = Some(style);
        self.key = Some(key);
    }

    /// Problems reading the manifest, for the startup line and `--check`.
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
            .map(|_| resolve::coverage(presentation, &self.resolved))
    }

    /// Slide `index`'s picture, when it has one and it is loaded. The first
    /// ask starts the load; asking again moves a waiting load to the front.
    pub fn picture(&mut self, index: usize) -> Option<Arc<Prepared>> {
        self.request(index, true)
    }

    /// Start loading every slide's picture (the window does this up front
    /// so the first pass through the deck never waits).
    pub fn preload(&mut self) {
        for i in 0..self.resolved.len() {
            let _ = self.request(i, false);
        }
    }

    fn request(&mut self, index: usize, urgent: bool) -> Option<Arc<Prepared>> {
        let style = self.style.as_ref()?;
        let resolved = self.resolved.get(index)?.as_ref()?;
        let key: Key = (resolved.file.clone(), style.kind, self.strategy);
        {
            let slots = self.slots.lock().ok()?;
            match slots.get(&key) {
                Some(Slot::Ready(p)) => return Some(p.clone()),
                Some(Slot::Loading) => {
                    if urgent && let Some(pool) = &self.pool {
                        pool.hurry(&key);
                    }
                    return None;
                }
                Some(Slot::Failed) => return None,
                None => {}
            }
        }
        if self.background {
            self.slots.lock().ok()?.insert(key.clone(), Slot::Loading);
            let pool = self.pool.get_or_insert_with(|| {
                let slots = self.slots.clone();
                let repaint = self.repaint.clone();
                Pool::new(WORKERS, move |key: Key| {
                    let slot = load(&key);
                    if let Ok(mut s) = slots.lock() {
                        s.insert(key, slot);
                    }
                    if let Some(ctx) = repaint.lock().ok().and_then(|r| r.clone()) {
                        ctx.request_repaint();
                    }
                })
            });
            pool.push(key, urgent);
            None
        } else {
            let slot = load(&key);
            let out = match &slot {
                Slot::Ready(p) => Some(p.clone()),
                _ => None,
            };
            self.slots.lock().ok()?.insert(key, slot);
            out
        }
    }
}

/// Read and prepare one picture.
fn load(key: &Key) -> Slot {
    match std::fs::read(&key.0)
        .map_err(anyhow::Error::from)
        .and_then(|bytes| prepare(&bytes, key.1, key.2))
    {
        Ok(p) => Slot::Ready(Arc::new(p)),
        Err(e) => {
            eprintln!("art: {}: {e}", key.0.display());
            Slot::Failed
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
