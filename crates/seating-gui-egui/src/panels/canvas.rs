//! Central-panel interactive seating canvas.
//!
//! Stage 4 owns this file exclusively (module ownership rule in the spec):
//! zoom/pan, seat rendering, hover tooltips, drag-and-drop via
//! [`apply_seat_drop`], score-delta toasts, and SVG/PNG export.

use crate::state::{ClosenessRow, MessageKind, SharedState};
use eframe::egui::{
    self, Align2, Color32, FontId, Galley, Id, Pos2, Rect, ScrollArea, Sense, Stroke, StrokeKind,
    UiBuilder, Vec2,
};
use seating_core::{
    COLOR_BACKGROUND, COLOR_CARD, COLOR_GUEST_TEXT, COLOR_MUTED, COLOR_SEAT_FILL,
    COLOR_SEAT_STROKE, COLOR_STROKE, COLOR_TABLE_FILL, COLOR_TABLE_STROKE, LayoutSeat, LayoutTable,
    Person, ProjectInput, RenderOptions, SeatDropOutcome, SeatingAssignment, SeatingLayout,
    TableSurface, apply_seat_drop, build_layout, compact_table_numbers, min_seat_spacing,
    render_png, render_svg, swap_table_numbers, unassign_person, unassigned_people,
};
use std::collections::HashMap;
use std::sync::Arc;

