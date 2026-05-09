//! # wedding-seating GUI
//!
//! Native desktop application for the wedding seating optimizer, built with
//! [iced](https://github.com/iced-rs/iced).
//!
//! All business logic lives in the `seating-core` crate.  This binary is
//! responsible only for rendering the UI, handling user actions, and delegating
//! to `seating-core` for parsing, validation, optimization, and serialization.
//!
//! ## Features
//!
//! - Create a new blank project or load existing CSV/JSON files.
//! - Edit the People CSV, Closeness CSV, and Tables JSON in multiline editors.
//! - Set the RNG seed and run the optimizer.
//! - View the resulting seating in text form.
//! - Save / export any file back to disk in the canonical format.
//!
//! ## Limitations
//!
//! Optimization runs synchronously on the GUI thread (via `iced::Sandbox`).
//! For large guest lists, a future version should switch to `iced::Application`
//! with async `Command`s or a background thread so the window stays responsive.

use iced::widget::{button, column, container, row, scrollable, text, text_editor, text_input};
use iced::{Alignment, Element, Length, Sandbox, Settings, Theme};
use rfd::FileDialog;
use seating_core::{
    make_project, parse_closeness_csv, parse_people_csv, parse_tables_json, write_closeness_csv,
    write_people_csv, write_seating_csv, write_tables_json, HeuristicOptimizer, OptimizationConfig,
    SeatingOptimizer,
};
use std::fs;

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> iced::Result {
    GuiApp::run(Settings::default())
}

// ── Application state ─────────────────────────────────────────────────────────

/// Root application state.
///
/// Each `Content` field is the multiline editor state for one of the three
/// input files.  The optimizer output is stored as a plain `String` since it
/// is read-only in the current UI.
struct GuiApp {
    /// Multiline editor content for the people CSV.
    people_content: text_editor::Content,
    /// Multiline editor content for the closeness CSV.
    closeness_content: text_editor::Content,
    /// Multiline editor content for the tables JSON.
    tables_content: text_editor::Content,
    /// Read-only seating output CSV (populated after optimization).
    output_csv: String,
    /// Current seed value shown in the seed input.
    seed: String,
    /// Last status / error message shown to the user.
    message: String,
}

// ── Messages ──────────────────────────────────────────────────────────────────

/// All messages that can be dispatched by the UI.
#[derive(Debug, Clone)]
enum Msg {
    /// Reset the application to a fresh blank project.
    NewProject,
    /// Open a file dialog to load the people CSV.
    LoadPeople,
    /// Save the people CSV to a user-chosen file.
    SavePeople,
    /// Open a file dialog to load the closeness CSV.
    LoadCloseness,
    /// Save the closeness CSV to a user-chosen file.
    SaveCloseness,
    /// Open a file dialog to load the tables JSON.
    LoadTables,
    /// Save the tables JSON to a user-chosen file.
    SaveTables,
    /// Run the optimizer and populate the output area.
    Optimize,
    /// Save the seating output CSV to a user-chosen file.
    SaveSeating,
    /// Text editing action forwarded from the people CSV editor.
    PeopleEdited(text_editor::Action),
    /// Text editing action forwarded from the closeness CSV editor.
    ClosenessEdited(text_editor::Action),
    /// Text editing action forwarded from the tables JSON editor.
    TablesEdited(text_editor::Action),
    /// Update the seed text field.
    SeedChanged(String),
}

// ── Sandbox implementation ────────────────────────────────────────────────────

impl Sandbox for GuiApp {
    type Message = Msg;

    fn new() -> Self {
        Self {
            people_content: text_editor::Content::with_text(
                "id,name,table_type,groups,locked_table,locked_seat\n",
            ),
            closeness_content: text_editor::Content::with_text("left_id,right_id,score\n"),
            tables_content: text_editor::Content::with_text(concat!(
                "{\n",
                "  \"round_10\": {\n",
                "    \"shape\": \"round\",\n",
                "    \"max_people\": 10,\n",
                "    \"recommended_people\": 9,\n",
                "    \"min_people\": 8,\n",
                "    \"number_of_tables\": 1\n",
                "  }\n",
                "}\n"
            )),
            output_csv: String::new(),
            seed: "42".to_string(),
            message: "Create a new project or load files.".to_string(),
        }
    }

