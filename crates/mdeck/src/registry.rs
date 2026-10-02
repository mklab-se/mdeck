//! The registry the running mdeck looks engines, visuals, themes and point
//! clouds up in (D11, EXT-07). [`crate::run`] installs the one it is given;
//! without that (tests, tools) the built-ins alone are used.

use std::sync::OnceLock;

pub use mdeck_sdk::registry::Registry;

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Make `registry` the one this process uses. The first install wins: a
/// later one is handed back.
pub fn install(registry: Registry) -> Result<(), Box<Registry>> {
    REGISTRY.set(registry).map_err(Box::new)
}

/// The registry in use: the installed one, else the built-ins (and, in
/// unit tests, the [`test_extensions`]).
pub fn get() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        let mut r = Registry::new();
        crate::builtins(&mut r).expect("the built-ins register without clashes");
        #[cfg(test)]
        {
            r.set_origin("tests");
            test_extensions::register(&mut r).expect("the test extensions register");
        }
        r
    })
}

/// A code design set and a transition registered next to the built-ins in
/// unit tests, so the lookups an extension's get (EXT-05) can be tested
/// without a custom build.
#[cfg(test)]
pub mod test_extensions {
    use mdeck_sdk::content::{Block, Slide};
    use mdeck_sdk::design::{DesignCx, DesignSet};
    use mdeck_sdk::paint::{Rect, Vec2};
    use mdeck_sdk::problem::Problem;
    use mdeck_sdk::registry::{Registry, RegistryError};
    use mdeck_sdk::transition::{SideLook, Transition};

    /// The test design set's name.
    pub const DESIGN_SET: &str = "test-cards";
    /// The test transition's name.
    pub const TRANSITION: &str = "test-drop";
    /// A theme that names both.
    pub const THEME: &str = "test-extensions";

    pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
        r.design_set(Box::new(Cards))?;
        r.transition(Box::new(Drop))?;
        r.theme(
            THEME,
            "name: test-extensions\nextends: dark\ndesigns: test-cards\ntransition: test-drop\n",
        )
    }

    /// Fills the slide with the accent and reports code blocks.
    struct Cards;

    impl DesignSet for Cards {
        fn name(&self) -> &str {
            DESIGN_SET
        }
        fn render(&self, cx: &mut DesignCx, _slide: &Slide, rect: Rect) {
            cx.painter().rect_filled(rect, 0.0, cx.tokens().accent);
        }
        fn measure(&self, _cx: &mut DesignCx, _slide: &Slide, rect: Rect) -> f32 {
            rect.height() * 2.0
        }
        fn unsupported(&self, slide: &Slide) -> Vec<Problem> {
            slide
                .blocks
                .iter()
                .filter(|b| matches!(b, Block::CodeBlock { .. }))
                .map(|_| Problem::new("design", "code is not shown"))
                .collect()
        }
    }

    /// The new slide drops in from above, a second long.
    struct Drop;

    impl Transition for Drop {
        fn name(&self) -> &str {
            TRANSITION
        }
        fn summary(&self) -> &str {
            "The next slide drops in from above."
        }
        fn duration(&self) -> f32 {
            1.0
        }
        fn look(&self, t: f32, _forward: bool, rect: Rect) -> (SideLook, SideLook) {
            let to = SideLook::SHOWN.with_offset(Vec2::new(0.0, (t - 1.0) * rect.height()));
            (SideLook::SHOWN.with_opacity(1.0 - t), to)
        }
    }
}
