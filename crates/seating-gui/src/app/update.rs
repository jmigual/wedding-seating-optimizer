use super::*;

pub(super) fn update(app: &mut GuiApp, message: Msg) -> Command<Msg> {
    // A pending whole-project confirmation only survives into this dispatch
    // if the incoming message is the matching repeated activation; any other
    // message cancels it.
    if !matches!(
        (&message, &app.pending_confirm),
        (Msg::NewProject, Some(PendingConfirm::NewProject))
            | (Msg::OpenProject, Some(PendingConfirm::OpenProject))
            | (Msg::ExportProjectCsv, Some(PendingConfirm::ExportCsv(_)))
    ) {
        app.pending_confirm = None;
    }

    match message {
        Msg::FocusNext => return iced::widget::focus_next(),
        Msg::FocusPrevious => return iced::widget::focus_previous(),
        Msg::SelectTab(tab) => app.active_tab = tab,
        Msg::NewProject => {
            if app.dirty && app.pending_confirm.is_none() {
                app.pending_confirm = Some(PendingConfirm::NewProject);
                app.set_message(
                    MessageKind::Info,
                    "Unsaved changes — click New Project again to discard them.",
                );
            } else {
                app.pending_confirm = None;
                *app = GuiApp::empty();
                app.set_message(MessageKind::Success, "Started a new empty project.");
            }
        }
        Msg::OpenProject => {
            if app.dirty && app.pending_confirm.is_none() {
                app.pending_confirm = Some(PendingConfirm::OpenProject);
                app.set_message(
                    MessageKind::Info,
                    "Unsaved changes — click Open Project again to discard them.",
                );
            } else {
                app.pending_confirm = None;
                app.open_project();
            }
        }
        Msg::SaveProject => app.save_project(false),
        Msg::SaveProjectAs => app.save_project(true),
        Msg::ExportProjectCsv => app.export_project_csv(),
        Msg::ImportPeople => app.import_people(),
        Msg::SavePeople => app.save_people(false),
        Msg::ExportPeople => app.save_people(true),
        Msg::ImportCloseness => app.import_closeness(),
        Msg::SaveCloseness => app.save_closeness(false),
        Msg::ExportCloseness => app.save_closeness(true),
        Msg::ImportTables => app.import_tables(),
        Msg::SaveTables => app.save_tables(false),
        Msg::ExportTables => app.save_tables(true),
        Msg::AddPerson => {
            app.people.push(PersonRowState::default());
            app.refresh_validation_and_layout();
        }
        Msg::DeletePerson(index) => {
            if index < app.people.len() {
                app.people.remove(index);
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdatePersonId(index, value) => {
            if let Some(row) = app.people.get_mut(index) {
                row.person.id = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdatePersonName(index, value) => {
            if let Some(row) = app.people.get_mut(index) {
                row.person.name = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdatePersonTableType(index, choice) => {
            if let Some(row) = app.people.get_mut(index) {
                row.person.table_type = choice.value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdatePersonLockedTable(index, choice) => {
            if let Some(row) = app.people.get_mut(index) {
                row.person.locked_table = choice.value;
                if row.person.locked_table.is_none() {
                    row.person.locked_seat = None;
                }
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdatePersonLockedSeat(index, choice) => {
            if let Some(row) = app.people.get_mut(index) {
                row.person.locked_seat = choice.value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateNewGroup(index, value) => {
            if let Some(row) = app.people.get_mut(index) {
                row.new_group = value;
            }
        }
        Msg::AddGroup(index) => {
            if let Some(row) = app.people.get_mut(index) {
                let group = row.new_group.trim();
                if !group.is_empty() && !row.person.groups.iter().any(|existing| existing == group)
                {
                    row.person.groups.push(group.to_string());
                }
                row.new_group.clear();
                app.refresh_validation_and_layout();
            }
        }
        Msg::RemoveGroup(person_index, group_index) => {
            if let Some(row) = app.people.get_mut(person_index) {
                if group_index < row.person.groups.len() {
                    row.person.groups.remove(group_index);
                    app.refresh_validation_and_layout();
                }
            }
        }
        Msg::AddClosenessRule => {
            app.closeness_rules.push(ClosenessRowState::default());
            app.refresh_validation_and_layout();
        }
        Msg::DeleteClosenessRule(index) => {
            if index < app.closeness_rules.len() {
                app.closeness_rules.remove(index);
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateClosenessLeft(index, value) => {
            if let Some(row) = app.closeness_rules.get_mut(index) {
                row.left_id = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateClosenessRight(index, value) => {
            if let Some(row) = app.closeness_rules.get_mut(index) {
                row.right_id = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::SelectClosenessLeft(index, value) => {
            if let Some(row) = app.closeness_rules.get_mut(index) {
                row.left_id = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::SelectClosenessRight(index, value) => {
            if let Some(row) = app.closeness_rules.get_mut(index) {
                row.right_id = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateClosenessScore(index, value) => {
            if let Some(row) = app.closeness_rules.get_mut(index) {
                row.score_input = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::AddTableConfig => {
            app.table_configs.push(TableConfigState::default());
            app.refresh_validation_and_layout();
        }
        Msg::DeleteTableConfig(index) => {
            if index < app.table_configs.len() {
                app.table_configs.remove(index);
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateTableTypeId(index, value) => {
            if let Some(row) = app.table_configs.get_mut(index) {
                row.table_type_id = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateTableShape(index, shape) => {
            if let Some(row) = app.table_configs.get_mut(index) {
                row.shape = shape;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateMaxPeople(index, value) => {
            if let Some(row) = app.table_configs.get_mut(index) {
                row.max_people_input = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateMinPeople(index, value) => {
            if let Some(row) = app.table_configs.get_mut(index) {
                row.min_people_input = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateRecommendedPeople(index, value) => {
            if let Some(row) = app.table_configs.get_mut(index) {
                row.recommended_people_input = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateNumberOfTables(index, value) => {
            if let Some(row) = app.table_configs.get_mut(index) {
                row.number_of_tables_input = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdatePeoplePerSide(index, value) => {
            if let Some(row) = app.table_configs.get_mut(index) {
                row.people_per_side_input = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::SeedChanged(value) => app.seed = value,
        Msg::AttemptsChanged(value) => app.attempts = value,
        Msg::IterationsChanged(value) => app.iterations = value,
        Msg::SolutionsChanged(value) => app.solutions = value,
        Msg::ProximityWeightChanged(value) => app.proximity_weight = value,
        Msg::UsedTableWeightChanged(value) => app.used_table_weight = value,
        Msg::OptimalTableSizeWeightChanged(value) => app.optimal_table_size_weight = value,
        Msg::Optimize => return run_optimize(app),
        Msg::OptimizeFinished(result) => optimize_finished(app, result),
        Msg::SaveSeating => app.save_seating(),
        Msg::ExportPlanSvg => app.export_plan_svg(),
        Msg::ExportPlanPng => app.export_plan_png(),
        Msg::ZoomIn => app.zoom = (app.zoom + 0.2).min(3.0),
        Msg::ZoomOut => app.zoom = (app.zoom - 0.2).max(0.4),
        Msg::ZoomReset => app.zoom = 1.0,
    }
    Command::none()
}

/// Kick off an optimizer run on a background executor thread so the UI event
/// loop keeps repainting. `HeuristicOptimizer::optimize` is synchronous, but
/// iced's default executor is a thread pool distinct from the UI thread, so
/// wrapping the call in an async block already keeps the window responsive
/// without needing `spawn_blocking` or the `tokio` executor feature.
fn run_optimize(app: &mut GuiApp) -> Command<Msg> {
    if app.is_optimizing {
        return Command::none();
    }
    let config = match app.optimization_config() {
        Ok(config) => config,
        Err(report) => {
            app.set_message(
                MessageKind::Error,
                format!(
                    "Cannot optimize invalid settings: {}",
                    GuiApp::report_summary(&report)
                ),
            );
            return Command::none();
        }
    };
    let project = match app.current_project() {
        Ok(project) => project,
        Err(report) => {
            app.set_message(
                MessageKind::Error,
                format!(
                    "Cannot optimize invalid data: {}",
                    GuiApp::report_summary(&report)
                ),
            );
            return Command::none();
        }
    };
    if let Err(report) = validate_project(&project) {
        app.set_message(
            MessageKind::Error,
            format!(
                "Cannot optimize invalid data: {}",
                GuiApp::report_summary(&report)
            ),
        );
        app.validation_errors = report.errors;
        return Command::none();
    }
    app.is_optimizing = true;
    app.set_message(MessageKind::Info, "Optimizing…");
    Command::perform(
        async move { HeuristicOptimizer.optimize(&project, &config) },
        Msg::OptimizeFinished,
    )
}

fn optimize_finished(app: &mut GuiApp, result: Result<OptimizationResult, ValidationReport>) {
    app.is_optimizing = false;
    match result {
        Err(report) => app.set_message(
            MessageKind::Error,
            format!("Optimization failed: {}", GuiApp::report_summary(&report)),
        ),
        Ok(result) => match result.solutions.into_iter().next() {
            Some(solution) => {
                let score = solution.score;
                app.assignments = solution.assignments;
                app.refresh_validation_and_layout();
                app.active_tab = Tab::SeatingPlan;
                // refresh_validation_and_layout may have already set a more
                // specific message if rendering the new plan failed.
                if app.layout_error.is_none() && app.validation_errors.is_empty() {
                    app.set_message(
                        MessageKind::Success,
                        format!("Optimization complete. Score: {score:.3}"),
                    );
                }
            }
            None => app.set_message(MessageKind::Error, "Optimizer returned no solutions."),
        },
    }
}
