use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Display, Formatter};

pub type PersonId = String;
pub type GroupId = String;
pub type TableTypeId = String;
pub type SeatIndex = usize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TableShape {
    Round,
    Rectangular,
    Square,
}

impl Default for TableShape {
    fn default() -> Self {
        Self::Round
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableTypeConfig {
    #[serde(default)]
    pub shape: TableShape,
    pub people_per_side: Option<Vec<usize>>,
    pub max_people: usize,
    pub recommended_people: Option<usize>,
    pub min_people: Option<usize>,
    pub number_of_tables: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: PersonId,
    pub name: String,
    pub table_type: Option<TableTypeId>,
    pub groups: Vec<GroupId>,
    pub locked_table: Option<usize>,
    pub locked_seat: Option<SeatIndex>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosenessRule {
    pub left_id: String,
    pub right_id: String,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct ProjectInput {
    pub people: Vec<Person>,
    pub closeness_rules: Vec<ClosenessRule>,
    pub table_types: BTreeMap<TableTypeId, TableTypeConfig>,
}

#[derive(Debug, Clone)]
pub struct TableInstance {
    pub number: usize,
    pub table_type: TableTypeId,
    pub shape: TableShape,
    pub max_people: usize,
    pub min_people: Option<usize>,
    pub recommended_people: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeatingAssignment {
    pub table_number: usize,
    pub table_type: TableTypeId,
    pub seat_index: SeatIndex,
    pub person_id: PersonId,
    pub person_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeatingSolution {
    pub assignments: Vec<SeatingAssignment>,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    pub seed: u64,
    pub iterations: usize,
    pub solutions: usize,
    pub recommended_capacity_weight: f64,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            iterations: 200,
            solutions: 1,
            recommended_capacity_weight: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub solutions: Vec<SeatingSolution>,
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
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

#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Duplicate person id: {0}")]
    DuplicatePersonId(String),
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
    #[error("people_per_side sum ({sum}) must match max_people ({max}) for table type '{table_type}'")]
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
}

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
struct SeatingCsvRow {
    table_number: usize,
    table_type: String,
    seat_index: usize,
    person_id: String,
    person_name: String,
}

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
struct SeatingCsvOut<'a> {
    table_number: usize,
    table_type: &'a str,
    seat_index: usize,
    person_id: &'a str,
    person_name: &'a str,
}

pub fn parse_people_csv(input: &str) -> Result<Vec<Person>, ValidationError> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(input.as_bytes());
    let mut people = Vec::new();
    for row in rdr.deserialize::<PersonCsvRow>() {
        let row = row.map_err(|e| ValidationError::MalformedInput(format!("people CSV: {e}")))?;
        let locked_table = if row.locked_table.is_empty() {
            None
        } else {
            Some(row.locked_table.parse::<usize>().map_err(|e| {
                ValidationError::MalformedInput(format!(
                    "invalid locked_table '{}': {e}",
                    row.locked_table
                ))
            })?)
        };
        let locked_seat = if row.locked_seat.is_empty() {
            None
        } else {
            Some(row.locked_seat.parse::<usize>().map_err(|e| {
                ValidationError::MalformedInput(format!("invalid locked_seat '{}': {e}", row.locked_seat))
            })?)
        };
        let groups = if row.groups.is_empty() {
            vec![]
        } else {
            row.groups
                .split('|')
                .filter(|g| !g.trim().is_empty())
                .map(|g| g.trim().to_string())
                .collect()
        };
        people.push(Person {
            id: row.id,
            name: row.name,
            table_type: if row.table_type.trim().is_empty() {
                None
            } else {
                Some(row.table_type.trim().to_string())
            },
            groups,
            locked_table,
            locked_seat,
        });
    }
    Ok(people)
}

pub fn parse_closeness_csv(input: &str) -> Result<Vec<ClosenessRule>, ValidationError> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(input.as_bytes());
    let mut rules = Vec::new();
    for row in rdr.deserialize::<ClosenessCsvRow>() {
        let row = row.map_err(|e| ValidationError::MalformedInput(format!("closeness CSV: {e}")))?;
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

pub fn parse_tables_json(input: &str) -> Result<BTreeMap<TableTypeId, TableTypeConfig>, ValidationError> {
    serde_json::from_str(input).map_err(|e| ValidationError::MalformedInput(format!("tables JSON: {e}")))
}

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
    String::from_utf8(
        wtr.into_inner()
            .map_err(|e| ValidationError::MalformedInput(format!("people CSV finalize: {e}")))?,
    )
    .map_err(|e| ValidationError::MalformedInput(format!("people CSV utf8: {e}")))
}

pub fn write_closeness_csv(rules: &[ClosenessRule]) -> Result<String, ValidationError> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    for r in rules {
        wtr.serialize(ClosenessCsvOut {
            left_id: &r.left_id,
            right_id: &r.right_id,
            score: r.score,
        })
        .map_err(|e| ValidationError::MalformedInput(format!("closeness CSV serialization: {e}")))?;
    }
    String::from_utf8(
        wtr.into_inner()
            .map_err(|e| ValidationError::MalformedInput(format!("closeness CSV finalize: {e}")))?,
    )
    .map_err(|e| ValidationError::MalformedInput(format!("closeness CSV utf8: {e}")))
}

pub fn write_tables_json(tables: &BTreeMap<TableTypeId, TableTypeConfig>) -> Result<String, ValidationError> {
    serde_json::to_string_pretty(tables)
        .map_err(|e| ValidationError::MalformedInput(format!("tables JSON serialization: {e}")))
}

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
    String::from_utf8(
        wtr.into_inner()
            .map_err(|e| ValidationError::MalformedInput(format!("seating CSV finalize: {e}")))?,
    )
    .map_err(|e| ValidationError::MalformedInput(format!("seating CSV utf8: {e}")))
}

pub fn make_project(people_csv: &str, closeness_csv: &str, tables_json: &str) -> Result<ProjectInput, ValidationError> {
    Ok(ProjectInput {
        people: parse_people_csv(people_csv)?,
        closeness_rules: parse_closeness_csv(closeness_csv)?,
        table_types: parse_tables_json(tables_json)?,
    })
}

fn canonical_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

pub fn generate_table_instances(project: &ProjectInput) -> Vec<TableInstance> {
    let max_locked = project
        .people
        .iter()
        .filter_map(|p| p.locked_table)
        .max()
        .unwrap_or(0);
    let dynamic_count = project.people.len().max(1);
    let mut instances = Vec::new();
    let mut number = 1usize;
    for (table_type_id, cfg) in &project.table_types {
        let count = match cfg.number_of_tables {
            Some(n) => n,
            None => dynamic_count.max(max_locked),
        };
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

pub fn validate_project(project: &ProjectInput) -> Result<(), ValidationReport> {
    let mut errors = Vec::new();
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
    for pid in &person_ids {
        if group_ids.contains(pid) {
            errors.push(ValidationError::NamespaceCollision(pid.clone()));
        }
    }

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

    let known_ids: HashSet<String> = person_ids
        .iter()
        .cloned()
        .chain(group_ids.iter().cloned())
        .collect();
    let mut seen_pairs = HashSet::new();
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

    let instances = generate_table_instances(project);
    let table_by_number: HashMap<usize, &TableInstance> = instances.iter().map(|t| (t.number, t)).collect();
    let total_capacity: usize = instances.iter().map(|t| t.max_people).sum();
    if total_capacity < project.people.len() {
        errors.push(ValidationError::NotEnoughSeats {
            required: project.people.len(),
            available: total_capacity,
        });
    }

    let mut locked_taken = HashSet::new();
    for p in &project.people {
        if let Some(table_num) = p.locked_table {
            if let Some(table) = table_by_number.get(&table_num) {
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
            } else {
                errors.push(ValidationError::LockedTableDoesNotExist {
                    person_id: p.id.clone(),
                    table_number: table_num,
                });
            }
        }

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

    let mut group_sizes: HashMap<String, usize> = HashMap::new();
    for p in &project.people {
        for g in &p.groups {
            *group_sizes.entry(g.clone()).or_insert(0) += 1;
        }
    }
    let max_cap = project.table_types.values().map(|t| t.max_people).max().unwrap_or(0);
    for r in &project.closeness_rules {
        if r.left_id == r.right_id && group_ids.contains(&r.left_id) && r.score >= 50.0 {
            let size = group_sizes.get(&r.left_id).copied().unwrap_or(0);
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

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationReport { errors })
    }
}

pub fn circular_distance(a: usize, b: usize, seats: usize) -> usize {
    let raw = a.abs_diff(b);
    raw.min(seats.saturating_sub(raw))
}

pub fn perimeter_distance(a: usize, b: usize, seats: usize) -> usize {
    circular_distance(a, b, seats)
}

pub fn default_proximity_weight(distance: usize) -> f64 {
    match distance {
        0 | 1 => 1.0,
        2 => 0.75,
        3 => 0.5,
        d => 1.0 / d as f64,
    }
}

fn closeness_lookup(rules: &[ClosenessRule]) -> Result<HashMap<(String, String), f64>, ValidationError> {
    let mut map = HashMap::new();
    for rule in rules {
        let key = canonical_pair(&rule.left_id, &rule.right_id);
        if map.insert(key.clone(), rule.score).is_some() {
            return Err(ValidationError::DuplicateClosenessRule(key.0, key.1));
        }
    }
    Ok(map)
}

pub fn effective_person_pair_score(
    project: &ProjectInput,
    a: &Person,
    b: &Person,
) -> Result<f64, ValidationError> {
    let closeness = closeness_lookup(&project.closeness_rules)?;
    let mut score = *closeness.get(&canonical_pair(&a.id, &b.id)).unwrap_or(&0.0);
    let mut best_group = None;
    for ga in &a.groups {
        for gb in &b.groups {
            if let Some(gs) = closeness.get(&canonical_pair(ga, gb)) {
                best_group = Some(best_group.map_or(*gs, |v: f64| v.max(*gs)));
            }
        }
    }
    if let Some(gs) = best_group {
        score += gs;
    }
    Ok(score)
}

pub fn validate_seating_solution(project: &ProjectInput, assignments: &[SeatingAssignment]) -> Result<(), ValidationReport> {
    let mut errors = validate_project(project).err().map(|r| r.errors).unwrap_or_default();
    let instances = generate_table_instances(project);
    let table_by_number: HashMap<usize, &TableInstance> = instances.iter().map(|t| (t.number, t)).collect();
    let person_map: HashMap<&str, &Person> = project.people.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut seen_people = HashSet::new();
    let mut seen_seats = HashSet::new();
    let mut occupancy: HashMap<usize, usize> = HashMap::new();

    for a in assignments {
        if !person_map.contains_key(a.person_id.as_str()) {
            errors.push(ValidationError::UnknownPersonInSeating(a.person_id.clone()));
            continue;
        }
        if !seen_people.insert(a.person_id.clone()) {
            errors.push(ValidationError::MissingOrDuplicatePerson(a.person_id.clone()));
        }
        let Some(table) = table_by_number.get(&a.table_number) else {
            errors.push(ValidationError::UnknownTableInSeating(a.table_number));
            continue;
        };
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

pub fn score_solution(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    recommended_weight: f64,
) -> Result<f64, ValidationReport> {
    validate_seating_solution(project, assignments)?;
    let instances = generate_table_instances(project);
    let table_by_number: HashMap<usize, &TableInstance> = instances.iter().map(|t| (t.number, t)).collect();
    let person_map: HashMap<&str, &Person> = project.people.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut by_table: HashMap<usize, Vec<&SeatingAssignment>> = HashMap::new();
    for a in assignments {
        by_table.entry(a.table_number).or_default().push(a);
    }

    let mut total = 0.0;
    for seated in by_table.values() {
        for i in 0..seated.len() {
            for j in i + 1..seated.len() {
                let a = seated[i];
                let b = seated[j];
                let pa = person_map.get(a.person_id.as_str()).unwrap();
                let pb = person_map.get(b.person_id.as_str()).unwrap();
                let pair_score = effective_person_pair_score(project, pa, pb)
                    .map_err(|e| ValidationReport { errors: vec![e] })?;
                let table = table_by_number.get(&a.table_number).unwrap();
                let distance = match table.shape {
                    TableShape::Round => circular_distance(a.seat_index, b.seat_index, table.max_people),
                    TableShape::Rectangular | TableShape::Square => {
                        perimeter_distance(a.seat_index, b.seat_index, table.max_people)
                    }
                };
                total += pair_score * default_proximity_weight(distance);
            }
        }
    }

    for table in &instances {
        if let Some(recommended) = table.recommended_people {
            let count = by_table.get(&table.number).map(|v| v.len()).unwrap_or(0);
            if count > 0 {
                total -= (count as isize - recommended as isize).unsigned_abs() as f64 * recommended_weight;
            }
        }
    }
    Ok(total)
}

pub trait SeatingOptimizer {
    fn optimize(&self, project: &ProjectInput, config: &OptimizationConfig)
        -> Result<OptimizationResult, ValidationReport>;
}

#[derive(Debug, Default)]
pub struct HeuristicOptimizer;

impl HeuristicOptimizer {
    fn random_feasible_assignment(&self, project: &ProjectInput, seed: u64) -> Option<Vec<SeatingAssignment>> {
        let instances = generate_table_instances(project);
        let table_lookup: HashMap<usize, &TableInstance> = instances.iter().map(|t| (t.number, t)).collect();
        let mut occupied: HashSet<(usize, usize)> = HashSet::new();
        let mut assigned: HashMap<String, (usize, usize)> = HashMap::new();

        for p in &project.people {
            if let (Some(table_num), Some(seat)) = (p.locked_table, p.locked_seat) {
                let table = table_lookup.get(&table_num)?;
                if seat >= table.max_people || !occupied.insert((table_num, seat)) {
                    return None;
                }
                assigned.insert(p.id.clone(), (table_num, seat));
            }
        }

        let mut rng = StdRng::seed_from_u64(seed);
        let mut pending: Vec<&Person> = project.people.iter().filter(|p| !assigned.contains_key(&p.id)).collect();
        pending.shuffle(&mut rng);

        for p in pending {
            let mut candidates = Vec::new();
            for t in &instances {
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
            if candidates.is_empty() {
                return None;
            }
            let chosen = candidates[rng.gen_range(0..candidates.len())];
            occupied.insert(chosen);
            assigned.insert(p.id.clone(), chosen);
        }

        let mut per_table: HashMap<usize, usize> = HashMap::new();
        for (table_num, _) in assigned.values() {
            *per_table.entry(*table_num).or_insert(0) += 1;
        }
        for table in &instances {
            if let Some(min) = table.min_people {
                let count = per_table.get(&table.number).copied().unwrap_or(0);
                if count > 0 && count < min {
                    return None;
                }
            }
        }

        Some(
            project
                .people
                .iter()
                .map(|p| {
                    let (table_number, seat_index) = assigned.get(&p.id).copied().unwrap();
                    SeatingAssignment {
                        table_number,
                        table_type: table_lookup.get(&table_number).unwrap().table_type.clone(),
                        seat_index,
                        person_id: p.id.clone(),
                        person_name: p.name.clone(),
                    }
                })
                .collect(),
        )
    }

    fn local_improve(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
        mut assignments: Vec<SeatingAssignment>,
        seed: u64,
    ) -> Vec<SeatingAssignment> {
        let mut rng = StdRng::seed_from_u64(seed);
        let locked_ids: HashSet<&str> = project
            .people
            .iter()
            .filter(|p| p.locked_seat.is_some())
            .map(|p| p.id.as_str())
            .collect();
        let mut best = score_solution(project, &assignments, config.recommended_capacity_weight)
            .unwrap_or(f64::NEG_INFINITY);

        for _ in 0..config.iterations.max(50) {
            let i = rng.gen_range(0..assignments.len());
            let j = rng.gen_range(0..assignments.len());
            if i == j
                || locked_ids.contains(assignments[i].person_id.as_str())
                || locked_ids.contains(assignments[j].person_id.as_str())
            {
                continue;
            }

            let a = assignments[i].clone();
            let b = assignments[j].clone();
            assignments[i].table_number = b.table_number;
            assignments[i].table_type = b.table_type.clone();
            assignments[i].seat_index = b.seat_index;
            assignments[j].table_number = a.table_number;
            assignments[j].table_type = a.table_type.clone();
            assignments[j].seat_index = a.seat_index;

            if validate_seating_solution(project, &assignments).is_ok() {
                if let Ok(score) = score_solution(project, &assignments, config.recommended_capacity_weight) {
                    if score > best {
                        best = score;
                        continue;
                    }
                }
            }
            assignments[i] = a;
            assignments[j] = b;
        }
        assignments
    }
}

impl SeatingOptimizer for HeuristicOptimizer {
    fn optimize(
        &self,
        project: &ProjectInput,
        config: &OptimizationConfig,
    ) -> Result<OptimizationResult, ValidationReport> {
        validate_project(project)?;
        let mut best = Vec::new();
        for attempt in 0..config.iterations.max(1) {
            let seed = config.seed.wrapping_add((attempt as u64) * 17);
            let Some(initial) = self.random_feasible_assignment(project, seed) else {
                continue;
            };
            let candidate = self.local_improve(project, config, initial, seed ^ 0xA5A5_5A5A);
            let Ok(score) = score_solution(project, &candidate, config.recommended_capacity_weight) else {
                continue;
            };
            best.push(SeatingSolution {
                assignments: candidate,
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
