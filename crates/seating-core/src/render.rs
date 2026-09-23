//! Reusable seating-plan layout and rendering.
//!
//! The GUI and CLI both use this module to turn a validated seating solution
//! into a layout description and exportable SVG/PNG outputs.

use crate::models::{ProjectInput, SeatingAssignment, TableShape, ValidationReport};
use crate::validation::{
    generate_table_instances, validate_partial_seating_solution, validate_seating_solution,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Geometry and spacing options for layout/rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// Outer margin around the full plan.
    pub margin: f32,
    /// Horizontal gap between tables.
    pub column_gap: f32,
    /// Vertical gap between tables.
    pub row_gap: f32,
    /// Width of each table card.
    pub table_width: f32,
    /// Height of each table card.
    pub table_height: f32,
    /// Radius of each rendered seat marker.
    pub seat_radius: f32,
    /// Base font size for labels.
    pub font_size: f32,
    /// PNG rasterization scale factor (1.0 = CSS pixel size).
    pub png_scale: f32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            margin: 24.0,
            column_gap: 36.0,
            row_gap: 36.0,
            table_width: 240.0,
            table_height: 220.0,
            seat_radius: 13.0,
            font_size: 14.0,
            png_scale: 2.0,
        }
    }
}

/// Navy color palette shared by SVG/PNG rendering and the GUI canvas, so both
/// draw the same plan in the same colors. RGB components, `0..=255`.
pub const COLOR_BACKGROUND: (u8, u8, u8) = (0x10, 0x15, 0x1c);
/// Table card fill.
pub const COLOR_CARD: (u8, u8, u8) = (0x17, 0x21, 0x2b);
/// Card and unoccupied-seat stroke.
pub const COLOR_STROKE: (u8, u8, u8) = (0x3f, 0x53, 0x68);
/// Table surface fill.
pub const COLOR_TABLE_FILL: (u8, u8, u8) = (0x35, 0x50, 0x70);
/// Table surface stroke.
pub const COLOR_TABLE_STROKE: (u8, u8, u8) = (0x90, 0xe0, 0xef);
/// Occupied-seat fill; also the default label text color.
pub const COLOR_SEAT_FILL: (u8, u8, u8) = (0xf5, 0xf7, 0xfa);
/// Occupied-seat stroke.
pub const COLOR_SEAT_STROKE: (u8, u8, u8) = (0x5c, 0x67, 0x73);
/// Guest-name label text.
pub const COLOR_GUEST_TEXT: (u8, u8, u8) = (0xdb, 0xe7, 0xff);
/// Muted/secondary text.
pub const COLOR_MUTED: (u8, u8, u8) = (0xb8, 0xc1, 0xcc);

/// Format an RGB tuple as a `#rrggbb` SVG color.
fn hex(color: (u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2)
}

/// A computed seating-plan layout.
#[derive(Debug, Clone, PartialEq)]
pub struct SeatingLayout {
    /// Total SVG/canvas width.
    pub width: f32,
    /// Total SVG/canvas height.
    pub height: f32,
    /// Tables in reading order.
    pub tables: Vec<LayoutTable>,
}

/// One rendered table within a [`SeatingLayout`].
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutTable {
    /// Table number.
    pub table_number: usize,
    /// Table type identifier.
    pub table_type: String,
    /// Table shape.
    pub shape: TableShape,
    /// Top-left X coordinate of the card.
    pub x: f32,
    /// Top-left Y coordinate of the card.
    pub y: f32,
    /// Card width.
    pub width: f32,
    /// Card height.
    pub height: f32,
    /// Concrete seat positions around the table, one per capacity slot.
    pub seats: Vec<LayoutSeat>,
    /// Geometry of the table surface itself, computed from the same center
    /// used to place `seats` so renderers never re-derive divergent geometry.
    pub surface: TableSurface,
}

