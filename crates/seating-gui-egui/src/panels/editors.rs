//! Left side-panel editors: People / Closeness / Tables / Settings /
//! Diagnostics.
//!
//! This module owns [`EditorsState`] exclusively (module ownership rule in
//! the spec): only `crate::state::SharedState` is read/mutated from here,
//! and every mutation ends with [`SharedState::refresh`].

use crate::state::{ClosenessRow, ImportDecision, PendingImport, SharedState, TableConfigRow};
use eframe::egui;
use seating_core::{
    ClosenessRule, OptimizationConfig, ReferenceIdOption, TableShape, ValidationError,
    collect_group_ids, parse_f64_value, reference_id_options, reference_label, remove_group,
    rename_group, rules_match,
};
use std::collections::HashMap;

const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(220, 90, 90);
const SUCCESS_COLOR: egui::Color32 = egui::Color32::from_rgb(90, 200, 120);

/// Item spacing budgeted between fields on an explicit-width row line
/// (matches egui's default `Spacing::item_spacing.x`).
const ROW_SPACING: f32 = 8.0;
/// Budgeted width of a "Delete" button. Its text is a constant, so this is a
/// one-time, generous, deterministic estimate — not something that varies
/// per row.
const DELETE_BUDGET: f32 = 90.0;

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
        .id_salt("editors_people")
        .default_open(true)
        .show(ui, |ui| people_section(shared, state, ui));

    egui::CollapsingHeader::new(format!(
        "Groups ({})",
        collect_group_ids(&shared.people).len()
    ))
    .id_salt("editors_groups")
    .default_open(false)
    .show(ui, |ui| groups_section(shared, state, ui));

    egui::CollapsingHeader::new(format!("Closeness ({})", shared.closeness_rules.len()))
        .id_salt("editors_closeness")
        .default_open(false)
        .show(ui, |ui| closeness_section(shared, ui));

    egui::CollapsingHeader::new(format!("Tables ({})", shared.table_configs.len()))
        .id_salt("editors_tables")
        .default_open(false)
        .show(ui, |ui| tables_section(shared, ui));

    egui::CollapsingHeader::new("Settings")
        .id_salt("editors_settings")
        .default_open(false)
        .show(ui, |ui| settings_section(shared, ui));

    egui::CollapsingHeader::new(format!("Diagnostics ({})", shared.validation.len()))
        .id_salt("editors_diagnostics")
        .default_open(!shared.validation.is_empty())
        .show(ui, |ui| diagnostics_section(shared, ui));
}

// ── People ──────────────────────────────────────────────────────────────────

const PERSON_ID_W: f32 = 70.0;
const PERSON_NAME_MIN_W: f32 = 80.0;
const TABLE_TYPE_COMBO_W: f32 = 100.0;
const TABLE_TYPE_MAX_CHARS: usize = 9;
const LOCKED_TABLE_COMBO_W: f32 = 90.0;
const LOCKED_SEAT_COMBO_W: f32 = 90.0;

/// Small deterministic cushion added on top of a measured label width, to
/// absorb sub-pixel differences between this measurement and egui's own
/// `ui.label()` layout (rounding/hinting), so the tier boundary never
/// under-shoots and causes the row to overflow its budgeted width. Same
/// "generous, deterministic estimate" philosophy as `DELETE_BUDGET`.
const LABEL_MEASURE_SLACK: f32 = 4.0;