    fn title(&self) -> String {
        "Wedding Seating".to_string()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Msg::NewProject => {
                *self = Self::new();
                self.message = "New project created.".to_string();
            }

            Msg::LoadPeople => match Self::load_file() {
                Ok(Some(content)) => {
                    self.people_content = text_editor::Content::with_text(&content);
                    self.message = "Loaded people CSV.".to_string();
                }
                Ok(None) => {}
                Err(e) => self.message = format!("Load failed: {e}"),
            },
            Msg::SavePeople => {
                let data = self.people_content.text();
                self.save_data(&data, "people.csv");
            }

            Msg::LoadCloseness => match Self::load_file() {
                Ok(Some(content)) => {
                    self.closeness_content = text_editor::Content::with_text(&content);
                    self.message = "Loaded closeness CSV.".to_string();
                }
                Ok(None) => {}
                Err(e) => self.message = format!("Load failed: {e}"),
            },
            Msg::SaveCloseness => {
                let data = self.closeness_content.text();
                self.save_data(&data, "closeness.csv");
            }

            Msg::LoadTables => match Self::load_file() {
                Ok(Some(content)) => {
                    self.tables_content = text_editor::Content::with_text(&content);
                    self.message = "Loaded tables JSON.".to_string();
                }
                Ok(None) => {}
                Err(e) => self.message = format!("Load failed: {e}"),
            },
            Msg::SaveTables => {
                let data = self.tables_content.text();
                self.save_data(&data, "tables.json");
            }

            Msg::PeopleEdited(action) => self.people_content.perform(action),
            Msg::ClosenessEdited(action) => self.closeness_content.perform(action),
            Msg::TablesEdited(action) => self.tables_content.perform(action),
            Msg::SeedChanged(value) => self.seed = value,

            Msg::Optimize => self.run_optimize(),

            Msg::SaveSeating => {
                let data = self.output_csv.clone();
                self.save_data(&data, "seating.csv");
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // Top button row: project-level actions.
        let controls = row![
            button("New").on_press(Msg::NewProject),
            button("Load People").on_press(Msg::LoadPeople),
            button("Save People").on_press(Msg::SavePeople),
            button("Load Closeness").on_press(Msg::LoadCloseness),
            button("Save Closeness").on_press(Msg::SaveCloseness),
            button("Load Tables").on_press(Msg::LoadTables),
            button("Save Tables").on_press(Msg::SaveTables),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        // Second row: optimizer controls.
        let optimize_row = row![
            text("Seed:"),
            text_input("42", &self.seed)
                .on_input(Msg::SeedChanged)
                .width(Length::Fixed(100.0)),
            button("Run Optimize").on_press(Msg::Optimize),
            button("Save Seating").on_press(Msg::SaveSeating),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        // Three side-by-side multiline editors for the input files.
        let editors = row![
            column![
                text("People CSV"),
                text_editor(&self.people_content)
                    .on_action(Msg::PeopleEdited)
                    .height(Length::Fill)
                    .padding(8),
            ]
            .width(Length::FillPortion(1)),
            column![
                text("Closeness CSV"),
                text_editor(&self.closeness_content)
                    .on_action(Msg::ClosenessEdited)
                    .height(Length::Fill)
                    .padding(8),
            ]
            .width(Length::FillPortion(1)),
            column![
                text("Tables JSON"),
                text_editor(&self.tables_content)
                    .on_action(Msg::TablesEdited)
                    .height(Length::Fill)
                    .padding(8),
            ]
            .width(Length::FillPortion(1)),
        ]
        .spacing(12)
        .height(Length::FillPortion(3));

        // Output area showing the seating CSV and the last status message.
        let output_area = column![
            text("Result seating CSV"),
            scrollable(text(&self.output_csv).size(14)).height(Length::Fixed(200.0)),
            text(&self.message),
        ]
        .spacing(8);

        container(
            column![controls, optimize_row, editors, output_area]
                .spacing(12)
                .padding(12),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

impl GuiApp {
    /// Open a native file-picker dialog and read the chosen file's contents.
    ///
    /// Returns `Ok(None)` when the user cancels the dialog without choosing
    /// a file.
    fn load_file() -> Result<Option<String>, String> {
        if let Some(path) = FileDialog::new().pick_file() {
            fs::read_to_string(&path).map(Some).map_err(|e| e.to_string())
        } else {
            Ok(None)
        }
    }

    /// Open a native save dialog and write `source` to the chosen path.
    ///
    /// For the three input file types the data is first round-tripped through
    /// the core parse/serialize functions to ensure the saved file is always
    /// in the canonical format.
    fn save_data(&mut self, source: &str, default_name: &str) {
        let Some(path) = FileDialog::new().set_file_name(default_name).save_file() else {
            return;
        };
        let output = match default_name {
            "people.csv" => parse_people_csv(source).and_then(|p| write_people_csv(&p)),
            "closeness.csv" => parse_closeness_csv(source).and_then(|c| write_closeness_csv(&c)),
            "tables.json" => parse_tables_json(source).and_then(|t| write_tables_json(&t)),
            _ => Ok(source.to_string()),
        };
        match output {
            Ok(text) => match fs::write(&path, text) {
                Ok(_) => self.message = format!("Saved to {}", path.display()),
                Err(e) => self.message = format!("Save failed: {e}"),
            },
            Err(e) => self.message = format!("Cannot save invalid data: {e}"),
        }
    }

    /// Parse inputs, run the optimizer, and store the result in `output_csv`.
    ///
    /// This runs synchronously on the GUI thread.  For large guest lists the
    /// window will be unresponsive while the optimizer is running.
    fn run_optimize(&mut self) {
        let seed = self.seed.parse::<u64>().unwrap_or(42);
        let people_text = self.people_content.text();
        let closeness_text = self.closeness_content.text();
        let tables_text = self.tables_content.text();
        match make_project(&people_text, &closeness_text, &tables_text) {
            Err(e) => self.message = format!("Input error: {e}"),
            Ok(project) => match HeuristicOptimizer.optimize(
                &project,
                &OptimizationConfig {
                    seed,
                    attempts: 10,
                    iterations: 200,
                    solutions: 1,
                    recommended_capacity_weight: 1.0,
                },
            ) {
                Err(e) => self.message = format!("Optimization failed:\n{e}"),
                Ok(result) => match result.solutions.first() {
                    None => self.message = "No solution returned.".to_string(),
                    Some(best) => match write_seating_csv(&best.assignments) {
                        Err(e) => self.message = format!("Export error: {e}"),
                        Ok(csv) => {
                            self.output_csv = csv;
                            self.message = format!("Optimization complete. Score: {:.3}", best.score);
                        }
                    },
                },
            },
        }
    }
}