/// Table-surface geometry for one [`LayoutTable`], computed once in
/// [`build_layout`] and consumed by both `render_svg` and any UI drawing the
/// same layout, so the surface and its seats never drift apart.
#[derive(Debug, Clone, PartialEq)]
pub enum TableSurface {
    /// Round table: center and radius. `cx`/`cy` are the same center used to
    /// place the table's seats.
    Round { cx: f32, cy: f32, radius: f32 },
    /// Rectangular/square table: an inset surface rect.
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Semicircle table: flat edge on `y = cy`, arc above (seats sit at `y < cy`).
    Semicircle { cx: f32, cy: f32, radius: f32 },
}

/// One rendered seat marker within a table layout.
///
/// Every capacity slot for the table is represented, not just occupied ones,
/// so the rendered geometry always matches the table's real seat count.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSeat {
    /// Zero-based seat index.
    pub seat_index: usize,
    /// Seat-center X coordinate.
    pub x: f32,
    /// Seat-center Y coordinate.
    pub y: f32,
    /// Occupant name, or `None` when this capacity slot is unassigned.
    pub person_name: Option<String>,
}

/// Rendering/export error.
#[derive(Debug, thiserror::Error)]
pub enum RenderingError {
    #[error("failed to parse generated SVG: {0}")]
    SvgParse(String),
    #[error("failed to allocate PNG buffer sized {width}x{height}")]
    PixmapAllocation { width: u32, height: u32 },
    #[error("failed to write PNG file: {0}")]
    WritePng(String),
}

/// Build a reusable layout from a validated project and seating assignment.
///
/// Every person must be seated exactly once (see [`validate_seating_solution`]);
/// use [`build_editor_layout`] for a GUI editor where guests may still be
/// unassigned. Tables with no occupants are omitted.
pub fn build_layout(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
) -> Result<SeatingLayout, ValidationReport> {
    build_layout_impl(project, assignments, false, true)
}

/// Build a reusable layout like [`build_layout`], but tolerating a *partial*
/// seating (some guests not yet assigned — see
/// [`validate_partial_seating_solution`]), and optionally including tables
/// with no occupants (e.g. a table just added via the GUI), so the canvas
/// can still render and hit-test drops onto them.
///
/// With `include_empty_tables: true`, at most one empty table per type is
/// shown (the lowest-numbered one), regardless of how many that type
/// actually has sitting empty — a *limited* type materializes every one of
/// its `number_of_tables` instances up front (see
/// [`generate_table_instances`]), so without this cap they would all show
/// up as guests are moved out of them. An empty table some guest is
/// `locked_table`ed to is always shown regardless of that cap (and
/// regardless of `include_empty_tables`), so an unseated locked guest
/// always has their table rendered as a drop target.
pub fn build_editor_layout(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    include_empty_tables: bool,
) -> Result<SeatingLayout, ValidationReport> {
    build_layout_impl(project, assignments, include_empty_tables, false)
}

