//! Domain model types used throughout the seating-core crate.
//!
//! All application logic operates on the plain-data structs defined here.
//! They intentionally carry no parsing, serialization, or optimization logic.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

// ── Type aliases ──────────────────────────────────────────────────────────────

/// Unique identifier for a guest (must be distinct from all group IDs).
pub type PersonId = String;

/// Identifier for a group of guests (must be distinct from all person IDs).
pub type GroupId = String;

/// Identifier for a table type (key in the tables CSV file).
pub type TableTypeId = String;

/// Zero-based position index within a table.
pub type SeatIndex = usize;

// ── Table configuration ───────────────────────────────────────────────────────

/// Physical shape of a table, which determines how seat distances are computed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TableShape {
    /// Circular table – distance wraps around the perimeter.
    #[default]
    Round,
    /// Rectangular table – seats are ordered around all four sides.
    Rectangular,
    /// Square table – seats are ordered around all four sides (equal length).
    Square,
}

/// Configuration for one table type, as parsed from the tables CSV file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableTypeConfig {
    /// Physical shape; defaults to [`TableShape::Round`] when absent in CSV.
    #[serde(default)]
    pub shape: TableShape,
    /// Number of seats per side, required for rectangular and square tables.
    pub people_per_side: Option<Vec<usize>>,
    /// Hard capacity upper bound.
    pub max_people: usize,
    /// Soft preferred occupancy (deviations are penalized in scoring).
    pub recommended_people: Option<usize>,
    /// Hard minimum occupancy for any used table of this type.
    pub min_people: Option<usize>,
    /// Maximum number of table instances to create for this type.
    /// `None` means the optimizer may generate as many as needed.
    pub number_of_tables: Option<usize>,
}

// ── Guest model ───────────────────────────────────────────────────────────────

/// A single wedding guest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Person {
    /// Unique guest identifier.
    pub id: PersonId,
    /// Display name shown in all outputs.
    pub name: String,
    /// When present, restricts the guest to tables of this type only.
    pub table_type: Option<TableTypeId>,
    /// Groups the guest belongs to; used for group-level closeness rules.
    pub groups: Vec<GroupId>,
    /// When present, the guest must be placed at this table number.
    pub locked_table: Option<usize>,
    /// When present, the guest must occupy this exact seat index within their table.
    /// Requires [`locked_table`](Person::locked_table) to also be set.
    pub locked_seat: Option<SeatIndex>,
}

// ── Closeness rules ───────────────────────────────────────────────────────────

/// A single pairwise closeness preference between two people or groups.
///
/// Positive scores encourage proximity; negative scores encourage separation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClosenessRule {
    /// Left-hand identifier (person ID or group ID).
    pub left_id: String,
    /// Right-hand identifier (person ID or group ID).
    pub right_id: String,
    /// Arbitrary floating-point preference score.
    pub score: f64,
}

// ── Project input ─────────────────────────────────────────────────────────────

/// The complete set of inputs required for validation and optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInput {
    /// All guests.
    pub people: Vec<Person>,
    /// All pairwise closeness rules.
    pub closeness_rules: Vec<ClosenessRule>,
    /// All table type configurations keyed by type name.
    pub table_types: BTreeMap<TableTypeId, TableTypeConfig>,
}

// ── Table instances ───────────────────────────────────────────────────────────

/// A concrete table instance generated from a [`TableTypeConfig`].
///
/// The optimizer assigns guests to instances, not to types.
#[derive(Debug, Clone)]
pub struct TableInstance {
    /// Sequential table number (1-based), used in output and for locking.
    pub number: usize,
    /// Name of the table type this instance belongs to.
    pub table_type: TableTypeId,
    /// Physical shape of this table.
    pub shape: TableShape,
    /// Maximum number of guests that can sit here.
    pub max_people: usize,
    /// Minimum occupancy when the table is used (hard constraint).
    pub min_people: Option<usize>,
    /// Preferred occupancy (soft constraint, penalized in scoring).
    pub recommended_people: Option<usize>,
}

// ── Seating assignment and solution ──────────────────────────────────────────

/// Placement of one guest at a specific seat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeatingAssignment {
    /// Number of the table this guest is assigned to.
    pub table_number: usize,
    /// Type of that table.
    pub table_type: TableTypeId,
    /// Zero-based seat index within the table.
    pub seat_index: SeatIndex,
    /// Guest identifier.
    pub person_id: PersonId,
    /// Guest display name (duplicated here for easy export).
    pub person_name: String,
}

