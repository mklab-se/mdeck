//! `mdeck ai story`: write story scripts for the Ember theme with AI.
//!
//! For each target slide the model receives the story vocabulary, the deck's
//! outline and cast so far, the slide's copy and notes, and the author's
//! ```@story hint when there is one. It answers with JSON (every JSON
//! document is valid YAML, and models are more reliable at strict JSON than at
//! indentation), which is validated and written to the YAML sidecar.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::parser::{self, Block, Inline, Presentation, Slide};
use crate::render::illustration::Library;
use crate::render::story::sidecar::{self, Entry, Sidecar};
use crate::render::story::{self, Script};

const SYSTEM_PROMPT: &str = r#"You choreograph the particle field behind a presentation slide.

The slide's copy is on the left. You direct a small scene on the stage to the right: a cast of people and props, flows of light between them, and beats the presenter releases one by one with the space bar. The look is calm, monochrome and precise. Particles are actors and nothing rushes at the viewer.

Rules for good stories:
- Lead with a person. Name them (a first name), give them an ordinary working situation. Never abstract labels for people. People are the figure kinds: person, man, woman, and the specialised thermographer, presenter-up and presenter-down when the situation calls for one.
- A hooded figure is the attacker or the risk. Use it rarely; its absence is itself an argument.
- Props are things the person touches: laptop, inbox, doc, db, cloud, mail, folder, orb (a model or assistant), box (a system), gate (a control), and whatever else the kinds list below offers (server, phone, robot, ...). Use only listed kinds.
- Two to six cast members. Every cast member has a unique cell; spread them out.
- Two to five beats. Each beat shows what appears or heats up, and has one short spoken line (max 140 characters) the presenter can say verbatim.
- Beat 0 is what is visible when the slide appears. Flows start at the beat where the transfer happens.
- Labels are two or three words, sentence case.
- If the author supplied a hint, follow it faithfully; it outranks the slide copy.

Answer with ONE JSON object and nothing else, in exactly this shape:
{
  "cast": [ { "id": "anders", "kind": "person", "label": "Anders", "cell": "left" },
            { "id": "queue", "kind": "inbox", "label": "Support queue", "cell": "center-top" },
            { "id": "model", "kind": "orb", "label": "The model", "cell": "right", "fill": "brain" } ],
  "flows": [ { "from": "queue", "to": "model", "color": "white", "at": 1 },
             { "from": "model", "to": "anders", "color": "ember", "at": 2 } ],
  "beats": [ { "show": ["anders", "queue"], "say": "Anders stopped reading the tickets." },
             { "show": ["model"], "say": "He pointed the assistant at the queue." },
             { "hot": ["model"], "say": "Nobody noticed what came back." } ]
}

Allowed values:
"#;

/// Plain-text rendering of a slide's copy for the prompt.
fn slide_text(slide: &Slide) -> String {
    fn inlines(v: &[Inline]) -> String {
        v.iter()
            .map(|i| match i {
                Inline::Text(s) | Inline::Code(s) => s.clone(),
                Inline::Bold(c) | Inline::Italic(c) | Inline::Strikethrough(c) => inlines(c),
                Inline::Link { text, .. } => inlines(text),
                Inline::Math { tex, .. } => tex.clone(),
            })
            .collect()
    }
    let mut out = String::new();
    for b in &slide.blocks {
        match b {
            Block::Heading { level, inlines: v } => {
                out.push_str(&format!("{} {}\n", "#".repeat(*level as usize), inlines(v)));
            }
            Block::Paragraph { inlines: v } | Block::BlockQuote { inlines: v } => {
                out.push_str(&inlines(v));
                out.push('\n');
            }
            Block::List { items, .. } => {
                for it in items {
                    out.push_str(&format!("- {}\n", inlines(&it.inlines)));
                }
            }
            Block::CodeBlock { code, .. } => out.push_str(&format!("```\n{code}\n```\n")),
            _ => {}
        }
    }
    out
}

