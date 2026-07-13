//! Left side-panel editors: People / Closeness / Tables / Settings /
//! Diagnostics.
//!
//! This module owns [`EditorsState`] exclusively (module ownership rule in
//! the spec): only `crate::state::SharedState` is read/mutated from here,
//! and every mutation ends with [`SharedState::refresh`].

use crate::state::{ClosenessRow, SharedState, TableConfigRow};
use eframe::egui;
use seating_core::{
    collect_group_ids, parse_f64_value, reference_id_options, reference_label, remove_group,
    rename_group, rules_match, ClosenessRule, OptimizationConfig, TableShape, ValidationError,
};
use std::collections::HashMap;

const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 90, 90);
const SUCCESS_COLOR: egui::Color32 = egui::Color32::from_rgb(90, 200, 120);

/// UI-only state for the editors panel: per-person "new group" scratch
/// input (kept index-aligned with `SharedState::people`), and per-group
/// rename scratch input (keyed by the group's current name).
#[derive(Default)]
pub(crate) struct EditorsState {
    new_group_inputs: Vec<String>,
    group_rename_inputs: HashMap<String, String>,
}

pub(crate) fn show(shared: &mut SharedState, state: &mut EditorsState, ui: &mut egui::Ui) {
    ui.heading("Editors");

    egui::CollapsingHeader::new(format!("People ({})", shared.people.len()))
        .id_source("editors_people")
        .default_open(true)
        .show(ui, |ui| people_section(shared, state, ui));

    egui::CollapsingHeader::new(format!(
        "Groups ({})",
        collect_group_ids(&shared.people).len()
    ))
    .id_source("editors_groups")
    .default_open(false)
    .show(ui, |ui| groups_section(shared, state, ui));

    egui::CollapsingHeader::new(format!("Closeness ({})", shared.closeness_rules.len()))
        .id_source("editors_closeness")
        .default_open(false)
        .show(ui, |ui| closeness_section(shared, ui));

    egui::CollapsingHeader::new(format!("Tables ({})", shared.table_configs.len()))
        .id_source("editors_tables")
        .default_open(false)
        .show(ui, |ui| tables_section(shared, ui));

    egui::CollapsingHeader::new("Settings")
        .id_source("editors_settings")
        .default_open(false)
        .show(ui, |ui| settings_section(shared, ui));

    egui::CollapsingHeader::new(format!("Diagnostics ({})", shared.validation.len()))
        .id_source("editors_diagnostics")
        .default_open(!shared.validation.is_empty())
        .show(ui, |ui| diagnostics_section(shared, ui));
}

// ── People ──────────────────────────────────────────────────────────────────

