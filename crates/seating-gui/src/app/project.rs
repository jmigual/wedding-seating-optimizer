use super::*;

impl GuiApp {
    /// Set the single message-bar line and its severity. Multi-error
    /// `ValidationReport`s should be summarized to one line before calling
    /// this (see [`Self::report_summary`]) — the bar never inlines a full
    /// report; that detail lives in the Diagnostics tab.
    pub(super) fn set_message(&mut self, kind: MessageKind, text: impl Into<String>) {
        self.message = text.into();
        self.message_kind = kind;
    }

    /// Summarize a `ValidationReport` to one line: the lone error verbatim,
    /// or an error count pointing at the Diagnostics tab.
    pub(super) fn report_summary(report: &ValidationReport) -> String {
        match report.errors.as_slice() {
            [error] => error.to_string(),
            errors => format!("{} validation errors — see Diagnostics", errors.len()),
        }
    }

    pub(super) fn people_data(&self) -> Vec<Person> {
        self.people.iter().map(|row| row.person.clone()).collect()
    }

    pub(super) fn materialize_closeness_rules(
        &self,
    ) -> Result<Vec<ClosenessRule>, ValidationReport> {
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

    pub(super) fn materialize_table_entries(
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
            let people_per_side = if row.shape == ShapeChoice::Round {
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
                    shape: TableShape::from(row.shape),
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

    pub(super) fn current_project(&self) -> Result<ProjectInput, ValidationReport> {
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
                people: self.people_data(),
                closeness_rules,
                table_types,
            })
        } else {
            Err(ValidationReport { errors })
        }
    }

