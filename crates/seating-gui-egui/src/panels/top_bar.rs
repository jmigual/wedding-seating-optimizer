//! Top panel: project actions, Optimize, and the score badge.

use crate::app::{PendingConfirm, SeatingApp};
use crate::state::{MessageKind, SharedState};
use eframe::egui;

pub(crate) fn show(app: &mut SeatingApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    let mut new_or_open_clicked = false;
    let mut other_action_clicked = false;

    ui.horizontal(|ui| {
        ui.heading("Wedding Seating");
        ui.separator();

        // Disabled while optimizing: a late result would otherwise be stamped
        // onto the freshly loaded/blank project.
        ui.add_enabled_ui(!app.is_optimizing, |ui| {
            if ui.button("New").clicked() {
                new_or_open_clicked = true;
                discard_and_run(app, PendingConfirm::New, SharedState::new_project);
            }
            if ui.button("Open").clicked() {
                new_or_open_clicked = true;
                discard_and_run(app, PendingConfirm::Open, SharedState::open_project);
            }
        });
        if ui.button("Save").clicked() {
            other_action_clicked = true;
            app.shared.save_project(false);
        }
        if ui.button("Save As").clicked() {
            other_action_clicked = true;
            app.shared.save_project(true);
        }
        if ui.button("Export CSVs").clicked() {
            other_action_clicked = true;
            app.shared.export_csvs();
        }

        ui.separator();

        ui.add_enabled_ui(!app.is_optimizing, |ui| {
            if ui.button("Optimize").clicked() {
                other_action_clicked = true;
                app.start_optimize(ctx);
            }
        });
        if app.is_optimizing {
            ui.spinner();
        }

        ui.separator();
        let score_text = match &app.shared.score_breakdown {
            Some(b) => format!(
                "Score: {:.1}  (proximity {:+.1} · tables {:+.1} · size {:+.1})",
                b.total, b.proximity, -b.used_table_penalty, -b.size_penalty
            ),
            None => "Score: —".to_string(),
        };
        ui.strong(score_text);
    });

    // Cancel pending discard confirmation only if a different action button was clicked.
    if other_action_clicked {
        app.pending_confirm = None;
    }

    ui.add_space(4.0);
}

/// Run `action` on `app.shared`, first requiring a second click to confirm
/// if there are unsaved changes.
fn discard_and_run(app: &mut SeatingApp, kind: PendingConfirm, action: fn(&mut SharedState)) {
    if app.pending_confirm == Some(kind) {
        app.pending_confirm = None;
        action(&mut app.shared);
    } else if app.shared.dirty {
        app.pending_confirm = Some(kind);
        app.shared.set_message(
            MessageKind::Info,
            "Unsaved changes — click again to discard them.",
        );
    } else {
        action(&mut app.shared);
    }
}
