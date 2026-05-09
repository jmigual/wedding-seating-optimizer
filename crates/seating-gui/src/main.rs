use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Sandbox, Settings, Theme};
use rfd::FileDialog;
use seating_core::{
    make_project, parse_closeness_csv, parse_people_csv, parse_tables_json, write_closeness_csv, write_people_csv,
    write_seating_csv, write_tables_json, HeuristicOptimizer, OptimizationConfig, SeatingOptimizer,
};
use std::fs;

fn main() -> iced::Result {
    GuiApp::run(Settings::default())
}

#[derive(Default)]
struct GuiApp {
    people_csv: String,
    closeness_csv: String,
    tables_json: String,
    output_csv: String,
    seed: String,
    message: String,
}

#[derive(Debug, Clone)]
enum Msg {
    NewProject,
    LoadPeople,
    SavePeople,
    LoadCloseness,
    SaveCloseness,
    LoadTables,
    SaveTables,
    Optimize,
    SaveSeating,
    PeopleChanged(String),
    ClosenessChanged(String),
    TablesChanged(String),
    SeedChanged(String),
}

impl Sandbox for GuiApp {
    type Message = Msg;

    fn new() -> Self {
        Self {
            people_csv: "id,name,table_type,groups,locked_table,locked_seat\n".to_string(),
            closeness_csv: "left_id,right_id,score\n".to_string(),
            tables_json: "{\n  \"round_10\": {\n    \"shape\": \"round\",\n    \"max_people\": 10,\n    \"recommended_people\": 9,\n    \"min_people\": 8,\n    \"number_of_tables\": 1\n  }\n}\n".to_string(),
            output_csv: String::new(),
            seed: "42".to_string(),
            message: "Create a new project or load files.".to_string(),
        }
    }

    fn title(&self) -> String {
        "Wedding Seating GUI".to_string()
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
                    self.people_csv = content;
                    self.message = "Loaded people CSV.".to_string();
                }
                Ok(None) => {}
                Err(e) => self.message = format!("Load failed: {e}"),
            },
            Msg::SavePeople => {
                let data = self.people_csv.clone();
                self.save_data(&data, "people.csv");
            }
            Msg::LoadCloseness => match Self::load_file() {
                Ok(Some(content)) => {
                    self.closeness_csv = content;
                    self.message = "Loaded closeness CSV.".to_string();
                }
                Ok(None) => {}
                Err(e) => self.message = format!("Load failed: {e}"),
            },
            Msg::SaveCloseness => {
                let data = self.closeness_csv.clone();
                self.save_data(&data, "closeness.csv");
            }
            Msg::LoadTables => match Self::load_file() {
                Ok(Some(content)) => {
                    self.tables_json = content;
                    self.message = "Loaded tables JSON.".to_string();
                }
                Ok(None) => {}
                Err(e) => self.message = format!("Load failed: {e}"),
            },
            Msg::SaveTables => {
                let data = self.tables_json.clone();
                self.save_data(&data, "tables.json");
            }
            Msg::PeopleChanged(value) => self.people_csv = value,
            Msg::ClosenessChanged(value) => self.closeness_csv = value,
            Msg::TablesChanged(value) => self.tables_json = value,
            Msg::SeedChanged(value) => self.seed = value,
            Msg::Optimize => {
                let seed = self.seed.parse::<u64>().unwrap_or(42);
                match make_project(&self.people_csv, &self.closeness_csv, &self.tables_json) {
                    Ok(project) => match HeuristicOptimizer.optimize(
                        &project,
                        &OptimizationConfig {
                            seed,
                            iterations: 200,
                            solutions: 1,
                            recommended_capacity_weight: 1.0,
                        },
                    ) {
                        Ok(result) => {
                            if let Some(best) = result.solutions.first() {
                                match write_seating_csv(&best.assignments) {
                                    Ok(csv) => {
                                        self.output_csv = csv;
                                        self.message =
                                            format!("Optimization complete. Score: {:.3}", best.score);
                                    }
                                    Err(e) => self.message = format!("Export error: {e}"),
                                }
                            } else {
                                self.message = "No solution returned.".to_string();
                            }
                        }
                        Err(e) => self.message = format!("Optimization failed:\n{e}"),
                    },
                    Err(e) => self.message = format!("Input error: {e}"),
                }
            }
            Msg::SaveSeating => {
                let data = self.output_csv.clone();
                self.save_data(&data, "seating.csv");
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
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

        let optimize_row = row![
            text("Seed:"),
            text_input("42", &self.seed).on_input(Msg::SeedChanged).width(Length::Fixed(100.0)),
            button("Run Optimize").on_press(Msg::Optimize),
            button("Save Seating").on_press(Msg::SaveSeating),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let editors = row![
            column![
                text("People CSV"),
                text_input("", &self.people_csv)
                    .on_input(Msg::PeopleChanged)
                    .padding(8)
                    .size(14)
            ]
            .width(Length::FillPortion(1)),
            column![
                text("Closeness CSV"),
                text_input("", &self.closeness_csv)
                    .on_input(Msg::ClosenessChanged)
                    .padding(8)
                    .size(14)
            ]
            .width(Length::FillPortion(1)),
            column![
                text("Tables JSON"),
                text_input("", &self.tables_json)
                    .on_input(Msg::TablesChanged)
                    .padding(8)
                    .size(14)
            ]
            .width(Length::FillPortion(1)),
        ]
        .spacing(12);

        let output = column![
            text("Result seating CSV / visual table-order view"),
            scrollable(text(&self.output_csv).size(14)).height(Length::Fixed(200.0)),
            text(&self.message)
        ]
        .spacing(8);

        container(
            column![controls, optimize_row, editors, output]
                .spacing(12)
                .padding(12),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

impl GuiApp {
    fn load_file() -> Result<Option<String>, String> {
        if let Some(path) = FileDialog::new().pick_file() {
            fs::read_to_string(&path).map(Some).map_err(|e| e.to_string())
        } else {
            Ok(None)
        }
    }

    fn save_data(&mut self, source: &str, default_name: &str) {
        if let Some(path) = FileDialog::new().set_file_name(default_name).save_file() {
            let output = match default_name {
                "people.csv" => parse_people_csv(source).and_then(|p| write_people_csv(&p)),
                "closeness.csv" => parse_closeness_csv(source).and_then(|c| write_closeness_csv(&c)),
                "tables.json" => parse_tables_json(source).and_then(|t| write_tables_json(&t)),
                _ => Ok(source.to_string()),
            };

            match output {
                Ok(text) => match fs::write(&path, text) {
                    Ok(_) => self.message = format!("Saved {}", path.display()),
                    Err(e) => self.message = format!("Save failed: {e}"),
                },
                Err(e) => self.message = format!("Cannot save invalid data: {e}"),
            }
        }
    }
}
