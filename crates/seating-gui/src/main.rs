//! # wedding-seating GUI
//!
//! Native desktop application for the wedding seating optimizer, built with
//! [iced](https://github.com/iced-rs/iced).
//!
//! The GUI edits typed `seating-core` domain data, delegates all parsing,
//! serialization, validation, optimization, and seating-plan rendering to the
//! core crate, and only uses CSV/JSON files for import/export/save/load.

use iced::widget::{button, column, container, pick_list, row, scrollable, svg, text, text_input};
use iced::{
    theme, Alignment, Background, Border, Color, Element, Length, Sandbox, Settings, Theme,
};
use rfd::FileDialog;
use seating_core::{
    build_layout, build_table_type_map, generate_table_instances, parse_closeness_csv,
    parse_f64_value, parse_optional_usize_value, parse_people_csv, parse_required_usize_value,
    parse_tables_json, reference_id_options, render_png, render_svg, validate_project,
    write_closeness_csv, write_people_csv, write_seating_csv, write_tables_json, ClosenessRule,
    HeuristicOptimizer, OptimizationConfig, Person, ProjectInput, RenderOptions, SeatingAssignment,
    SeatingLayout, SeatingOptimizer, TableShape, TableTypeConfig, ValidationError,
    ValidationReport,
};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::PathBuf;

fn main() -> iced::Result {
    GuiApp::run(Settings::default())
}

#[derive(Debug, Clone)]
enum Msg {
    SelectTab(Tab),
    NewProject,
    ImportPeople,
    SavePeople,
    ExportPeople,
    ImportCloseness,
    SaveCloseness,
    ExportCloseness,
    ImportTables,
    SaveTables,
    ExportTables,
    AddPerson,
    DeletePerson(usize),
    UpdatePersonId(usize, String),
    UpdatePersonName(usize, String),
    UpdatePersonTableType(usize, MaybeStringChoice),
    UpdatePersonLockedTable(usize, MaybeUsizeChoice),
    UpdatePersonLockedSeat(usize, MaybeUsizeChoice),
    UpdateNewGroup(usize, String),
    AddGroup(usize),
    RemoveGroup(usize, usize),
    AddClosenessRule,
    DeleteClosenessRule(usize),
    UpdateClosenessLeft(usize, String),
    UpdateClosenessRight(usize, String),
    SelectClosenessLeft(usize, String),
    SelectClosenessRight(usize, String),
    UpdateClosenessScore(usize, String),
    AddTableConfig,
    DeleteTableConfig(usize),
    UpdateTableTypeId(usize, String),
    UpdateTableShape(usize, ShapeChoice),
    UpdateMaxPeople(usize, String),
    UpdateMinPeople(usize, String),
    UpdateRecommendedPeople(usize, String),
    UpdateNumberOfTables(usize, String),
    UpdatePeoplePerSide(usize, usize, String),
    SeedChanged(String),
    Optimize,
    SaveSeating,
    ExportPlanSvg,
    ExportPlanPng,
    ZoomIn,
    ZoomOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    People,
    Closeness,
    Tables,
    Optimize,
    SeatingPlan,
    Diagnostics,
}

impl Tab {
    const ALL: [Tab; 6] = [
        Tab::People,
        Tab::Closeness,
        Tab::Tables,
        Tab::Optimize,
        Tab::SeatingPlan,
        Tab::Diagnostics,
    ];

    fn label(self) -> &'static str {
        match self {
            Tab::People => "People",
            Tab::Closeness => "Closeness",
            Tab::Tables => "Tables",
            Tab::Optimize => "Optimize",
            Tab::SeatingPlan => "Seating Plan",
            Tab::Diagnostics => "Validation / Diagnostics",
        }
    }

    fn width(self) -> f32 {
        match self {
            Tab::Diagnostics => 230.0,
            Tab::SeatingPlan => 150.0,
            _ => 140.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MaybeStringChoice {
    label: String,
    value: Option<String>,
}

impl Display for MaybeStringChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MaybeUsizeChoice {
    label: String,
    value: Option<usize>,
}

impl Display for MaybeUsizeChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeChoice {
    Round,
    Rectangular,
    Square,
}

impl ShapeChoice {
    const ALL: [ShapeChoice; 3] = [
        ShapeChoice::Round,
        ShapeChoice::Rectangular,
        ShapeChoice::Square,
    ];
}

impl Display for ShapeChoice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            ShapeChoice::Round => "round",
            ShapeChoice::Rectangular => "rectangular",
            ShapeChoice::Square => "square",
        };
        write!(f, "{label}")
    }
}

impl From<TableShape> for ShapeChoice {
    fn from(value: TableShape) -> Self {
        match value {
            TableShape::Round => ShapeChoice::Round,
            TableShape::Rectangular => ShapeChoice::Rectangular,
            TableShape::Square => ShapeChoice::Square,
        }
    }
}

impl From<ShapeChoice> for TableShape {
    fn from(value: ShapeChoice) -> Self {
        match value {
            ShapeChoice::Round => TableShape::Round,
            ShapeChoice::Rectangular => TableShape::Rectangular,
            ShapeChoice::Square => TableShape::Square,
        }
    }
}

