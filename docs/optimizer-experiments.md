# Optimizer experiments (2026-09-24)

Lessons from the optimizer study run on 2026-09-24 in a throwaway lab
worktree (since deleted). Reconstructed from the session transcripts: the
raw result logs were lost, so the numbers below are the ones the runs printed
directly.

All datasets are referred to by guest count only; no guest names, group
names, or other content from `wedding.wseat` appear below.

## What is already implemented (cross-checked against current code)

Everything this investigation recommended for production is now live in
`crates/seating-core/src/optimizer.rs` and `models.rs`:

- `OptimizationConfig::default().steps` is `200_000` (was `50_000`).
- `HeuristicOptimizer` runs parallel ILS chains: every odd-numbered
  fresh-random chain opens one extra table (`open_extra_table`); a chain
  kicks (a few random moves, 25% chance of opening a table) after
  `stall_patience(guests) = (4 * guests^2).max(2000)` steps without a new
  best.
- `KICK_MOVES = 3`, `KICK_OPEN_TABLE_PERCENT = 25`, `PATIENCE_FLOOR = 2000`
  (10 x `LAHC_HISTORY_LEN`) in `optimizer.rs` match the recommendation below
  exactly.

Commits that adopted them: `dc7275b` (open an extra table on every other
start), `89cb95b` (steps 50k -> 200k), `d4d5dfb` (ILS chains that kick on
stagnation).

## Round 1: is a genetic algorithm worth it?

**Question:** the user suspected LAHC hill-climbing was getting stuck in
local optima on a big combinatorial space, and asked whether a genetic
algorithm (GA) + local search would do better.

**Setup:** the dataset was the user's own file, copied read-only into
`experiments/wedding.wseat` — 81 guests, 18 groups, 46 closeness rules (3
"keep apart" at -200), no locked guests, 16 generated tables (1
semicircle head table + 15 round tables, 164 seats total). A worktree-only
lab harness (`optimizer/lab.rs` + `examples/lab.rs`) reimplemented the LAHC
loop generalized over acceptance rule and move mix, checked bitwise
identical to production via a `production_parity` test. All runs seeded,
single-threaded unless noted, 30 seeds per configuration (`sweep1.txt`,
`sweep2.txt`, `ma1.txt`, `div1.txt`, `timed.txt`, `long.txt`).

**What each experiment file was:**

| File | What it swept |
|---|---|
| `sweep1.txt` | Steps per attempt (50k-12.8M) and restart count (1-256) at fixed total budget, to see whether more steps or more restarts helps more. |
| `sweep2.txt` | LAHC history length (1k, 5k, 20k) and simulated annealing starting temperatures (2, 5, 10, 20), all at 3.2M steps. |
| `ma1.txt` | Memetic algorithm (GA + LAHC repair) variants: population size, per-individual init steps, child steps, generation count, all at ~3.2M total evaluations. |
| `div1.txt` | The table-count-diversified-starts fix (open 1/2/3 extra tables on varied restarts), the fix that turned out to matter. |
| `timed.txt` | The production app's real config (10 threads/attempts, 10 s) at the old 50k-step and a raised 200k-step default. |
| `sweep3.txt` | A follow-up sweep after `ma1.txt`/`div1.txt`: shorter LAHC steps (50k/100k/150k) and the diversified-start memetic variant (`ma:...:1` = 1 extra table). `ma:16:100000:20000:80:200:1` and `ma:10:200000:20000:60:200:1` both hit 1030.3 in 22-23 of 30 at ~4 s; `sa:16:200000:10:0.05` (16 restarts x 200k, T0=10) hit 1021.9 in 30/30, confirming SA alone still can't clear the table-count ceiling. |
| `long.txt` | Long reference runs (div and MA, 32-39 s) to establish a best-known score. |

**Results (81 guests, all at ~3.2M evaluations / ~3-4 s unless noted):**