fn build_layout_impl(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    include_empty_tables: bool,
    require_all_people: bool,
) -> Result<SeatingLayout, ValidationReport> {
    if require_all_people {
        validate_seating_solution(project, assignments)?;
    } else {
        validate_partial_seating_solution(project, assignments)?;
    }

    let options = RenderOptions::default();
    let instances = generate_table_instances(project);
    let mut assignments_by_table: HashMap<usize, Vec<&SeatingAssignment>> = HashMap::new();
    for assignment in assignments {
        assignments_by_table
            .entry(assignment.table_number)
            .or_default()
            .push(assignment);
    }
    for table_assignments in assignments_by_table.values_mut() {
        table_assignments.sort_by_key(|assignment| assignment.seat_index);
    }

    // When showing empty tables, cap them at one per type: a *limited* type
    // always has every one of its `number_of_tables` instances materialized
    // from the start (see `generate_table_instances`), and even an
    // *unlimited* type's derived floor (`ceil(person_count / max_people)`)
    // can leave more than one spare once guests move out — `ensure_spare_tables`
    // only tops a type back up when it runs dry, it doesn't cap it — so
    // without this filter every empty instance would render.
    let mut lowest_empty_by_type: HashMap<&str, usize> = HashMap::new();
    for table in &instances {
        if !assignments_by_table.contains_key(&table.number) {
            lowest_empty_by_type
                .entry(table.table_type.as_str())
                .and_modify(|lowest| *lowest = (*lowest).min(table.number))
                .or_insert(table.number);
        }
    }
    // A table some guest is locked to is always shown, even if empty and
    // not the lowest-numbered spare of its type — otherwise an unseated
    // locked guest would have no valid drop target rendered at all. Mirrors
    // the same exception in `compact_table_numbers`.
    let locked_numbers: HashSet<usize> = project
        .people
        .iter()
        .filter_map(|person| person.locked_table)
        .collect();
    let used_instances = instances
        .iter()
        .filter(|table| {
            assignments_by_table.contains_key(&table.number)
                || locked_numbers.contains(&table.number)
                || (include_empty_tables
                    && lowest_empty_by_type.get(table.table_type.as_str()) == Some(&table.number))
        })
        .collect::<Vec<_>>();
    let columns = columns_for(used_instances.len());
    let mut tables = Vec::new();

    for (index, table) in used_instances.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let x = options.margin + column as f32 * (options.table_width + options.column_gap);
        let y = options.margin + row as f32 * (options.table_height + options.row_gap);
        let config = &project.table_types[&table.table_type];
        let table_assignments = assignments_by_table
            .get(&table.number)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let seats = build_seat_positions(
            table.shape.clone(),
            config.people_per_side.as_deref(),
            table.max_people,
            x,
            y,
            &options,
            table_assignments,
        );
        let surface = build_surface(&table.shape, x, y, &options);
        tables.push(LayoutTable {
            table_number: table.number,
            table_type: table.table_type.clone(),
            shape: table.shape.clone(),
            x,
            y,
            width: options.table_width,
            height: options.table_height,
            seats,
            surface,
        });
    }

    let rows = if tables.is_empty() {
        0
    } else {
        (tables.len() - 1) / columns + 1
    };
    let width = if columns == 0 {
        options.margin * 2.0
    } else {
        options.margin * 2.0
            + columns as f32 * options.table_width
            + columns.saturating_sub(1) as f32 * options.column_gap
    };
    let height = if rows == 0 {
        options.margin * 2.0
    } else {
        options.margin * 2.0
            + rows as f32 * options.table_height
            + rows.saturating_sub(1) as f32 * options.row_gap
    };

    Ok(SeatingLayout {
        width,
        height,
        tables,
    })
}

