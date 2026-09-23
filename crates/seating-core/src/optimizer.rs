//! Optimizer abstraction and heuristic seating optimizer.
//!
//! [`SeatingOptimizer`] is the public trait that any optimizer must implement.
//! [`HeuristicOptimizer`] provides a practical default: multiple random (or
//! warm-started) restarts of late acceptance hill climbing, keeping the top-N
//! solutions.

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

/// Multi-restart late acceptance hill climbing (LAHC) optimizer.
///
/// An **approximate** metaheuristic: it returns the best solution it visits,
/// with no optimality guarantee.
///
/// **Algorithm:**
/// 1. For each restart attempt, build a structurally valid assignment with a
///    deterministic seed derived from `config.seed` (or start from the
///    warm-start solution), consolidating guests onto fewer tables when
///    `min_people` allows (see [`HeuristicOptimizer::repair_min_constraints`]).
/// 2. Run `config.steps` LAHC moves (see `lahc_search`):
///    a move is accepted when it is at least as good as the current score
///    *or* as the score held `LAHC_HISTORY_LEN` steps ago, which lets the
///    search cross score-neutral and mildly worse plateaus. Moves: guest
///    pair swap, "join" (move a guest onto the table of another guest, or
///    to another seat of their own table), whole-table occupant swap (the
///    move that opens an unused, larger table for a group), table split
///    (the move that empties a table by splitting its occupants across two
///    smaller ones, for a group that doesn't fit either alone), and cluster
///    exchange (the move that swaps a coherent multi-guest subset between
///    two tables at once, for a rearrangement that no sequence of
///    single-guest moves can reach without a strictly worse intermediate
///    state — see [`SearchState::propose_cluster_exchange`]).
/// 3. Score the best state visited and keep the top-N solutions.
///
/// **Determinism:** attempt `i` is fully determined by
/// `(config.seed, config.steps, i)` and, when given, the warm-start
/// solution; the same input always produces the same result.
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
/// - `TABLE_SWAP_MOVE_PERCENT` whole-table occupant swaps — opens an unused,
///   larger table for a group that fits it whole.
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

