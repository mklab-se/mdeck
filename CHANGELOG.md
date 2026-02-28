# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- Full CLI with clap: `presemd <file.md>` to launch presentations
- Subcommands: `ai init/status/remove`, `config show/set`, `completion`, `version`
- Shell completions for bash, zsh, fish, and powershell (static and dynamic)
- AI provider configuration with auto-detection (Claude, Codex, Copilot, Ollama)
- YAML-based configuration at `~/.config/presemd/config.yaml`
- Configurable defaults: theme, transition, aspect ratio
- Global flags: `--verbose`, `--quiet`, `--no-color`
- Sample presentations for testing (`sample-presentations/`)

## [0.1.1] - 2026-02-28

### Added

- Initial implementation with hardcoded demo slides
- Slide transitions: fade and horizontal slide with easing
- Keyboard navigation with arrow keys
- FPS overlay
- `--version` flag support
