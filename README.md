# wedding-seating-optimizer

Rust workspace for wedding seating optimization:

- `crates/seating-core`: models, parsing, validation, scoring, optimization, tests.
- `crates/seating-cli`: clap-based CLI (`validate`, `optimize`, `score`).
- `crates/seating-gui`: native iced GUI that loads/edits/saves shared CSV/JSON formats and runs optimization.

## CLI quick start

```bash
cargo run -p seating-cli -- validate --people people.csv --closeness closeness.csv --tables tables.json
cargo run -p seating-cli -- optimize --people people.csv --closeness closeness.csv --tables tables.json --output seating.csv --seed 1234 --solutions 1
cargo run -p seating-cli -- score --people people.csv --closeness closeness.csv --tables tables.json --seating seating.csv
```

## GUI

```bash
cargo run -p seating-gui
```