| Configuration | Best | Median | Worst |
|---|---|---|---|
| Production defaults, 50k steps, 10 s, 20 threads (5 runs) | 1007.9 | 1002.9 | 1001.2 |
| Same, 200k steps per attempt | 1021.9 | 1021.9 | 1021.9 |
| 16 attempts x 200k steps (single thread) | 1021.9 | 1021.9 | 1021.9 |
| LAHC history 20,000 (vs. production's 200) | 902.4 | 886.5 | 878.2 |
| Simulated annealing (best T0=10, 16x200k) | 1021.9 | 1021.9 | 1021.9 |
| GA + local search (pop 16 x 100k, 80 children x 20k) | **1030.3** (1/30 hits) | 1021.9 | 1021.9 |
| **Varied starts (every other attempt opens 1 extra table)** | **1030.3** (25/30 hits) | **1030.3** | 1028.8 |
| GA + local search, with varied starts too | 1030.3 (23/30) | 1030.3 | 1028.8 |
| Long run, varied starts (64 x 500k, 10 runs) | 1030.3 | 1030.3 | 1030.3 |

Best-known score at this point: **1030.3** (9 tables), found in every one of
20 long runs. A loose upper bound worked out from each guest's best 11
positive-scoring tablemates, weighted by the best-possible 12-seat
round-table proximity profile, gave **1092.4** — so 1030.3 sits within 5.7%
of the true optimum.

**Root cause found:** no LAHC move can *open* an empty table. Join only
targets an occupied table; swap, table-swap and cluster-exchange preserve
the table count; only table-split can increase it, and only when a smaller
table type exists to split onto. Every random start fills tables to
capacity (8 tables for this dataset), so a plan needing a 9th table (all the
>1021.9 plans) was structurally unreachable, no matter the step budget.
Starting some restarts with 1-3 extra empty tables reached 1030.3 in 21-25
of 30 runs at equal cost; going further (up to 2 or 3 extra tables) gave no
further gain.

**Conclusion (adopted): tune, don't replace.** A GA at equal cost matched
the tuned optimizer's median (1021.9) and only beat it when crossover
happened to produce a 9-table child (1/30 runs) — no better than plain
varied starts (25/30), while adding ~200 lines and 4 more parameters to
tune. **Rejected:** GA/memetic algorithm, longer LAHC history, plain
simulated annealing (all converged to the same 1021.9 ceiling as production
LAHC — accepting worse moves was never the bottleneck). **Adopted:** the
budget was raised (steps 50k -> 200k) and restarts were varied by
opening extra tables — these became the two production changes named above.

## Round 2: stagnation restarts, ILS, and reheating

**Question (user follow-up):** should an attempt stop once it's no longer
improving, and would either simulated annealing on a stalled run
("reheating") or perturb-and-continue from a finished attempt
("iterated local search", ILS) help further? Asked in parallel with another
coder wiring the round-1 fixes into production `optimizer.rs`.

**Setup:** same lab harness, extended with a generalized `search()` that can
stop early after `patience` steps without a new best (returns
`steps_used`/`last_improvement`/`max_gap` for diagnostics), and a
`budgeted()` driver that chains segments under a `Strategy` (`seg_cap`,
`patience`, a `Kick`, and `fresh_every`). `Kick` is `None` (fresh restart),
`Moves{count, open_pct}` (ILS: perturb the chain's best with `count` random
swaps/joins, `open_pct`% chance of first opening a table), or
`Sa{t0, len}` (reheat: `len` SA steps cooling from `t0`, then resume LAHC).
Compared at **equal step budgets**, not equal wall-clock time — a concurrent
build was running and distorting timings (`budget1m.txt`, `budget3m.txt`,
`budget10m.txt`, `k26.txt`, `x2_budget4m.txt`, `x2_budget12m.txt`,
`conv_x1.err`, `conv_x2.err`).

A second, larger dataset was built for the scaling question:
**the 162-guest doubled wedding** (`wedding_x2.wseat`, via `scale.py`) — two
independent copies of every guest/group/rule with double the tables;
cross-copy pairs score 0. Best known there: **2065.6** (5 above 2 x 1030.3,
since the two copies compete for the same tables).

**What div/k26/conv were:**

- `div1.txt` (round 1, above) already established that opening 1-3 extra
  tables on a start reaches the same ceiling; k26 (below) tests how the
  *stagnation patience* should scale, not the table-opening question.
- `k26.txt`: patience = 26,000 (≈ 4 x 81² / 1000, i.e. checking the
  `4·n²`-scaling rule at 81 guests) against patience 20k/50k/100k, at 1M and
  3M step budgets.
- `conv_x1.err` / `conv_x2.err`: unlimited single LAHC runs (`conv` mode)
  logging the step of last improvement and the longest stagnation gap, 30
  seeds each, on the 81-guest and 162-guest datasets — this is where the
  `4·n²` patience formula's constant came from.

**Convergence data** (step of last improvement, one unlimited LAHC run from
a varied start):

| Guests | Median | 90th pct |
|---|---|---|
| 81 | 78,304 | 118,759 |
| 162 | 269,160 | 411,255 |

Doubling the guest count multiplied convergence time by 3.4-4.4x — consistent
with roughly *n*² scaling, which is the basis of the `4·n²` patience formula
now in `optimizer.rs` (`stall_patience`).

**Results, 81 guests, 3M-step budget (30 seeds, hits of 1030.3):**

| Configuration | Hits | Median | Worst |
|---|---|---|---|
| Baseline (fixed 200k-step restarts, varied starts) | 23/30 | 1030.3 | 1028.8 |
| Stagnation-stop only (K=20k/26k/50k) | 25-26/30 | 1030.3 | ~1029.6 |
| ILS, 3 moves, open 25%, never a periodic fresh start, K=50k | **30/30** | 1030.3 | 1030.3 |
| ILS, same + fresh start every 5th segment | 30/30 | 1030.3 | 1030.3 |
| ILS but never opens a table (any move count) | 0/30 | 1021.9 | 1021.9 |
| Reheat (SA burst), no fresh starts | 0/30 | 1021.9 | 1000.9-1021.9 |
| Reheat + fresh start every 5th | 17-27/30 | 1030.3 | varies |

**Results, 162 guests, 12M-step budget:**

| Configuration | Hits of 2065.6 | Median |
|---|---|---|
| Baseline (fixed restarts) | 0/30 | 2058.35 |
| Stagnation-stop only | 0/30 | 2060.35-2060.6 |
| **ILS, 3 moves, open 25%, K=100k** | **25/30** | **2065.6** |
| ILS, 3 moves, never opens a table | 0/30 | 2052.3 |

**Findings:**
- The kick (or reheat) must be able to open a table, or it's stuck on the
  same 8-table (81 guests) / competing-for-tables (162 guests) ceiling as
  round 1 — same root cause, different mechanism.
- **Reheating (SA burst on stall) never beat plain restarts** at equal
  budget on its own; only combined with periodic fresh starts did it
  approach ILS, and never exceeded it. **Rejected.**
- Stagnation-stop alone is a small, safe gain (removes needing per-size step
  tuning) but never reached 2065.6 on 162 guests by itself.
- ILS is clearly the best method, and the gap widens with problem size: 0/30
  for restarts vs. 25/30 for ILS at 162 guests, vs. a much smaller gap at 81
  guests.
- Weak spot: at a very small budget (1M steps, 81 guests) ILS's worst case
  was still 1021.9 — a chain that hadn't yet had a kick land on the 9-table
  configuration.
- Periodic fresh starts (every 5th segment) were within noise; the report
  recommended dropping them since independent per-thread chains already
  provide the diversity.

**Conclusion (adopted): iterated local search**, with the exact parameters
now in production:
- Patience: `4·n²` steps without a new best (26k at 81 guests, 105k at 162),
  floored at `10 x LAHC_HISTORY_LEN` for tiny lists.
- Kick: from the chain's best; 25% chance to open one table (filled to its
  minimum from tables above their own minimum), then 3 random guest
  swaps/relocations, accepted unconditionally.
