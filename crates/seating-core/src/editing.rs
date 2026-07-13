//! Reusable editor-facing helpers for structured project data.
//!
//! These APIs support GUI or other interactive frontends without duplicating
//! parsing or project-inspection logic outside `seating-core`.

use crate::models::{
    ClosenessRule, GroupId, Person, ProjectInput, SeatingAssignment, TableTypeConfig, TableTypeId,
    ValidationError, ValidationReport,
};
use crate::validation::{generate_table_instances, validate_seating_solution};
use std::collections::{BTreeMap, BTreeSet};

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
    for person in people {
        options.push(ReferenceIdOption {
            id: person.id.clone(),
            label: format!("{} — {} — person", person.id, person.name),
        });
    }
    for group_id in collect_group_ids(people) {
        options.push(ReferenceIdOption {
            label: format!("{} — group", group_id),
            id: group_id,
        });
    }
    options.sort_by(|left, right| left.label.cmp(&right.label));
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
            | ValidationError::TableBelowMin { .. }
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
            | ValidationError::TableBelowMin { .. }
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
            | ValidationError::TableBelowMin { .. }
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
            label: "p1 — Alice — person".to_string(),
        }];
        assert_eq!(reference_label("p1", &options), "p1 — Alice — person");
        assert_eq!(reference_label("missing", &options), "missing — unknown");
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
}