/// Render a layout as standalone SVG markup.
///
/// The document's height grows past `layout.height` when a wrapped guest
/// label's lowest line would otherwise be clipped by the image bottom (e.g.
/// a long name at a seat in the last row); the header/footer geometry is
/// unaffected, only the canvas the image is drawn on.
pub fn render_svg(layout: &SeatingLayout, options: &RenderOptions) -> String {
    let mut body = String::new();
    let mut max_label_bottom: f32 = 0.0;
    body.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        hex(COLOR_BACKGROUND)
    ));
    body.push_str(&format!(
        "<style>text {{ fill: {}; font-family: Arial, Helvetica, sans-serif; font-size: {}px; }} .muted {{ fill: {}; }} .seat-index {{ fill: {}; font-size: {}px; font-weight: bold; }} .guest {{ fill: {}; font-size: {}px; }}</style>",
        hex(COLOR_SEAT_FILL),
        options.font_size,
        hex(COLOR_MUTED),
        hex(COLOR_BACKGROUND),
        options.font_size - 3.0,
        hex(COLOR_GUEST_TEXT),
        options.font_size - 1.0
    ));

    let label_font_size = options.font_size - 1.0;
    let label_char_width = (label_font_size * 0.55).max(1.0);
    // Words still wrap on word boundaries in the common case; without this
    // floor, a tight seat spacing (e.g. rectangular-table corners) could
    // shrink the budget to a couple of characters per line.
    let min_label_budget = 10.0 * label_char_width;
    let label_line_height = label_font_size * 1.2;

    for table in &layout.tables {
        let label_x = table.x + table.width / 2.0;
        let (title_y, subtitle_y) = header_positions(table.y, options);
        body.push_str(&format!(
            "<g><rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"18\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            table.x, table.y, table.width, table.height, hex(COLOR_CARD), hex(COLOR_STROKE)
        ));
        body.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">Table {} — {}</text>",
            label_x,
            title_y,
            table.table_number,
            escape_xml(&table.table_type)
        ));
        body.push_str(&format!(
            "<text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">Shape: {}</text>",
            label_x,
            subtitle_y,
            shape_label(&table.shape)
        ));

        match &table.surface {
            TableSurface::Round { cx, cy, radius } => {
                body.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx, cy, radius, hex(COLOR_TABLE_FILL), hex(COLOR_TABLE_STROKE)
                ));
            }
            TableSurface::Rect {
                x,
                y,
                width,
                height,
            } => body.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                x, y, width, height, hex(COLOR_TABLE_FILL), hex(COLOR_TABLE_STROKE)
            )),
            TableSurface::Semicircle { cx, cy, radius } => body.push_str(&format!(
                "<path d=\"M{:.1},{:.1} A{:.1},{:.1} 0 0 1 {:.1},{:.1} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx - radius,
                cy,
                radius,
                radius,
                cx + radius,
                cy,
                hex(COLOR_TABLE_FILL),
                hex(COLOR_TABLE_STROKE)
            )),
        }

        let label_budget = min_seat_spacing(&table.seats)
            .unwrap_or(table.width - 40.0)
            .max(min_label_budget);

        for seat in &table.seats {
            body.push_str("<g>");
            if let Some(name) = seat.person_name.as_deref() {
                body.push_str(&format!("<title>{}</title>", escape_xml(name)));
            }
            if let Some(person_name) = seat.person_name.as_deref() {
                body.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    seat.x, seat.y, options.seat_radius, hex(COLOR_SEAT_FILL), hex(COLOR_SEAT_STROKE)
                ));
                body.push_str(&format!(
                    "<text class=\"seat-index\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>",
                    seat.x,
                    seat.y + 0.5,
                    seat.seat_index
                ));
                let lines = wrap_label(person_name, label_budget, label_font_size);
                let label_y = seat.y + options.seat_radius + 16.0;
                body.push_str(&format!(
                    "<text class=\"guest\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">",
                    seat.x, label_y
                ));
                for (line_index, line) in lines.iter().enumerate() {
                    let dy = if line_index == 0 {
                        0.0
                    } else {
                        label_line_height
                    };
                    body.push_str(&format!(
                        "<tspan x=\"{:.1}\" dy=\"{:.1}\">{}</tspan>",
                        seat.x,
                        dy,
                        escape_xml(line)
                    ));
                }
                body.push_str("</text>");
                let label_bottom = label_y
                    + (lines.len().saturating_sub(1)) as f32 * label_line_height
                    + label_font_size;
                max_label_bottom = max_label_bottom.max(label_bottom);
            } else {
                // Unoccupied capacity slot: hollow, dimmed marker.
                body.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"3,3\" opacity=\"0.6\"/>",
                    seat.x, seat.y, options.seat_radius, hex(COLOR_STROKE)
                ));
                body.push_str(&format!(
                    "<text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" dominant-baseline=\"middle\" font-size=\"{:.1}\">{}</text>",
                    seat.x,
                    seat.y + 0.5,
                    options.font_size - 3.0,
                    seat.seat_index
                ));
            }
            body.push_str("</g>");
        }
        body.push_str("</g>");
    }

    // Grow the canvas past `layout.height` when a wrapped label's lowest
    // line would otherwise be clipped by the image bottom.
    let height = layout.height.max(max_label_bottom + 8.0);

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}" viewBox="0 0 {:.0} {:.0}">"#,
        layout.width, height, layout.width, height
    ));
    svg.push_str(&body);
    svg.push_str("</svg>");
    svg
}

