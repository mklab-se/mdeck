use std::collections::BTreeSet;
use std::fmt;

/// Category of a check warning, for grouping and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CheckCategory {
    DiagramRouting,
    /// `picture` names that do not resolve or layouts that cannot show one.
    PointCloud,
    /// Text the available fonts cannot draw (CJK without a system CJK face).
    Fonts,
    /// `$...$` / `$$...$$` formulas that do not parse.
    Math,
    /// The deck's theme: unknown name, invalid file, fallbacks, weak contrast.
    Theme,
    /// Deck and slide settings: unknown names (with a suggestion), invalid
    /// values, deck settings in a slide, duplicates, and v1 syntax with its
    /// v2 form.
    Settings,
    /// Markdown that will not show as written.
    Content,
    /// Content the deck's engine does not show, and `engine` problems.
    Engine,
    /// Generated assets (artworks, images, icons, point clouds): missing,
    /// stale, or a manifest that cannot be read.
    Assets,
    /// `background` images that are missing or unreadable, bad opacities.
    Background,
    /// `@thermal` blocks: unreadable sources, colour input, unsupported
    /// settings, comparisons that cannot share a scale.
    Thermal,
    /// Visual fences: unknown or renamed tags, lines that are neither a
    /// setting nor an item, unknown settings and attributes, values that do
    /// not parse.
    Visual,
    /// Packs and extensions the deck `requires` that are not installed.
    Extensions,
}

impl fmt::Display for CheckCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckCategory::DiagramRouting => write!(f, "architecture"),
            CheckCategory::PointCloud => write!(f, "point-cloud"),
            CheckCategory::Fonts => write!(f, "fonts"),
            CheckCategory::Math => write!(f, "math"),
            CheckCategory::Theme => write!(f, "theme"),
            CheckCategory::Settings => write!(f, "settings"),
            CheckCategory::Extensions => write!(f, "extensions"),
            CheckCategory::Content => write!(f, "content"),
            CheckCategory::Engine => write!(f, "engine"),
            CheckCategory::Assets => write!(f, "assets"),
            CheckCategory::Background => write!(f, "background"),
            CheckCategory::Thermal => write!(f, "thermal"),
            CheckCategory::Visual => write!(f, "visual"),
        }
    }
}

/// A single warning produced during presentation checking.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckWarning {
    /// 1-indexed slide number.
    pub slide: usize,
    /// 1-based line in the deck file: the directive or block the warning is
    /// about when known, else the slide's first line. 0 for no line.
    pub line: usize,
    pub category: CheckCategory,
    pub message: String,
    /// Where the problem is when it is not in the deck file, shown in place
    /// of the slide: a theme file, or the extension a theme came from.
    pub place: Option<String>,
}

impl fmt::Display for CheckWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.place {
            Some(place) => write!(f, "  {place}")?,
            None => write!(f, "  slide {}", self.slide)?,
        }
        if self.line > 0 {
            write!(f, " (line {})", self.line)?;
        }
        write!(f, ": [{}] {}", self.category, self.message)
    }
}

/// Collects deduplicated, sorted warnings from a presentation check pass.
#[derive(Debug, Clone, Default)]
pub struct CheckReport {
    warnings: BTreeSet<CheckWarning>,
}

impl CheckReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, warning: CheckWarning) {
        self.warnings.insert(warning);
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    pub fn warnings(&self) -> impl Iterator<Item = &CheckWarning> {
        self.warnings.iter()
    }

    /// Print a brief one-liner summary to stderr (for GUI mode).
    pub fn print_brief(&self) {
        if self.has_warnings() {
            eprintln!(
                "warning: {} diagram routing issue(s) found (run with --check for details)",
                self.warning_count()
            );
        }
    }

    /// Print detailed per-warning output to stderr (for --check mode).
    pub fn print_detailed(&self) {
        for w in self.warnings() {
            eprintln!("{w}");
        }
        eprintln!();
        eprintln!("{} warning(s) found.", self.warning_count());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warning(slide: usize, msg: &str) -> CheckWarning {
        CheckWarning {
            slide,
            line: 0,
            category: CheckCategory::DiagramRouting,
            message: msg.to_string(),
            place: None,
        }
    }

    #[test]
    fn dedup_identical_warnings() {
        let mut report = CheckReport::new();
        report.add(warning(1, "Could not route A -> B"));
        report.add(warning(1, "Could not route A -> B"));
        assert_eq!(report.warning_count(), 1);
    }

    #[test]
    fn sorted_by_slide_then_message() {
        let mut report = CheckReport::new();
        report.add(warning(3, "Z warning"));
        report.add(warning(1, "A warning"));
        report.add(warning(3, "A warning"));
        let slides: Vec<(usize, &str)> = report
            .warnings()
            .map(|w| (w.slide, w.message.as_str()))
            .collect();
        assert_eq!(
            slides,
            vec![(1, "A warning"), (3, "A warning"), (3, "Z warning"),]
        );
    }

    #[test]
    fn add_from_multiple_sources_deduplicates() {
        let mut report = CheckReport::new();
        report.add(warning(1, "msg1"));
        report.add(warning(2, "msg2"));
        report.add(warning(2, "msg2")); // duplicate
        report.add(warning(3, "msg3"));
        assert_eq!(report.warning_count(), 3);
    }

    #[test]
    fn display_names_the_line_when_known() {
        let mut w = warning(3, "Could not route A -> B");
        assert_eq!(
            w.to_string(),
            "  slide 3: [architecture] Could not route A -> B"
        );
        w.line = 42;
        assert_eq!(
            w.to_string(),
            "  slide 3 (line 42): [architecture] Could not route A -> B"
        );
    }

    #[test]
    fn sorted_by_slide_then_line() {
        let mut report = CheckReport::new();
        for (slide, line, msg) in [(2, 9, "A"), (2, 7, "Z"), (1, 3, "M")] {
            report.add(CheckWarning {
                line,
                ..warning(slide, msg)
            });
        }
        let order: Vec<usize> = report.warnings().map(|w| w.line).collect();
        assert_eq!(order, [3, 7, 9]);
    }

    #[test]
    fn empty_report_has_no_warnings() {
        let report = CheckReport::new();
        assert!(!report.has_warnings());
        assert_eq!(report.warning_count(), 0);
    }
}
