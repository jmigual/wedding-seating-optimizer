//! Reusable editor-facing helpers for structured project data.
//!
//! These APIs support GUI or other interactive frontends without duplicating
//! parsing or project-inspection logic outside `seating-core`.

use crate::models::{
    ClosenessRule, GroupId, Person, ProjectInput, SeatingAssignment, TableInstance,
    TableTypeConfig, TableTypeId, ValidationError, ValidationReport,
};
use crate::validation::{canonical_pair, generate_table_instances, validate_seating_solution};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Describes an identifier that can be referenced by a closeness rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceIdOption {
    /// Raw identifier stored in the domain model.
    pub id: String,
    /// Human-readable label for chooser UIs.
    pub label: String,
}

/// Build a validated table-type map from editable `(id, config)` rows.
///
/// This is useful for structured editors where users may temporarily create
/// duplicate or empty table type IDs before saving.
pub fn build_table_type_map<I>(
    entries: I,
) -> Result<BTreeMap<TableTypeId, TableTypeConfig>, ValidationReport>
where
    I: IntoIterator<Item = (TableTypeId, TableTypeConfig)>,
{
    let mut errors = Vec::new();
    let mut table_types = BTreeMap::new();

    for (table_type_id, config) in entries {
        let table_type_id = table_type_id.trim().to_string();
        if table_type_id.is_empty() {
            errors.push(ValidationError::EmptyTableTypeId);
            continue;
        }
        if table_types.insert(table_type_id.clone(), config).is_some() {
            errors.push(ValidationError::DuplicateTableTypeId(table_type_id));
        }
    }

    if errors.is_empty() {
        Ok(table_types)
    } else {
        Err(ValidationReport { errors })
    }
}

/// Collect the distinct group IDs present across all people in sorted order.
pub fn collect_group_ids(people: &[Person]) -> Vec<GroupId> {
    let mut groups = BTreeSet::new();
    for person in people {
        for group in &person.groups {
            groups.insert(group.clone());
        }
    }
    groups.into_iter().collect()
}

/// Build searchable reference options for closeness-rule editors.
pub fn reference_id_options(people: &[Person]) -> Vec<ReferenceIdOption> {
    let mut options = Vec::new();
    for group_id in collect_group_ids(people) {
        options.push(ReferenceIdOption {
            label: format!("{} — group", group_id),
            id: group_id,
        });
    }
    let mut people_sorted: Vec<&Person> = people.iter().collect();
    people_sorted.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    for person in people_sorted {
        options.push(ReferenceIdOption {
            id: person.id.clone(),
            label: if person.name.trim().is_empty() {
                person.id.clone()
            } else {
                person.name.clone()
            },
        });
    }
    options
}

/// Rename group `old` to `new` everywhere it is referenced.
///
/// Updates every person's `groups` list — deduping if the person already
/// belongs to `new` — and rewrites any closeness rule id (`left_id`/
/// `right_id`) equal to `old`. A no-op if `new` is blank or equal to `old`.
pub fn rename_group(people: &mut [Person], rules: &mut [ClosenessRule], old: &str, new: &str) {
    let new = new.trim();
    if new.is_empty() || new == old {
        return;
    }
    for person in people.iter_mut() {
        if person.groups.iter().any(|group| group == old) {
            person.groups.retain(|group| group != old);
            if !person.groups.iter().any(|group| group == new) {
                person.groups.push(new.to_string());
            }
        }
    }
    for rule in rules.iter_mut() {
        if rule.left_id == old {
            rule.left_id = new.to_string();
        }
        if rule.right_id == old {
            rule.right_id = new.to_string();
        }
    }
}

/// Remove group `group` everywhere it is referenced: from every person's
/// `groups` list, and any closeness rule that names it as `left_id` or
/// `right_id`.
pub fn remove_group(people: &mut [Person], rules: &mut Vec<ClosenessRule>, group: &str) {
    for person in people.iter_mut() {
        person.groups.retain(|g| g != group);
    }
    rules.retain(|rule| rule.left_id != group && rule.right_id != group);
}

/// Parse an optional non-negative integer field from structured-editor input.
pub fn parse_optional_usize_value(
    input: &str,
    field_name: &str,
) -> Result<Option<usize>, ValidationError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed.parse::<usize>().map(Some).map_err(|error| {
            ValidationError::MalformedInput(format!("invalid {field_name} '{trimmed}': {error}"))
        })
    }
}

/// Parse a required non-negative integer field from structured-editor input.
pub fn parse_required_usize_value(input: &str, field_name: &str) -> Result<usize, ValidationError> {
    parse_optional_usize_value(input, field_name)?
        .ok_or_else(|| ValidationError::MalformedInput(format!("{field_name} is required")))
}