fn slide_title(slide: &Slide) -> Option<String> {
    slide.blocks.iter().find_map(|b| match b {
        Block::Heading { inlines, .. } => Some(
            inlines
                .iter()
                .filter_map(|i| match i {
                    Inline::Text(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect::<String>(),
        ),
        _ => None,
    })
}

/// Build the user message for one slide.
fn user_prompt(pres: &Presentation, index: usize, cast_so_far: &[String]) -> String {
    let slide = &pres.slides[index];
    let mut msg = String::new();
    if let Some(t) = &pres.meta.title {
        msg.push_str(&format!("Deck: {t}\n"));
    }
    if let Some(h) = &pres.meta.story {
        msg.push_str(&format!("Deck-level direction from the author:\n{h}\n\n"));
    }
    let outline: Vec<String> = pres
        .slides
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{}{}. {}",
                if i == index { "> " } else { "  " },
                i + 1,
                slide_title(s).unwrap_or_else(|| "(untitled)".into())
            )
        })
        .collect();
    msg.push_str(&format!(
        "Deck outline (this slide marked with >):\n{}\n\n",
        outline.join("\n")
    ));
    if !cast_so_far.is_empty() {
        msg.push_str(&format!(
            "Cast already introduced on earlier slides (reuse the same people where it makes sense): {}\n\n",
            cast_so_far.join(", ")
        ));
    }
    msg.push_str(&format!(
        "Slide {} copy:\n{}\n",
        index + 1,
        slide_text(slide)
    ));
    if let Some(n) = &slide.notes {
        msg.push_str(&format!("\nSpeaker notes:\n{n}\n"));
    }
    if let Some(h) = &slide.story_hint {
        msg.push_str(&format!("\nAUTHOR'S STORY HINT (follow this):\n{h}\n"));
    }
    msg.push_str("\nWrite the JSON now.");
    msg
}

/// Pull the JSON object out of a model reply that may wrap it in a fence.
fn extract_json(reply: &str) -> &str {
    let t = reply.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .map(|r| r.trim_end_matches("```"))
        .unwrap_or(t);
    let start = t.find('{').unwrap_or(0);
    let end = t.rfind('}').map(|i| i + 1).unwrap_or(t.len());
    t[start..end.max(start)].trim()
}

/// Ask the model for one slide's script, retrying once with the validation
/// error when the first answer does not pass.
pub async fn generate_script(
    client: &ailloy::Client,
    pres: &Presentation,
    index: usize,
    cast_so_far: &[String],
    lib: &mut Library,
) -> Result<Script> {
    let system = format!("{SYSTEM_PROMPT}{}\n", story::vocabulary(&lib.names()));
    let mut history = vec![
        ailloy::Message::system(&system),
        ailloy::Message::user(user_prompt(pres, index, cast_so_far)),
    ];
    let mut last_err = String::new();
    for attempt in 0..2 {
        let response = client.chat(&history).await.context("AI request failed")?;
        let json = extract_json(&response.content).to_string();
        let parsed = Script::parse(&json).and_then(|script| {
            let layout = pres.slides[index].layout;
            let unknown = script.unknown_kinds(lib);
            if !unknown.is_empty() {
                return Err(format!(
                    "unknown kinds: {}. Use only the listed kinds",
                    unknown.join(", ")
                ));
            }
            let staged = story::stage(&script, layout, 16.0 / 9.0, lib);
            let clashes = story::label_collisions(&staged, 16.0 / 9.0);
            if clashes.is_empty() {
                Ok(script)
            } else {
                let list: Vec<String> = clashes
                    .iter()
                    .map(|(a, b)| format!("`{a}` overlaps `{b}`"))
                    .collect();
                Err(format!(
                    "labels collide: {}. Use shorter labels or cells further apart",
                    list.join(", ")
                ))
            }
        });
        match parsed {
            Ok(script) => return Ok(script),
            Err(e) => {
                last_err = e.clone();
                if attempt == 0 {
                    history.push(ailloy::Message::assistant(&response.content));
                    history.push(ailloy::Message::user(format!(
                        "That script is invalid: {e}. Fix it and answer with the corrected JSON object only."
                    )));
                }
            }
        }
    }
    bail!("model produced an invalid script: {last_err}")
}

/// Parse `--range 3-7` (1-based, inclusive).
fn parse_range(range: &str, count: usize) -> Result<Vec<usize>> {
    let (a, b) = range.split_once('-').context("range must look like 3-7")?;
    let a: usize = a.trim().parse().context("range start")?;
    let b: usize = b.trim().parse().context("range end")?;
    if a == 0 || b < a || b > count {
        bail!("range {range} is outside 1-{count}");
    }
    Ok((a - 1..b).collect())
}

