//! Seat-distance computation, proximity weighting, and seating score calculation.
//!
//! The scoring model works in two stages:
//!
//! 1. **Effective pair score** – resolves person-pair and group-pair closeness
//!    rules into a single scalar via [`effective_person_pair_score`].
//! 2. **Solution score** – sums weighted same-table pair scores via
//!    [`score_solution`], then applies global table-use and occupancy penalties.

use crate::models::{
    OptimizationConfig, Person, ProjectInput, SeatingAssignment, TableInstance, TableShape,
    ValidationError, ValidationReport,
};
use crate::validation::{
    build_closeness_lookup, canonical_pair, generate_table_instances, validate_seating_solution,
};
use std::collections::{BTreeMap, HashMap};

// ── Distance functions ────────────────────────────────────────────────────────

/// Circular distance between seats `a` and `b` on a table with `seats` positions.
///
/// Seats are treated as equally spaced around a circle, so the distance is the
/// shorter of the clockwise and counter-clockwise paths.
///
/// # Example
/// ```
/// use seating_core::scoring::circular_distance;
/// // Round table with 5 seats: A=0, B=1, C=2, D=3, E=4
/// assert_eq!(circular_distance(0, 4, 5), 1); // wrap-around
/// assert_eq!(circular_distance(0, 2, 5), 2);
/// ```
pub fn circular_distance(a: usize, b: usize, seats: usize) -> usize {
    let raw = a.abs_diff(b);
    raw.min(seats.saturating_sub(raw))
}

/// Perimeter distance for rectangular or square tables.
///
/// Currently delegates to [`circular_distance`], treating seats as ordered
/// around the table perimeter.  The API is kept separate so a geometry-aware
/// implementation can be substituted later without changing callers.
pub fn perimeter_distance(a: usize, b: usize, seats: usize) -> usize {
    circular_distance(a, b, seats)
}

/// Linear distance between seats `a` and `b`, with no wrap-around.
///
/// Used for semicircle tables, where seats sit only on the arc and the two
/// ends are not adjacent (unlike a full circle).
///
/// # Example
/// ```
/// use seating_core::scoring::linear_distance;
/// assert_eq!(linear_distance(0, 3), 3);
/// ```
pub fn linear_distance(a: usize, b: usize) -> usize {
    a.abs_diff(b)
}

/// Seat distance for `shape`, dispatching to the geometry-appropriate
/// distance function. Shared by [`score_solution_breakdown`] and the
/// optimizer's `ScoringContext::score_positions` so both agree on the
/// per-shape distance rule.
pub fn seat_distance(shape: &TableShape, a: usize, b: usize, seats: usize) -> usize {
    match shape {
        TableShape::Round => circular_distance(a, b, seats),
        TableShape::Rectangular | TableShape::Square => perimeter_distance(a, b, seats),
        TableShape::Semicircle => linear_distance(a, b),
    }
}

// ── Proximity weighting ───────────────────────────────────────────────────────

/// Default seat-proximity weight profile for pairwise scoring.
///
/// | distance | weight |
/// |----------|--------|
/// | 0 or 1   | 1.0    |
/// | 2        | 0.75   |
/// | 3        | 0.5    |
/// | d > 3    | 1/d    |
///
/// Distances 0–1 are treated as immediate neighbors (full weight).  A smooth
/// decay is applied for distances 2–3, and for larger distances we use inverse
/// distance so influence trends toward zero while remaining positive.
pub fn default_proximity_weight(distance: usize) -> f64 {
    match distance {
        0 | 1 => 1.0,
        2 => 0.75,
        3 => 0.5,
        d => 1.0 / d as f64,
    }
}

// ── Pair-level scoring ────────────────────────────────────────────────────────

/// Resolve the effective closeness contribution between `a` and `b` given an
/// arbitrary pair-score lookup function.
///
/// The result is the sum of:
/// 1. The direct person-pair closeness (if a rule exists for `(a.id, b.id)`).
/// 2. Every group-pair score across all group combinations of `a` and `b`,
///    counted once per unordered group pair (each shared group's self-rule
///    `G,G,s`, and any cross-group rule `G,H,s` for group pairs the two
///    people are in) — overlapping rules accumulate rather than the
///    strongest one winning, so e.g. sharing both a venue group and a family
///    group counts both, and a keep-apart rule (e.g. -10) nets against a
///    positive group rule instead of being outvoted by it.
///
/// If no matching rules exist the score is 0.
fn resolve_pair_score(a: &Person, b: &Person, lookup: impl Fn(&str, &str) -> Option<f64>) -> f64 {
    let mut score = lookup(&a.id, &b.id).unwrap_or(0.0);
    for ga in &a.groups {
        for gb in &b.groups {
            // When both people belong to both `ga` and `gb`, the unordered
            // pair `{ga, gb}` is enumerated twice (once as `(ga, gb)`, once
            // as `(gb, ga)`); skip the mirrored, higher-ordered visit so a
            // cross-group rule is only counted once.
            if ga > gb && a.groups.contains(gb) && b.groups.contains(ga) {
                continue;
            }
            if let Some(gs) = lookup(ga, gb) {
                score += gs;
            }
        }
    }
    score
}

