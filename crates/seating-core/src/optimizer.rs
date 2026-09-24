//! Optimizer abstraction and heuristic seating optimizer.
//!
//! [`SeatingOptimizer`] is the public trait that any optimizer must implement.
//! [`HeuristicOptimizer`] provides a practical default: parallel chains of
//! iterated local search over late acceptance hill climbing, each started
//! from a random (or warm-start) seating, keeping the top-N solutions.

use crate::models::{
    OptimizationConfig, OptimizationResult, Person, ProjectInput, SeatingAssignment,
    SeatingSolution, TableInstance, ValidationError, ValidationReport,
};
use crate::scoring::{ScoringContext, score_solution};
use crate::validation::{validate_project, validate_seating_solution};
use rand::{RngExt, SeedableRng, rngs::StdRng, seq::SliceRandom};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::{Duration, Instant};

// ── Optimizer trait ───────────────────────────────────────────────────────────

/// Trait for seating optimizers.
///
/// Implementors receive a [`ProjectInput`] and an [`OptimizationConfig`], and
/// must return an [`OptimizationResult`] containing at most
/// `config.solutions` [`SeatingSolution`]s sorted best-first, or a
/// [`ValidationReport`] describing why optimization is impossible.
pub trait SeatingOptimizer {
    /// Run the optimizer and return the best solution(s).
    fn optimize(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
    ) -> Result<OptimizationResult, ValidationReport>;
}

// ── Heuristic optimizer ───────────────────────────────────────────────────────

/// Iterated local search (ILS) over late acceptance hill climbing (LAHC),
/// run as independent parallel chains.
///
/// An **approximate** metaheuristic: it returns the best solution it visits,
/// with no optimality guarantee.
///
/// **Algorithm:**
/// 1. For each chain, build a structurally valid assignment with a
///    deterministic seed derived from `config.seed` and the chain index (or
///    start from the warm-start solution), consolidating guests onto fewer
///    tables when `min_people` allows (see
///    [`HeuristicOptimizer::repair_min_constraints`]).
///    Every odd-numbered fresh-random chain then reserves one more,
///    currently-empty table (see [`HeuristicOptimizer::open_extra_table`]);
///    chain 0 starts without one, but its kicks (step 3) can open one too.
///    This is a heuristic, not an exact fix for a structural blind spot:
///    join and swap only touch tables that already have an occupant, and
///    cluster exchange can never target an empty table at all (it selects
///    both tables from an existing occupant's position), so none of those
///    can increase how many tables are in use. Table swap trades whole
///    occupant sets between two tables of *different* types — it can move a
///    group onto a currently-unused table, but the table it came from
///    becomes unused in exchange, so the total count in use never changes.
///    Table split is the one move that *can* increase it, by sending part of
///    an overflowing table onto a second, currently-empty, differently
///    (typically smaller) typed table — but only when such a smaller type
///    exists. So when every table shares one type, or there is no smaller
///    type to split onto, the count of tables in use can only go down during
///    a run, and a plan whose optimum needs more tables than a
///    capacity-filling start ever opens is unreachable regardless of step
///    budget; otherwise it is reachable only rarely, since table split needs
///    a specific split of a specific table to land exactly right. Seeding
///    half the fresh chains with one extra table trades a wasted start
///    (when the extra table doesn't help) for making that class of plan
///    reachable at all.
/// 2. Run LAHC moves (see `lahc_search`):
///    a move is accepted when it is at least as good as the current score
///    *or* as the score held `LAHC_HISTORY_LEN` steps ago, which lets the
///    search cross score-neutral and mildly worse plateaus. Moves: guest
///    pair swap, "join" (move a guest onto the table of another guest, or
///    to another seat of their own table), whole-table occupant swap (the
///    move that relocates a group onto an unused, larger table by trading
///    places with it — the table count in use is unchanged, since the
///    group's old table becomes unused in exchange), table split
///    (the move that empties a table by splitting its occupants across two
///    smaller ones, for a group that doesn't fit either alone), and cluster
///    exchange (the move that swaps a coherent multi-guest subset between
///    two tables at once, for a rearrangement that no sequence of
///    single-guest moves can reach without a strictly worse intermediate
///    state — see [`SearchState::propose_cluster_exchange`]).
/// 3. When a chain's current segment goes [`stall_patience`] steps without
///    improving on its own best, kick: restart from the chain's best seating
///    after a few random moves (see [`HeuristicOptimizer::kick`]), sometimes
///    opening a table, reset the LAHC history to the kicked score and keep
///    climbing. Each run of steps between two kicks is a *segment*.
/// 4. Stop at the deadline or, without one, after `config.steps` steps per
///    chain. Score each chain's best state and keep the top-N solutions.
///
/// **Tradeoff:** a kick keeps most of a good seating and only has to
/// re-optimize around a few changes, so it reaches better plans per step
/// than a fresh random restart — at the risk of a chain staying near one
/// basin. The table-opening kicks and the parallel, independently seeded
/// chains are what keep it diverse; without table opening ILS stays stuck
/// on plans with too few tables. The patience trades kicking too early
/// (before LAHC has converged) against idling on a converged seating.
///
/// **Determinism:** chain `i` is fully determined by
/// `(config.seed, i, steps run)` and, when given, the warm-start solution.
/// With `time_limit_secs == 0` every chain runs exactly `config.steps`
/// steps, so the run is bit-reproducible. Timed runs depend on how many
/// steps each chain completed before the deadline, which varies with
/// machine load, so they are not reproducible.
#[derive(Debug, Default)]
pub struct HeuristicOptimizer;

/// Length of the LAHC acceptance history. Longer histories accept worse
/// moves for longer (more exploration, escapes deeper local optima) at the
/// cost of slower convergence within a fixed step budget.
const LAHC_HISTORY_LEN: usize = 200;

/// Move mix, in percent of proposed steps:
/// - `SWAP_MOVE_PERCENT` guest pair swaps — cheap, local refinement.
/// - `JOIN_MOVE_PERCENT` "join" moves — relocate one guest, the bulk of
///   fine-grained exploration.
/// - `TABLE_SWAP_MOVE_PERCENT` whole-table occupant swaps — relocates a
///   group onto an unused, larger table by trading places with it (table
///   count in use is unchanged).
/// - `CLUSTER_EXCHANGE_MOVE_PERCENT` cluster exchanges — swaps a coherent
///   multi-guest subset between two tables in one move (see
///   [`SearchState::propose_cluster_exchange`]).
/// - the remainder, table splits — opens two smaller tables for a group that
///   doesn't fit either alone (see [`SearchState::propose_table_split`]).
///
/// The last three are rarer because they only pay off for groups near a
/// table boundary or split across tables; most steps are spent on cheaper
/// local moves.
const SWAP_MOVE_PERCENT: u32 = 30;
const JOIN_MOVE_PERCENT: u32 = 40;
const TABLE_SWAP_MOVE_PERCENT: u32 = 10;
const CLUSTER_EXCHANGE_MOVE_PERCENT: u32 = 10;

/// Random guest swaps or joins a kick applies to the chain's best seating.
/// Few enough to stay in a good seating's neighborhood, enough to leave its
/// LAHC basin; measured better than 8 or 20 on an 81-guest wedding.
const KICK_MOVES: usize = 3;

/// Chance, in percent, that a kick first opens one currently-empty table.
/// Without it a chain can never grow past its start's table count.
const KICK_OPEN_TABLE_PERCENT: u32 = 25;

/// Lower bound for [`stall_patience`]: ten LAHC history lengths, so the
/// late-acceptance window of even a tiny project settles before a kick.
const PATIENCE_FLOOR: usize = 10 * LAHC_HISTORY_LEN;

