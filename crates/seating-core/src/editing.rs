//! Reusable editor-facing helpers for structured project data.
//!
//! These APIs support GUI or other interactive frontends without duplicating
//! parsing or project-inspection logic outside `seating-core`.

use crate::models::{
    GroupId, Person, TableTypeConfig, TableTypeId, ValidationError, ValidationReport,
};
use std::collections::{BTreeMap, BTreeSet};

/// Describes an identifier that can be referenced by a closeness rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceIdOption {
    /// Raw identifier stored in the domain model.
    pub id: String,
    /// Human-readable label for chooser UIs.
    pub label: String,
    /// Whether this ID refers to a person or a group.
    pub kind: ReferenceIdKind,
}

/// Kind of closeness reference identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceIdKind {
    /// Person identifier.
    Person,
    /// Group identifier.
    Group,
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
            kind: ReferenceIdKind::Person,
        });
    }
    for group_id in collect_group_ids(people) {
        options.push(ReferenceIdOption {
            label: format!("{} — group", group_id),
            id: group_id,
            kind: ReferenceIdKind::Group,
        });
    }
    options.sort_by(|left, right| left.label.cmp(&right.label));
    options
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
