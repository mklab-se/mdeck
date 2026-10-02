# template-engine

An engine for [mdeck](https://github.com/mklab-se/mdeck), made with
`mdeck sdk new engine template-engine`.

## Files

| File | What it is |
|---|---|
| `src/lib.rs` | The engine: its definition (`DEF`), `register()` and the `Engine` hooks |
| `theme.yaml` | The showcase theme, registered as `template-engine` |
| `deck.md` | A sample deck that shows the engine on common designs |
| `tests/golden.rs` | A golden-image test of the settled frame |

## Build an mdeck with this engine

```bash
mdeck build --with .            # writes ./target/release/mdeck
./target/release/mdeck deck.md  # present the sample deck
```

## Look at it without a window

```bash
./target/release/mdeck export deck.md --output-dir out                 # every slide, settled
./target/release/mdeck export deck.md --slide 2 --at 1.5               # slide 2, 1.5 s into its motion
./target/release/mdeck export deck.md --at 0.8 --moment countdown      # the countdown
./target/release/mdeck export deck.md --at 3 --moment end              # the end act
./target/release/mdeck deck.md --check                                 # problems the engine reports
```

## Test

```bash
cargo test                          # unit tests and the golden image
MDECK_UPDATE_GOLDEN=1 cargo test    # accept a deliberate change of look
```

The first test run records `tests/golden/template-engine.png`. Commit it.

## Learn more

The SDK guides (getting started, concepts, a three-step engine tutorial) are in
[`docs/sdk`](https://github.com/mklab-se/mdeck/tree/main/docs/sdk).
