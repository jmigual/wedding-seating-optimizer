# wedding-seating-optimizer

Rust workspace for wedding seating optimization:

- `crates/seating-core`: models, parsing, validation, scoring, optimization, rendering, tests.
- `crates/seating-cli`: clap-based CLI (`validate`, `optimize`, `score`, `render`, project file tools).
- `crates/seating-gui-egui`: native egui GUI — structured editors, background optimization, and an
  interactive seating canvas with drag-and-drop seat editing and live score feedback.

## CLI quick start

```bash
cargo run -p seating-cli -- validate --people people.csv --closeness closeness.csv --tables tables.csv
cargo run -p seating-cli -- optimize --people people.csv --closeness closeness.csv --tables tables.csv --output seating.csv --seed 1234 --solutions 1
cargo run -p seating-cli -- score --people people.csv --closeness closeness.csv --tables tables.csv --seating seating.csv
cargo run -p seating-cli -- render --people people.csv --tables tables.csv --seating seating.csv --output seating-plan.svg
```

## CSV formats

Sample files matching each format live in `examples/` (`people.csv`, `closeness.csv`,
`tables.csv`).

- **People** — `id,name,table_type,groups,locked_table,locked_seat`
  `groups` is pipe-separated (e.g. `family|friends`); `table_type`, `locked_table`, and
  `locked_seat` are optional.
- **Closeness** — `left_id,right_id,score`
  `left_id`/`right_id` are person or group ids. A group paired with itself means "seat its
  members together"; negative scores keep people apart.
- **Tables** — `table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side`
  `shape` is `round`, `rectangular`, `square`, or `semicircle`. `people_per_side` (e.g. `2|2|1|1`) is
  required for `rectangular`/`square` and must sum to `max_people`. Leave `number_of_tables`
  blank to auto-generate enough tables to seat everyone.
- **Seating** (optimizer output) — `table_number,table_type,seat_index,person_id,person_name`

## GUI

```bash
cargo run -p seating-gui-egui
```

Edit guests, groups, closeness rules, and tables in the side panel; run the optimizer in the
background; then fine-tune the plan directly on the canvas — drag a guest onto another seat to
move or swap them (locks and table compatibility are enforced), and watch the score update with
each change. The plan exports to SVG/PNG with the same palette the app uses.

![Wedding Seating egui GUI with drag-and-drop seating canvas](docs/gui-egui-canvas.png)