/// Render a layout to a PNG file by rasterizing the generated SVG.
pub fn render_png(
    layout: &SeatingLayout,
    options: &RenderOptions,
    path: impl AsRef<Path>,
) -> Result<(), RenderingError> {
    let svg = render_svg(layout, options);
    let mut svg_options = resvg::usvg::Options::default();
    svg_options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_str(&svg, &svg_options)
        .map_err(|error| RenderingError::SvgParse(error.to_string()))?;
    let size = tree.size().to_int_size();
    let scale = if options.png_scale.is_finite() && options.png_scale > 0.0 {
        options.png_scale
    } else {
        1.0
    };
    let scaled_width = ((size.width() as f32) * scale).round().max(1.0) as u32;
    let scaled_height = ((size.height() as f32) * scale).round().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(scaled_width, scaled_height).ok_or(
        RenderingError::PixmapAllocation {
            width: scaled_width,
            height: scaled_height,
        },
    )?;
    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap_mut,
    );
    pixmap
        .save_png(path)
        .map_err(|error| RenderingError::WritePng(error.to_string()))
}

fn columns_for(table_count: usize) -> usize {
    match table_count {
        0 => 0,
        1..=4 => table_count,
        _ => (table_count as f32).sqrt().ceil() as usize,
    }
}

/// Baseline Y coordinates of a table card's title and subtitle rows,
/// relative to the card's top-left `y`. Tied to `options.font_size` so a
/// larger label font pushes both rows (and anything reserved below them)
/// down accordingly.
fn header_positions(y: f32, options: &RenderOptions) -> (f32, f32) {
    let title_y = y + options.font_size + 10.0;
    let subtitle_y = title_y + options.font_size + 6.0;
    (title_y, subtitle_y)
}

/// Y coordinate below which seats may be drawn without overlapping the
/// title/subtitle header rows.
fn header_bottom(y: f32, options: &RenderOptions) -> f32 {
    header_positions(y, options).1 + 8.0
}

/// Center and ring radius for a round table's seats, sized from the card
/// dimensions and `seat_radius` so the ring fits inside the card and the
/// topmost seat clears the header rows.
fn round_table_metrics(x: f32, y: f32, options: &RenderOptions) -> (f32, f32, f32) {
    let top = header_bottom(y, options);
    let bottom = y + options.table_height - options.font_size - 6.0;
    let center_x = x + options.table_width / 2.0;
    let center_y = (top + bottom) / 2.0;
    let vertical_radius = ((bottom - top) / 2.0 - options.seat_radius).max(20.0);
    let horizontal_radius = (options.table_width / 2.0 - options.seat_radius - 20.0).max(20.0);
    let radius = vertical_radius.min(horizontal_radius);
    (center_x, center_y, radius)
}

/// Center and ring radius for a semicircle table's seats, sized to use the
/// card's full available height instead of half of it.
///
/// [`round_table_metrics`] centers its ring in the card so a full circle
/// fits, giving a semicircle (which only draws its upper arc) roughly half
/// the usable height. Centering the flat edge near the card's bottom
/// instead lets the arc span almost the full height below the header,
/// roughly doubling how many seats fit before adjacent markers overlap.
fn semicircle_table_metrics(x: f32, y: f32, options: &RenderOptions) -> (f32, f32, f32) {
    let top = header_bottom(y, options);
    let bottom = y + options.table_height - options.font_size - 6.0;
    let center_x = x + options.table_width / 2.0;
    let center_y = bottom;
    let vertical_radius = (bottom - top - options.seat_radius - 20.0).max(20.0);
    // No extra aesthetic buffer here (unlike `round_table_metrics`'s
    // `- 20.0`): the arc needs every available pixel of width to seat a
    // realistic 10-12 person table without overlapping markers, and the
    // `- seat_radius` term alone already keeps seats from spilling past the
    // card edge.
    let horizontal_radius = (options.table_width / 2.0 - options.seat_radius).max(20.0);
    let radius = vertical_radius.min(horizontal_radius);
    (center_x, center_y, radius)
}

