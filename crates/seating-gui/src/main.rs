//! # wedding-seating GUI
//!
//! Native desktop application for the wedding seating optimizer, built with
//! [iced](https://github.com/iced-rs/iced).

mod app;
mod components;
mod state;
mod styles;

use app::GuiApp;
use iced::{Sandbox, Settings};

fn main() -> iced::Result {
    GuiApp::run(Settings::default())
}
