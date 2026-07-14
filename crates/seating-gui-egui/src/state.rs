//! Shared project state: raw-string editable rows, derived (validation,
//! layout, score) state, and the file-operation methods that mutate them.
//!
//! `SharedState` is the one type both `panels::editors` and `panels::canvas`
//! receive — it carries everything the spec calls out (project fields,
//! assignments, layout, score, message, [`SharedState::refresh`]) so those
//! panel modules never need to reach into `app.rs`.

use seating_core::{
    ClosenessRule, OptimizationConfig, Person, ProjectFile, ProjectInput, ScoreBreakdown,
    SeatingAssignment, SeatingLayout, TableShape, TableTypeConfig, TableTypeId, ValidationError,
    ValidationReport, build_layout, build_table_type_map, generate_table_instances,
    merge_closeness_rules, merge_people, merge_table_types, parse_closeness_csv, parse_f64_value,
    parse_optional_usize_value, parse_people_csv, parse_people_per_side, parse_project_file,
    parse_required_usize_value, parse_tables_csv, score_solution_breakdown, validate_project,
    write_closeness_csv, write_people_csv, write_project_file, write_tables_csv,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// Severity of [`SharedState`]'s single status-bar message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum MessageKind {
    #[default]
    Info,
    Success,
    Error,
}

/// The user's choice when resolving a [`PendingImport`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportDecision {
    Replace,
    Merge,
    Cancel,
}

/// A successfully-parsed CSV import awaiting a Replace/Merge/Cancel
/// decision from the user before it is applied to project state.
#[derive(Debug)]
pub(crate) enum PendingImport {
    People {
        path: PathBuf,
        people: Vec<Person>,
    },
    Closeness {
        path: PathBuf,
        rules: Vec<ClosenessRule>,
    },
    Tables {
        path: PathBuf,
        tables: BTreeMap<TableTypeId, TableTypeConfig>,
    },
}

/// One editable closeness-rule row with raw string inputs, parsed at
/// materialize time.
#[derive(Debug, Clone)]
pub(crate) struct ClosenessRow {
    pub(crate) left_id: String,
    pub(crate) right_id: String,
    pub(crate) score_input: String,
}

impl From<ClosenessRule> for ClosenessRow {
    fn from(rule: ClosenessRule) -> Self {
        Self {
            left_id: rule.left_id,
            right_id: rule.right_id,
            score_input: rule.score.to_string(),
        }
    }
}

/// One editable table-type row with raw string inputs for numeric fields,
/// parsed at materialize time.
#[derive(Debug, Clone)]
pub(crate) struct TableConfigRow {
    pub(crate) table_type_id: String,
    pub(crate) shape: TableShape,
    pub(crate) max_people_input: String,
    pub(crate) min_people_input: String,
    pub(crate) recommended_people_input: String,
    pub(crate) number_of_tables_input: String,
    pub(crate) people_per_side_input: String,
}

