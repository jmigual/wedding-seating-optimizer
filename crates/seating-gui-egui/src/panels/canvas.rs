//! Central-panel interactive seating canvas.
//!
//! Stage 4 owns this file exclusively (module ownership rule in the spec):
//! zoom/pan, seat rendering, hover tooltips, drag-and-drop via
//! [`apply_seat_drop`], score-delta toasts, and SVG/PNG export.

use crate::state::{ClosenessRow, MessageKind, SharedState};
use eframe::egui::{self, Align2, Color32, FontId, Id, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use seating_core::{
    apply_seat_drop, render_png, render_svg, LayoutSeat, LayoutTable, Person, ProjectInput,
    RenderOptions, SeatDropOutcome, SeatingAssignment, SeatingLayout, TableSurface,
    COLOR_BACKGROUND, COLOR_CARD, COLOR_MUTED, COLOR_SEAT_FILL, COLOR_SEAT_STROKE, COLOR_STROKE,
    COLOR_TABLE_FILL, COLOR_TABLE_STROKE,
};
use std::collections::HashMap;

const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 3.0;
const TOAST_LIFETIME: f64 = 2.2;

/// UI-only state for the canvas panel: zoom/pan, an in-progress drag, and
/// the fading score-delta toast.
pub(crate) struct CanvasState {
    zoom: f32,
    pan: Vec2,
    pending_fit: bool,
    drag: Option<DragState>,
    toast: Option<ScoreToast>,
    last_score: Option<f64>,
    /// Set right after a drop applies its own precise before/after toast, so
    /// the generic score-change detector (which also catches
    /// optimizer-driven changes) doesn't spawn a duplicate on the next frame.
    suppress_diff_toast: bool,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            pending_fit: false,
            drag: None,
            toast: None,
            last_score: None,
            suppress_diff_toast: false,
        }
    }
}

/// A guest picked up off a seat, mid-drag.
struct DragState {
    person_id: String,
    person_name: String,
    /// Snapshot of the project at drag-start, reused every frame to
    /// dry-run [`apply_seat_drop`] against the candidate seat under the
    /// pointer — cheap at the scale this canvas draws.
    project: ProjectInput,
    /// The dragged guest's `locked_table`, if any: while dragging, seats on
    /// this table are pre-highlighted as the only legal drop targets.
    allowed_table: Option<usize>,
}

struct ScoreToast {
    delta: f64,
    spawned_at: f64,
}

/// Screen-space transform for the current zoom/pan.
#[derive(Clone, Copy)]
struct Transform {
    origin: Pos2,
    pan: Vec2,
    zoom: f32,
}

impl Transform {
    fn to_screen(self, layout_pt: (f32, f32)) -> Pos2 {
        Pos2::new(
            self.origin.x + self.pan.x + layout_pt.0 * self.zoom,
            self.origin.y + self.pan.y + layout_pt.1 * self.zoom,
        )
    }

    fn to_layout(self, screen_pt: Pos2) -> (f32, f32) {
        (
            (screen_pt.x - self.origin.x - self.pan.x) / self.zoom,
            (screen_pt.y - self.origin.y - self.pan.y) / self.zoom,
        )
    }
}

pub(crate) fn show(shared: &mut SharedState, state: &mut CanvasState, ui: &mut egui::Ui) {
    track_score_toast(shared, state, ui.ctx());

    toolbar(shared, state, ui);
    ui.separator();

    if shared.layout.is_some() {
        canvas_area(shared, state, ui);
    } else {
        empty_state(shared, ui);
    }
}

/// Detects score changes since the last frame (drops apply their own
/// precise toast and suppress this one; this generic path exists so an
/// optimizer run — which this module has no direct signal for — still
/// flashes a delta).
fn track_score_toast(shared: &SharedState, state: &mut CanvasState, ctx: &egui::Context) {
    if shared.score == state.last_score {
        return;
    }
    if let (Some(previous), Some(current)) = (state.last_score, shared.score) {
        if state.suppress_diff_toast {
            state.suppress_diff_toast = false;
        } else {
            state.toast = Some(ScoreToast {
                delta: current - previous,
                spawned_at: ctx.input(|i| i.time),
            });
        }
    }
    state.last_score = shared.score;
}

