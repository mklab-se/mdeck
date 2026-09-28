//! Reading a chat model's answer: the fence it may wrap the answer in, the
//! JSON object inside it, and the ask, check, ask again loop the AI commands
//! share.

use anyhow::{Result, bail};

/// The reply without the one code fence a model may wrap it in despite being
/// asked not to. Only a fence around the whole reply is removed, and only
/// when its tag is one of `tags` (or empty), so an answer that merely starts
/// with a code block (```` ```rust ````) is left alone.
pub fn strip_fence<'a>(reply: &'a str, tags: &[&str]) -> &'a str {
    let t = reply.trim();
    let Some((tag, body)) = t.strip_prefix("```").and_then(|r| r.split_once('\n')) else {
        return t;
    };
    let tag = tag.trim();
    if !tag.is_empty() && !tags.iter().any(|x| x.eq_ignore_ascii_case(tag)) {
        return t;
    }
    body.trim_end().strip_suffix("```").map_or(t, str::trim)
}

/// The JSON object in a reply: the fence stripped, then everything from the
/// first `{` to the last `}` (models like to add a sentence around it).
pub fn json_object(reply: &str) -> &str {
    let t = strip_fence(reply, &["json"]);
    let start = t.find('{').unwrap_or(0);
    let end = t.rfind('}').map_or(t.len(), |i| i + 1);
    t[start..end.max(start)].trim()
}

/// Ask with `history`, check the answer with `validate`, and when it does not
/// pass ask once more with the model's answer and `fix(error)` appended.
/// Returns the first answer that passes; otherwise fails with
/// `"{what}: {last error}"`.
pub async fn chat_validated<T>(
    mut ask: impl AsyncFnMut(&[ailloy::Message]) -> Result<String>,
    mut history: Vec<ailloy::Message>,
    mut validate: impl FnMut(&str) -> Result<T, String>,
    fix: impl Fn(&str) -> String,
    what: &str,
) -> Result<T> {
    let mut last_err = String::new();
    for attempt in 0..2 {
        let reply = ask(&history).await?;
        match validate(&reply) {
            Ok(value) => return Ok(value),
            Err(e) => {
                if attempt == 0 {
                    history.push(ailloy::Message::assistant(&reply));
                    history.push(ailloy::Message::user(fix(&e)));
                }
                last_err = e;
            }
        }
    }
    bail!("{what}: {last_err}")
}

/// `chat_validated` against a real client.
pub async fn chat_client_validated<T>(
    client: &ailloy::Client,
    history: Vec<ailloy::Message>,
    validate: impl FnMut(&str) -> Result<T, String>,
    fix: impl Fn(&str) -> String,
    what: &str,
) -> Result<T> {
    use anyhow::Context;
    chat_validated(
        async |h: &[ailloy::Message]| {
            Ok(client.chat(h).await.context("AI request failed")?.content)
        },
        history,
        validate,
        fix,
        what,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_a_tagged_or_bare_fence() {
        assert_eq!(
            strip_fence("```yaml\nname: x\n```", &["yaml", "yml"]),
            "name: x"
        );
        assert_eq!(strip_fence("  ```\nplain\n```  ", &["yaml"]), "plain");
        assert_eq!(strip_fence("```JSON\n{}\n```", &["json"]), "{}");
    }

    #[test]
    fn leaves_other_fences_and_open_fences_alone() {
        let rust = "```rust\nfn main() {}\n```";
        assert_eq!(strip_fence(rust, &["markdown", "md"]), rust);
        // a deck that starts with a code block and goes on after it
        let deck = "```\ncode\n```\n\n# Next";
        assert_eq!(strip_fence(deck, &["markdown"]), deck);
        assert_eq!(strip_fence("name: x", &["yaml"]), "name: x");
    }

    #[test]
    fn json_object_ignores_fence_and_prose() {
        assert_eq!(
            json_object("```json\n{\"cast\": []}\n```"),
            "{\"cast\": []}"
        );
        assert_eq!(json_object("Here you go: {\"a\":1} Enjoy!"), "{\"a\":1}");
        assert_eq!(json_object("  {\"a\":1}  "), "{\"a\":1}");
    }

    fn run<T>(f: impl Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[test]
    fn retries_once_with_the_error() {
        let mut replies = vec!["bad", "good"].into_iter();
        let mut seen = Vec::new();
        let got = run(chat_validated(
            async |h: &[ailloy::Message]| {
                seen.push(h.len());
                Ok(replies.next().unwrap().to_string())
            },
            vec![ailloy::Message::user("q")],
            |r| {
                if r == "good" {
                    Ok(r.len())
                } else {
                    Err("nope".into())
                }
            },
            |e| format!("fix: {e}"),
            "invalid",
        ));
        assert_eq!(got.unwrap(), 4);
        // second ask carries the bad answer and the fix request
        assert_eq!(seen, vec![1, 3]);
    }

    #[test]
    fn gives_up_after_two_answers() {
        let mut asked = 0;
        let got: Result<()> = run(chat_validated(
            async |_: &[ailloy::Message]| {
                asked += 1;
                Ok(String::from("bad"))
            },
            vec![],
            |_| Err("still wrong".into()),
            |e| e.to_string(),
            "the model did not produce a valid theme",
        ));
        assert_eq!(asked, 2);
        assert_eq!(
            got.unwrap_err().to_string(),
            "the model did not produce a valid theme: still wrong"
        );
    }
}
