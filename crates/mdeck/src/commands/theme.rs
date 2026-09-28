//! `mdeck theme`: list, check, scaffold (optionally from a design system
//! with AI) and preview custom themes.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::cli::ThemeCommands;
use crate::theme::file::ThemeFile;
use crate::theme::lookup::{self, Lookup, Origin};
use crate::theme::validate;

const SPEC: &str = include_str!("../../doc/mdeck-spec.md");

/// Section 9.4 of the spec: the theme format, the design-system mapping and
/// the conversion recipe. It is what the model reads for `--from`, so the
/// spec stays the one description of the format.
pub fn theme_section() -> &'static str {
    let start = SPEC.find("### 9.4 Custom themes").unwrap_or(0);
    let end = SPEC[start..]
        .find("\n## 10.")
        .map(|i| start + i)
        .unwrap_or(SPEC.len());
    &SPEC[start..end]
}

/// Themes are looked up from the working directory, as for a deck there.
fn here() -> Lookup {
    Lookup::for_deck(Some(Path::new(".")))
}

pub async fn run(cmd: ThemeCommands, quiet: bool) -> Result<()> {
    match cmd {
        ThemeCommands::List => list(),
        ThemeCommands::Check { name } => check(&name),
        ThemeCommands::New {
            name,
            from,
            user,
            force,
        } => new(&name, from.as_deref(), user, force, quiet).await,
        ThemeCommands::Preview {
            name,
            output_dir,
            width,
            height,
        } => preview(&name, output_dir, width, height),
    }
}

fn list() -> Result<()> {
    let l = here();
    for found in l.available() {
        let (label, path) = match &found.origin {
            Origin::Builtin => ("built-in", String::new()),
            o => (
                o.label(),
                o.path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            ),
        };
        let desc = match l.load_found(&found) {
            Ok(b) => format!("{} engine", b.theme.engine.name()),
            Err(_) => "invalid (run `mdeck theme check`)".red().to_string(),
        };
        println!(
            "  {:<16} {:<9} {:<14} {}",
            found.name,
            label,
            desc,
            path.dimmed()
        );
    }
    Ok(())
}

/// Print a theme's problems; returns whether it loaded.
fn report(name: &str, l: &Lookup) -> bool {
    match l.load(name) {
        Err(e) => {
            println!("{} {e}", "error:".red().bold());
            false
        }
        Ok(built) => {
            let t = &built.theme;
            let from = t
                .source
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "built-in".into());
            println!(
                "{} ({from}): {} engine, countdown {}",
                t.name.bold(),
                t.engine.name(),
                t.countdown.name()
            );
            let advice = validate::review(t);
            for w in &built.warnings {
                println!("  {} {w}", "warning:".yellow().bold());
            }
            for a in &advice {
                println!("  {} {a}", "contrast:".yellow().bold());
            }
            if built.warnings.is_empty() && advice.is_empty() {
                println!("  {}", "No issues found.".green());
            }
            true
        }
    }
}

fn check(name: &str) -> Result<()> {
    if !report(name, &here()) {
        std::process::exit(1);
    }
    Ok(())
}

/// Where `new` writes: `./themes` or the user folder.
fn target_dir(user: bool) -> Result<PathBuf> {
    if user {
        lookup::user_dir().context("no user configuration folder on this system")
    } else {
        Ok(PathBuf::from("themes"))
    }
}