/// Table-surface geometry for `shape`, using the same center/insets as the
/// corresponding `build_*_seats` function so the drawn surface never drifts
/// from the seat ring/perimeter.
fn build_surface(shape: &TableShape, x: f32, y: f32, options: &RenderOptions) -> TableSurface {
    match shape {
        TableShape::Round => {
            let (center_x, center_y, ring_radius) = round_table_metrics(x, y, options);
            // The table surface sits inside the seat ring, leaving room for
            // the seat markers themselves.
            let surface_radius = (ring_radius - options.seat_radius - 6.0).max(20.0);
            TableSurface::Round {
                cx: center_x,
                cy: center_y,
                radius: surface_radius,
            }
        }
        // Rectangular/square insets are fixed proportions of the default
        // 240x220 card rather than derived from RenderOptions; a
        // table_width/table_height far below the defaults can crowd seats
        // against the card edge.
        TableShape::Rectangular | TableShape::Square => TableSurface::Rect {
            x: x + 60.0,
            y: y + 72.0,
            width: options.table_width - 120.0,
            height: options.table_height - 110.0,
        },
        TableShape::Semicircle => {
            let (center_x, center_y, ring_radius) = semicircle_table_metrics(x, y, options);
            let surface_radius = (ring_radius - options.seat_radius - 6.0).max(20.0);
            TableSurface::Semicircle {
                cx: center_x,
                cy: center_y,
                radius: surface_radius,
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_seat_positions(
    shape: TableShape,
    people_per_side: Option<&[usize]>,
    max_people: usize,
    x: f32,
    y: f32,
    options: &RenderOptions,
    table_assignments: &[&SeatingAssignment],
) -> Vec<LayoutSeat> {
    match shape {
        TableShape::Round => build_round_seats(max_people, table_assignments, x, y, options),
        TableShape::Rectangular | TableShape::Square => build_rectangular_seats(
            max_people,
            people_per_side.unwrap_or(&[]),
            table_assignments,
            x,
            y,
            options,
        ),
        TableShape::Semicircle => {
            build_semicircle_seats(max_people, table_assignments, x, y, options)
        }
    }
}

/// Look up the assignment occupying `seat_index`, if any. `table_assignments`
/// is sorted by `seat_index` (see [`build_layout`]).
fn occupant_at<'a>(
    table_assignments: &[&'a SeatingAssignment],
    seat_index: usize,
) -> Option<&'a SeatingAssignment> {
    table_assignments
        .binary_search_by_key(&seat_index, |assignment| assignment.seat_index)
        .ok()
        .map(|position| table_assignments[position])
}

/// Places one seat per capacity slot evenly around a ring, regardless of how
/// many are actually occupied, so the rendered angle always matches
/// `TAU * seat_index / max_people` — the same geometry `scoring::circular_distance`
/// assumes.
fn build_round_seats(
    max_people: usize,
    table_assignments: &[&SeatingAssignment],
    x: f32,
    y: f32,
    options: &RenderOptions,
) -> Vec<LayoutSeat> {
    let (center_x, center_y, radius) = round_table_metrics(x, y, options);
    let seat_count = max_people.max(1);
    (0..max_people)
        .map(|seat_index| {
            let angle = std::f32::consts::TAU * seat_index as f32 / seat_count as f32
                - std::f32::consts::FRAC_PI_2;
            LayoutSeat {
                seat_index,
                x: center_x + radius * angle.cos(),
                y: center_y + radius * angle.sin(),
                person_name: occupant_at(table_assignments, seat_index)
                    .map(|assignment| assignment.person_name.clone()),
            }
        })
        .collect()
}

/// Places one seat per capacity slot along the arc of a semicircle table,
/// on the ring [`semicircle_table_metrics`] computes. Seats span angles
/// `PI..2*PI` (the upper half of the ring in screen space, where `y`
/// increases downward), so `x` increases and `y` stays above the flat edge
/// (`y < cy`) for every seat — no wrap-around, unlike a round table.
fn build_semicircle_seats(
    max_people: usize,
    table_assignments: &[&SeatingAssignment],
    x: f32,
    y: f32,
    options: &RenderOptions,
) -> Vec<LayoutSeat> {
    let (center_x, center_y, radius) = semicircle_table_metrics(x, y, options);
    let seat_count = max_people.max(1);
    (0..max_people)
        .map(|seat_index| {
            let angle = std::f32::consts::PI
                + std::f32::consts::PI * (seat_index as f32 + 0.5) / seat_count as f32;
            LayoutSeat {
                seat_index,
                x: center_x + radius * angle.cos(),
                y: center_y + radius * angle.sin(),
                person_name: occupant_at(table_assignments, seat_index)
                    .map(|assignment| assignment.person_name.clone()),
            }
        })
        .collect()
}

/// Places one seat per capacity slot around the table perimeter, regardless
/// of how many are actually occupied.
fn build_rectangular_seats(
    max_people: usize,
    people_per_side: &[usize],
    table_assignments: &[&SeatingAssignment],
    x: f32,
    y: f32,
    options: &RenderOptions,
) -> Vec<LayoutSeat> {
    let mut points = Vec::new();
    // Fixed proportions of the default 240x220 card; see the render_svg
    // surface-rect comment for the same caveat.
    let left = x + 52.0;
    let right = x + options.table_width - 52.0;
    let top = y + 68.0;
    let bottom = y + options.table_height - 58.0;
    let counts =
        if people_per_side.len() == 4 && people_per_side.iter().sum::<usize>() == max_people {
            people_per_side.to_vec()
        } else {
            spread_evenly(max_people)
        };

    points.extend(line_points(counts[0], left + 14.0, right - 14.0, top, top));
    points.extend(line_points(
        counts[1],
        right,
        right,
        top + 14.0,
        bottom - 14.0,
    ));
    points.extend(line_points(
        counts[2],
        right - 14.0,
        left + 14.0,
        bottom,
        bottom,
    ));
    points.extend(line_points(
        counts[3],
        left,
        left,
        bottom - 14.0,
        top + 14.0,
    ));

    points
        .into_iter()
        .take(max_people)
        .enumerate()
        .map(|(seat_index, (seat_x, seat_y))| LayoutSeat {
            seat_index,
            x: seat_x,
            y: seat_y,
            person_name: occupant_at(table_assignments, seat_index)
                .map(|assignment| assignment.person_name.clone()),
        })
        .collect()
}

fn line_points(
    count: usize,
    start_x: f32,
    end_x: f32,
    start_y: f32,
    end_y: f32,
) -> Vec<(f32, f32)> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![((start_x + end_x) / 2.0, (start_y + end_y) / 2.0)];
    }
    (0..count)
        .map(|index| {
            let ratio = index as f32 / (count - 1) as f32;
            (
                start_x + (end_x - start_x) * ratio,
                start_y + (end_y - start_y) * ratio,
            )
        })
        .collect()
}

fn spread_evenly(seat_count: usize) -> Vec<usize> {
    let base = seat_count / 4;
    let remainder = seat_count % 4;
    (0..4)
        .map(|index| base + usize::from(index < remainder))
        .collect()
}

/// Smallest center-to-center distance between any two seats at a table,
/// used as a guest-label width budget so labels don't overlap their
/// neighbors. `None` when there are fewer than two seats to compare.
pub fn min_seat_spacing(seats: &[LayoutSeat]) -> Option<f32> {
    let mut min_dist = f32::INFINITY;
    for i in 0..seats.len() {
        for j in (i + 1)..seats.len() {
            let dx = seats[i].x - seats[j].x;
            let dy = seats[i].y - seats[j].y;
            min_dist = min_dist.min((dx * dx + dy * dy).sqrt());
        }
    }
    min_dist.is_finite().then_some(min_dist)
}

/// Greedily word-wraps `name` into lines that each fit within `budget_px`,
/// approximating Arial glyph width as `0.55 * font_size` per character.
/// Words longer than one line are split across lines by character. Never
/// truncates or adds an ellipsis; the full name is always recoverable by
/// rejoining the returned lines, and is also preserved separately in a
/// `<title>` element for hover text.
fn wrap_label(name: &str, budget_px: f32, font_size: f32) -> Vec<String> {
    let char_width = (font_size * 0.55).max(1.0);
    let max_chars = (budget_px / char_width).floor().max(1.0) as usize;

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in name.split_whitespace() {
        for chunk in split_into_chunks(word, max_chars) {
            if current.is_empty() {
                current = chunk;
            } else if current.chars().count() + 1 + chunk.chars().count() <= max_chars {
                current.push(' ');
                current.push_str(&chunk);
            } else {
                lines.push(std::mem::take(&mut current));
                current = chunk;
            }
        }
    }
    lines.push(current);
    lines
}

/// Splits `word` into chunks of at most `max_chars` characters, so a single
/// word longer than the label budget still wraps instead of overflowing.
fn split_into_chunks(word: &str, max_chars: usize) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    if chars.len() <= max_chars {
        return vec![word.to_string()];
    }
    chars
        .chunks(max_chars)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn shape_label(shape: &TableShape) -> &'static str {
    match shape {
        TableShape::Round => "round",
        TableShape::Rectangular => "rectangular",
        TableShape::Square => "square",
        TableShape::Semicircle => "semicircle",
    }
}