fn people_section(shared: &mut SharedState, state: &mut EditorsState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("+ Add Person").clicked() {
            let id = unique_id("person", shared.people.iter().map(|p| p.id.as_str()));
            shared.people.push(seating_core::Person {
                id,
                name: String::new(),
                table_type: None,
                groups: Vec::new(),
                locked_table: None,
                locked_seat: None,
            });
            state.new_group_inputs.push(String::new());
            shared.refresh();
        }
        if ui.button("Import CSV…").clicked() {
            shared.import_people_csv();
        }
    });

    if shared.people.is_empty() {
        ui.label(egui::RichText::new("No guests yet — click \"+ Add Person\" to start.").weak());
        return;
    }
    while state.new_group_inputs.len() < shared.people.len() {
        state.new_group_inputs.push(String::new());
    }

    let table_type_ids: Vec<String> = shared
        .table_configs
        .iter()
        .map(|row| row.table_type_id.clone())
        .collect();
    let all_groups = collect_group_ids(&shared.people);

    let mut delete_index = None;
    for index in 0..shared.people.len() {
        ui.group(|ui| {
            let mut changed = false;

            // Wrapped so the id/name/delete, table-type, and locked
            // table/seat clusters flow onto fewer lines on a wide panel and
            // degrade to one cluster per line (the old stacked layout) on a
            // narrow one.
            //
            // Each cluster below is itself `horizontal_wrapped`, not plain
            // `horizontal`: a plain nested `horizontal` is laid out with an
            // egui-reported "desired size" of whatever room is left in the
            // outer row (not its actual content width), so when its content
            // doesn't fit, it silently overflows sideways instead of
            // wrapping — and that overflow bubbles up through the
            // `ScrollArea`/`Frame` to the SidePanel's persisted width,
            // permanently widening the panel. A wrapped cluster instead
            // drops its own overflow to a second internal line, which
            // bounds its width to what it was given.
            ui.horizontal_wrapped(|ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut shared.people[index].id)
                                .hint_text("id")
                                .desired_width(70.0),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(&mut shared.people[index].name)
                                .hint_text("name")
                                .desired_width(110.0),
                        )
                        .changed();
                    if ui.button("Delete").clicked() {
                        delete_index = Some(index);
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label("Table type:");
                    let current = shared.people[index].table_type.clone();
                    egui::ComboBox::from_id_source(("person_table_type", index))
                        .selected_text(current.clone().unwrap_or_else(|| "(any)".to_string()))
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(current.is_none(), "(any)").clicked() {
                                shared.people[index].table_type = None;
                                changed = true;
                            }
                            for table_type_id in &table_type_ids {
                                let selected = current.as_deref() == Some(table_type_id.as_str());
                                if ui.selectable_label(selected, table_type_id).clicked() {
                                    shared.people[index].table_type = Some(table_type_id.clone());
                                    changed = true;
                                }
                            }
                        });
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label("Locked table:");
                    let current_table = shared.people[index].locked_table;
                    egui::ComboBox::from_id_source(("person_locked_table", index))
                        .selected_text(
                            current_table
                                .map(|t| t.to_string())
                                .unwrap_or_else(|| "(none)".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(current_table.is_none(), "(none)")
                                .clicked()
                            {
                                shared.people[index].locked_table = None;
                                shared.people[index].locked_seat = None;
                                changed = true;
                            }
                            for &table_number in &shared.generated_table_numbers {
                                let selected = current_table == Some(table_number);
                                if ui
                                    .selectable_label(selected, table_number.to_string())
                                    .clicked()
                                {
                                    shared.people[index].locked_table = Some(table_number);
                                    changed = true;
                                }
                            }
                        });

                    let locked_table = shared.people[index].locked_table;
                    let capacity =
                        locked_table.and_then(|t| shared.table_capacities.get(&t).copied());
                    // Label lives inside the enabled scope too, so hovering
                    // either it or the combo shows why the control is
                    // disabled (a seat index is meaningless without a
                    // locked table).
                    ui.add_enabled_ui(locked_table.is_some(), |ui| {
                        ui.label("Locked seat:");
                        let current_seat = shared.people[index].locked_seat;
                        egui::ComboBox::from_id_source(("person_locked_seat", index))
                            .selected_text(
                                current_seat
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| "(none)".to_string()),
                            )
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(current_seat.is_none(), "(none)")
                                    .clicked()
                                {
                                    shared.people[index].locked_seat = None;
                                    changed = true;
                                }
                                for seat in 0..capacity.unwrap_or(0) {
                                    let selected = current_seat == Some(seat);
                                    if ui.selectable_label(selected, seat.to_string()).clicked() {
                                        shared.people[index].locked_seat = Some(seat);
                                        changed = true;
                                    }
                                }
                            });
                    })
                    .response
                    .on_disabled_hover_text("Select a locked table first");
                });
            });

            ui.horizontal_wrapped(|ui| {
                ui.label("Groups:");
                let mut remove_group = None;
                for (group_index, group) in shared.people[index].groups.iter().enumerate() {
                    if ui.button(format!("{group} ×")).clicked() {
                        remove_group = Some(group_index);
                    }
                }
                if let Some(group_index) = remove_group {
                    shared.people[index].groups.remove(group_index);
                    changed = true;
                }
                let mut add_existing_group = None;
                egui::ComboBox::from_id_source(("person_group_picker", index))
                    .selected_text("+ existing group")
                    .show_ui(ui, |ui| {
                        for group in &all_groups {
                            if shared.people[index].groups.contains(group) {
                                continue;
                            }
                            if ui.selectable_label(false, group).clicked() {
                                add_existing_group = Some(group.clone());
                            }
                        }
                    });
                if let Some(group) = add_existing_group {
                    shared.people[index].groups.push(group);
                    changed = true;
                }
                ui.add(
                    egui::TextEdit::singleline(&mut state.new_group_inputs[index])
                        .hint_text("new group")
                        .desired_width(70.0),
                );
                let has_input = !state.new_group_inputs[index].trim().is_empty();
                if ui.add_enabled(has_input, egui::Button::new("+")).clicked() {
                    let group = state.new_group_inputs[index].trim().to_string();
                    if !shared.people[index].groups.contains(&group) {
                        shared.people[index].groups.push(group);
                        changed = true;
                    }
                    state.new_group_inputs[index].clear();
                }
            });

            let person_id = shared.people[index].id.clone();
            for error in &shared.validation {
                if error.person_id() == Some(person_id.as_str()) {
                    ui.colored_label(ERROR_COLOR, error.to_string());
                }
            }

            if changed {
                shared.refresh();
            }
        });
    }

    if let Some(index) = delete_index {
        shared.people.remove(index);
        state.new_group_inputs.remove(index);
        shared.refresh();
    }
}

