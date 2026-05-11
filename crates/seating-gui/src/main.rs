//! # wedding-seating GUI
//!
//! Native desktop application for the wedding seating optimizer, built with
//! [iced](https://github.com/iced-rs/iced).

mod app;
mod components;
mod state;
mod styles;

use app::GuiApp;
use iced::{Application, Settings};

fn main() -> iced::Result {
    let mut settings = Settings::default();
    settings.window.icon =
        iced::window::icon::from_file_data(include_bytes!("../assets/app-icon.png"), None).ok();
    GuiApp::run(settings)
}
