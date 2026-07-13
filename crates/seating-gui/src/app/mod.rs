//! Native iced application shell.

mod project;
mod update;
mod views;

use crate::state::{
    display_path, selected_string_choice, selected_usize_choice, ClosenessRowState,
    MaybeStringChoice, MaybeUsizeChoice, MessageKind, Msg, PendingConfirm, PersonRowState,
    ShapeChoice, Tab, TableConfigState,
};
use crate::styles::{
    app_background_style, canvas_style, message_style, row_card_style, shell_style, AppButtonStyle,
};
use iced::keyboard::{key, Key, Modifiers};
use iced::widget::{button, container, pick_list, scrollable, svg, text, text_input};
use iced::{
    executor, theme, Alignment, Application, Command, Element, Length, Subscription, Theme,
};
use rfd::FileDialog;
use seating_core::{
    build_layout, build_table_type_map, generate_table_instances, parse_closeness_csv,
    parse_f64_value, parse_optional_usize_value, parse_people_csv, parse_people_per_side,
    parse_project_file, parse_required_usize_value, parse_tables_csv, reference_id_options,
    reference_label, reference_matches, render_png, render_svg, rules_match, validate_project,
    write_closeness_csv, write_people_csv, write_project_file, write_seating_csv, write_tables_csv,
    ClosenessRule, HeuristicOptimizer, OptimizationConfig, OptimizationResult, Person, ProjectFile,
    ProjectInput, RenderOptions, SeatingAssignment, SeatingLayout, SeatingOptimizer, TableShape,
    TableTypeConfig, ValidationError, ValidationReport,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

pub(crate) struct GuiApp {
    active_tab: Tab,
    people: Vec<PersonRowState>,
    closeness_rules: Vec<ClosenessRowState>,
    table_configs: Vec<TableConfigState>,
    assignments: Vec<SeatingAssignment>,
    layout: Option<SeatingLayout>,
    layout_svg: Option<String>,
    layout_error: Option<String>,
    seed: String,
    attempts: String,
    iterations: String,
    solutions: String,
    proximity_weight: String,
    used_table_weight: String,
    optimal_table_size_weight: String,
    zoom: f32,
    validation_errors: Vec<ValidationError>,
    generated_table_numbers: Vec<usize>,
    table_capacities: BTreeMap<usize, usize>,
    message: String,
    message_kind: MessageKind,
    project_path: Option<PathBuf>,
    people_path: Option<PathBuf>,
    closeness_path: Option<PathBuf>,
    tables_path: Option<PathBuf>,
    seating_path: Option<PathBuf>,
    dirty: bool,
    pending_confirm: Option<PendingConfirm>,
    is_optimizing: bool,
}

impl GuiApp {
    fn empty() -> Self {
        let defaults = OptimizationConfig::default();
        let mut app = Self {
            active_tab: Tab::People,
            people: Vec::new(),
            closeness_rules: Vec::new(),
            table_configs: Vec::new(),
            assignments: Vec::new(),
            layout: None,
            layout_svg: None,
            layout_error: None,
            seed: defaults.seed.to_string(),
            attempts: defaults.attempts.to_string(),
            iterations: defaults.iterations.to_string(),
            solutions: defaults.solutions.to_string(),
            proximity_weight: defaults.proximity_weight.to_string(),
            used_table_weight: defaults.used_table_weight.to_string(),
            optimal_table_size_weight: defaults.optimal_table_size_weight.to_string(),
            zoom: 1.0,
            validation_errors: Vec::new(),
            generated_table_numbers: Vec::new(),
            table_capacities: BTreeMap::new(),
            message: "Create a new project or import CSV files.".to_string(),
            message_kind: MessageKind::Info,
            project_path: None,
            people_path: None,
            closeness_path: None,
            tables_path: None,
            seating_path: None,
            dirty: false,
            pending_confirm: None,
            is_optimizing: false,
        };
        app.recompute_validation_and_layout();
        app
    }
}

impl Application for GuiApp {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Msg;
    type Theme = Theme;

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (Self::empty(), Command::none())
    }

    fn title(&self) -> String {
        if self.dirty {
            "Wedding Seating *".to_string()
        } else {
            "Wedding Seating".to_string()
        }
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        update::update(self, message)
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        iced::keyboard::on_key_press(handle_key_press)
    }

    fn view(&self) -> Element<'_, Self::Message> {
        self.view_shell()
    }
}

fn handle_key_press(key: Key, modifiers: Modifiers) -> Option<Msg> {
    match key.as_ref() {
        Key::Named(key::Named::Tab) if modifiers.shift() => Some(Msg::FocusPrevious),
        Key::Named(key::Named::Tab) => Some(Msg::FocusNext),
        _ => None,
    }
}
