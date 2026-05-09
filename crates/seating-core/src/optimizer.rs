//! Optimizer abstraction and heuristic seating optimizer.
//!
//! [`SeatingOptimizer`] is the public trait that any optimizer must implement.
//! [`HeuristicOptimizer`] provides a practical default: it performs multiple
//! random restarts with local-swap hill-climbing and keeps the top-N solutions.

use crate::models::{
    OptimizationConfig, OptimizationResult, ProjectInput, SeatingAssignment, SeatingSolution,
    ValidationError, ValidationReport,
};
use crate::scoring::score_solution;
use crate::validation::{generate_table_instances, validate_project, validate_seating_solution};
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
///    feasible assignment with a deterministic seed derived from `config.seed`.
/// 2. Apply `config.iterations` greedy pairwise-swap local improvement steps.
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
        let table_lookup: HashMap<usize, &crate::models::TableInstance> =
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
        let mut pending: Vec<&crate::models::Person> = project
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

        // Reject the assignment if any table violates its min_people constraint.
        if !self.satisfies_min_constraints(&assigned, &instances) {
            return None;
        }

        Some(self.build_assignments(project, &assigned, &table_lookup))
    }

    /// Enumerate all valid (table_number, seat_index) pairs for a guest.
    fn seat_candidates(
        &self,
        p: &crate::models::Person,
        instances: &[crate::models::TableInstance],
        occupied: &HashSet<(usize, usize)>,
    ) -> Vec<(usize, usize)> {
        let mut candidates = Vec::new();
        for t in instances {
            if p.table_type.as_ref().map(|tt| tt != &t.table_type).unwrap_or(false) {
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
        instances: &[crate::models::TableInstance],
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
        table_lookup: &HashMap<usize, &crate::models::TableInstance>,
    ) -> Vec<SeatingAssignment> {
        project
            .people
            .iter()
            .map(|p| {
                let &(table_number, seat_index) = assigned.get(&p.id).expect("every person was assigned");
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

    /// Apply greedy pairwise-swap local improvement.
    ///
    /// At each of `config.iterations` steps, a random pair of non-locked guests
    /// is chosen and their seat positions are swapped if the swap improves the
    /// score and remains feasible.
    ///
    /// Guests are considered locked (and therefore excluded from swapping) when
    /// they have a `locked_table` **or** a `locked_seat`; swapping them could
    /// violate hard placement constraints.
    fn local_improve(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
        mut assignments: Vec<SeatingAssignment>,
        seed: u64,
    ) -> Vec<SeatingAssignment> {
        let mut rng = StdRng::seed_from_u64(seed);
        // Any person with a locked table or seat must not be moved.
        let locked_ids: HashSet<&str> = project
            .people
            .iter()
            .filter(|p| p.locked_seat.is_some() || p.locked_table.is_some())
            .map(|p| p.id.as_str())
            .collect();
        let mut best_score =
            score_solution(project, &assignments, config.recommended_capacity_weight).unwrap_or(f64::NEG_INFINITY);

        for _ in 0..config.iterations.max(50) {
            let i = rng.gen_range(0..assignments.len());
            let j = rng.gen_range(0..assignments.len());

            if i == j
                || locked_ids.contains(assignments[i].person_id.as_str())
                || locked_ids.contains(assignments[j].person_id.as_str())
            {
                continue;
            }

            // Swap the table/seat positions of two guests.
            let (a, b) = (assignments[i].clone(), assignments[j].clone());
            assignments[i].table_number = b.table_number;
            assignments[i].table_type = b.table_type.clone();
            assignments[i].seat_index = b.seat_index;
            assignments[j].table_number = a.table_number;
            assignments[j].table_type = a.table_type.clone();
            assignments[j].seat_index = a.seat_index;

            // Accept only improving feasible swaps.
            if validate_seating_solution(project, &assignments).is_ok() {
                if let Ok(score) = score_solution(project, &assignments, config.recommended_capacity_weight) {
                    if score > best_score {
                        best_score = score;
                        continue; // Keep the swap.
                    }
                }
            }
            // Revert.
            assignments[i] = a;
            assignments[j] = b;
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
            let Ok(score) = score_solution(project, &improved, config.recommended_capacity_weight) else {
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
                errors: vec![ValidationError::MalformedInput(
                    "unable to construct a feasible seating assignment".to_string(),
                )],
            });
        }

        Ok(OptimizationResult { solutions: best })
    }
}