/// Parse a required finite floating-point field from structured-editor input.
pub fn parse_f64_value(input: &str, field_name: &str) -> Result<f64, ValidationError> {
    let trimmed = input.trim();
    let value = trimmed.parse::<f64>().map_err(|error| {
        ValidationError::MalformedInput(format!("invalid {field_name} '{trimmed}': {error}"))
    })?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ValidationError::MalformedInput(format!(
            "invalid {field_name} '{trimmed}': value must be finite"
        )))
    }
}

/// Parse a pipe-separated list of non-negative integers (e.g. `"2|2|1|1"`).
///
/// An input that is empty, or that contains only empty/whitespace tokens
/// (e.g. `""`, `"|"`, `"||"`), is treated identically and returns `None` —
/// there is no meaningful difference between "no value entered" and "only
/// separators entered".
pub(crate) fn parse_optional_usize_list(
    input: &str,
    field_name: &str,
) -> Result<Option<Vec<usize>>, ValidationError> {
    let mut values = Vec::new();
    for token in input.split('|') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        values.push(trimmed.parse::<usize>().map_err(|error| {
            ValidationError::MalformedInput(format!(
                "invalid {field_name} value '{trimmed}': {error}"
            ))
        })?);
    }
    if values.is_empty() {
        Ok(None)
    } else {
        Ok(Some(values))
    }
}

/// Parse a table type's pipe-separated `people_per_side` field (e.g. `"2|2|1|1"`).
///
/// Delegates to the same parser used by CSV table-type ingestion, so a value
/// typed into a structured editor validates identically to one loaded from
/// `tables.csv`.
pub fn parse_people_per_side(input: &str) -> Result<Option<Vec<usize>>, ValidationError> {
    parse_optional_usize_list(input, "people_per_side")
}

/// Unordered-pair equality: `true` when `{a_left, a_right}` and
/// `{b_left, b_right}` name the same two identifiers, regardless of order.
///
/// Useful for matching a canonicalized pair reported in a
/// [`ValidationError`] (e.g. [`DuplicateClosenessRule`](ValidationError::DuplicateClosenessRule))
/// back to the rule that produced it.
pub fn rules_match(a_left: &str, a_right: &str, b_left: &str, b_right: &str) -> bool {
    (a_left == b_left && a_right == b_right) || (a_left == b_right && a_right == b_left)
}

/// Filter `options` to those matching `query` (case-insensitive substring on
/// id or label), capped to the first 5 matches. An empty query matches
/// everything.
pub fn reference_matches(options: &[ReferenceIdOption], query: &str) -> Vec<ReferenceIdOption> {
    let normalized = query.trim().to_ascii_lowercase();
    options
        .iter()
        .filter(|option| {
            normalized.is_empty()
                || option.id.to_ascii_lowercase().contains(&normalized)
                || option.label.to_ascii_lowercase().contains(&normalized)
        })
        .take(5)
        .cloned()
        .collect()
}

/// Look up the display label for `id` among `options`, or a fallback marking
/// it as unknown if no option has that id.
pub fn reference_label(id: &str, options: &[ReferenceIdOption]) -> String {
    options
        .iter()
        .find(|option| option.id == id)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| format!("{id} — unknown"))
}

// ── CSV import merge ──────────────────────────────────────────────────────────

/// Upsert `imported` into `existing`, keyed by `Person.id`. An imported
/// person whose id matches an existing one replaces that entry in place;
/// an imported person with a new id is appended (in `imported`'s order);
/// an existing person whose id is absent from `imported` is kept unchanged
/// and in its original position.
pub fn merge_people(existing: &[Person], imported: Vec<Person>) -> Vec<Person> {
    let mut result = existing.to_vec();
    let mut index: HashMap<String, usize> = result
        .iter()
        .enumerate()
        .map(|(i, person)| (person.id.clone(), i))
        .collect();
    for person in imported {
        if let Some(&position) = index.get(&person.id) {
            result[position] = person;
        } else {
            index.insert(person.id.clone(), result.len());
            result.push(person);
        }
    }
    result
}

/// Upsert `imported` into `existing`, keyed by the unordered
/// `(left_id, right_id)` pair (see [`canonical_pair`]) — a rule for
/// `(a, b)` and one for `(b, a)` are the same key. Same replace/append/keep
/// semantics as [`merge_people`].
pub fn merge_closeness_rules(
    existing: &[ClosenessRule],
    imported: Vec<ClosenessRule>,
) -> Vec<ClosenessRule> {
    let mut result = existing.to_vec();
    let mut index: HashMap<(String, String), usize> = result
        .iter()
        .enumerate()
        .map(|(i, rule)| (canonical_pair(&rule.left_id, &rule.right_id), i))
        .collect();
    for rule in imported {
        let key = canonical_pair(&rule.left_id, &rule.right_id);
        if let Some(&position) = index.get(&key) {
            result[position] = rule;
        } else {
            index.insert(key, result.len());
            result.push(rule);
        }
    }
    result
}

