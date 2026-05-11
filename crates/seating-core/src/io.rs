//! CSV parsing and serialization for all input/output file formats.
//!
//! Public API:
//! - [`parse_people_csv`] / [`write_people_csv`]
//! - [`parse_closeness_csv`] / [`write_closeness_csv`]
//! - [`parse_tables_csv`] / [`write_tables_csv`]
//! - [`parse_seating_csv`] / [`write_seating_csv`]
//! - [`make_project`] – convenience constructor that parses all three inputs

use crate::models::{
    ClosenessRule, Person, ProjectInput, SeatingAssignment, TableShape, TableTypeConfig,
    TableTypeId, ValidationError,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ── Internal deserialization structs ──────────────────────────────────────────
// These are intentionally private; callers always receive the public domain types.

#[derive(Debug, Deserialize)]
struct PersonCsvRow {
    id: String,
    name: String,
    #[serde(default)]
    table_type: String,
    #[serde(default)]
    groups: String,
    #[serde(default)]
    locked_table: String,
    #[serde(default)]
    locked_seat: String,
}

#[derive(Debug, Deserialize)]
struct ClosenessCsvRow {
    left_id: String,
    right_id: String,
    score: String,
}

#[derive(Debug, Deserialize)]
struct TableCsvRow {
    table_type_id: String,
    shape: String,
    max_people: String,
    #[serde(default)]
    recommended_people: String,
    #[serde(default)]
    min_people: String,
    #[serde(default)]
    number_of_tables: String,
    #[serde(default)]
    people_per_side: String,
}

#[derive(Debug, Deserialize)]
struct SeatingCsvRow {
    table_number: usize,
    table_type: String,
    seat_index: usize,
    person_id: String,
    person_name: String,
}

// ── Internal serialization structs ────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct PersonCsvOut<'a> {
    id: &'a str,
    name: &'a str,
    table_type: &'a str,
    groups: String,
    locked_table: String,
    locked_seat: String,
}

#[derive(Debug, Serialize)]
struct ClosenessCsvOut<'a> {
    left_id: &'a str,
    right_id: &'a str,
    score: f64,
}

#[derive(Debug, Serialize)]
struct TableCsvOut<'a> {
    table_type_id: &'a str,
    shape: &'a str,
    max_people: usize,
    recommended_people: String,
    min_people: String,
    number_of_tables: String,
    people_per_side: String,
}

#[derive(Debug, Serialize)]
struct SeatingCsvOut<'a> {
    table_number: usize,
    table_type: &'a str,
    seat_index: usize,
    person_id: &'a str,
    person_name: &'a str,
}

// ── Parsing ───────────────────────────────────────────────────────────────────

/// Parse the people CSV into a list of [`Person`] records.
///
/// Fields `table_type`, `groups`, `locked_table`, and `locked_seat` are
/// optional; an empty string is treated as absent.
///
/// # Errors
/// Returns [`ValidationError::MalformedInput`] on any CSV or field-parsing error.
pub fn parse_people_csv(input: &str) -> Result<Vec<Person>, ValidationError> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(input.as_bytes());
    let mut people = Vec::new();
    for row in rdr.deserialize::<PersonCsvRow>() {
        let row = row.map_err(|e| ValidationError::MalformedInput(format!("people CSV: {e}")))?;
        let locked_table = parse_optional_usize(&row.locked_table, "locked_table")?;
        let locked_seat = parse_optional_usize(&row.locked_seat, "locked_seat")?;
        let groups = parse_pipe_separated(&row.groups);
        people.push(Person {
            id: row.id,
            name: row.name,
            table_type: nonempty(row.table_type.trim()),
            groups,
            locked_table,
            locked_seat,
        });
    }
    Ok(people)
}

/// Parse the closeness CSV into a list of [`ClosenessRule`] records.
///
/// # Errors
/// Returns [`ValidationError::MalformedInput`] on any CSV or float-parsing error.
pub fn parse_closeness_csv(input: &str) -> Result<Vec<ClosenessRule>, ValidationError> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(input.as_bytes());
    let mut rules = Vec::new();
    for row in rdr.deserialize::<ClosenessCsvRow>() {
        let row =
            row.map_err(|e| ValidationError::MalformedInput(format!("closeness CSV: {e}")))?;
        let score = row.score.parse::<f64>().map_err(|e| {
            ValidationError::MalformedInput(format!("invalid closeness score '{}': {e}", row.score))
        })?;
        rules.push(ClosenessRule {
            left_id: row.left_id,
            right_id: row.right_id,
            score,
        });
    }
    Ok(rules)
}

