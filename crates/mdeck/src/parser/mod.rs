pub mod blocks;
pub mod design;
pub mod frontmatter;
pub mod inline;
mod layout;
mod math;
mod model;
mod notes;
mod references;
mod settings;
pub mod splitter;
pub mod steps;
mod text;

pub use design::{Design, Recognition};
pub use layout::Layout;
pub use model::{
    Alert, Align, Block, Chart, ImageDirectives, Inline, ListItem, ListMarker, Presentation,
    PresentationMeta, Problem, ProblemKind, Setting, Slide,
};
pub use settings::{parse_setting_line, setting};
pub use text::{for_each_inlines, inlines_to_text};

use notes::extract_notes;

pub fn parse(content: &str) -> Presentation {
    let (meta, body, first_line) = frontmatter::extract(content);
    let (body, definitions) = references::collect(&body);
    let raws = splitter::split(&body, meta.slide_level);
    let located = splitter::locate(&body, &raws);
    let reveal = meta.reveal.unwrap_or(true);
    let slides = references::with(definitions, || {
        raws.into_iter()
            .zip(located)
            .filter(|(raw, _)| !raw.trim().is_empty())
            .map(|(raw, lines)| {
                let lines = lines.into_iter().map(|l| l + first_line).collect();
                parse_slide(raw, lines, reveal)
            })
            .collect::<Vec<_>>()
    });
    let mut slides = slides;
    for (i, slide) in slides.iter_mut().enumerate() {
        assign_design(slide, i == 0);
    }
    Presentation { meta, slides }
}

/// Image options that were not understood, each on the line that holds it.
fn image_problems(
    blocks: &[Block],
    raw: &str,
    file_line: &dyn Fn(usize) -> usize,
    problems: &mut Vec<Problem>,
) {
    for b in blocks {
        if let Block::Image {
            path, directives, ..
        } = b
        {
            let offset = raw
                .lines()
                .position(|l| l.contains("![") && l.contains(path.as_str()))
                .unwrap_or(0);
            for message in &directives.problems {
                problems.push(Problem {
                    kind: ProblemKind::Content,
                    line: file_line(offset),
                    message: message.clone(),
                });
            }
        }
    }
}

/// Parse one raw slide from the splitter: notes, settings, blocks, steps,
/// layout. `source_lines` holds the deck file line of each line of `raw`;
/// `deck_reveal` is the deck's `reveal` (a slide's own overrides it).
fn parse_slide(raw: String, source_lines: Vec<usize>, deck_reveal: bool) -> Slide {
    let line = source_lines.first().copied().unwrap_or(0);
    let file_line = |offset: usize| source_lines.get(offset).copied().unwrap_or(line + offset);
    let (content, notes) = extract_notes(&raw);
    let (mut settings, mut problems, content) = settings::extract_settings(&content);
    for s in &mut settings {
        s.line = file_line(s.line);
    }
    for p in &mut problems {
        p.line = file_line(p.line);
    }
    let mut blocks = blocks::parse(&content);
    let (footnotes, missing) = references::take_footnotes();
    for id in missing {
        problems.push(Problem {
            kind: ProblemKind::Content,
            line,
            message: format!("footnote [^{id}] has no definition (`[^{id}]: text`)"),
        });
    }
    image_problems(&blocks, &raw, &file_line, &mut problems);
    let notes = match (notes, footnotes) {
        (Some(n), Some(f)) => Some(format!("{n}\n\n{f}")),
        (n, f) => n.or(f),
    };
    // A slide's thermal-window is the common window of its thermal images.
    if let Some(window) = setting(&settings, "thermal-window") {
        for b in &mut blocks {
            if let Block::Chart {
                kind: Chart::Thermal,
                content,
                ..
            } = b
            {
                content.push_str(&format!("\nslide-window: {window}\n"));
            }
        }
    }
    let reveal = match setting(&settings, "reveal") {
        Some("none") => false,
        Some("steps") => true,
        _ => deck_reveal,
    };
    let steps = steps::number(&mut blocks, reveal, &steps::default_visual_steps);
    // `picture` names the fallback (a point cloud or an image) behind a
    // generated artwork (D13); `picture: none` opts out of both.
    let picture = setting(&settings, "picture");
    let illustration = picture.filter(|p| *p != "none").map(|p| p.to_lowercase());
    let art = match picture {
        Some("none") => Some("none".to_string()),
        _ => setting(&settings, "picture-prompt").map(str::to_string),
    };
    let logo = setting(&settings, "logo").map(str::to_string);
    Slide {
        settings,
        blocks,
        raw_source: raw,
        line,
        source_lines,
        notes,
        reveal,
        steps,
        illustration,
        logo,
        art,
        problems,
        ..Default::default()
    }
}

