//! What an export walks through: the page size, the target slides and
//! their reveal steps.

/// The page size and which pages to export.
pub(super) struct Job {
    pub width: u32,
    pub height: u32,
    /// Export every reveal step, not just the fully revealed slide.
    pub debug: bool,
    /// Slide indices to export, in order.
    pub targets: Vec<usize>,
    /// `--at` / `--moment`.
    pub rehearsal: super::rehearsal::Rehearsal,
    /// Draw the presenter view instead of the slides.
    pub presenter_view: bool,
}

/// Which slide, and which reveal step of it, is being exported.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Cursor {
    pub(super) targets: Vec<usize>,
    /// Position in `targets`.
    pub(super) pos: usize,
    pub(super) step: usize,
    pub(super) debug: bool,
}

impl Cursor {
    pub(super) fn new(targets: Vec<usize>, debug: bool) -> Self {
        Self {
            targets,
            pos: 0,
            step: 0,
            debug,
        }
    }

    pub(super) fn slide(&self) -> usize {
        self.targets.get(self.pos).copied().unwrap_or(0)
    }

    /// The reveal step to draw: every step in debug, else the last one.
    pub(super) fn reveal(&self, max_step: usize) -> usize {
        if self.debug { self.step } else { max_step }
    }

    /// Move to the next page, given the current slide's last step. Returns
    /// false when every target has been exported.
    pub(super) fn advance(&mut self, max_step: usize) -> bool {
        if self.debug && self.step < max_step {
            self.step += 1;
            return true;
        }
        self.step = 0;
        self.pos += 1;
        self.pos < self.targets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_exports_each_target_once() {
        let mut c = Cursor::new(vec![2, 5], false);
        assert_eq!((c.slide(), c.reveal(3)), (2, 3));
        assert!(c.advance(3));
        assert_eq!(c.slide(), 5);
        assert!(!c.advance(0));
    }

    #[test]
    fn debug_cursor_walks_every_step() {
        let mut c = Cursor::new(vec![0, 1], true);
        let mut seen = vec![(c.slide(), c.reveal(2))];
        let max = [2, 0];
        while c.advance(max[c.slide()]) {
            seen.push((c.slide(), c.reveal(max[c.slide()])));
        }
        assert_eq!(seen, vec![(0, 0), (0, 1), (0, 2), (1, 0)]);
    }
}
