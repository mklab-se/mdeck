//! The story prompt: what the model is told about the slide and the deck,
//! and how its answer is checked.

use anyhow::Result;

use crate::commands::ai_reply;
use crate::parser::{Block, Inline, Presentation, Slide};
use crate::render::illustration::Library;
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
                s.title().unwrap_or_else(|| "(untitled)".into())
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
    let history = vec![
        ailloy::Message::system(&system),
        ailloy::Message::user(user_prompt(pres, index, cast_so_far)),
    ];
    let validate = |reply: &str| {
        Script::parse(ai_reply::json_object(reply)).and_then(|script| {
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
        })
    };
    ai_reply::chat_client_validated(
        client,
        history,
        validate,
        |e| {
            format!(
                "That script is invalid: {e}. Fix it and answer with the corrected JSON object only."
            )
        },
        "model produced an invalid script",
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use std::path::Path;

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