/// Generate a script for one slide from a running app: loads the deck and
/// sidecar afresh, writes the entry, and returns the script. Blocks; call it
/// from a worker thread.
pub fn generate_one_blocking(deck: &Path, index: usize) -> Result<Script> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let content = std::fs::read_to_string(deck)?;
        let base = deck.parent().unwrap_or(Path::new("."));
        let pres = parser::parse(&content, base);
        if index >= pres.slides.len() {
            bail!("slide {} does not exist", index + 1);
        }
        let client = ailloy::Client::for_capability("chat")?;
        let mut sc = sidecar::load(deck)
            .map_err(|e| anyhow::anyhow!(e))?
            .unwrap_or(Sidecar {
                version: sidecar::VERSION,
                slides: vec![],
            });
        if sc.slides.iter().any(|e| e.pinned && e.slide == index + 1) {
            bail!("slide {} has a pinned (hand-written) story", index + 1);
        }
        let cast = known_cast(&sc);
        let mut lib = Library::for_deck(Some(base));
        let script = generate_script(&client, &pres, index, &cast, &mut lib).await?;
        upsert(&mut sc, &pres, index, script.clone());
        sidecar::save(deck, &sc).map_err(|e| anyhow::anyhow!(e))?;
        Ok(script)
    })
}

fn known_cast(sc: &Sidecar) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for e in &sc.slides {
        for m in &e.scene.cast {
            if m.kind == "person"
                && let Some(l) = &m.label
                && !names.contains(l)
            {
                names.push(l.clone());
            }
        }
    }
    names
}

fn upsert(sc: &mut Sidecar, pres: &Presentation, index: usize, script: Script) {
    let slide = &pres.slides[index];
    let entry = Entry {
        slide: index + 1,
        title: slide_title(slide),
        hash: sidecar::slide_hash(slide, pres.meta.story.as_deref()),
        generated: Some(timestamp()),
        pinned: false,
        scene: script,
    };
    sc.slides.retain(|e| e.slide != index + 1);
    sc.slides.push(entry);
    sc.slides.sort_by_key(|e| e.slide);
}

/// UTC timestamp as `YYYY-MM-DD HH:MM`, without a date crate.
pub(crate) fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60
    )
}