/// Upsert `imported` into `existing`, keyed by the map's own `TableTypeId`
/// key. Equivalent to `existing.clone()` followed by `.extend(imported)`,
/// which is exactly `BTreeMap`'s upsert semantics (overwrite on collision,
/// insert if new) — stated as a named function so the GUI never
/// reimplements this and so it's covered by its own test.
pub fn merge_table_types(
    existing: &BTreeMap<TableTypeId, TableTypeConfig>,
    imported: BTreeMap<TableTypeId, TableTypeConfig>,
) -> BTreeMap<TableTypeId, TableTypeConfig> {
    let mut merged = existing.clone();
    merged.extend(imported);
    merged
}

// ── Seat drag-and-drop ────────────────────────────────────────────────────────

/// Outcome of [`apply_seat_drop`]: whether the target seat was empty (a plain
/// move) or occupied (a swap of the two guests' positions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatDropOutcome {
    /// The target seat was empty; the dragged guest was placed there.
    Moved,
    /// The target seat was occupied; the two guests swapped positions.
    Swapped,
}

/// Apply a drag-and-drop of `person_id` onto (`target_table`, `target_seat`).
///
/// An empty target seat results in a move; an occupied one results in a swap
/// of the two guests' `(table, seat)` positions. The candidate result is
/// validated via [`validate_seating_solution`], so locks, table_type
/// compatibility, capacity, and seat range are all enforced by that call.
/// The input `assignments` is not mutated on failure.
///
/// A no-op drop (dropping a guest onto their own current seat) always
/// succeeds and returns [`SeatDropOutcome::Moved`] with `assignments`
/// unchanged.
pub fn apply_seat_drop(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    person_id: &str,
    target_table: usize,
    target_seat: usize,
) -> Result<(Vec<SeatingAssignment>, SeatDropOutcome), ValidationReport> {
    let Some(mover_index) = assignments.iter().position(|a| a.person_id == person_id) else {
        return Err(ValidationReport {
            errors: vec![ValidationError::MissingOrDuplicatePerson(
                person_id.to_string(),
            )],
        });
    };

    if assignments[mover_index].table_number == target_table
        && assignments[mover_index].seat_index == target_seat
    {
        return Ok((assignments.to_vec(), SeatDropOutcome::Moved));
    }

    let instances = generate_table_instances(project);
    let target_table_type = instances
        .iter()
        .find(|instance| instance.number == target_table)
        .map(|instance| instance.table_type.clone())
        .unwrap_or_else(|| assignments[mover_index].table_type.clone());

    let occupant_index = assignments
        .iter()
        .position(|a| a.table_number == target_table && a.seat_index == target_seat);

    let mut updated = assignments.to_vec();
    let outcome = if let Some(occupant_index) = occupant_index {
        let mover_table = updated[mover_index].table_number;
        let mover_seat = updated[mover_index].seat_index;
        let mover_table_type = updated[mover_index].table_type.clone();

        updated[mover_index].table_number = target_table;
        updated[mover_index].seat_index = target_seat;
        updated[mover_index].table_type = target_table_type;

        updated[occupant_index].table_number = mover_table;
        updated[occupant_index].seat_index = mover_seat;
        updated[occupant_index].table_type = mover_table_type;

        SeatDropOutcome::Swapped
    } else {
        updated[mover_index].table_number = target_table;
        updated[mover_index].seat_index = target_seat;
        updated[mover_index].table_type = target_table_type;

        SeatDropOutcome::Moved
    };

    validate_seating_solution(project, &updated)?;
    Ok((updated, outcome))
}

// ── Table renumbering ─────────────────────────────────────────────────────────

/// Map old table numbers to their new numbers after [`generate_table_instances`]
/// is re-run following a [`TableTypeConfig`] change (e.g. `number_of_tables`).
///
/// Instances are matched by `(table_type, ordinal-within-type)`, not by raw
/// number, since bumping one type's count shifts every later type's numbers.
/// An old table whose `(table_type, ordinal)` no longer exists among
/// `new_instances` (its type was shrunk or removed) has no entry in the
/// returned map — callers should leave assignments referencing it unchanged
/// so validation reports it rather than silently dropping the guest.
///
/// [`generate_table_instances`]: crate::validation::generate_table_instances
pub fn table_number_remap(
    old_instances: &[TableInstance],
    new_instances: &[TableInstance],
) -> BTreeMap<usize, usize> {
    fn by_type_ordinal(instances: &[TableInstance]) -> HashMap<(TableTypeId, usize), usize> {
        let mut counts: HashMap<TableTypeId, usize> = HashMap::new();
        instances
            .iter()
            .map(|instance| {
                let ordinal = counts.entry(instance.table_type.clone()).or_insert(0);
                let key = (instance.table_type.clone(), *ordinal);
                *ordinal += 1;
                (key, instance.number)
            })
            .collect()
    }

    let old_by_key = by_type_ordinal(old_instances);
    let new_by_key = by_type_ordinal(new_instances);
    old_by_key
        .into_iter()
        .filter_map(|(key, old_number)| {
            new_by_key
                .get(&key)
                .map(|&new_number| (old_number, new_number))
        })
        .collect()
}

