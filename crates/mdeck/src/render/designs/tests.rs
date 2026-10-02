//! The renderer against its promises: every design lays out in both sets,
//! nothing is dropped, measurement is the drawing's own layout, and slides
//! fit before they scroll.

use eframe::egui::{self, Pos2, Rect};

use super::copy::{Kind, Mark};
use super::*;
use crate::parser::{Design, parse};
use crate::render::test_support::with_ui;
use crate::theme::arrangement::Arrangements;

fn rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0))
}

fn themed(set: &str) -> Theme {
    let mut t = Theme::dark();
    t.arrangements = Arrangements::resolve(set, None).unwrap();
    t
}

/// One slide per design, in catalogue order.
const DECK: &str = "# Title\n\nSub\n\n---\n\n## Section\n\n### kicker\n\n---\n\n## Statement\n\nOne idea.\n\n---\n\n## Points\n\n- a\n- b\n\n---\n\n## Split\n\n![x](x.png)\n\n- a\n\n---\n\n## Media\n\n![x](x.png)\n\nCaption\n\n---\n\n## Gallery\n\n![a](a.png)\n\n![b](b.png)\n\n---\n\n## Quote\n\n> q\n\n-- who\n\n---\n\n## Code\n\n```rust\nfn main() {}\n```\n\n---\n\n## Visual\n\n```@pie\n- A: 1\n```\n\n---\n\n## Columns\n\nl\n\n+++\n\nr\n\n---\n\n## Table\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n---\n\n## Content\n\n- a\n\n```rust\nx\n```\n";

#[test]
fn every_design_lays_out_in_both_sets_and_fits() {
    let pres = parse(DECK);
    let designs: Vec<Design> = pres.slides.iter().map(|s| s.design).collect();
    assert_eq!(designs, Design::ALL, "the sample deck covers the catalogue");
    with_ui(|ui| {
        for set in ["standard", "editorial"] {
            let theme = themed(set);
            for slide in &pres.slides {
                let deck = SlideContext::default();
                let (_, plan) = layout(ui, slide, &theme, rect(), 1.0, &deck);
                assert!(
                    !plan.pieces.is_empty() || !plan.plate.is_empty(),
                    "{set} {:?}: nothing laid out",
                    slide.design
                );
                assert!(
                    plan.content <= plan.available + 0.5,
                    "{set} {:?}: {} > {}",
                    slide.design,
                    plan.content,
                    plan.available
                );
                // everything inside the slide
                for p in &plan.pieces {
                    assert!(
                        rect().expand(1.0).contains_rect(p.bounds),
                        "{set} {:?}: piece outside the slide {:?}",
                        slide.design,
                        p.bounds
                    );
                }
            }
        }
    });
}

#[test]
fn a_chosen_design_shows_what_its_roles_do_not_take() {
    // DES-04: a quote design on a slide with a list and code shows them all
    let pres = parse("## Q\n<!-- design: quote -->\n\n> wise\n\n- one\n- two\n\n```rust\nx\n```\n");
    with_ui(|ui| {
        let (_, plan) = layout(
            ui,
            &pres.slides[0],
            &themed("editorial"),
            rect(),
            1.0,
            &SlideContext::default(),
        );
        // title, quote, two items, code
        assert_eq!(plan.pieces.len(), 1 + 1 + 1 + 2 + 1, "eyebrow included");
        assert!(
            plan.pieces
                .iter()
                .any(|p| matches!(p.kind, Kind::Block { .. }))
        );
    });
}

#[test]
fn editorial_lists_number_and_nest() {
    // D23: ordered lists show numbers and every nesting level is drawn
    let pres = parse("## L\n\n1. one\n   - inner\n     - deeper\n2. two\n");
    with_ui(|ui| {
        let (_, plan) = layout(
            ui,
            &pres.slides[0],
            &themed("editorial"),
            rect(),
            1.0,
            &SlideContext::default(),
        );
        let marks: Vec<(f32, Option<String>)> = plan
            .pieces
            .iter()
            .filter_map(|p| match &p.kind {
                Kind::Text {
                    anchor,
                    mark: Some(m),
                    ..
                } => Some((
                    anchor.x,
                    match m {
                        Mark::Glyph(g, _) => Some(g.job.text.clone()),
                        _ => None,
                    },
                )),
                _ => None,
            })
            .collect();
        assert_eq!(marks.len(), 4, "{marks:?}");
        assert_eq!(marks[0].1.as_deref(), Some("1."));
        assert_eq!(marks[3].1.as_deref(), Some("2."));
        assert!(
            marks[1].0 > marks[0].0 && marks[2].0 > marks[1].0,
            "{marks:?}"
        );
    });
}

