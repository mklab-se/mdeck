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

/// The registry in use: the installed one, else the built-ins.
pub fn get() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        let mut r = Registry::new();
        crate::builtins(&mut r).expect("the built-ins register without clashes");
        r
    })
}