/// A complete seating arrangement with its aggregate score.
#[derive(Debug, Clone, PartialEq)]
pub struct SeatingSolution {
    /// All per-guest assignments.
    pub assignments: Vec<SeatingAssignment>,
    /// Total seating score; higher is better.
    pub score: f64,
}

// ── Optimization configuration ────────────────────────────────────────────────

/// Configuration parameters forwarded to the optimizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OptimizationConfig {
    /// RNG seed for reproducible runs.
    pub seed: u64,
    /// Number of independent random-restart attempts.
    ///
    /// More attempts increase the chance of finding a better global optimum.
    pub attempts: usize,
    /// Number of local pairwise-swap improvement steps per attempt.
    ///
    /// More iterations refine each individual attempt further.
    pub iterations: usize,
    /// How many top solutions to keep and return.
    pub solutions: usize,
    /// Global multiplier for closeness and proximity contributions.
    pub proximity_weight: f64,
    /// Penalty subtracted for every table used by a solution.
    pub used_table_weight: f64,
    /// Weight applied to the penalty for deviating from `recommended_people`.
    pub optimal_table_size_weight: f64,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            attempts: 10,
            iterations: 200,
            solutions: 1,
            proximity_weight: 1.0,
            used_table_weight: 0.0,
            optimal_table_size_weight: 1.0,
        }
    }
}

/// Schema version used by `.wseat` project files.
pub const PROJECT_FILE_VERSION: u32 = 1;

/// A complete persisted application project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    /// Project file schema version.
    pub version: u32,
    /// All guests.
    pub people: Vec<Person>,
    /// All pairwise closeness rules.
    pub closeness_rules: Vec<ClosenessRule>,
    /// All table type configurations keyed by type name.
    pub table_types: BTreeMap<TableTypeId, TableTypeConfig>,
    /// Optimization settings last used by the project.
    #[serde(default)]
    pub optimization: OptimizationConfig,
    /// Optional generated seating assignments.
    #[serde(default)]
    pub seating: Vec<SeatingAssignment>,
}

impl ProjectFile {
    /// Build a project file from a project input and runtime settings.
    pub fn new(
        project: ProjectInput,
        optimization: OptimizationConfig,
        seating: Vec<SeatingAssignment>,
    ) -> Self {
        Self {
            version: PROJECT_FILE_VERSION,
            people: project.people,
            closeness_rules: project.closeness_rules,
            table_types: project.table_types,
            optimization,
            seating,
        }
    }

    /// Return the editable project input contained in this file.
    pub fn project_input(&self) -> ProjectInput {
        ProjectInput {
            people: self.people.clone(),
            closeness_rules: self.closeness_rules.clone(),
            table_types: self.table_types.clone(),
        }
    }
}

/// The result returned by an optimizer.
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Top solutions, sorted from best to worst score.
    pub solutions: Vec<SeatingSolution>,
}

// ── Validation types ──────────────────────────────────────────────────────────

/// A collection of validation errors discovered during project or solution validation.
///
/// Implements [`std::error::Error`] so it can be propagated as an error type.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// All validation errors found.
    pub errors: Vec<ValidationError>,
}