#[derive(Debug, Clone)]
struct PersonRowState {
    person: Person,
    new_group: String,
}

#[derive(Debug, Clone)]
struct ClosenessRowState {
    rule: ClosenessRule,
    score_input: String,
}

#[derive(Debug, Clone)]
struct TableConfigState {
    table_type_id: String,
    shape: ShapeChoice,
    max_people_input: String,
    min_people_input: String,
    recommended_people_input: String,
    number_of_tables_input: String,
    people_per_side_inputs: Vec<String>,
}

struct GuiApp {
    active_tab: Tab,
    people: Vec<PersonRowState>,
    closeness_rules: Vec<ClosenessRowState>,
    table_configs: Vec<TableConfigState>,
    assignments: Vec<SeatingAssignment>,
    seating_csv: String,
    layout: Option<SeatingLayout>,
    layout_svg: Option<String>,
    seed: String,
    zoom: f32,
    validation_errors: Vec<ValidationError>,
    message: String,
    people_path: Option<PathBuf>,
    closeness_path: Option<PathBuf>,
    tables_path: Option<PathBuf>,
    seating_path: Option<PathBuf>,
}

impl Sandbox for GuiApp {
    type Message = Msg;

    fn new() -> Self {
        let mut app = Self {
            active_tab: Tab::People,
            people: Vec::new(),
            closeness_rules: Vec::new(),
            table_configs: Vec::new(),
            assignments: Vec::new(),
            seating_csv: String::new(),
            layout: None,
            layout_svg: None,
            seed: "42".to_string(),
            zoom: 1.0,
            validation_errors: Vec::new(),
            message: "Create a new project or import CSV/JSON files.".to_string(),
            people_path: None,
            closeness_path: None,
            tables_path: None,
            seating_path: None,
        };
        app.refresh_validation_and_layout();
        app
    }

