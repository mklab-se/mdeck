# template-design-set

A design set for [mdeck](https://github.com/mklab-se/mdeck), made with
`mdeck sdk new design-set template-design-set`. A theme selects it with
`designs: template-design-set`.

## Files

| File | What it is |
|---|---|
| `src/lib.rs` | The design set: `render`, `unsupported` and `register()` |
| `theme.yaml` | A theme that uses it, registered as `template-design-set` |
| `deck.md` | A sample deck |
| `tests/golden.rs` | A golden-image test |

## Build, present, export, check

```bash
mdeck build --with .                                   # writes ./target/release/mdeck
./target/release/mdeck deck.md                         # present the sample deck
./target/release/mdeck export deck.md --output-dir out # stills of every slide
./target/release/mdeck deck.md --check                 # what the design set cannot show
```

## Test

```bash
cargo test                          # unit tests and the golden image
MDECK_UPDATE_GOLDEN=1 cargo test    # accept a deliberate change of look
```

The first test run records `tests/golden/template-design-set.png`. Commit it.

See the [design sets guide](https://github.com/mklab-se/mdeck/tree/main/docs/sdk/design-sets.md).
