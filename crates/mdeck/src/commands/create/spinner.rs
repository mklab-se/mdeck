//! A terminal spinner for the silent AI steps.

use std::io::{self, Write};

/// A terminal spinner that animates on a background thread.
pub(super) struct Spinner {
    handle: Option<std::thread::JoinHandle<()>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Spinner {
    /// Start a spinner with the given message. The spinner animates until `stop()` is called.
    pub(super) fn start(message: String) -> Self {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_clone = stop.clone();
        let handle = std::thread::spawn(move || {
            const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            while !stop_clone.load(std::sync::atomic::Ordering::Relaxed) {
                eprint!("\r  {} {}", FRAMES[i % FRAMES.len()], message);
                let _ = io::stderr().flush();
                i += 1;
                std::thread::sleep(std::time::Duration::from_millis(80));
            }
        });
        Self {
            handle: Some(handle),
            stop,
        }
    }

    /// A spinner unless `quiet`.
    pub(super) fn unless_quiet(quiet: bool, message: String) -> Option<Self> {
        (!quiet).then(|| Self::start(message))
    }

    /// Stop the spinner and replace its line with a completion message.
    pub(super) fn stop_with(mut self, message: &str) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        // Clear the spinner line and print the completion message
        eprint!("\r\x1b[2K  {message}\n");
        let _ = io::stderr().flush();
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