/// Parse the tables CSV file into a map of type name → [`TableTypeConfig`].
///
/// # Errors
/// Returns [`ValidationError::MalformedInput`] on any CSV or field-parsing error.
pub fn parse_tables_csv(
    input: &str,
) -> Result<BTreeMap<TableTypeId, TableTypeConfig>, ValidationError> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(input.as_bytes());
    let mut tables = BTreeMap::new();
    for row in rdr.deserialize::<TableCsvRow>() {
        let row = row.map_err(|e| ValidationError::MalformedInput(format!("tables CSV: {e}")))?;
        let table_type_id = row.table_type_id.trim().to_string();
        if table_type_id.is_empty() {
            return Err(ValidationError::EmptyTableTypeId);
        }
        let shape = parse_table_shape(&row.shape)?;
        let max_people = parse_required_usize(&row.max_people, "max_people")?;
        let recommended_people =
            parse_optional_usize(&row.recommended_people, "recommended_people")?;
        let min_people = parse_optional_usize(&row.min_people, "min_people")?;
        let number_of_tables = parse_optional_usize(&row.number_of_tables, "number_of_tables")?;
        let people_per_side = parse_optional_usize_list(&row.people_per_side, "people_per_side")?;
        if tables
            .insert(
                table_type_id.clone(),
                TableTypeConfig {
                    shape,
                    people_per_side,
                    max_people,
                    recommended_people,
                    min_people,
                    number_of_tables,
                },
            )
            .is_some()
        {
            return Err(ValidationError::DuplicateTableTypeId(table_type_id));
        }
    }
    Ok(tables)
}

/// Parse the seating output CSV into a list of [`SeatingAssignment`] records.
///
/// # Errors
/// Returns [`ValidationError::MalformedInput`] on any CSV or field error.
pub fn parse_seating_csv(input: &str) -> Result<Vec<SeatingAssignment>, ValidationError> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(input.as_bytes());
    let mut rows = Vec::new();
    for row in rdr.deserialize::<SeatingCsvRow>() {
        let row = row.map_err(|e| ValidationError::MalformedInput(format!("seating CSV: {e}")))?;
        rows.push(SeatingAssignment {
            table_number: row.table_number,
            table_type: row.table_type,
            seat_index: row.seat_index,
            person_id: row.person_id,
            person_name: row.person_name,
        });
    }
    Ok(rows)
}

// ── Serialization ─────────────────────────────────────────────────────────────

/// Serialize a list of [`Person`] records to a UTF-8 CSV string.
///
/// # Errors
/// Returns [`ValidationError::MalformedInput`] on any serialization error.
pub fn write_people_csv(people: &[Person]) -> Result<String, ValidationError> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    for p in people {
        wtr.serialize(PersonCsvOut {
            id: &p.id,
            name: &p.name,
            table_type: p.table_type.as_deref().unwrap_or(""),
            groups: p.groups.join("|"),
            locked_table: p.locked_table.map(|v| v.to_string()).unwrap_or_default(),
            locked_seat: p.locked_seat.map(|v| v.to_string()).unwrap_or_default(),
        })
        .map_err(|e| ValidationError::MalformedInput(format!("people CSV serialization: {e}")))?;
    }
    finish_writer(wtr, "people CSV")
}

/// Serialize a list of [`ClosenessRule`] records to a UTF-8 CSV string.
///
/// # Errors
/// Returns [`ValidationError::MalformedInput`] on any serialization error.
pub fn write_closeness_csv(rules: &[ClosenessRule]) -> Result<String, ValidationError> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    for r in rules {
        wtr.serialize(ClosenessCsvOut {
            left_id: &r.left_id,
            right_id: &r.right_id,
            score: r.score,
        })
        .map_err(|e| {
            ValidationError::MalformedInput(format!("closeness CSV serialization: {e}"))
        })?;
    }
    finish_writer(wtr, "closeness CSV")
}

/// Serialize a table-type configuration map to a UTF-8 CSV string.
///
/// # Errors
/// Returns [`ValidationError::MalformedInput`] on any serialization error.
pub fn write_tables_csv(
    tables: &BTreeMap<TableTypeId, TableTypeConfig>,
) -> Result<String, ValidationError> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    for (table_type_id, table) in tables {
        wtr.serialize(TableCsvOut {
            table_type_id,
            shape: table_shape_label(&table.shape),
            max_people: table.max_people,
            recommended_people: table
                .recommended_people
                .map(|value| value.to_string())
                .unwrap_or_default(),
            min_people: table
                .min_people
                .map(|value| value.to_string())
                .unwrap_or_default(),
            number_of_tables: table
                .number_of_tables
                .map(|value| value.to_string())
                .unwrap_or_default(),
            people_per_side: table
                .people_per_side
                .as_ref()
                .map(|values| {
                    values
                        .iter()
                        .map(|value| value.to_string())
                        .collect::<Vec<_>>()
                        .join("|")
                })
                .unwrap_or_default(),
        })
        .map_err(|e| ValidationError::MalformedInput(format!("tables CSV serialization: {e}")))?;
    }
    finish_writer(wtr, "tables CSV")
}

