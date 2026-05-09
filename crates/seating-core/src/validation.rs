//! Project and seating-solution validation.
//!
//! [`validate_project`] checks the input configuration for hard-constraint
//! violations before optimization begins.  [`validate_seating_solution`]
//! verifies that a proposed seating arrangement is feasible.
//!
//! Both functions accumulate *all* errors before returning, so callers receive
//! a complete diagnostic list rather than failing on the first problem.
//!
//! [`generate_table_instances`] is also exposed here because it is shared
//! between validation, scoring, and the optimizer.

use crate::models::{
    ProjectInput, TableInstance, TableShape, ValidationError, ValidationReport,
    SeatingAssignment,
};
use std::collections::{HashMap, HashSet};

// ── Table instance generation ─────────────────────────────────────────────────

/// Expand the table type configurations into concrete numbered [`TableInstance`]s.
///
/// For each type:
/// - If `number_of_tables` is specified, exactly that many instances are created.
/// - Otherwise, the count defaults to `max(ceil(people / max_people), max_locked_table_number)`,
///   which is a safe upper bound ensuring every lock can be satisfied without
///   generating an excessive number of instances.
///
/// Instances are numbered sequentially (1-based). Because [`ProjectInput::table_types`]
/// is a [`std::collections::BTreeMap`], types are iterated in **lexicographic key order**,
/// so table numbers are stable and deterministic across platforms.
pub fn generate_table_instances(project: &ProjectInput) -> Vec<TableInstance> {
    let max_locked = project
        .people
        .iter()
        .filter_map(|p| p.locked_table)
        .max()
        .unwrap_or(0);
    let person_count = project.people.len().max(1);
    let mut instances = Vec::new();
    let mut number = 1usize;
    for (table_type_id, cfg) in &project.table_types {
        // When not specified, generate the minimum number of tables of this
        // type that could in theory seat everyone (ceiling division), subject
        // to honouring the highest locked-table number.
        let count = cfg.number_of_tables.unwrap_or_else(|| {
            let by_capacity = person_count.div_ceil(cfg.max_people);
            by_capacity.max(max_locked)
        });
        for _ in 0..count {
            instances.push(TableInstance {
                number,
                table_type: table_type_id.clone(),
                shape: cfg.shape.clone(),
                max_people: cfg.max_people,
                min_people: cfg.min_people,
                recommended_people: cfg.recommended_people,
            });
            number += 1;
        }
    }
    instances
}

// ── Project validation ────────────────────────────────────────────────────────

/// Validate the project inputs, returning a [`ValidationReport`] on failure.
///
/// Checks performed (all accumulated before returning):
/// - Duplicate person IDs
/// - Person ID / group ID namespace collisions
/// - Unknown table types referenced by persons
/// - Locked seat without a locked table
/// - Shape constraints (`people_per_side` required for non-round tables)
/// - `people_per_side` sum must equal `max_people`
/// - `min_people` ≤ `max_people`
/// - Unknown IDs in closeness rules
/// - Duplicate closeness rules
/// - Non-finite closeness scores
/// - Sufficient total seat capacity
/// - Valid and compatible locked table / seat assignments
/// - Seat collision among locked guests
/// - Impossible person assignments (no compatible table exists)
/// - High-priority groups larger than any compatible table
pub fn validate_project(project: &ProjectInput) -> Result<(), ValidationReport> {
    let mut errors = Vec::new();

    let (person_ids, group_ids) = collect_id_sets(project, &mut errors);
    validate_table_type_configs(project, &mut errors);
    validate_closeness_rules(project, &person_ids, &group_ids, &mut errors);

    let instances = generate_table_instances(project);
    let table_by_number: HashMap<usize, &TableInstance> = instances.iter().map(|t| (t.number, t)).collect();
    validate_capacity(&instances, project.people.len(), &mut errors);
    validate_locked_assignments(project, &table_by_number, &mut errors);
    validate_impossible_assignments(project, &instances, &mut errors);
    validate_large_groups(project, &group_ids, &instances, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationReport { errors })
    }
}