/// Compute the effective closeness score between two guests using a pre-built lookup.
///
/// See [`resolve_pair_score`] for the group-resolution rule.
fn pair_score_with_lookup(
    closeness: &HashMap<(String, String), f64>,
    a: &Person,
    b: &Person,
) -> f64 {
    resolve_pair_score(a, b, |x, y| closeness.get(&canonical_pair(x, y)).copied())
}

/// Compute the effective closeness score between two guests.
///
/// Builds the closeness lookup from `project.closeness_rules` on every call.
/// Prefer [`score_solution`] for bulk scoring since it precomputes the lookup
/// once and reuses it across all pairs.
///
/// # Errors
/// Returns [`ValidationError::DuplicateClosenessRule`] if the rules list
/// contains contradictory entries for the same pair.
pub fn effective_person_pair_score(
    project: &ProjectInput,
    a: &Person,
    b: &Person,
) -> Result<f64, ValidationError> {
    let closeness = build_closeness_lookup(&project.closeness_rules)?;
    Ok(pair_score_with_lookup(&closeness, a, b))
}

// ── Solution-level scoring ────────────────────────────────────────────────────

/// Component breakdown of [`score_solution`]'s total.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreBreakdown {
    /// Sum of same-table pairwise closeness contributions
    /// (`pair_score × proximity_weight(distance) × config.proximity_weight`).
    pub proximity: f64,
    /// `used_table_count × config.used_table_weight` — the magnitude
    /// subtracted from `total` for using tables at all.
    pub used_table_penalty: f64,
    /// Sum over tables of `|occupancy - recommended_people| × config.optimal_table_size_weight`
    /// — the magnitude subtracted from `total` for deviating from recommended occupancy.
    pub size_penalty: f64,
    /// Sum over used tables of `(min_people - occupancy) × config.min_people_weight`
    /// for tables below their `min_people` — the magnitude subtracted from
    /// `total` for violating the (soft) minimum occupancy.
    pub min_people_penalty: f64,
    /// `proximity - used_table_penalty - size_penalty - min_people_penalty`,
    /// computed via the exact same running-accumulator sequence as the
    /// current `score_solution` body (not recombined from the fields above
    /// at the end) so floating-point rounding is unchanged run to run.
    pub total: f64,
}

