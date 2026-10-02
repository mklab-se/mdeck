//! `mdeck point-cloud contribute`: offer a point cloud as a built-in
//! through a prefilled GitHub issue.

use std::path::Path;

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::render::illustration::{self, Cloud, Source};

/// Where contributions go.
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
/// Longest issue link a browser and GitHub reliably accept.
const MAX_URL_LEN: usize = 8000;

/// Offer a deck, user or pack point cloud as a built-in: write a `.json` copy
/// GitHub will accept as an attachment, and open a new-issue page with the
/// title and body filled in. The person drags the file onto the issue and
/// submits; nothing needs to be installed.
pub(super) fn contribute(name: &str, no_open: bool, quiet: bool) -> Result<()> {
    let Some((src, cloud)) = illustration::resolve(name, Some(Path::new(".")))? else {
        bail!("no point cloud named `{name}` (run `mdeck point-cloud list`)");
    };
    let path = match &src {
        Source::Deck(p) | Source::User(p) | Source::Pack(p) => p.clone(),
        Source::Builtin => bail!("`{name}` is already a built-in point cloud"),
    };
    // GitHub attaches .json, not .mdpc
    let attachment = path.with_extension("mdpc.json");
    std::fs::copy(&path, &attachment)
        .with_context(|| format!("writing {}", attachment.display()))?;

    let title = format!("Point cloud: {name}");
    let body = issue_body(
        &cloud,
        attachment
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("the file"),
    );
    let url = issue_url(REPOSITORY, &title, &body);

    if !quiet {
        println!(
            "{} {} is ready to attach ({} points, aspect {:.2})",
            "✓".green().bold(),
            attachment.display(),
            cloud.points.len(),
            cloud.aspect
        );
        println!();
        println!("{}", braille(&cloud, 48, 24));
        println!();
        println!("Drag that file onto the issue text and submit:");
        println!("  {}", url.cyan());
    }
    if !no_open && open_in_browser(&url).is_err() && !quiet {
        println!("  (could not open a browser; paste the link yourself)");
    }
    Ok(())
}

/// The issue text: what the cloud is, how it was made, a braille sketch and
/// what to do next.
pub fn issue_body(cloud: &Cloud, attachment: &str) -> String {
    let mut b = String::new();
    b.push_str(&format!("**Name:** `{}`\n", cloud.name));
    b.push_str(&format!("**Description:** {}\n", cloud.description));
    b.push_str(&format!(
        "**Points:** {}, aspect {:.2}\n",
        cloud.points.len(),
        cloud.aspect
    ));
    if let Some(p) = &cloud.prompt {
        b.push_str(&format!("**Prompt:** {p}\n"));
    }
    // small: every braille cell costs nine bytes in the link
    b.push_str("\n```\n");
    b.push_str(&braille(cloud, 32, 12));
    b.push_str("\n```\n\n");
    b.push_str(&format!(
        "**Attach `{attachment}`** by dragging it onto this text box, then submit.\n\n"
    ));
    b.push_str(
        "I made this point cloud and contribute it under the project's license, for \
         inclusion in MDeck's built-in set.\n",
    );
    b
}

