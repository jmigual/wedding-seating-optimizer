//! Optimizer abstraction and heuristic seating optimizer.
//!
//! [`SeatingOptimizer`] is the public trait that any optimizer must implement.
//! [`HeuristicOptimizer`] provides a practical default: it performs multiple
//! random restarts with local-swap hill-climbing and keeps the top-N solutions.

use crate::models::{
    OptimizationConfig, OptimizationResult, Person, ProjectInput, SeatingAssignment,
    SeatingSolution, TableInstance, ValidationError, ValidationReport,
};
use crate::scoring::{score_solution, ScoringContext};
use crate::validation::{generate_table_instances, validate_project};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

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

/// Multi-restart hill-climbing optimizer.
///
/// **Algorithm:**
/// 1. For each of `config.attempts` independent random restarts, build a
///    feasible assignment with a deterministic seed derived from `config.seed`,
///    repairing any `min_people` violations by consolidating guests onto
///    fewer tables (see [`HeuristicOptimizer::repair_min_constraints`]).
/// 2. Apply local improvement: `config.iterations` greedy pairwise-swap
///    steps, followed by `config.iterations` greedy single-guest relocation
///    steps (see [`HeuristicOptimizer::local_improve`]).
/// 3. Score the result and keep the top-N solutions.
///
/// Reproducibility is guaranteed: the same `seed` always produces the same
/// result for the same input.
#[derive(Debug, Default)]
pub struct HeuristicOptimizer;

impl HeuristicOptimizer {
    /// Construct a random, constraint-satisfying seating assignment.
    ///
    /// Returns `None` if no feasible assignment can be found for the given seed
    /// (e.g., due to tight min-capacity constraints).
    fn random_feasible_assignment(
        &self,
        project: &ProjectInput,
        seed: u64,
    ) -> Option<Vec<SeatingAssignment>> {
        let instances = generate_table_instances(project);
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

        for p in pending {
            let candidates = self.seat_candidates(p, &instances, &occupied);
            if candidates.is_empty() {
                return None;
            }
            let chosen = candidates[rng.gen_range(0..candidates.len())];
            occupied.insert(chosen);
            assigned.insert(p.id.clone(), chosen);
        }

        // Uniform random placement is the worst strategy for min_people: it
        // spreads guests thinly across every table. Rather than rejecting
        // the whole attempt, deterministically repair under-min tables by
        // consolidating guests (respecting locks and table_type).
        if !self.satisfies_min_constraints(&assigned, &instances) {
            let repaired = self.repair_min_constraints(
                project,
                &instances,
                &table_lookup,
                &mut assigned,
                &mut occupied,
            );
            if !repaired {
                return None; // Genuinely unrepairable.
            }
        }

        Some(self.build_assignments(project, &assigned, &table_lookup))
    }