- No periodic fresh starts — parallel chains already provide the diversity.
- Multi-threaded determinism: each chain's RNG is seeded from
  `(config.seed, chain index)`; results are bit-reproducible for a fixed
  per-chain step budget, but not under a wall-clock deadline (steps
  completed per chain then depend on machine load) — same caveat production
  already documents for existing timed runs.

**Open questions the transcripts flag as unmeasured:**
- The ILS/patience formula was never measured inside the real
  multi-threaded `optimize_timed` path (10 s / N threads) — only
  single-thread, fixed-step-budget lab runs.
- A patience floor for guest lists much smaller than 81 is untested.
- The "cleaner long-term fix" for the table-opening blind spot — a move
  that opens a new table by gathering guests from several tables at once,
  instead of biasing starts/kicks — was named but never prototyped.

## Not preserved

- The raw result logs (`sweep*`, `ma1`, `div1`, `timed`, `long`, `budget*`,
  `k26`, `x2_budget*`, `conv_*`) and the 162-guest doubled dataset.
- The lab harness (`optimizer/lab.rs`, `examples/lab.rs`: an LAHC loop
  generalized over acceptance rule, move mix and restart strategy, checked
  bitwise-identical to production by a `production_parity` test) and its
  helper scripts. It needed two uncommitted hooks into the crate, never meant
  for `main`: `pub mod lab;` in `optimizer.rs` and `pair_matrix` made
  `pub(crate)` in `scoring.rs` for the GA crossover.