/// The starter theme `mdeck theme new` writes: every key, commented, with
/// the values `dark` gives.
fn starter(name: &str) -> String {
    format!(
        r##"# {name}: a custom MDeck theme. Every key is optional; unset keys come
# from the theme named by `extends`. See `mdeck spec`, section 9.4.
# Check it with `mdeck theme check {name}` and look at it with
# `mdeck theme preview {name} --output-dir /tmp/{name}`.
name: {name}
extends: dark              # dark | light | nord | ember | another theme
# engine: plain            # plain | particles | led | splitflap | laser | blocks
# countdown: none          # none | plain | burst (burst: the engine's own countdown)
colors:
  background: "#1e1e1e"    # slide background
  text: "#c8c8c8"          # body text
  heading: "#ffffff"       # headings
  accent: "#5294e2"        # links, quote bars, highlights
  # muted: "#8a8a8a"       # captions, eyebrows, slide numbers
  # strong: "#ffffff"      # **bold** text
  # rule: "#3a3a3a"        # hairlines
  # accent-soft: "#8fb8ee" # lighter accent
  # secondary: "#e8a838"   # a second, rarer highlight
  # code-background: "#2d2d2d"
  # code-text: "#d4d4d4"
  # positive: "#5cdb95"
  # negative: "#ff6b6b"
  # series: ["#5cb8ff", "#ff7e67", "#5cdb95", "#e8a838"]
# fonts:                   # a bundled face or a .ttf/.otf file in this folder
#   display: sans          # sans, mono, spectral-light, hanken-light,
#   body: sans             # hanken-regular, hanken-medium, jetbrains-mono
#   strong: sans
#   mono: mono
# sizes: {{ h1: 96, h2: 72, h3: 52, body: 44, code: 30 }}   # px at 1920x1080
# text: {{ line-height: 1.4 }}
# charts: {{ fill-opacity: 0.85 }}
# code: {{ syntax: base16-ocean.dark }}
# logo:                    # a PNG or SVG in this folder, in a corner of every slide
#   file: logo.svg
#   position: top-right    # top-left | top-right | bottom-left | bottom-right
#   height: 56             # px at 1920x1080
#   opacity: 0.6
"##
    )
}

async fn new(name: &str, from: Option<&Path>, user: bool, force: bool, quiet: bool) -> Result<()> {
    if !lookup::valid_name(name) {
        bail!("theme names are lowercase letters, digits, '-' and '_' (got '{name}')");
    }
    let dir = target_dir(user)?;
    let single = dir.join(format!("{name}.yaml"));
    let folder = dir.join(name);
    if !force && (single.exists() || folder.join("theme.yaml").exists()) {
        bail!(
            "theme '{name}' already exists in {} (use --force to overwrite)",
            dir.display()
        );
    }
    let Some(from) = from else {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(&single, starter(name))?;
        if !quiet {
            println!("Wrote {}", single.display());
            println!("Use it with `@theme: {name}` in a deck's frontmatter.");
        }
        return Ok(());
    };

    if !from.is_dir() {
        bail!("{} is not a folder", from.display());
    }
    if !crate::commands::ai::has_capability("chat") {
        bail!("`--from` writes the theme with AI, which is not configured: run `mdeck ai enable`");
    }
    let sources = gather(from)?;
    if sources.files.is_empty() {
        bail!(
            "found no design system files in {} (SKILL.md, readme.md, CSS, *.tokens.json, tailwind config)",
            from.display()
        );
    }
    // Font files go into a theme folder so the theme can name them.
    let (out_dir, out_file) = if sources.fonts.is_empty() && sources.logos.is_empty() {
        (dir.clone(), single.clone())
    } else {
        (folder.clone(), folder.join("theme.yaml"))
    };
    std::fs::create_dir_all(&out_dir)?;
    let mut copied = Vec::new();
    for f in &sources.fonts {
        let name = f.file_name().context("font file name")?;
        let dest_dir = out_dir.join("fonts");
        std::fs::create_dir_all(&dest_dir)?;
        std::fs::copy(f, dest_dir.join(name))?;
        copied.push(format!("fonts/{}", name.to_string_lossy()));
    }
    let mut logos = Vec::new();
    for f in &sources.logos {
        let name = f.file_name().context("logo file name")?;
        let dest_dir = out_dir.join("logos");
        std::fs::create_dir_all(&dest_dir)?;
        std::fs::copy(f, dest_dir.join(name))?;
        logos.push(format!("logos/{}", name.to_string_lossy()));
    }
    if !quiet {
        println!(
            "Reading {} file(s) from {} and writing {} with AI...",
            sources.files.len(),
            from.display(),
            out_file.display()
        );
    }
    let client = ailloy::Client::for_capability("chat")?;
    let yaml = generate(&client, name, &sources, &copied, &logos, &dir, &out_file).await?;
    std::fs::write(&out_file, yaml)?;
    if !quiet {
        println!("Wrote {}", out_file.display());
    }
    println!();
    report(name, &written_in(&dir));
    if !quiet {
        println!();
        println!("Look at it: mdeck theme preview {name} --output-dir /tmp/{name}");
    }
    Ok(())
}