/// Compute the aggregate score for a complete seating arrangement, broken
/// down into its components.
///
/// For each pair of guests at the **same** table the contribution is:
/// ```text
/// effective_pair_score × proximity_weight(seat_distance) × config.proximity_weight
/// ```
///
/// Pairs at **different** tables contribute 0 regardless of their closeness.
///
/// Additional global penalties are applied for every used table and for each
/// used table where the occupancy deviates from `recommended_people`:
/// ```text
/// table_penalty = used_table_count × config.used_table_weight
/// size_penalty = |occupancy - recommended_people| × config.optimal_table_size_weight
/// ```
///
/// The closeness lookup is built **once** and reused for all pair evaluations,
/// making this O(rules + n²) rather than O(n² × rules).
///
/// # Errors
/// Returns a [`ValidationReport`] if the solution or project is invalid.
pub fn score_solution_breakdown(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    config: &OptimizationConfig,
) -> Result<ScoreBreakdown, ValidationReport> {
    validate_seating_solution(project, assignments)?;

    // Build the closeness lookup once for all pair evaluations.
    let closeness = build_closeness_lookup(&project.closeness_rules)
        .map_err(|e| ValidationReport { errors: vec![e] })?;

    let instances = generate_table_instances(project);
    let table_by_number: HashMap<usize, &TableInstance> =
        instances.iter().map(|t| (t.number, t)).collect();
    let person_map: HashMap<&str, &Person> =
        project.people.iter().map(|p| (p.id.as_str(), p)).collect();

    // A `BTreeMap` (rather than `HashMap`) keeps table iteration order
    // deterministic across runs; `HashMap`'s per-process random seed would
    // otherwise let f64 addition (non-associative) sum pairs in a different
    // order each run, producing bitwise-different scores for the same input.
    let mut by_table: BTreeMap<usize, Vec<&SeatingAssignment>> = BTreeMap::new();
    for a in assignments {
        by_table.entry(a.table_number).or_default().push(a);
    }

    let mut total = 0.0;
    let mut proximity = 0.0;

    // Pairwise same-table score.
    for seated in by_table.values() {
        for i in 0..seated.len() {
            for j in (i + 1)..seated.len() {
                let a = seated[i];
                let b = seated[j];
                let pa = person_map[a.person_id.as_str()];
                let pb = person_map[b.person_id.as_str()];
                let pair_score = pair_score_with_lookup(&closeness, pa, pb);
                let table = table_by_number[&a.table_number];
                let distance =
                    seat_distance(&table.shape, a.seat_index, b.seat_index, table.max_people);
                let contribution =
                    pair_score * default_proximity_weight(distance) * config.proximity_weight;
                total += contribution;
                proximity += contribution;
            }
        }
    }

    let used_table_penalty = by_table.len() as f64 * config.used_table_weight;
    total -= used_table_penalty;

    // Soft penalty for deviation from recommended occupancy.
    let mut size_penalty = 0.0;
    for table in &instances {
        if let Some(recommended) = table.recommended_people {
            let count = by_table.get(&table.number).map(|v| v.len()).unwrap_or(0);
            if count > 0 {
                let deviation = (count as isize - recommended as isize).unsigned_abs() as f64
                    * config.optimal_table_size_weight;
                total -= deviation;
                size_penalty += deviation;
            }
        }
    }

    // Soft penalty for used tables below their min_people.
    let mut min_people_penalty = 0.0;
    for table in &instances {
        if let Some(min) = table.min_people {
            let count = by_table.get(&table.number).map(|v| v.len()).unwrap_or(0);
            if count > 0 && count < min {
                let penalty = (min - count) as f64 * config.min_people_weight;
                total -= penalty;
                min_people_penalty += penalty;
            }
        }
    }

    Ok(ScoreBreakdown {
        proximity,
        used_table_penalty,
        size_penalty,
        min_people_penalty,
        total,
    })
}

/// Compute the aggregate score for a complete seating arrangement.
///
/// See [`score_solution_breakdown`] for the component breakdown; this
/// function returns just the `total`.
pub fn score_solution(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    config: &OptimizationConfig,
) -> Result<f64, ValidationReport> {
    score_solution_breakdown(project, assignments, config).map(|b| b.total)
}

// ── Optimizer-internal scoring context ────────────────────────────────────────

/// Canonically order `a`/`b` without allocating, unlike [`canonical_pair`].
fn canonical_pair_ref<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if a <= b { (a, b) } else { (b, a) }
}

/// Precomputed, allocation-light scoring context reused across every
/// accept/reject decision within one optimizer attempt.
///
/// Building this once per attempt — rather than once per step — avoids
/// regenerating table instances and resolving closeness rules per pair:
/// every effective pair score is resolved once into `pair_matrix`, so a
/// step costs one table lookup and one `f64` read per same-table pair.
/// [`Self::score_positions`] intentionally does **not** validate: callers
/// (`HeuristicOptimizer`) must only ever present structurally valid moves.
pub(crate) struct ScoringContext<'p> {
    /// Table instances in number order; instance `number` lives at `number - 1`.
    pub(crate) instances: Vec<TableInstance>,
    /// Person id → index into `project.people` (the index used by `positions`).
    pub(crate) person_index: HashMap<&'p str, usize>,
    /// `n × n` row-major effective pair scores, symmetric.
    pair_matrix: Vec<f64>,
}

impl<'p> ScoringContext<'p> {
    /// Build the context for `project`. `project` is assumed already valid
    /// (callers run [`crate::validation::validate_project`] beforehand), so
    /// closeness rules are assumed duplicate-free.
    pub(crate) fn build(project: &'p ProjectInput) -> Self {
        let instances = generate_table_instances(project);
        debug_assert!(
            instances
                .iter()
                .enumerate()
                .all(|(index, table)| table.number == index + 1),
            "table instances must be numbered 1..=len sequentially"
        );
        let person_index = project
            .people
            .iter()
            .enumerate()
            .map(|(index, p)| (p.id.as_str(), index))
            .collect();
        let mut closeness = HashMap::new();
        for rule in &project.closeness_rules {
            let key = canonical_pair_ref(&rule.left_id, &rule.right_id);
            closeness.insert(key, rule.score);
        }

        let people = &project.people;
        let n = people.len();
        let mut pair_matrix = vec![0.0; n * n];
        for i in 0..n {
            for j in (i + 1)..n {
                let score = resolve_pair_score(&people[i], &people[j], |x, y| {
                    closeness.get(&canonical_pair_ref(x, y)).copied()
                });
                pair_matrix[i * n + j] = score;
                pair_matrix[j * n + i] = score;
            }
        }
        Self {
            instances,
            person_index,
            pair_matrix,
        }
    }

