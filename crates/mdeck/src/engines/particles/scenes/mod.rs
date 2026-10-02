//! Scenes inferred from slide content.
//!
//! Every layout gets a default choreography that carries its meaning: a
//! title slide opens on a constellation, a bullet slide lights one cluster
//! per item as they reveal, a quote slide burns like a candle, a code slide
//! rains. Nothing here is authored per deck.

use super::{Drift, Group, Home};

pub use hints::from_hints;
pub use layouts::{constellation, for_slide};
pub use moments::{
    burst, digit, end_bang, end_dance, end_words, illustration_backdrop, illustration_stage,
};

mod backdrop;
mod formation;
mod hints;
mod layouts;
mod moments;

/// Faint dust everywhere, so no part of the slide is ever dead black.
fn dust(share: f32) -> Group {
    Group::new(
        share,
        Home::Field {
            u0: 0.02,
            v0: 0.04,
            u1: 0.98,
            v1: 0.96,
        },
    )
    .alpha(0.10, 0.30)
    .size(0.45, 0.9)
    .drift(Drift::Breathe {
        amp: 0.011,
        speed: 1.1,
    })
}

/// A few big, very soft lights out of focus.
fn bokeh(share: f32) -> Group {
    Group::new(
        share,
        Home::Field {
            u0: 0.02,
            v0: 0.05,
            u1: 0.98,
            v1: 0.95,
        },
    )
    .alpha(0.07, 0.18)
    .size(2.6, 4.2)
    .drift(Drift::Breathe {
        amp: 0.016,
        speed: 0.7,
    })
}