fn toolbar(shared: &mut SharedState, state: &mut CanvasState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("Fit").clicked() {
            state.pending_fit = true;
        }
        if ui.button("Reset Zoom").clicked() {
            state.zoom = 1.0;
            state.pan = Vec2::ZERO;
        }
        if ui.button("-").clicked() {
            state.zoom = (state.zoom * 0.8).clamp(MIN_ZOOM, MAX_ZOOM);
        }
        ui.label(format!("{:.0}%", state.zoom * 100.0));
        if ui.button("+").clicked() {
            state.zoom = (state.zoom * 1.25).clamp(MIN_ZOOM, MAX_ZOOM);
        }
        ui.separator();
        ui.add_enabled_ui(shared.layout.is_some(), |ui| {
            if ui.button("Export SVG").clicked() {
                export_svg(shared);
            }
            if ui.button("Export PNG").clicked() {
                export_png(shared);
            }
        });
    });
}

/// Friendly hints distinguishing "data isn't valid yet" from "valid but
/// nothing optimized yet" from "nothing entered yet".
fn empty_state(shared: &SharedState, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);
        if !shared.validation.is_empty() {
            ui.heading("Fix validation errors first");
            ui.label(format!(
                "{} validation error(s) — see Diagnostics in the left panel.",
                shared.validation.len()
            ));
        } else if shared.people.is_empty() || shared.generated_table_numbers.is_empty() {
            ui.heading("Nothing to seat yet");
            ui.label("Add guests and at least one table type, then click Optimize.");
        } else if shared.assignments.is_empty() {
            ui.heading("No seating plan yet");
            ui.label("Click Optimize in the top bar to generate a seating plan.");
        } else {
            ui.heading("Seating plan unavailable");
            ui.label(shared.message.clone());
        }
    });
}

