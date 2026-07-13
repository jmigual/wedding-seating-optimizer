//! Bottom status bar: severity-colored last message and a validation count.

use crate::state::{MessageKind, SharedState};
use eframe::egui;

pub(crate) fn show(shared: &SharedState, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let color = match shared.message_kind {
            MessageKind::Info => ui.visuals().text_color(),
            MessageKind::Success => egui::Color32::from_rgb(90, 200, 120),
            MessageKind::Error => egui::Color32::from_rgb(220, 90, 90),
        };
        ui.colored_label(color, &shared.message);

        ui.separator();

        let error_count = shared.validation.len();
        if error_count == 0 {
            ui.label("No validation errors");
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(220, 90, 90),
                format!("{error_count} validation error(s)"),
            );
        }
    });
}