/// `<repository>/issues/new?...` with the title and body filled in. The body
/// is trimmed if the link would grow too long for a browser.
pub fn issue_url(repository: &str, title: &str, body: &str) -> String {
    let base = format!(
        "{}/issues/new?labels=illustration&title={}&body=",
        repository.trim_end_matches('/'),
        percent_encode(title)
    );
    let mut body = body.to_string();
    let mut url = format!("{base}{}", percent_encode(&body));
    while url.len() > MAX_URL_LEN && body.len() > 200 {
        let cut = body.len() - 200;
        let cut = body
            .char_indices()
            .map(|(i, _)| i)
            .take_while(|&i| i <= cut)
            .last()
            .unwrap_or(0);
        body.truncate(cut);
        url = format!("{base}{}", percent_encode(&body));
    }
    url
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The cloud as braille dots, at most `cols` characters wide and `rows`
/// tall, keeping the cloud's aspect (a braille cell is two dots wide and
/// four tall, and a terminal cell is about twice as tall as it is wide).
pub fn braille(cloud: &Cloud, cols: usize, rows: usize) -> String {
    let (cols, rows) = (cols.max(1), rows.max(1));
    // dot grid that keeps the aspect within the box
    let mut w = (cols * 2) as f32;
    let mut h = w * cloud.aspect;
    if h > (rows * 4) as f32 {
        h = (rows * 4) as f32;
        w = h / cloud.aspect.max(0.01);
    }
    let (w, h) = ((w as usize).max(2), (h as usize).max(4));
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(4));
    let mut cells = vec![0u8; cw * ch];
    for p in cloud.points.iter() {
        let x = ((p[0] * (w - 1) as f32).round() as usize).min(w - 1);
        let y = ((p[1] * (h - 1) as f32).round() as usize).min(h - 1);
        let (cx, cy) = (x / 2, y / 4);
        let bit = match (x % 2, y % 4) {
            (0, 0) => 0x01,
            (0, 1) => 0x02,
            (0, 2) => 0x04,
            (1, 0) => 0x08,
            (1, 1) => 0x10,
            (1, 2) => 0x20,
            (0, _) => 0x40,
            _ => 0x80,
        };
        cells[cy * cw + cx] |= bit;
    }
    cells
        .chunks(cw)
        .map(|row| {
            row.iter()
                .map(|&bits| char::from_u32(0x2800 + bits as u32).unwrap_or(' '))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn open_in_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .and_then(|s| {
            if s.success() {
                Ok(())
            } else {
                Err(std::io::Error::other("browser did not open"))
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn ring_cloud(aspect: f32) -> Cloud {
        Cloud {
            version: illustration::VERSION,
            name: "ring".into(),
            description: "A ring".into(),
            prompt: Some("A ring. Sparse particles.".into()),
            generated: None,
            aspect,
            points: Arc::new(
                (0..200)
                    .map(|i| {
                        let a = i as f32 / 200.0 * std::f32::consts::TAU;
                        [0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()]
                    })
                    .collect(),
            ),
        }
    }

    #[test]
    fn braille_keeps_the_aspect_and_draws_only_the_ring() {
        let art = braille(&ring_cloud(1.0), 40, 20);
        let lines: Vec<&str> = art.lines().collect();
        // 40 cols x 2 = 80 dots wide, square -> 80 dots tall = 20 rows
        assert_eq!(lines.len(), 20);
        assert!(lines.iter().all(|l| l.chars().count() == 40));
        let blank = char::from_u32(0x2800).unwrap();
        // the centre of the ring is empty, the edge is not
        assert_eq!(lines[10].chars().nth(20), Some(blank));
        assert_ne!(lines[0].chars().nth(20), Some(blank));
        // a tall cloud is capped by the row count instead
        let tall = braille(&ring_cloud(3.0), 40, 20);
        let lines: Vec<&str> = tall.lines().collect();
        assert_eq!(lines.len(), 20);
        assert!(lines[0].chars().count() < 40);
    }

    #[test]
    fn issue_body_and_link_carry_the_facts_and_stay_short() {
        let cloud = ring_cloud(1.0);
        let body = issue_body(&cloud, "ring.mdpc.json");
        assert!(body.contains("`ring`"));
        assert!(body.contains("A ring"));
        assert!(body.contains("Sparse particles"));
        assert!(body.contains("ring.mdpc.json"));
        assert!(body.contains("license"));
        let url = issue_url(
            "https://github.com/mklab-se/mdeck",
            "Point cloud: ring",
            &body,
        );
        assert!(url.starts_with("https://github.com/mklab-se/mdeck/issues/new?labels=illustration&title=Point%20cloud%3A%20ring&body="));
        assert!(url.len() <= MAX_URL_LEN);
        // a huge prompt is trimmed rather than producing an unusable link
        let mut big = cloud.clone();
        big.prompt = Some("x".repeat(20_000));
        let url = issue_url(
            "https://github.com/mklab-se/mdeck",
            "t",
            &issue_body(&big, "f"),
        );
        assert!(url.len() <= MAX_URL_LEN);
        assert_eq!(percent_encode("a b/c"), "a%20b%2Fc");
    }
}