fn canvas_area(shared: &mut SharedState, state: &mut CanvasState, ui: &mut egui::Ui) {
    // Owned snapshot: lets the rest of this function mutate `shared` freely
    // (on a drop) without fighting a borrow of `shared.layout` — cheap at
    // this scale, and next frame's redraw picks up any change anyway.
    let layout = shared
        .layout
        .clone()
        .expect("canvas_area only called when shared.layout is Some");
    let seat_radius_base = RenderOptions::default().seat_radius;

    let desired_size = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(desired_size, Sense::hover());
    let painter = ui.painter_at(rect);

    // Pan-drag sense is inset a few points from the left edge so this
    // full-bleed rect — allocated after (and thus, per egui's hit-testing,
    // on top of) the SidePanel's own resize-drag sense — doesn't overlap the
    // resize band and steal it (egui's default `resize_grab_radius_side` is
    // 5.0 points either side of the panel border).
    let pan_rect = Rect::from_min_max(rect.min + Vec2::new(8.0, 0.0), rect.max);
    let background = ui.interact(pan_rect, ui.id().with("canvas_pan"), Sense::drag());

    if state.pending_fit {
        fit_layout(state, &layout, rect);
        state.pending_fit = false;
    }

    if background.hovered() {
        let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() > f32::EPSILON {
            let pointer = ui
                .ctx()
                .input(|i| i.pointer.hover_pos())
                .unwrap_or_else(|| rect.center());
            zoom_at(state, rect, zoom_delta, pointer);
        }
    }
    if background.dragged() {
        state.pan += background.drag_delta();
    }

    painter.rect_filled(rect, Rounding::ZERO, rgb(COLOR_BACKGROUND));

    let transform = Transform {
        origin: rect.min,
        pan: state.pan,
        zoom: state.zoom,
    };

    let person_by_id: HashMap<&str, &Person> =
        shared.people.iter().map(|p| (p.id.as_str(), p)).collect();
    let assignment_by_seat: HashMap<(usize, usize), &SeatingAssignment> = shared
        .assignments
        .iter()
        .map(|a| ((a.table_number, a.seat_index), a))
        .collect();

    let pointer_pos = ui.ctx().input(|i| i.pointer.interact_pos());
    let hit_radius = (seat_radius_base + 10.0) * state.zoom;
    let drop_preview = state.drag.as_ref().and_then(|drag| {
        let pointer = pointer_pos?;
        let (table_number, seat_index) = find_seat_under(&layout, transform, pointer, hit_radius)?;
        let ok = apply_seat_drop(
            &drag.project,
            &shared.assignments,
            &drag.person_id,
            table_number,
            seat_index,
        )
        .is_ok();
        Some((table_number, seat_index, ok))
    });

    let mut pending_drop: Option<(usize, usize)> = None;
    let mut drag_cancelled = false;

    for table in &layout.tables {
        draw_table(&painter, table, transform, state.zoom);
        for seat in &table.seats {
            let center = transform.to_screen((seat.x, seat.y));
            let radius = seat_radius_base * state.zoom;
            let person = assignment_by_seat
                .get(&(table.table_number, seat.seat_index))
                .and_then(|a| person_by_id.get(a.person_id.as_str()).copied());
            let locked = person.is_some_and(|p| p.locked_table.is_some());
            let draggable = person.is_some_and(|p| p.locked_seat.is_none());
            let is_being_dragged = state.drag.as_ref().is_some_and(|drag| {
                assignment_by_seat
                    .get(&(table.table_number, seat.seat_index))
                    .is_some_and(|a| a.person_id == drag.person_id)
            });
            let drop_highlight = drop_preview
                .filter(|(t, s, _)| *t == table.table_number && *s == seat.seat_index)
                .map(|(_, _, ok)| ok);
            // Pre-highlight the locked-table guest's only legal table while dragging.
            let table_hint = state
                .drag
                .as_ref()
                .and_then(|drag| drag.allowed_table)
                .is_some_and(|allowed| allowed == table.table_number)
                && !is_being_dragged;

            draw_seat(
                &painter,
                seat,
                center,
                radius,
                locked,
                drop_highlight,
                table_hint,
                is_being_dragged,
                state.zoom,
            );

            let hit_rect = Rect::from_center_size(center, Vec2::splat((radius * 2.0).max(20.0)));
            if draggable {
                let seat_id = Id::new(("seat_drag", table.table_number, seat.seat_index));
                let mut seat_response = ui.interact(hit_rect, seat_id, Sense::click_and_drag());

                if state.drag.is_none() {
                    if let Some(person) = person {
                        seat_response =
                            seat_response.on_hover_ui(|ui| person_tooltip(ui, person, shared));
                    }
                }
                if seat_response.drag_started() && state.drag.is_none() {
                    if let (Some(assignment), Some(person)) = (
                        assignment_by_seat.get(&(table.table_number, seat.seat_index)),
                        person,
                    ) {
                        if let Ok(project) = shared.materialize_project() {
                            state.drag = Some(DragState {
                                person_id: assignment.person_id.clone(),
                                person_name: person.name.clone(),
                                project,
                                allowed_table: person.locked_table,
                            });
                        }
                    }
                }
                if seat_response.drag_stopped() {
                    match pointer_pos.and_then(|pointer| {
                        find_seat_under(&layout, transform, pointer, hit_radius)
                    }) {
                        Some(target) => pending_drop = Some(target),
                        None => drag_cancelled = true,
                    }
                }
            } else if let Some(person) = person {
                let seat_id = Id::new(("seat_hover", table.table_number, seat.seat_index));
                let seat_response = ui.interact(hit_rect, seat_id, Sense::hover());
                if state.drag.is_none() {
                    seat_response.on_hover_ui(|ui| person_tooltip(ui, person, shared));
                }
            }
        }
    }

    if let (Some(drag), Some(pointer)) = (&state.drag, pointer_pos) {
        draw_ghost(&painter, pointer, &drag.person_name);
    }

    draw_toast(&painter, rect, state, ui.ctx());

    // Every borrow of `shared` taken above (person_by_id, assignment_by_seat)
    // is dead by now, so mutating it here is safe.
    if let Some((table_number, seat_index)) = pending_drop {
        finish_drop(shared, state, table_number, seat_index, ui.ctx());
    } else if drag_cancelled {
        state.drag = None;
        shared.set_message(
            MessageKind::Info,
            "Drop cancelled — released outside a seat.",
        );
    }
}