    pub(super) fn optimization_config(&self) -> Result<OptimizationConfig, ValidationReport> {
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

    pub(super) fn current_project_file(&self) -> Result<ProjectFile, ValidationReport> {
        let mut errors = Vec::new();
        let project = match self.current_project() {
            Ok(project) => Some(project),
            Err(report) => {
                errors.extend(report.errors);
                None
            }
        };
        let optimization = match self.optimization_config() {
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

    pub(super) fn apply_project_file(&mut self, project: ProjectFile) {
        self.people = project
            .people
            .into_iter()
            .map(PersonRowState::from)
            .collect();
        self.closeness_rules = project
            .closeness_rules
            .into_iter()
            .map(ClosenessRowState::from)
            .collect();
        self.table_configs = project
            .table_types
            .into_iter()
            .map(|(table_type_id, config)| TableConfigState::from_pair(table_type_id, config))
            .collect();
        self.assignments = project.seating;
        self.seed = project.optimization.seed.to_string();
        self.attempts = project.optimization.attempts.to_string();
        self.iterations = project.optimization.iterations.to_string();
        self.solutions = project.optimization.solutions.to_string();
        self.proximity_weight = project.optimization.proximity_weight.to_string();
        self.used_table_weight = project.optimization.used_table_weight.to_string();
        self.optimal_table_size_weight = project.optimization.optimal_table_size_weight.to_string();
        self.people_path = None;
        self.closeness_path = None;
        self.tables_path = None;
        self.seating_path = None;
        self.recompute_validation_and_layout();
        self.dirty = false;
    }

    /// Recompute derived state (seating CSV preview, validation, generated
    /// table lookups, seating-plan layout) from the current editable fields.
    ///
    /// Does not touch `dirty` — callers that represent a user edit should go
    /// through [`Self::refresh_validation_and_layout`] instead, which marks
    /// the project dirty before delegating here.
    pub(super) fn recompute_validation_and_layout(&mut self) {
        self.layout = None;
        self.layout_svg = None;
        self.layout_error = None;
        let project = match self.current_project() {
            Ok(project) => project,
            Err(report) => {
                self.validation_errors = report.errors;
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

        self.validation_errors = validate_project(&project)
            .err()
            .map(|report| report.errors)
            .unwrap_or_default();
        if self.assignments.is_empty() || !self.validation_errors.is_empty() {
            return;
        }
        match build_layout(&project, &self.assignments) {
            Ok(layout) => {
                let svg_markup = render_svg(&layout, &RenderOptions::default());
                self.layout = Some(layout);
                self.layout_svg = Some(svg_markup);
            }
            Err(report) => {
                self.set_message(
                    MessageKind::Error,
                    format!("Seating plan render failed: {report}"),
                );
                self.layout_error = Some(report.to_string());
            }
        }
    }

    pub(super) fn refresh_validation_and_layout(&mut self) {
        self.dirty = true;
        self.recompute_validation_and_layout();
    }

    pub(super) fn open_project(&mut self) {
        match Self::load_text_file() {
            Ok(Some((path, contents))) => match parse_project_file(&contents) {
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
            Ok(None) => {}
            Err(error) => {
                self.set_message(MessageKind::Error, format!("Project open failed: {error}"))
            }
        }
    }

    pub(super) fn save_project(&mut self, save_as: bool) {
        let Some(path) = self.resolve_save_path(save_as, &self.project_path, "wedding.wseat")
        else {
            return;
        };
        match self.current_project_file() {
            Ok(project) => {
                let serialized = write_project_file(&project).map_err(|error| error.to_string());
                if self.save_output(&path, serialized, "project", false) {
                    self.project_path = Some(path);
                    self.dirty = false;
                }
            }
            Err(report) => self.set_message(
                MessageKind::Error,
                format!("Project save failed: {}", Self::report_summary(&report)),
            ),
        }
    }

    /// Resolve a folder path for CSV export, prompting for confirmation
    /// (via a repeated `Msg::ExportProjectCsv`) if it would overwrite
    /// existing people/closeness/tables CSV files.
    fn resolve_export_csv_folder(&mut self) -> Option<PathBuf> {
        if let Some(PendingConfirm::ExportCsv(folder)) = self.pending_confirm.take() {
            return Some(folder);
        }
        let folder = FileDialog::new().pick_folder()?;
        let would_overwrite = ["people.csv", "closeness.csv", "tables.csv"]
            .iter()
            .any(|name| folder.join(name).exists());
        if would_overwrite {
            self.pending_confirm = Some(PendingConfirm::ExportCsv(folder));
            self.set_message(
                MessageKind::Info,
                "Folder already has CSV files — click Export CSVs again to overwrite.",
            );
            return None;
        }
        Some(folder)
    }

    pub(super) fn export_project_csv(&mut self) {
        let Some(folder) = self.resolve_export_csv_folder() else {
            return;
        };
        let project = match self.current_project() {
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
            match contents
                .and_then(|contents| fs::write(&path, contents).map_err(|e| e.to_string()))
            {
                Ok(_) => {}
                Err(error) => {
                    self.set_message(
                        MessageKind::Error,
                        format!("CSV export failed for {}: {error}", path.display()),
                    );
                    return;
                }
            }
        }
        self.set_message(
            MessageKind::Success,
            format!("Exported project CSV files to {}", folder.display()),
        );
    }

    pub(super) fn import_people(&mut self) {
        match Self::load_text_file() {
            Ok(Some((path, contents))) => match parse_people_csv(&contents) {
                Ok(people) => {
                    self.people = people.into_iter().map(PersonRowState::from).collect();
                    self.people_path = Some(path);
                    self.set_message(MessageKind::Success, "Imported people CSV.");
                    self.refresh_validation_and_layout();
                }
                Err(error) => {
                    self.set_message(MessageKind::Error, format!("People import failed: {error}"))
                }
            },
            Ok(None) => {}
            Err(error) => {
                self.set_message(MessageKind::Error, format!("People import failed: {error}"))
            }
        }
    }

    pub(super) fn import_closeness(&mut self) {
        match Self::load_text_file() {
            Ok(Some((path, contents))) => match parse_closeness_csv(&contents) {
                Ok(rules) => {
                    self.closeness_rules = rules.into_iter().map(ClosenessRowState::from).collect();
                    self.closeness_path = Some(path);
                    self.set_message(MessageKind::Success, "Imported closeness CSV.");
                    self.refresh_validation_and_layout();
                }
                Err(error) => self.set_message(
                    MessageKind::Error,
                    format!("Closeness import failed: {error}"),
                ),
            },
            Ok(None) => {}
            Err(error) => self.set_message(
                MessageKind::Error,
                format!("Closeness import failed: {error}"),
            ),
        }
    }

    pub(super) fn import_tables(&mut self) {
        match Self::load_text_file() {
            Ok(Some((path, contents))) => match parse_tables_csv(&contents) {
                Ok(table_types) => {
                    self.table_configs = table_types
                        .into_iter()
                        .map(|(table_type_id, config)| {
                            TableConfigState::from_pair(table_type_id, config)
                        })
                        .collect();
                    self.tables_path = Some(path);
                    self.set_message(MessageKind::Success, "Imported tables CSV.");
                    self.refresh_validation_and_layout();
                }
                Err(error) => {
                    self.set_message(MessageKind::Error, format!("Tables import failed: {error}"))
                }
            },
            Ok(None) => {}
            Err(error) => {
                self.set_message(MessageKind::Error, format!("Tables import failed: {error}"))
            }
        }
    }

    /// Serialize `contents`, write it to `path`, and set the message bar to
    /// report the outcome. Returns whether the write succeeded, so callers
    /// can decide whether to remember `path` as the entity's saved location.
    fn save_output<E: std::fmt::Display>(
        &mut self,
        path: &std::path::Path,
        contents: Result<String, E>,
        label: &str,
        export_as: bool,
    ) -> bool {
        match contents {
            Ok(contents) => match fs::write(path, contents) {
                Ok(_) => {
                    self.set_message(
                        MessageKind::Success,
                        format!(
                            "{} {label} to {}",
                            if export_as { "Exported" } else { "Saved" },
                            path.display()
                        ),
                    );
                    true
                }
                Err(error) => {
                    self.set_message(MessageKind::Error, format!("{label} save failed: {error}"));
                    false
                }
            },
            Err(error) => {
                self.set_message(MessageKind::Error, format!("{label} save failed: {error}"));
                false
            }
        }
    }

    pub(super) fn save_people(&mut self, export_as: bool) {
        let Some(path) = self.resolve_save_path(export_as, &self.people_path, "people.csv") else {
            return;
        };
        let serialized = write_people_csv(&self.people_data()).map_err(|error| error.to_string());
        if self.save_output(&path, serialized, "people CSV", export_as) && !export_as {
            self.people_path = Some(path);
        }
    }

    pub(super) fn save_closeness(&mut self, export_as: bool) {
        let Some(path) = self.resolve_save_path(export_as, &self.closeness_path, "closeness.csv")
        else {
            return;
        };
        match self.materialize_closeness_rules() {
            Ok(rules) => {
                let serialized = write_closeness_csv(&rules).map_err(|error| error.to_string());
                if self.save_output(&path, serialized, "closeness CSV", export_as) && !export_as {
                    self.closeness_path = Some(path);
                }
            }
            Err(report) => self.set_message(
                MessageKind::Error,
                format!("Closeness save failed: {}", Self::report_summary(&report)),
            ),
        }
    }

    pub(super) fn save_tables(&mut self, export_as: bool) {
        let Some(path) = self.resolve_save_path(export_as, &self.tables_path, "tables.csv") else {
            return;
        };
        let table_map = self
            .materialize_table_entries()
            .and_then(build_table_type_map);
        match table_map {
            Ok(table_types) => {
                let serialized = write_tables_csv(&table_types).map_err(|error| error.to_string());
                if self.save_output(&path, serialized, "tables CSV", export_as) && !export_as {
                    self.tables_path = Some(path);
                }
            }
            Err(report) => self.set_message(
                MessageKind::Error,
                format!("Tables save failed: {}", Self::report_summary(&report)),
            ),
        }
    }

    pub(super) fn save_seating(&mut self) {
        if self.assignments.is_empty() {
            self.set_message(MessageKind::Error, "No seating assignment to save yet.");
            return;
        }
        let Some(path) = self.resolve_save_path(false, &self.seating_path, "seating.csv") else {
            return;
        };
        let serialized = write_seating_csv(&self.assignments).map_err(|error| error.to_string());
        if self.save_output(&path, serialized, "seating CSV", false) {
            self.seating_path = Some(path);
        }
    }

    pub(super) fn export_plan_svg(&mut self) {
        let Some(layout) = &self.layout else {
            self.set_message(
                MessageKind::Error,
                "No valid seating plan available for SVG export.",
            );
            return;
        };
        let Some(path) = FileDialog::new()
            .set_file_name("seating-plan.svg")
            .save_file()
        else {
            return;
        };
        match fs::write(&path, render_svg(layout, &RenderOptions::default())) {
            Ok(_) => self.set_message(
                MessageKind::Success,
                format!("Exported SVG to {}", path.display()),
            ),
            Err(error) => {
                self.set_message(MessageKind::Error, format!("SVG export failed: {error}"))
            }
        }
    }

    pub(super) fn export_plan_png(&mut self) {
        let Some(layout) = &self.layout else {
            self.set_message(
                MessageKind::Error,
                "No valid seating plan available for PNG export.",
            );
            return;
        };
        let Some(path) = FileDialog::new()
            .set_file_name("seating-plan.png")
            .save_file()
        else {
            return;
        };
        match render_png(layout, &RenderOptions::default(), &path) {
            Ok(_) => self.set_message(
                MessageKind::Success,
                format!("Exported PNG to {}", path.display()),
            ),
            Err(error) => {
                self.set_message(MessageKind::Error, format!("PNG export failed: {error}"))
            }
        }
    }

    fn load_text_file() -> Result<Option<(PathBuf, String)>, String> {
        if let Some(path) = FileDialog::new().pick_file() {
            let contents = fs::read_to_string(&path).map_err(|error| error.to_string())?;
            Ok(Some((path, contents)))
        } else {
            Ok(None)
        }
    }

    fn resolve_save_path(
        &self,
        save_as: bool,
        current_path: &Option<PathBuf>,
        default_name: &str,
    ) -> Option<PathBuf> {
        if save_as {
            FileDialog::new().set_file_name(default_name).save_file()
        } else {
            current_path
                .clone()
                .or_else(|| FileDialog::new().set_file_name(default_name).save_file())
        }
    }

    pub(super) fn person_table_type_options(
        &self,
        current: &Option<String>,
    ) -> Vec<MaybeStringChoice> {
        let mut values = self
            .table_configs
            .iter()
            .filter_map(|row| {
                let id = row.table_type_id.trim();
                if id.is_empty() {
                    None
                } else {
                    Some(id.to_string())
                }
            })
            .collect::<Vec<_>>();
        values.sort();
        values.dedup();
        let mut choices = vec![MaybeStringChoice {
            label: "No restriction".to_string(),
            value: None,
        }];
        choices.extend(values.into_iter().map(|value| MaybeStringChoice {
            label: value.clone(),
            value: Some(value),
        }));
        if let Some(current) = current {
            if choices
                .iter()
                .all(|choice| choice.value.as_ref() != Some(current))
            {
                choices.push(MaybeStringChoice {
                    label: format!("⚠ unknown: {current}"),
                    value: Some(current.clone()),
                });
            }
        }
        choices
    }

    pub(super) fn locked_table_options(&self, current: Option<usize>) -> Vec<MaybeUsizeChoice> {
        let mut choices = vec![MaybeUsizeChoice {
            label: "No lock".to_string(),
            value: None,
        }];
        choices.extend(
            self.generated_table_numbers
                .iter()
                .map(|&number| MaybeUsizeChoice {
                    label: format!("Table {number}"),
                    value: Some(number),
                }),
        );
        if let Some(current) = current {
            if choices.iter().all(|choice| choice.value != Some(current)) {
                choices.push(MaybeUsizeChoice {
                    label: format!("⚠ invalid table {current}"),
                    value: Some(current),
                });
            }
        }
        choices
    }

    pub(super) fn locked_seat_options(&self, person: &Person) -> Vec<MaybeUsizeChoice> {
        let mut choices = vec![MaybeUsizeChoice {
            label: "No seat lock".to_string(),
            value: None,
        }];
        if let Some(table_number) = person.locked_table {
            if let Some(&capacity) = self.table_capacities.get(&table_number) {
                choices.extend((0..capacity).map(|seat| MaybeUsizeChoice {
                    label: format!("Seat {seat}"),
                    value: Some(seat),
                }));
            }
        }
        if let Some(current) = person.locked_seat {
            if choices.iter().all(|choice| choice.value != Some(current)) {
                choices.push(MaybeUsizeChoice {
                    label: format!("⚠ invalid seat {current}"),
                    value: Some(current),
                });
            }
        }
        choices
    }

    pub(super) fn generated_table_summary(&self) -> Vec<String> {
        self.current_project()
            .ok()
            .map(|project| {
                generate_table_instances(&project)
                    .into_iter()
                    .map(|table| format!("Table {} — {}", table.number, table.table_type))
                    .collect()
            })
            .unwrap_or_else(|| {
                vec!["Generated tables are unavailable until table inputs parse.".to_string()]
            })
    }

    pub(super) fn person_errors(&self, person: &Person) -> Vec<String> {
        self.validation_errors
            .iter()
            .filter(|error| error.person_id() == Some(person.id.as_str()))
            .map(ToString::to_string)
            .collect()
    }

    pub(super) fn closeness_errors(
        &self,
        left_id: &str,
        right_id: &str,
        score_input: &str,
    ) -> Vec<String> {
        let mut errors: Vec<String> = self
            .validation_errors
            .iter()
            .filter(|error| {
                matches!(error, ValidationError::UnknownIdInCloseness(id) if id == left_id || id == right_id)
                    || error
                        .closeness_pair()
                        .is_some_and(|(left, right)| rules_match(left, right, left_id, right_id))
            })
            .map(ToString::to_string)
            .collect();
        if let Err(error) = parse_f64_value(score_input, "score") {
            errors.push(error.to_string());
        }
        errors
    }

    pub(super) fn table_errors(&self, row: &TableConfigState) -> Vec<String> {
        let mut errors: Vec<String> = self
            .validation_errors
            .iter()
            .filter(|error| {
                error.table_type_id() == Some(row.table_type_id.as_str())
                    || (matches!(error, ValidationError::EmptyTableTypeId)
                        && row.table_type_id.trim().is_empty())
            })
            .map(ToString::to_string)
            .collect();

        for (field, input) in [
            ("max_people", row.max_people_input.as_str()),
            ("min_people", row.min_people_input.as_str()),
            ("recommended_people", row.recommended_people_input.as_str()),
            ("number_of_tables", row.number_of_tables_input.as_str()),
        ] {
            let result = if field == "max_people" {
                parse_required_usize_value(input, field).map(|_| ())
            } else {
                parse_optional_usize_value(input, field).map(|_| ())
            };
            if let Err(error) = result {
                errors.push(error.to_string());
            }
        }

        if row.shape != ShapeChoice::Round {
            if let Err(error) = parse_people_per_side(&row.people_per_side_input) {
                errors.push(error.to_string());
            }
        }

        errors
    }
}
