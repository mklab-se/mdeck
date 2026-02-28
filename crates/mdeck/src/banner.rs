use colored::Colorize;

pub fn print_banner_with_version() {
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    println!("mdeck {}", version.dimmed());
}