#[cfg(test)]
mod tests {
    use super::wrap_label;

    /// Every line `wrap_label` returns must fit the same width estimate the
    /// budget was computed with, and rejoining the lines (ignoring the
    /// whitespace introduced/removed at wrap points) must reproduce the
    /// original name — i.e. no characters are dropped or replaced with an
    /// ellipsis.
    #[test]
    fn wrap_label_fits_budget_and_preserves_all_characters() {
        let name = "Alexandria Montgomery-Featherstonehaugh";
        let font_size: f32 = 13.0;
        let budget_px: f32 = 90.0;
        let char_width = (font_size * 0.55).max(1.0);
        let max_chars = (budget_px / char_width).floor().max(1.0) as usize;

        let lines = wrap_label(name, budget_px, font_size);
        assert!(lines.len() >= 2);
        for line in &lines {
            assert!(
                line.chars().count() <= max_chars,
                "line {line:?} exceeds max_chars {max_chars}"
            );
        }

        let rejoined: String = lines
            .concat()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let original: String = name.chars().filter(|c| !c.is_whitespace()).collect();
        assert_eq!(rejoined, original);
    }

    /// A budget that fits any single word but not two adjacent ones wraps
    /// one word per line.
    #[test]
    fn wrap_label_breaks_on_word_boundaries() {
        let lines = wrap_label("Ana Maria Lopez", 45.0, 13.0);
        assert_eq!(lines, vec!["Ana", "Maria", "Lopez"]);
    }

    /// Multi-byte (accented) characters count as one character each, same
    /// as the width estimate assumes; combining-accent edge cases are out of
    /// scope. The budget here (max_chars = 8) sits strictly between
    /// "Ångström"'s char length (8, fits) and its UTF-8 byte length (10,
    /// wouldn't fit) — a regression to byte-counting would wrongly split
    /// this word.
    #[test]
    fn wrap_label_handles_multi_byte_names() {
        let lines = wrap_label("Núñez Ångström", 60.0, 13.0);
        assert_eq!(lines, vec!["Núñez", "Ångström"]);
    }

    #[test]
    fn wrap_label_of_empty_name_returns_one_empty_line() {
        let lines = wrap_label("", 90.0, 13.0);
        assert_eq!(lines, vec![String::new()]);
    }
}