    /// Deterministically repair `min_people` violations by consolidating
    /// guests onto fewer tables.
    ///
    /// Repeatedly resolves the smallest deficient (used but under-min) table
    /// by either vacating it entirely (relocating every occupant elsewhere,
    /// respecting locks and required `table_type`) or, when it has guests
    /// locked to it, topping it up from other tables instead. Uses no
    /// randomness, so seed-determinism is preserved. Returns `false` only
    /// when the deficiency is genuinely unrepairable (not enough compatible
    /// guests or seats to consolidate around).
    fn repair_min_constraints(
        &self,
        project: &ProjectInput,
        instances: &[TableInstance],
        table_lookup: &HashMap<usize, &TableInstance>,
        assigned: &mut HashMap<String, (usize, usize)>,
        occupied: &mut HashSet<(usize, usize)>,
    ) -> bool {
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
                return true; // No deficiencies left.
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
                return false; // Genuinely unrepairable.
            }
        }
        false
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

    /// Convert the internal assignment map to the public [`SeatingAssignment`] format.
    fn build_assignments(
        &self,
        project: &ProjectInput,
        assigned: &HashMap<String, (usize, usize)>,
        table_lookup: &HashMap<usize, &TableInstance>,
    ) -> Vec<SeatingAssignment> {
        project
            .people
            .iter()
            .map(|p| {
                let &(table_number, seat_index) =
                    assigned.get(&p.id).expect("every person was assigned");
                SeatingAssignment {
                    table_number,
                    table_type: table_lookup[&table_number].table_type.clone(),
                    seat_index,
                    person_id: p.id.clone(),
                    person_name: p.name.clone(),
                }
            })
            .collect()
    }

    /// Apply local improvement: greedy pairwise-swap steps followed by greedy
    /// single-guest relocation steps.
    ///
    /// **Phase 1 (swap):** at each of `config.iterations` steps, a random
    /// pair of guests is chosen and their table/seat positions are swapped if
    /// the swap improves the score and remains feasible.
    ///
    /// **Phase 2 (relocate):** at each of `config.iterations` steps, a random
    /// guest is moved to a random compatible free seat if the move improves
    /// the score and remains feasible.
    ///
    /// Guests with a `locked_seat` are frozen (excluded from both phases) —
    /// there is nothing to optimize for them, since their seat is fixed.
    /// Guests with only a `locked_table` (no `locked_seat`) still participate:
    /// phase 2's candidate seats are already restricted to their locked table
    /// by [`Self::seat_candidates`], and phase 1 only accepts swaps that keep
    /// each locked-table guest on their required table — so a "head table"
    /// where everyone is `locked_table`-only still gets its internal seating
    /// optimized.
    ///
    /// Every move considered here is constructed to be structurally valid
    /// (seat/table_type/lock/capacity-respecting) by construction, so this
    /// loop scores candidates directly via a [`ScoringContext`] built once
    /// per attempt, instead of re-running full project validation on every
    /// iteration.
    fn local_improve(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
        mut assignments: Vec<SeatingAssignment>,
        seed: u64,
    ) -> Vec<SeatingAssignment> {
        let mut rng = StdRng::seed_from_u64(seed);
        let ctx = ScoringContext::build(project);

        // Guests with a locked seat have exactly one legal seat: their own.
        let locked_seat_ids: HashSet<&str> = project
            .people
            .iter()
            .filter(|p| p.locked_seat.is_some())
            .map(|p| p.id.as_str())
            .collect();
        // Guests with a locked table (seat or not) must stay on that table.
        let locked_table_of: HashMap<&str, usize> = project
            .people
            .iter()
            .filter_map(|p| p.locked_table.map(|table_num| (p.id.as_str(), table_num)))
            .collect();

        let mut best_score = ctx.score(&assignments, config);

        // Phase 1: pairwise swaps. A swap never changes any table's
        // occupancy count, so it cannot violate `min_people`; only the
        // locked-table and locked-seat constraints need checking here.
        if assignments.len() >= 2 {
            for _ in 0..config.iterations {
                let i = rng.gen_range(0..assignments.len());
                let j = rng.gen_range(0..assignments.len());
                if i == j {
                    continue;
                }
                let (id_i, id_j) = (
                    assignments[i].person_id.as_str(),
                    assignments[j].person_id.as_str(),
                );
                if locked_seat_ids.contains(id_i) || locked_seat_ids.contains(id_j) {
                    continue;
                }
                let new_table_i = assignments[j].table_number;
                let new_table_j = assignments[i].table_number;
                if locked_table_of.get(id_i).is_some_and(|&t| t != new_table_i)
                    || locked_table_of.get(id_j).is_some_and(|&t| t != new_table_j)
                {
                    continue;
                }
                if let (Some(pi), Some(pj)) = (ctx.person_map.get(id_i), ctx.person_map.get(id_j)) {
                    let type_i_ok = pi
                        .table_type
                        .as_ref()
                        .is_none_or(|tt| tt == &assignments[j].table_type);
                    let type_j_ok = pj
                        .table_type
                        .as_ref()
                        .is_none_or(|tt| tt == &assignments[i].table_type);
                    if !type_i_ok || !type_j_ok {
                        continue;
                    }
                }

                let (a, b) = (assignments[i].clone(), assignments[j].clone());
                assignments[i].table_number = b.table_number;
                assignments[i].table_type = b.table_type.clone();
                assignments[i].seat_index = b.seat_index;
                assignments[j].table_number = a.table_number;
                assignments[j].table_type = a.table_type.clone();
                assignments[j].seat_index = a.seat_index;

                let score = ctx.score(&assignments, config);
                if score > best_score {
                    best_score = score;
                    continue; // Keep the swap.
                }
                assignments[i] = a;
                assignments[j] = b;
            }
        }

        // Phase 2: single-guest relocation.
        if !assignments.is_empty() {
            for _ in 0..config.iterations {
                let i = rng.gen_range(0..assignments.len());
                if locked_seat_ids.contains(assignments[i].person_id.as_str()) {
                    continue;
                }

                let mut occupied: HashSet<(usize, usize)> = HashSet::new();
                let mut counts: HashMap<usize, usize> = HashMap::new();
                for (index, assignment) in assignments.iter().enumerate() {
                    if index == i {
                        continue;
                    }
                    occupied.insert((assignment.table_number, assignment.seat_index));
                    *counts.entry(assignment.table_number).or_insert(0) += 1;
                }

                let Some(&person) = ctx.person_map.get(assignments[i].person_id.as_str()) else {
                    continue;
                };
                let candidates = self.seat_candidates(person, &ctx.instances, &occupied);
                if candidates.is_empty() {
                    continue;
                }
                let chosen = candidates[rng.gen_range(0..candidates.len())];
                let Some(dest_table) = ctx.table_by_number.get(&chosen.0) else {
                    continue;
                };

                // Preserve min_people: removing the guest must not leave the
                // source table with a non-zero, under-min remainder, and
                // adding them must bring the destination to at least min.
                let source_table_number = assignments[i].table_number;
                if let Some(source_table) = ctx.table_by_number.get(&source_table_number) {
                    if let Some(min) = source_table.min_people {
                        let remaining = counts.get(&source_table_number).copied().unwrap_or(0);
                        if remaining > 0 && remaining < min {
                            continue;
                        }
                    }
                }
                if let Some(min) = dest_table.min_people {
                    let dest_count_before = counts.get(&chosen.0).copied().unwrap_or(0);
                    if dest_count_before + 1 < min {
                        continue;
                    }
                }

                let original = assignments[i].clone();
                assignments[i].table_number = chosen.0;
                assignments[i].table_type = dest_table.table_type.clone();
                assignments[i].seat_index = chosen.1;

                let score = ctx.score(&assignments, config);
                if score > best_score {
                    best_score = score;
                    continue; // Keep the move.
                }
                assignments[i] = original;
            }
        }
        assignments
    }
}