// ── Table reordering ──────────────────────────────────────────────────────────

/// The current type of every table, indexed by table number − 1.
///
/// Materialized from [`generate_table_instances`] so the reorder helpers work
/// whether or not [`ProjectInput::table_order`] was already set.
fn current_table_order(project: &ProjectInput) -> Vec<TableTypeId> {
    generate_table_instances(project)
        .into_iter()
        .map(|instance| instance.table_type)
        .collect()
}

/// Build the old→new table-number map implied by `numbers`, where `numbers[i]`
/// is the table that now carries number `i + 1`.
fn table_number_map(numbers: &[usize]) -> BTreeMap<usize, usize> {
    numbers
        .iter()
        .enumerate()
        .map(|(index, &old)| (old, index + 1))
        .collect()
}

/// Swap the numbers of tables `a` and `b`, returning the new explicit
/// [`ProjectInput::table_order`] and an old→new table-number map covering
/// every table.
///
/// The **whole table** moves: its type, its shape *and* its guests. Callers
/// must therefore apply the returned map to every
/// [`SeatingAssignment::table_number`] and every [`Person::locked_table`], and
/// store the returned order in [`ProjectInput::table_order`].
/// [`SeatingAssignment::table_type`] needs no update — the type travels with
/// the number.
///
/// Score-preserving up to f64 summation order and validity-preserving: every
/// table keeps its own occupants and its own type, so capacity, seat and lock
/// constraints still hold even when the two tables have different
/// capacities; only the labels change.
///
/// Returns `None` if `a` or `b` is not an existing (1-based) table number.
pub fn swap_table_numbers(
    project: &ProjectInput,
    a: usize,
    b: usize,
) -> Option<(Vec<TableTypeId>, BTreeMap<usize, usize>)> {
    let mut order = current_table_order(project);
    if a == 0 || b == 0 || a > order.len() || b > order.len() {
        return None;
    }
    let mut numbers: Vec<usize> = (1..=order.len()).collect();
    order.swap(a - 1, b - 1);
    numbers.swap(a - 1, b - 1);
    Some((order, table_number_map(&numbers)))
}

/// Renumber the table currently numbered `from` to `to`, shifting every table
/// in between (drag-to-reorder), and return the new explicit
/// [`ProjectInput::table_order`] plus an old→new table-number map covering
/// every table.
///
/// Carries the same semantics as [`swap_table_numbers`]: whole tables move
/// with their type, shape and guests, so the caller applies the map to
/// [`SeatingAssignment::table_number`] and [`Person::locked_table`], and the
/// operation is score-preserving up to f64 summation order and
/// validity-preserving.
///
/// Returns `None` if `from` or `to` is not an existing (1-based) table number.
pub fn move_table_number(
    project: &ProjectInput,
    from: usize,
    to: usize,
) -> Option<(Vec<TableTypeId>, BTreeMap<usize, usize>)> {
    let mut order = current_table_order(project);
    if from == 0 || to == 0 || from > order.len() || to > order.len() {
        return None;
    }
    let mut numbers: Vec<usize> = (1..=order.len()).collect();
    let moved_type = order.remove(from - 1);
    order.insert(to - 1, moved_type);
    let moved_number = numbers.remove(from - 1);
    numbers.insert(to - 1, moved_number);
    Some((order, table_number_map(&numbers)))
}

// ── Table compaction ──────────────────────────────────────────────────────────