const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 3.0;
const TOAST_LIFETIME: f64 = 2.2;
/// Floor for the name-label wrap width, in the same layout units as
/// `LayoutSeat::x`/`y` (i.e. before zoom or the label font scale are
/// applied), so a table with tightly packed seats can't wrap a guest's name
/// down to single letters.
const NAME_WRAP_MIN_LAYOUT: f32 = 40.0;
/// Fixed height, in screen pixels, reserved for the "Unassigned" band at the
/// bottom of the canvas when at least one guest is unassigned. Unlike the
/// rest of the canvas, the band ignores zoom/pan.
const UNASSIGNED_BAND_HEIGHT: f32 = 120.0;
/// Font size for name chips in the unassigned band, in screen pixels
/// (unaffected by zoom, unlike seat labels).
const CHIP_FONT: f32 = 13.0;

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

    // Also gate on `generated_table_numbers`: a brand-new project with no
    // table types yet still builds a trivial empty layout (nothing to
    // render), and should keep showing `empty_state`'s hints rather than a
    // blank canvas.
    if shared.layout.is_some() && !shared.generated_table_numbers.is_empty() {
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
        if ui
            .checkbox(&mut shared.show_empty_tables, "Show empty tables")
            .changed()
        {
            shared.recompute();
        }
        if ui
            .add_enabled(
                shared.score_breakdown.is_some(),
                egui::Button::new("Compact tables"),
            )
            .clicked()
            && let Ok(project) = shared.materialize_project()
        {
            let (order, map) = compact_table_numbers(&project, &shared.assignments);
            shared.apply_table_number_map(&map);
            shared.table_order = order;
            shared.refresh();
            shared.set_message(MessageKind::Success, "Compacted table numbers");
        }
        ui.separator();
        // Export uses the strict `build_layout`, which needs every guest
        // seated — gate on `score_breakdown` (only `Some` for a strictly
        // valid, fully-seated solution), not just `layout` (which also
        // exists for a partial seating).
        ui.add_enabled_ui(shared.score_breakdown.is_some(), |ui| {
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

    // Unassigned guests get a fixed band at the bottom of the canvas
    // (screen space, unaffected by zoom/pan); its height is carved out of
    // the canvas's own drawing area up front so tables never end up hidden
    // underneath it, and collapses to zero once everyone is seated.
    let unassigned = unassigned_people(&shared.people, &shared.assignments);
    let band_height = if unassigned.is_empty() {
        0.0
    } else {
        UNASSIGNED_BAND_HEIGHT
    };

    let desired_size = ui.available_size();
    let canvas_size = Vec2::new(desired_size.x, (desired_size.y - band_height).max(0.0));
    let (rect, _) = ui.allocate_exact_size(canvas_size, Sense::hover());
    let band_rect = if band_height > 0.0 {
        Some(
            ui.allocate_exact_size(Vec2::new(desired_size.x, band_height), Sense::hover())
                .0,
        )
    } else {
        None
    };
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

    painter.rect_filled(rect, 0.0, rgb(COLOR_BACKGROUND));

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
    let mut pending_unassign = false;
    let mut pending_lock: Option<(String, Option<usize>, Option<usize>)> = None;
    let mut pending_swap: Option<(usize, usize)> = None;

    // Guest-name labels are collected here and painted once after every
    // table/seat has been drawn, so a later seat, its drop-highlight disc,
    // or the next table's card never paints over an earlier seat's label.
    let mut pending_labels: Vec<(Pos2, Align2, Arc<Galley>, Color32)> = Vec::new();

    // The label font size follows zoom down to a floor of 7.0; scale the
    // wrap width by the same ratio (rather than raw zoom) so chars-per-line
    // stays constant even once the font itself has floored out.
    let name_font = (11.0 * state.zoom).max(7.0);
    let name_font_scale = name_font / 11.0;

    for table in &layout.tables {
        draw_table(&painter, table, transform, state.zoom);

        // Wrap width for names radiating outward from each seat: the
        // smallest center-to-center distance between two seats at this
        // table (or the layout-unit floor for a lone-seat table), scaled
        // by the label font's own zoom ratio.
        let spacing = min_seat_spacing(&table.seats).unwrap_or(NAME_WRAP_MIN_LAYOUT);
        let name_wrap_width = spacing.max(NAME_WRAP_MIN_LAYOUT) * name_font_scale;
        let surface_center = surface_center_screen(&table.surface, transform);

        let card_rect = Rect::from_two_pos(
            transform.to_screen((table.x, table.y)),
            transform.to_screen((table.x + table.width, table.y + table.height)),
        );
        let table_id = Id::new(("table_surface", table.table_number));
        let table_response = ui.interact(card_rect, table_id, Sense::click());
        if state.drag.is_none() {
            table_response.context_menu(|ui| {
                table_swap_menu(ui, &layout, table.table_number, &mut pending_swap);
            });
        }

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

            if let Some(name) = seat.person_name.as_deref() {
                let name_color = faded(rgb(COLOR_GUEST_TEXT), seat_alpha(is_being_dragged));
                let galley = painter.layout(
                    name.to_string(),
                    FontId::proportional(name_font),
                    name_color,
                    name_wrap_width,
                );
                let (anchor, align) =
                    label_anchor(center, surface_center, radius, 2.0 * state.zoom);
                pending_labels.push((anchor, align, galley, name_color));
            }

            let hit_rect = Rect::from_center_size(center, Vec2::splat((radius * 2.0).max(20.0)));
            if draggable {
                let seat_id = Id::new(("seat_drag", table.table_number, seat.seat_index));
                let mut seat_response = ui.interact(hit_rect, seat_id, Sense::click_and_drag());

                if state.drag.is_none()
                    && let Some(person) = person
                {
                    seat_response =
                        seat_response.on_hover_ui(|ui| person_tooltip(ui, person, shared));
                    seat_response.context_menu(|ui| {
                        seat_lock_menu(
                            ui,
                            person,
                            table.table_number,
                            seat.seat_index,
                            &mut pending_lock,
                        );
                    });
                }
                if seat_response.drag_started()
                    && state.drag.is_none()
                    && let (Some(assignment), Some(person)) = (
                        assignment_by_seat.get(&(table.table_number, seat.seat_index)),
                        person,
                    )
                    && let Ok(project) = shared.materialize_project()
                {
                    state.drag = Some(DragState {
                        person_id: assignment.person_id.clone(),
                        person_name: person.name.clone(),
                        project,
                        allowed_table: person.locked_table,
                    });
                }
                if seat_response.drag_stopped() {
                    if let Some(target) = pointer_pos.and_then(|pointer| {
                        find_seat_under(&layout, transform, pointer, hit_radius)
                    }) {
                        pending_drop = Some(target);
                    } else if pointer_pos.is_some_and(|pointer| {
                        band_rect.is_some_and(|band_rect| band_rect.contains(pointer))
                    }) {
                        pending_unassign = true;
                    } else {
                        drag_cancelled = true;
                    }
                }
            } else if let Some(person) = person {
                let seat_id = Id::new(("seat_hover", table.table_number, seat.seat_index));
                let seat_response = ui.interact(hit_rect, seat_id, Sense::click());
                if state.drag.is_none() {
                    seat_response.context_menu(|ui| {
                        seat_lock_menu(
                            ui,
                            person,
                            table.table_number,
                            seat.seat_index,
                            &mut pending_lock,
                        );
                    });
                    seat_response.on_hover_ui(|ui| person_tooltip(ui, person, shared));
                }
            }
        }
    }

    for (anchor, align, galley, color) in pending_labels {
        let pos = align.anchor_size(anchor, galley.size()).min;
        painter.galley(pos, galley, color);
    }

    if let (Some(drag), Some(pointer)) = (&state.drag, pointer_pos) {
        draw_ghost(&painter, pointer, &drag.person_name);
    }

    draw_toast(&painter, rect, state, ui.ctx());

    if let Some(band_rect) = band_rect {
        let (chip_drop, chip_cancelled) = draw_unassigned_band(
            ui,
            band_rect,
            shared,
            state,
            &layout,
            transform,
            pointer_pos,
            hit_radius,
            &unassigned,
        );
        pending_drop = pending_drop.or(chip_drop);
        drag_cancelled = drag_cancelled || chip_cancelled;
    }

    // Every borrow of `shared` taken above (person_by_id, assignment_by_seat,
    // unassigned) is dead by now, so mutating it here is safe.
    if let Some((table_number, seat_index)) = pending_drop {
        finish_drop(shared, state, table_number, seat_index, ui.ctx());
    } else if pending_unassign {
        finish_unassign(shared, state, ui.ctx());
    } else if drag_cancelled {
        state.drag = None;
        shared.set_message(
            MessageKind::Info,
            "Drop cancelled — released outside a seat.",
        );
    } else if let Some((person_id, locked_table, locked_seat)) = pending_lock {
        finish_lock(shared, &person_id, locked_table, locked_seat);
    } else if let Some((a, b)) = pending_swap {
        finish_swap(shared, a, b);
    }
}

/// Submenu contents for right-clicking a table's surface: swap its number
/// (and with it, its whole occupant set) with another table currently drawn
/// on the canvas.
fn table_swap_menu(
    ui: &mut egui::Ui,
    layout: &SeatingLayout,
    table_number: usize,
    pending_swap: &mut Option<(usize, usize)>,
) {
    ui.menu_button("Swap with", |ui| {
        for other in &layout.tables {
            if other.table_number == table_number {
                continue;
            }
            if ui
                .button(format!(
                    "Table {} — {}",
                    other.table_number, other.table_type
                ))
                .clicked()
            {
                *pending_swap = Some((table_number, other.table_number));
                ui.close();
            }
        }
    });
}

/// Applies a right-click "Swap with" selection: renumbers tables `a` and `b`
/// (whole occupant set and lock travel with the number) via
/// [`swap_table_numbers`], then re-validates and re-scores.
fn finish_swap(shared: &mut SharedState, a: usize, b: usize) {
    let Ok(project) = shared.materialize_project() else {
        return;
    };
    let Some((order, map)) = swap_table_numbers(&project, a, b) else {
        return;
    };
    shared.apply_table_number_map(&map);
    shared.table_order = order;
    shared.refresh();
    shared.set_message(
        MessageKind::Success,
        format!("Swapped table {a} and table {b}."),
    );
}

/// Menu contents for right-clicking an occupied seat: lock the guest to
/// their current table/seat, to just the table, or unlock them — offering
/// only the options that would actually change their current lock state.
fn seat_lock_menu(
    ui: &mut egui::Ui,
    person: &Person,
    table_number: usize,
    seat_index: usize,
    pending_lock: &mut Option<(String, Option<usize>, Option<usize>)>,
) {
    if (person.locked_table != Some(table_number) || person.locked_seat != Some(seat_index))
        && ui
            .button(format!("Lock to table {table_number}, seat {seat_index}"))
            .clicked()
    {
        *pending_lock = Some((person.id.clone(), Some(table_number), Some(seat_index)));
        ui.close();
    }
    if (person.locked_table != Some(table_number) || person.locked_seat.is_some())
        && ui.button(format!("Lock to table {table_number}")).clicked()
    {
        *pending_lock = Some((person.id.clone(), Some(table_number), None));
        ui.close();
    }
    if (person.locked_table.is_some() || person.locked_seat.is_some())
        && ui.button("Unlock").clicked()
    {
        *pending_lock = Some((person.id.clone(), None, None));
        ui.close();
    }
}

/// Applies a lock/unlock chosen from [`seat_lock_menu`] to the guest's
/// `locked_table`/`locked_seat` fields, then re-validates and re-scores.
fn finish_lock(
    shared: &mut SharedState,
    person_id: &str,
    locked_table: Option<usize>,
    locked_seat: Option<usize>,
) {
    let Some(person) = shared.people.iter_mut().find(|p| p.id == person_id) else {
        return;
    };
    person.locked_table = locked_table;
    person.locked_seat = locked_seat;
    let name = person.name.clone();
    shared.refresh();
    let message = match (locked_table, locked_seat) {
        (Some(t), Some(s)) => format!("Locked {name} to table {t}, seat {s}."),
        (Some(t), None) => format!("Locked {name} to table {t}."),
        _ => format!("Unlocked {name}."),
    };
    shared.set_message(MessageKind::Success, message);
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

/// Applies dropping a seated guest onto the "Unassigned" band: sends them
/// back to unassigned via [`unassign_person`] (which refuses a locked
/// guest), then re-validates and re-scores like [`finish_drop`].
fn finish_unassign(shared: &mut SharedState, state: &mut CanvasState, ctx: &egui::Context) {
    let Some(drag) = state.drag.take() else {
        return;
    };
    let score_before = shared.score;
    match unassign_person(&drag.project, &shared.assignments, &drag.person_id) {
        Ok(updated) => {
            shared.assignments = updated;
            shared.refresh();
            if let (Some(before), Some(after)) = (score_before, shared.score) {
                state.toast = Some(ScoreToast {
                    delta: after - before,
                    spawned_at: ctx.input(|i| i.time),
                });
                state.suppress_diff_toast = true;
            }
            shared.set_message(
                MessageKind::Success,
                format!("Unassigned {}.", drag.person_name),
            );
        }
        Err(report) => {
            shared.set_message(
                MessageKind::Error,
                format!("Unassign failed: {}", SharedState::report_summary(&report)),
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

/// Screen-space center of a table's surface, for every [`TableSurface`]
/// variant — the point guest-name labels radiate outward from.
fn surface_center_screen(surface: &TableSurface, transform: Transform) -> Pos2 {
    let center = match surface {
        TableSurface::Round { cx, cy, .. } | TableSurface::Semicircle { cx, cy, .. } => (*cx, *cy),
        TableSurface::Rect {
            x,
            y,
            width,
            height,
        } => (x + width / 2.0, y + height / 2.0),
    };
    transform.to_screen(center)
}

/// Anchor point and alignment for a guest's name label, placed just outside
/// the seat circle in the direction pointing away from the table's surface
/// center. Radiating labels outward (rather than always dropping them
/// straight down) keeps a ring table's labels spread apart like its seats,
/// and keeps a semicircle's top-row labels off the table surface.
///
/// The offset moves along the dominant axis of that direction only (not
/// diagonally): a diagonal seat still gets the full `radius + gap` of
/// clearance on the axis that matters, instead of splitting it between both
/// axes and landing the label closer to the seat circle (and its lock icon)
/// than intended.
fn label_anchor(seat_center: Pos2, surface_center: Pos2, radius: f32, gap: f32) -> (Pos2, Align2) {
    let dir = (seat_center - surface_center).normalized();
    let offset = radius + gap;
    if dir.x.abs() > dir.y.abs() {
        let align = if dir.x >= 0.0 {
            Align2::LEFT_CENTER
        } else {
            Align2::RIGHT_CENTER
        };
        (seat_center + Vec2::new(dir.x.signum() * offset, 0.0), align)
    } else {
        let align = if dir.y >= 0.0 {
            Align2::CENTER_TOP
        } else {
            Align2::CENTER_BOTTOM
        };
        (seat_center + Vec2::new(0.0, dir.y.signum() * offset), align)
    }
}

/// Alpha multiplier for a seat's fill/text while it's mid-drag (dimmed) vs.
/// at rest.
fn seat_alpha(dimmed: bool) -> f32 {
    if dimmed { 0.35 } else { 1.0 }
}

fn draw_table(painter: &egui::Painter, table: &LayoutTable, transform: Transform, zoom: f32) {
    let card_rect = Rect::from_two_pos(
        transform.to_screen((table.x, table.y)),
        transform.to_screen((table.x + table.width, table.y + table.height)),
    );
    painter.rect_filled(card_rect, 12.0 * zoom, rgb(COLOR_CARD));
    painter.rect_stroke(
        card_rect,
        12.0 * zoom,
        Stroke::new(1.2_f32, rgb(COLOR_STROKE)),
        StrokeKind::Middle,
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
                8.0 * zoom,
                surface_fill,
                surface_stroke,
                StrokeKind::Middle,
            );
        }
        TableSurface::Semicircle { cx, cy, radius } => {
            let center = transform.to_screen((*cx, *cy));
            let screen_radius = radius * zoom;
            const ARC_POINTS: usize = 32;
            let points: Vec<Pos2> = (0..=ARC_POINTS)
                .map(|i| {
                    let angle =
                        std::f32::consts::PI + std::f32::consts::PI * i as f32 / ARC_POINTS as f32;
                    Pos2::new(
                        center.x + screen_radius * angle.cos(),
                        center.y + screen_radius * angle.sin(),
                    )
                })
                .collect();
            painter.add(egui::Shape::convex_polygon(
                points,
                surface_fill,
                surface_stroke,
            ));
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
    let alpha = seat_alpha(dimmed);
    if occupied {
        painter.circle_filled(center, radius, faded(rgb(COLOR_SEAT_FILL), alpha));
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(1.3_f32, faded(rgb(COLOR_SEAT_STROKE), alpha)),
        );
    } else {
        painter.circle_stroke(center, radius, Stroke::new(1.3_f32, rgb(COLOR_STROKE)));
    }

    // The circle only ever holds the seat index now — it stays small and
    // never overflows. The guest's full name is drawn wrapped outward from
    // the seat instead of being squeezed (and truncated) inside it (see
    // `pending_labels` in `canvas_area`, which paints names after every
    // seat has been drawn so later seats can't cover earlier labels).
    let box_side = radius * 1.5;
    let base_font = 10.0 * zoom;
    let min_font = 5.0 * zoom;
    let text_color = if occupied {
        faded(rgb(COLOR_BACKGROUND), alpha)
    } else {
        rgb(COLOR_MUTED)
    };
    let index_text = seat.seat_index.to_string();
    let index_size = painter
        .layout_no_wrap(
            index_text.clone(),
            FontId::proportional(base_font),
            Color32::PLACEHOLDER,
        )
        .size();
    let index_font = (base_font * fit_scale(index_size, box_side)).max(min_font);
    painter.text(
        center,
        Align2::CENTER_CENTER,
        index_text,
        FontId::proportional(index_font),
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

/// Draws the fixed "Unassigned (n)" band and each unassigned guest's name
/// chip, and handles starting/ending a drag from a chip. Chips reuse the
/// canvas's own `DragState`/ghost/drop-preview machinery — `apply_seat_drop`
/// already treats an unassigned mover the same as a seated one, so a chip
/// dropped onto a seat needs no separate code path here; only *starting* a
/// drag from a chip (rather than a seat) is band-specific.
///
/// Returns the seat a released chip landed on (if any) and whether a chip
/// drag ended outside both a seat and the band (a cancel, same as a seat
/// drag released outside every seat).
#[allow(clippy::too_many_arguments)]
fn draw_unassigned_band(
    ui: &mut egui::Ui,
    band_rect: Rect,
    shared: &SharedState,
    state: &mut CanvasState,
    layout: &SeatingLayout,
    transform: Transform,
    pointer_pos: Option<Pos2>,
    hit_radius: f32,
    unassigned: &[&Person],
) -> (Option<(usize, usize)>, bool) {
    let painter = ui.painter_at(band_rect);
    painter.rect_filled(band_rect, 8.0, rgb(COLOR_CARD));
    painter.rect_stroke(
        band_rect,
        8.0,
        Stroke::new(1.0_f32, rgb(COLOR_STROKE)),
        StrokeKind::Middle,
    );
    painter.text(
        band_rect.min + Vec2::new(12.0, 8.0),
        Align2::LEFT_TOP,
        format!("Unassigned ({})", unassigned.len()),
        FontId::proportional(CHIP_FONT),
        rgb(COLOR_GUEST_TEXT),
    );

    let chips_rect = Rect::from_min_max(
        band_rect.min + Vec2::new(8.0, 30.0),
        band_rect.max - Vec2::new(8.0, 8.0),
    );

    let mut pending_drop = None;
    let mut drag_cancelled = false;

    ui.scope_builder(UiBuilder::new().max_rect(chips_rect), |ui| {
        ScrollArea::vertical()
            .id_salt("unassigned_band_scroll")
            .max_height(chips_rect.height().max(0.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for person in unassigned {
                        let galley = ui.painter().layout_no_wrap(
                            person.name.clone(),
                            FontId::proportional(CHIP_FONT),
                            rgb(COLOR_BACKGROUND),
                        );
                        let chip_size = galley.size() + Vec2::new(16.0, 10.0);
                        let (chip_rect, response) =
                            ui.allocate_exact_size(chip_size, Sense::click_and_drag());

                        let is_being_dragged = state
                            .drag
                            .as_ref()
                            .is_some_and(|drag| drag.person_id == person.id);
                        let alpha = seat_alpha(is_being_dragged);
                        ui.painter().rect_filled(
                            chip_rect,
                            10.0,
                            faded(rgb(COLOR_SEAT_FILL), alpha),
                        );
                        let text_pos = Align2::CENTER_CENTER
                            .anchor_size(chip_rect.center(), galley.size())
                            .min;
                        ui.painter()
                            .galley(text_pos, galley, faded(rgb(COLOR_BACKGROUND), alpha));

                        if response.drag_started()
                            && state.drag.is_none()
                            && let Ok(project) = shared.materialize_project()
                        {
                            state.drag = Some(DragState {
                                person_id: person.id.clone(),
                                person_name: person.name.clone(),
                                project,
                                allowed_table: person.locked_table,
                            });
                        }
                        if response.drag_stopped() {
                            match pointer_pos.and_then(|pointer| {
                                find_seat_under(layout, transform, pointer, hit_radius)
                            }) {
                                Some(target) => pending_drop = Some(target),
                                None => drag_cancelled = true,
                            }
                        }
                        if state.drag.is_none() {
                            response.on_hover_ui(|ui| person_tooltip(ui, person, shared));
                        }
                    }
                });
            });
    });

    (pending_drop, drag_cancelled)
}

fn draw_ghost(painter: &egui::Painter, pointer: Pos2, person_name: &str) {
    let color = rgb(COLOR_GUEST_TEXT);
    let galley = painter.layout_no_wrap(person_name.to_string(), FontId::proportional(12.0), color);
    let rect = Rect::from_min_size(
        pointer + Vec2::new(14.0, 14.0),
        galley.size() + Vec2::new(16.0, 12.0),
    );
    let (r, g, b) = COLOR_CARD;
    painter.rect_filled(rect, 6.0, Color32::from_rgba_unmultiplied(r, g, b, 235));
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0_f32, rgb(COLOR_TABLE_STROKE)),
        StrokeKind::Middle,
    );
    let text_pos = Align2::CENTER_CENTER
        .anchor_size(rect.center(), galley.size())
        .min;
    painter.galley(text_pos, galley, color);
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

/// Font scale that shrinks `size` (a laid-out label's width/height) to fit
/// inside a `box_side` x `box_side` square, without ever growing it.
fn fit_scale(size: Vec2, box_side: f32) -> f32 {
    (box_side / size.x).min(box_side / size.y).min(1.0)
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
                rule_endpoint_label(&rule.left_id, shared),
                rule_endpoint_label(&rule.right_id, shared),
                rule.score_input
            ));
        }
    }
}

