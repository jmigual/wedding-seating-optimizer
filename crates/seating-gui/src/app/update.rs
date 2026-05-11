use super::*;

pub(super) fn update(app: &mut GuiApp, message: Msg) -> Command<Msg> {
    match message {
        Msg::FocusNext => return iced::widget::focus_next(),
        Msg::FocusPrevious => return iced::widget::focus_previous(),
        Msg::SelectTab(tab) => app.active_tab = tab,
        Msg::NewProject => {
            *app = GuiApp::empty();
            app.message = "Started a new empty project.".to_string();
        }
        Msg::OpenProject => app.open_project(),
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
                row.rule.left_id = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateClosenessRight(index, value) => {
            if let Some(row) = app.closeness_rules.get_mut(index) {
                row.rule.right_id = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::SelectClosenessLeft(index, value) => {
            if let Some(row) = app.closeness_rules.get_mut(index) {
                row.rule.left_id = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::SelectClosenessRight(index, value) => {
            if let Some(row) = app.closeness_rules.get_mut(index) {
                row.rule.right_id = value;
                app.refresh_validation_and_layout();
            }
        }
        Msg::UpdateClosenessScore(index, value) => {
            if let Some(row) = app.closeness_rules.get_mut(index) {
                row.score_input = value;
                if let Ok(score) = parse_f64_value(&row.score_input, "score") {
                    row.rule.score = score;
                }
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
        Msg::ProximityWeightChanged(value) => app.proximity_weight = value,
        Msg::UsedTableWeightChanged(value) => app.used_table_weight = value,
        Msg::OptimalTableSizeWeightChanged(value) => app.optimal_table_size_weight = value,
        Msg::Optimize => run_optimize(app),
        Msg::SaveSeating => app.save_seating(),
        Msg::ExportPlanSvg => app.export_plan_svg(),
        Msg::ExportPlanPng => app.export_plan_png(),
        Msg::ZoomIn => app.zoom = (app.zoom + 0.2).min(3.0),
        Msg::ZoomOut => app.zoom = (app.zoom - 0.2).max(0.4),
    }
    Command::none()
}

fn run_optimize(app: &mut GuiApp) {
    let config = match app.optimization_config() {
        Ok(config) => config,
        Err(report) => {
            app.message = format!("Cannot optimize invalid settings:\n{report}");
            return;
        }
    };
    let project = match app.current_project() {
        Ok(project) => project,
        Err(report) => {
            app.message = format!("Cannot optimize invalid data:\n{report}");
            return;
        }
    };
    if let Err(report) = validate_project(&project) {
        app.validation_errors = report.errors.clone();
        app.message = format!("Cannot optimize invalid data:\n{report}");
        return;
    }
    match HeuristicOptimizer.optimize(&project, &config) {
        Err(report) => app.message = format!("Optimization failed:\n{report}"),
        Ok(result) => match result.solutions.first() {
            Some(solution) => {
                app.assignments = solution.assignments.clone();
                app.seating_csv = write_seating_csv(&app.assignments).unwrap_or_default();
                app.validation_errors.clear();
                if let Ok(layout) = build_layout(&project, &app.assignments) {
                    let svg_markup = render_svg(&layout, &RenderOptions::default());
                    app.layout = Some(layout);
                    app.layout_svg = Some(svg_markup);
                } else {
                    app.layout = None;
                    app.layout_svg = None;
                }
                app.active_tab = Tab::SeatingPlan;
                app.message = format!("Optimization complete. Score: {:.3}", solution.score);
            }
            None => app.message = "Optimizer returned no solutions.".to_string(),
        },
    }
}