    fn title(&self) -> String {
        "Wedding Seating".to_string()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Msg::SelectTab(tab) => self.active_tab = tab,
            Msg::NewProject => {
                *self = Self::new();
                self.message = "Started a new empty project.".to_string();
            }
            Msg::ImportPeople => self.import_people(),
            Msg::SavePeople => self.save_people(false),
            Msg::ExportPeople => self.save_people(true),
            Msg::ImportCloseness => self.import_closeness(),
            Msg::SaveCloseness => self.save_closeness(false),
            Msg::ExportCloseness => self.save_closeness(true),
            Msg::ImportTables => self.import_tables(),
            Msg::SaveTables => self.save_tables(false),
            Msg::ExportTables => self.save_tables(true),
            Msg::AddPerson => {
                self.people.push(PersonRowState::default());
                self.refresh_validation_and_layout();
            }
            Msg::DeletePerson(index) => {
                if index < self.people.len() {
                    self.people.remove(index);
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdatePersonId(index, value) => {
                if let Some(row) = self.people.get_mut(index) {
                    row.person.id = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdatePersonName(index, value) => {
                if let Some(row) = self.people.get_mut(index) {
                    row.person.name = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdatePersonTableType(index, choice) => {
                if let Some(row) = self.people.get_mut(index) {
                    row.person.table_type = choice.value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdatePersonLockedTable(index, choice) => {
                if let Some(row) = self.people.get_mut(index) {
                    row.person.locked_table = choice.value;
                    if row.person.locked_table.is_none() {
                        row.person.locked_seat = None;
                    }
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdatePersonLockedSeat(index, choice) => {
                if let Some(row) = self.people.get_mut(index) {
                    row.person.locked_seat = choice.value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdateNewGroup(index, value) => {
                if let Some(row) = self.people.get_mut(index) {
                    row.new_group = value;
                }
            }
            Msg::AddGroup(index) => {
                if let Some(row) = self.people.get_mut(index) {
                    let group = row.new_group.trim();
                    if !group.is_empty()
                        && !row.person.groups.iter().any(|existing| existing == group)
                    {
                        row.person.groups.push(group.to_string());
                    }
                    row.new_group.clear();
                    self.refresh_validation_and_layout();
                }
            }
            Msg::RemoveGroup(person_index, group_index) => {
                if let Some(row) = self.people.get_mut(person_index) {
                    if group_index < row.person.groups.len() {
                        row.person.groups.remove(group_index);
                        self.refresh_validation_and_layout();
                    }
                }
            }
            Msg::AddClosenessRule => {
                self.closeness_rules.push(ClosenessRowState::default());
                self.refresh_validation_and_layout();
            }
            Msg::DeleteClosenessRule(index) => {
                if index < self.closeness_rules.len() {
                    self.closeness_rules.remove(index);
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdateClosenessLeft(index, value) => {
                if let Some(row) = self.closeness_rules.get_mut(index) {
                    row.rule.left_id = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdateClosenessRight(index, value) => {
                if let Some(row) = self.closeness_rules.get_mut(index) {
                    row.rule.right_id = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::SelectClosenessLeft(index, value) => {
                if let Some(row) = self.closeness_rules.get_mut(index) {
                    row.rule.left_id = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::SelectClosenessRight(index, value) => {
                if let Some(row) = self.closeness_rules.get_mut(index) {
                    row.rule.right_id = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdateClosenessScore(index, value) => {
                if let Some(row) = self.closeness_rules.get_mut(index) {
                    row.score_input = value;
                    if let Ok(score) = parse_f64_value(&row.score_input, "score") {
                        row.rule.score = score;
                    }
                    self.refresh_validation_and_layout();
                }
            }
            Msg::AddTableConfig => {
                self.table_configs.push(TableConfigState::default());
                self.refresh_validation_and_layout();
            }
            Msg::DeleteTableConfig(index) => {
                if index < self.table_configs.len() {
                    self.table_configs.remove(index);
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdateTableTypeId(index, value) => {
                if let Some(row) = self.table_configs.get_mut(index) {
                    row.table_type_id = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdateTableShape(index, shape) => {
                if let Some(row) = self.table_configs.get_mut(index) {
                    row.shape = shape;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdateMaxPeople(index, value) => {
                if let Some(row) = self.table_configs.get_mut(index) {
                    row.max_people_input = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdateMinPeople(index, value) => {
                if let Some(row) = self.table_configs.get_mut(index) {
                    row.min_people_input = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdateRecommendedPeople(index, value) => {
                if let Some(row) = self.table_configs.get_mut(index) {
                    row.recommended_people_input = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdateNumberOfTables(index, value) => {
                if let Some(row) = self.table_configs.get_mut(index) {
                    row.number_of_tables_input = value;
                    self.refresh_validation_and_layout();
                }
            }
            Msg::UpdatePeoplePerSide(index, side, value) => {
                if let Some(row) = self.table_configs.get_mut(index) {
                    row.set_people_per_side_input(side, value);
                    self.refresh_validation_and_layout();
                }
            }
            Msg::SeedChanged(value) => self.seed = value,
            Msg::Optimize => self.run_optimize(),
            Msg::SaveSeating => self.save_seating(),
            Msg::ExportPlanSvg => self.export_plan_svg(),
            Msg::ExportPlanPng => self.export_plan_png(),
            Msg::ZoomIn => self.zoom = (self.zoom + 0.2).min(3.0),
            Msg::ZoomOut => self.zoom = (self.zoom - 0.2).max(0.4),
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let header = container(
            row![
                text("Wedding Seating").size(16),
                container(row![]).width(Length::Fill),
                button(text("New Project").size(13))
                    .on_press(Msg::NewProject)
                    .padding([8, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
            ]
            .spacing(16)
            .align_items(Alignment::Center),
        )
        .padding([14, 28])
        .width(Length::Fill)
        .style(header_style);

        let tabs = Tab::ALL.into_iter().fold(
            row![].spacing(8).align_items(Alignment::Center),
            |tabs, tab| tabs.push(self.tab_button(tab)),
        );

        let content = match self.active_tab {
            Tab::People => self.view_people_tab(),
            Tab::Closeness => self.view_closeness_tab(),
            Tab::Tables => self.view_tables_tab(),
            Tab::Optimize => self.view_optimize_tab(),
            Tab::SeatingPlan => self.view_seating_plan_tab(),
            Tab::Diagnostics => self.view_diagnostics_tab(),
        };

        let message = container(text(&self.message).size(13))
            .padding([12, 18])
            .width(Length::Fill)
            .style(message_style);

        let shell = container(
            column![
                header,
                container(tabs)
                    .padding([20, 28, 10, 28])
                    .width(Length::Fill),
                container(message)
                    .padding([0, 28, 10, 28])
                    .width(Length::Fill),
                container(content)
                    .padding([18, 28, 28, 28])
                    .width(Length::Fill)
                    .height(Length::Fill)
            ]
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .max_width(1440)
        .style(shell_style);

        container(shell)
            .padding(34)
            .center_x()
            .width(Length::Fill)
            .height(Length::Fill)
            .style(app_background_style)
            .into()
    }
}

impl GuiApp {
    fn tab_button(&self, tab: Tab) -> Element<'_, Msg> {
        button(text(tab.label()).size(13))
            .on_press(Msg::SelectTab(tab))
            .padding([10, 14])
            .width(Length::Fixed(tab.width()))
            .style(theme::Button::custom(AppButtonStyle::tab(
                tab == self.active_tab,
            )))
            .into()
    }

    fn view_people_tab(&self) -> Element<'_, Msg> {
        let actions = container(
            row![
                button(text("Import People CSV").size(13))
                    .on_press(Msg::ImportPeople)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Save People CSV").size(13))
                    .on_press(Msg::SavePeople)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Export People CSV As...").size(13))
                    .on_press(Msg::ExportPeople)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                container(row![]).width(Length::Fill),
                button(text("Add Person").size(13))
                    .on_press(Msg::AddPerson)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::primary())),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .style(toolbar_style);

        let mut content = column![actions].spacing(12);

        for (index, row_state) in self.people.iter().enumerate() {
            let table_type_options = self.person_table_type_options(&row_state.person.table_type);
            let locked_table_options = self.locked_table_options(row_state.person.locked_table);
            let locked_seat_options = self.locked_seat_options(&row_state.person);
            let groups = row_state.person.groups.iter().enumerate().fold(
                row![].spacing(6),
                |group_row, (group_index, group)| {
                    group_row.push(
                        button(text(format!("{group} x")).size(12))
                            .on_press(Msg::RemoveGroup(index, group_index))
                            .padding([5, 9])
                            .style(theme::Button::custom(AppButtonStyle::chip())),
                    )
                },
            );
            let errors = self.person_errors(&row_state.person);

            let card = column![
                row![
                    text(format!("Person {}", index + 1)).width(Length::Fixed(90.0)),
                    text_input("id", &row_state.person.id)
                        .on_input(move |value| Msg::UpdatePersonId(index, value))
                        .width(Length::Fixed(140.0)),
                    text_input("name", &row_state.person.name)
                        .on_input(move |value| Msg::UpdatePersonName(index, value))
                        .width(Length::Fixed(180.0)),
                    pick_list(
                        table_type_options.clone(),
                        selected_string_choice(&table_type_options, &row_state.person.table_type),
                        move |choice| Msg::UpdatePersonTableType(index, choice),
                    )
                    .placeholder("table type")
                    .width(Length::Fixed(220.0)),
                    pick_list(
                        locked_table_options.clone(),
                        selected_usize_choice(&locked_table_options, row_state.person.locked_table),
                        move |choice| Msg::UpdatePersonLockedTable(index, choice),
                    )
                    .placeholder("locked table")
                    .width(Length::Fixed(170.0)),
                    pick_list(
                        locked_seat_options.clone(),
                        selected_usize_choice(&locked_seat_options, row_state.person.locked_seat),
                        move |choice| Msg::UpdatePersonLockedSeat(index, choice),
                    )
                    .placeholder("locked seat")
                    .width(Length::Fixed(170.0)),
                    button(text("Delete").size(13))
                        .on_press(Msg::DeletePerson(index))
                        .padding([8, 12])
                        .style(theme::Button::custom(AppButtonStyle::danger())),
                ]
                .spacing(8)
                .align_items(Alignment::Center),
                row![
                    text("Groups:"),
                    groups,
                    text_input("new group", &row_state.new_group)
                        .on_input(move |value| Msg::UpdateNewGroup(index, value))
                        .width(Length::Fixed(180.0)),
                    button(text("Add Group").size(13))
                        .on_press(Msg::AddGroup(index))
                        .padding([8, 12])
                        .style(theme::Button::custom(AppButtonStyle::secondary())),
                ]
                .spacing(8)
                .align_items(Alignment::Center),
                self.error_column(errors),
            ]
            .spacing(8);

            content = content.push(
                container(card)
                    .padding(12)
                    .width(Length::Fill)
                    .style(row_card_style),
            );
        }

        scrollable(content).into()
    }

    fn view_closeness_tab(&self) -> Element<'_, Msg> {
        let options = reference_id_options(&self.people_data());
        let actions = container(
            row![
                button(text("Import Closeness CSV").size(13))
                    .on_press(Msg::ImportCloseness)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Save Closeness CSV").size(13))
                    .on_press(Msg::SaveCloseness)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Export Closeness CSV As...").size(13))
                    .on_press(Msg::ExportCloseness)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                container(row![]).width(Length::Fill),
                button(text("Add Rule").size(13))
                    .on_press(Msg::AddClosenessRule)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::primary())),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .style(toolbar_style);

        let mut content = column![actions].spacing(12);

        for (index, row_state) in self.closeness_rules.iter().enumerate() {
            let left_matches = self.reference_matches(&options, &row_state.rule.left_id);
            let right_matches = self.reference_matches(&options, &row_state.rule.right_id);
            let errors = self.closeness_errors(&row_state.rule, &row_state.score_input);

            content = content.push(
                container(column![
                    row![
                        text(format!("Rule {}", index + 1)).width(Length::Fixed(80.0)),
                        text_input("left id", &row_state.rule.left_id)
                            .on_input(move |value| Msg::UpdateClosenessLeft(index, value))
                            .width(Length::Fixed(180.0)),
                        text_input("right id", &row_state.rule.right_id)
                            .on_input(move |value| Msg::UpdateClosenessRight(index, value))
                            .width(Length::Fixed(180.0)),
                        text_input("score", &row_state.score_input)
                            .on_input(move |value| Msg::UpdateClosenessScore(index, value))
                            .width(Length::Fixed(120.0)),
                        button(text("Delete").size(13))
                            .on_press(Msg::DeleteClosenessRule(index))
                            .padding([8, 12])
                            .style(theme::Button::custom(AppButtonStyle::danger())),
                    ]
                    .spacing(8)
                    .align_items(Alignment::Center),
                    row![
                        text(format!(
                            "Left: {}",
                            self.reference_label(&row_state.rule.left_id, &options)
                        ))
                        .width(Length::FillPortion(1)),
                        text(format!(
                            "Right: {}",
                            self.reference_label(&row_state.rule.right_id, &options)
                        ))
                        .width(Length::FillPortion(1)),
                    ]
                    .spacing(8),
                    self.suggestion_row("Left suggestions:", left_matches, move |value| {
                        Msg::SelectClosenessLeft(index, value)
                    },),
                    self.suggestion_row("Right suggestions:", right_matches, move |value| {
                        Msg::SelectClosenessRight(index, value)
                    },),
                    self.error_column(errors),
                ])
                .padding(12)
                .width(Length::Fill)
                .style(row_card_style),
            );
        }

        scrollable(content).into()
    }

    fn view_tables_tab(&self) -> Element<'_, Msg> {
        let generated_instances = self.generated_table_summary();
        let actions = container(
            row![
                button(text("Import Tables JSON").size(13))
                    .on_press(Msg::ImportTables)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Save Tables JSON").size(13))
                    .on_press(Msg::SaveTables)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Export Tables JSON As...").size(13))
                    .on_press(Msg::ExportTables)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                container(row![]).width(Length::Fill),
                button(text("Add Table Type").size(13))
                    .on_press(Msg::AddTableConfig)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::primary())),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .style(toolbar_style);

        let mut content = column![actions, text("Generated table instances")].spacing(12);

        for summary in generated_instances {
            content = content.push(text(summary));
        }

        for (index, row_state) in self.table_configs.iter().enumerate() {
            let errors = self.table_errors(row_state);
            let people_per_side = if row_state.shape == ShapeChoice::Round {
                column![text("people_per_side is disabled for round tables.")]
            } else {
                column![
                    text("people_per_side (top, right, bottom, left)"),
                    row![
                        text_input("top", row_state.people_per_side_input(0))
                            .on_input(move |value| Msg::UpdatePeoplePerSide(index, 0, value))
                            .width(Length::Fixed(100.0)),
                        text_input("right", row_state.people_per_side_input(1))
                            .on_input(move |value| Msg::UpdatePeoplePerSide(index, 1, value))
                            .width(Length::Fixed(100.0)),
                        text_input("bottom", row_state.people_per_side_input(2))
                            .on_input(move |value| Msg::UpdatePeoplePerSide(index, 2, value))
                            .width(Length::Fixed(100.0)),
                        text_input("left", row_state.people_per_side_input(3))
                            .on_input(move |value| Msg::UpdatePeoplePerSide(index, 3, value))
                            .width(Length::Fixed(100.0)),
                    ]
                    .spacing(8)
                ]
            };

            content = content.push(
                container(column![
                    row![
                        text(format!("Table type {}", index + 1)).width(Length::Fixed(110.0)),
                        text_input("table_type_id", &row_state.table_type_id)
                            .on_input(move |value| Msg::UpdateTableTypeId(index, value))
                            .width(Length::Fixed(180.0)),
                        pick_list(
                            ShapeChoice::ALL.to_vec(),
                            Some(row_state.shape),
                            move |shape| Msg::UpdateTableShape(index, shape),
                        )
                        .width(Length::Fixed(150.0)),
                        button(text("Delete").size(13))
                            .on_press(Msg::DeleteTableConfig(index))
                            .padding([8, 12])
                            .style(theme::Button::custom(AppButtonStyle::danger())),
                    ]
                    .spacing(8)
                    .align_items(Alignment::Center),
                    row![
                        text_input("max_people", &row_state.max_people_input)
                            .on_input(move |value| Msg::UpdateMaxPeople(index, value))
                            .width(Length::Fixed(120.0)),
                        text_input("min_people", &row_state.min_people_input)
                            .on_input(move |value| Msg::UpdateMinPeople(index, value))
                            .width(Length::Fixed(120.0)),
                        text_input("recommended_people", &row_state.recommended_people_input)
                            .on_input(move |value| Msg::UpdateRecommendedPeople(index, value))
                            .width(Length::Fixed(170.0)),
                        text_input("number_of_tables", &row_state.number_of_tables_input)
                            .on_input(move |value| Msg::UpdateNumberOfTables(index, value))
                            .width(Length::Fixed(160.0)),
                    ]
                    .spacing(8),
                    people_per_side,
                    self.error_column(errors),
                ])
                .padding(12)
                .width(Length::Fill)
                .style(row_card_style),
            );
        }

        scrollable(content).into()
    }

    fn view_optimize_tab(&self) -> Element<'_, Msg> {
        let seating_preview = if self.seating_csv.is_empty() {
            "No seating CSV yet. Run Optimize to generate a plan.".to_string()
        } else {
            self.seating_csv.clone()
        };

        let actions = container(
            row![
                text("Seed").size(13),
                text_input("42", &self.seed)
                    .on_input(Msg::SeedChanged)
                    .width(Length::Fixed(100.0)),
                container(row![]).width(Length::Fill),
                button(text("Run Optimize").size(13))
                    .on_press(Msg::Optimize)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::primary())),
                button(text("Save Seating CSV").size(13))
                    .on_press(Msg::SaveSeating)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .style(toolbar_style);

        scrollable(
            column![
                actions,
                text("Current seating assignments").size(15),
                container(text(seating_preview).size(13))
                    .padding(14)
                    .width(Length::Fill)
                    .style(row_card_style),
            ]
            .spacing(12),
        )
        .into()
    }

    fn view_seating_plan_tab(&self) -> Element<'_, Msg> {
        let controls = container(
            row![
                button(text("Export seating plan as SVG").size(13))
                    .on_press(Msg::ExportPlanSvg)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Export seating plan as PNG").size(13))
                    .on_press(Msg::ExportPlanPng)
                    .padding([9, 14])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                container(row![]).width(Length::Fill),
                button(text("Zoom -").size(13))
                    .on_press(Msg::ZoomOut)
                    .padding([9, 12])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                button(text("Zoom +").size(13))
                    .on_press(Msg::ZoomIn)
                    .padding([9, 12])
                    .style(theme::Button::custom(AppButtonStyle::secondary())),
                text(format!("Zoom: {:.1}x", self.zoom)).size(13),
            ]
            .spacing(8)
            .align_items(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .style(toolbar_style);

        let body: Element<'_, Msg> = match (&self.layout, &self.layout_svg) {
            (Some(layout), Some(svg_markup)) => scrollable(
                container(
                    svg(svg::Handle::from_memory(svg_markup.as_bytes().to_vec()))
                        .width(Length::Fixed(layout.width * self.zoom))
                        .height(Length::Fixed(layout.height * self.zoom)),
                )
                .width(Length::Shrink)
                .height(Length::Shrink),
            )
            .into(),
            _ => text("No valid seating plan to render yet.").into(),
        };

        column![
            controls,
            container(body)
                .padding(18)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(canvas_style)
        ]
        .spacing(12)
        .height(Length::Fill)
        .into()
    }

    fn view_diagnostics_tab(&self) -> Element<'_, Msg> {
        let errors = if self.validation_errors.is_empty() {
            vec!["No validation issues.".to_string()]
        } else {
            self.validation_errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        };

        let paths = [
            format!("People path: {}", display_path(&self.people_path)),
            format!("Closeness path: {}", display_path(&self.closeness_path)),
            format!("Tables path: {}", display_path(&self.tables_path)),
            format!("Seating path: {}", display_path(&self.seating_path)),
        ];

        let content = paths.into_iter().fold(
            column![text("Validation errors").size(15)].spacing(8),
            |column, entry| column.push(text(entry)),
        );
        let content = errors
            .into_iter()
            .fold(content, |column, error| column.push(text(error)));

        scrollable(
            column![container(content)
                .padding(14)
                .width(Length::Fill)
                .style(row_card_style)]
            .spacing(12),
        )
        .into()
    }

    fn people_data(&self) -> Vec<Person> {
        self.people.iter().map(|row| row.person.clone()).collect()
    }

    fn materialize_closeness_rules(&self) -> Result<Vec<ClosenessRule>, ValidationReport> {
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
            let people_per_side = if row.shape == ShapeChoice::Round {
                None
            } else {
                let mut side_values = Vec::new();
                for value in &row.people_per_side_inputs {
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

    fn current_project(&self) -> Result<ProjectInput, ValidationReport> {
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

    fn refresh_validation_and_layout(&mut self) {
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

    fn run_optimize(&mut self) {
        let seed = match parse_optional_usize_value(&self.seed, "seed") {
            Ok(Some(value)) => value as u64,
            Ok(None) => 42,
            Err(error) => {
                self.message = error.to_string();
                return;
            }
        };
        let project = match self.current_project() {
            Ok(project) => project,
            Err(report) => {
                self.message = format!("Cannot optimize invalid data:\n{report}");
                return;
            }
        };
        if let Err(report) = validate_project(&project) {
            self.validation_errors = report.errors.clone();
            self.message = format!("Cannot optimize invalid data:\n{report}");
            return;
        }
        match HeuristicOptimizer.optimize(
            &project,
            &OptimizationConfig {
                seed,
                attempts: 10,
                iterations: 200,
                solutions: 1,
                recommended_capacity_weight: 1.0,
            },
        ) {
            Err(report) => self.message = format!("Optimization failed:\n{report}"),
            Ok(result) => match result.solutions.first() {
                Some(solution) => {
                    self.assignments = solution.assignments.clone();
                    self.seating_csv = write_seating_csv(&self.assignments).unwrap_or_default();
                    self.validation_errors.clear();
                    if let Ok(layout) = build_layout(&project, &self.assignments) {
                        let svg_markup = render_svg(&layout, &RenderOptions::default());
                        self.layout = Some(layout);
                        self.layout_svg = Some(svg_markup);
                    } else {
                        self.layout = None;
                        self.layout_svg = None;
                    }
                    self.active_tab = Tab::SeatingPlan;
                    self.message = format!("Optimization complete. Score: {:.3}", solution.score);
                }
                None => self.message = "Optimizer returned no solutions.".to_string(),
            },
        }
    }

    fn import_people(&mut self) {
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

    fn import_closeness(&mut self) {
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

    fn import_tables(&mut self) {
        match Self::load_text_file() {
            Ok(Some((path, contents))) => match parse_tables_json(&contents) {
                Ok(table_types) => {
                    self.table_configs = table_types
                        .into_iter()
                        .map(|(table_type_id, config)| {
                            TableConfigState::from_pair(table_type_id, config)
                        })
                        .collect();
                    self.tables_path = Some(path);
                    self.message = "Imported tables JSON.".to_string();
                    self.refresh_validation_and_layout();
                }
                Err(error) => self.message = format!("Tables import failed: {error}"),
            },
            Ok(None) => {}
            Err(error) => self.message = format!("Tables import failed: {error}"),
        }
    }

    fn save_people(&mut self, export_as: bool) {
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

    fn save_closeness(&mut self, export_as: bool) {
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

    fn save_tables(&mut self, export_as: bool) {
        if let Some(path) = self.resolve_save_path(export_as, &self.tables_path, "tables.json") {
            let table_map = self
                .materialize_table_entries()
                .and_then(build_table_type_map);
            match table_map {
                Ok(table_types) => match write_tables_json(&table_types) {
                    Ok(contents) => match fs::write(&path, contents) {
                        Ok(_) => {
                            if !export_as {
                                self.tables_path = Some(path.clone());
                            }
                            self.message = format!(
                                "{} tables JSON to {}",
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

    fn save_seating(&mut self) {
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

    fn export_plan_svg(&mut self) {
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

    fn export_plan_png(&mut self) {
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
        export_as: bool,
        current_path: &Option<PathBuf>,
        default_name: &str,
    ) -> Option<PathBuf> {
        if export_as {
            FileDialog::new().set_file_name(default_name).save_file()
        } else {
            current_path
                .clone()
                .or_else(|| FileDialog::new().set_file_name(default_name).save_file())
        }
    }

    fn person_table_type_options(&self, current: &Option<String>) -> Vec<MaybeStringChoice> {
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

    fn locked_table_options(&self, current: Option<usize>) -> Vec<MaybeUsizeChoice> {
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

    fn locked_seat_options(&self, person: &Person) -> Vec<MaybeUsizeChoice> {
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

    fn generated_table_numbers(&self) -> Vec<usize> {
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

    fn table_capacity_for_number(&self, table_number: usize) -> Option<usize> {
        self.current_project().ok().and_then(|project| {
            generate_table_instances(&project)
                .into_iter()
                .find(|table| table.number == table_number)
                .map(|table| table.max_people)
        })
    }

    fn generated_table_summary(&self) -> Vec<String> {
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

    fn person_errors(&self, person: &Person) -> Vec<String> {
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

    fn closeness_errors(&self, rule: &ClosenessRule, score_input: &str) -> Vec<String> {
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

    fn table_errors(&self, row: &TableConfigState) -> Vec<String> {
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
            for input in &row.people_per_side_inputs {
                if let Err(error) = parse_required_usize_value(input, "people_per_side") {
                    errors.push(error.to_string());
                }
            }
        }

        errors
    }

    fn reference_matches(
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

    fn reference_label(&self, id: &str, options: &[seating_core::ReferenceIdOption]) -> String {
        options
            .iter()
            .find(|option| option.id == id)
            .map(|option| option.label.clone())
            .unwrap_or_else(|| format!("{id} — unknown"))
    }

    fn suggestion_row<F>(
        &self,
        label: &str,
        suggestions: Vec<seating_core::ReferenceIdOption>,
        on_press: F,
    ) -> Element<'_, Msg>
    where
        F: 'static + Clone + Fn(String) -> Msg,
    {
        let row = suggestions.into_iter().fold(
            row![text(label)].spacing(6).align_items(Alignment::Center),
            |suggestion_row, suggestion| {
                suggestion_row.push(
                    button(text(suggestion.label.clone()).size(12))
                        .on_press(on_press.clone()(suggestion.id.clone()))
                        .padding([5, 9])
                        .style(theme::Button::custom(AppButtonStyle::chip())),
                )
            },
        );
        row.into()
    }

    fn error_column(&self, errors: Vec<String>) -> Element<'_, Msg> {
        errors
            .into_iter()
            .fold(column![].spacing(4), |column, error| {
                column.push(text(format!("⚠ {error}")))
            })
            .into()
    }
}

#[derive(Debug, Clone, Copy)]
enum AppButtonKind {
    Tab { active: bool },
    Primary,
    Secondary,
    Danger,
    Chip,
}

#[derive(Debug, Clone, Copy)]
struct AppButtonStyle {
    kind: AppButtonKind,
}

impl AppButtonStyle {
    fn tab(active: bool) -> Self {
        Self {
            kind: AppButtonKind::Tab { active },
        }
    }

    fn primary() -> Self {
        Self {
            kind: AppButtonKind::Primary,
        }
    }

    fn secondary() -> Self {
        Self {
            kind: AppButtonKind::Secondary,
        }
    }

    fn danger() -> Self {
        Self {
            kind: AppButtonKind::Danger,
        }
    }

    fn chip() -> Self {
        Self {
            kind: AppButtonKind::Chip,
        }
    }
}

impl button::StyleSheet for AppButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        let (background, text_color, border_color) = match self.kind {
            AppButtonKind::Tab { active: true } => {
                (rgb(58, 83, 116), rgb(246, 249, 252), rgb(81, 114, 154))
            }
            AppButtonKind::Tab { active: false } => {
                (rgb(31, 48, 66), rgb(223, 231, 239), rgb(31, 48, 66))
            }
            AppButtonKind::Primary => (rgb(73, 112, 154), rgb(249, 252, 255), rgb(88, 134, 184)),
            AppButtonKind::Secondary => (rgb(32, 49, 67), rgb(230, 236, 243), rgb(42, 62, 82)),
            AppButtonKind::Danger => (rgb(116, 55, 62), rgb(255, 244, 244), rgb(144, 72, 82)),
            AppButtonKind::Chip => (rgb(40, 59, 78), rgb(230, 238, 247), rgb(62, 83, 105)),
        };

        button::Appearance {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut appearance = self.active(style);
        appearance.background = Some(Background::Color(match self.kind {
            AppButtonKind::Tab { active: true } => rgb(67, 96, 132),
            AppButtonKind::Tab { active: false } => rgb(39, 58, 78),
            AppButtonKind::Primary => rgb(84, 127, 173),
            AppButtonKind::Secondary => rgb(41, 60, 80),
            AppButtonKind::Danger => rgb(133, 63, 72),
            AppButtonKind::Chip => rgb(48, 70, 92),
        }));
        appearance
    }
}

fn app_background_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(12, 20, 28))),
        text_color: Some(rgb(232, 238, 245)),
        ..container::Appearance::default()
    }
}

fn shell_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(20, 32, 43))),
        text_color: Some(rgb(232, 238, 245)),
        border: Border {
            color: rgb(61, 88, 116),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Appearance::default()
    }
}

fn header_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(11, 17, 24))),
        text_color: Some(rgb(238, 243, 248)),
        border: Border {
            color: rgb(61, 88, 116),
            width: 1.0,
            radius: [8.0, 8.0, 8.0, 8.0].into(),
        },
        ..container::Appearance::default()
    }
}

fn message_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(32, 49, 67))),
        text_color: Some(rgb(229, 237, 246)),
        border: Border {
            color: rgb(32, 49, 67),
            width: 1.0,
            radius: 7.0.into(),
        },
        ..container::Appearance::default()
    }
}

fn toolbar_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(18, 29, 39))),
        text_color: Some(rgb(232, 238, 245)),
        border: Border {
            color: rgb(42, 63, 84),
            width: 1.0,
            radius: 7.0.into(),
        },
        ..container::Appearance::default()
    }
}

fn row_card_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(17, 28, 38))),
        text_color: Some(rgb(232, 238, 245)),
        border: Border {
            color: rgb(44, 66, 88),
            width: 1.0,
            radius: 7.0.into(),
        },
        ..container::Appearance::default()
    }
}

fn canvas_style(_theme: &Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(rgb(10, 17, 24))),
        text_color: Some(rgb(232, 238, 245)),
        border: Border {
            color: rgb(61, 88, 116),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Appearance::default()
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

impl Default for PersonRowState {
    fn default() -> Self {
        Self {
            person: Person {
                id: String::new(),
                name: String::new(),
                table_type: None,
                groups: Vec::new(),
                locked_table: None,
                locked_seat: None,
            },
            new_group: String::new(),
        }
    }
}

impl From<Person> for PersonRowState {
    fn from(person: Person) -> Self {
        Self {
            person,
            new_group: String::new(),
        }
    }
}

impl Default for ClosenessRowState {
    fn default() -> Self {
        Self {
            rule: ClosenessRule {
                left_id: String::new(),
                right_id: String::new(),
                score: 0.0,
            },
            score_input: "0".to_string(),
        }
    }
}

impl From<ClosenessRule> for ClosenessRowState {
    fn from(rule: ClosenessRule) -> Self {
        Self {
            score_input: rule.score.to_string(),
            rule,
        }
    }
}

impl Default for TableConfigState {
    fn default() -> Self {
        Self {
            table_type_id: String::new(),
            shape: ShapeChoice::Round,
            max_people_input: "8".to_string(),
            min_people_input: String::new(),
            recommended_people_input: String::new(),
            number_of_tables_input: "1".to_string(),
            people_per_side_inputs: vec![
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
            ],
        }
    }
}

impl TableConfigState {
    fn from_pair(table_type_id: String, config: TableTypeConfig) -> Self {
        let inputs = config
            .people_per_side
            .as_ref()
            .map(|values| values.iter().map(|value| value.to_string()).collect())
            .unwrap_or_else(|| {
                vec![
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                ]
            });
        Self {
            table_type_id,
            shape: ShapeChoice::from(config.shape),
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
            people_per_side_inputs: inputs,
        }
    }

    fn people_per_side_input(&self, side: usize) -> &str {
        self.people_per_side_inputs
            .get(side)
            .map(String::as_str)
            .unwrap_or("")
    }

    fn set_people_per_side_input(&mut self, side: usize, value: String) {
        if self.people_per_side_inputs.len() <= side {
            self.people_per_side_inputs.resize(side + 1, String::new());
        }
        self.people_per_side_inputs[side] = value;
    }
}

fn selected_string_choice(
    options: &[MaybeStringChoice],
    value: &Option<String>,
) -> Option<MaybeStringChoice> {
    options
        .iter()
        .find(|choice| &choice.value == value)
        .cloned()
}

fn selected_usize_choice(
    options: &[MaybeUsizeChoice],
    value: Option<usize>,
) -> Option<MaybeUsizeChoice> {
    options.iter().find(|choice| choice.value == value).cloned()
}

fn same_pair(left_a: &str, right_a: &str, left_b: &str, right_b: &str) -> bool {
    (left_a == left_b && right_a == right_b) || (left_a == right_b && right_a == left_b)
}

fn display_path(path: &Option<PathBuf>) -> String {
    path.as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(not saved yet)".to_string())
}
