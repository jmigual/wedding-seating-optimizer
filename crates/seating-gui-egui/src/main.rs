//! # wedding-seating GUI (egui)
//!
//! Experimental native desktop application for the wedding seating
//! optimizer, built with [egui](https://github.com/emilk/egui)/`eframe`.
//! Lives alongside the existing `seating-gui` (iced) front-end.
#![windows_subsystem = "windows"]

mod app;
mod panels;
mod state;

use app::SeatingApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../seating-gui/assets/app-icon.png"))
            .ok();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(egui::vec2(1440.0, 860.0))
        .with_min_inner_size(egui::vec2(900.0, 620.0));
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Wedding Seating",
        native_options,
        Box::new(|_cc| Box::new(SeatingApp::new())),
    )
}
