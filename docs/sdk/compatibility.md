# Compatibility

Whoever builds on mdeck, a deck author, a theme designer or a company with a private engine, must
be able to rely on it for a whole major version.

## The promise

Within a major version, nothing that users build on breaks:

- the deck format;
- the theme, arrangement and pack formats;
- the CLI commands and flags;
- the stable SDK surface.

A deck, theme or extension made for 2.0 works unchanged on every 2.x release. Breaking changes are
collected and shipped only in the next major version, each listed with before and after code on
the [upgrading page](upgrading.md).

## Versions

The SDK is versioned in lockstep with mdeck: `mdeck-sdk` 2.x works with mdeck 2.x. Depend on the
major version and let Cargo pick the newest minor:

```toml
[dependencies]
mdeck-sdk = "2"
```

`mdeck build` compiles your extension against the SDK of the mdeck it builds, so a custom build
for mdeck 2.4 uses `mdeck-sdk` 2.4. Your crate needs no change for that. Rebuild your custom mdeck
for each mdeck release you want to use.

Minor versions add: new methods, new optional hooks with default implementations, new painter
operations, new hint kinds, new moments, new capabilities and new content fields. Write code that
tolerates additions:

- implement only the trait methods you need; new hooks come with defaults;
- match on SDK enums with a wildcard arm (`_ => {}`), and on their struct-like variants with `..`
  (`Moment::End { words, .. }`, `Block::List { items, .. }`);
- build SDK values with their constructors, never with a struct literal.

The compiler enforces the last two. Every SDK type that may grow is `#[non_exhaustive]`: the
content model (`Slide`, `Block`, `Inline`, `ListItem`, `CalloutKind`, ...), `Hint`, `Moment`,
`Look`, `PictureSource`, `Picture`, `Stage`, `Frame`, `Tokens`, `EngineDef`, `Capabilities`,
`Needs`, `SettingKind`, `SettingSpec`, `Medium`, `SideLook`, `Annotation`, `Problem`, `Cloud`,
`Mask`, `Font`, `FontRole` and the registry's errors. Outside the SDK you cannot write a struct
literal for them (or `..Default::default()` on them), and a `match` without a wildcard arm does not
compile. A field or variant added in 2.x therefore cannot break your crate.

How to make each value:

| Value | Make it with |
|---|---|
| an engine definition | `EngineDef::new(name, summary, create)`, then `.with_capabilities(..)`, `.with_settings(..)`, `.with_needs(..)`, `.with_ending_caption_delay(..)`, `.with_board(..)`; all `const`, so it can be a `static` |
| capabilities, needs | `Capabilities::NONE.with_picture().with_countdown()`, `Needs::NONE.with_page()` |
| a setting | `SettingSpec::new(key, kind, summary)` |
| an art medium | `Medium::new(name, kind, strategy)` |
| a side of a transition | `SideLook::SHOWN.with_offset(v).with_opacity(o).with_scale(s)` |
| a stage, a frame | `Stage::new(moment)`, `Frame::new(rect, &tokens, &settings)`, then set fields |
| a moment | `Moment::Slide`, `Moment::countdown(digit, mask, progress)`, `Moment::burst(p)`, `Moment::end(elapsed, words)` |
| a picture | `Picture::new(source, place)`, then `p.backdrop = true` |
| a heading hint | `Hint::text(text, font, pos, color, slide)` (the other hints are tuple variants) |
| a slide | `Slide::new(design)` or `Slide::default()`, then set fields (`s.line = 3`) |
| blocks, inlines | `Block::heading`, `paragraph`, `list`, `ordered_list`, `image`, `code`, `quote`, `callout`, `table`, `visual`; `Inline::text`, `math`, `link` |
| a list item | `ListItem::new(marker, inlines)`, then set `step`, `children`, `checked` |
| theme colours | `Tokens::default()`, then set fields (`t.accent = ...`) |
| an annotation | `Annotation::new(points, color, width)` |

The few types that stay exhaustive are closed by nature: the paint primitives (`Color`, `Pos2`,
`Vec2`, `Rect`, `Stroke`, `Mesh`, `Vertex`), `Place`, `tokens::Value` (a YAML value), and the art
pipeline's `Artwork`, `Strategy` and `MediumKind`.

## No third-party types

The stable SDK surface exposes **no third-party types**. Everything an extension touches is an
mdeck type: `Color`, `Pos2`, `Rect`, `Mesh`, `Texture`, `Font`, `Painter`. mdeck draws through
egui internally, but no egui type appears in the SDK, so mdeck can upgrade egui (and every other
dependency) in a minor release without breaking you. The SDK's own test checks that no public item
names an egui or glow type.

## The `unstable-egui` escape hatch

If you need a drawing operation `mdeck_sdk::paint` does not offer yet, you can opt into raw egui
access:

```toml
[dependencies]
mdeck-sdk = { version = "2", features = ["unstable-egui"] }
```

This adds `Painter::egui_painter()`, `VisualCx::egui_ui()` and `DesignCx::egui_ui()`.

The feature is **outside the compatibility promise**. mdeck upgrades egui whenever a new version
is out, and any minor release of mdeck may then break code that uses these methods. Use it for
experiments, keep the egui code small and in one place, and please open an issue describing what
you needed: every use of the escape hatch is a request to extend `mdeck_sdk::paint` in the next
minor version.

## Hidden items

The `mdeck_sdk::host` module is hidden from the documentation. It is how mdeck itself hands
extensions their painters and contexts; extensions never call it, and it changes whenever mdeck's
internals do. It is not part of the stable surface.