/// Measure the rendered width of a plain label string in the current body
/// text style, via egui's memoized font-layout cache. Depends only on font
/// metrics + text + style — never on `ui.available_width()` or any other
/// panel-size-derived quantity — so unlike the container-size feedback loop
/// this design forbids, this measurement cannot feed back into panel size.
fn measured_label_width(ui: &egui::Ui, text: &str) -> f32 {
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    ui.ctx()
        .fonts_mut(|fonts| fonts.layout_no_wrap(text.to_string(), font_id, egui::Color32::WHITE))
        .size()
        .x
}

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

    if let Some(PendingImport::People { path, people }) = &shared.pending_import {
        let summary = format!("{} people from {}", people.len(), path.display());
        if let Some(decision) = import_decision_modal(ui.ctx(), "Import People", &summary)
            && shared.resolve_pending_import(decision)
        {
            state.new_group_inputs = vec![String::new(); shared.people.len()];
        }
    }

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

    let table_type_label_w = measured_label_width(ui, "Table type:");
    let locked_table_label_w = measured_label_width(ui, "Locked table:");
    let locked_seat_label_w = measured_label_width(ui, "Locked seat:");

    // Wide tier's one line has 9 items (id, name, delete, 3×(label+combo)) → 8 gaps.
    // Below this width the name field can't shrink further without dropping under
    // PERSON_NAME_MIN_W, so the tier can no longer fit.
    let wide_required = PERSON_ID_W
        + PERSON_NAME_MIN_W
        + DELETE_BUDGET
        + table_type_label_w
        + TABLE_TYPE_COMBO_W
        + locked_table_label_w
        + LOCKED_TABLE_COMBO_W
        + locked_seat_label_w
        + LOCKED_SEAT_COMBO_W
        + ROW_SPACING * 8.0
        + LABEL_MEASURE_SLACK;

    // Medium tier's line 1 (id, name, delete — name flexes to PERSON_NAME_MIN_W)
    // and line 2 (3×(label+combo), no flex field, 6 items → 5 gaps) must both fit;
    // line 2 is the true bottleneck (matches the reported symptom).
    let medium_line1_required = PERSON_ID_W + PERSON_NAME_MIN_W + DELETE_BUDGET + ROW_SPACING * 2.0;
    let medium_line2_required = table_type_label_w
        + TABLE_TYPE_COMBO_W
        + locked_table_label_w
        + LOCKED_TABLE_COMBO_W
        + locked_seat_label_w
        + LOCKED_SEAT_COMBO_W
        + ROW_SPACING * 5.0
        + LABEL_MEASURE_SLACK;
    let medium_required = medium_line1_required.max(medium_line2_required);

    let mut delete_index = None;
    for index in 0..shared.people.len() {
        ui.group(|ui| {
            let mut changed = false;

            // Explicit width-branched layout, never `horizontal_wrapped`:
            // pick a tier from this row's actual available width (already
            // the panel's real, resized width — the SidePanel's ScrollArea
            // runs with `auto_shrink` off, so it reports the assigned width
            // rather than re-measuring content and pushing that back up),
            // then lay out plain `ui.horizontal` lines. A label can never
            // wrap mid-word here because these lines are never
            // `horizontal_wrapped`. Every non-flex widget on a line gets an
            // explicit, deterministic width — TextEdit honors
            // `desired_width` exactly, and ComboBox text is truncated
            // (`truncate_label`) so its "at least" width (egui 0.27 grows a
            // ComboBox past `.width()` to fit unwrapped `selected_text`)
            // never exceeds what we budget for it. The one flex field per
            // line (`name`) gets whatever's left, computed *before* it's
            // drawn from those same deterministic budgets. So every line's
            // total is bounded by `w` by construction: the row's min_rect
            // can never demand more width than the panel actually granted
            // it, which is what stops the resize/snap-back loop this
            // replaces. Tier thresholds are computed per-frame from label
            // widths measured via the font-layout cache rather than
            // hardcoded, so they track the actual rendered text; the
            // measurement is a pure function of font metrics + text + style,
            // never of panel width, so it cannot feed back into container
            // size or reintroduce that loop.
            let w = ui.available_width();
            let wide = w >= wide_required;
            let medium = !wide && w >= medium_required;

            if wide {
                let name_w = (w
                    - PERSON_ID_W
                    - DELETE_BUDGET
                    - table_type_label_w
                    - TABLE_TYPE_COMBO_W
                    - locked_table_label_w
                    - LOCKED_TABLE_COMBO_W
                    - locked_seat_label_w
                    - LOCKED_SEAT_COMBO_W
                    - ROW_SPACING * 8.0)
                    .max(PERSON_NAME_MIN_W);
                ui.horizontal(|ui| {
                    changed |= id_field(ui, shared, index, PERSON_ID_W);
                    changed |= name_field(ui, shared, index, name_w);
                    if delete_button(ui) {
                        delete_index = Some(index);
                    }
                    changed |=
                        table_type_field(ui, shared, &table_type_ids, index, TABLE_TYPE_COMBO_W);
                    changed |= locked_table_field(ui, shared, index, LOCKED_TABLE_COMBO_W);
                    changed |= locked_seat_field(ui, shared, index, LOCKED_SEAT_COMBO_W);
                });
            } else if medium {
                let name_w =
                    (w - PERSON_ID_W - DELETE_BUDGET - ROW_SPACING * 2.0).max(PERSON_NAME_MIN_W);
                ui.horizontal(|ui| {
                    changed |= id_field(ui, shared, index, PERSON_ID_W);
                    changed |= name_field(ui, shared, index, name_w);
                    if delete_button(ui) {
                        delete_index = Some(index);
                    }
                });
                ui.horizontal(|ui| {
                    changed |=
                        table_type_field(ui, shared, &table_type_ids, index, TABLE_TYPE_COMBO_W);
                    changed |= locked_table_field(ui, shared, index, LOCKED_TABLE_COMBO_W);
                    changed |= locked_seat_field(ui, shared, index, LOCKED_SEAT_COMBO_W);
                });
            } else {
                ui.horizontal(|ui| {
                    changed |= id_field(ui, shared, index, PERSON_ID_W);
                    if delete_button(ui) {
                        delete_index = Some(index);
                    }
                });
                ui.horizontal(|ui| {
                    let name_w = (w - ROW_SPACING).max(PERSON_NAME_MIN_W);
                    changed |= name_field(ui, shared, index, name_w);
                });
                ui.horizontal(|ui| {
                    changed |=
                        table_type_field(ui, shared, &table_type_ids, index, TABLE_TYPE_COMBO_W);
                });
                ui.horizontal(|ui| {
                    changed |= locked_table_field(ui, shared, index, LOCKED_TABLE_COMBO_W);
                });
                ui.horizontal(|ui| {
                    changed |= locked_seat_field(ui, shared, index, LOCKED_SEAT_COMBO_W);
                });
            }

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
                egui::ComboBox::from_id_salt(("person_group_picker", index))
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

fn id_field(ui: &mut egui::Ui, shared: &mut SharedState, index: usize, width: f32) -> bool {
    ui.add(
        egui::TextEdit::singleline(&mut shared.people[index].id)
            .hint_text("id")
            .desired_width(width),
    )
    .changed()
}

fn name_field(ui: &mut egui::Ui, shared: &mut SharedState, index: usize, width: f32) -> bool {
    ui.add(
        egui::TextEdit::singleline(&mut shared.people[index].name)
            .hint_text("name")
            .desired_width(width),
    )
    .changed()
}

fn table_type_field(
    ui: &mut egui::Ui,
    shared: &mut SharedState,
    table_type_ids: &[String],
    index: usize,
    combo_width: f32,
) -> bool {
    let mut changed = false;
    ui.label("Table type:");
    let current = shared.people[index].table_type.clone();
    let selected = current
        .clone()
        .map(|id| truncate_label(&id, TABLE_TYPE_MAX_CHARS))
        .unwrap_or_else(|| "(any)".to_string());
    egui::ComboBox::from_id_salt(("person_table_type", index))
        .selected_text(selected)
        .width(combo_width)
        .show_ui(ui, |ui| {
            if ui.selectable_label(current.is_none(), "(any)").clicked() {
                shared.people[index].table_type = None;
                changed = true;
            }
            for table_type_id in table_type_ids {
                let is_selected = current.as_deref() == Some(table_type_id.as_str());
                if ui.selectable_label(is_selected, table_type_id).clicked() {
                    shared.people[index].table_type = Some(table_type_id.clone());
                    changed = true;
                }
            }
        });
    changed
}

fn locked_table_field(
    ui: &mut egui::Ui,
    shared: &mut SharedState,
    index: usize,
    combo_width: f32,
) -> bool {
    let mut changed = false;
    ui.label("Locked table:");
    let current_table = shared.people[index].locked_table;
    egui::ComboBox::from_id_salt(("person_locked_table", index))
        .selected_text(
            current_table
                .map(|t| t.to_string())
                .unwrap_or_else(|| "(none)".to_string()),
        )
        .width(combo_width)
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
                let is_selected = current_table == Some(table_number);
                if ui
                    .selectable_label(is_selected, table_number.to_string())
                    .clicked()
                {
                    shared.people[index].locked_table = Some(table_number);
                    changed = true;
                }
            }
        });
    changed
}

