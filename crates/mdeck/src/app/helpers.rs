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
                flatten_item_steps(items, &mut steps);
                for (flat_idx, item_step) in steps.iter().enumerate() {
                    if *item_step == step {
                        let bottom = y + (flat_idx + 1) as f32 * item_height;
                        best = Some(best.map_or(bottom, |b: f32| b.max(bottom)));
                    }
                }
            }
            parser::Block::Diagram { step_base, .. } | parser::Block::Chart { step_base, .. } => {
                let own = parser::steps::default_visual_steps(block);
                if step > *step_base && step <= step_base + own {
                    let bottom = y + h;
                    best = Some(best.map_or(bottom, |b: f32| b.max(bottom)));
                }
            }
            _ => {}
        }
        y += h + block_spacing;
    }
    best
}

/// Every list item's step in render order (depth first).
fn flatten_item_steps(items: &[parser::ListItem], out: &mut Vec<usize>) {
    for item in items {
        out.push(item.step);
        flatten_item_steps(&item.children, out);
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
    use crate::parser::Block;
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

    /// Blocks parsed and numbered as a slide's.
    fn numbered(md: &str) -> Vec<Block> {
        let mut blocks = crate::parser::blocks::parse(md);
        crate::parser::steps::number(
            &mut blocks,
            true,
            &crate::parser::steps::default_visual_steps,
        );
        blocks
    }

    #[test]
    fn flatten_item_steps_follows_render_order() {
        let blocks = numbered("- a\n+ b\n  - b1\n+ c\n- d");
        let Block::List { items, .. } = &blocks[0] else {
            panic!()
        };
        let mut steps = Vec::new();
        flatten_item_steps(items, &mut steps);
        assert_eq!(steps, vec![0, 1, 1, 2, 0]);
    }

    #[test]
    fn revealed_bottom_finds_lowest_item_for_step() {
        let blocks = numbered("# H\n\n+ one\n+ two\n  - two a");
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
        let mut blocks =
            crate::parser::blocks::parse("text\n\n```@bar\n- A: 1\n+ B: 2\n+ C: 3\n```");
        let max = crate::parser::steps::number(
            &mut blocks,
            true,
            &crate::parser::steps::default_visual_steps,
        );
        let heights = vec![50.0, 300.0];
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