impl SeatingOptimizer for HeuristicOptimizer {
    /// Run the multi-restart heuristic and return the best solutions.
    ///
    /// Runs `config.attempts` independent random restarts. Each attempt
    /// applies `config.iterations` local improvement steps. Only the top
    /// `config.solutions` solutions (by score) are returned.
    ///
    /// Each attempt uses a distinct, deterministic seed derived from
    /// `config.seed` so results are reproducible.
    fn optimize(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
    ) -> Result<OptimizationResult, ValidationReport> {
        validate_project(project)?;

        let mut best: Vec<SeatingSolution> = Vec::new();

        for attempt in 0..config.attempts.max(1) {
            // Derive a per-attempt seed that is deterministic and distinct.
            let attempt_seed = config.seed.wrapping_add((attempt as u64) * 17);
            let Some(initial) = self.random_feasible_assignment(project, attempt_seed) else {
                continue;
            };
            // Use a fixed bit-mixing constant to derive a deterministic but
            // decorrelated RNG stream for local improvement from the base seed.
            let improved = self.local_improve(project, config, initial, attempt_seed ^ 0xA5A5_5A5A);
            let Ok(score) = score_solution(project, &improved, config) else {
                continue;
            };
            best.push(SeatingSolution {
                assignments: improved,
                score,
            });
            best.sort_by(|a, b| b.score.total_cmp(&a.score));
            best.truncate(config.solutions.max(1));
        }

        if best.is_empty() {
            return Err(ValidationReport {
                errors: vec![ValidationError::NoFeasibleAssignment],
            });
        }

        Ok(OptimizationResult { solutions: best })
    }
}