/// Steps between deadline checks, keeping the clock read off the hot path
/// while bounding a timed run's overshoot to one block of steps.
const DEADLINE_CHECK_STEPS: usize = 1024;

/// Steps a segment may go without improving its own best before the chain
/// kicks: `4 · guests²`, at least [`PATIENCE_FLOOR`].
///
/// The time LAHC needs to converge grows about quadratically with the guest
/// count (last improvement at a median ~78k steps for 81 guests, ~269k for
/// 162, from a random start); `4 · n²` (26k at 81 guests, 105k at 162) was
/// measured to reach the best-known plans most often.
fn stall_patience(guests: usize) -> usize {
    (4 * guests * guests).max(PATIENCE_FLOOR)
}

impl HeuristicOptimizer {
    /// Run the heuristic, optionally warm starting every chain from `initial`
    /// instead of a random feasible assignment.
    ///
    /// With `config.time_limit_secs > 0`, one chain per available thread
    /// searches until the time limit passes (`config.attempts` and
    /// `config.steps` are ignored). With `config.time_limit_secs == 0`,
    /// exactly `config.attempts` chains run `config.steps` steps each, same
    /// as [`SeatingOptimizer::optimize`]; so does a time limit too large to
    /// represent as a deadline. On a single-thread machine a timed run is
    /// chain 0 alone, which starts without an extra table and relies on its
    /// kicks to open one.
    ///
    /// **Determinism contract:** chain `i` is fully determined by
    /// `(config.seed, i, steps run)` (and, when given, `initial`), and chain
    /// results are merged in ascending chain order via a stable sort. An
    /// untimed run is therefore bit-reproducible; a timed one is not, since
    /// the steps each chain completes depend on machine load.
    ///
    /// Every returned solution's assignments are in `project.people` order,
    /// regardless of `initial`'s ordering.
    ///
    /// # Errors
    /// Returns a [`ValidationReport`] if `project` is invalid, or if
    /// `initial` is `Some` and is not a valid seating solution for `project`.
    pub fn optimize_timed(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
        initial: Option<&[SeatingAssignment]>,
    ) -> Result<OptimizationResult, ValidationReport> {
        validate_project(project)?;
        if let Some(initial) = initial {
            validate_seating_solution(project, initial)?;
        }
        // An unrepresentable deadline (absurdly large limit) means no deadline.
        let deadline = (config.time_limit_secs > 0)
            .then(|| Instant::now().checked_add(Duration::from_secs(config.time_limit_secs)))
            .flatten();
        self.run_chains(project, config, initial, deadline)
    }

    /// Run search chain `chain` from its start (see the type-level docs)
    /// until `deadline` or, without one, for exactly `config.steps` steps.
    /// Returns the chain's best solution and its segment count (the start
    /// plus one per kick), or `None` if no feasible start exists.
    fn run_chain(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
        chain: usize,
        initial: Option<&[SeatingAssignment]>,
        deadline: Option<Instant>,
    ) -> Option<(SeatingSolution, usize)> {
        let chain_seed = config.seed.wrapping_add((chain as u64) * 17);
        let ctx = ScoringContext::build(project);
        let positions = match initial {
            Some(initial) => {
                let mut positions = vec![(0, 0); project.people.len()];
                for a in initial {
                    let person = *ctx.person_index.get(a.person_id.as_str())?;
                    positions[person] = (a.table_number, a.seat_index);
                }
                positions
            }
            // Every odd-numbered fresh-random chain opens one extra table
            // beyond what capacity-filling would, so the search can reach
            // plans an all-capacity-filling start structurally cannot; see
            // the type-level doc for why no LAHC move can do this on its own.
            None => self.random_feasible_assignment(
                project,
                &ctx.instances,
                chain_seed,
                chain % 2 == 1,
            )?,
        };
        let (improved, segments) = self.lahc_search(
            project,
            &ctx,
            config,
            positions,
            chain_seed ^ 0xA5A5_5A5A,
            deadline,
        );
        let assignments = self.build_assignments(project, &ctx.instances, &improved);
        debug_assert!(
            validate_seating_solution(project, &assignments).is_ok(),
            "lahc_search produced an illegal move: every candidate must be legal by construction"
        );
        let score = score_solution(project, &assignments, config).ok()?;
        Some((SeatingSolution { assignments, score }, segments))
    }

    fn merge_solution(
        &self,
        best: &mut Vec<SeatingSolution>,
        solution: SeatingSolution,
        keep: usize,
    ) {
        best.push(solution);
        best.sort_by(|left, right| right.score.total_cmp(&left.score));
        best.truncate(keep.max(1));
    }

    /// Run the search chains in parallel batches sized to the machine's core
    /// count. With a `deadline`, runs one chain per core until it passes;
    /// without one, runs exactly `config.attempts.max(1)` chains of
    /// `config.steps` steps each. When `initial` is `Some`, every chain
    /// warm-starts from it (with its own per-chain seed) instead of a fresh
    /// random feasible assignment. `project` must already be validated by
    /// the caller.
    fn run_chains(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
        initial: Option<&[SeatingAssignment]>,
        deadline: Option<Instant>,
    ) -> Result<OptimizationResult, ValidationReport> {
        let worker_count = thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1)
            .max(1);
        let chain_count = if deadline.is_some() {
            worker_count
        } else {
            config.attempts.max(1)
        };
        let mut best = Vec::new();
        let mut segments = 0usize;

        for batch_start in (0..chain_count).step_by(worker_count) {
            let batch_end = (batch_start + worker_count).min(chain_count);
            thread::scope(|scope| {
                let handles: Vec<_> = (batch_start..batch_end)
                    .map(|chain| {
                        scope.spawn(move || {
                            self.run_chain(project, config, chain, initial, deadline)
                        })
                    })
                    .collect();
                for handle in handles {
                    if let Some((solution, chain_segments)) =
                        handle.join().expect("optimizer worker panicked")
                    {
                        segments += chain_segments;
                        self.merge_solution(&mut best, solution, config.solutions);
                    }
                }
            });
        }

        if best.is_empty() {
            return Err(ValidationReport {
                errors: vec![ValidationError::NoFeasibleAssignment],
            });
        }

