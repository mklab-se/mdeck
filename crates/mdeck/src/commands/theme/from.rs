//! `mdeck theme new --from <folder>`: read a design system and have the
//! chat model write the theme.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::{report, theme_section, written_in};
use crate::commands::ai_reply;
use crate::theme::file::ThemeFile;
use crate::theme::lookup::Origin;

/// Where the new theme goes.
pub(super) struct Target<'a> {
    pub name: &'a str,
    /// `./themes` or the user folder.
    pub dir: &'a Path,
    /// `<dir>/<name>.yaml`.
    pub single: &'a Path,
    /// `<dir>/<name>/`, used when there are fonts or logos to copy.
    pub folder: &'a Path,
}

/// Write the theme `target.name` from the design system in `from`.
pub(super) async fn new_from(target: &Target<'_>, from: &Path, quiet: bool) -> Result<()> {
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
        (target.dir.to_path_buf(), target.single.to_path_buf())
    } else {
        (
            target.folder.to_path_buf(),
            target.folder.join("theme.yaml"),
        )
    };
    std::fs::create_dir_all(&out_dir)?;
    let fonts = copy_into(&sources.fonts, &out_dir, "fonts", "font file name")?;
    let logos = copy_into(&sources.logos, &out_dir, "logos", "logo file name")?;
    if !quiet {
        println!(
            "Reading {} file(s) from {} and writing {} with AI...",
            sources.files.len(),
            from.display(),
            out_file.display()
        );
    }
    let client = ailloy::Client::for_capability("chat")?;
    let request = Request {
        name: target.name,
        sources: &sources,
        fonts: &fonts,
        logos: &logos,
    };
    let yaml = generate(&client, &request, target.dir, &out_file).await?;
    std::fs::write(&out_file, yaml)?;
    let name = target.name;
    if !quiet {
        println!("Wrote {}", out_file.display());
    }
    println!();
    report(name, &written_in(target.dir));
    if !quiet {
        println!();
        println!("Look at it: mdeck theme preview {name} --output-dir /tmp/{name}");
    }
    Ok(())
}

/// Copy `files` into `<out_dir>/<sub>/`; returns their paths relative to `out_dir`.
fn copy_into(files: &[PathBuf], out_dir: &Path, sub: &str, what: &str) -> Result<Vec<String>> {
    let mut copied = Vec::new();
    for f in files {
        let name = f.file_name().context(what.to_string())?;
        let dest_dir = out_dir.join(sub);
        std::fs::create_dir_all(&dest_dir)?;
        std::fs::copy(f, dest_dir.join(name))?;
        copied.push(format!("{sub}/{}", name.to_string_lossy()));
    }
    Ok(copied)
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

/// Rules first, then tokens: the order the model should read them in.
fn rank(rel: &str) -> usize {
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
}

fn gather(root: &Path) -> Result<Sources> {
    let mut all = Vec::new();
    walk(root, 0, &mut all)?;
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
    logos.sort();
    Ok(Sources {
        files: read_capped(texts),
        fonts,
        logos,
    })
}

/// Read the files in order, each cut to `MAX_FILE`, until `MAX_TOTAL`.
fn read_capped(texts: Vec<(usize, String, PathBuf)>) -> Vec<(String, String)> {
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
    files
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

/// What the model is asked to write.
struct Request<'a> {
    name: &'a str,
    sources: &'a Sources,
    /// Font files copied into the theme folder, relative to it.
    fonts: &'a [String],
    /// Logo files copied into the theme folder, relative to it.
    logos: &'a [String],
}

fn user_prompt(req: &Request) -> String {
    let Request {
        name,
        sources,
        fonts,
        logos,
    } = req;
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
           character: `particles` (glow), `led` (neon, signage), `blocks` (games, playful), `splitflap` (travel, schedules; text only). \
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

/// The theme YAML in a reply, without a code fence the model may add.
fn strip_fences(s: &str) -> String {
    format!("{}\n", ai_reply::strip_fence(s, &["yaml", "yml"]))
}

/// Ask for the theme, check it the way MDeck will load it, and retry once
/// with the error when it does not pass.
async fn generate(
    client: &ailloy::Client,
    req: &Request<'_>,
    dir: &Path,
    out_file: &Path,
) -> Result<String> {
    let name = req.name;
    let history = vec![
        ailloy::Message::system(SYSTEM_PROMPT),
        ailloy::Message::user(user_prompt(req)),
    ];
    let validate = |reply: &str| {
        let yaml = strip_fences(reply);
        ThemeFile::parse(&yaml)
            .map_err(|e| e.to_string())
            .and_then(|_| {
                // Load it for real (fonts, extends) from where it will live.
                std::fs::write(out_file, &yaml).map_err(|e| e.to_string())?;
                let l = written_in(dir);
                let found = l
                    .find_all(name)
                    .into_iter()
                    .find(|f| f.origin != Origin::Builtin)
                    .ok_or_else(|| "the written theme could not be found".to_string())?;
                l.load_found(&found)
                    .map(|_| yaml)
                    .map_err(|e| e.to_string())
            })
    };
    let theme = ai_reply::chat_client_validated(
        client,
        history,
        validate,
        |e| {
            format!(
                "MDeck rejects that theme: {e}. Fix it and answer with the corrected YAML only."
            )
        },
        "the model did not produce a valid theme",
    )
    .await;
    if theme.is_err() {
        let _ = std::fs::remove_file(out_file);
    }
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fences_are_stripped() {
        assert_eq!(strip_fences("```yaml\nname: x\n```"), "name: x\n");
        assert_eq!(strip_fences("name: x"), "name: x\n");
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

    #[test]
    fn the_prompt_lists_fonts_logos_and_files_in_order() {
        let sources = Sources {
            files: vec![("readme.md".into(), "rules".into())],
            fonts: vec![],
            logos: vec![],
        };
        let req = Request {
            name: "acme",
            sources: &sources,
            fonts: &["fonts/A.ttf".into()],
            logos: &[],
        };
        let p = user_prompt(&req);
        assert!(p.contains("Write the MDeck theme `acme`"));
        assert!(p.contains("can be named by path: fonts/A.ttf."));
        assert!(!p.contains("logo files"));
        assert!(p.ends_with("===== readme.md =====\nrules\n\n"));
    }
}