/// A lookup that finds themes written to `dir` (a deck's `themes/` folder or
/// the user folder) before anything else.
fn written_in(dir: &Path) -> Lookup {
    Lookup {
        deck: None,
        user: Some(dir.to_path_buf()),
    }
}

/// What `--from` found in a design system folder.
struct Sources {
    /// (relative path, contents), in reading order, capped in size.
    files: Vec<(String, String)>,
    /// TTF/OTF files to copy into the theme folder.
    fonts: Vec<PathBuf>,
    /// Logo files (PNG or SVG) to copy into the theme folder.
    logos: Vec<PathBuf>,
}

/// Most text sent to the model, in bytes.
const MAX_TOTAL: usize = 160_000;
/// Most text taken from one file, in bytes.
const MAX_FILE: usize = 40_000;

fn gather(root: &Path) -> Result<Sources> {
    let mut all = Vec::new();
    walk(root, 0, &mut all)?;
    // Rules first, then tokens: the order the model should read them in.
    let rank = |rel: &str| {
        let l = rel.to_ascii_lowercase();
        let file = l.rsplit('/').next().unwrap_or(&l).to_string();
        if file == "skill.md" {
            0
        } else if file.starts_with("readme") || file.starts_with("design") {
            1
        } else if l.ends_with(".tokens.json") || l.ends_with(".tokens") {
            2
        } else if file.starts_with("tailwind.config") {
            3
        } else if l.ends_with(".css") {
            4
        } else {
            9
        }
    };
    let mut texts: Vec<(usize, String, PathBuf)> = Vec::new();
    let mut fonts = Vec::new();
    let mut logos = Vec::new();
    for p in all {
        let rel = p
            .strip_prefix(root)
            .unwrap_or(&p)
            .to_string_lossy()
            .replace('\\', "/");
        let l = rel.to_ascii_lowercase();
        if l.ends_with(".ttf") || l.ends_with(".otf") {
            fonts.push(p);
            continue;
        }
        if (l.ends_with(".svg") || l.ends_with(".png")) && l.contains("logo") {
            logos.push(p);
            continue;
        }
        let r = rank(&rel);
        // Only the design system's own documents: its top-level readme and
        // skill, token files and stylesheets (component docs are noise here).
        let top_level = !rel.contains('/');
        let wanted = match r {
            0 | 1 => top_level,
            2..=4 => true,
            _ => false,
        };
        if wanted {
            texts.push((r, rel, p));
        }
    }
    texts.sort();
    let mut files = Vec::new();
    let mut total = 0;
    for (_, rel, p) in texts {
        let Ok(mut text) = std::fs::read_to_string(&p) else {
            continue;
        };
        if text.len() > MAX_FILE {
            let mut cut = MAX_FILE;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
        }
        if total + text.len() > MAX_TOTAL {
            break;
        }
        total += text.len();
        files.push((rel, text));
    }
    logos.sort();
    Ok(Sources {
        files,
        fonts,
        logos,
    })
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) -> Result<()> {
    if depth > 4 {
        return Ok(());
    }
    for e in std::fs::read_dir(dir)?.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "node_modules" || name.starts_with('_') {
            continue;
        }
        if p.is_dir() {
            walk(&p, depth + 1, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}

const SYSTEM_PROMPT: &str = "You convert design systems into MDeck presentation themes. \
You answer with one YAML document and nothing else: no prose, no code fences. \
Follow the format and the mapping in the specification exactly; unknown keys are errors.";

fn user_prompt(name: &str, sources: &Sources, fonts: &[String], logos: &[String]) -> String {
    let mut p = String::new();
    p.push_str("Specification of MDeck theme files (from `mdeck spec`):\n\n");
    p.push_str(theme_section());
    p.push_str(&format!(
        "\n\nWrite the MDeck theme `{name}` for the design system below.\n\
         - Use `name:` with the design system's brand name.\n\
         - Resolve every CSS variable and alias to a hex colour (#rrggbb or #rrggbbaa).\n\
         - Set every colour key in the mapping table that the design system can answer.\n\
         - Keep the built-in slide sizes unless the design system is itself about slides.\n\
         - Choose `extends:` from dark or light by the design system's background.\n\
         - Choose an engine with `countdown: burst` only when the brand already has that \
           character: `particles` (glow), `led` (neon, signage), `laser` (engineering, \
           precision), `blocks` (games, playful), `splitflap` (travel, schedules; text only). \
           Otherwise leave the engine out.\n"
    ));
    if fonts.is_empty() {
        p.push_str(
            "- No font files are available: use a bundled face where the design system names \
             the same family, and leave other font roles out.\n",
        );
    } else {
        p.push_str(&format!(
            "- These font files are in the theme folder and can be named by path: {}. \
             Bundled faces are fine where the family matches.\n",
            fonts.join(", ")
        ));
    }
    if !logos.is_empty() {
        p.push_str(&format!(
            "- These logo files are in the theme folder: {}. Set `logo.file` to the one that \
             reads on the theme's background (a white or light mark on a dark background, a \
             dark mark on a light one), and follow the design system's rules for showing it \
             (quiet brands: lower opacity).\n",
            logos.join(", ")
        ));
    }
    p.push_str("- Add a short YAML comment on the keys where you made a judgement call.\n\n");
    for (rel, text) in &sources.files {
        p.push_str(&format!("===== {rel} =====\n{text}\n\n"));
    }
    p
}

/// Strip code fences a model may add despite being asked not to.
fn strip_fences(s: &str) -> String {
    let t = s.trim();
    let t = t
        .strip_prefix("```yaml")
        .or_else(|| t.strip_prefix("```yml"))
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    format!("{}\n", t.trim())
}

/// Ask for the theme, check it the way MDeck will load it, and retry once
/// with the error when it does not pass.
async fn generate(
    client: &ailloy::Client,
    name: &str,
    sources: &Sources,
    fonts: &[String],
    logos: &[String],
    dir: &Path,
    out_file: &Path,
) -> Result<String> {
    let mut history = vec![
        ailloy::Message::system(SYSTEM_PROMPT),
        ailloy::Message::user(user_prompt(name, sources, fonts, logos)),
    ];
    let mut last_err = String::new();
    for attempt in 0..2 {
        let response = client.chat(&history).await.context("AI request failed")?;
        let yaml = strip_fences(&response.content);
        let verdict = ThemeFile::parse(&yaml).and_then(|_| {
            // Load it for real (fonts, extends) from where it will live.
            std::fs::write(out_file, &yaml).map_err(|e| e.to_string())?;
            let l = written_in(dir);
            let found = l
                .find_all(name)
                .into_iter()
                .find(|f| f.origin != Origin::Builtin)
                .ok_or_else(|| "the written theme could not be found".to_string())?;
            l.load_found(&found).map(|_| ())
        });
        match verdict {
            Ok(()) => return Ok(yaml),
            Err(e) => {
                last_err = e.clone();
                if attempt == 0 {
                    history.push(ailloy::Message::assistant(&response.content));
                    history.push(ailloy::Message::user(format!(
                        "MDeck rejects that theme: {e}. Fix it and answer with the corrected YAML only."
                    )));
                }
            }
        }
    }
    let _ = std::fs::remove_file(out_file);
    bail!("the model did not produce a valid theme: {last_err}")
}

/// A sampler deck that shows every part of a theme once.
const SAMPLER: &str = r#"---
title: Theme preview
---

# Theme preview

A sampler of every part of a theme

---

## Bullets and inline styles

- Body text with **bold**, *emphasis* and `inline code`
- A [link](https://example.com) and a longer line that wraps onto a second line to show the leading
- Nested items
  - stay readable
  - at a smaller indent

---

## Code

```rust
/// Greets the room.
fn main() {
    let name = "world";
    println!("Hello, {name}!");
}
```

---

## Chart

```@barchart
- Design: 42
- Build: 68
- Launch: 55
- Grow: 81
```

---

## KPIs

```@kpi
- Revenue: $4.2M (trend: +12%)
- Churn: 3.2% (trend: -0.5%)
- NPS: 61 (trend: +4)
```

---

## Diagram

```@architecture
- Client -> Gateway
- Gateway -> Service
- Service -> Database
```

---

## Table

| Role | Colour | Used for |
|---|---|---|
| Accent | the brand colour | links, highlights |
| Text | a quiet step | body copy |
| Heading | the brightest | titles |

---

> Simplicity is the ultimate sophistication.

-- Leonardo da Vinci
"#;

fn preview(name: &str, output_dir: PathBuf, width: u32, height: u32) -> Result<()> {
    let built = here().load(name).map_err(|e| anyhow::anyhow!("{e}"))?;
    for w in &built.warnings {
        eprintln!("warning: theme: {w}");
    }
    let dir = std::env::temp_dir().join(format!("mdeck-theme-preview-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let deck = dir.join("theme-preview.md");
    std::fs::write(&deck, SAMPLER)?;
    let result = crate::commands::export::run(
        deck,
        output_dir,
        width,
        height,
        false,
        None,
        None,
        crate::commands::export::Format::Png,
        false,
        crate::commands::export::ThemeChoice::Given(Box::new(built.theme)),
        None,
    );
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn the_theme_section_is_the_whole_of_9_4() {
        let s = theme_section();
        assert!(s.starts_with("### 9.4 Custom themes"));
        assert!(s.contains("From a design system to a theme"));
        assert!(s.contains("mdeck theme preview"));
        assert!(!s.contains("## 10."));
    }

    #[test]
    fn the_starter_theme_loads() {
        let f = ThemeFile::parse(&starter("acme"))
            .unwrap()
            .over(&lookup::builtin_file("dark").unwrap());
        let t = Theme::build("acme", &f).unwrap();
        assert_eq!(t.theme.name, "acme");
        assert!(t.warnings.is_empty());
    }

    #[test]
    fn fences_are_stripped() {
        assert_eq!(strip_fences("```yaml\nname: x\n```"), "name: x\n");
        assert_eq!(strip_fences("name: x"), "name: x\n");
    }

    #[test]
    fn the_sampler_deck_parses_into_its_slides() {
        let p = crate::parser::parse(SAMPLER, Path::new("."));
        assert_eq!(p.slides.len(), 8);
    }

    #[test]
    fn gather_reads_rules_and_tokens_and_finds_fonts() {
        let d = std::env::temp_dir().join(format!("mdeck-gather-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("tokens")).unwrap();
        std::fs::create_dir_all(d.join("components/core")).unwrap();
        std::fs::create_dir_all(d.join("fonts")).unwrap();
        std::fs::write(d.join("SKILL.md"), "skill").unwrap();
        std::fs::write(d.join("readme.md"), "rules").unwrap();
        std::fs::write(d.join("tokens/colors.css"), ":root{--a:#fff}").unwrap();
        std::fs::write(d.join("components/core/Button.prompt.md"), "noise").unwrap();
        std::fs::write(d.join("_ds_bundle.js"), "noise").unwrap();
        std::fs::write(d.join("fonts/Brand.ttf"), "x").unwrap();
        std::fs::create_dir_all(d.join("assets/logos")).unwrap();
        std::fs::write(d.join("assets/logos/brand-white.svg"), "<svg/>").unwrap();
        std::fs::write(d.join("assets/photo.png"), "x").unwrap();
        let s = gather(&d).unwrap();
        let names: Vec<&str> = s.files.iter().map(|(r, _)| r.as_str()).collect();
        assert_eq!(names, ["SKILL.md", "readme.md", "tokens/colors.css"]);
        assert_eq!(s.fonts.len(), 1);
        assert_eq!(s.logos.len(), 1);
        std::fs::remove_dir_all(&d).ok();
    }
}
