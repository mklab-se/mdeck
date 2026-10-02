# @{{name}}

A visual kind for [mdeck](https://github.com/mklab-se/mdeck), made with
`mdeck sdk new visual {{name}}`. A deck uses it with a fenced block:

````markdown
```@{{name}}
title: Revenue
- North: 42
- South: 31 (color: 1)
+ East: 55 (color: 2)
```
````

## Files

| File | What it is |
|---|---|
| `src/lib.rs` | The visual: the fence parser, `check`, `steps`, `draw` and `register()` |
| `theme.yaml` | A theme to show it in, registered as `{{name}}` |
| `deck.md` | A sample deck |
| `tests/golden.rs` | A golden-image test |

## Build, present, export, check

```bash
mdeck build --with .                                   # writes ./target/release/mdeck
./target/release/mdeck deck.md                         # present the sample deck
./target/release/mdeck export deck.md --output-dir out # stills of every slide
./target/release/mdeck deck.md --check                 # problems the visual reports
```

## Test

```bash
cargo test                          # unit tests and the golden image
MDECK_UPDATE_GOLDEN=1 cargo test    # accept a deliberate change of look
```

The first test run records `tests/golden/{{name}}.png`. Commit it.

See the [visuals guide](https://github.com/mklab-se/mdeck/tree/main/docs/sdk/visuals.md).