fn finish_drop(
    shared: &mut SharedState,
    state: &mut CanvasState,
    table_number: usize,
    seat_index: usize,
    ctx: &egui::Context,
) {
    let Some(drag) = state.drag.take() else {
        return;
    };
    let score_before = shared.score;
    match apply_seat_drop(
        &drag.project,
        &shared.assignments,
        &drag.person_id,
        table_number,
        seat_index,
    ) {
        Ok((updated, outcome)) => {
            shared.assignments = updated;
            shared.refresh();
            if let (Some(before), Some(after)) = (score_before, shared.score) {
                state.toast = Some(ScoreToast {
                    delta: after - before,
                    spawned_at: ctx.input(|i| i.time),
                });
                state.suppress_diff_toast = true;
            }
            let verb = match outcome {
                SeatDropOutcome::Moved => "Moved",
                SeatDropOutcome::Swapped => "Swapped",
            };
            shared.set_message(
                MessageKind::Success,
                format!(
                    "{verb} {} to table {table_number}, seat {seat_index}.",
                    drag.person_name
                ),
            );
        }
        Err(report) => {
            shared.set_message(
                MessageKind::Error,
                format!("Move failed: {}", SharedState::report_summary(&report)),
            );
        }
    }
}

fn zoom_at(state: &mut CanvasState, rect: Rect, factor: f32, pointer: Pos2) {
    let transform = Transform {
        origin: rect.min,
        pan: state.pan,
        zoom: state.zoom,
    };
    let layout_pt = transform.to_layout(pointer);
    let new_zoom = (state.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    let zeroed = Transform {
        origin: rect.min,
        pan: Vec2::ZERO,
        zoom: new_zoom,
    };
    state.pan = pointer - zeroed.to_screen(layout_pt);
    state.zoom = new_zoom;
}

fn fit_layout(state: &mut CanvasState, layout: &SeatingLayout, rect: Rect) {
    let padding = 24.0;
    let avail_w = (rect.width() - padding * 2.0).max(1.0);
    let avail_h = (rect.height() - padding * 2.0).max(1.0);
    let zoom = (avail_w / layout.width.max(1.0))
        .min(avail_h / layout.height.max(1.0))
        .clamp(MIN_ZOOM, MAX_ZOOM);
    state.zoom = zoom;
    let content_w = layout.width * zoom;
    let content_h = layout.height * zoom;
    state.pan = Vec2::new(
        (rect.width() - content_w) / 2.0,
        (rect.height() - content_h) / 2.0,
    );
}

/// Nearest seat (any capacity slot, occupied or not) to `pointer`, within
/// `hit_radius` screen pixels.
fn find_seat_under(
    layout: &SeatingLayout,
    transform: Transform,
    pointer: Pos2,
    hit_radius: f32,
) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, f32)> = None;
    for table in &layout.tables {
        for seat in &table.seats {
            let center = transform.to_screen((seat.x, seat.y));
            let dist = (center - pointer).length();
            if dist <= hit_radius && best.is_none_or(|(_, _, best_dist)| dist < best_dist) {
                best = Some((table.table_number, seat.seat_index, dist));
            }
        }
    }
    best.map(|(t, s, _)| (t, s))
}

