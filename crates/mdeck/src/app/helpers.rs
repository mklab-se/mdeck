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

/// Whether a watcher event for `event_path` refers to the presentation file.
///
/// The watcher observes the parent directory (so atomic saves that replace the
/// inode are still seen), so events for sibling files must be ignored.
pub(super) fn event_matches_file(event_path: &std::path::Path, file: &std::path::Path) -> bool {
    if event_path == file {
        return true;
    }
    match (event_path.file_name(), file.file_name()) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

pub(super) fn spawn_file_watcher(
    path: &std::path::Path,
    ctx: egui::Context,
    incident_log: Arc<IncidentLog>,
) -> anyhow::Result<(mpsc::Receiver<()>, Debouncer<notify::RecommendedWatcher>)> {
    let (tx, rx) = mpsc::channel();
    let file = path.to_path_buf();
    let mut debouncer = new_debouncer(
        std::time::Duration::from_millis(500),
        move |events: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
            match events {
                Ok(events) => {
                    if events.iter().any(|e| {
                        e.kind == DebouncedEventKind::Any && event_matches_file(&e.path, &file)
                    }) {
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
    // Watch the parent directory rather than the file itself: editors that save
    // atomically (write temp + rename) replace the inode, which would silently
    // detach a per-file inotify watch after the first save.
    let watch_target = path.parent().filter(|p| !p.as_os_str().is_empty());
    match watch_target {
        Some(dir) => debouncer
            .watcher()
            .watch(dir, notify::RecursiveMode::NonRecursive)?,
        None => debouncer
            .watcher()
            .watch(path, notify::RecursiveMode::NonRecursive)?,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn event_matches_same_path_or_same_file_name() {
        let file = Path::new("/tmp/deck/slides.md");
        assert!(event_matches_file(Path::new("/tmp/deck/slides.md"), file));
        assert!(event_matches_file(
            Path::new("/private/tmp/deck/slides.md"),
            file
        ));
        assert!(!event_matches_file(Path::new("/tmp/deck/other.md"), file));
        assert!(!event_matches_file(
            Path::new("/tmp/deck/.slides.md.swp"),
            file
        ));
        assert!(!event_matches_file(Path::new("/tmp/deck/slides.md~"), file));
    }

    use crate::parser::Slide;

    fn slide(raw: &str) -> Slide {
        Slide {
            raw_source: raw.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn find_matching_slide_exact_match() {
        let _slides = [slide("a"), slide("b"), slide("c")];
        // Was at index 1 ("b"), new slides inserted "x" before it
        let new_slides = vec![slide("x"), slide("a"), slide("b"), slide("c")];
        assert_eq!(find_matching_slide(Some("b"), 1, &new_slides), 2);
    }

    #[test]
    fn find_matching_slide_edited_stays_at_index() {
        let old_raw = "old content";
        let new_slides = vec![slide("a"), slide("new content"), slide("c")];
        // Old raw doesn't match any new slide: clamp to old index
        assert_eq!(find_matching_slide(Some(old_raw), 1, &new_slides), 1);
    }

    #[test]
    fn find_matching_slide_clamps_when_out_of_bounds() {
        let new_slides = vec![slide("a"), slide("b")];
        // Was at index 5, only 2 slides now
        assert_eq!(find_matching_slide(Some("gone"), 5, &new_slides), 1);
    }

    #[test]
    fn find_matching_slide_no_old_raw_returns_zero() {
        let new_slides = vec![slide("a"), slide("b")];
        assert_eq!(find_matching_slide(None, 0, &new_slides), 0);
    }
}