// ── Groups ──────────────────────────────────────────────────────────────────

/// Build a `Vec<ClosenessRule>` from raw closeness rows for passing to the
/// core group-editing functions. Unparseable scores fall back to `0.0`;
/// those functions only ever touch `left_id`/`right_id`, never `score`.
fn closeness_rules_from_rows(rows: &[ClosenessRow]) -> Vec<ClosenessRule> {
    rows.iter()
        .map(|row| ClosenessRule {
            left_id: row.left_id.clone(),
            right_id: row.right_id.clone(),
            score: parse_f64_value(&row.score_input, "score").unwrap_or(0.0),
        })
        .collect()
}

fn groups_section(shared: &mut SharedState, state: &mut EditorsState, ui: &mut egui::Ui) {
    let groups = collect_group_ids(&shared.people);
    if groups.is_empty() {
        ui.label(egui::RichText::new("No groups yet — add one to a person above.").weak());
        return;
    }

    let mut rename_action = None;
    let mut delete_action = None;
    for group in &groups {
        let member_count = shared
            .people
            .iter()
            .filter(|person| person.groups.iter().any(|g| g == group))
            .count();
        ui.horizontal(|ui| {
            ui.label(format!("{group} ({member_count})"));
            let buffer = state
                .group_rename_inputs
                .entry(group.clone())
                .or_insert_with(|| group.clone());
            ui.add(egui::TextEdit::singleline(buffer).desired_width(120.0));
            let new_name = buffer.trim().to_string();
            let can_rename = !new_name.is_empty() && new_name != *group;
            if ui
                .add_enabled(can_rename, egui::Button::new("Rename"))
                .clicked()
            {
                rename_action = Some((group.clone(), new_name));
            }
            if ui.button("Delete").clicked() {
                delete_action = Some(group.clone());
            }
        });
    }

    if let Some((old, new)) = rename_action {
        let mut rules = closeness_rules_from_rows(&shared.closeness_rules);
        rename_group(&mut shared.people, &mut rules, &old, &new);
        shared.closeness_rules = rules.into_iter().map(ClosenessRow::from).collect();
        state.group_rename_inputs.remove(&old);
        shared.refresh();
    }
    if let Some(group) = delete_action {
        let mut rules = closeness_rules_from_rows(&shared.closeness_rules);
        remove_group(&mut shared.people, &mut rules, &group);
        shared.closeness_rules = rules.into_iter().map(ClosenessRow::from).collect();
        state.group_rename_inputs.remove(&group);
        shared.refresh();
    }
}

// ── Closeness ───────────────────────────────────────────────────────────────