/// Repack each table type's used instances onto that type's lowest-numbered
/// instances, so used tables sort before empty ones (e.g. table 3 moves to
/// table 2 when table 2 of the same type is empty). Preserves each guest's
/// seat index and the relative order of used tables within their type.
///
/// A table holding any guest with [`locked_table`](Person::locked_table)
/// set is pinned: it keeps its number, and the other used tables of that
/// type fill the remaining lowest, non-pinned numbers in order.
///
/// Score-preserving up to f64 summation order and validity-preserving:
/// instances of the same table type are identical for scoring (same
/// shape/capacity/min/recommended), so this only relabels which
/// interchangeable instance a guest's occupant set sits at — it does not
/// change which pairs of guests share a table, and moving a whole occupant
/// set between same-type tables cannot violate capacity, seat, or lock
/// invariants.
///
/// For the same reason it leaves [`ProjectInput::table_order`] valid: each
/// table number keeps its type, so the order needs no update.
pub fn compact_table_numbers(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
) -> Vec<SeatingAssignment> {
    let instances = generate_table_instances(project);
    let mut numbers_by_type: BTreeMap<&TableTypeId, Vec<usize>> = BTreeMap::new();
    for instance in &instances {
        numbers_by_type
            .entry(&instance.table_type)
            .or_default()
            .push(instance.number);
    }

    let locked_person_ids: HashSet<&str> = project
        .people
        .iter()
        .filter(|person| person.locked_table.is_some())
        .map(|person| person.id.as_str())
        .collect();

    let mut used_numbers: BTreeSet<usize> = BTreeSet::new();
    let mut pinned_numbers: BTreeSet<usize> = BTreeSet::new();
    for assignment in assignments {
        used_numbers.insert(assignment.table_number);
        if locked_person_ids.contains(assignment.person_id.as_str()) {
            pinned_numbers.insert(assignment.table_number);
        }
    }

    let mut number_map: HashMap<usize, usize> = HashMap::new();
    for numbers in numbers_by_type.values() {
        let free_numbers = numbers
            .iter()
            .copied()
            .filter(|n| !pinned_numbers.contains(n));
        let unpinned_used = numbers
            .iter()
            .copied()
            .filter(|n| used_numbers.contains(n) && !pinned_numbers.contains(n));
        for (old, new) in unpinned_used.zip(free_numbers) {
            number_map.insert(old, new);
        }
    }

    assignments
        .iter()
        .map(|assignment| SeatingAssignment {
            table_number: number_map
                .get(&assignment.table_number)
                .copied()
                .unwrap_or(assignment.table_number),
            ..assignment.clone()
        })
        .collect()
}

// ── ValidationError association helpers ───────────────────────────────────────

impl ValidationError {
    /// The person id this error is about, if any.
    ///
    /// Exhaustive over every [`ValidationError`] variant so a newly added
    /// variant fails to compile here until it is classified, rather than
    /// silently falling through a wildcard arm.
    pub fn person_id(&self) -> Option<&str> {
        match self {
            ValidationError::DuplicatePersonId(id) => Some(id),
            ValidationError::NamespaceCollision(id) => Some(id),
            ValidationError::UnknownTableTypeForPerson { person_id, .. } => Some(person_id),
            ValidationError::LockedSeatRequiresLockedTable(id) => Some(id),
            ValidationError::LockedTableDoesNotExist { person_id, .. } => Some(person_id),
            ValidationError::LockedSeatOutOfRange { person_id, .. } => Some(person_id),
            ValidationError::SeatIndexOutOfRange { person_id, .. } => Some(person_id),
            ValidationError::LockedTableTypeMismatch { person_id, .. } => Some(person_id),
            ValidationError::ImpossiblePersonAssignment { person_id } => Some(person_id),
            ValidationError::MissingOrDuplicatePerson(id) => Some(id),
            ValidationError::UnknownPersonInSeating(id) => Some(id),
            ValidationError::SeatingPersonTableTypeMismatch { person_id, .. } => Some(person_id),
            ValidationError::SeatingViolatesLockedTable { person_id, .. } => Some(person_id),
            ValidationError::SeatingViolatesLockedSeat { person_id, .. } => Some(person_id),
            ValidationError::DuplicateTableTypeId(_)
            | ValidationError::EmptyTableTypeId
            | ValidationError::EmptyPersonId
            | ValidationError::UnknownIdInCloseness(_)
            | ValidationError::DuplicateClosenessRule(_, _)
            | ValidationError::InvalidClosenessScore { .. }
            | ValidationError::PersonSelfClosenessRule(_)
            | ValidationError::MissingPeoplePerSide(_)
            | ValidationError::InvalidPeoplePerSideLength { .. }
            | ValidationError::PeoplePerSideMismatch { .. }
            | ValidationError::InvalidMinMax { .. }
            | ValidationError::InvalidRecommendedPeople { .. }
            | ValidationError::InvalidNumberOfTables { .. }
            | ValidationError::DuplicateLockedSeat { .. }
            | ValidationError::LockedTableOverbooked { .. }
            | ValidationError::NotEnoughSeats { .. }
            | ValidationError::NotEnoughSeatsForTableType { .. }
            | ValidationError::LargeHighPriorityGroup { .. }
            | ValidationError::UnknownTableInSeating(_)
            | ValidationError::SeatCollision { .. }
            | ValidationError::TableCapacityExceeded { .. }
            | ValidationError::MalformedInput(_)
            | ValidationError::UnsupportedProjectVersion { .. }
            | ValidationError::NoFeasibleAssignment
            | ValidationError::SeatingTableTypeMismatch { .. } => None,
        }
    }

