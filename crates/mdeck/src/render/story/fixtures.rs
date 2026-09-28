//! Shared test fixtures: a sample script and a library that resolves its kinds.

use std::sync::Arc;

use super::is_figure;
use crate::render::illustration::{Cloud, Library, VERSION};

/// A library with a ring-shaped cloud for every kind the tests cast.
pub(crate) fn test_library() -> Library {
    let mut lib = Library::with_dirs(None, None);
    for name in ["person", "hooded", "inbox", "orb", "box", "laptop"] {
        let points: Vec<[f32; 2]> = (0..64)
            .map(|i| {
                let a = i as f32 / 64.0 * std::f32::consts::TAU;
                [0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()]
            })
            .collect();
        lib.insert(Cloud {
            version: VERSION,
            name: name.into(),
            description: String::new(),
            prompt: None,
            generated: None,
            aspect: if is_figure(name) { 1.35 } else { 1.0 },
            points: Arc::new(points),
        });
    }
    lib
}

pub(crate) const SAMPLE: &str = r#"
cast:
  - { id: anders, kind: person, label: Anders, cell: left }
  - { id: queue, kind: inbox, label: Support queue, cell: center-top }
  - { id: model, kind: orb, label: The model, cell: right, fill: brain }
flows:
  - { from: queue, to: model, color: white, at: 1 }
  - { from: model, to: anders, color: ember, at: 2 }
beats:
  - { show: [anders, queue], say: "Anders stopped reading the tickets." }
  - { show: [model], say: "He pointed the assistant at the queue." }
  - { hot: [model], say: "Nobody noticed what came back." }
"#;
