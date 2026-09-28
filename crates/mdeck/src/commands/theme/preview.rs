//! `mdeck theme preview`: export a sampler deck in a theme.

use std::path::PathBuf;

use anyhow::Result;

use super::here;

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

pub(super) fn preview(name: &str, output_dir: PathBuf, width: u32, height: u32) -> Result<()> {
    let built = here().load(name).map_err(|e| anyhow::anyhow!("{e}"))?;
    for w in &built.warnings {
        eprintln!("warning: theme: {w}");
    }
    let dir = std::env::temp_dir().join(format!("mdeck-theme-preview-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let deck = dir.join("theme-preview.md");
    std::fs::write(&deck, SAMPLER)?;
    let result = crate::commands::export::run(crate::commands::export::ExportArgs {
        file: deck,
        output_dir,
        width,
        height,
        debug: false,
        slide: None,
        range: None,
        format: crate::commands::export::Format::Png,
        notes: false,
        theme: crate::commands::export::ThemeChoice::Given(Box::new(built.theme)),
        engine: None,
    });
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sampler_deck_parses_into_its_slides() {
        let p = crate::parser::parse(SAMPLER);
        assert_eq!(p.slides.len(), 8);
    }
}