/// The `mdeck ai story` command.
pub async fn run(
    file: PathBuf,
    slide: Option<usize>,
    range: Option<String>,
    stale_only: bool,
    force: bool,
    dry_run: bool,
    quiet: bool,
) -> Result<()> {
    let content = std::fs::read_to_string(&file)?;
    let base = file.parent().unwrap_or(Path::new("."));
    let pres = parser::parse(&content, base);
    if pres.slides.is_empty() {
        bail!("No slides found in {}", file.display());
    }
    let count = pres.slides.len();

    let (path, both) = sidecar::resolve_path(&file);
    if both && !quiet {
        eprintln!(
            "{} both .yaml and .yml sidecars exist; using {}",
            "Warning:".yellow().bold(),
            path.display()
        );
    }
    let mut sc = sidecar::load(&file)
        .map_err(|e| anyhow::anyhow!(e))?
        .unwrap_or(Sidecar {
            version: sidecar::VERSION,
            slides: vec![],
        });

    let mut targets: Vec<usize> = if let Some(n) = slide {
        if n == 0 || n > count {
            bail!("slide {n} is outside 1-{count}");
        }
        vec![n - 1]
    } else if let Some(r) = &range {
        parse_range(r, count)?
    } else {
        (0..count).collect()
    };

    // Only slides with a stage get stories; pinned (hand-written) entries are
    // left alone.
    let requested = targets.len();
    targets.retain(|&i| story::allowed(&pres.slides[i], i));
    let no_stage = requested - targets.len();
    if no_stage > 0 && !quiet {
        eprintln!(
            "Skipping {no_stage} slide{} without a stage (code, charts, diagrams, tables, images, titles).",
            if no_stage == 1 { "" } else { "s" }
        );
    }
    targets.retain(|&i| !sc.slides.iter().any(|e| e.pinned && e.slide == i + 1));

    // Entries for slides that can no longer play a story are dropped.
    let before = sc.slides.len();
    sc.slides.retain(|e| {
        pres.slides
            .get(e.slide.wrapping_sub(1))
            .is_some_and(|s| story::allowed(s, e.slide - 1))
    });
    let pruned = before - sc.slides.len();
    if pruned > 0 && !dry_run {
        sidecar::save(&file, &sc).map_err(|e| anyhow::anyhow!(e))?;
        if !quiet {
            eprintln!(
                "Removed {pruned} stored stor{} for slides without a stage.",
                if pruned == 1 { "y" } else { "ies" }
            );
        }
    }

    // Unless forced, skip slides whose sidecar entry is still current.
    if !force {
        targets.retain(|&i| {
            let hash = sidecar::slide_hash(&pres.slides[i], pres.meta.story.as_deref());
            !sc.slides.iter().any(|e| e.hash == hash)
        });
    }
    if stale_only {
        targets.retain(|&i| sc.slides.iter().any(|e| e.slide == i + 1));
    }

    if targets.is_empty() {
        if !quiet {
            eprintln!("Nothing to do: every targeted slide already has a current story.");
        }
        return Ok(());
    }

    let client = ailloy::Client::for_capability("chat")?;
    if !quiet {
        eprintln!(
            "Writing stories for {} slide{} of {} → {}",
            targets.len(),
            if targets.len() == 1 { "" } else { "s" },
            file.display(),
            path.display()
        );
    }

    let mut failures = 0usize;
    let mut lib = Library::for_deck(Some(base));
    for i in targets {
        let title = slide_title(&pres.slides[i]).unwrap_or_default();
        if !quiet {
            eprint!("  slide {:>2}  {:<40} ", i + 1, truncate(&title, 40));
        }
        let cast = known_cast(&sc);
        match generate_script(&client, &pres, i, &cast, &mut lib).await {
            Ok(script) => {
                if !quiet {
                    eprintln!(
                        "{} {} cast, {} beats",
                        "✓".green(),
                        script.cast.len(),
                        script.beats.len()
                    );
                    for (b, beat) in script.beats.iter().enumerate() {
                        if let Some(say) = &beat.say {
                            eprintln!("            {}  {say}", format!("{}.", b + 1).dimmed());
                        }
                    }
                }
                if dry_run {
                    if !quiet {
                        eprintln!(
                            "{}",
                            serde_yaml::to_string(&script).unwrap_or_default().dimmed()
                        );
                    }
                    continue;
                }
                upsert(&mut sc, &pres, i, script);
                // Save after every slide so an interruption keeps the work so far.
                sidecar::save(&file, &sc).map_err(|e| anyhow::anyhow!(e))?;
            }
            Err(e) => {
                failures += 1;
                if !quiet {
                    eprintln!("{} {e}", "✗".red());
                }
            }
        }
    }

    if failures > 0 {
        bail!(
            "{failures} slide(s) failed; the others were saved to {}",
            path.display()
        );
    }
    if !quiet {
        if dry_run {
            eprintln!("Dry run: nothing was written.");
        } else {
            eprintln!("Done. Present with `mdeck {}`.", file.display());
        }
    }
    Ok(())
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let cut: String = s.chars().take(n.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_json_from_fenced_replies() {
        let fenced = "Here you go:\n```json\n{\"cast\": []}\n```";
        assert_eq!(extract_json(fenced), "{\"cast\": []}");
        assert_eq!(extract_json("  {\"a\":1}  "), "{\"a\":1}");
    }

    #[test]
    fn ranges_are_one_based_and_inclusive() {
        assert_eq!(parse_range("3-5", 10).unwrap(), vec![2, 3, 4]);
        assert!(parse_range("0-2", 10).is_err());
        assert!(parse_range("5-3", 10).is_err());
        assert!(parse_range("9-12", 10).is_err());
    }

    #[test]
    fn prompt_puts_the_hint_last_and_marks_the_slide() {
        let md = "---\ntitle: T\n@story: Keep it calm\n---\n# One\n\n- a\n\n```@story\nShow a person at a desk.\n```\n\n---\n\n# Two\n";
        let pres = parser::parse(md, Path::new("."));
        let p = user_prompt(&pres, 0, &["Anders".into()]);
        assert!(p.contains("> 1. One"));
        assert!(p.contains("  2. Two"));
        assert!(p.contains("Keep it calm"));
        assert!(p.contains("Anders"));
        assert!(p.trim_end().ends_with("Write the JSON now."));
        assert!(p.contains("Show a person at a desk."));
    }
}
