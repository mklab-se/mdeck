//! AI agent skill information for mdeck.
//!
//! `mdeck ai skill`: print the setup guide.
//! `mdeck ai skill --emit`: print the skill markdown file to stdout.
//! `mdeck ai skill --reference`: print the full reference documentation.

const SUPPLEMENT: &str = include_str!("../../doc/ai-reference-supplement.md");

pub fn run(emit: bool, reference: bool) {
    if emit {
        print_skill_file();
    } else if reference {
        print_reference();
    } else {
        print_setup_guide();
    }
}

fn print_setup_guide() {
    println!(
        r#"mdeck AI Skill Setup
====================

mdeck is a markdown-based presentation tool. A skill helps AI agents
write, check and convert decks in the mdeck 2 format.

To create the skill file, run:

  mdeck ai skill --emit > ~/.claude/skills/mdeck.md

Or ask your AI agent:

  "Use `mdeck ai skill --emit` to set up a skill for creating presentations"

The skill instructs the AI agent to run `mdeck ai skill --reference` at
runtime to fetch the full format specification and documentation, so the
agent always has up-to-date syntax details without bloating the skill file
itself."#
    );
}

fn print_skill_file() {
    print!(
        r#"---
name: mdeck
description: Write, check and present slide decks in plain markdown with mdeck 2. Recognised slide designs, 20 chart and diagram kinds, steps, speaker notes, themes and engines, PNG and PDF export, and optional AI-generated images and pictures.
---

# mdeck: presentations from markdown

Use mdeck when the user wants to create, edit, convert, check or present a slide deck written
in markdown.

## Load the current reference first

Before writing any deck content, run:

```bash
mdeck ai skill --reference
```

It prints the complete format reference for the installed mdeck (slides, designs, settings,
steps, notes, every visual's syntax, themes, engines, export, checking and the v1 to v2 map)
and a guide for agents. Do not write a deck from memory: the syntax is precise.

## Commands

- `mdeck <file.md>`: present (fullscreen; `--windowed`, `--presenter`, `--theme`, `--engine`)
- `mdeck <file.md> --check`: validate without opening a window; `-v` prints each slide's design
- `mdeck export <file.md>`: PNGs in `./export` (`--format pdf`, `--notes`, `--slide N`)
- `mdeck ai <file.md>`: generate every asset the deck is missing (`![prompt](generate:)` images,
  `icon: generate:` icons, pictures on art engines, point clouds)
- `mdeck spec --short`: the quick reference card
- `mdeck ai status`, `mdeck ai config`: AI configuration

## Workflow

1. Run `mdeck ai skill --reference` and read it.
2. Write or edit the deck. Plain YAML frontmatter (`theme: ember`), slide settings in HTML
   comments (`<!-- design: quote -->`), notes in ```` ```@notes ```` blocks.
3. Run `mdeck <file.md> --check` and fix every warning.
4. Export a slide (`mdeck export <file.md> --slide N -o /tmp/look`) and look at it when the
   layout matters.
5. If the deck has `generate:` placeholders or uses an art engine, run `mdeck ai <file.md>`.
"#
    );
}

fn print_reference() {
    println!("# mdeck Reference Documentation\n");
    println!("## Format Specification\n");
    println!("{}", super::spec::full_reference());
    println!("\n{SUPPLEMENT}");
}
