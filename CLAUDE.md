# Wedding Seating Optimizer

This repository is a Rust workspace for optimizing wedding seating plans: it parses guest
lists and closeness rules, runs a heuristic optimizer that assigns people to table seats,
scores solutions, and renders them as SVG/PNG. It ships both a CLI and a native GUI.

## Workspace Overview

Three crates under `crates/`, with deliberately separated responsibilities:

- `seating-core`: the library crate. It owns **all** domain logic — models, CSV/JSON I/O,
  validation, scoring, the optimizer, editing helpers, and rendering. The CLI and GUI contain
  **no** business logic of their own; everything lives here.
- `seating-cli`: a thin `clap` executable over `seating-core`. Keep it to argument parsing,
  file I/O, command dispatch, and user-facing output.
- `seating-gui`: a native `iced` GUI over `seating-core`. Keep it to view/update/state wiring;
  any logic worth testing belongs in `seating-core`.

Layout of the library (`crates/seating-core/src/`):

- `models.rs` — domain types: `Person`, `TableTypeConfig`/`TableInstance`, `ClosenessRule`,
  `ProjectInput`/`ProjectFile`, `SeatingSolution`/`SeatingAssignment`, `OptimizationConfig`,
  `OptimizationResult`, and error/report types. Id aliases: `PersonId`, `GroupId`,
  `TableTypeId`, `SeatIndex` (plain type aliases, **not** newtypes — they document intent
  but do not prevent mixing; use them consistently anyway).
- `io.rs` — CSV parsing/serialization for people/closeness/tables/seating, plus the
  `ProjectFile` JSON format (`PROJECT_FILE_VERSION`).
- `validation.rs` — pre-optimization project validation, table-instance generation, and
  seating-solution validation.
- `scoring.rs` — distance functions (circular/perimeter), proximity weights, pairwise and
  whole-solution scoring.
- `optimizer.rs` — the `SeatingOptimizer` trait and `HeuristicOptimizer` (multi-restart
  hill-climbing).
- `render.rs` — layout building and SVG/PNG rendering via `resvg`.
- `editing.rs` — GUI-facing parse/lookup helpers for editing project data.

The public surface is re-exported from `lib.rs`. Sample inputs live in `examples/`.

## Architectural Expectations

- Put any reusable logic in `seating-core`, never in the CLI or GUI. If you find yourself
  writing scoring, validation, or optimization logic inside `seating-cli` or `seating-gui`,
  it belongs in `core`.
- Keep `seating-cli` thin: parse args, read/write files, call into `core`, format output.
- Keep `seating-gui` to Elm-style `state` / `update` / `view` wiring. Logic that could be
  unit-tested without a window belongs in `core`.
- No dependency edge from `seating-core` back into `seating-cli` or `seating-gui`.
- Don't leak UI concerns (terminal formatting, `iced` message types, exit handling) into
  `core`'s public API.
- New CLI subcommand → add the domain capability to `core` first, then a thin dispatch arm in
  `seating-cli`.

## Domain Guidance

- Preserve **seating validity** as the load-bearing invariant: table capacity (min/max
  people), no seat double-booked, locked guests placed at their locked table/seat, and every
  person assigned exactly once. Validation lives in `validation.rs`; changes that touch the
  optimizer or models must keep these intact and prove it with a test.
- **Determinism is a contract.** `HeuristicOptimizer` seeds every RNG stream from
  `OptimizationConfig::seed` (`StdRng::seed_from_u64`, per-attempt seeds derived
  deterministically). The same seed + same input must always produce the same result. Never
  reach for `thread_rng()` or any nondeterministic source in the optimizer or scoring.
- Scoring drives optimization decisions — treat changes to distance functions, proximity
  weights, or `score_solution` as behavior changes and cover them with tests, not just the
  happy path.
- Prefer the existing id aliases and explicit domain types over loose primitives.
- When adding or changing a heuristic, document the tradeoff and whether the behavior is
  exact, greedy, or approximate.

## Rust Conventions

- All crates are **edition 2021**. Follow idiomatic Rust; run `cargo fmt` and keep
  `cargo clippy` warning-free.
- `snake_case` for functions/modules, `CamelCase` for types/traits,
  `SCREAMING_SNAKE_CASE` for consts.
- Prefer borrowing over cloning; `&str` over `String` for parameters that don't need ownership.
- Prefer `pub(crate)` unless an item is intentionally part of `seating-core`'s public API.
  Because `core` is consumed by two front-ends, a `pub` item with no caller in *one* crate may
  still be used by the other — check both before calling it dead code.
