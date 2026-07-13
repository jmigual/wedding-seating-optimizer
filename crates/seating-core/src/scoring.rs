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
/// 2. The group-pair score with the **largest absolute value** across all
///    group combinations of `a` and `b` (this ensures a strong keep-apart
///    rule, e.g. -100, is not silently outvoted by a weaker positive one,
///    e.g. +10 — the most *significant* group relationship dominates, not
///    merely the most positive one).
///
/// If no matching rules exist the score is 0.
fn resolve_pair_score(a: &Person, b: &Person, lookup: impl Fn(&str, &str) -> Option<f64>) -> f64 {
    let mut score = lookup(&a.id, &b.id).unwrap_or(0.0);
    let mut best_group: Option<f64> = None;
    for ga in &a.groups {
        for gb in &b.groups {
            if let Some(gs) = lookup(ga, gb) {
                best_group = Some(match best_group {
                    Some(current) if current.abs() >= gs.abs() => current,
                    _ => gs,
                });
            }
        }
    }
    if let Some(gs) = best_group {
        score += gs;
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

/// Compute the aggregate score for a complete seating arrangement.
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
pub fn score_solution(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    config: &OptimizationConfig,
) -> Result<f64, ValidationReport> {
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
                let distance = match table.shape {
                    TableShape::Round => {
                        circular_distance(a.seat_index, b.seat_index, table.max_people)
                    }
                    TableShape::Rectangular | TableShape::Square => {
                        perimeter_distance(a.seat_index, b.seat_index, table.max_people)
                    }
                };
                total += pair_score * default_proximity_weight(distance) * config.proximity_weight;
            }
        }
    }

    total -= by_table.len() as f64 * config.used_table_weight;

    // Soft penalty for deviation from recommended occupancy.
    for table in &instances {
        if let Some(recommended) = table.recommended_people {
            let count = by_table.get(&table.number).map(|v| v.len()).unwrap_or(0);
            if count > 0 {
                total -= (count as isize - recommended as isize).unsigned_abs() as f64
                    * config.optimal_table_size_weight;
            }
        }
    }

    Ok(total)
}

// ── Optimizer-internal scoring context ────────────────────────────────────────

/// Canonically order `a`/`b` without allocating, unlike [`canonical_pair`].
fn canonical_pair_ref<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Precomputed, allocation-light scoring context reused across every
/// accept/reject decision within one optimizer attempt.
///
/// Building this once per attempt — rather than once per iteration — avoids
/// regenerating table instances, rebuilding the closeness lookup (keyed by
/// borrowed `&str`s instead of allocated `String`s), and re-running full
/// project validation on every hill-climbing step. [`Self::score`]
/// intentionally does **not** validate: callers (`HeuristicOptimizer`) must
/// only ever present structurally valid candidate moves.
pub(crate) struct ScoringContext<'p> {
    pub(crate) instances: Vec<TableInstance>,
    pub(crate) table_by_number: HashMap<usize, TableInstance>,
    pub(crate) person_map: HashMap<&'p str, &'p Person>,
    closeness: HashMap<(&'p str, &'p str), f64>,
}

impl<'p> ScoringContext<'p> {
    /// Build the context for `project`. `project` is assumed already valid
    /// (callers run [`crate::validation::validate_project`] beforehand), so
    /// closeness rules are assumed duplicate-free.
    pub(crate) fn build(project: &'p ProjectInput) -> Self {
        let instances = generate_table_instances(project);
        let table_by_number = instances.iter().map(|t| (t.number, t.clone())).collect();
        let person_map = project.people.iter().map(|p| (p.id.as_str(), p)).collect();
        let mut closeness = HashMap::new();
        for rule in &project.closeness_rules {
            let key = canonical_pair_ref(&rule.left_id, &rule.right_id);
            closeness.insert(key, rule.score);
        }
        Self {
            instances,
            table_by_number,
            person_map,
            closeness,
        }
    }

    fn pair_score(&self, a: &Person, b: &Person) -> f64 {
        resolve_pair_score(a, b, |x, y| {
            self.closeness.get(&canonical_pair_ref(x, y)).copied()
        })
    }

    /// Score `assignments` using the precomputed context. Mirrors
    /// [`score_solution`]'s math without validating the project or solution.
    pub(crate) fn score(
        &self,
        assignments: &[SeatingAssignment],
        config: &OptimizationConfig,
    ) -> f64 {
        let mut by_table: BTreeMap<usize, Vec<&SeatingAssignment>> = BTreeMap::new();
        for a in assignments {
            by_table.entry(a.table_number).or_default().push(a);
        }

        let mut total = 0.0;
        for seated in by_table.values() {
            for i in 0..seated.len() {
                for j in (i + 1)..seated.len() {
                    let a = seated[i];
                    let b = seated[j];
                    let pa = self.person_map[a.person_id.as_str()];
                    let pb = self.person_map[b.person_id.as_str()];
                    let pair_score = self.pair_score(pa, pb);
                    let table = &self.table_by_number[&a.table_number];
                    let distance = match table.shape {
                        TableShape::Round => {
                            circular_distance(a.seat_index, b.seat_index, table.max_people)
                        }
                        TableShape::Rectangular | TableShape::Square => {
                            perimeter_distance(a.seat_index, b.seat_index, table.max_people)
                        }
                    };
                    total +=
                        pair_score * default_proximity_weight(distance) * config.proximity_weight;
                }
            }
        }

        total -= by_table.len() as f64 * config.used_table_weight;

        // Deterministic table iteration order (`self.instances` is a `Vec`)
        // for the same non-associativity reason as `score_solution`.
        for table in &self.instances {
            if let Some(recommended) = table.recommended_people {
                let count = by_table.get(&table.number).map(|v| v.len()).unwrap_or(0);
                if count > 0 {
                    total -= (count as isize - recommended as isize).unsigned_abs() as f64
                        * config.optimal_table_size_weight;
                }
            }
        }

        total
    }
}
