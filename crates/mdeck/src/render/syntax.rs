use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use eframe::egui::{self, Color32, FontFamily, FontId};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::theme::Theme;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// A run of source text with the colour syntect assigned to it.
struct Span {
    rgb: [u8; 3],
    text: String,
}

/// Highlighted spans, cached by (code, language, syntect theme). Syntect is
/// far too slow to run every frame; building a `LayoutJob` from cached spans is
/// cheap, so font size, opacity and wrap width are applied on the way out.
static SPAN_CACHE: LazyLock<Mutex<HashMap<u64, Arc<Vec<Span>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Cache is cleared once it holds more than this many code blocks.
const CACHE_CAPACITY: usize = 128;

static CACHE_HITS: AtomicUsize = AtomicUsize::new(0);

/// Number of times a highlight request was served from the cache (for tests
/// and diagnostics).
#[cfg(test)]
pub fn cache_hits() -> usize {
    CACHE_HITS.load(Ordering::Relaxed)
}

fn cache_key(code: &str, language: Option<&str>, theme_name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    code.hash(&mut hasher);
    language.hash(&mut hasher);
    theme_name.hash(&mut hasher);
    hasher.finish()
}

fn highlight_spans(code: &str, language: Option<&str>, theme_name: &str) -> Arc<Vec<Span>> {
    let key = cache_key(code, language, theme_name);

    if let Some(spans) = SPAN_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
    {
        CACHE_HITS.fetch_add(1, Ordering::Relaxed);
        return Arc::clone(spans);
    }

    let ss = &*SYNTAX_SET;
    let ts = &*THEME_SET;

    let syntax = language
        .and_then(|lang| ss.find_syntax_by_token(lang))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let syntect_theme = ts
        .themes
        .get(theme_name)
        .unwrap_or_else(|| ts.themes.values().next().unwrap());

    let mut highlighter = HighlightLines::new(syntax, syntect_theme);
    let mut spans = Vec::new();

    // The `newlines` syntax set expects each line to keep its trailing '\n';
    // stripping it breaks multi-line constructs such as block comments.
    for line in LinesWithEndings::from(code) {
        let ranges = highlighter
            .highlight_line(line, ss)
            .unwrap_or_else(|_| vec![]);
        for (style, text) in ranges {
            let fg = style.foreground;
            spans.push(Span {
                rgb: [fg.r, fg.g, fg.b],
                text: text.to_string(),
            });
        }
    }

    let spans = Arc::new(spans);
    let mut cache = SPAN_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() >= CACHE_CAPACITY {
        cache.clear();
    }
    cache.insert(key, Arc::clone(&spans));
    spans
}

/// Create a syntax-highlighted `LayoutJob` for a code block.
pub fn highlight_code(
    code: &str,
    language: Option<&str>,
    font_size: f32,
    opacity: f32,
    theme: &Theme,
    max_width: f32,
) -> egui::text::LayoutJob {
    let spans = highlight_spans(code, language, theme.syntect_theme_name());

    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = max_width;
    let font_id = FontId::new(font_size, FontFamily::Monospace);
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;

    for span in spans.iter() {
        let [r, g, b] = span.rgb;
        let format = egui::text::TextFormat {
            font_id: font_id.clone(),
            color: Color32::from_rgba_unmultiplied(r, g, b, alpha),
            ..Default::default()
        };
        job.append(&span.text, 0.0, format);
    }

    job
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Colour of the section containing `needle` in the job.
    fn color_of(job: &egui::text::LayoutJob, needle: &str) -> Color32 {
        let start = job.text.find(needle).expect("needle present");
        job.sections
            .iter()
            .find(|s| s.byte_range.contains(&egui::text::ByteIndex(start)))
            .expect("section for needle")
            .format
            .color
    }

    #[test]
    fn block_comment_keeps_comment_colour_on_second_line() {
        let code = "/* first line\nstill a comment */\nfn main() {}";
        let job = highlight_code(code, Some("rust"), 20.0, 1.0, &Theme::dark(), 1000.0);
        let first = color_of(&job, "first line");
        let second = color_of(&job, "still a comment");
        let keyword = color_of(&job, "fn");
        assert_eq!(
            first, second,
            "second comment line must be comment-coloured"
        );
        assert_ne!(first, keyword, "comment and keyword colours differ");
    }

    #[test]
    fn newlines_are_preserved_in_the_layout_text() {
        let code = "let a = 1;\nlet b = 2;";
        let job = highlight_code(code, Some("rust"), 20.0, 1.0, &Theme::dark(), 1000.0);
        assert_eq!(job.text, code);
        assert_eq!(job.wrap.max_width, 1000.0);
    }

    #[test]
    fn repeated_calls_hit_the_cache_and_return_equal_jobs() {
        let code = "def cache_me(x):\n    return x * 2\n";
        let theme = Theme::light();
        let a = highlight_code(code, Some("python"), 24.0, 1.0, &theme, 800.0);
        let before = cache_hits();
        let b = highlight_code(code, Some("python"), 24.0, 1.0, &theme, 800.0);
        assert_eq!(a, b);
        assert!(cache_hits() > before, "second call must be a cache hit");

        // Different opacity / size reuse the cached spans but restyle them.
        let faded = highlight_code(code, Some("python"), 30.0, 0.5, &theme, 800.0);
        assert_eq!(faded.sections[0].format.color.a(), 127);
        assert_eq!(faded.sections[0].format.font_id.size, 30.0);
        assert_eq!(faded.text, a.text);
    }

    #[test]
    fn unknown_language_falls_back_to_plain_text() {
        let job = highlight_code(
            "plain",
            Some("no-such-lang"),
            20.0,
            1.0,
            &Theme::dark(),
            500.0,
        );
        assert_eq!(job.text, "plain");
        assert!(!job.sections.is_empty());
    }
}