impl HeuristicOptimizer {
    /// Run the heuristic for up to `config.time_limit_secs`, optionally warm
    /// starting every restart attempt from `initial` instead of a random
    /// feasible assignment.
    ///
    /// **Determinism contract:** attempt `i` is fully determined by
    /// `(config.seed, config.steps, i)` (and, when given, `initial`). The
    /// time limit only changes how many attempts complete before the run
    /// stops; results are always merged in ascending attempt order via a
    /// stable sort, so a run is reproducible given the number of attempts
    /// completed — reported as [`OptimizationResult::attempts_completed`].
    /// `config.time_limit_secs == 0` runs exactly `config.attempts` attempts,
    /// same as [`SeatingOptimizer::optimize`].
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
        self.run_attempts(project, config, initial, deadline)
    }

    fn run_attempt(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
        attempt: usize,
        initial: Option<&[SeatingAssignment]>,
    ) -> Option<SeatingSolution> {
        let attempt_seed = config.seed.wrapping_add((attempt as u64) * 17);
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
            None => self.random_feasible_assignment(project, &ctx.instances, attempt_seed)?,
        };
        let improved =
            self.lahc_search(project, &ctx, config, positions, attempt_seed ^ 0xA5A5_5A5A);
        let assignments = self.build_assignments(project, &ctx.instances, &improved);
        debug_assert!(
            validate_seating_solution(project, &assignments).is_ok(),
            "lahc_search produced an illegal move: every candidate must be legal by construction"
        );
        let score = score_solution(project, &assignments, config).ok()?;
        Some(SeatingSolution { assignments, score })
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

    /// Run restart attempts in parallel batches sized to the machine's core
    /// count, until at least `config.attempts` have completed and, if
    /// `deadline` is set, until it passes. A `None` deadline stops after
    /// exactly `config.attempts.max(1)` attempts. When `initial` is `Some`,
    /// every attempt warm-starts local improvement from it (with its own
    /// per-attempt seed) instead of a fresh random feasible assignment.
    /// `project` must already be validated by the caller.
    fn run_attempts(
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
        let min_attempts = config.attempts.max(1);
        let mut best = Vec::new();
        let mut next_attempt = 0usize;

        loop {
            if next_attempt >= min_attempts && deadline.is_none_or(|end| Instant::now() >= end) {
                break;
            }

            let batch_start = next_attempt;
            let batch_len = if next_attempt < min_attempts {
                worker_count.min(min_attempts - next_attempt)
            } else {
                worker_count
            };
            let batch_end = batch_start + batch_len;
            let project_owned = project.clone();
            let config_owned = config.clone();

            thread::scope(|scope| {
                let mut handles = Vec::with_capacity(batch_len);
                for attempt in batch_start..batch_end {
                    let project_ref = &project_owned;
                    let config_ref = &config_owned;
                    handles.push(scope.spawn(move || {
                        self.run_attempt(project_ref, config_ref, attempt, initial)
                    }));
                }
                for handle in handles {
                    if let Some(solution) = handle.join().expect("optimizer worker panicked") {
                        self.merge_solution(&mut best, solution, config.solutions);
                    }
                }
            });

            next_attempt = batch_end;
        }

        if best.is_empty() {
            return Err(ValidationReport {
                errors: vec![ValidationError::NoFeasibleAssignment],
            });
        }

        Ok(OptimizationResult {
            solutions: best,
            attempts_completed: next_attempt,
        })
    }

    /// Construct a random, structurally valid seating assignment as
    /// `positions[i] = (table_number, seat_index)` of `project.people[i]`.
    ///
    /// Returns `None` only when locks conflict or a guest has no candidate
    /// seat at all; an under-`min_people` start is returned as is (it is
    /// penalized by scoring, not rejected).
    fn random_feasible_assignment(
        &self,
        project: &ProjectInput,
        instances: &[TableInstance],
        seed: u64,
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

        project
            .people
            .iter()
            .map(|p| assigned.get(&p.id).copied())
            .collect()
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
                .filter(|p| p.locked_table.is_none())
                .filter(|p| {
                    assigned
                        .get(&p.id)
                        .map(|(t, _)| *t != table_num)
                        .unwrap_or(false)
                })
                .filter(|p| {
                    p.table_type
                        .as_ref()
                        .map(|tt| tt == &table.table_type)
                        .unwrap_or(true)
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

    /// Late acceptance hill climbing over `positions` (person-indexed
    /// `(table_number, seat_index)`), returning the best state visited.
    ///
    /// Each step proposes one move — pair swap, join, whole-table swap, or
    /// table split (see [`SearchState`]) — skips it if structurally illegal, and
    /// otherwise accepts it when the new score is at least the current one
    /// or at least the score recorded [`LAHC_HISTORY_LEN`] steps earlier.
    /// Every candidate is legal by construction (locks, `table_type`,
    /// capacity, no double booking), so scoring skips validation.
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
    ) -> Vec<(usize, usize)> {
        if positions.is_empty() {
            return positions;
        }
        let mut rng = StdRng::seed_from_u64(seed);
        let mut state = SearchState::new(&project.people, &ctx.instances, positions);
        let mut scratch: Vec<Vec<usize>> = Vec::new();
        let mut current = ctx.score_positions(&state.positions, config, &mut scratch);
        let mut best = state.positions.clone();
        let mut best_score = current;
        let mut history = vec![current; LAHC_HISTORY_LEN];
        let mut moves: Vec<Move> = Vec::new();
        let mut undo: Vec<Move> = Vec::new();

        for step in 0..config.steps {
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
            let score = ctx.score_positions(&state.positions, config, &mut scratch);
            if score >= current || score >= history[slot] {
                current = score;
                if score > best_score {
                    best_score = score;
                    best.copy_from_slice(&state.positions);
                }
            } else {
                state.apply(&undo);
            }
            history[slot] = current;
        }
        best
    }
}

/// Mutable state of one LAHC attempt: `positions[i]` is person `i`'s
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
    /// destination table lacks room, or if `may_sit_at` disallows any
    /// member of either cluster at the other's table (locks, `table_type`).
    fn propose_cluster_exchange(&mut self, rng: &mut StdRng, moves: &mut Vec<Move>) -> bool {
        let n = self.positions.len();
        let p = rng.random_range(0..n);
        let q = rng.random_range(0..n);
        let table_a = self.positions[p].0;
        let table_b = self.positions[q].0;
        if table_a == table_b {
            return false;
        }

        // A plain `&[Person]` local (rather than calling `self.shares_group`
        // from the closures below) so the compiler sees these filters borrow
        // only `people`, not all of `self` — which would otherwise conflict
        // with the disjoint `self.cluster_a`/`self.cluster_b` mutation.
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
    /// Run the multi-restart heuristic and return the best solutions.
    ///
    /// Runs exactly `config.attempts.max(1)` independent random restarts.
    /// Each attempt applies `config.steps` local improvement steps. Only the
    /// top `config.solutions` solutions (by score) are returned.
    ///
    /// Each attempt uses a distinct, deterministic seed derived from
    /// `config.seed` so results are reproducible — see the determinism
    /// contract documented on [`HeuristicOptimizer::optimize_timed`], which
    /// this delegates to with no warm start and no deadline.
    fn optimize(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
    ) -> Result<OptimizationResult, ValidationReport> {
        validate_project(project)?;
        self.run_attempts(project, config, None, None)
    }
}