fn draw_table(painter: &egui::Painter, table: &LayoutTable, transform: Transform, zoom: f32) {
    let card_rect = Rect::from_two_pos(
        transform.to_screen((table.x, table.y)),
        transform.to_screen((table.x + table.width, table.y + table.height)),
    );
    painter.rect_filled(card_rect, Rounding::same(12.0 * zoom), rgb(COLOR_CARD));
    painter.rect_stroke(
        card_rect,
        Rounding::same(12.0 * zoom),
        Stroke::new(1.2_f32, rgb(COLOR_STROKE)),
    );

    painter.text(
        Pos2::new(card_rect.center().x, card_rect.top() + 8.0 * zoom),
        Align2::CENTER_TOP,
        format!("Table {} — {}", table.table_number, table.table_type),
        FontId::proportional((14.0 * zoom).max(8.0)),
        rgb(COLOR_SEAT_FILL),
    );

    let surface_fill = rgb(COLOR_TABLE_FILL);
    let surface_stroke = Stroke::new(1.5_f32, rgb(COLOR_TABLE_STROKE));
    // Surface geometry comes straight from `table.surface`, computed in
    // `build_layout` from the same center used to place seats — do not
    // re-derive it from the card rect here (that previously drifted from
    // the seat ring whenever header space shifted the seat center).
    match &table.surface {
        TableSurface::Round { cx, cy, radius } => {
            let center = transform.to_screen((*cx, *cy));
            painter.circle(center, radius * zoom, surface_fill, surface_stroke);
        }
        TableSurface::Rect {
            x,
            y,
            width,
            height,
        } => {
            let surface_rect = Rect::from_two_pos(
                transform.to_screen((*x, *y)),
                transform.to_screen((*x + width, *y + height)),
            );
            painter.rect(
                surface_rect,
                Rounding::same(8.0 * zoom),
                surface_fill,
                surface_stroke,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_seat(
    painter: &egui::Painter,
    seat: &LayoutSeat,
    center: Pos2,
    radius: f32,
    locked: bool,
    drop_highlight: Option<bool>,
    table_hint: bool,
    dimmed: bool,
    zoom: f32,
) {
    if let Some(ok) = drop_highlight {
        let highlight = if ok {
            Color32::from_rgb(90, 200, 120)
        } else {
            Color32::from_rgb(220, 90, 90)
        };
        painter.circle_filled(center, radius + 5.0 * zoom, faded(highlight, 0.35));
    } else if table_hint {
        painter.circle_stroke(
            center,
            radius + 4.0 * zoom,
            Stroke::new(1.5_f32, faded(Color32::from_rgb(90, 200, 120), 0.55)),
        );
    }

    let occupied = seat.person_name.is_some();
    let seat_alpha = if dimmed { 0.35 } else { 1.0 };
    if occupied {
        painter.circle_filled(center, radius, faded(rgb(COLOR_SEAT_FILL), seat_alpha));
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(1.3_f32, faded(rgb(COLOR_SEAT_STROKE), seat_alpha)),
        );
    } else {
        painter.circle_stroke(center, radius, Stroke::new(1.3_f32, rgb(COLOR_STROKE)));
    }

    let label = seat
        .person_name
        .as_deref()
        .map(short_label)
        .unwrap_or_else(|| seat.seat_index.to_string());
    let text_color = if occupied {
        faded(rgb(COLOR_BACKGROUND), seat_alpha)
    } else {
        rgb(COLOR_MUTED)
    };
    painter.text(
        center,
        Align2::CENTER_CENTER,
        label,
        FontId::proportional((11.0 * zoom).max(7.0)),
        text_color,
    );

    if locked {
        let lock_pos = center + Vec2::new(radius * 0.75, -radius * 0.75);
        painter.text(
            lock_pos,
            Align2::CENTER_CENTER,
            "\u{1F512}",
            FontId::proportional((10.0 * zoom).max(7.0)),
            Color32::from_rgb(255, 210, 110),
        );
    }
}

fn draw_ghost(painter: &egui::Painter, pointer: Pos2, person_name: &str) {
    let rect = Rect::from_min_size(pointer + Vec2::new(14.0, 14.0), Vec2::new(96.0, 24.0));
    let (r, g, b) = COLOR_CARD;
    painter.rect_filled(
        rect,
        Rounding::same(6.0),
        Color32::from_rgba_unmultiplied(r, g, b, 235),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(6.0),
        Stroke::new(1.0_f32, rgb(COLOR_TABLE_STROKE)),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        short_label(person_name),
        FontId::proportional(12.0),
        rgb(COLOR_SEAT_FILL),
    );
}

/// Convert a shared `(r, g, b)` palette tuple into an egui color.
fn rgb((r, g, b): (u8, u8, u8)) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn draw_toast(painter: &egui::Painter, rect: Rect, state: &mut CanvasState, ctx: &egui::Context) {
    let Some(toast) = &state.toast else {
        return;
    };
    let elapsed = ctx.input(|i| i.time) - toast.spawned_at;
    if elapsed > TOAST_LIFETIME {
        state.toast = None;
        return;
    }
    let alpha = (1.0 - elapsed / TOAST_LIFETIME).clamp(0.0, 1.0) as f32;
    let color = if toast.delta >= 0.0 {
        Color32::from_rgb(90, 200, 120)
    } else {
        Color32::from_rgb(220, 90, 90)
    };
    painter.text(
        Pos2::new(rect.center().x, rect.top() + 12.0),
        Align2::CENTER_TOP,
        format!("{:+.1}", toast.delta),
        FontId::proportional(22.0),
        faded(color, alpha),
    );
    ctx.request_repaint();
}

fn faded(color: Color32, alpha: f32) -> Color32 {
    let [r, g, b, _] = color.to_array();
    Color32::from_rgba_unmultiplied(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}

/// First name, truncated so it fits a seat label.
fn short_label(name: &str) -> String {
    let first = name.split_whitespace().next().unwrap_or(name);
    if first.chars().count() > 10 {
        format!("{}…", first.chars().take(9).collect::<String>())
    } else {
        first.to_string()
    }
}

fn person_tooltip(ui: &mut egui::Ui, person: &Person, shared: &SharedState) {
    ui.strong(&person.name);
    ui.label(format!("id: {}", person.id));
    if !person.groups.is_empty() {
        ui.label(format!("groups: {}", person.groups.join(", ")));
    }
    match (person.locked_table, person.locked_seat) {
        (Some(table), Some(seat)) => {
            ui.label(format!("locked to table {table}, seat {seat}"));
        }
        (Some(table), None) => {
            ui.label(format!("locked to table {table}"));
        }
        _ => {}
    }

    let rules: Vec<&ClosenessRow> = shared
        .closeness_rules
        .iter()
        .filter(|rule| involves(&rule.left_id, person) || involves(&rule.right_id, person))
        .collect();
    if !rules.is_empty() {
        ui.separator();
        ui.label("Closeness rules:");
        for rule in rules {
            ui.label(format!(
                "{} \u{2194} {}: {}",
                rule.left_id, rule.right_id, rule.score_input
            ));
        }
    }
}

fn involves(id: &str, person: &Person) -> bool {
    id == person.id || person.groups.iter().any(|group| group == id)
}

fn export_svg(shared: &mut SharedState) {
    let Some(layout) = shared.layout.as_ref() else {
        return;
    };
    let svg = render_svg(layout, &RenderOptions::default());
    let Some(path) = rfd::FileDialog::new()
        .set_file_name("seating.svg")
        .add_filter("SVG", &["svg"])
        .save_file()
    else {
        return;
    };
    match std::fs::write(&path, svg) {
        Ok(()) => shared.set_message(
            MessageKind::Success,
            format!("Exported SVG to {}", path.display()),
        ),
        Err(error) => shared.set_message(MessageKind::Error, format!("SVG export failed: {error}")),
    }
}

fn export_png(shared: &mut SharedState) {
    let Some(layout) = shared.layout.as_ref() else {
        return;
    };
    let Some(path) = rfd::FileDialog::new()
        .set_file_name("seating.png")
        .add_filter("PNG", &["png"])
        .save_file()
    else {
        return;
    };
    match render_png(layout, &RenderOptions::default(), &path) {
        Ok(()) => shared.set_message(
            MessageKind::Success,
            format!("Exported PNG to {}", path.display()),
        ),
        Err(error) => shared.set_message(MessageKind::Error, format!("PNG export failed: {error}")),
    }
}