impl TableConfigRow {
    fn from_pair(table_type_id: String, config: TableTypeConfig) -> Self {
        let people_per_side_input = config
            .people_per_side
            .as_ref()
            .map(|values| {
                values
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .unwrap_or_default();
        Self {
            table_type_id,
            shape: config.shape,
            max_people_input: config.max_people.to_string(),
            min_people_input: config
                .min_people
                .map(|value| value.to_string())
                .unwrap_or_default(),
            recommended_people_input: config
                .recommended_people
                .map(|value| value.to_string())
                .unwrap_or_default(),
            number_of_tables_input: config
                .number_of_tables
                .map(|value| value.to_string())
                .unwrap_or_default(),
            people_per_side_input,
        }
    }
}

/// Project data, editable-row state, and derived (validation/layout/score)
/// state shared by every panel.
pub(crate) struct SharedState {
    pub(crate) people: Vec<Person>,
    pub(crate) closeness_rules: Vec<ClosenessRow>,
    pub(crate) table_configs: Vec<TableConfigRow>,
    pub(crate) assignments: Vec<SeatingAssignment>,
    pub(crate) layout: Option<SeatingLayout>,
    pub(crate) validation: Vec<ValidationError>,
    pub(crate) generated_table_numbers: Vec<usize>,
    pub(crate) table_capacities: BTreeMap<usize, usize>,
    pub(crate) score: Option<f64>,
    pub(crate) score_breakdown: Option<ScoreBreakdown>,
    pub(crate) seed: String,
    pub(crate) attempts: String,
    pub(crate) iterations: String,
    pub(crate) solutions: String,
    pub(crate) proximity_weight: String,
    pub(crate) used_table_weight: String,
    pub(crate) optimal_table_size_weight: String,
    pub(crate) message: String,
    pub(crate) message_kind: MessageKind,
    pub(crate) project_path: Option<PathBuf>,
    pub(crate) dirty: bool,
    pub(crate) pending_import: Option<PendingImport>,
}

impl SharedState {
    /// Build an empty, freshly-validated project (used at startup and by
    /// "New Project").
    pub(crate) fn empty() -> Self {
        let defaults = OptimizationConfig::default();
        let mut state = Self {
            people: Vec::new(),
            closeness_rules: Vec::new(),
            table_configs: Vec::new(),
            assignments: Vec::new(),
            layout: None,
            validation: Vec::new(),
            generated_table_numbers: Vec::new(),
            table_capacities: BTreeMap::new(),
            score: None,
            score_breakdown: None,
            seed: defaults.seed.to_string(),
            attempts: defaults.attempts.to_string(),
            iterations: defaults.iterations.to_string(),
            solutions: defaults.solutions.to_string(),
            proximity_weight: defaults.proximity_weight.to_string(),
            used_table_weight: defaults.used_table_weight.to_string(),
            optimal_table_size_weight: defaults.optimal_table_size_weight.to_string(),
            message: "Create a new project or open a .wseat file.".to_string(),
            message_kind: MessageKind::Info,
            project_path: None,
            dirty: false,
            pending_import: None,
        };
        state.recompute();
        state
    }

    pub(crate) fn set_message(&mut self, kind: MessageKind, text: impl Into<String>) {
        self.message = text.into();
        self.message_kind = kind;
    }

    pub(crate) fn report_summary(report: &ValidationReport) -> String {
        match report.errors.as_slice() {
            [error] => error.to_string(),
            errors => format!("{} validation errors", errors.len()),
        }
    }

    fn materialize_closeness_rules(&self) -> Result<Vec<ClosenessRule>, ValidationReport> {
        let mut errors = Vec::new();
        let mut rules = Vec::new();
        for row in &self.closeness_rules {
            match parse_f64_value(&row.score_input, "score") {
                Ok(score) => rules.push(ClosenessRule {
                    left_id: row.left_id.trim().to_string(),
                    right_id: row.right_id.trim().to_string(),
                    score,
                }),
                Err(error) => errors.push(error),
            }
        }
        if errors.is_empty() {
            Ok(rules)
        } else {
            Err(ValidationReport { errors })
        }
    }

    fn materialize_table_entries(
        &self,
    ) -> Result<Vec<(String, TableTypeConfig)>, ValidationReport> {
        let mut errors = Vec::new();
        let mut entries = Vec::new();
        for row in &self.table_configs {
            let max_people = match parse_required_usize_value(&row.max_people_input, "max_people") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            let min_people = match parse_optional_usize_value(&row.min_people_input, "min_people") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(error);
                    None
                }
            };
            let recommended_people = match parse_optional_usize_value(
                &row.recommended_people_input,
                "recommended_people",
            ) {
                Ok(value) => value,
                Err(error) => {
                    errors.push(error);
                    None
                }
            };
            let number_of_tables =
                match parse_optional_usize_value(&row.number_of_tables_input, "number_of_tables") {
                    Ok(value) => value,
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                };
            let people_per_side = if row.shape == TableShape::Round {
                None
            } else {
                match parse_people_per_side(&row.people_per_side_input) {
                    Ok(value) => value,
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                }
            };
            entries.push((
                row.table_type_id.trim().to_string(),
                TableTypeConfig {
                    shape: row.shape.clone(),
                    people_per_side,
                    max_people,
                    recommended_people,
                    min_people,
                    number_of_tables,
                },
            ));
        }
        if errors.is_empty() {
            Ok(entries)
        } else {
            Err(ValidationReport { errors })
        }
    }

