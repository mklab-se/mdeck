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
operations, new hint kinds and content fields. Write code that tolerates additions:

- implement only the trait methods you need; new hooks come with defaults;
- match on SDK enums that may grow (`Hint`, `Block`, `Inline`, `PictureSource`) with a wildcard
  arm (`_ => {}`);
- build SDK structs with `..Default::default()` or the provided constructors (`Frame::new`,
  `Stage::new`, `Capabilities { picture: true, ..Capabilities::NONE }`) where you can.

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