/// Give `slide` its design (the `design:` setting or the recognition
/// table) and the layout view engines key on. `first`: the deck's first
/// slide.
pub fn assign_design(slide: &mut Slide, first: bool) {
    let recognition = design::recognise(setting(&slide.settings, "design"), &slide.blocks, first);
    slide.layout = Layout::of(recognition.design, &slide.blocks);
    slide.design = recognition.design;
    slide.recognition = recognition;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picture_names_a_cloud() {
        let p = parse("# Title\n<!-- picture: Server-Rack -->\n\n- one\n\n---\n\n# Plain\n");
        assert_eq!(p.slides[0].illustration.as_deref(), Some("server-rack"));
        assert!(p.slides[1].illustration.is_none());
        let p = parse("# T\n<!-- picture: none -->\n");
        assert!(p.slides[0].illustration.is_none());
        assert_eq!(p.slides[0].art.as_deref(), Some("none"));
        let p = parse("# T\n<!-- picture-prompt: a lighthouse -->\n");
        assert_eq!(p.slides[0].art.as_deref(), Some("a lighthouse"));
    }

    #[test]
    fn settings_apply_to_the_slide_they_are_in() {
        // MD-07: under the heading, or anywhere in the slide; the last wins.
        let md = "# First\n\n- one\n\n# Second\n<!-- picture: account -->\n\n- two\n";
        let pres = parse(md);
        assert_eq!(pres.slides.len(), 2);
        let s = &pres.slides[1];
        assert_eq!(s.illustration.as_deref(), Some("account"));
        assert!(matches!(s.layout, Layout::Bullet), "{:?}", s.layout);
        assert_eq!(
            s.blocks.len(),
            2,
            "the comment is not a block: {:?}",
            s.blocks
        );

        let md = "# A\n\nIntro\n\n<!-- design: columns -->\n\nLeft\n\n+++\n\nRight\n<!-- logo: none -->\n<!-- design: content -->\n";
        let s = &parse(md).slides[0];
        assert!(matches!(s.layout, Layout::Content), "{:?}", s.layout);
        assert_eq!(s.logo.as_deref(), Some("none"));

        // MD-08: a setting above the next heading stays on its own slide.
        let pres = parse("# First\n\n- one\n\n<!-- picture: account -->\n# Second\n\n- two\n");
        assert_eq!(pres.slides[0].illustration.as_deref(), Some("account"));
        assert_eq!(pres.slides[1].illustration, None);
    }

    #[test]
    fn v1_directives_are_plain_text() {
        // LANG-10: `@key: value` lines are no longer settings.
        let md = "# A\n@layout: quote\n\n- one\n";
        let s = &parse(md).slides[0];
        assert!(s.settings.is_empty(), "{:?}", s.settings);
        assert!(matches!(s.layout, Layout::Bullet) || matches!(s.layout, Layout::Content));
        assert!(
            s.blocks
                .iter()
                .any(|b| matches!(b, Block::Paragraph { inlines }
            if inlines_to_text(inlines) == "@layout: quote"))
        );
    }

    #[test]
    fn test_poker_night_parses() {
        let content = include_str!("../../../../samples/poker-night.md");
        let pres = parse(content);
        assert_eq!(pres.meta.theme.as_deref(), Some("dark"));
        assert_eq!(pres.meta.transition.as_deref(), Some("slide"));
        assert!(
            pres.slides.len() >= 14,
            "Expected at least 14 slides, got {}",
            pres.slides.len()
        );
        assert!(matches!(pres.slides[0].layout, Layout::Title));
    }

    #[test]
    fn test_saloon_workshop_parses() {
        let content = include_str!("../../../../samples/saloon-workshop.md");
        let pres = parse(content);
        assert_eq!(pres.meta.theme.as_deref(), Some("light"));
        assert_eq!(pres.meta.transition.as_deref(), Some("fade"));
        assert_eq!(
            pres.meta.footer.as_deref(),
            Some("mdeck sample presentation")
        );
        assert!(
            pres.slides.len() >= 16,
            "Expected at least 16 slides, got {}",
            pres.slides.len()
        );
        assert!(matches!(pres.slides[0].layout, Layout::Title));
    }

    #[test]
    fn test_slide_separator_dashes() {
        let pres = parse("# Slide One\n\n---\n\n# Slide Two");
        assert_eq!(pres.slides.len(), 2);
        let pres = parse("# First\n\nSome content\n\n# Second\n\nMore content");
        assert_eq!(pres.slides.len(), 2);
    }

    #[test]
    fn test_crlf_document_end_to_end() {
        let content = "---\r\ntitle: \"CRLF\"\r\ntheme: dark\r\n---\r\n\r\n# One\r\n\r\n- a\r\n- b\r\n\r\n---\r\n\r\n# Two\r\n\r\nText\r\n";
        let pres = parse(content);
        assert_eq!(pres.meta.title.as_deref(), Some("CRLF"));
        assert_eq!(pres.meta.theme.as_deref(), Some("dark"));
        assert_eq!(pres.slides.len(), 2, "{:?}", pres.slides);
        assert!(matches!(pres.slides[0].layout, Layout::Bullet));
        assert!(matches!(pres.slides[1].layout, Layout::Title));
    }

    #[test]
    fn blank_setting_values_count_as_unset() {
        let s = &parse("# A\n<!--\nlogo:   brand.svg\npicture-prompt:\n-->\n").slides[0];
        assert_eq!(setting(&s.settings, "logo"), Some("brand.svg"));
        assert_eq!(setting(&s.settings, "picture-prompt"), None);
        assert_eq!(s.art, None);
    }

    #[test]
    fn slides_and_settings_know_their_file_lines() {
        let md = "---\ntitle: T\n---\n\n# One\n\n- a\n\n<!-- picture: cup -->\n\n# Two\n<!-- logo: none -->\n\nx\n\n---\n\n<!-- design: code -->\n\n# Three\n";
        let p = parse(md);
        let lines: Vec<usize> = p.slides.iter().map(|s| s.line).collect();
        assert_eq!(lines, [5, 11, 18]);
        assert_eq!(p.slides[0].setting_line("picture"), 9);
        let two = &p.slides[1];
        assert_eq!(two.setting_line("logo"), 12);
        assert_eq!(two.setting_line("design"), 11, "none: the slide's line");
        assert_eq!(two.line_at(1), 12);
        assert_eq!(p.slides[2].setting_line("design"), 18);
        // Without frontmatter the first line is line 1.
        assert_eq!(parse("# A\n\n# B\n").slides[1].line, 3);
    }

    #[test]
    fn steps_and_reveal_none() {
        let p = parse("# A\n\n+ a\n  - a1\n+ b\n\n# B\n<!-- reveal: none -->\n\n+ c\n");
        assert_eq!(p.slides[0].steps, 2);
        assert_eq!(p.slides[1].steps, 0);
        let p = parse("---\nreveal: none\n---\n# A\n\n+ a\n\n# B\n<!-- reveal: steps -->\n\n+ b\n");
        assert_eq!(p.slides[0].steps, 0);
        assert_eq!(p.slides[1].steps, 1);
    }

    #[test]
    fn reference_links_and_footnotes_resolve_across_the_deck() {
        let md = "# A\n\nSee [the docs][d].[^1]\n\n# B\n\nText\n\n[d]: https://d.dev\n[^1]: Source: the docs.\n";
        let p = parse(md);
        let Block::Paragraph { inlines } = &p.slides[0].blocks[1] else {
            panic!("{:?}", p.slides[0].blocks);
        };
        assert!(matches!(&inlines[1], Inline::Link { url, .. } if url == "https://d.dev"));
        assert_eq!(inlines_to_text(inlines), "See the docs.");
        assert_eq!(
            p.slides[0].notes.as_deref(),
            Some("[^1]: Source: the docs.")
        );
        assert_eq!(p.slides[1].blocks.len(), 2, "definitions are hidden");
        let missing = parse("# A\n\nText[^x]\n");
        assert_eq!(missing.slides[0].problems.len(), 1);
    }
}