        Ok(OptimizationResult {
            solutions: best,
            attempts_completed: segments,
        })
    }

    /// Construct a random, structurally valid seating assignment as
    /// `positions[i] = (table_number, seat_index)` of `project.people[i]`.
    ///
    /// Returns `None` only when locks conflict or a guest has no candidate
    /// seat at all; an under-`min_people` start is returned as is (it is
    /// penalized by scoring, not rejected). When `open_extra_table` is true,
    /// reserves one more, currently-unused table after the usual
    /// capacity-filling placement (see [`Self::open_extra_table`]).
    fn random_feasible_assignment(
        &self,
        project: &ProjectInput,
        instances: &[TableInstance],
        seed: u64,
        open_extra_table: bool,
    ) -> Option<Vec<(usize, usize)>> {
        let table_lookup: HashMap<usize, &TableInstance> =
            instances.iter().map(|t| (t.number, t)).collect();
        let mut occupied: HashSet<(usize, usize)> = HashSet::new();
        let mut assigned: HashMap<String, (usize, usize)> = HashMap::new();

        // Place locked guests first so their positions are reserved.
        for p in &project.people {
            if let (Some(table_num), Some(seat)) = (p.locked_table, p.locked_seat) {
                let table = table_lookup.get(&table_num)?;
                if seat >= table.max_people || !occupied.insert((table_num, seat)) {
                    return None; // Conflicting locks.
                }
                assigned.insert(p.id.clone(), (table_num, seat));
            }
        }

        let mut rng = StdRng::seed_from_u64(seed);
        let mut pending: Vec<&Person> = project
            .people
            .iter()
            .filter(|p| !assigned.contains_key(&p.id))
            .collect();
        pending.shuffle(&mut rng);
        pending.sort_by_key(|p| self.compatible_table_count(p, instances));

        let mut occupancy: HashMap<usize, usize> = HashMap::new();
        for (table_num, _) in assigned.values() {
            *occupancy.entry(*table_num).or_insert(0) += 1;
        }

        for p in pending {
            let candidates = self.seat_candidates(p, instances, &occupied);
            if candidates.is_empty() {
                return None;
            }
            let chosen =
                self.choose_initial_seat(&mut rng, &candidates, &occupancy, &table_lookup)?;
            occupied.insert(chosen);
            assigned.insert(p.id.clone(), chosen);
            *occupancy.entry(chosen.0).or_insert(0) += 1;
        }

        // Uniform random placement is the worst strategy for min_people: it
        // spreads guests thinly across every table. Deterministically repair
        // under-min tables by consolidating guests (respecting locks and
        // table_type) so the search starts near a feasible region.
        // ponytail: min is soft (scored), so an unrepairable start is still a legal start.
        if !self.satisfies_min_constraints(&assigned, instances) {
            self.repair_min_constraints(
                project,
                instances,
                &table_lookup,
                &mut assigned,
                &mut occupied,
            );
        }

        if open_extra_table {
            self.open_extra_table(
                project,
                instances,
                &table_lookup,
                &mut assigned,
                &mut occupied,
                seed,
            );
        }

        project
            .people
            .iter()
            .map(|p| assigned.get(&p.id).copied())
            .collect()
    }

    /// Reserve one additional, currently-empty table for the search to grow
    /// into: try each currently-unused table (in an order shuffled
    /// deterministically from `seed`) and open the first one that can be
    /// filled to `min_people` (or 1, if the table type sets no minimum) from
    /// other tables' surplus above their own minimum — trying every
    /// candidate rather than just the first means one unfillable empty table
    /// (e.g. a head-table type nobody is compatible with) doesn't waste the
    /// chain's start or kick when another empty table could have been opened
    /// instead. Donors are shuffled deterministically too, so the choice of
    /// table and donors is reproducible.
    ///
    /// Leaves `assigned`/`occupied` unchanged if there is no empty table, or
    /// none of them can be filled without leaving a donor table both used
    /// and under its own minimum — the caller's fallback is simply the
    /// seating it passed in: the capacity-filling start for
    /// [`Self::random_feasible_assignment`], the unchanged best seating for
    /// [`Self::kick`].
    fn open_extra_table(
        &self,
        project: &ProjectInput,
        instances: &[TableInstance],
        table_lookup: &HashMap<usize, &TableInstance>,
        assigned: &mut HashMap<String, (usize, usize)>,
        occupied: &mut HashSet<(usize, usize)>,
        seed: u64,
    ) {
        let mut counts: HashMap<usize, usize> = HashMap::new();
        for (table_num, _) in assigned.values() {
            *counts.entry(*table_num).or_insert(0) += 1;
        }
        let mut empty: Vec<usize> = instances
            .iter()
            .map(|t| t.number)
            .filter(|number| counts.get(number).copied().unwrap_or(0) == 0)
            .collect();
        if empty.is_empty() {
            return; // No extra table to open; fall back to the normal start.
        }
        let mut rng = StdRng::seed_from_u64(seed ^ 0x0BE7_0BE7);
        empty.shuffle(&mut rng);

        for target in empty {
            let table = table_lookup[&target];
            let need = table.min_people.unwrap_or(1).max(1).min(table.max_people);

            let mut donors: Vec<&str> = project
                .people
                .iter()
                .filter(|p| Self::movable_to(p, table))
                .map(|p| p.id.as_str())
                .collect();
            donors.shuffle(&mut rng);

            let mut moves: Vec<(String, usize)> = Vec::new();
            let mut trial_counts = counts.clone();
            let mut moved = 0usize;
            for id in donors {
                if moved == need {
                    break;
                }
                let (src, _) = assigned[id];
                let src_min = table_lookup[&src].min_people.unwrap_or(1).max(1);
                let src_count = trial_counts.get(&src).copied().unwrap_or(0);
                if src_count <= src_min {
                    continue; // Donating would leave the source under its own minimum.
                }
                moves.push((id.to_string(), moved));
                *trial_counts.entry(src).or_insert(0) -= 1;
                moved += 1;
            }

            if moved < need {
                continue; // Not enough surplus at this table; try the next one.
            }
            for (id, seat) in moves {
                let old = assigned
                    .insert(id, (target, seat))
                    .expect("donor already assigned");
                occupied.remove(&old);
                occupied.insert((target, seat));
            }
            return;
        }
        // No candidate table could be filled; leave the capacity-filling start as is.
    }

    /// Whether `person` is a movable donor candidate for `table`: not locked
    /// to any table (locking always fixes a guest's table, seat or not), and
    /// either untyped or matching `table`'s `table_type`.
    fn movable_to(person: &Person, table: &TableInstance) -> bool {
        person.locked_table.is_none()
            && person
                .table_type
                .as_ref()
                .map(|tt| tt == &table.table_type)
                .unwrap_or(true)
    }

    fn compatible_table_count(&self, person: &Person, instances: &[TableInstance]) -> usize {
        instances
            .iter()
            .filter(|table| {
                person
                    .table_type
                    .as_ref()
                    .map(|table_type| table_type == &table.table_type)
                    .unwrap_or(true)
            })
            .filter(|table| {
                person
                    .locked_table
                    .map(|locked_table| locked_table == table.number)
                    .unwrap_or(true)
            })
            .count()
    }

    fn choose_initial_seat(
        &self,
        rng: &mut StdRng,
        candidates: &[(usize, usize)],
        occupancy: &HashMap<usize, usize>,
        table_lookup: &HashMap<usize, &TableInstance>,
    ) -> Option<(usize, usize)> {
        type InitialTableRank = (usize, usize, Reverse<usize>, usize);

        let mut by_table: HashMap<usize, Vec<usize>> = HashMap::new();
        for &(table_number, seat_index) in candidates {
            by_table.entry(table_number).or_default().push(seat_index);
        }

        let mut ranked_tables: Vec<(usize, InitialTableRank)> = by_table
            .keys()
            .copied()
            .filter_map(|table_number| {
                let table = table_lookup.get(&table_number)?;
                let current = occupancy.get(&table_number).copied().unwrap_or(0);
                let shortfall_after_placement = table
                    .min_people
                    .map(|min| min.saturating_sub(current + 1))
                    .unwrap_or(0);
                Some((
                    table_number,
                    (
                        usize::from(current == 0),
                        shortfall_after_placement,
                        Reverse(current),
                        table_number,
                    ),
                ))
            })
            .collect();
        ranked_tables.sort_by_key(|(_, rank)| *rank);

        let best_rank = ranked_tables.first().map(|(_, rank)| *rank)?;
        let best_tables: Vec<usize> = ranked_tables
            .into_iter()
            .take_while(|(_, rank)| *rank == best_rank)
            .map(|(table_number, _)| table_number)
            .collect();
        let chosen_table = best_tables[rng.random_range(0..best_tables.len())];

        let mut seats = by_table.remove(&chosen_table)?;
        seats.sort_unstable();
        let chosen_seat = seats[rng.random_range(0..seats.len())];
        Some((chosen_table, chosen_seat))
    }

    /// Deterministically repair `min_people` violations by consolidating
    /// guests onto fewer tables.
    ///
    /// Repeatedly resolves the smallest deficient (used but under-min) table
    /// by either vacating it entirely (relocating every occupant elsewhere,
    /// respecting locks and required `table_type`) or, when it has guests
    /// locked to it, topping it up from other tables instead. Uses no
    /// randomness, so seed-determinism is preserved.
    fn repair_min_constraints(
        &self,
        project: &ProjectInput,
        instances: &[TableInstance],
        table_lookup: &HashMap<usize, &TableInstance>,
        assigned: &mut HashMap<String, (usize, usize)>,
        occupied: &mut HashSet<(usize, usize)>,
    ) {
        let locked_people: HashSet<&str> = project
            .people
            .iter()
            .filter(|p| p.locked_table.is_some())
            .map(|p| p.id.as_str())
            .collect();
        let person_lookup: HashMap<&str, &Person> =
            project.people.iter().map(|p| (p.id.as_str(), p)).collect();

        // Generous, finite bound so a genuinely unrepairable input terminates
        // instead of looping forever; ordinary inputs converge in far fewer
        // rounds since every round either closes a table or tops one up.
        let max_rounds = (project.people.len() + instances.len() + 4) * 4;
        for _ in 0..max_rounds {
            let mut counts: HashMap<usize, usize> = HashMap::new();
            for (table_num, _) in assigned.values() {
                *counts.entry(*table_num).or_insert(0) += 1;
            }

            let mut deficient: Vec<usize> = instances
                .iter()
                .filter_map(|t| {
                    let min = t.min_people?;
                    let count = counts.get(&t.number).copied().unwrap_or(0);
                    (count > 0 && count < min).then_some(t.number)
                })
                .collect();
            deficient.sort_by_key(|&n| (counts.get(&n).copied().unwrap_or(0), n));

            let Some(table_num) = deficient.first().copied() else {
                return; // No deficiencies left.
            };
            let table = table_lookup[&table_num];

            let mut occupants: Vec<String> = assigned
                .iter()
                .filter(|(_, (t, _))| *t == table_num)
                .map(|(id, _)| id.clone())
                .collect();
            occupants.sort();
            let movable: Vec<&String> = occupants
                .iter()
                .filter(|id| !locked_people.contains(id.as_str()))
                .collect();

            let mut dissolved = false;
            if movable.len() == occupants.len() {
                // No locked occupants: try to relocate everyone elsewhere,
                // vacating the table entirely (an empty table has no min
                // constraint to violate).
                let mut trial_occupied: HashSet<(usize, usize)> = occupied
                    .iter()
                    .copied()
                    .filter(|&(t, _)| t != table_num)
                    .collect();
                let mut moves: Vec<(String, (usize, usize))> = Vec::new();
                let mut ok = true;
                for id in &movable {
                    let person = person_lookup[id.as_str()];
                    let dest = instances.iter().find_map(|dest_table| {
                        if dest_table.number == table_num {
                            return None;
                        }
                        if counts.get(&dest_table.number).copied().unwrap_or(0) == 0 {
                            return None;
                        }
                        if person
                            .table_type
                            .as_ref()
                            .is_some_and(|tt| tt != &dest_table.table_type)
                        {
                            return None;
                        }
                        (0..dest_table.max_people)
                            .find(|seat| !trial_occupied.contains(&(dest_table.number, *seat)))
                            .map(|seat| (dest_table.number, seat))
                    });
                    match dest {
                        Some(seat) => {
                            trial_occupied.insert(seat);
                            moves.push(((*id).clone(), seat));
                        }
                        None => {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok {
                    for (id, seat) in moves {
                        let old = assigned
                            .insert(id, seat)
                            .expect("occupant already assigned");
                        occupied.remove(&old);
                        occupied.insert(seat);
                    }
                    dissolved = true;
                }
            }
            if dissolved {
                continue;
            }

            // Could not vacate the table (locked occupants, or no room
            // elsewhere); try to top it up from other compatible tables.
            let min = table.min_people.expect("deficient table always has a min");
            let mut count = counts.get(&table_num).copied().unwrap_or(0);
            let mut donors: Vec<String> = project
                .people
                .iter()
                .filter(|p| Self::movable_to(p, table))
                .filter(|p| {
                    assigned
                        .get(&p.id)
                        .map(|(t, _)| *t != table_num)
                        .unwrap_or(false)
                })
                .map(|p| p.id.clone())
                .collect();
            donors.sort();

            let mut progressed = false;
            for id in donors {
                if count >= min {
                    break;
                }
                let Some(seat) =
                    (0..table.max_people).find(|s| !occupied.contains(&(table_num, *s)))
                else {
                    break;
                };
                let old = assigned[&id];
                assigned.insert(id, (table_num, seat));
                occupied.remove(&old);
                occupied.insert((table_num, seat));
                count += 1;
                progressed = true;
            }

            if !progressed || count < min {
                return; // Genuinely unrepairable.
            }
        }
    }

    /// Enumerate all valid (table_number, seat_index) pairs for a guest.
    fn seat_candidates(
        &self,
        p: &Person,
        instances: &[TableInstance],
        occupied: &HashSet<(usize, usize)>,
    ) -> Vec<(usize, usize)> {
        let mut candidates = Vec::new();
        for t in instances {
            if p.table_type
                .as_ref()
                .map(|tt| tt != &t.table_type)
                .unwrap_or(false)
            {
                continue;
            }
            if p.locked_table.map(|l| l != t.number).unwrap_or(false) {
                continue;
            }
            if let Some(seat) = p.locked_seat {
                if seat < t.max_people && !occupied.contains(&(t.number, seat)) {
                    candidates.push((t.number, seat));
                }
                continue;
            }
            for seat in 0..t.max_people {
                if !occupied.contains(&(t.number, seat)) {
                    candidates.push((t.number, seat));
                }
            }
        }
        candidates
    }

    /// Check that all used tables satisfy their `min_people` constraint.
    fn satisfies_min_constraints(
        &self,
        assigned: &HashMap<String, (usize, usize)>,
        instances: &[TableInstance],
    ) -> bool {
        let mut per_table: HashMap<usize, usize> = HashMap::new();
        for (table_num, _) in assigned.values() {
            *per_table.entry(*table_num).or_insert(0) += 1;
        }
        for table in instances {
            if let Some(min) = table.min_people {
                let count = per_table.get(&table.number).copied().unwrap_or(0);
                if count > 0 && count < min {
                    return false;
                }
            }
        }
        true
    }

    /// Convert person-indexed `positions` to the public [`SeatingAssignment`]
    /// format, in `project.people` order.
    fn build_assignments(
        &self,
        project: &ProjectInput,
        instances: &[TableInstance],
        positions: &[(usize, usize)],
    ) -> Vec<SeatingAssignment> {
        project
            .people
            .iter()
            .zip(positions)
            .map(|(p, &(table_number, seat_index))| SeatingAssignment {
                table_number,
                table_type: instances[table_number - 1].table_type.clone(),
                seat_index,
                person_id: p.id.clone(),
                person_name: p.name.clone(),
            })
            .collect()
    }

    /// Perturb `best` for an iterated local search restart: with
    /// [`KICK_OPEN_TABLE_PERCENT`]% probability first open one
    /// currently-empty table (see [`Self::open_extra_table`]), then apply
    /// [`KICK_MOVES`] random guest swaps or joins (50/50), accepted
    /// regardless of score.
    ///
    /// Every move is legal by construction — capacity, locks, `table_type`,
    /// no double booking — so the result is a valid seating: guests with a
    /// `locked_seat` never move and guests with a `locked_table` stay at it.
    /// A move can be refused (e.g. a full table or a locked guest), so the
    /// retries are bounded; a project with nothing movable comes back as is.
    // ponytail: the id-keyed maps are rebuilt per table-opening kick (a rare
    // event) because `open_extra_table` works on them; port it to positions
    // if kicks ever become frequent.
    fn kick(
        &self,
        project: &ProjectInput,
        instances: &[TableInstance],
        table_lookup: &HashMap<usize, &TableInstance>,
        best: &[(usize, usize)],
        rng: &mut StdRng,
    ) -> Vec<(usize, usize)> {
        let mut positions = best.to_vec();
        if rng.random_range(0..100u32) < KICK_OPEN_TABLE_PERCENT {
            let mut assigned: HashMap<String, (usize, usize)> = project
                .people
                .iter()
                .zip(&positions)
                .map(|(p, &position)| (p.id.clone(), position))
                .collect();
            let mut occupied: HashSet<(usize, usize)> = positions.iter().copied().collect();
            self.open_extra_table(
                project,
                instances,
                table_lookup,
                &mut assigned,
                &mut occupied,
                rng.random(),
            );
            positions = project.people.iter().map(|p| assigned[&p.id]).collect();
        }

        let mut state = SearchState::new(&project.people, instances, positions);
        let mut moves: Vec<Move> = Vec::new();
        let mut applied = 0;
        for _ in 0..KICK_MOVES * 20 {
            if applied == KICK_MOVES {
                break;
            }
            moves.clear();
            let proposed = if rng.random_range(0..2u32) == 0 {
                state.propose_swap(rng, &mut moves)
            } else {
                state.propose_join(rng, &mut moves)
            };
            if proposed {
                state.apply(&moves);
                applied += 1;
            }
        }
        state.positions
    }

    /// Iterated local search over late acceptance hill climbing, from
    /// `positions` (person-indexed `(table_number, seat_index)`). Runs until
    /// `deadline` or, without one, for exactly `config.steps` steps, and
    /// returns the best state visited plus the number of segments (the
    /// start plus one per kick).
    ///
    /// Each step proposes one move — pair swap, join, whole-table swap,
    /// cluster exchange, or table split (see [`SearchState`]) — skips it if
    /// structurally illegal, and otherwise accepts it when the new score is
    /// at least the current one or at least the score recorded
    /// [`LAHC_HISTORY_LEN`] steps earlier.
    /// Every candidate is legal by construction (locks, `table_type`,
    /// capacity, no double booking), so scoring skips validation.
    ///
    /// When the current segment goes [`stall_patience`] steps without
    /// beating its own best, the search [kicks](Self::kick) the overall best
    /// state and continues LAHC from there with the history reset to the
    /// kicked score. Kicks always restart from the best, never from the
    /// (possibly worse) state the stalled segment ended in.
    ///
    /// Guests with a `locked_seat` never move; guests with only a
    /// `locked_table` change seats within it, so a "head table" of
    /// locked-table guests still gets its internal seating optimized.
    // ponytail: full rescoring per step; delta-scoring the two touched tables is the upgrade if step cost matters past ~200 guests.
    fn lahc_search(
        &self,
        project: &ProjectInput,
        ctx: &ScoringContext,
        config: &OptimizationConfig,
        positions: Vec<(usize, usize)>,
        seed: u64,
        deadline: Option<Instant>,
    ) -> (Vec<(usize, usize)>, usize) {
        if positions.is_empty() {
            return (positions, 1);
        }
        let mut rng = StdRng::seed_from_u64(seed);
        let mut state = SearchState::new(&project.people, &ctx.instances, positions);
        let mut scratch: Vec<Vec<usize>> = Vec::new();
        let mut ranks: Vec<usize> = Vec::new();
        let mut current = ctx.score_positions(&state.positions, config, &mut scratch, &mut ranks);
        let mut best = state.positions.clone();
        let mut best_score = current;
        let mut history = vec![current; LAHC_HISTORY_LEN];
        let mut moves: Vec<Move> = Vec::new();
        let mut undo: Vec<Move> = Vec::new();
        let table_lookup: HashMap<usize, &TableInstance> =
            ctx.instances.iter().map(|t| (t.number, t)).collect();
        let patience = stall_patience(project.people.len());
        let mut segment_best = current;
        let mut stalled = 0usize;
        let mut segments = 1usize;
        let max_steps = if deadline.is_some() {
            usize::MAX
        } else {
            config.steps
        };

        for step in 0..max_steps {
            if step % DEADLINE_CHECK_STEPS == 0 && deadline.is_some_and(|end| Instant::now() >= end)
            {
                break;
            }
            if stalled >= patience {
                let kicked = self.kick(project, &ctx.instances, &table_lookup, &best, &mut rng);
                state = SearchState::new(&project.people, &ctx.instances, kicked);
                current = ctx.score_positions(&state.positions, config, &mut scratch, &mut ranks);
                if current > best_score {
                    best_score = current;
                    best.copy_from_slice(&state.positions);
                }
                history.fill(current);
                segment_best = current;
                stalled = 0;
                segments += 1;
            }
            stalled += 1;

            let slot = step % LAHC_HISTORY_LEN;
            moves.clear();
            let roll = rng.random_range(0..100u32);
            let proposed = if roll < SWAP_MOVE_PERCENT {
                state.propose_swap(&mut rng, &mut moves)
            } else if roll < SWAP_MOVE_PERCENT + JOIN_MOVE_PERCENT {
                state.propose_join(&mut rng, &mut moves)
            } else if roll < SWAP_MOVE_PERCENT + JOIN_MOVE_PERCENT + TABLE_SWAP_MOVE_PERCENT {
                state.propose_table_swap(&mut rng, &mut moves)
            } else if roll
                < SWAP_MOVE_PERCENT
                    + JOIN_MOVE_PERCENT
                    + TABLE_SWAP_MOVE_PERCENT
                    + CLUSTER_EXCHANGE_MOVE_PERCENT
            {
                state.propose_cluster_exchange(&mut rng, &mut moves)
            } else {
                state.propose_table_split(&mut rng, &mut moves)
            };
            if !proposed {
                history[slot] = current;
                continue;
            }

            undo.clear();
            undo.extend(
                moves
                    .iter()
                    .map(|&(person, _)| (person, state.positions[person])),
            );
            state.apply(&moves);
            let score = ctx.score_positions(&state.positions, config, &mut scratch, &mut ranks);
            if score >= current || score >= history[slot] {
                current = score;
                if score > segment_best {
                    segment_best = score;
                    stalled = 0;
                }
                if score > best_score {
                    best_score = score;
                    best.copy_from_slice(&state.positions);
                }
            } else {
                state.apply(&undo);
            }
            history[slot] = current;
        }
        (best, segments)
    }
}

/// Mutable state of one LAHC chain: `positions[i]` is person `i`'s
/// `(table_number, seat_index)`, mirrored by `seats[table_number - 1][seat]`
/// for O(1) occupancy checks and cheap reverts.
struct SearchState<'a> {
    people: &'a [Person],
    instances: &'a [TableInstance],
    positions: Vec<(usize, usize)>,
    seats: Vec<Vec<Option<usize>>>,
    /// Reused shuffle buffer for [`Self::propose_table_split`], to avoid
    /// reallocating one `Vec` per proposal.
    shuffle_scratch: Vec<usize>,
    /// Reused cluster-member buffers for [`Self::propose_cluster_exchange`]:
    /// `cluster_a`/`cluster_b` hold the two clusters being exchanged,
    /// `seats_a`/`seats_b` the destination seats computed for them.
    cluster_a: Vec<usize>,
    cluster_b: Vec<usize>,
    seats_a: Vec<usize>,
    seats_b: Vec<usize>,
}

/// One relocation: `(person, (table_number, seat_index))`.
type Move = (usize, (usize, usize));

impl<'a> SearchState<'a> {
    fn new(
        people: &'a [Person],
        instances: &'a [TableInstance],
        positions: Vec<(usize, usize)>,
    ) -> Self {
        let mut seats: Vec<Vec<Option<usize>>> = instances
            .iter()
            .map(|table| vec![None; table.max_people])
            .collect();
        for (person, &(table_number, seat)) in positions.iter().enumerate() {
            seats[table_number - 1][seat] = Some(person);
        }
        Self {
            people,
            instances,
            positions,
            seats,
            shuffle_scratch: Vec::new(),
            cluster_a: Vec::new(),
            cluster_b: Vec::new(),
            seats_a: Vec::new(),
            seats_b: Vec::new(),
        }
    }

    /// Whether `person` may be moved onto `table_number` at all: not frozen
    /// by a `locked_seat`, not locked to another table, and of a compatible
    /// `table_type`.
    fn may_sit_at(&self, person: usize, table_number: usize) -> bool {
        let p = &self.people[person];
        p.locked_seat.is_none()
            && p.locked_table.is_none_or(|locked| locked == table_number)
            && p.table_type
                .as_deref()
                .is_none_or(|tt| tt == self.instances[table_number - 1].table_type)
    }

    /// Exchange the positions of two random guests (possibly on the same table).
    fn propose_swap(&self, rng: &mut StdRng, moves: &mut Vec<Move>) -> bool {
        let i = rng.random_range(0..self.positions.len());
        let j = rng.random_range(0..self.positions.len());
        if i == j
            || !self.may_sit_at(i, self.positions[j].0)
            || !self.may_sit_at(j, self.positions[i].0)
        {
            return false;
        }
        moves.push((i, self.positions[j]));
        moves.push((j, self.positions[i]));
        true
    }

    /// Move guest `p` to a random free seat on the table of guest `q` — a
    /// table change, or a seat change when `q` shares `p`'s table.
    fn propose_join(&self, rng: &mut StdRng, moves: &mut Vec<Move>) -> bool {
        let p = rng.random_range(0..self.positions.len());
        let q = rng.random_range(0..self.positions.len());
        let dest = self.positions[q].0;
        if p == q || !self.may_sit_at(p, dest) {
            return false;
        }
        let row = &self.seats[dest - 1];
        let free = row.iter().filter(|occupant| occupant.is_none()).count();
        if free == 0 {
            return false;
        }
        let pick = rng.random_range(0..free);
        let Some((seat, _)) = row
            .iter()
            .enumerate()
            .filter(|(_, occupant)| occupant.is_none())
            .nth(pick)
        else {
            return false;
        };
        moves.push((p, (dest, seat)));
        true
    }

    /// Exchange the whole occupant sets of two random tables. Seat indices
    /// are kept when they all fit the destination, otherwise re-indexed by
    /// rank. Score-neutral apart from size/min penalties, this is what
    /// opens an unused larger table for a group that outgrows its current one.
    fn propose_table_swap(&self, rng: &mut StdRng, moves: &mut Vec<Move>) -> bool {
        let table_count = self.instances.len();
        if table_count < 2 {
            return false;
        }
        let a = rng.random_range(1..=table_count);
        let b = rng.random_range(1..=table_count);
        // Same-type tables are score-identical (same shape/max/min/recommended),
        // so swapping their occupants is a pure relabel: no scoring gain, and
        // it gratuitously renumbers the guests' tables.
        if a == b || self.instances[a - 1].table_type == self.instances[b - 1].table_type {
            return false;
        }
        let occupants =
            |table_number: usize| self.seats[table_number - 1].iter().flatten().copied();
        if occupants(a).next().is_none() && occupants(b).next().is_none() {
            return false;
        }
        for (from, dest) in [(a, b), (b, a)] {
            let capacity = self.instances[dest - 1].max_people;
            if occupants(from).count() > capacity
                || occupants(from).any(|person| !self.may_sit_at(person, dest))
            {
                return false;
            }
            let keep_seats = occupants(from).all(|person| self.positions[person].1 < capacity);
            for (rank, person) in occupants(from).enumerate() {
                let seat = if keep_seats {
                    self.positions[person].1
                } else {
                    rank
                };
                moves.push((person, (dest, seat)));
            }
        }
        true
    }

    /// Exchange a "cluster" — one guest plus every table-mate who shares a
    /// group with them — between two different tables in a single move.
    ///
    /// Unlike [`Self::propose_swap`] (one guest at a time) or
    /// [`Self::propose_join`] (one guest to one seat), this relocates a
    /// coherent multi-guest subset on each side at once. That is what lets
    /// the search cross a valley no single-guest move can: e.g. splitting a
    /// table's close-knit pair to make room for a different group scores
    /// worse the instant the pair is broken up, before either half gains
    /// anything, so single-guest swaps and joins never take that first step.
    ///
    /// `p`'s cluster is `p` plus every other occupant of `p`'s table who
    /// shares a group with `p`; `q`'s cluster is defined symmetrically on
    /// `q`'s (different) table. The two clusters swap tables: each first
    /// takes the seats the other vacated, then any remaining free seats in
    /// ascending order, if it is the larger cluster. Rejected if either
    /// destination table lacks room, if `may_sit_at` disallows any member of
    /// either cluster at the other's table (locks, `table_type`), or if the
    /// exchange would be a pure relabel (both clusters are a whole,
    /// same-type table — `propose_table_swap` already covers that case with
    /// no scoring gain).
    ///
    /// Cluster membership is entirely group-based, which is both the
    /// mechanism and its limitation: a group shared by everyone at a table
    /// makes the "cluster" the whole table (fine — that degenerates to a
    /// whole-table exchange), while a single locked-table or locked-seat
    /// guest sharing a group with `p` or `q` makes `may_sit_at` reject the
    /// whole cluster, blocking the move even though the other members could
    /// otherwise move freely.
    fn propose_cluster_exchange(&mut self, rng: &mut StdRng, moves: &mut Vec<Move>) -> bool {
        let n = self.positions.len();
        let p = rng.random_range(0..n);
        let q = rng.random_range(0..n);
        let table_a = self.positions[p].0;
        let table_b = self.positions[q].0;
        if table_a == table_b {
            return false;
        }

        // A plain `&[Person]` local (rather than a `&self` method) so the
        // compiler sees the filters below borrow only `people`, not all of
        // `self` — which would otherwise conflict with the disjoint
        // `self.cluster_a`/`self.cluster_b` mutation in the same statement.
        let people = self.people;
        let shares_group = |a: usize, b: usize| {
            people[a]
                .groups
                .iter()
                .any(|g| people[b].groups.contains(g))
        };

        self.cluster_a.clear();
        self.cluster_a.extend(
            self.seats[table_a - 1]
                .iter()
                .flatten()
                .copied()
                .filter(|&person| person == p || shares_group(p, person)),
        );
        self.cluster_b.clear();
        self.cluster_b.extend(
            self.seats[table_b - 1]
                .iter()
                .flatten()
                .copied()
                .filter(|&person| person == q || shares_group(q, person)),
        );
        let (k1, k2) = (self.cluster_a.len(), self.cluster_b.len());

        let occ_a = self.seats[table_a - 1].iter().flatten().count();
        let occ_b = self.seats[table_b - 1].iter().flatten().count();
        // Same-type tables are score-identical (same shape/max/min/recommended),
        // so exchanging their whole occupant sets is a pure relabel with no
        // scoring gain — `propose_table_swap` already refuses this for the
        // same reason.
        if k1 == occ_a
            && k2 == occ_b
            && self.instances[table_a - 1].table_type == self.instances[table_b - 1].table_type
        {
            return false;
        }
        if occ_a - k1 + k2 > self.instances[table_a - 1].max_people
            || occ_b - k2 + k1 > self.instances[table_b - 1].max_people
        {
            return false;
        }
        if self
            .cluster_a
            .iter()
            .any(|&person| !self.may_sit_at(person, table_b))
            || self
                .cluster_b
                .iter()
                .any(|&person| !self.may_sit_at(person, table_a))
        {
            return false;
        }

        // Destination seats for cluster_a (at table_b): the seats cluster_b
        // vacates there, ascending, then free seats ascending if cluster_a
        // is larger.
        self.seats_b.clear();
        self.seats_b.extend(
            self.cluster_b
                .iter()
                .map(|&person| self.positions[person].1),
        );
        if self.seats_b.len() < k1 {
            self.seats_b.extend(
                self.seats[table_b - 1]
                    .iter()
                    .enumerate()
                    .filter(|(_, occupant)| occupant.is_none())
                    .map(|(seat, _)| seat),
            );
        }
        self.seats_b.truncate(k1);

        self.seats_a.clear();
        self.seats_a.extend(
            self.cluster_a
                .iter()
                .map(|&person| self.positions[person].1),
        );
        if self.seats_a.len() < k2 {
            self.seats_a.extend(
                self.seats[table_a - 1]
                    .iter()
                    .enumerate()
                    .filter(|(_, occupant)| occupant.is_none())
                    .map(|(seat, _)| seat),
            );
        }
        self.seats_a.truncate(k2);

        for (&person, &seat) in self.cluster_a.iter().zip(self.seats_b.iter()) {
            moves.push((person, (table_b, seat)));
        }
        for (&person, &seat) in self.cluster_b.iter().zip(self.seats_a.iter()) {
            moves.push((person, (table_a, seat)));
        }
        true
    }

    /// Empty a source table by splitting its occupants across two other
    /// tables. Unlike [`Self::propose_table_swap`] (which only fires when
    /// the whole group fits one destination), this handles a group that
    /// fits neither destination alone but fits split across both — and,
    /// because the source ends up with zero occupants, it never pays a
    /// `min_people` penalty for the source along the way, letting the
    /// search cross a valley that single-person relocations can't.
    fn propose_table_split(&mut self, rng: &mut StdRng, moves: &mut Vec<Move>) -> bool {
        let table_count = self.instances.len();
        if table_count < 3 {
            return false;
        }
        let source = rng.random_range(1..=table_count);
        let seats = &self.seats;
        let occupants = |table_number: usize| seats[table_number - 1].iter().flatten().copied();
        self.shuffle_scratch.clear();
        self.shuffle_scratch.extend(occupants(source));
        let k = self.shuffle_scratch.len();
        if k < 2 {
            return false;
        }

        let b = rng.random_range(1..=table_count);
        let c = rng.random_range(1..=table_count);
        if b == source || c == source || b == c {
            return false;
        }

        let free_seats = |table_number: usize| {
            seats[table_number - 1]
                .iter()
                .enumerate()
                .filter(|(_, occupant)| occupant.is_none())
                .map(|(seat, _)| seat)
        };
        let free_b_count = free_seats(b).count();
        let free_c_count = free_seats(c).count();
        if free_b_count == 0 || free_c_count == 0 || free_b_count + free_c_count < k {
            return false;
        }

        let to_b_count = free_b_count.min(k);
        let to_c_count = k - to_b_count;
        // A whole-table move onto an empty, same-type table B is the
        // score-neutral relabel `propose_table_swap` already refuses to
        // avoid gratuitous table renumbering; only take it here when B is a
        // different (e.g. larger) table type.
        if to_c_count == 0
            && self.instances[b - 1].table_type == self.instances[source - 1].table_type
        {
            return false;
        }

        self.shuffle_scratch.shuffle(rng);
        let (to_b, to_c) = self.shuffle_scratch.split_at(to_b_count);

        if to_b.iter().any(|&person| !self.may_sit_at(person, b))
            || to_c.iter().any(|&person| !self.may_sit_at(person, c))
        {
            return false;
        }

        for (&person, seat) in to_b.iter().zip(free_seats(b)) {
            moves.push((person, (b, seat)));
        }
        for (&person, seat) in to_c.iter().zip(free_seats(c)) {
            moves.push((person, (c, seat)));
        }
        true
    }

    /// Relocate every listed guest to its new position. All old seats are
    /// vacated before any new seat is taken, so a move set may permute
    /// seats among its own members.
    fn apply(&mut self, moves: &[Move]) {
        for &(person, _) in moves {
            let (table_number, seat) = self.positions[person];
            self.seats[table_number - 1][seat] = None;
        }
        for &(person, (table_number, seat)) in moves {
            self.seats[table_number - 1][seat] = Some(person);
            self.positions[person] = (table_number, seat);
        }
    }
}

impl SeatingOptimizer for HeuristicOptimizer {
    /// Run the heuristic and return the best solutions.
    ///
    /// Runs exactly `config.attempts.max(1)` independent search chains of
    /// `config.steps` steps each, ignoring `config.time_limit_secs`. Only the
    /// top `config.solutions` solutions (by score) are returned.
    ///
    /// Each chain uses a distinct, deterministic seed derived from
    /// `config.seed` so results are reproducible — see the determinism
    /// contract documented on [`HeuristicOptimizer::optimize_timed`]; this
    /// is its untimed mode with no warm start.
    fn optimize(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
    ) -> Result<OptimizationResult, ValidationReport> {
        validate_project(project)?;
        self.run_chains(project, config, None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::make_project;
    use crate::validation::generate_table_instances;

    fn used_table_counts(positions: &[(usize, usize)]) -> HashMap<usize, usize> {
        let mut counts = HashMap::new();
        for &(table, _) in positions {
            *counts.entry(table).or_insert(0) += 1;
        }
        counts
    }

    /// 12 guests (2 locked) and one table type with `min_people: 3`, three
    /// instances. Capacity-filling fills exactly two of the three tables to
    /// their `max_people: 6` (12 people, 6+6), leaving the third empty.
    /// `open_extra_table` must fill that third table to its `min_people`
    /// (3, so `need > 1`), donating from the two full tables (each with
    /// surplus `6 - 3 = 3` above its own minimum — the source-min guard is
    /// exercised since a table can donate down to, but not below, 3).
    #[test]
    fn open_extra_table_adds_one_table_and_respects_locks_and_min() {
        let project = make_project(
            "id,name,table_type,groups,locked_table,locked_seat\n\
             lt1,LT1,,,1,\n\
             ls1,LS1,,,2,0\n\
             r1,R1,,,,\nr2,R2,,,,\nr3,R3,,,,\nr4,R4,,,,\nr5,R5,,,,\n\
             r6,R6,,,,\nr7,R7,,,,\nr8,R8,,,,\nr9,R9,,,,\nr10,R10,,,,\n",
            "left_id,right_id,score\n",
            "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nrt,round,6,,3,3,\n",
        )
        .unwrap();
        let instances = generate_table_instances(&project);
        let optimizer = HeuristicOptimizer;
        let seed = 11;

        let without = optimizer
            .random_feasible_assignment(&project, &instances, seed, false)
            .unwrap();
        let with = optimizer
            .random_feasible_assignment(&project, &instances, seed, true)
            .unwrap();

        let counts_without = used_table_counts(&without);
        let counts_with = used_table_counts(&with);
        assert_eq!(
            counts_with.len(),
            counts_without.len() + 1,
            "expected exactly one more table in use: without {counts_without:?}, with {counts_with:?}"
        );

        // Locked guests never move, with or without the extra table.
        let lt1 = project.people.iter().position(|p| p.id == "lt1").unwrap();
        let ls1 = project.people.iter().position(|p| p.id == "ls1").unwrap();
        assert_eq!(without[lt1].0, 1);
        assert_eq!(with[lt1].0, 1);
        assert_eq!(without[ls1], (2, 0));
        assert_eq!(with[ls1], (2, 0));

        // No used table is left under its min_people.
        for table in &instances {
            if let Some(&count) = counts_with.get(&table.number) {
                assert!(
                    count >= table.min_people.unwrap_or(0),
                    "table {} under min: {count} < {:?}",
                    table.number,
                    table.min_people
                );
            }
        }
    }

    #[test]
    fn stall_patience_scales_with_guests_squared_above_its_floor() {
        assert_eq!(stall_patience(81), 26_244);
        assert_eq!(stall_patience(162), 104_976);
        assert_eq!(stall_patience(0), PATIENCE_FLOOR);
        assert_eq!(stall_patience(9), PATIENCE_FLOOR);
        // 4 · 22² = 1936 is still under the floor; 4 · 23² = 2116 is not.
        assert_eq!(stall_patience(22), PATIENCE_FLOOR);
        assert_eq!(stall_patience(23), 2_116);
    }

    /// Same fixture as the first `open_extra_table` test: the capacity-filling
    /// start uses two of three tables and `lt1`/`ls1` are locked. Across many
    /// kick seeds, every kicked seating must stay valid with the locks
    /// intact, and the kicks must both change the seating and, sometimes,
    /// open the third table.
    #[test]
    fn kick_keeps_seating_valid_and_locked_guests_in_place() {
        let project = make_project(
            "id,name,table_type,groups,locked_table,locked_seat\n\
             lt1,LT1,,,1,\n\
             ls1,LS1,,,2,0\n\
             r1,R1,,,,\nr2,R2,,,,\nr3,R3,,,,\nr4,R4,,,,\nr5,R5,,,,\n\
             r6,R6,,,,\nr7,R7,,,,\nr8,R8,,,,\nr9,R9,,,,\nr10,R10,,,,\n",
            "left_id,right_id,score\n",
            "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nrt,round,6,,3,3,\n",
        )
        .unwrap();
        let instances = generate_table_instances(&project);
        let table_lookup: HashMap<usize, &TableInstance> =
            instances.iter().map(|t| (t.number, t)).collect();
        let optimizer = HeuristicOptimizer;
        let start = optimizer
            .random_feasible_assignment(&project, &instances, 11, false)
            .unwrap();
        let tables_at_start = used_table_counts(&start).len();
        let lt1 = project.people.iter().position(|p| p.id == "lt1").unwrap();
        let ls1 = project.people.iter().position(|p| p.id == "ls1").unwrap();

        let mut changed = false;
        let mut opened = false;
        for seed in 0..100u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let kicked = optimizer.kick(&project, &instances, &table_lookup, &start, &mut rng);

            let assignments = optimizer.build_assignments(&project, &instances, &kicked);
            validate_seating_solution(&project, &assignments)
                .unwrap_or_else(|report| panic!("seed {seed}: invalid kick: {report:?}"));
            assert_eq!(kicked[lt1].0, 1, "seed {seed}");
            assert_eq!(kicked[ls1], (2, 0), "seed {seed}");

            changed |= kicked != start;
            opened |= used_table_counts(&kicked).len() > tables_at_start;
        }
        assert!(changed, "no kick changed the seating");
        assert!(opened, "no kick opened the empty table");
    }

    /// Fallback: every table is already in use, so there is nothing for
    /// `open_extra_table` to open.
    #[test]
    fn open_extra_table_is_a_no_op_when_no_table_is_empty() {
        let project = make_project(
            "id,name,table_type,groups,locked_table,locked_seat\n\
             p1,P1,,,,\np2,P2,,,,\np3,P3,,,,\np4,P4,,,,\np5,P5,,,,\np6,P6,,,,\np7,P7,,,,\n",
            "left_id,right_id,score\n",
            "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nrt,round,4,,,2,\n",
        )
        .unwrap();
        let instances = generate_table_instances(&project);
        let optimizer = HeuristicOptimizer;
        let seed = 5;

        let without = optimizer
            .random_feasible_assignment(&project, &instances, seed, false)
            .unwrap();
        let with = optimizer
            .random_feasible_assignment(&project, &instances, seed, true)
            .unwrap();
        assert_eq!(
            with, without,
            "no empty table to open, so the two starts must match"
        );
    }

    /// Fallback: an empty table exists, but every other table is exactly at
    /// its own `min_people`, so no guest can be donated without violating
    /// the source's minimum.
    #[test]
    fn open_extra_table_is_a_no_op_when_no_donor_has_surplus() {
        let project = make_project(
            "id,name,table_type,groups,locked_table,locked_seat\n\
             p1,P1,,,,\np2,P2,,,,\np3,P3,,,,\np4,P4,,,,\n\
             p5,P5,,,,\np6,P6,,,,\np7,P7,,,,\np8,P8,,,,\n",
            "left_id,right_id,score\n",
            "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nrt,round,4,,4,3,\n",
        )
        .unwrap();
        let instances = generate_table_instances(&project);
        let optimizer = HeuristicOptimizer;
        let seed = 9;

        let without = optimizer
            .random_feasible_assignment(&project, &instances, seed, false)
            .unwrap();
        let with = optimizer
            .random_feasible_assignment(&project, &instances, seed, true)
            .unwrap();
        assert_eq!(
            with, without,
            "every used table is exactly at its own min, so no donor can spare a guest"
        );
    }

    /// Regression for trying only the first shuffled empty table: `special`
    /// (a 2-seat, min-2 table nobody is compatible with, since every guest is
    /// typed `rt`) can never be filled, but the empty `rt` table always can.
    /// Before the fix (trying only `empty[0]`), whenever `special` happened
    /// to shuffle first the attempt gave up instead of trying the fillable
    /// `rt` table next — looping over seeds so at least one hits that order.
    #[test]
    fn open_extra_table_tries_every_empty_table_not_just_the_first() {
        let project = make_project(
            "id,name,table_type,groups,locked_table,locked_seat\n\
             p1,P1,rt,,,\np2,P2,rt,,,\np3,P3,rt,,,\np4,P4,rt,,,\n\
             p5,P5,rt,,,\np6,P6,rt,,,\np7,P7,rt,,,\np8,P8,rt,,,\n",
            "left_id,right_id,score\n",
            "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\n\
             rt,round,6,,,3,\nspecial,round,2,,2,1,\n",
        )
        .unwrap();
        let instances = generate_table_instances(&project);
        let optimizer = HeuristicOptimizer;

        for seed in 1..=20u64 {
            let without = optimizer
                .random_feasible_assignment(&project, &instances, seed, false)
                .unwrap();
            let with = optimizer
                .random_feasible_assignment(&project, &instances, seed, true)
                .unwrap();
            let counts_without = used_table_counts(&without);
            let counts_with = used_table_counts(&with);
            assert_eq!(
                counts_with.len(),
                counts_without.len() + 1,
                "seed {seed}: expected the fillable rt table to open even when \
                 `special` shuffles first: without {counts_without:?}, with {counts_with:?}"
            );
        }
    }
}
