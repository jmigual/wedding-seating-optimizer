//! `SeatingApp`: top-level `eframe::App` state and update loop.

use crate::panels::{self, CanvasState, EditorsState};
use crate::state::{MessageKind, SharedState};
use eframe::egui::{self, Color32};
use seating_core::{
    COLOR_BACKGROUND, COLOR_CARD, HeuristicOptimizer, OptimizationResult, SeatingOptimizer,
    ValidationReport, validate_project,
};
use std::sync::mpsc;
use std::thread;

/// A whole-project action that would discard unsaved work, awaiting a
/// second confirming activation of the same button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingConfirm {
    New,
    Open,
}

pub(crate) struct SeatingApp {
    pub(crate) shared: SharedState,
    editors_state: EditorsState,
    canvas_state: CanvasState,
    pub(crate) is_optimizing: bool,
    optimize_rx: Option<mpsc::Receiver<Result<OptimizationResult, ValidationReport>>>,
    pub(crate) pending_confirm: Option<PendingConfirm>,
    /// Last `dirty` value reflected in the window title, so we only send a
    /// `ViewportCommand::Title` when it actually changes.
    shown_dirty: bool,
    /// Set once the dark navy chrome (`egui::Visuals`) has been applied, so
    /// `update` only sets it on the first frame.
    chrome_set: bool,
}

impl SeatingApp {
    pub(crate) fn new(initial_project: Option<std::path::PathBuf>) -> Self {
        let mut shared = SharedState::empty();
        if let Some(path) = initial_project {
            shared.open_project_path(path);
        }
        Self {
            shared,
            editors_state: EditorsState::default(),
            canvas_state: CanvasState::default(),
            is_optimizing: false,
            optimize_rx: None,
            pending_confirm: None,
            shown_dirty: false,
            chrome_set: false,
        }
    }

    /// Kick off an optimizer run on a background thread so the UI stays
    /// responsive. `HeuristicOptimizer::optimize` is synchronous and can
    /// take a while, so it must never run on the UI thread.
    pub(crate) fn start_optimize(&mut self, ctx: &egui::Context) {
        if self.is_optimizing {
            return;
        }
        let config = match self.shared.materialize_optimization_config() {
            Ok(config) => config,
            Err(report) => {
                self.shared.set_message(
                    MessageKind::Error,
                    format!(
                        "Cannot optimize invalid settings: {}",
                        SharedState::report_summary(&report)
                    ),
                );
                return;
            }
        };
        let project = match self.shared.materialize_project() {
            Ok(project) => project,
            Err(report) => {
                self.shared.set_message(
                    MessageKind::Error,
                    format!(
                        "Cannot optimize invalid data: {}",
                        SharedState::report_summary(&report)
                    ),
                );
                return;
            }
        };
        if let Err(report) = validate_project(&project) {
            self.shared.set_message(
                MessageKind::Error,
                format!(
                    "Cannot optimize invalid data: {}",
                    SharedState::report_summary(&report)
                ),
            );
            self.shared.validation = report.errors;
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.optimize_rx = Some(rx);
        self.is_optimizing = true;
        self.shared.set_message(MessageKind::Info, "Optimizing…");

        let repaint_ctx = ctx.clone();
        thread::spawn(move || {
            let result = HeuristicOptimizer.optimize(&project, &config);
            let _ = tx.send(result);
            repaint_ctx.request_repaint();
        });
    }

    fn poll_optimize(&mut self) {
        let Some(rx) = &self.optimize_rx else {
            return;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.is_optimizing = false;
                self.optimize_rx = None;
                self.shared
                    .set_message(MessageKind::Error, "Optimizer thread stopped unexpectedly.");
                return;
            }
        };
        self.is_optimizing = false;
        self.optimize_rx = None;
        match result {
            Ok(result) => match result.solutions.into_iter().next() {
                Some(solution) => {
                    let score = solution.score;
                    self.shared.assignments = solution.assignments;
                    self.shared.refresh();
                    self.shared.set_message(
                        MessageKind::Success,
                        format!("Optimization complete. Score: {score:.3}"),
                    );
                }
                None => self
                    .shared
                    .set_message(MessageKind::Error, "Optimizer returned no solutions."),
            },
            Err(report) => self.shared.set_message(
                MessageKind::Error,
                format!(
                    "Optimization failed: {}",
                    SharedState::report_summary(&report)
                ),
            ),
        }
    }
}

impl eframe::App for SeatingApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.chrome_set {
            ctx.set_visuals(navy_visuals());
            self.chrome_set = true;
        }
        self.poll_optimize();

        let dirty = self.shared.dirty;
        if dirty != self.shown_dirty {
            let title = if dirty {
                "Wedding Seating *"
            } else {
                "Wedding Seating"
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.to_string()));
            self.shown_dirty = dirty;
        }

        egui::Panel::top("top_bar").show(ui, |ui| {
            panels::top_bar::show(self, &ctx, ui);
        });
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            panels::status_bar::show(&self.shared, ui);
        });
        egui::Panel::left("editors_panel")
            .resizable(true)
            .default_size(380.0)
            .size_range(280.0..=f32::INFINITY)
            .show(ui, |ui| {
                // `auto_shrink` off: with horizontal scrolling disabled,
                // egui's default shrink-to-content otherwise makes the
                // ScrollArea (and thus the panel) re-report the content's
                // natural width every frame, overriding a user's resize drag.
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        panels::editors::show(&mut self.shared, &mut self.editors_state, ui);
                    });
            });
        egui::CentralPanel::default().show(ui, |ui| {
            panels::canvas::show(&mut self.shared, &mut self.canvas_state, ui);
        });
    }
}

/// Dark navy chrome matching the SVG/PNG export palette, so panel/window
/// fills harmonize with the canvas instead of egui's default gray.
fn navy_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = rgb(COLOR_CARD);
    visuals.window_fill = rgb(COLOR_BACKGROUND);
    visuals.extreme_bg_color = rgb(COLOR_BACKGROUND);
    visuals
}

fn rgb((r, g, b): (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(r, g, b)
}