/// Serialize a list of [`SeatingAssignment`] records to a UTF-8 CSV string.
///
/// The rows are sorted by (table_number, seat_index, person_id) to give a
/// deterministic, human-readable output regardless of optimizer ordering.
///
/// # Errors
/// Returns [`ValidationError::MalformedInput`] on any serialization error.
pub fn write_seating_csv(assignments: &[SeatingAssignment]) -> Result<String, ValidationError> {
    let mut sorted = assignments.to_vec();
    sorted.sort_by(|a, b| {
        a.table_number
            .cmp(&b.table_number)
            .then(a.seat_index.cmp(&b.seat_index))
            .then(a.person_id.cmp(&b.person_id))
    });
    let mut wtr = csv::Writer::from_writer(vec![]);
    for a in &sorted {
        wtr.serialize(SeatingCsvOut {
            table_number: a.table_number,
            table_type: &a.table_type,
            seat_index: a.seat_index,
            person_id: &a.person_id,
            person_name: &a.person_name,
        })
        .map_err(|e| ValidationError::MalformedInput(format!("seating CSV serialization: {e}")))?;
    }
    finish_writer(wtr, "seating CSV")
}

// ── Convenience constructor ───────────────────────────────────────────────────

/// Parse all three input sources and assemble a [`ProjectInput`].
///
/// # Errors
/// Propagates any parsing error from the individual parse functions.
pub fn make_project(
    people_csv: &str,
    closeness_csv: &str,
    tables_csv: &str,
) -> Result<ProjectInput, ValidationError> {
    Ok(ProjectInput {
        people: parse_people_csv(people_csv)?,
        closeness_rules: parse_closeness_csv(closeness_csv)?,
        table_types: parse_tables_csv(tables_csv)?,
    })
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Convert a CSV writer's internal buffer to a UTF-8 string.
fn finish_writer(wtr: csv::Writer<Vec<u8>>, label: &str) -> Result<String, ValidationError> {
    let bytes = wtr
        .into_inner()
        .map_err(|e| ValidationError::MalformedInput(format!("{label} finalize: {e}")))?;
    String::from_utf8(bytes)
        .map_err(|e| ValidationError::MalformedInput(format!("{label} utf8: {e}")))
}

/// Parse a string as `usize`, returning `None` for empty strings.
fn parse_optional_usize(s: &str, field: &str) -> Result<Option<usize>, ValidationError> {
    if s.is_empty() {
        Ok(None)
    } else {
        s.parse::<usize>()
            .map(Some)
            .map_err(|e| ValidationError::MalformedInput(format!("invalid {field} '{s}': {e}")))
    }
}

fn parse_required_usize(s: &str, field: &str) -> Result<usize, ValidationError> {
    if s.is_empty() {
        Err(ValidationError::MalformedInput(format!(
            "missing required {field}"
        )))
    } else {
        s.parse::<usize>()
            .map_err(|e| ValidationError::MalformedInput(format!("invalid {field} '{s}': {e}")))
    }
}

fn parse_optional_usize_list(s: &str, field: &str) -> Result<Option<Vec<usize>>, ValidationError> {
    if s.is_empty() {
        return Ok(None);
    }
    let mut values = Vec::new();
    for value in s.split('|') {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        values.push(trimmed.parse::<usize>().map_err(|e| {
            ValidationError::MalformedInput(format!("invalid {field} value '{trimmed}': {e}"))
        })?);
    }
    Ok(Some(values))
}

fn parse_table_shape(s: &str) -> Result<TableShape, ValidationError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "" | "round" => Ok(TableShape::Round),
        "rectangular" | "rectangle" | "rect" => Ok(TableShape::Rectangular),
        "square" => Ok(TableShape::Square),
        other => Err(ValidationError::MalformedInput(format!(
            "invalid table shape '{other}'"
        ))),
    }
}

fn table_shape_label(shape: &TableShape) -> &'static str {
    match shape {
        TableShape::Round => "round",
        TableShape::Rectangular => "rectangular",
        TableShape::Square => "square",
    }
}

/// Split a pipe-separated string into trimmed, non-empty tokens.
fn parse_pipe_separated(s: &str) -> Vec<String> {
    if s.is_empty() {
        vec![]
    } else {
        s.split('|')
            .filter(|g| !g.trim().is_empty())
            .map(|g| g.trim().to_string())
            .collect()
    }
}

/// Return `Some(s.to_string())` if `s` is non-empty, otherwise `None`.
fn nonempty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
