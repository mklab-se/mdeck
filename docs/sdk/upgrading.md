# Upgrading your extension

Each major version of mdeck has a section on this page that lists every breaking change to the SDK
and the formats extensions use, with before and after code. Within a major version nothing breaks
(see [compatibility](compatibility.md)), so you only need this page when you move to a new major.

## mdeck 2.0

SDK 2.0 is the first version of `mdeck-sdk`. There is nothing to upgrade from: before 2.0, engines
and visuals were modules inside mdeck itself, and adding one meant forking mdeck.

If you maintained such a fork, the closest mapping from mdeck 1.x internals to the SDK is:

| mdeck 1.x (inside the fork) | SDK 2.0 |
|---|---|
| a variant of `EngineKind` and its `EngineDef` | a `pub static DEF: EngineDef` in your crate, registered with `Registry::engine` |
| `Engine::update(stage)` / `Engine::paint(stage)` with egui painters | `Engine::update(frame, stage)` / `Engine::paint(painter, frame, stage)` with `mdeck_sdk::paint::Painter` |
| a variant of the `Chart` enum and a match arm in `visualizations::draw` | a type implementing `Visual`, registered with `Registry::visual` |
| a `Layout` variant | a `DesignSet`, registered with `Registry::design_set` |
| `render::hints::push` | `VisualCx::publish` / `DesignCx::publish` |
| theme files in `crates/mdeck/themes/` | `Registry::theme(name, include_str!(...))`, or a theme file next to the deck |

Start from `mdeck sdk new <kind> <name>` and move your drawing code into the new crate.