fn closeness_section(shared: &mut SharedState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("+ Add Rule").clicked() {
            shared.closeness_rules.push(ClosenessRow {
                left_id: String::new(),
                right_id: String::new(),
                score_input: "1.0".to_string(),
            });
            shared.refresh();
        }
        if ui.button("Import CSV…").clicked() {
            shared.import_closeness_csv();
        }
    });

    if shared.closeness_rules.is_empty() {
        ui.label(egui::RichText::new("No closeness rules yet — click \"+ Add Rule\".").weak());
        return;
    }

    let options = reference_id_options(&shared.people);
    let mut delete_index = None;
    for index in 0..shared.closeness_rules.len() {
        ui.group(|ui| {
            let mut changed = false;

            // Wrapped so Left/Right/Score+Delete flow onto fewer lines when
            // the panel is wide, and stack one per line when it's narrow.
            ui.horizontal_wrapped(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Left:");
                    let current = shared.closeness_rules[index].left_id.clone();
                    egui::ComboBox::from_id_source(("closeness_left", index))
                        .selected_text(if current.is_empty() {
                            "(select)".to_string()
                        } else {
                            reference_label(&current, &options)
                        })
                        .show_ui(ui, |ui| {
                            for option in &options {
                                let selected = current == option.id;
                                if ui.selectable_label(selected, &option.label).clicked() {
                                    shared.closeness_rules[index].left_id = option.id.clone();
                                    changed = true;
                                }
                            }
                        });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Right:");
                    let current = shared.closeness_rules[index].right_id.clone();
                    egui::ComboBox::from_id_source(("closeness_right", index))
                        .selected_text(if current.is_empty() {
                            "(select)".to_string()
                        } else {
                            reference_label(&current, &options)
                        })
                        .show_ui(ui, |ui| {
                            for option in &options {
                                let selected = current == option.id;
                                if ui.selectable_label(selected, &option.label).clicked() {
                                    shared.closeness_rules[index].right_id = option.id.clone();
                                    changed = true;
                                }
                            }
                        });
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Score:");
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(
                                &mut shared.closeness_rules[index].score_input,
                            )
                            .hint_text("e.g. 2.0")
                            .desired_width(70.0),
                        )
                        .changed();
                    if ui.button("Delete").clicked() {
                        delete_index = Some(index);
                    }
                });
            });

            let row = &shared.closeness_rules[index];
            let (left, right) = (
                row.left_id.trim().to_string(),
                row.right_id.trim().to_string(),
            );
            for error in &shared.validation {
                if let Some((error_left, error_right)) = error.closeness_pair() {
                    if rules_match(error_left, error_right, &left, &right) {
                        ui.colored_label(ERROR_COLOR, error.to_string());
                    }
                }
            }

            if changed {
                shared.refresh();
            }
        });
    }

    if let Some(index) = delete_index {
        shared.closeness_rules.remove(index);
        shared.refresh();
    }
}

// ── Tables ──────────────────────────────────────────────────────────────────

fn table_shape_label(shape: &TableShape) -> &'static str {
    match shape {
        TableShape::Round => "round",
        TableShape::Rectangular => "rectangular",
        TableShape::Square => "square",
    }
}

fn tables_section(shared: &mut SharedState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("+ Add Table Type").clicked() {
            let table_type_id = unique_id(
                "table_type",
                shared
                    .table_configs
                    .iter()
                    .map(|row| row.table_type_id.as_str()),
            );
            shared.table_configs.push(TableConfigRow {
                table_type_id,
                shape: TableShape::Round,
                max_people_input: String::new(),
                min_people_input: String::new(),
                recommended_people_input: String::new(),
                number_of_tables_input: String::new(),
                people_per_side_input: String::new(),
            });
            shared.refresh();
        }
        if ui.button("Import CSV…").clicked() {
            shared.import_tables_csv();
        }
    });

    if shared.table_configs.is_empty() {
        ui.label(egui::RichText::new("No table types yet — click \"+ Add Table Type\".").weak());
        return;
    }

    let mut delete_index = None;
    for index in 0..shared.table_configs.len() {
        ui.group(|ui| {
            let mut changed = false;

            // Wrapped so the name/shape/delete, capacity, count, and
            // people-per-side clusters flow onto fewer lines when the panel
            // is wide, and stack one per line when it's narrow.
            ui.horizontal_wrapped(|ui| {
                ui.horizontal_wrapped(|ui| {
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(
                                &mut shared.table_configs[index].table_type_id,
                            )
                            .hint_text("table_type_id")
                            .desired_width(100.0),
                        )
                        .changed();

                    let current_shape = shared.table_configs[index].shape.clone();
                    egui::ComboBox::from_id_source(("table_shape", index))
                        .selected_text(table_shape_label(&current_shape))
                        .show_ui(ui, |ui| {
                            for shape in [
                                TableShape::Round,
                                TableShape::Rectangular,
                                TableShape::Square,
                            ] {
                                let selected = current_shape == shape;
                                if ui
                                    .selectable_label(selected, table_shape_label(&shape))
                                    .clicked()
                                {
                                    shared.table_configs[index].shape = shape;
                                    changed = true;
                                }
                            }
                        });

                    if ui.button("Delete").clicked() {
                        delete_index = Some(index);
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label("max:");
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(
                                &mut shared.table_configs[index].max_people_input,
                            )
                            .desired_width(45.0),
                        )
                        .changed();
                    ui.label("min:");
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(
                                &mut shared.table_configs[index].min_people_input,
                            )
                            .desired_width(45.0),
                        )
                        .changed();
                    ui.label("recommended:");
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(
                                &mut shared.table_configs[index].recommended_people_input,
                            )
                            .desired_width(45.0),
                        )
                        .changed();
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("count:");
                    changed |= ui
                        .add(
                            egui::TextEdit::singleline(
                                &mut shared.table_configs[index].number_of_tables_input,
                            )
                            .hint_text("unlimited")
                            .desired_width(60.0),
                        )
                        .changed();
                });
                if matches!(
                    shared.table_configs[index].shape,
                    TableShape::Rectangular | TableShape::Square
                ) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("People per side (top|right|bottom|left):");
                        changed |= ui
                            .add(
                                egui::TextEdit::singleline(
                                    &mut shared.table_configs[index].people_per_side_input,
                                )
                                .hint_text("1|1|1|1")
                                .desired_width(150.0),
                            )
                            .changed();
                    });
                }
            });

            let table_type_id = shared.table_configs[index].table_type_id.clone();
            for error in &shared.validation {
                if error.table_type_id() == Some(table_type_id.as_str()) {
                    ui.colored_label(ERROR_COLOR, error.to_string());
                }
            }

            if changed {
                shared.refresh();
            }
        });
    }

    if let Some(index) = delete_index {
        shared.table_configs.remove(index);
        shared.refresh();
    }
}

