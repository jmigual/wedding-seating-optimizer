//! # wedding-seating GUI
//!
//! Native desktop application for the wedding seating optimizer, built with
//! [iced](https://github.com/iced-rs/iced).
#![windows_subsystem = "windows"]

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
    // Sizes are logical points, but on Windows the first window can be created
    // before the DPI scale is known — pick an initial size that stays usable
    // under either interpretation and let winit clamp it to the monitor.
    settings.window.size = iced::Size::new(1440.0, 860.0);
    settings.window.min_size = Some(iced::Size::new(900.0, 620.0));
    GuiApp::run(settings)
}
