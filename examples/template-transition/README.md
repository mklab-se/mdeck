# template-transition

A slide transition for [mdeck](https://github.com/mklab-se/mdeck), made with
`mdeck sdk new transition template-transition`. A deck selects it with
`transition: template-transition` in its frontmatter.

## Files

| File | What it is |
|---|---|
| `src/lib.rs` | The transition: `look`, `paint_over` and `register()` |
| `theme.yaml` | A theme that uses it, registered as `template-transition` |
| `deck.md` | A sample deck |
| `tests/golden.rs` | A golden-image test of the overlay |

## Build and present

```bash
mdeck build --with .            # writes ./target/release/mdeck
./target/release/mdeck deck.md  # present the sample deck; press the arrows
./target/release/mdeck export deck.md --moment transition --slide 2 --output-dir out # halfway into slide 2
```

## Test

```bash
cargo test                          # unit tests and the golden image
MDECK_UPDATE_GOLDEN=1 cargo test    # accept a deliberate change of look
```

The first test run records `tests/golden/template-transition.png`. Commit it.

See the [SDK guides](https://github.com/mklab-se/mdeck/tree/main/docs/sdk).