- Keep the re-export list in `lib.rs` in sync when you add or rename a public item.
- Avoid `unwrap()`/`expect()` outside tests unless the invariant is local, obvious, and
  documented. Return `Result`/`Option` from library code.
- Keep files focused. Don't grow a single module to cover unrelated concerns; split it.

## Error Handling

- `seating-core` uses `thiserror` domain error enums (`ValidationError`, `RenderingError`,
  parse errors). Return structured errors; don't panic on invalid user input.
- `ValidationReport` aggregates multiple problems — prefer reporting **all** validation errors
  in one pass over failing on the first.
- `seating-cli` and `seating-gui` use `anyhow` at the boundary to add context and present
  concise, user-facing diagnostics. They must not re-implement core logic.
- Panic only for genuine internal invariants, never for expected bad input.

## Testing and Validation

- Integration tests live in `crates/seating-core/tests/`. Unit tests sit in a `#[cfg(test)]`
  module beside the code, or a `tests` submodule file when large.
- When you change the optimizer, scoring, validation, parsing, or a public API, add or update a
  test that would have failed before the change.
- For the optimizer, assert on **invariants** (solution validity, capacity respected, score
  monotonicity, seed reproducibility) rather than an exact hand-written layout.
- Assert on error **type/variant**, not on a specific error message string.
- Reuse the sample inputs in `examples/` before writing new fixtures.

Useful commands:

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace`
- `cargo test -p seating-core`
- `cargo run -p seating-cli -- --help`
- `cargo run -p seating-gui`

## Style Pitfalls (found and fixed in the 2026-07 audit — do not reintroduce)

Every rule here traces to a real defect this codebase shipped with.

- **Determinism:** never let `HashMap` iteration order affect a result. Summing pair scores
  in `HashMap` order made identical solutions score bitwise-differently across runs (f64
  addition is not associative). Use `BTreeMap` or sort keys in scoring/optimizer paths.
- **Hot loops:** hoist loop-invariant structures (table lookups, id→person maps, closeness
  lookups, generated instances) out of iteration loops; never allocate owned `String`s just
  to perform a map lookup.
- `score_solution` validates the solution internally — do not pair it with a separate
  `validate_seating_solution` call.
- **No literal copies of defaults:** construct configs with struct-update syntax
  (`..OptimizationConfig::default()`). The GUI silently drifted from core defaults by
  re-typing `10`/`200`/`1` as literals.
- **Error variants:** reserve `ValidationError::MalformedInput` strictly for parse/format
  failures; any structurally distinct failure gets its own variant (tests assert on
  variants and cannot when everything is `MalformedInput`). One display voice crate-wide:
  lowercase fragment, no trailing period. Core errors implement `std::error::Error` —
  convert to `anyhow` with `?` or `anyhow::Error::new`, never `anyhow!(e.to_string())`.
- **Imports:** put types in the module's `use` block; do not scatter fully-qualified
  `crate::models::X` paths in later edits.
- **Single owner for parsing:** field-parsing helpers (optional/required usize, f64,
  pipe-separated lists) live once in `seating-core` (`editing.rs`); `io.rs` and the GUI
  reuse them. The GUI once re-implemented `people_per_side` parsing with divergent
  semantics — same data validated differently depending on entry path.
- **Never blanket-`#![allow(dead_code)]`** to silence unused warnings — wire the item up or
  delete it. GUI views use the `components.rs` helpers instead of re-inlining
  `button(text(..)).padding(..).style(..)` chains.
- **GUI conventions:** free-form input stays as `String` fields parsed via core helpers at
  materialize time; every mutating message must call `refresh_validation_and_layout()`;
  user feedback carries a severity (info/success/error) — never a single neutral message
  string. The GUI is deliberately dark-theme-only: `styles.rs` owns the palette (including
  semantic error/success colors) and ignores iced's `Theme`.
- **SVG safety:** `render.rs` builds SVG with `format!` — any new attribute interpolating
  user data must go through `escape_xml`.
- **Tests:** a new test must fail against the pre-change code. Don't assert `score > K` on
  a fixture where any optimizer output passes (the old integration test was a tautology);
  assert outcomes (who shares a table, exact weight profiles, variant matches).

## Preferred Change Strategy

- Fix issues at the owning layer (`core`), not by patching around them in the CLI or GUI.
- Preserve crate boundaries; keep changes minimal and local.
- Reuse existing domain types, the `SeatingOptimizer` trait, and I/O helpers before
  introducing new ones.
- Don't add dependencies unless they clearly reduce complexity. The workspace pins shared
  deps in the root `[workspace.dependencies]` — add there, not per-crate ad hoc.