    /// Score a solution given as `positions[i] = (table_number, seat_index)`
    /// of person `i` (index into `project.people`). Mirrors
    /// [`score_solution`]'s math and summation order — tables ascending,
    /// pairs in person order — so both produce bitwise-identical totals.
    ///
    /// `scratch` is a caller-owned, per-table occupant buffer reused across
    /// calls (e.g. every LAHC step) to avoid reallocating one `Vec` per table
    /// on every call; its contents on entry are irrelevant, it is cleared
    /// and repopulated here.
    pub(crate) fn score_positions(
        &self,
        positions: &[(usize, usize)],
        config: &OptimizationConfig,
        scratch: &mut Vec<Vec<usize>>,
    ) -> f64 {
        let n = positions.len();
        scratch.resize_with(self.instances.len(), Vec::new);
        for bucket in scratch.iter_mut() {
            bucket.clear();
        }
        for (person, &(table_number, _)) in positions.iter().enumerate() {
            scratch[table_number - 1].push(person);
        }

        let mut total = 0.0;
        for (table, seated) in self.instances.iter().zip(scratch.iter()) {
            for (k, &i) in seated.iter().enumerate() {
                for &j in &seated[k + 1..] {
                    let (seat_i, seat_j) = (positions[i].1, positions[j].1);
                    let distance = seat_distance(&table.shape, seat_i, seat_j, table.max_people);
                    total += self.pair_matrix[i * n + j]
                        * default_proximity_weight(distance)
                        * config.proximity_weight;
                }
            }
        }

        let used_tables = scratch.iter().filter(|seated| !seated.is_empty()).count();
        total -= used_tables as f64 * config.used_table_weight;

        for (table, seated) in self.instances.iter().zip(scratch.iter()) {
            if let Some(recommended) = table.recommended_people
                && !seated.is_empty()
            {
                total -= (seated.len() as isize - recommended as isize).unsigned_abs() as f64
                    * config.optimal_table_size_weight;
            }
        }

        for (table, seated) in self.instances.iter().zip(scratch.iter()) {
            if let Some(min) = table.min_people
                && !seated.is_empty()
                && seated.len() < min
            {
                total -= (min - seated.len()) as f64 * config.min_people_weight;
            }
        }

        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::make_project;

    /// The optimizer's hot-path scorer must agree bitwise with the public
    /// validated scorer on every term — including the soft min-people
    /// penalty, which only `score_positions` sees during a search.
    #[test]
    fn score_positions_matches_score_solution_including_min_people_penalty() {
        let project = make_project(
            "id,name,table_type,groups,locked_table,locked_seat\np1,A,,,,\np2,B,,,,\np3,C,,,,\n",
            "left_id,right_id,score\np1,p2,5\n",
            "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,3,2,2,\n",
        )
        .unwrap();
        let config = OptimizationConfig {
            used_table_weight: 2.0,
            ..OptimizationConfig::default()
        };
        // p1, p2 adjacent on table 1; p3 alone on table 2 (below min 2).
        let positions = [(1, 0), (1, 1), (2, 0)];
        let assignments: Vec<SeatingAssignment> = project
            .people
            .iter()
            .zip(positions)
            .map(|(p, (table_number, seat_index))| SeatingAssignment {
                table_number,
                table_type: "round_4".to_string(),
                seat_index,
                person_id: p.id.clone(),
                person_name: p.name.clone(),
            })
            .collect();

        let expected = score_solution(&project, &assignments, &config).unwrap();
        let actual =
            ScoringContext::build(&project).score_positions(&positions, &config, &mut Vec::new());

        // proximity 5 (distance 1) - used tables 2 * 2.0 - size |2-3| + |1-3|
        // = 3 - min shortfall 1 * 1000.0
        assert_eq!(expected, 5.0 - 4.0 - 3.0 - 1000.0);
        assert_eq!(actual, expected);
    }
}