/// The "Locked seat:" combo is only meaningful once a locked table is set,
/// so it's wrapped in `add_enabled_ui`. Two tooltip paths cover it: the
/// combo's own response (returned by `ComboBox::show_ui`, whose `.enabled`
/// correctly reflects the inner disabled scope) carries
/// `on_disabled_hover_text`; the label — which always senses hover, enabled
/// or not — unconditionally carries `on_hover_text` explaining the same
/// rule, so the hint is reachable regardless of exactly which widget the
/// pointer lands on.
///
/// Previously this called `.on_disabled_hover_text` on the `InnerResponse`
/// from `add_enabled_ui` itself. That response is allocated by
/// `Ui::scope_dyn` on the *outer*, still-enabled ui
/// (`self.allocate_rect(child_ui.min_rect(), Sense::hover())`), so its
/// `.enabled` is always `true` regardless of the `enabled` flag passed to
/// `add_enabled_ui` — the disabled-hover branch in
/// `Response::on_disabled_hover_ui` (`if !self.enabled && ...`) could never
/// fire.
fn locked_seat_field(
    ui: &mut egui::Ui,
    shared: &mut SharedState,
    index: usize,
    combo_width: f32,
) -> bool {
    let mut changed = false;
    let locked_table = shared.people[index].locked_table;
    let capacity = locked_table.and_then(|t| shared.table_capacities.get(&t).copied());
    ui.label("Locked seat:")
        .on_hover_text("Seat index only applies once this guest has a locked table.");
    ui.add_enabled_ui(locked_table.is_some(), |ui| {
        let current_seat = shared.people[index].locked_seat;
        let combo_response = egui::ComboBox::from_id_salt(("person_locked_seat", index))
            .selected_text(
                current_seat
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "(none)".to_string()),
            )
            .width(combo_width)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(current_seat.is_none(), "(none)")
                    .clicked()
                {
                    shared.people[index].locked_seat = None;
                    changed = true;
                }
                for seat in 0..capacity.unwrap_or(0) {
                    let is_selected = current_seat == Some(seat);
                    if ui.selectable_label(is_selected, seat.to_string()).clicked() {
                        shared.people[index].locked_seat = Some(seat);
                        changed = true;
                    }
                }
            })
            .response;
        combo_response.on_disabled_hover_text("Select a locked table first");
    });
    changed
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

