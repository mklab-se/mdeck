//! Problems: what an extension reports to `mdeck --check` and the startup
//! summary.

use std::fmt;

/// One thing wrong with a deck, a theme or a setting.
///
/// ```
/// use mdeck_sdk::problem::Problem;
/// let p = Problem::new("engine", "unknown setting `glow`").at(12);
/// assert_eq!(p.to_string(), "line 12: engine: unknown setting `glow`");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Problem {
    /// 1-based line in the deck (or theme) file, when known.
    pub line: Option<usize>,
    /// The check category (`engine`, `visual`, `design`, ...).
    pub category: String,
    /// What is wrong, in one sentence a deck author understands.
    pub message: String,
}

impl Problem {
    /// A problem in `category`, without a line.
    ///
    /// See [`Problem`] for an example.
    pub fn new(category: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            line: None,
            category: category.into(),
            message: message.into(),
        }
    }

    /// The same problem at `line` (1-based).
    ///
    /// See [`Problem`] for an example.
    pub fn at(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(line) = self.line {
            write!(f, "line {line}: ")?;
        }
        write!(f, "{}: {}", self.category, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_without_a_line() {
        assert_eq!(Problem::new("visual", "empty").to_string(), "visual: empty");
    }
}