// ── Settings ────────────────────────────────────────────────────────────────

fn settings_section(shared: &mut SharedState, ui: &mut egui::Ui) {
    let mut changed = false;

    egui::Grid::new("settings_grid")
        .num_columns(2)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            ui.label("Seed");
            changed |= ui.text_edit_singleline(&mut shared.seed).changed();
            ui.end_row();

            ui.label("Attempts");
            changed |= ui.text_edit_singleline(&mut shared.attempts).changed();
            ui.end_row();

            ui.label("Iterations");
            changed |= ui.text_edit_singleline(&mut shared.iterations).changed();
            ui.end_row();

            ui.label("Solutions");
            changed |= ui.text_edit_singleline(&mut shared.solutions).changed();
            ui.end_row();

            ui.label("Proximity weight");
            changed |= ui
                .text_edit_singleline(&mut shared.proximity_weight)
                .changed();
            ui.end_row();

            ui.label("Used table weight");
            changed |= ui
                .text_edit_singleline(&mut shared.used_table_weight)
                .changed();
            ui.end_row();

            ui.label("Table size weight");
            changed |= ui
                .text_edit_singleline(&mut shared.optimal_table_size_weight)
                .changed();
            ui.end_row();
        });

    if ui.button("Reset to defaults").clicked() {
        let defaults = OptimizationConfig::default();
        shared.seed = defaults.seed.to_string();
        shared.attempts = defaults.attempts.to_string();
        shared.iterations = defaults.iterations.to_string();
        shared.solutions = defaults.solutions.to_string();
        shared.proximity_weight = defaults.proximity_weight.to_string();
        shared.used_table_weight = defaults.used_table_weight.to_string();
        shared.optimal_table_size_weight = defaults.optimal_table_size_weight.to_string();
        changed = true;
    }

    if let Err(report) = shared.materialize_optimization_config() {
        for error in &report.errors {
            ui.colored_label(ERROR_COLOR, error.to_string());
        }
    }

    if changed {
        shared.refresh();
    }
}

// ── Diagnostics ─────────────────────────────────────────────────────────────

fn diagnostics_section(shared: &SharedState, ui: &mut egui::Ui) {
    if shared.validation.is_empty() {
        ui.colored_label(SUCCESS_COLOR, "No validation issues.");
        return;
    }
    for error in &shared.validation {
        colored_error_row(ui, error);
    }
}

fn colored_error_row(ui: &mut egui::Ui, error: &ValidationError) {
    ui.colored_label(ERROR_COLOR, error.to_string());
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Generate an id of the form `"{prefix}_{n}"` (1-based) not already present
/// in `existing`.
fn unique_id<'a>(prefix: &str, existing: impl Iterator<Item = &'a str> + Clone) -> String {
    let mut n = 1;
    loop {
        let candidate = format!("{prefix}_{n}");
        if !existing.clone().any(|id| id == candidate) {
            return candidate;
        }
        n += 1;
    }
}