const CLOSENESS_WIDE_MIN_WIDTH: f32 = 710.0;
const CLOSENESS_MEDIUM_MIN_WIDTH: f32 = 450.0;
const REFERENCE_COMBO_W: f32 = 150.0;
const REFERENCE_MAX_CHARS: usize = 15;
const SCORE_FIELD_W: f32 = 70.0;

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

    if let Some(PendingImport::Closeness { path, rules }) = &shared.pending_import {
        let summary = format!("{} closeness rules from {}", rules.len(), path.display());
        if let Some(decision) = import_decision_modal(ui.ctx(), "Import Closeness Rules", &summary)
        {
            shared.resolve_pending_import(decision);
        }
    }

    if shared.closeness_rules.is_empty() {
        ui.label(egui::RichText::new("No closeness rules yet — click \"+ Add Rule\".").weak());
        return;
    }

    let options = reference_id_options(&shared.people);
    let mut delete_index = None;
    for index in 0..shared.closeness_rules.len() {
        ui.group(|ui| {
            let mut changed = false;

            // Explicit width-branched layout — same invariant as
            // `people_section`. Every field here has a deterministic width
            // (Left/Right combo text is truncated so egui's "at least"
            // ComboBox width never grows past `REFERENCE_COMBO_W`), so no
            // line needs a flex field to stay within `w`; the tiers only
            // decide how many fields share a line.
            let w = ui.available_width();
            let wide = w >= CLOSENESS_WIDE_MIN_WIDTH;
            let medium = !wide && w >= CLOSENESS_MEDIUM_MIN_WIDTH;

            if wide {
                ui.horizontal(|ui| {
                    changed |= left_field(ui, shared, &options, index);
                    changed |= right_field(ui, shared, &options, index);
                    changed |= score_field(ui, shared, index);
                    if delete_button(ui) {
                        delete_index = Some(index);
                    }
                });
            } else if medium {
                ui.horizontal(|ui| {
                    changed |= left_field(ui, shared, &options, index);
                    changed |= right_field(ui, shared, &options, index);
                });
                ui.horizontal(|ui| {
                    changed |= score_field(ui, shared, index);
                    if delete_button(ui) {
                        delete_index = Some(index);
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    changed |= left_field(ui, shared, &options, index);
                });
                ui.horizontal(|ui| {
                    changed |= right_field(ui, shared, &options, index);
                });
                ui.horizontal(|ui| {
                    changed |= score_field(ui, shared, index);
                    if delete_button(ui) {
                        delete_index = Some(index);
                    }
                });
            }

            let row = &shared.closeness_rules[index];
            let (left, right) = (
                row.left_id.trim().to_string(),
                row.right_id.trim().to_string(),
            );
            for error in &shared.validation {
                if let Some((error_left, error_right)) = error.closeness_pair()
                    && rules_match(error_left, error_right, &left, &right)
                {
                    ui.colored_label(ERROR_COLOR, error.to_string());
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

fn left_field(
    ui: &mut egui::Ui,
    shared: &mut SharedState,
    options: &[ReferenceIdOption],
    index: usize,
) -> bool {
    let mut changed = false;
    ui.label("Left:");
    let current = shared.closeness_rules[index].left_id.clone();
    let selected = if current.is_empty() {
        "(select)".to_string()
    } else {
        truncate_label(&reference_label(&current, options), REFERENCE_MAX_CHARS)
    };
    egui::ComboBox::from_id_salt(("closeness_left", index))
        .selected_text(selected)
        .width(REFERENCE_COMBO_W)
        .show_ui(ui, |ui| {
            for option in options {
                let is_selected = current == option.id;
                if ui.selectable_label(is_selected, &option.label).clicked() {
                    shared.closeness_rules[index].left_id = option.id.clone();
                    changed = true;
                }
            }
        });
    changed
}

fn right_field(
    ui: &mut egui::Ui,
    shared: &mut SharedState,
    options: &[ReferenceIdOption],
    index: usize,
) -> bool {
    let mut changed = false;
    ui.label("Right:");
    let current = shared.closeness_rules[index].right_id.clone();
    let selected = if current.is_empty() {
        "(select)".to_string()
    } else {
        truncate_label(&reference_label(&current, options), REFERENCE_MAX_CHARS)
    };
    egui::ComboBox::from_id_salt(("closeness_right", index))
        .selected_text(selected)
        .width(REFERENCE_COMBO_W)
        .show_ui(ui, |ui| {
            for option in options {
                let is_selected = current == option.id;
                if ui.selectable_label(is_selected, &option.label).clicked() {
                    shared.closeness_rules[index].right_id = option.id.clone();
                    changed = true;
                }
            }
        });
    changed
}

fn score_field(ui: &mut egui::Ui, shared: &mut SharedState, index: usize) -> bool {
    ui.label("Score:");
    ui.add(
        egui::TextEdit::singleline(&mut shared.closeness_rules[index].score_input)
            .hint_text("e.g. 2.0")
            .desired_width(SCORE_FIELD_W),
    )
    .changed()
}

// ── Tables ──────────────────────────────────────────────────────────────────

const TABLES_WIDE_MIN_WIDTH: f32 = 760.0;
const TABLES_MEDIUM_MIN_WIDTH: f32 = 460.0;
const TABLE_ID_W: f32 = 110.0;
const SHAPE_COMBO_W: f32 = 130.0;
const NUM_FIELD_W: f32 = 45.0;
const COUNT_FIELD_W: f32 = 60.0;
/// Shortened from the original "People per side (top|right|bottom|left):" —
/// at the panel's minimum width (280pt) that full label alone risks
/// approaching the available width before the field is even placed. The
/// abbreviated form plus the existing `1|1|1|1` hint text keeps the meaning
/// while giving the field's live leftover-width computation real room on
/// every tier.
const PPS_LABEL: &str = "Per side (T|R|B|L):";
const PPS_MIN_W: f32 = 70.0;
const PPS_MAX_W: f32 = 220.0;

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

    if let Some(PendingImport::Tables { path, tables }) = &shared.pending_import {
        let summary = format!("{} table types from {}", tables.len(), path.display());
        if let Some(decision) = import_decision_modal(ui.ctx(), "Import Table Types", &summary) {
            shared.resolve_pending_import(decision);
        }
    }

    if shared.table_configs.is_empty() {
        ui.label(egui::RichText::new("No table types yet — click \"+ Add Table Type\".").weak());
        return;
    }

    let mut delete_index = None;
    for index in 0..shared.table_configs.len() {
        ui.group(|ui| {
            let mut changed = false;

            // Explicit width-branched layout — see `people_section` for the
            // invariant. Every field width here is a deterministic budget
            // (TextEdit widths are exact; the shape combo's vocabulary is
            // fixed and short enough to fit `SHAPE_COMBO_W`), except the
            // people-per-side field, whose own dedicated line makes it safe
            // to size as pure live leftover.
            let w = ui.available_width();
            let wide = w >= TABLES_WIDE_MIN_WIDTH;
            let medium = !wide && w >= TABLES_MEDIUM_MIN_WIDTH;

            if wide {
                ui.horizontal(|ui| {
                    changed |= table_id_field(ui, shared, index, TABLE_ID_W);
                    changed |= shape_field(ui, shared, index, SHAPE_COMBO_W);
                    if delete_button(ui) {
                        delete_index = Some(index);
                    }
                });
                ui.horizontal(|ui| {
                    changed |= max_field(ui, shared, index, NUM_FIELD_W);
                    changed |= min_field(ui, shared, index, NUM_FIELD_W);
                    changed |= recommended_field(ui, shared, index, NUM_FIELD_W);
                    changed |= count_field(ui, shared, index, COUNT_FIELD_W);
                });
            } else if medium {
                ui.horizontal(|ui| {
                    changed |= table_id_field(ui, shared, index, TABLE_ID_W);
                    changed |= shape_field(ui, shared, index, SHAPE_COMBO_W);
                    if delete_button(ui) {
                        delete_index = Some(index);
                    }
                });
                ui.horizontal(|ui| {
                    changed |= max_field(ui, shared, index, NUM_FIELD_W);
                    changed |= min_field(ui, shared, index, NUM_FIELD_W);
                    changed |= recommended_field(ui, shared, index, NUM_FIELD_W);
                });
                ui.horizontal(|ui| {
                    changed |= count_field(ui, shared, index, COUNT_FIELD_W);
                });
            } else {
                ui.horizontal(|ui| {
                    let id_w = (w - ROW_SPACING).max(80.0);
                    changed |= table_id_field(ui, shared, index, id_w);
                });
                ui.horizontal(|ui| {
                    changed |= shape_field(ui, shared, index, SHAPE_COMBO_W);
                    if delete_button(ui) {
                        delete_index = Some(index);
                    }
                });
                ui.horizontal(|ui| {
                    changed |= max_field(ui, shared, index, NUM_FIELD_W);
                    changed |= min_field(ui, shared, index, NUM_FIELD_W);
                });
                ui.horizontal(|ui| {
                    changed |= recommended_field(ui, shared, index, NUM_FIELD_W);
                });
                ui.horizontal(|ui| {
                    changed |= count_field(ui, shared, index, COUNT_FIELD_W);
                });
            }

            if matches!(
                shared.table_configs[index].shape,
                TableShape::Rectangular | TableShape::Square
            ) {
                ui.horizontal(|ui| {
                    changed |= people_per_side_field(ui, shared, index);
                });
            }

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

fn table_id_field(ui: &mut egui::Ui, shared: &mut SharedState, index: usize, width: f32) -> bool {
    ui.add(
        egui::TextEdit::singleline(&mut shared.table_configs[index].table_type_id)
            .hint_text("table_type_id")
            .desired_width(width),
    )
    .changed()
}

fn shape_field(
    ui: &mut egui::Ui,
    shared: &mut SharedState,
    index: usize,
    combo_width: f32,
) -> bool {
    let mut changed = false;
    let current_shape = shared.table_configs[index].shape.clone();
    egui::ComboBox::from_id_salt(("table_shape", index))
        .selected_text(table_shape_label(&current_shape))
        .width(combo_width)
        .show_ui(ui, |ui| {
            for shape in [
                TableShape::Round,
                TableShape::Rectangular,
                TableShape::Square,
            ] {
                let is_selected = current_shape == shape;
                if ui
                    .selectable_label(is_selected, table_shape_label(&shape))
                    .clicked()
                {
                    shared.table_configs[index].shape = shape;
                    changed = true;
                }
            }
        });
    changed
}

fn max_field(ui: &mut egui::Ui, shared: &mut SharedState, index: usize, width: f32) -> bool {
    ui.label("max:");
    ui.add(
        egui::TextEdit::singleline(&mut shared.table_configs[index].max_people_input)
            .desired_width(width),
    )
    .changed()
}

fn min_field(ui: &mut egui::Ui, shared: &mut SharedState, index: usize, width: f32) -> bool {
    ui.label("min:");
    ui.add(
        egui::TextEdit::singleline(&mut shared.table_configs[index].min_people_input)
            .desired_width(width),
    )
    .changed()
}

fn recommended_field(
    ui: &mut egui::Ui,
    shared: &mut SharedState,
    index: usize,
    width: f32,
) -> bool {
    ui.label("recommended:");
    ui.add(
        egui::TextEdit::singleline(&mut shared.table_configs[index].recommended_people_input)
            .desired_width(width),
    )
    .changed()
}

fn count_field(ui: &mut egui::Ui, shared: &mut SharedState, index: usize, width: f32) -> bool {
    ui.label("count:");
    ui.add(
        egui::TextEdit::singleline(&mut shared.table_configs[index].number_of_tables_input)
            .hint_text("unlimited")
            .desired_width(width),
    )
    .changed()
}

fn people_per_side_field(ui: &mut egui::Ui, shared: &mut SharedState, index: usize) -> bool {
    ui.label(PPS_LABEL);
    // Live leftover on this row's own dedicated line: correct regardless of
    // the label's actual rendered width, since it's queried *after* the
    // label is placed.
    let field_w = ui.available_width().clamp(PPS_MIN_W, PPS_MAX_W);
    ui.add(
        egui::TextEdit::singleline(&mut shared.table_configs[index].people_per_side_input)
            .hint_text("1|1|1|1")
            .desired_width(field_w),
    )
    .changed()
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

/// Delete button shared by every editor row across People/Closeness/Tables.
fn delete_button(ui: &mut egui::Ui) -> bool {
    ui.button("Delete").clicked()
}

/// Show the Replace/Merge/Cancel modal for a pending CSV import. Returns
/// the user's decision the frame a button is clicked, `None` while still
/// awaiting input. `title` doubles as the modal's stable `Id` salt.
fn import_decision_modal(
    ctx: &egui::Context,
    title: &str,
    summary: &str,
) -> Option<ImportDecision> {
    let mut decision = None;
    egui::Modal::new(egui::Id::new(("import_decision_modal", title))).show(ctx, |ui| {
        ui.heading(title);
        ui.label(format!("Loaded {summary}."));
        ui.label("Replace clears existing entries first. Merge overwrites matching entries and keeps the rest.");
        ui.horizontal(|ui| {
            if ui.button("Replace").clicked() {
                decision = Some(ImportDecision::Replace);
            }
            if ui.button("Merge").clicked() {
                decision = Some(ImportDecision::Merge);
            }
            if ui.button("Cancel").clicked() {
                decision = Some(ImportDecision::Cancel);
            }
        });
    });
    decision
}

/// Truncate a user-entered id/name before handing it to
/// `ComboBox::selected_text`. egui 0.27's `ComboBox::width` is only a
/// *minimum* — `combo_box_dyn` computes `width.at_least(full_minimum_width)`
/// from the unwrapped selected-text galley, so an unbounded string would
/// grow the combo past the width a row's layout budgeted for it, undoing the
/// whole point of the explicit-width tiers above.
fn truncate_label(text: &str, max_chars: usize) -> String {
    if text.chars().count() > max_chars {
        format!("{}…", text.chars().take(max_chars).collect::<String>())
    } else {
        text.to_string()
    }
}