// ── Seating-solution validation ───────────────────────────────────────────────

/// Validate that a seating solution is feasible given the project configuration.
///
/// Also runs [`validate_project`] internally so that caller does not need to
/// call both functions separately.
///
/// Additional checks beyond project validation:
/// - Every person appears exactly once.
/// - All table numbers exist.
/// - No seat collisions.
/// - `SeatingAssignment.table_type` matches the resolved instance type.
/// - Person's required `table_type` (if any) is respected.
/// - Locked-table and locked-seat constraints are honoured.
/// - Occupancy bounds (`max_people`, `min_people`) are satisfied.
pub fn validate_seating_solution(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
) -> Result<(), ValidationReport> {
    let mut errors = validate_project(project).err().map(|r| r.errors).unwrap_or_default();

    let instances = generate_table_instances(project);
    let table_by_number: HashMap<usize, &TableInstance> = instances.iter().map(|t| (t.number, t)).collect();
    let person_map: HashMap<&str, &crate::models::Person> =
        project.people.iter().map(|p| (p.id.as_str(), p)).collect();

    let mut seen_people: HashSet<String> = HashSet::new();
    let mut seen_seats: HashSet<(usize, usize)> = HashSet::new();
    let mut occupancy: HashMap<usize, usize> = HashMap::new();

    for a in assignments {
        let Some(person) = person_map.get(a.person_id.as_str()).copied() else {
            errors.push(ValidationError::UnknownPersonInSeating(a.person_id.clone()));
            continue;
        };
        if !seen_people.insert(a.person_id.clone()) {
            errors.push(ValidationError::MissingOrDuplicatePerson(a.person_id.clone()));
        }
        let Some(table) = table_by_number.get(&a.table_number) else {
            errors.push(ValidationError::UnknownTableInSeating(a.table_number));
            continue;
        };

        // The table_type recorded in the assignment must match the instance.
        if a.table_type != table.table_type {
            errors.push(ValidationError::SeatingTableTypeMismatch {
                table_number: a.table_number,
                actual_type: table.table_type.clone(),
                recorded_type: a.table_type.clone(),
            });
        }

        // The person's required table_type (if any) must match.
        if let Some(required) = &person.table_type {
            if *required != table.table_type {
                errors.push(ValidationError::SeatingPersonTableTypeMismatch {
                    person_id: a.person_id.clone(),
                    table_number: a.table_number,
                    required_type: required.clone(),
                    assigned_type: table.table_type.clone(),
                });
            }
        }

        // Locked-table constraint.
        if let Some(locked_table) = person.locked_table {
            if locked_table != a.table_number {
                errors.push(ValidationError::SeatingViolatesLockedTable {
                    person_id: a.person_id.clone(),
                    locked_table,
                    assigned_table: a.table_number,
                });
            }
        }

        // Locked-seat constraint.
        if let Some(locked_seat) = person.locked_seat {
            if locked_seat != a.seat_index {
                errors.push(ValidationError::SeatingViolatesLockedSeat {
                    person_id: a.person_id.clone(),
                    locked_seat,
                    assigned_seat: a.seat_index,
                });
            }
        }

        if a.seat_index >= table.max_people {
            errors.push(ValidationError::LockedSeatOutOfRange {
                person_id: a.person_id.clone(),
                seat: a.seat_index,
                capacity: table.max_people,
            });
        }
        if !seen_seats.insert((a.table_number, a.seat_index)) {
            errors.push(ValidationError::SeatCollision {
                table_number: a.table_number,
                seat: a.seat_index,
            });
        }
        *occupancy.entry(a.table_number).or_insert(0) += 1;
    }

    for p in &project.people {
        if !seen_people.contains(&p.id) {
            errors.push(ValidationError::MissingOrDuplicatePerson(p.id.clone()));
        }
    }

    for table in &instances {
        let count = occupancy.get(&table.number).copied().unwrap_or(0);
        if count > table.max_people {
            errors.push(ValidationError::TableCapacityExceeded {
                table_number: table.number,
                count,
                capacity: table.max_people,
            });
        }
        if count > 0 {
            if let Some(min_people) = table.min_people {
                if count < min_people {
                    errors.push(ValidationError::TableBelowMin {
                        table_number: table.number,
                        count,
                        min: min_people,
                    });
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationReport { errors })
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Collect person IDs and group IDs, recording duplicate person ID errors.
fn collect_id_sets(
    project: &ProjectInput,
    errors: &mut Vec<ValidationError>,
) -> (HashSet<String>, HashSet<String>) {
    let mut person_ids = HashSet::new();
    let mut group_ids = HashSet::new();

    for p in &project.people {
        if !person_ids.insert(p.id.clone()) {
            errors.push(ValidationError::DuplicatePersonId(p.id.clone()));
        }
        for g in &p.groups {
            group_ids.insert(g.clone());
        }
        if p.locked_seat.is_some() && p.locked_table.is_none() {
            errors.push(ValidationError::LockedSeatRequiresLockedTable(p.id.clone()));
        }
        if let Some(tt) = &p.table_type {
            if !project.table_types.contains_key(tt) {
                errors.push(ValidationError::UnknownTableTypeForPerson {
                    person_id: p.id.clone(),
                    table_type: tt.clone(),
                });
            }
        }
    }

    // Detect namespace collisions (person ID == group ID).
    for pid in &person_ids {
        if group_ids.contains(pid) {
            errors.push(ValidationError::NamespaceCollision(pid.clone()));
        }
    }

    (person_ids, group_ids)
}

/// Validate per-type constraints: shape/side consistency and min/max ordering.
fn validate_table_type_configs(project: &ProjectInput, errors: &mut Vec<ValidationError>) {
    for (table_type, cfg) in &project.table_types {
        if matches!(cfg.shape, TableShape::Rectangular | TableShape::Square) && cfg.people_per_side.is_none() {
            errors.push(ValidationError::MissingPeoplePerSide(table_type.clone()));
        }
        if let Some(side) = &cfg.people_per_side {
            let sum: usize = side.iter().sum();
            if sum != cfg.max_people {
                errors.push(ValidationError::PeoplePerSideMismatch {
                    table_type: table_type.clone(),
                    sum,
                    max: cfg.max_people,
                });
            }
        }
        if let Some(min) = cfg.min_people {
            if min > cfg.max_people {
                errors.push(ValidationError::InvalidMinMax {
                    table_type: table_type.clone(),
                    min,
                    max: cfg.max_people,
                });
            }
        }
    }
}

/// Validate all closeness rules: ID existence, finiteness, and uniqueness.
fn validate_closeness_rules(
    project: &ProjectInput,
    person_ids: &HashSet<String>,
    group_ids: &HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    let known_ids: HashSet<&String> = person_ids.iter().chain(group_ids.iter()).collect();
    let mut seen_pairs: HashSet<(String, String)> = HashSet::new();

    for r in &project.closeness_rules {
        if !r.score.is_finite() {
            errors.push(ValidationError::InvalidClosenessScore {
                left: r.left_id.clone(),
                right: r.right_id.clone(),
            });
        }
        if !known_ids.contains(&r.left_id) {
            errors.push(ValidationError::UnknownIdInCloseness(r.left_id.clone()));
        }
        if !known_ids.contains(&r.right_id) {
            errors.push(ValidationError::UnknownIdInCloseness(r.right_id.clone()));
        }
        let pair = canonical_pair(&r.left_id, &r.right_id);
        if !seen_pairs.insert(pair.clone()) {
            errors.push(ValidationError::DuplicateClosenessRule(pair.0, pair.1));
        }
    }
}

/// Check that total seat capacity across all instances covers all guests.
fn validate_capacity(instances: &[TableInstance], person_count: usize, errors: &mut Vec<ValidationError>) {
    let total_capacity: usize = instances.iter().map(|t| t.max_people).sum();
    if total_capacity < person_count {
        errors.push(ValidationError::NotEnoughSeats {
            required: person_count,
            available: total_capacity,
        });
    }
}

/// Validate locked-table and locked-seat constraints for each guest.
fn validate_locked_assignments(
    project: &ProjectInput,
    table_by_number: &HashMap<usize, &TableInstance>,
    errors: &mut Vec<ValidationError>,
) {
    let mut locked_taken: HashSet<(usize, usize)> = HashSet::new();
    for p in &project.people {
        let Some(table_num) = p.locked_table else { continue };
        let Some(table) = table_by_number.get(&table_num) else {
            errors.push(ValidationError::LockedTableDoesNotExist {
                person_id: p.id.clone(),
                table_number: table_num,
            });
            continue;
        };
        if let Some(required) = &p.table_type {
            if required != &table.table_type {
                errors.push(ValidationError::LockedTableTypeMismatch {
                    person_id: p.id.clone(),
                    table_number: table_num,
                    locked_type: table.table_type.clone(),
                    required_type: required.clone(),
                });
            }
        }
        if let Some(seat) = p.locked_seat {
            if seat >= table.max_people {
                errors.push(ValidationError::LockedSeatOutOfRange {
                    person_id: p.id.clone(),
                    seat,
                    capacity: table.max_people,
                });
            }
            if !locked_taken.insert((table_num, seat)) {
                errors.push(ValidationError::DuplicateLockedSeat {
                    table_number: table_num,
                    seat,
                });
            }
        }
    }
}

/// Verify that every guest has at least one table they can be assigned to.
fn validate_impossible_assignments(
    project: &ProjectInput,
    instances: &[TableInstance],
    errors: &mut Vec<ValidationError>,
) {
    for p in &project.people {
        let has_compatible_table = instances.iter().any(|table| {
            p.table_type
                .as_ref()
                .map(|tt| tt == &table.table_type)
                .unwrap_or(true)
                && p.locked_table.map(|n| n == table.number).unwrap_or(true)
        });
        if !has_compatible_table {
            errors.push(ValidationError::ImpossiblePersonAssignment {
                person_id: p.id.clone(),
            });
        }
    }
}

/// Hard-error when a group with very high self-closeness is larger than any table.
///
/// If a group has `self-closeness ≥ 50` and more members than the largest
/// table capacity, it is physically impossible to seat all members together,
/// making the high-priority preference unsatisfiable.  This is treated as a
/// hard validation error rather than a warning so that users are forced to
/// resolve the mismatch before running the optimizer.
fn validate_large_groups(
    project: &ProjectInput,
    group_ids: &HashSet<String>,
    instances: &[TableInstance],
    errors: &mut Vec<ValidationError>,
) {
    let mut group_sizes: HashMap<&str, usize> = HashMap::new();
    for p in &project.people {
        for g in &p.groups {
            *group_sizes.entry(g.as_str()).or_insert(0) += 1;
        }
    }
    let max_cap = instances.iter().map(|t| t.max_people).max().unwrap_or(0);
    for r in &project.closeness_rules {
        if r.left_id == r.right_id && group_ids.contains(&r.left_id) && r.score >= 50.0 {
            let size = group_sizes.get(r.left_id.as_str()).copied().unwrap_or(0);
            if size > max_cap {
                errors.push(ValidationError::LargeHighPriorityGroup {
                    group_id: r.left_id.clone(),
                    score: r.score,
                    size,
                    max_compatible_capacity: max_cap,
                });
            }
        }
    }
}

/// Returns a lexicographically ordered pair used as a canonical key.
///
/// This normalizes bidirectional relationships (A,B) and (B,A) to the same
/// map key for closeness lookups and duplicate detection.
pub(crate) fn canonical_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

/// Build a lookup map from canonical pair to score from a slice of closeness rules.
pub(crate) fn build_closeness_lookup(
    rules: &[crate::models::ClosenessRule],
) -> Result<HashMap<(String, String), f64>, ValidationError> {
    let mut map = HashMap::new();
    for rule in rules {
        let key = canonical_pair(&rule.left_id, &rule.right_id);
        if map.insert(key.clone(), rule.score).is_some() {
            return Err(ValidationError::DuplicateClosenessRule(key.0, key.1));
        }
    }
    Ok(map)
}

