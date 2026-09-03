use eframe::egui;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, mpsc};

use notify_debouncer_mini::{DebouncedEventKind, Debouncer, new_debouncer, notify};

use crate::incident_log::IncidentLog;
use crate::parser;

pub(super) fn lerp_rect(a: egui::Rect, b: egui::Rect, t: f32) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(
            a.min.x + (b.min.x - a.min.x) * t,
            a.min.y + (b.min.y - a.min.y) * t,
        ),
        egui::pos2(
            a.max.x + (b.max.x - a.max.x) * t,
            a.max.y + (b.max.y - a.max.y) * t,
        ),
    )
}

pub(super) fn load_app_icon() -> Option<egui::IconData> {
    let png_bytes = include_bytes!("../../media/MDeck-logo.png");
    let image = image::load_from_memory(png_bytes).ok()?.into_rgba8();
    let (w, h) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width: w,
        height: h,
    })
}

/// Compute a hash of file content for change detection.
pub(super) fn hash_content(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Find the best matching slide index after a reload.
///
/// If the old slide's raw source matches a new slide exactly, use that index.
/// Otherwise fall back to the old index, clamped to bounds.
pub(super) fn find_matching_slide(
    old_raw: Option<&str>,
    old_index: usize,
    new_slides: &[parser::Slide],
) -> usize {
    if let Some(raw) = old_raw
        && let Some(pos) = new_slides.iter().position(|s| s.raw_source == raw)
    {
        return pos;
    }
    old_index.min(new_slides.len().saturating_sub(1))
}

pub(super) fn spawn_file_watcher(
    path: &std::path::Path,
    ctx: egui::Context,
    incident_log: Arc<IncidentLog>,
) -> anyhow::Result<(mpsc::Receiver<()>, Debouncer<notify::RecommendedWatcher>)> {
    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(
        std::time::Duration::from_millis(500),
        move |events: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
            match events {
                Ok(events) => {
                    if events.iter().any(|e| e.kind == DebouncedEventKind::Any) {
                        let _ = tx.send(());
                        ctx.request_repaint();
                    }
                }
                Err(e) => {
                    incident_log.record(
                        "file_watcher_error",
                        "file watcher reported an error",
                        &format!("{e}"),
                    );
                }
            }
        },
    )?;
    debouncer
        .watcher()
        .watch(path, notify::RecursiveMode::NonRecursive)?;
    Ok((rx, debouncer))
}

pub(super) fn print_incident_summary(log: &IncidentLog) {
    if let Some((path, count)) = log.summary() {
        eprintln!(
            "{count} issue(s) encountered during this session. Details: {}",
            path.display(),
        );
    }
}