    /// The table type id this error is about, if any.
    pub fn table_type_id(&self) -> Option<&str> {
        match self {
            ValidationError::DuplicateTableTypeId(id) => Some(id),
            ValidationError::UnknownTableTypeForPerson { table_type, .. } => Some(table_type),
            ValidationError::MissingPeoplePerSide(table_type) => Some(table_type),
            ValidationError::InvalidPeoplePerSideLength { table_type, .. } => Some(table_type),
            ValidationError::PeoplePerSideMismatch { table_type, .. } => Some(table_type),
            ValidationError::InvalidMinMax { table_type, .. } => Some(table_type),
            ValidationError::InvalidRecommendedPeople { table_type, .. } => Some(table_type),
            ValidationError::InvalidNumberOfTables { table_type, .. } => Some(table_type),
            ValidationError::NotEnoughSeatsForTableType { table_type, .. } => Some(table_type),
            ValidationError::LockedTableTypeMismatch { locked_type, .. } => Some(locked_type),
            ValidationError::SeatingTableTypeMismatch { actual_type, .. } => Some(actual_type),
            ValidationError::SeatingPersonTableTypeMismatch { required_type, .. } => {
                Some(required_type)
            }
            ValidationError::EmptyTableTypeId
            | ValidationError::EmptyPersonId
            | ValidationError::DuplicatePersonId(_)
            | ValidationError::NamespaceCollision(_)
            | ValidationError::LockedSeatRequiresLockedTable(_)
            | ValidationError::LockedTableDoesNotExist { .. }
            | ValidationError::LockedSeatOutOfRange { .. }
            | ValidationError::SeatIndexOutOfRange { .. }
            | ValidationError::UnknownIdInCloseness(_)
            | ValidationError::DuplicateClosenessRule(_, _)
            | ValidationError::InvalidClosenessScore { .. }
            | ValidationError::PersonSelfClosenessRule(_)
            | ValidationError::DuplicateLockedSeat { .. }
            | ValidationError::LockedTableOverbooked { .. }
            | ValidationError::NotEnoughSeats { .. }
            | ValidationError::ImpossiblePersonAssignment { .. }
            | ValidationError::LargeHighPriorityGroup { .. }
            | ValidationError::MissingOrDuplicatePerson(_)
            | ValidationError::UnknownPersonInSeating(_)
            | ValidationError::UnknownTableInSeating(_)
            | ValidationError::SeatCollision { .. }
            | ValidationError::TableCapacityExceeded { .. }
            | ValidationError::MalformedInput(_)
            | ValidationError::UnsupportedProjectVersion { .. }
            | ValidationError::NoFeasibleAssignment
            | ValidationError::SeatingViolatesLockedTable { .. }
            | ValidationError::SeatingViolatesLockedSeat { .. } => None,
        }
    }

    /// The closeness-rule pair (left, right) this error is about, if any.
    pub fn closeness_pair(&self) -> Option<(&str, &str)> {
        match self {
            ValidationError::DuplicateClosenessRule(left, right) => Some((left, right)),
            ValidationError::InvalidClosenessScore { left, right } => Some((left, right)),
            ValidationError::UnknownIdInCloseness(_)
            | ValidationError::PersonSelfClosenessRule(_)
            | ValidationError::DuplicatePersonId(_)
            | ValidationError::DuplicateTableTypeId(_)
            | ValidationError::EmptyTableTypeId
            | ValidationError::EmptyPersonId
            | ValidationError::NamespaceCollision(_)
            | ValidationError::UnknownTableTypeForPerson { .. }
            | ValidationError::LockedSeatRequiresLockedTable(_)
            | ValidationError::LockedTableDoesNotExist { .. }
            | ValidationError::LockedSeatOutOfRange { .. }
            | ValidationError::SeatIndexOutOfRange { .. }
            | ValidationError::MissingPeoplePerSide(_)
            | ValidationError::InvalidPeoplePerSideLength { .. }
            | ValidationError::PeoplePerSideMismatch { .. }
            | ValidationError::InvalidMinMax { .. }
            | ValidationError::InvalidRecommendedPeople { .. }
            | ValidationError::InvalidNumberOfTables { .. }
            | ValidationError::DuplicateLockedSeat { .. }
            | ValidationError::LockedTableTypeMismatch { .. }
            | ValidationError::LockedTableOverbooked { .. }
            | ValidationError::NotEnoughSeats { .. }
            | ValidationError::NotEnoughSeatsForTableType { .. }
            | ValidationError::ImpossiblePersonAssignment { .. }
            | ValidationError::LargeHighPriorityGroup { .. }
            | ValidationError::MissingOrDuplicatePerson(_)
            | ValidationError::UnknownPersonInSeating(_)
            | ValidationError::UnknownTableInSeating(_)
            | ValidationError::SeatCollision { .. }
            | ValidationError::TableCapacityExceeded { .. }
            | ValidationError::MalformedInput(_)
            | ValidationError::UnsupportedProjectVersion { .. }
            | ValidationError::NoFeasibleAssignment
            | ValidationError::SeatingTableTypeMismatch { .. }
            | ValidationError::SeatingPersonTableTypeMismatch { .. }
            | ValidationError::SeatingViolatesLockedTable { .. }
            | ValidationError::SeatingViolatesLockedSeat { .. } => None,
        }
    }
}

// ── Closeness display ordering ────────────────────────────────────────────────

