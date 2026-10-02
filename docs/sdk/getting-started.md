# Getting started

This page takes you from nothing to your own engine running in a custom mdeck build. It takes
about fifteen minutes, most of it compiling.

## What you need

- mdeck 2.x, installed as usual (`mdeck --version`).
- A Rust toolchain (`rustup`, stable). You need it to build an mdeck with your extension in it;
  the people you give that build to do not.

## 1. Create an engine

```bash
mdeck sdk new engine glow
cd glow
```

This creates a crate that builds and runs as it is:

| File | What it is |
|---|---|
| `Cargo.toml` | Depends on `mdeck-sdk` and nothing else |
| `src/lib.rs` | The engine, with a comment on every hook |
| `theme.yaml` | A showcase theme named `glow` that selects the engine |
| `deck.md` | A sample deck that uses the theme |
| `tests/golden.rs` | A golden-image test of the engine's settled frame |
| `README.md` | The commands on this page |

The heart of `src/lib.rs` is three things: a definition, an entry point and the engine itself.

```rust
pub static DEF: EngineDef = EngineDef {
    name: "glow",                        // themes select it with `engine: glow`
    summary: "A few slow glows breathing under every slide.",
    capabilities: Capabilities::NONE,    // it only decorates
    settings: &[],                       // it reads no settings
    needs: Needs { page: false },
    ending_caption_delay: 1.0,
    create: |_settings| Box::new(Glow::default()),
    board: None,
};

pub fn register(r: &mut Registry) -> Result<(), RegistryError> {
    r.engine(&DEF)?;
    r.theme("glow", include_str!("../theme.yaml"))
}

impl Engine for Glow {
    fn update(&mut self, frame: &Frame, stage: &Stage) { /* advance the clock */ }
    fn paint(&mut self, painter: &mut Painter, frame: &Frame, stage: &Stage) { /* draw */ }
    fn animating(&self) -> bool { !self.settled }
}
```

`register` is the one function mdeck calls in your crate. It registers everything the crate
brings: here an engine and the theme that shows it off.

## 2. Test it

```bash
cargo test
```

The first run renders the engine headlessly (no window, no GPU) and records
`tests/golden/glow.png`. Look at it, then commit it. Later runs compare against it and fail when
the look changes. When you change the look on purpose, accept the new image:

```bash
MDECK_UPDATE_GOLDEN=1 cargo test
```

A failing comparison writes the new frame next to the golden one as `glow.actual.png`, so you can
look at both. The helpers are in [`mdeck_sdk::testing`](https://docs.rs/mdeck-sdk/latest/mdeck_sdk/testing/):
`Headless::render_engine` draws one frame, `assert_golden` compares it.

## 3. Build an mdeck with your engine

```bash
mdeck build --with .
```

`mdeck build` generates a small crate that depends on mdeck and on every extension you name, calls
each extension's `register`, and compiles it in release mode. It prints where the new binary is
(`./target/release/mdeck`). Name as many extensions as you like, as paths or crate names:

```bash
mdeck build --with ./glow --with ../acme-roadmap --with acme-brand-engines
```

Nothing about mdeck's source changes: the custom build is mdeck plus your crates, compiled together.

## 4. See it live

```bash
./target/release/mdeck deck.md
```

The sample deck selects `theme: glow`, which selects `engine: glow`. To try the engine with any
other deck, without changing the deck:

```bash
./target/release/mdeck talk.md --engine glow
```

Press `H` for the HUD (it shows the frame rate), `Shift+T` to cycle themes, `Esc` twice to quit.

## 5. Export stills

Stills are how you check an engine without watching it: they are deterministic, so you can
compare them, attach them to a pull request, or diff them after a change.

```bash
./target/release/mdeck export deck.md --output-dir out              # every slide, settled
./target/release/mdeck export deck.md --slide 2 --at 1.5            # slide 2, 1.5 s into its motion
./target/release/mdeck export deck.md --at 0.8 --moment countdown   # the opening countdown
./target/release/mdeck export deck.md --at 3 --moment end           # the end act
```

`--at <seconds>` runs the engine's clock to that moment before the still is taken; without it,
you get the settled pose. `--moment countdown` and `--moment end` export the opening and the
ending instead of a slide.

## 6. Check it

```bash
./target/release/mdeck deck.md --check
```

`--check` lists problems in the deck, including those your extension reports: settings of the
wrong type or unknown to the engine, things a design set cannot show, mistakes in a visual's fence.

## Where next

- [Concepts](concepts.md): the model your extension lives in, and the contract it keeps.
- [Tutorial step 1](tutorial-1-ambience.md): a calm animated ground, explained line by line.
- `mdeck sdk new visual <name>`, `design-set` and `transition` scaffold the other extension
  points the same way.
