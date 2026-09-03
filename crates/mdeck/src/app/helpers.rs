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
    if let Some(raw) = old_raw {
        if let Some(pos) = new_slides.iter().position(|s| s.raw_source == raw) {
            return pos;
        }
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

/// Resolve a setting with precedence: frontmatter > config default > built-in.
pub(super) fn resolve_setting(
    frontmatter: Option<&str>,
    config_default: Option<&str>,
    builtin: &str,
) -> String {
    frontmatter
        .or(config_default)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(builtin)
        .to_string()
}

/// Bottom edge (relative to the content top) of the lowest element revealed at
/// exactly `step`, using pre-measured block `heights` and the list geometry
/// used by the renderer. Returns `None` when nothing is revealed at that step.
pub(super) fn revealed_bottom(
    blocks: &[parser::Block],
    heights: &[f32],
    step: usize,
    item_height: f32,
    block_spacing: f32,
) -> Option<f32> {
    if step == 0 {
        return None;
    }
    let mut y = 0.0;
    let mut best: Option<f32> = None;
    for (i, block) in blocks.iter().enumerate() {
        let h = heights.get(i).copied().unwrap_or(0.0);
        match block {
            parser::Block::List { items, .. } => {
                let mut steps = Vec::new();
                let mut counter = 0;
                flatten_item_steps(items, &mut counter, &mut steps);
                for (flat_idx, item_step) in steps.iter().enumerate() {
                    if *item_step == step {
                        let bottom = y + (flat_idx + 1) as f32 * item_height;
                        best = Some(best.map_or(bottom, |b: f32| b.max(bottom)));
                    }
                }
            }
            other => {
                if parser::compute_max_steps(std::slice::from_ref(other)) >= step {
                    let bottom = y + h;
                    best = Some(best.map_or(bottom, |b: f32| b.max(bottom)));
                }
            }
        }
        y += h + block_spacing;
    }
    best
}

/// Assign a reveal step to every list item in render order (depth first).
fn flatten_item_steps(items: &[parser::ListItem], counter: &mut usize, out: &mut Vec<usize>) {
    for item in items {
        let step = match item.marker {
            parser::ListMarker::NextStep => {
                *counter += 1;
                *counter
            }
            parser::ListMarker::WithPrev => *counter,
            parser::ListMarker::Static | parser::ListMarker::Ordered => 0,
        };
        out.push(step);
        flatten_item_steps(&item.children, counter, out);
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
    use crate::parser::{Block, Inline, ListItem, ListMarker};
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

    #[test]
    fn resolve_setting_precedence() {
        assert_eq!(resolve_setting(Some("nord"), Some("dark"), "light"), "nord");
        assert_eq!(resolve_setting(None, Some("dark"), "light"), "dark");
        assert_eq!(resolve_setting(None, None, "light"), "light");
        assert_eq!(resolve_setting(None, Some("  "), "light"), "light");
    }

    fn item(marker: ListMarker, children: Vec<ListItem>) -> ListItem {
        ListItem {
            marker,
            inlines: vec![Inline::Text("x".into())],
            children,
        }
    }

    fn list(items: Vec<ListItem>) -> Block {
        Block::List {
            ordered: false,
            items,
        }
    }

    #[test]
    fn flatten_item_steps_numbers_next_and_with_prev() {
        let items = vec![
            item(ListMarker::Static, vec![]),
            item(
                ListMarker::NextStep,
                vec![item(ListMarker::WithPrev, vec![])],
            ),
            item(ListMarker::NextStep, vec![]),
            item(ListMarker::WithPrev, vec![]),
        ];
        let mut steps = Vec::new();
        let mut counter = 0;
        flatten_item_steps(&items, &mut counter, &mut steps);
        assert_eq!(steps, vec![0, 1, 1, 2, 2]);
    }

    #[test]
    fn revealed_bottom_finds_lowest_item_for_step() {
        let heading = Block::Heading {
            level: 1,
            inlines: vec![],
        };
        let items = vec![
            item(ListMarker::NextStep, vec![]),
            item(ListMarker::NextStep, vec![]),
            item(ListMarker::WithPrev, vec![]),
        ];
        let blocks = vec![heading, list(items)];
        let heights = vec![60.0, 3.0 * 40.0];
        // Heading (60) + spacing (20) = list top at 80; item i bottom = 80 + (i+1)*40
        assert_eq!(revealed_bottom(&blocks, &heights, 0, 40.0, 20.0), None);
        assert_eq!(
            revealed_bottom(&blocks, &heights, 1, 40.0, 20.0),
            Some(120.0)
        );
        // Step 2 reveals items 2 and 3 → the lower one wins
        assert_eq!(
            revealed_bottom(&blocks, &heights, 2, 40.0, 20.0),
            Some(200.0)
        );
        assert_eq!(revealed_bottom(&blocks, &heights, 3, 40.0, 20.0), None);
    }

    #[test]
    fn revealed_bottom_handles_stepped_non_list_blocks() {
        // A bar chart with two reveal steps below a paragraph
        let blocks = vec![
            Block::Paragraph { inlines: vec![] },
            Block::BarChart {
                content: "- A: 1\n+ B: 2\n+ C: 3".to_string(),
            },
        ];
        let heights = vec![50.0, 300.0];
        let max = crate::parser::compute_max_steps(&blocks);
        assert!(max >= 1, "sample chart should have reveal steps");
        assert_eq!(
            revealed_bottom(&blocks, &heights, 1, 40.0, 20.0),
            Some(370.0)
        );
        assert_eq!(
            revealed_bottom(&blocks, &heights, max + 1, 40.0, 20.0),
            None
        );
    }
}
