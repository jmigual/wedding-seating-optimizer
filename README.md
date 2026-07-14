# wedding-seating-optimizer

Rust workspace for wedding seating optimization:

- `crates/seating-core`: models, parsing, validation, scoring, optimization, rendering, tests.
- `crates/seating-cli`: clap-based CLI (`validate`, `optimize`, `score`, `render`, project file tools).
- `crates/seating-gui-egui`: native egui GUI — structured editors, background optimization, and an
  interactive seating canvas with drag-and-drop seat editing and live score feedback.
- `crates/seating-gui`: the original iced GUI. **Deprecated** — kept frozen outside the workspace
  for comparison; new features land in the egui GUI only.

## CLI quick start

```bash
cargo run -p seating-cli -- validate --people people.csv --closeness closeness.csv --tables tables.csv
cargo run -p seating-cli -- optimize --people people.csv --closeness closeness.csv --tables tables.csv --output seating.csv --seed 1234 --solutions 1
cargo run -p seating-cli -- score --people people.csv --closeness closeness.csv --tables tables.csv --seating seating.csv
cargo run -p seating-cli -- render --people people.csv --tables tables.csv --seating seating.csv --output seating-plan.svg
```

## GUI

```bash
cargo run -p seating-gui-egui
```

Edit guests, groups, closeness rules, and tables in the side panel; run the optimizer in the
background; then fine-tune the plan directly on the canvas — drag a guest onto another seat to
move or swap them (locks and table compatibility are enforced), and watch the score update with
each change. The plan exports to SVG/PNG with the same palette the app uses.

![Wedding Seating egui GUI with drag-and-drop seating canvas](docs/gui-egui-canvas.png)

### Deprecated iced GUI

The original GUI still builds from its own manifest (it is excluded from the workspace so the
workspace can track current dependencies):

```bash
cargo build --manifest-path crates/seating-gui/Cargo.toml
```