#[test]
fn prose_shrinks_to_its_floor_before_scrolling() {
    with_ui(|ui| {
        let theme = themed("standard");
        let deck = SlideContext::default();
        // a list a little too long fits once prose shrinks
        let items: String = (0..9)
            .map(|i| format!("- Item {i} with a few words\n"))
            .collect();
        let pres = parse(&format!("## Fits\n\n{items}"));
        let (fitted, plan) = layout(ui, &pres.slides[0], &theme, rect(), 1.0, &deck);
        assert!(plan.content <= plan.available + 0.5);
        if fitted.body_size < theme.body_size {
            assert!(fitted.body_size >= theme.body_size * PROSE_FIT_FLOOR - 0.01);
        }
        // far too long: stops at the floor and scrolls
        let items: String = (0..40).map(|i| format!("- Item {i}\n")).collect();
        let pres = parse(&format!("## Scrolls\n\n{items}"));
        let (fitted, plan) = layout(ui, &pres.slides[0], &theme, rect(), 1.0, &deck);
        assert!((fitted.body_size - theme.body_size * PROSE_FIT_FLOOR).abs() < 0.01);
        assert!(plan.content > plan.available);
        // measuring is the drawing's own layout
        let (content, available) = measure(ui, &pres.slides[0], &theme, rect(), 1.0, &deck);
        assert_eq!((content, available), (plan.content, plan.available));
    });
}

#[test]
fn long_code_shrinks_before_prose() {
    with_ui(|ui| {
        let theme = themed("standard");
        let code: String = (0..30)
            .map(|i| format!("let value_{i} = compute({i});\n"))
            .collect();
        let pres = parse(&format!("## Code\n\n```rust\n{code}```\n"));
        let (fitted, plan) = layout(
            ui,
            &pres.slides[0],
            &theme,
            rect(),
            1.0,
            &SlideContext::default(),
        );
        assert!(fitted.code_size < theme.code_size);
        assert!(fitted.code_size >= theme.code_size * CODE_FIT_FLOOR);
        assert_eq!(fitted.body_size, theme.body_size, "prose untouched");
        assert!(plan.content <= plan.available + 0.5);
    });
}

#[test]
fn a_gallery_that_fits_does_not_scroll() {
    // D9: galleries were measured as stacked 400 px images
    let images: String = (0..6).map(|i| format!("![i{i}](i{i}.png)\n\n")).collect();
    let pres = parse(&format!("## Many\n\n{images}"));
    assert_eq!(pres.slides[0].design, Design::Gallery);
    with_ui(|ui| {
        for set in ["standard", "editorial"] {
            let (c, a) = measure(
                ui,
                &pres.slides[0],
                &themed(set),
                rect(),
                1.0,
                &SlideContext::default(),
            );
            assert!(c <= a + 0.5, "{set}: {c} > {a}");
        }
    });
}

#[test]
fn the_reveal_scroll_uses_the_design_layout() {
    // D10: the item revealed by a step is found where the design puts it
    let pres = parse("## Steps\n\n+ one\n+ two\n+ three\n");
    with_ui(|ui| {
        let theme = themed("editorial");
        let deck = SlideContext::default();
        let s = &pres.slides[0];
        let b1 = revealed_bottom(ui, s, &theme, rect(), 1.0, &deck, 1).unwrap();
        let b3 = revealed_bottom(ui, s, &theme, rect(), 1.0, &deck, 3).unwrap();
        assert!(b3 > b1);
        assert_eq!(revealed_bottom(ui, s, &theme, rect(), 1.0, &deck, 0), None);
    });
}

#[test]
fn a_side_image_stays_in_its_panel() {
    // D14: a split's image region never reaches into the copy column
    let pres = parse("## S\n\n![x @fill](x.png)\n\n- a\n- b\n");
    with_ui(|ui| {
        for set in ["standard", "editorial"] {
            let (_, plan) = layout(
                ui,
                &pres.slides[0],
                &themed(set),
                rect(),
                1.0,
                &SlideContext::default(),
            );
            let copy = plan.copy.unwrap();
            assert!(plan.plate[0].rect.left() > copy.right(), "{set}");
        }
    });
}