fn involves(id: &str, person: &Person) -> bool {
    id == person.id || person.groups.iter().any(|group| group == id)
}

/// A closeness rule endpoint is either a person id or a group id — show the
/// person's name when it resolves to one, otherwise the raw id (a group has
/// no separate display name).
fn rule_endpoint_label<'a>(id: &'a str, shared: &'a SharedState) -> &'a str {
    shared
        .people
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.name.as_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(id)
}

/// Rebuilds a layout without empty tables for export, regardless of the
/// canvas's "Show empty tables" toggle — exported plans should only depict
/// tables that actually seat someone.
fn export_layout(shared: &SharedState) -> Option<SeatingLayout> {
    let project = shared.materialize_project().ok()?;
    build_layout(&project, &shared.assignments).ok()
}

fn export_svg(shared: &mut SharedState) {
    let Some(layout) = export_layout(shared) else {
        return;
    };
    let svg = render_svg(&layout, &RenderOptions::default());
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
    let Some(layout) = export_layout(shared) else {
        return;
    };
    let Some(path) = rfd::FileDialog::new()
        .set_file_name("seating.png")
        .add_filter("PNG", &["png"])
        .save_file()
    else {
        return;
    };
    match render_png(&layout, &RenderOptions::default(), &path) {
        Ok(()) => shared.set_message(
            MessageKind::Success,
            format!("Exported PNG to {}", path.display()),
        ),
        Err(error) => shared.set_message(MessageKind::Error, format!("PNG export failed: {error}")),
    }
}
