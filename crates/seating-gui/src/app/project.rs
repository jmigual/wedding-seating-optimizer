use super::*;

impl GuiApp {
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
                    left_id: row.rule.left_id.trim().to_string(),
                    right_id: row.rule.right_id.trim().to_string(),
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
                let mut side_values = Vec::new();
                for value in row.people_per_side_input.split('|') {
                    let value = value.trim();
                    if value.is_empty() {
                        continue;
                    }
                    match parse_required_usize_value(value, "people_per_side") {
                        Ok(parsed) => side_values.push(parsed),
                        Err(error) => errors.push(error),
                    }
                }
                Some(side_values)
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
        let mut errors = Vec::new();
        let seed = match parse_optional_usize_value(&self.seed, "seed") {
            Ok(Some(value)) => value as u64,
            Ok(None) => 42,
            Err(error) => {
                errors.push(error);
                42
            }
        };
        let proximity_weight = match parse_f64_value(&self.proximity_weight, "proximity_weight") {
            Ok(value) => value,
            Err(error) => {
                errors.push(error);
                1.0
            }
        };
        let used_table_weight = match parse_f64_value(&self.used_table_weight, "used_table_weight")
        {
            Ok(value) => value,
            Err(error) => {
                errors.push(error);
                0.0
            }
        };
        let optimal_table_size_weight =
            match parse_f64_value(&self.optimal_table_size_weight, "optimal_table_size_weight") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(error);
                    1.0
                }
            };

        if errors.is_empty() {
            Ok(OptimizationConfig {
                seed,
                attempts: 10,
                iterations: 200,
                solutions: 1,
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
        self.seating_csv = write_seating_csv(&self.assignments).unwrap_or_default();
        self.seed = project.optimization.seed.to_string();
        self.proximity_weight = project.optimization.proximity_weight.to_string();
        self.used_table_weight = project.optimization.used_table_weight.to_string();
        self.optimal_table_size_weight = project.optimization.optimal_table_size_weight.to_string();
        self.people_path = None;
        self.closeness_path = None;
        self.tables_path = None;
        self.seating_path = None;
        self.refresh_validation_and_layout();
    }

    pub(super) fn refresh_validation_and_layout(&mut self) {
        self.layout = None;
        self.layout_svg = None;
        self.seating_csv = write_seating_csv(&self.assignments).unwrap_or_default();
        let project = match self.current_project() {
            Ok(project) => project,
            Err(report) => {
                self.validation_errors = report.errors;
                return;
            }
        };
        self.validation_errors = validate_project(&project)
            .err()
            .map(|report| report.errors)
            .unwrap_or_default();
        if self.assignments.is_empty() || !self.validation_errors.is_empty() {
            return;
        }
        if let Ok(layout) = build_layout(&project, &self.assignments) {
            let svg_markup = render_svg(&layout, &RenderOptions::default());
            self.layout = Some(layout);
            self.layout_svg = Some(svg_markup);
        }
    }

    pub(super) fn open_project(&mut self) {
        match Self::load_text_file() {
            Ok(Some((path, contents))) => match parse_project_file(&contents) {
                Ok(project) => {
                    self.apply_project_file(project);
                    self.project_path = Some(path.clone());
                    self.message = format!("Opened project {}", path.display());
                }
                Err(error) => self.message = format!("Project open failed: {error}"),
            },
            Ok(None) => {}
            Err(error) => self.message = format!("Project open failed: {error}"),
        }
    }

    pub(super) fn save_project(&mut self, save_as: bool) {
        let Some(path) = self.resolve_save_path(save_as, &self.project_path, "wedding.wseat")
        else {
            return;
        };
        match self.current_project_file() {
            Ok(project) => match write_project_file(&project) {
                Ok(contents) => match fs::write(&path, contents) {
                    Ok(_) => {
                        self.project_path = Some(path.clone());
                        self.message = format!("Saved project to {}", path.display());
                    }
                    Err(error) => self.message = format!("Project save failed: {error}"),
                },
                Err(error) => self.message = format!("Project save failed: {error}"),
            },
            Err(report) => self.message = format!("Project save failed:\n{report}"),
        }
    }

    pub(super) fn export_project_csv(&mut self) {
        let Some(folder) = FileDialog::new().pick_folder() else {
            return;
        };
        let project = match self.current_project() {
            Ok(project) => project,
            Err(report) => {
                self.message = format!("CSV export failed:\n{report}");
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
                    self.message = format!("CSV export failed for {}: {error}", path.display());
                    return;
                }
            }
        }
        self.message = format!("Exported project CSV files to {}", folder.display());
    }

    pub(super) fn import_people(&mut self) {
        match Self::load_text_file() {
            Ok(Some((path, contents))) => match parse_people_csv(&contents) {
                Ok(people) => {
                    self.people = people.into_iter().map(PersonRowState::from).collect();
                    self.people_path = Some(path);
                    self.message = "Imported people CSV.".to_string();
                    self.refresh_validation_and_layout();
                }
                Err(error) => self.message = format!("People import failed: {error}"),
            },
            Ok(None) => {}
            Err(error) => self.message = format!("People import failed: {error}"),
        }
    }

    pub(super) fn import_closeness(&mut self) {
        match Self::load_text_file() {
            Ok(Some((path, contents))) => match parse_closeness_csv(&contents) {
                Ok(rules) => {
                    self.closeness_rules = rules.into_iter().map(ClosenessRowState::from).collect();
                    self.closeness_path = Some(path);
                    self.message = "Imported closeness CSV.".to_string();
                    self.refresh_validation_and_layout();
                }
                Err(error) => self.message = format!("Closeness import failed: {error}"),
            },
            Ok(None) => {}
            Err(error) => self.message = format!("Closeness import failed: {error}"),
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
                    self.message = "Imported tables CSV.".to_string();
                    self.refresh_validation_and_layout();
                }
                Err(error) => self.message = format!("Tables import failed: {error}"),
            },
            Ok(None) => {}
            Err(error) => self.message = format!("Tables import failed: {error}"),
        }
    }

    pub(super) fn save_people(&mut self, export_as: bool) {
        if let Some(path) = self.resolve_save_path(export_as, &self.people_path, "people.csv") {
            match write_people_csv(&self.people_data()) {
                Ok(contents) => match fs::write(&path, contents) {
                    Ok(_) => {
                        if !export_as {
                            self.people_path = Some(path.clone());
                        }
                        self.message = format!(
                            "{} people CSV to {}",
                            if export_as { "Exported" } else { "Saved" },
                            path.display()
                        );
                    }
                    Err(error) => self.message = format!("People save failed: {error}"),
                },
                Err(error) => self.message = format!("People save failed: {error}"),
            }
        }
    }

    pub(super) fn save_closeness(&mut self, export_as: bool) {
        if let Some(path) = self.resolve_save_path(export_as, &self.closeness_path, "closeness.csv")
        {
            match self.materialize_closeness_rules() {
                Ok(rules) => match write_closeness_csv(&rules) {
                    Ok(contents) => match fs::write(&path, contents) {
                        Ok(_) => {
                            if !export_as {
                                self.closeness_path = Some(path.clone());
                            }
                            self.message = format!(
                                "{} closeness CSV to {}",
                                if export_as { "Exported" } else { "Saved" },
                                path.display()
                            );
                        }
                        Err(error) => self.message = format!("Closeness save failed: {error}"),
                    },
                    Err(error) => self.message = format!("Closeness save failed: {error}"),
                },
                Err(report) => self.message = format!("Closeness save failed:\n{report}"),
            }
        }
    }

    pub(super) fn save_tables(&mut self, export_as: bool) {
        if let Some(path) = self.resolve_save_path(export_as, &self.tables_path, "tables.csv") {
            let table_map = self
                .materialize_table_entries()
                .and_then(build_table_type_map);
            match table_map {
                Ok(table_types) => match write_tables_csv(&table_types) {
                    Ok(contents) => match fs::write(&path, contents) {
                        Ok(_) => {
                            if !export_as {
                                self.tables_path = Some(path.clone());
                            }
                            self.message = format!(
                                "{} tables CSV to {}",
                                if export_as { "Exported" } else { "Saved" },
                                path.display()
                            );
                        }
                        Err(error) => self.message = format!("Tables save failed: {error}"),
                    },
                    Err(error) => self.message = format!("Tables save failed: {error}"),
                },
                Err(report) => self.message = format!("Tables save failed:\n{report}"),
            }
        }
    }

    pub(super) fn save_seating(&mut self) {
        if self.assignments.is_empty() {
            self.message = "No seating assignment to save yet.".to_string();
            return;
        }
        if let Some(path) = self.resolve_save_path(false, &self.seating_path, "seating.csv") {
            match write_seating_csv(&self.assignments) {
                Ok(contents) => match fs::write(&path, contents) {
                    Ok(_) => {
                        self.seating_path = Some(path.clone());
                        self.message = format!("Saved seating CSV to {}", path.display());
                    }
                    Err(error) => self.message = format!("Seating save failed: {error}"),
                },
                Err(error) => self.message = format!("Seating save failed: {error}"),
            }
        }
    }

    pub(super) fn export_plan_svg(&mut self) {
        let Some(layout) = &self.layout else {
            self.message = "No valid seating plan available for SVG export.".to_string();
            return;
        };
        let Some(path) = FileDialog::new()
            .set_file_name("seating-plan.svg")
            .save_file()
        else {
            return;
        };
        match fs::write(&path, render_svg(layout, &RenderOptions::default())) {
            Ok(_) => self.message = format!("Exported SVG to {}", path.display()),
            Err(error) => self.message = format!("SVG export failed: {error}"),
        }
    }

    pub(super) fn export_plan_png(&mut self) {
        let Some(layout) = &self.layout else {
            self.message = "No valid seating plan available for PNG export.".to_string();
            return;
        };
        let Some(path) = FileDialog::new()
            .set_file_name("seating-plan.png")
            .save_file()
        else {
            return;
        };
        match render_png(layout, &RenderOptions::default(), &path) {
            Ok(_) => self.message = format!("Exported PNG to {}", path.display()),
            Err(error) => self.message = format!("PNG export failed: {error}"),
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
            self.generated_table_numbers()
                .into_iter()
                .map(|number| MaybeUsizeChoice {
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
            let capacity = self.table_capacity_for_number(table_number);
            if let Some(capacity) = capacity {
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

    pub(super) fn generated_table_numbers(&self) -> Vec<usize> {
        self.current_project()
            .ok()
            .map(|project| {
                generate_table_instances(&project)
                    .into_iter()
                    .map(|table| table.number)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn table_capacity_for_number(&self, table_number: usize) -> Option<usize> {
        self.current_project().ok().and_then(|project| {
            generate_table_instances(&project)
                .into_iter()
                .find(|table| table.number == table_number)
                .map(|table| table.max_people)
        })
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
            .filter_map(|error| match error {
                ValidationError::DuplicatePersonId(id) if id == &person.id => {
                    Some(error.to_string())
                }
                ValidationError::NamespaceCollision(id) if id == &person.id => {
                    Some(error.to_string())
                }
                ValidationError::UnknownTableTypeForPerson { person_id, .. }
                    if person_id == &person.id =>
                {
                    Some(error.to_string())
                }
                ValidationError::LockedSeatRequiresLockedTable(id) if id == &person.id => {
                    Some(error.to_string())
                }
                ValidationError::LockedTableDoesNotExist { person_id, .. }
                    if person_id == &person.id =>
                {
                    Some(error.to_string())
                }
                ValidationError::LockedSeatOutOfRange { person_id, .. }
                    if person_id == &person.id =>
                {
                    Some(error.to_string())
                }
                ValidationError::LockedTableTypeMismatch { person_id, .. }
                    if person_id == &person.id =>
                {
                    Some(error.to_string())
                }
                ValidationError::ImpossiblePersonAssignment { person_id }
                    if person_id == &person.id =>
                {
                    Some(error.to_string())
                }
                _ => None,
            })
            .collect()
    }

    pub(super) fn closeness_errors(&self, rule: &ClosenessRule, score_input: &str) -> Vec<String> {
        let mut errors: Vec<String> = self
            .validation_errors
            .iter()
            .filter_map(|error| match error {
                ValidationError::UnknownIdInCloseness(id)
                    if id == &rule.left_id || id == &rule.right_id =>
                {
                    Some(error.to_string())
                }
                ValidationError::DuplicateClosenessRule(left, right)
                    if same_pair(left, right, &rule.left_id, &rule.right_id) =>
                {
                    Some(error.to_string())
                }
                ValidationError::InvalidClosenessScore { left, right }
                    if left == &rule.left_id && right == &rule.right_id =>
                {
                    Some(error.to_string())
                }
                _ => None,
            })
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
            .filter_map(|error| match error {
                ValidationError::DuplicateTableTypeId(id) if id == &row.table_type_id => {
                    Some(error.to_string())
                }
                ValidationError::EmptyTableTypeId if row.table_type_id.trim().is_empty() => {
                    Some(error.to_string())
                }
                ValidationError::MissingPeoplePerSide(table_type)
                    if table_type == &row.table_type_id =>
                {
                    Some(error.to_string())
                }
                ValidationError::InvalidPeoplePerSideLength { table_type, .. }
                    if table_type == &row.table_type_id =>
                {
                    Some(error.to_string())
                }
                ValidationError::PeoplePerSideMismatch { table_type, .. }
                    if table_type == &row.table_type_id =>
                {
                    Some(error.to_string())
                }
                ValidationError::InvalidMinMax { table_type, .. }
                    if table_type == &row.table_type_id =>
                {
                    Some(error.to_string())
                }
                ValidationError::InvalidRecommendedPeople { table_type, .. }
                    if table_type == &row.table_type_id =>
                {
                    Some(error.to_string())
                }
                ValidationError::InvalidNumberOfTables { table_type, .. }
                    if table_type == &row.table_type_id =>
                {
                    Some(error.to_string())
                }
                _ => None,
            })
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
            for input in row.people_per_side_input.split('|') {
                let input = input.trim();
                if input.is_empty() {
                    continue;
                }
                if let Err(error) = parse_required_usize_value(input, "people_per_side") {
                    errors.push(error.to_string());
                }
            }
        }

        errors
    }

    pub(super) fn reference_matches(
        &self,
        options: &[seating_core::ReferenceIdOption],
        query: &str,
    ) -> Vec<seating_core::ReferenceIdOption> {
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

    pub(super) fn reference_label(
        &self,
        id: &str,
        options: &[seating_core::ReferenceIdOption],
    ) -> String {
        options
            .iter()
            .find(|option| option.id == id)
            .map(|option| option.label.clone())
            .unwrap_or_else(|| format!("{id} — unknown"))
    }
}
