//! The `mdeck` binary: mdeck with its built-in engines, visuals and themes.
//! A custom build calls [`mdeck::run`] with its own extensions registered
//! next to the built-ins (D15).

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut registry = mdeck_sdk::registry::Registry::new();
    if let Err(e) = mdeck::builtins(&mut registry) {
        eprintln!("Error: {e}");
        return ExitCode::FAILURE;
    }
    mdeck::run(registry)
}