#[test]
fn stage_follows_the_arrangement() {
    let pres = parse("## P\n\n- a\n\n---\n\n## C\n\n- a\n\n```rust\nx\n```\n");
    let (e, s) = (themed("editorial"), themed("standard"));
    assert!(crate::render::design_has_stage(&pres.slides[0], &e));
    assert!(!crate::render::design_has_stage(&pres.slides[0], &s));
    // a content slide with code gives its stage up
    assert!(!crate::render::design_has_stage(&pres.slides[1], &e));
}

/// The text of every text piece, where it is painted, and whether it has a
/// list mark.
fn texts(plan: &Plan) -> Vec<(String, Pos2, bool)> {
    plan.pieces
        .iter()
        .filter_map(|p| match &p.kind {
            Kind::Text {
                galley,
                anchor,
                mark,
                ..
            } => Some((galley.job.text.clone(), *anchor, mark.is_some())),
            _ => None,
        })
        .collect()
}

#[test]
fn quotes_keep_nested_quotes_and_lists_in_every_design() {
    // MD-12: a nested quote is an indented quote with a bar of its own, a
    // list in a quote is a list, and the attribution rule still holds
    let quote =
        "> Outer words\n>\n> > Inner words\n>\n> - first point\n> - second point\n>\n> -- Ada";
    with_ui(|ui| {
        for set in ["standard", "editorial"] {
            let theme = themed(set);
            for design in Design::ALL {
                let md = format!(
                    "## Heading\n<!-- design: {} -->\n\n{quote}\n",
                    design.name()
                );
                let pres = parse(&md);
                let (_, plan) = layout(
                    ui,
                    &pres.slides[0],
                    &theme,
                    rect(),
                    1.0,
                    &SlideContext::default(),
                );
                let t = texts(&plan);
                let find = |needle: &str| {
                    t.iter()
                        .find(|(s, ..)| s.to_lowercase().contains(&needle.to_lowercase()))
                        .unwrap_or_else(|| panic!("{set} {design:?}: no `{needle}` in {t:?}"))
                        .clone()
                };
                let outer = find("Outer words");
                let inner = find("Inner words");
                assert!(
                    !outer.0.contains("Inner") && !outer.0.contains("first point"),
                    "{set} {design:?}: flattened {:?}",
                    outer.0
                );
                assert!(
                    inner.1.x > outer.1.x + 20.0,
                    "{set} {design:?}: the nested quote is not indented"
                );
                let bars: Vec<Rect> = plan
                    .pieces
                    .iter()
                    .filter_map(|p| match p.kind {
                        Kind::Bar { rect, .. } => Some(rect),
                        _ => None,
                    })
                    .collect();
                assert!(
                    bars.iter().any(|b| b.center().x > outer.1.x
                        && b.center().x < inner.1.x
                        && b.top() <= inner.1.y + 8.0),
                    "{set} {design:?}: the nested quote has no bar of its own: {bars:?}"
                );
                let first = find("first point");
                let second = find("second point");
                assert!(
                    first.2 && second.2,
                    "{set} {design:?}: list items lost their marks"
                );
                assert!(second.1.y > first.1.y);
                let ada = find("Ada");
                assert!(
                    !ada.0.contains("Outer") && ada.1.y > second.1.y,
                    "{set} {design:?}: the attribution ran into the quote"
                );
            }
        }
    });
}

#[test]
fn a_plain_quote_keeps_its_single_passage() {
    // structure only where there is some: paragraphs still read as one run
    let long = "two, a closing paragraph long enough that it can never be read as an attribution of the quote above";
    let pres = parse(&format!("## Q\n\n> one\n>\n> {long}\n"));
    with_ui(|ui| {
        let (_, plan) = layout(
            ui,
            &pres.slides[0],
            &themed("standard"),
            rect(),
            1.0,
            &SlideContext::default(),
        );
        let t = texts(&plan);
        assert!(t.iter().any(|(s, ..)| s.contains("one\ntwo,")), "{t:?}");
        assert!(
            !plan
                .pieces
                .iter()
                .any(|p| matches!(p.kind, Kind::Bar { .. }))
        );
    });
}