    pub(crate) fn materialize_project(&self) -> Result<ProjectInput, ValidationReport> {
        let mut errors = Vec::new();

        let closeness_rules = match self.materialize_closeness_rules() {
            Ok(rules) => rules,
            Err(report) => {
                errors.extend(report.errors);
                Vec::new()
            }
        };
        let table_entries = match self.materialize_table_entries() {
            Ok(entries) => entries,
            Err(report) => {
                errors.extend(report.errors);
                Vec::new()
            }
        };
        let table_types = match build_table_type_map(table_entries) {
            Ok(table_types) => table_types,
            Err(report) => {
                errors.extend(report.errors);
                BTreeMap::new()
            }
        };

        if errors.is_empty() {
            Ok(ProjectInput {
                people: self.people.clone(),
                closeness_rules,
                table_types,
            })
        } else {
            Err(ValidationReport { errors })
        }
    }

    pub(crate) fn materialize_optimization_config(
        &self,
    ) -> Result<OptimizationConfig, ValidationReport> {
        let defaults = OptimizationConfig::default();
        let mut errors = Vec::new();
        let seed = match parse_optional_usize_value(&self.seed, "seed") {
            Ok(Some(value)) => value as u64,
            Ok(None) => defaults.seed,
            Err(error) => {
                errors.push(error);
                defaults.seed
            }
        };
        let attempts = match parse_optional_usize_value(&self.attempts, "attempts") {
            Ok(Some(value)) => value,
            Ok(None) => defaults.attempts,
            Err(error) => {
                errors.push(error);
                defaults.attempts
            }
        };
        let iterations = match parse_optional_usize_value(&self.iterations, "iterations") {
            Ok(Some(value)) => value,
            Ok(None) => defaults.iterations,
            Err(error) => {
                errors.push(error);
                defaults.iterations
            }
        };
        let solutions = match parse_optional_usize_value(&self.solutions, "solutions") {
            Ok(Some(value)) => value,
            Ok(None) => defaults.solutions,
            Err(error) => {
                errors.push(error);
                defaults.solutions
            }
        };
        let proximity_weight = match parse_f64_value(&self.proximity_weight, "proximity_weight") {
            Ok(value) => value,
            Err(error) => {
                errors.push(error);
                defaults.proximity_weight
            }
        };
        let used_table_weight = match parse_f64_value(&self.used_table_weight, "used_table_weight")
        {
            Ok(value) => value,
            Err(error) => {
                errors.push(error);
                defaults.used_table_weight
            }
        };
        let optimal_table_size_weight =
            match parse_f64_value(&self.optimal_table_size_weight, "optimal_table_size_weight") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(error);
                    defaults.optimal_table_size_weight
                }
            };

        if errors.is_empty() {
            Ok(OptimizationConfig {
                seed,
                attempts,
                iterations,
                solutions,
                proximity_weight,
                used_table_weight,
                optimal_table_size_weight,
            })
        } else {
            Err(ValidationReport { errors })
        }
    }

    fn materialize_project_file(&self) -> Result<ProjectFile, ValidationReport> {
        let mut errors = Vec::new();
        let project = match self.materialize_project() {
            Ok(project) => Some(project),
            Err(report) => {
                errors.extend(report.errors);
                None
            }
        };
        let optimization = match self.materialize_optimization_config() {
            Ok(config) => Some(config),
            Err(report) => {
                errors.extend(report.errors);
                None
            }
        };
        match (project, optimization) {
            (Some(project), Some(optimization)) if errors.is_empty() => Ok(ProjectFile::new(
                project,
                optimization,
                self.assignments.clone(),
            )),
            _ => Err(ValidationReport { errors }),
        }
    }

    fn apply_project_file(&mut self, project: ProjectFile) {
        self.people = project.people;
        self.closeness_rules = project
            .closeness_rules
            .into_iter()
            .map(ClosenessRow::from)
            .collect();
        self.table_configs = project
            .table_types
            .into_iter()
            .map(|(table_type_id, config)| TableConfigRow::from_pair(table_type_id, config))
            .collect();
        self.assignments = project.seating;
        self.seed = project.optimization.seed.to_string();
        self.attempts = project.optimization.attempts.to_string();
        self.iterations = project.optimization.iterations.to_string();
        self.solutions = project.optimization.solutions.to_string();
        self.proximity_weight = project.optimization.proximity_weight.to_string();
        self.used_table_weight = project.optimization.used_table_weight.to_string();
        self.optimal_table_size_weight = project.optimization.optimal_table_size_weight.to_string();
        self.recompute();
        self.dirty = false;
    }

    /// Recompute derived state (generated tables, validation, layout, score)
    /// from the current editable fields. Does not touch `dirty`; user edits
    /// should go through [`Self::refresh`] instead.
    fn recompute(&mut self) {
        self.layout = None;
        self.score = None;
        self.score_breakdown = None;
        let project = match self.materialize_project() {
            Ok(project) => project,
            Err(report) => {
                self.validation = report.errors;
                self.generated_table_numbers = Vec::new();
                self.table_capacities = BTreeMap::new();
                return;
            }
        };

        let instances = generate_table_instances(&project);
        self.generated_table_numbers = instances.iter().map(|table| table.number).collect();
        self.table_capacities = instances
            .iter()
            .map(|table| (table.number, table.max_people))
            .collect();

        self.validation = validate_project(&project)
            .err()
            .map(|report| report.errors)
            .unwrap_or_default();
        if self.assignments.is_empty() || !self.validation.is_empty() {
            return;
        }

        match build_layout(&project, &self.assignments) {
            Ok(layout) => self.layout = Some(layout),
            Err(error) => self.set_message(
                MessageKind::Error,
                format!("Seating plan render failed: {error}"),
            ),
        }

        // `score_solution_breakdown` validates the solution internally; a
        // failure here (e.g. a stale assignment after an edit) just clears
        // the badge.
        let config = self.materialize_optimization_config().unwrap_or_default();
        let breakdown = score_solution_breakdown(&project, &self.assignments, &config).ok();
        self.score = breakdown.as_ref().map(|b| b.total);
        self.score_breakdown = breakdown;
    }

    /// Recompute derived state after a user edit, marking the project dirty.
    pub(crate) fn refresh(&mut self) {
        self.dirty = true;
        self.recompute();
    }

    pub(crate) fn new_project(&mut self) {
        *self = Self::empty();
        self.set_message(MessageKind::Success, "Started a new empty project.");
    }

    pub(crate) fn open_project(&mut self) {
        let Some(path) = rfd::FileDialog::new().pick_file() else {
            return;
        };
        self.open_project_path(path);
    }

    /// Open a `.wseat` project from a known path (used by the Open dialog and
    /// by a path passed on the command line).
    pub(crate) fn open_project_path(&mut self, path: std::path::PathBuf) {
        match fs::read_to_string(&path) {
            Ok(contents) => match parse_project_file(&contents) {
                Ok(project) => {
                    self.apply_project_file(project);
                    self.project_path = Some(path.clone());
                    self.set_message(
                        MessageKind::Success,
                        format!("Opened project {}", path.display()),
                    );
                }
                Err(error) => {
                    self.set_message(MessageKind::Error, format!("Project open failed: {error}"))
                }
            },
            Err(error) => {
                self.set_message(MessageKind::Error, format!("Project open failed: {error}"))
            }
        }
    }

    pub(crate) fn save_project(&mut self, save_as: bool) {
        let Some(path) = (if save_as {
            rfd::FileDialog::new()
                .set_file_name("wedding.wseat")
                .save_file()
        } else {
            self.project_path.clone().or_else(|| {
                rfd::FileDialog::new()
                    .set_file_name("wedding.wseat")
                    .save_file()
            })
        }) else {
            return;
        };
        match self.materialize_project_file() {
            Ok(project) => match write_project_file(&project) {
                Ok(contents) => match fs::write(&path, contents) {
                    Ok(()) => {
                        self.set_message(
                            MessageKind::Success,
                            format!("Saved project to {}", path.display()),
                        );
                        self.project_path = Some(path);
                        self.dirty = false;
                    }
                    Err(error) => self
                        .set_message(MessageKind::Error, format!("Project save failed: {error}")),
                },
                Err(error) => {
                    self.set_message(MessageKind::Error, format!("Project save failed: {error}"))
                }
            },
            Err(report) => self.set_message(
                MessageKind::Error,
                format!("Project save failed: {}", Self::report_summary(&report)),
            ),
        }
    }

    pub(crate) fn export_csvs(&mut self) {
        let Some(folder) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let project = match self.materialize_project() {
            Ok(project) => project,
            Err(report) => {
                self.set_message(
                    MessageKind::Error,
                    format!("CSV export failed: {}", Self::report_summary(&report)),
                );
                return;
            }
        };
        let outputs = [
            (
                folder.join("people.csv"),
                write_people_csv(&project.people).map_err(|error| error.to_string()),
            ),
            (
                folder.join("closeness.csv"),
                write_closeness_csv(&project.closeness_rules).map_err(|error| error.to_string()),
            ),
            (
                folder.join("tables.csv"),
                write_tables_csv(&project.table_types).map_err(|error| error.to_string()),
            ),
        ];
        for (path, contents) in outputs {
            let write_result =
                contents.and_then(|contents| fs::write(&path, contents).map_err(|e| e.to_string()));
            if let Err(error) = write_result {
                self.set_message(
                    MessageKind::Error,
                    format!("CSV export failed for {}: {error}", path.display()),
                );
                return;
            }
        }
        self.set_message(
            MessageKind::Success,
            format!("Exported project CSV files to {}", folder.display()),
        );
    }

    /// Prompt for a file and read it to a string, reporting an error message
    /// (and returning `None`) on a read failure. Returns `None` silently if
    /// the user cancels the dialog.
    fn pick_and_read_csv(&mut self) -> Option<(PathBuf, String)> {
        let path = rfd::FileDialog::new().pick_file()?;
        match fs::read_to_string(&path) {
            Ok(contents) => Some((path, contents)),
            Err(error) => {
                self.set_message(MessageKind::Error, format!("CSV import failed: {error}"));
                None
            }
        }
    }

    pub(crate) fn import_people_csv(&mut self) {
        let Some((path, contents)) = self.pick_and_read_csv() else {
            return;
        };
        match parse_people_csv(&contents) {
            Ok(people) => {
                self.set_message(
                    MessageKind::Info,
                    format!(
                        "Loaded {} people from {} — choose Replace or Merge.",
                        people.len(),
                        path.display()
                    ),
                );
                self.pending_import = Some(PendingImport::People { path, people });
            }
            Err(error) => self.set_message(
                MessageKind::Error,
                format!("People CSV import failed: {error}"),
            ),
        }
    }

    pub(crate) fn import_closeness_csv(&mut self) {
        let Some((path, contents)) = self.pick_and_read_csv() else {
            return;
        };
        match parse_closeness_csv(&contents) {
            Ok(rules) => {
                self.set_message(
                    MessageKind::Info,
                    format!(
                        "Loaded {} closeness rules from {} — choose Replace or Merge.",
                        rules.len(),
                        path.display()
                    ),
                );
                self.pending_import = Some(PendingImport::Closeness { path, rules });
            }
            Err(error) => self.set_message(
                MessageKind::Error,
                format!("Closeness CSV import failed: {error}"),
            ),
        }
    }

    pub(crate) fn import_tables_csv(&mut self) {
        let Some((path, contents)) = self.pick_and_read_csv() else {
            return;
        };
        match parse_tables_csv(&contents) {
            Ok(tables) => {
                self.set_message(
                    MessageKind::Info,
                    format!(
                        "Loaded {} table types from {} — choose Replace or Merge.",
                        tables.len(),
                        path.display()
                    ),
                );
                self.pending_import = Some(PendingImport::Tables { path, tables });
            }
            Err(error) => self.set_message(
                MessageKind::Error,
                format!("Tables CSV import failed: {error}"),
            ),
        }
    }

    /// Apply the user's Replace/Merge/Cancel decision for `self.pending_import`,
    /// consuming it. Returns `true` if the decision changed `self.people`
    /// (Replace or Merge on a `PendingImport::People`) — callers that keep
    /// people-indexed UI scratch state (`EditorsState::new_group_inputs`) use
    /// this to know when to resync.
    pub(crate) fn resolve_pending_import(&mut self, decision: ImportDecision) -> bool {
        let Some(pending) = self.pending_import.take() else {
            return false;
        };
        if decision == ImportDecision::Cancel {
            self.set_message(MessageKind::Info, "Import cancelled.");
            return false;
        }
        match pending {
            PendingImport::People { path, people } => {
                self.people = match decision {
                    ImportDecision::Replace => people,
                    ImportDecision::Merge => merge_people(&self.people, people),
                    ImportDecision::Cancel => unreachable!(),
                };
                self.set_message(
                    MessageKind::Success,
                    format!("Imported people from {}", path.display()),
                );
                self.refresh();
                true
            }
            PendingImport::Closeness { path, rules } => {
                match decision {
                    ImportDecision::Replace => {
                        self.closeness_rules = rules.into_iter().map(ClosenessRow::from).collect();
                        self.set_message(
                            MessageKind::Success,
                            format!("Imported closeness rules from {}", path.display()),
                        );
                        self.refresh();
                    }
                    ImportDecision::Merge => match self.materialize_closeness_rules() {
                        Ok(existing) => {
                            let merged = merge_closeness_rules(&existing, rules);
                            self.closeness_rules =
                                merged.into_iter().map(ClosenessRow::from).collect();
                            self.set_message(
                                MessageKind::Success,
                                format!("Merged closeness rules from {}", path.display()),
                            );
                            self.refresh();
                        }
                        Err(report) => {
                            self.set_message(
                                MessageKind::Error,
                                format!(
                                    "Merge failed — fix existing closeness errors first, or Replace: {}",
                                    Self::report_summary(&report)
                                ),
                            );
                            self.pending_import = Some(PendingImport::Closeness { path, rules });
                        }
                    },
                    ImportDecision::Cancel => unreachable!(),
                }
                false
            }
            PendingImport::Tables { path, tables } => {
                match decision {
                    ImportDecision::Replace => {
                        self.table_configs = tables
                            .into_iter()
                            .map(|(id, cfg)| TableConfigRow::from_pair(id, cfg))
                            .collect();
                        self.set_message(
                            MessageKind::Success,
                            format!("Imported table types from {}", path.display()),
                        );
                        self.refresh();
                    }
                    ImportDecision::Merge => {
                        match self
                            .materialize_table_entries()
                            .and_then(build_table_type_map)
                        {
                            Ok(existing) => {
                                let merged = merge_table_types(&existing, tables);
                                self.table_configs = merged
                                    .into_iter()
                                    .map(|(id, cfg)| TableConfigRow::from_pair(id, cfg))
                                    .collect();
                                self.set_message(
                                    MessageKind::Success,
                                    format!("Merged table types from {}", path.display()),
                                );
                                self.refresh();
                            }
                            Err(report) => {
                                self.set_message(
                                    MessageKind::Error,
                                    format!(
                                        "Merge failed — fix existing table errors first, or Replace: {}",
                                        Self::report_summary(&report)
                                    ),
                                );
                                self.pending_import = Some(PendingImport::Tables { path, tables });
                            }
                        }
                    }
                    ImportDecision::Cancel => unreachable!(),
                }
                false
            }
        }
    }
}
