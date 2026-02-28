use crate::cli::{Cli, Shell};
use clap::CommandFactory;
use clap_complete::generate;
use colored::Colorize;
use std::io;

pub fn run(shell: Shell) {
    let shell_name = match shell {
        Shell::Bash => "bash",
        Shell::Zsh => "zsh",
        Shell::Fish => "fish",
        Shell::Powershell => "powershell",
    };

    let clap_shell = match shell {
        Shell::Bash => clap_complete::Shell::Bash,
        Shell::Zsh => clap_complete::Shell::Zsh,
        Shell::Fish => clap_complete::Shell::Fish,
        Shell::Powershell => clap_complete::Shell::PowerShell,
    };

    let mut cmd = Cli::command();
    generate(clap_shell, &mut cmd, "mdeck", &mut io::stdout());

    eprintln!();
    eprintln!(
        "{} For dynamic completions (with tab-completion), use instead:",
        "Tip:".bold()
    );
    eprintln!(
        "  {}",
        format!("source <(COMPLETE={shell_name} mdeck)").cyan()
    );
}
