//! Short messages at the bottom of the screen ("Theme: ember").

use std::time::Instant;

pub(super) struct Toast {
    pub(super) message: String,
    pub(super) start: Instant,
}

impl Toast {
    pub(super) fn new(message: String) -> Self {
        Self {
            message,
            start: Instant::now(),
        }
    }

    pub(super) fn opacity(&self) -> f32 {
        let elapsed = self.start.elapsed().as_secs_f32();
        let duration = 1.5;
        let fade_start = 1.0;
        if elapsed < fade_start {
            1.0
        } else if elapsed < duration {
            1.0 - (elapsed - fade_start) / (duration - fade_start)
        } else {
            0.0
        }
    }

    pub(super) fn is_expired(&self) -> bool {
        self.start.elapsed().as_secs_f32() >= 1.5
    }
}