/// Compute a display order for closeness rules: group↔group pairs first,
/// then group↔person (either direction), then person↔person. Within a
/// category, rules are ordered by their displayed labels (case-insensitive),
/// first id then second.
///
/// Returns indices into `pairs` in display order. This is display ordering
/// only — it does not reorder or mutate the underlying rules, so callers
/// must index back through the returned permutation when editing or
/// deleting a row.
pub fn closeness_display_order(
    pairs: &[(&str, &str)],
    groups: &[GroupId],
    options: &[ReferenceIdOption],
) -> Vec<usize> {
    let is_group = |id: &str| groups.iter().any(|group| group.as_str() == id);
    let mut order: Vec<usize> = (0..pairs.len()).collect();
    order.sort_by_key(|&i| {
        let (left, right) = pairs[i];
        let rank = match (is_group(left), is_group(right)) {
            (true, true) => 0,
            (false, false) => 2,
            _ => 1,
        };
        (
            rank,
            reference_label(left, options).to_ascii_lowercase(),
            reference_label(right, options).to_ascii_lowercase(),
        )
    });
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_people_per_side_treats_separator_only_input_as_absent() {
        assert_eq!(parse_people_per_side("").unwrap(), None);
        assert_eq!(parse_people_per_side("|").unwrap(), None);
        assert_eq!(parse_people_per_side("||").unwrap(), None);
    }

    #[test]
    fn parse_people_per_side_parses_values() {
        assert_eq!(
            parse_people_per_side("2|2|1|1").unwrap(),
            Some(vec![2, 2, 1, 1])
        );
    }

    #[test]
    fn parse_people_per_side_rejects_non_numeric_token() {
        let err = parse_people_per_side("2|x|1|1").unwrap_err();
        assert!(matches!(err, ValidationError::MalformedInput(_)));
    }

    #[test]
    fn rules_match_is_order_independent() {
        assert!(rules_match("a", "b", "b", "a"));
        assert!(rules_match("a", "b", "a", "b"));
        assert!(!rules_match("a", "b", "a", "c"));
    }

    #[test]
    fn reference_matches_filters_case_insensitively_and_caps_at_five() {
        let options: Vec<ReferenceIdOption> = (0..10)
            .map(|i| ReferenceIdOption {
                id: format!("p{i}"),
                label: format!("Guest {i}"),
            })
            .collect();
        assert_eq!(reference_matches(&options, "").len(), 5);
        let hits = reference_matches(&options, "GUEST 1");
        assert!(hits.iter().all(|o| o.label.contains("Guest 1")));
    }

    #[test]
    fn reference_label_falls_back_for_unknown_id() {
        let options = vec![ReferenceIdOption {
            id: "p1".to_string(),
            label: "Alice".to_string(),
        }];
        assert_eq!(reference_label("p1", &options), "Alice");
        assert_eq!(reference_label("missing", &options), "missing — unknown");
    }

    #[test]
    fn reference_id_options_put_groups_first_then_people_by_name() {
        let people = vec![
            Person {
                id: "p2".to_string(),
                name: "Zoe".to_string(),
                table_type: None,
                groups: vec!["family".to_string()],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p1".to_string(),
                name: "Alice".to_string(),
                table_type: None,
                groups: vec!["friends".to_string()],
                locked_table: None,
                locked_seat: None,
            },
        ];

        let options = reference_id_options(&people);
        let labels: Vec<&str> = options.iter().map(|option| option.label.as_str()).collect();
        let ids: Vec<&str> = options.iter().map(|option| option.id.as_str()).collect();

        assert_eq!(
            labels,
            vec!["family — group", "friends — group", "Alice", "Zoe"]
        );
        assert_eq!(ids, vec!["family", "friends", "p1", "p2"]);
    }

    fn person(id: &str, groups: &[&str]) -> Person {
        Person {
            id: id.to_string(),
            name: id.to_string(),
            table_type: None,
            groups: groups.iter().map(|g| g.to_string()).collect(),
            locked_table: None,
            locked_seat: None,
        }
    }

    #[test]
    fn rename_group_updates_people_and_rules() {
        let mut people = vec![person("p1", &["family"]), person("p2", &["friends"])];
        let mut rules = vec![ClosenessRule {
            left_id: "family".to_string(),
            right_id: "p2".to_string(),
            score: 2.0,
        }];
        rename_group(&mut people, &mut rules, "family", "relatives");
        assert_eq!(people[0].groups, vec!["relatives".to_string()]);
        assert_eq!(people[1].groups, vec!["friends".to_string()]);
        assert_eq!(rules[0].left_id, "relatives");
    }

    #[test]
    fn rename_group_dedupes_when_person_already_has_new_name() {
        let mut people = vec![person("p1", &["family", "relatives"])];
        let mut rules: Vec<ClosenessRule> = Vec::new();
        rename_group(&mut people, &mut rules, "family", "relatives");
        assert_eq!(people[0].groups, vec!["relatives".to_string()]);
    }

    #[test]
    fn remove_group_drops_from_people_and_rules() {
        let mut people = vec![
            person("p1", &["family", "friends"]),
            person("p2", &["family"]),
        ];
        let mut rules = vec![
            ClosenessRule {
                left_id: "family".to_string(),
                right_id: "p2".to_string(),
                score: 2.0,
            },
            ClosenessRule {
                left_id: "p1".to_string(),
                right_id: "p2".to_string(),
                score: 1.0,
            },
        ];
        remove_group(&mut people, &mut rules, "family");
        assert_eq!(people[0].groups, vec!["friends".to_string()]);
        assert!(people[1].groups.is_empty());
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].left_id, "p1");
    }

    #[test]
    fn merge_people_upserts_by_id() {
        let p1 = person("p1", &[]);
        let p2 = person("p2", &[]);
        let existing = vec![p1.clone(), p2.clone()];
        let mut p2_updated = person("p2", &["family"]);
        p2_updated.name = "P2 Updated".to_string();
        let p3 = person("p3", &[]);
        let merged = merge_people(&existing, vec![p2_updated.clone(), p3.clone()]);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].id, "p1");
        assert_eq!(merged[1].id, "p2");
        assert_eq!(merged[1].name, "P2 Updated");
        assert_eq!(merged[2].id, "p3");
    }

    #[test]
    fn merge_closeness_rules_upserts_by_unordered_pair() {
        let existing = vec![
            ClosenessRule {
                left_id: "a".to_string(),
                right_id: "b".to_string(),
                score: 1.0,
            },
            ClosenessRule {
                left_id: "e".to_string(),
                right_id: "f".to_string(),
                score: 9.0,
            },
        ];
        let imported = vec![
            ClosenessRule {
                left_id: "b".to_string(),
                right_id: "a".to_string(),
                score: 5.0,
            },
            ClosenessRule {
                left_id: "c".to_string(),
                right_id: "d".to_string(),
                score: 2.0,
            },
        ];
        let merged = merge_closeness_rules(&existing, imported);
        assert_eq!(merged.len(), 3);
        // a-b collided with imported b-a: imported wins.
        assert_eq!(merged[0].score, 5.0);
        // e-f absent from import: kept unchanged.
        assert_eq!(merged[1].left_id, "e");
        assert_eq!(merged[1].score, 9.0);
        // c-d is new: appended.
        assert_eq!(merged[2].left_id, "c");
    }

    fn table_config(max_people: usize) -> TableTypeConfig {
        TableTypeConfig {
            shape: crate::models::TableShape::Round,
            people_per_side: None,
            max_people,
            recommended_people: None,
            min_people: None,
            number_of_tables: None,
        }
    }

    #[test]
    fn merge_table_types_upserts_by_key() {
        let mut existing = BTreeMap::new();
        existing.insert("round_8".to_string(), table_config(8));
        existing.insert("round_10".to_string(), table_config(10));

        let mut imported = BTreeMap::new();
        imported.insert("round_10".to_string(), table_config(99));
        imported.insert("round_12".to_string(), table_config(12));

        let merged = merge_table_types(&existing, imported);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged["round_8"].max_people, 8);
        assert_eq!(merged["round_10"].max_people, 99);
        assert_eq!(merged["round_12"].max_people, 12);
    }

    #[test]
    fn validation_error_association_helpers_are_exhaustive_and_correct() {
        let error = ValidationError::UnknownTableTypeForPerson {
            person_id: "p1".to_string(),
            table_type: "round_4".to_string(),
        };
        assert_eq!(error.person_id(), Some("p1"));
        assert_eq!(error.table_type_id(), Some("round_4"));
        assert_eq!(error.closeness_pair(), None);

        let closeness_error =
            ValidationError::DuplicateClosenessRule("a".to_string(), "b".to_string());
        assert_eq!(closeness_error.closeness_pair(), Some(("a", "b")));
    }

    #[test]
    fn closeness_display_order_sorts_group_group_then_group_person_then_person_person() {
        let groups = vec!["family".to_string(), "friends".to_string()];
        let options = vec![
            ReferenceIdOption {
                id: "family".to_string(),
                label: "family — group".to_string(),
            },
            ReferenceIdOption {
                id: "friends".to_string(),
                label: "friends — group".to_string(),
            },
            ReferenceIdOption {
                id: "p1".to_string(),
                label: "Zoe".to_string(),
            },
            ReferenceIdOption {
                id: "p2".to_string(),
                label: "Alice".to_string(),
            },
        ];
        // person-person, group-group, group-person, in stored order.
        let pairs = [("p1", "p2"), ("friends", "family"), ("family", "p1")];

        let order = closeness_display_order(&pairs, &groups, &options);

        assert_eq!(order, vec![1, 2, 0]);
    }
}