impl Display for ValidationReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (idx, err) in self.errors.iter().enumerate() {
            if idx > 0 {
                writeln!(f)?;
            }
            write!(f, "- {err}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationReport {}

/// Individual validation error variants, each with descriptive context.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Duplicate person id: {0}")]
    DuplicatePersonId(String),
    #[error("Duplicate table type id: {0}")]
    DuplicateTableTypeId(String),
    #[error("Table type id cannot be empty")]
    EmptyTableTypeId,
    #[error("Person ID collides with group ID namespace: {0}")]
    NamespaceCollision(String),
    #[error("Unknown table type '{table_type}' referenced by person '{person_id}'")]
    UnknownTableTypeForPerson {
        person_id: String,
        table_type: String,
    },
    #[error("Unknown ID in closeness rules: {0}")]
    UnknownIdInCloseness(String),
    #[error("Duplicate closeness rule for pair: {0} <-> {1}")]
    DuplicateClosenessRule(String, String),
    #[error("Invalid closeness score for pair {left} <-> {right}: score must be finite")]
    InvalidClosenessScore { left: String, right: String },
    #[error("Locked seat requires locked table for person '{0}'")]
    LockedSeatRequiresLockedTable(String),
    #[error("Locked table {table_number} does not exist for person '{person_id}'")]
    LockedTableDoesNotExist {
        person_id: String,
        table_number: usize,
    },
    #[error("Locked seat {seat} exceeds table capacity {capacity} for person '{person_id}'")]
    LockedSeatOutOfRange {
        person_id: String,
        seat: usize,
        capacity: usize,
    },
    #[error("Table shape '{0}' requires people_per_side")]
    MissingPeoplePerSide(String),
    #[error(
        "Table type '{table_type}' must define exactly four people_per_side values, found {len}"
    )]
    InvalidPeoplePerSideLength { table_type: String, len: usize },
    #[error(
        "people_per_side sum ({sum}) must match max_people ({max}) for table type '{table_type}'"
    )]
    PeoplePerSideMismatch {
        table_type: String,
        sum: usize,
        max: usize,
    },
    #[error("Table type '{table_type}' has min_people ({min}) > max_people ({max})")]
    InvalidMinMax {
        table_type: String,
        min: usize,
        max: usize,
    },
    #[error("Table type '{table_type}' has recommended_people ({recommended}) outside the allowed range {min}..={max}")]
    InvalidRecommendedPeople {
        table_type: String,
        recommended: usize,
        min: usize,
        max: usize,
    },
    #[error(
        "Table type '{table_type}' must have number_of_tables > 0 when provided (got {count})"
    )]
    InvalidNumberOfTables { table_type: String, count: usize },
    #[error("Multiple people locked to same seat: table {table_number}, seat {seat}")]
    DuplicateLockedSeat { table_number: usize, seat: usize },
    #[error("Person '{person_id}' is locked to table {table_number} of type '{locked_type}', incompatible with required table_type '{required_type}'")]
    LockedTableTypeMismatch {
        person_id: String,
        table_number: usize,
        locked_type: String,
        required_type: String,
    },
    #[error("Not enough available seats: required {required}, available {available}")]
    NotEnoughSeats { required: usize, available: usize },
    #[error("No compatible tables available for person '{person_id}'")]
    ImpossiblePersonAssignment { person_id: String },
    #[error("Group '{group_id}' has very high self-closeness ({score}) and size ({size}), larger than any compatible table capacity ({max_compatible_capacity})")]
    LargeHighPriorityGroup {
        group_id: String,
        score: f64,
        size: usize,
        max_compatible_capacity: usize,
    },
    #[error("Each person must appear exactly once in seating: {0}")]
    MissingOrDuplicatePerson(String),
    #[error("Unknown person in seating output: {0}")]
    UnknownPersonInSeating(String),
    #[error("Unknown table in seating output: {0}")]
    UnknownTableInSeating(usize),
    #[error("Seat collision in seating output: table {table_number}, seat {seat}")]
    SeatCollision { table_number: usize, seat: usize },
    #[error("Table {table_number} exceeds capacity: {count} > {capacity}")]
    TableCapacityExceeded {
        table_number: usize,
        count: usize,
        capacity: usize,
    },
    #[error("Used table {table_number} violates min_people: {count} < {min}")]
    TableBelowMin {
        table_number: usize,
        count: usize,
        min: usize,
    },
    #[error("Malformed input: {0}")]
    MalformedInput(String),
    /// `SeatingAssignment.table_type` does not match the actual type of the resolved table instance.
    #[error("Seating assignment records table_type '{recorded_type}' for table {table_number}, but the table instance has type '{actual_type}'")]
    SeatingTableTypeMismatch {
        table_number: usize,
        actual_type: String,
        recorded_type: String,
    },
    /// The assigned table type does not satisfy the person's required table type.
    #[error("Person '{person_id}' requires table type '{required_type}' but is seated at table {table_number} of type '{assigned_type}'")]
    SeatingPersonTableTypeMismatch {
        person_id: String,
        table_number: usize,
        required_type: String,
        assigned_type: String,
    },
    /// A person with a locked table is assigned to a different table.
    #[error("Person '{person_id}' must be at table {locked_table} but is seated at table {assigned_table}")]
    SeatingViolatesLockedTable {
        person_id: String,
        locked_table: usize,
        assigned_table: usize,
    },
    /// A person with a locked seat is assigned to a different seat.
    #[error(
        "Person '{person_id}' must be at seat {locked_seat} but is placed at seat {assigned_seat}"
    )]
    SeatingViolatesLockedSeat {
        person_id: String,
        locked_seat: usize,
        assigned_seat: usize,
    },
}
