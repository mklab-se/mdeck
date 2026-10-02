# Decisions

Decisions the owner made on the questions raised while specifying mdeck 2 (October 2026). Each
row names the requirement that records it. New questions are added at the end, marked **open**
until they are decided.

| # | Question | Decision | Where |
|---|---|---|---|
| Q1 | Which theme is the default for a plain markdown file? | A plain dark theme with a bright foreground, the `standard` design set, no engine animation and fade transitions. It shows that mdeck is simple; the wow effect is one setting away. | VIS-07, THM-15, RUN-08 |
| Q2 | Keep `---` as an optional explicit slide break? | Yes; headings never require it. | MD-04 |
| Q3 | Where do notes end? | At the slide boundary only, never at a heading, because notes are markdown and may contain headings. Notes move into a ```` ```@notes ```` fenced block, which makes the boundary explicit; `???` is removed. | MD-14, MD-15, LANG-15 |
| Q4 | Plain frontmatter keys, or nested under `mdeck:`? | Plain keys. | LANG-06 |
| Q5 | Slide settings with an explicit `mdeck` marker? | No marker; validation catches typos. | LANG-08 |
| Q6 | Normalise visual tag names? | Drop the `chart` suffix; one name per kind, no aliases. | VIZ-02, LANG-11 |
| Q7 | Keep `+++` for columns? | Yes. | LANG-14 |
| Q8 | Implicit theme parent? | The default theme (`dark`); one root. | THM-03 |
| Q9 | How much of a design's arrangement is themeable? | Start with regions, alignment, role styling, ornaments, entry motion and stage; extend on demand. | DES-11 |
| Q10 | Should the SDK expose `egui` directly or wrap drawing? | Wrap it. Extensions draw through mdeck's own drawing interface, so no third-party type is in the stable SDK. The SDK is versioned in lockstep with mdeck (SDK 2.x supports mdeck 2.x), and nothing users build on breaks within a major version. Dependencies stay current unless an upgrade would break that promise. (Reverses an earlier "re-export egui" decision.) | EXT-12, EXT-23 to EXT-27 |
| Q11 | Ship external visual programs (tier D) in v2.0? | Yes. | EXT-18 |
| Q12 | Presenter view in v2.0? | Yes, with notes rendered as markdown. | RUN-03, MD-15 |
| Q13 | Video or animated export? | Later, not in v2.0. | RUN-18 |
| Q14 | Stories? | Removed for now. | PIC-06 |
| Q15 | Which of the 12 engines stay in the default build? | Ten engines. Laser (and its `etch` theme) is removed; blueprint and chalkboard merge into one `line` engine with a surface setting; the other nine stay. Decided after the side-by-side engine review. | ENG-17a |
| Q16 | Placeholder syntax for generated images? | `![prompt](generate:)`, with the asset manifest mapping it to a file. | GEN-04 |
| Q17 | Add `statement` and `table` designs? | Yes to both. | DES-02 |
| Q18 | Migration from v1? | No migration tool. v2.0 breaks cleanly; the format reference is precise enough for an AI harness to convert a deck, the release notes map v1 to v2, and `--check` names the v2 form of any v1 construct it finds. | LANG-07 |
