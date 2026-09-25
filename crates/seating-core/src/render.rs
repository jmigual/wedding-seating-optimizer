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

mod labels;

pub use labels::{LabelAlign, MIN_LABEL_FONT_SIZE, SeatLabel, seat_label};

/// Height of one row of [`LayoutTable::empty_seats`] markers, added below
/// the table area for each row needed in [`build_editor_layout`] (a table
/// with many free seats wraps into several). The strict [`build_layout`]
/// never adds this since it never populates `empty_seats`.
const EMPTY_SEAT_ROW_HEIGHT: f32 = 40.0;

/// Default [`RenderOptions::label_font_size`]. Below it the seat area keeps
/// this size; above it the seat area grows proportionally (see
/// [`seat_area_size`]).
const DEFAULT_LABEL_FONT_SIZE: f32 = 9.0;

/// Seat-area size (the box every seat marker sits in) at or below
/// [`DEFAULT_LABEL_FONT_SIZE`]. Wide enough for a 12-seat semicircle of
/// radius > 100 and tall enough for a round ring of radius 67, whose 9-seat
/// chord fits a 9-character word at the default label size.
const SEAT_AREA_WIDTH: f32 = 231.0;
const SEAT_AREA_HEIGHT: f32 = 160.0;

/// Room reserved for guest labels on the left and right of the seat area,
/// in multiples of `label_font_size`: a 9-character word beside a
/// rectangular table's side seats, plus the seat gap and card inset.
const LABEL_ROOM_X: f32 = 6.0;
/// Room reserved for guest labels above and below the seat area, in
/// multiples of `label_font_size`: three wrapped lines plus the seat gap and
/// card inset.
const LABEL_ROOM_Y: f32 = 4.5;

/// Geometry and spacing options for layout/rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// Outer margin around the full plan.
    pub margin: f32,
    /// Horizontal gap between tables.
    pub column_gap: f32,
    /// Vertical gap between tables.
    pub row_gap: f32,
    /// Minimum width of each table card. Cards grow past it to fit the seat
    /// area plus the guest-label room `label_font_size` needs.
    pub table_width: f32,
    /// Minimum height of each table card's table area (before any editor
    /// free-seat rows). Grows the same way as `table_width`.
    pub table_height: f32,
    /// Radius of each rendered seat marker.
    pub seat_radius: f32,
    /// Base font size for table titles and seat indices.
    pub font_size: f32,
    /// Requested font size for guest-name labels. [`seat_label`] shrinks it
    /// per label, down to [`MIN_LABEL_FONT_SIZE`], only when a name does not
    /// fit its box; table cards grow with it.
    pub label_font_size: f32,
    /// PNG rasterization scale factor (1.0 = CSS pixel size).
    pub png_scale: f32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        let mut options = Self {
            margin: 24.0,
            column_gap: 36.0,
            row_gap: 36.0,
            // Replaced below by the derived card size.
            table_width: 0.0,
            table_height: 0.0,
            seat_radius: 13.0,
            font_size: 14.0,
            label_font_size: DEFAULT_LABEL_FONT_SIZE,
            png_scale: 2.0,
        };
        // The minimum card is exactly what `card_size` derives at the default
        // sizes, so the two can never drift apart.
        (options.table_width, options.table_height) = card_size(&options);
        options
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
    /// Concrete seat positions for the table's OCCUPIED seats only, spaced
    /// evenly by rank (position among occupants, `0..k`) rather than by raw
    /// capacity slot — an empty seat never leaves a visual gap. `seat_index`
    /// on each entry is still the real seat index, so drops keep identifying
    /// the correct seat.
    pub seats: Vec<LayoutSeat>,
    /// Markers for the table's free (unoccupied) seats, populated only by
    /// [`build_editor_layout`] and laid out in one or more rows below the
    /// table area (the card is grown to fit them); always empty for the
    /// strict [`build_layout`] used by exports.
    pub empty_seats: Vec<LayoutSeat>,
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
/// Used for both [`LayoutTable::seats`] (occupied seats, `person_name` always
/// `Some`) and [`LayoutTable::empty_seats`] (free seats, `person_name` always
/// `None`).
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
/// unassigned. Tables with no occupants are omitted. Card and seat geometry
/// follow `options` (notably `label_font_size`), so render the result with
/// the same options.
pub fn build_layout(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    options: &RenderOptions,
) -> Result<SeatingLayout, ValidationReport> {
    build_layout_impl(project, assignments, options, false, true)
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
    options: &RenderOptions,
) -> Result<SeatingLayout, ValidationReport> {
    build_layout_impl(project, assignments, options, include_empty_tables, false)
}

fn build_layout_impl(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    options: &RenderOptions,
    include_empty_tables: bool,
    require_all_people: bool,
) -> Result<SeatingLayout, ValidationReport> {
    if require_all_people {
        validate_seating_solution(project, assignments)?;
    } else {
        validate_partial_seating_solution(project, assignments)?;
    }

    let (table_width, table_height) = card_size(options);
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

    // Editor tables reserve extra rows of height for `empty_seats`, which
    // only the editor layout ever populates; the strict export layout keeps
    // the plain card height. Every card in the grid shares one uniform
    // height, sized to whichever table needs the most empty-seat rows (at
    // least one, even for a fully-occupied table) so free-seat markers never
    // overlap regardless of how many seats are free.
    let max_per_row = empty_seat_row_capacity(options);
    let empty_seats_by_table: Vec<Vec<usize>> = if require_all_people {
        Vec::new()
    } else {
        used_instances
            .iter()
            .map(|table| {
                let occupied: HashSet<usize> = assignments_by_table
                    .get(&table.number)
                    .map(|assignments| assignments.iter().map(|a| a.seat_index).collect())
                    .unwrap_or_default();
                (0..table.max_people)
                    .filter(|seat| !occupied.contains(seat))
                    .collect()
            })
            .collect()
    };
    let extra_rows = empty_seats_by_table
        .iter()
        .map(|free| free.len().div_ceil(max_per_row).max(1))
        .max()
        .unwrap_or(0);
    let card_height = if require_all_people {
        table_height
    } else {
        table_height + extra_rows as f32 * EMPTY_SEAT_ROW_HEIGHT
    };
    let mut tables = Vec::new();

    for (index, table) in used_instances.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let x = options.margin + column as f32 * (table_width + options.column_gap);
        let y = options.margin + row as f32 * (card_height + options.row_gap);
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
            options,
            table_assignments,
        );
        let empty_seats = empty_seats_by_table
            .get(index)
            .map(|free_seats| build_empty_seat_row(free_seats, x, y, options, max_per_row))
            .unwrap_or_default();
        let surface = build_surface(&table.shape, x, y, options);
        tables.push(LayoutTable {
            table_number: table.number,
            table_type: table.table_type.clone(),
            shape: table.shape.clone(),
            x,
            y,
            width: table_width,
            height: card_height,
            seats,
            empty_seats,
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
            + columns as f32 * table_width
            + columns.saturating_sub(1) as f32 * options.column_gap
    };
    let height = if rows == 0 {
        options.margin * 2.0
    } else {
        options.margin * 2.0
            + rows as f32 * card_height
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
/// Guest labels are wrapped and fitted by [`seat_label`] (widths estimated
/// at `0.55 * font_size` per character), so they stay inside their card;
/// pass the same `options` the layout was built with.
pub fn render_svg(layout: &SeatingLayout, options: &RenderOptions) -> String {
    let mut body = String::new();
    body.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        hex(COLOR_BACKGROUND)
    ));
    body.push_str(&format!(
        "<style>text {{ fill: {}; font-family: Arial, Helvetica, sans-serif; font-size: {}px; }} .muted {{ fill: {}; }} .seat-index {{ fill: {}; font-size: {}px; font-weight: bold; }} .guest {{ fill: {}; }}</style>",
        hex(COLOR_SEAT_FILL),
        options.font_size,
        hex(COLOR_MUTED),
        hex(COLOR_BACKGROUND),
        options.font_size - 3.0,
        hex(COLOR_GUEST_TEXT)
    ));

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

        for seat in &table.seats {
            body.push_str("<g>");
            if let Some(name) = seat.person_name.as_deref() {
                body.push_str(&format!("<title>{}</title>", escape_xml(name)));
            }
            if seat.person_name.is_some() {
                body.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    seat.x, seat.y, options.seat_radius, hex(COLOR_SEAT_FILL), hex(COLOR_SEAT_STROKE)
                ));
                body.push_str(&format!(
                    "<text class=\"seat-index\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>",
                    seat.x,
                    seat.y + 0.5,
                    seat.seat_index + 1
                ));
                if let Some(label) = seat_label(table, seat, options, |text, font_size| {
                    text.chars().count() as f32 * 0.55 * font_size
                }) {
                    let anchor = match label.align {
                        LabelAlign::Start => "start",
                        LabelAlign::Center => "middle",
                        LabelAlign::End => "end",
                    };
                    // Inline `style`, not a `font-size` attribute: the
                    // stylesheet's `text` rule would override an attribute.
                    body.push_str(&format!(
                        "<text class=\"guest\" text-anchor=\"{anchor}\" style=\"font-size: {:.2}px\">",
                        label.font_size
                    ));
                    for (line_index, line) in label.lines.iter().enumerate() {
                        // Baseline of a line whose box starts at `top`.
                        let baseline =
                            label.top + line_index as f32 * label.line_height + label.font_size;
                        body.push_str(&format!(
                            "<tspan x=\"{:.1}\" y=\"{:.1}\">{}</tspan>",
                            label.x,
                            baseline,
                            escape_xml(line)
                        ));
                    }
                    body.push_str("</text>");
                }
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

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}" viewBox="0 0 {:.0} {:.0}">"#,
        layout.width, layout.height, layout.width, layout.height
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

/// An axis-aligned box in layout units.
#[derive(Debug, Clone, Copy)]
struct Bounds {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

/// Seat-area width and height: [`SEAT_AREA_WIDTH`]x[`SEAT_AREA_HEIGHT`] up
/// to the default label font size, growing proportionally above it so seat
/// spacing (and the label width it bounds) keeps up with bigger text.
fn seat_area_size(options: &RenderOptions) -> (f32, f32) {
    let grow = (options.label_font_size / DEFAULT_LABEL_FONT_SIZE).max(1.0);
    (SEAT_AREA_WIDTH * grow, SEAT_AREA_HEIGHT * grow)
}

/// Card width and table-area height (the card before any editor free-seat
/// rows): the seat area plus [`LABEL_ROOM_X`]/[`LABEL_ROOM_Y`] of label room
/// on each side plus the header, never below `table_width`/`table_height`.
fn card_size(options: &RenderOptions) -> (f32, f32) {
    let (seat_width, seat_height) = seat_area_size(options);
    let room_x = LABEL_ROOM_X * options.label_font_size;
    let room_y = LABEL_ROOM_Y * options.label_font_size;
    let width = seat_width + 2.0 * room_x;
    let height = header_bottom(0.0, options) + seat_height + 2.0 * room_y;
    (
        width.max(options.table_width),
        height.max(options.table_height),
    )
}

/// The box every seat marker (center ± `seat_radius`) of the card at
/// `(x, y)` sits in: centered horizontally in the card and vertically
/// between the header and the table-area bottom, leaving label room around it.
fn seat_area(x: f32, y: f32, options: &RenderOptions) -> Bounds {
    let (card_width, card_height) = card_size(options);
    let (width, height) = seat_area_size(options);
    let header = header_bottom(y, options);
    let left = x + (card_width - width) / 2.0;
    let top = header + (y + card_height - header - height) / 2.0;
    Bounds {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}

/// Center and ring radius for a round table's seats: the largest ring
/// whose markers fit inside the [`seat_area`], centered in it.
fn round_table_metrics(x: f32, y: f32, options: &RenderOptions) -> (f32, f32, f32) {
    let area = seat_area(x, y, options);
    let center_x = (area.left + area.right) / 2.0;
    let center_y = (area.top + area.bottom) / 2.0;
    let radius = ((area.bottom - area.top).min(area.right - area.left) / 2.0 - options.seat_radius)
        .max(20.0);
    (center_x, center_y, radius)
}

/// Center and ring radius for a semicircle table's seats, sized to use the
/// seat area's full height instead of half of it.
///
/// [`round_table_metrics`] centers its ring so a full circle fits, giving a
/// semicircle (which only draws its upper arc) roughly half the usable
/// height. Putting the flat edge at the seat area's bottom instead lets the
/// arc span its full height, roughly doubling how many seats fit before
/// adjacent markers overlap.
fn semicircle_table_metrics(x: f32, y: f32, options: &RenderOptions) -> (f32, f32, f32) {
    let area = seat_area(x, y, options);
    let center_x = (area.left + area.right) / 2.0;
    let center_y = area.bottom - options.seat_radius;
    let vertical_radius = area.bottom - area.top - 2.0 * options.seat_radius;
    let horizontal_radius = (area.right - area.left) / 2.0 - options.seat_radius;
    let radius = vertical_radius.min(horizontal_radius).max(20.0);
    (center_x, center_y, radius)
}

/// Seat-center lines of a rectangular/square table (left, top, right,
/// bottom): the [`seat_area`] inset by `seat_radius`, so every marker stays
/// inside it.
fn rectangular_seat_lines(x: f32, y: f32, options: &RenderOptions) -> Bounds {
    let area = seat_area(x, y, options);
    Bounds {
        left: area.left + options.seat_radius,
        top: area.top + options.seat_radius,
        right: area.right - options.seat_radius,
        bottom: area.bottom - options.seat_radius,
    }
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
        // The surface sits just inside the seat lines, so seats straddle
        // its edges.
        TableShape::Rectangular | TableShape::Square => {
            let lines = rectangular_seat_lines(x, y, options);
            TableSurface::Rect {
                x: lines.left + 8.0,
                y: lines.top + 4.0,
                width: lines.right - lines.left - 16.0,
                height: lines.bottom - lines.top - 8.0,
            }
        }
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
        TableShape::Round => build_round_seats(table_assignments, x, y, options),
        TableShape::Rectangular | TableShape::Square => build_rectangular_seats(
            max_people,
            people_per_side.unwrap_or(&[]),
            table_assignments,
            x,
            y,
            options,
        ),
        TableShape::Semicircle => build_semicircle_seats(table_assignments, x, y, options),
    }
}

/// Places one seat per OCCUPIED seat, evenly spaced around the ring by rank
/// (position among occupants, `0..k`) rather than by raw seat index — an
/// empty seat between two guests never opens a visual (or scoring) gap. The
/// same rank-based angle `scoring::circular_distance` assumes.
/// `table_assignments` is sorted by `seat_index` (see [`build_layout`]).
fn build_round_seats(
    table_assignments: &[&SeatingAssignment],
    x: f32,
    y: f32,
    options: &RenderOptions,
) -> Vec<LayoutSeat> {
    let (center_x, center_y, radius) = round_table_metrics(x, y, options);
    let k = table_assignments.len();
    table_assignments
        .iter()
        .enumerate()
        .map(|(rank, assignment)| {
            let angle =
                std::f32::consts::TAU * rank as f32 / k as f32 - std::f32::consts::FRAC_PI_2;
            LayoutSeat {
                seat_index: assignment.seat_index,
                x: center_x + radius * angle.cos(),
                y: center_y + radius * angle.sin(),
                person_name: Some(assignment.person_name.clone()),
            }
        })
        .collect()
}

/// Places one seat per OCCUPIED seat along the arc of a semicircle table,
/// spaced by rank the same way [`build_round_seats`] is. Seats span angles
/// `PI..2*PI` (the upper half of the ring in screen space, where `y`
/// increases downward), so `x` increases and `y` stays above the flat edge
/// (`y < cy`) for every seat — no wrap-around, unlike a round table.
fn build_semicircle_seats(
    table_assignments: &[&SeatingAssignment],
    x: f32,
    y: f32,
    options: &RenderOptions,
) -> Vec<LayoutSeat> {
    let (center_x, center_y, radius) = semicircle_table_metrics(x, y, options);
    let k = table_assignments.len();
    table_assignments
        .iter()
        .enumerate()
        .map(|(rank, assignment)| {
            let angle =
                std::f32::consts::PI + std::f32::consts::PI * (rank as f32 + 0.5) / k as f32;
            LayoutSeat {
                seat_index: assignment.seat_index,
                x: center_x + radius * angle.cos(),
                y: center_y + radius * angle.sin(),
                person_name: Some(assignment.person_name.clone()),
            }
        })
        .collect()
}

/// Places one seat per OCCUPIED seat around the table perimeter. The seat
/// counts per side are apportioned from `people_per_side` (or an even spread)
/// down to the number actually occupied — see [`apportion`] — so a
/// half-empty table still spaces its occupants evenly along each side
/// instead of leaving capacity-sized gaps.
fn build_rectangular_seats(
    max_people: usize,
    people_per_side: &[usize],
    table_assignments: &[&SeatingAssignment],
    x: f32,
    y: f32,
    options: &RenderOptions,
) -> Vec<LayoutSeat> {
    let mut points = Vec::new();
    let Bounds {
        left,
        top,
        right,
        bottom,
    } = rectangular_seat_lines(x, y, options);
    let base_counts =
        if people_per_side.len() == 4 && people_per_side.iter().sum::<usize>() == max_people {
            people_per_side.to_vec()
        } else {
            spread_evenly(max_people)
        };
    let counts = apportion(&base_counts, table_assignments.len());
    // Each side's end seats stop this far short of the corner, so the end
    // seats of two adjacent sides sit `corner * sqrt(2)` (> 2 * seat_radius)
    // apart and their markers never overlap.
    let corner = options.seat_radius * 1.5;

    points.extend(line_points(
        counts[0],
        left + corner,
        right - corner,
        top,
        top,
    ));
    points.extend(line_points(
        counts[1],
        right,
        right,
        top + corner,
        bottom - corner,
    ));
    points.extend(line_points(
        counts[2],
        right - corner,
        left + corner,
        bottom,
        bottom,
    ));
    points.extend(line_points(
        counts[3],
        left,
        left,
        bottom - corner,
        top + corner,
    ));

    points
        .into_iter()
        .zip(table_assignments.iter())
        .map(|((seat_x, seat_y), assignment)| LayoutSeat {
            seat_index: assignment.seat_index,
            x: seat_x,
            y: seat_y,
            person_name: Some(assignment.person_name.clone()),
        })
        .collect()
}

/// Apportion `total` items across `weights.len()` buckets, proportionally to
/// `weights`, via the largest-remainder method: each bucket gets
/// `floor(weight * total / sum(weights))`, then the leftover units go to the
/// buckets with the largest fractional remainder, lower-index buckets
/// breaking ties. Used to shrink a table's per-side seat counts down to the
/// number actually occupied while keeping the same side proportions.
fn apportion(weights: &[usize], total: usize) -> Vec<usize> {
    let sum_weights: usize = weights.iter().sum();
    if sum_weights == 0 {
        return vec![0; weights.len()];
    }
    let mut counts = Vec::with_capacity(weights.len());
    let mut remainders = Vec::with_capacity(weights.len());
    for &weight in weights {
        let product = weight * total;
        counts.push(product / sum_weights);
        remainders.push(product % sum_weights);
    }
    let mut remaining = total - counts.iter().sum::<usize>();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by(|&a, &b| remainders[b].cmp(&remainders[a]).then(a.cmp(&b)));
    for index in order {
        if remaining == 0 {
            break;
        }
        counts[index] += 1;
        remaining -= 1;
    }
    counts
}

/// The most free-seat markers [`build_empty_seat_row`] can fit in one row
/// without any two adjacent centers landing closer than `2 * seat_radius`
/// (the diameter, i.e. touching) apart, given the row's usable width (the
/// same left/right inset [`build_empty_seat_row`] places markers within).
fn empty_seat_row_capacity(options: &RenderOptions) -> usize {
    let usable_width = (card_size(options).0 - 48.0).max(0.0);
    let diameter = (options.seat_radius * 2.0).max(1.0);
    ((usable_width / diameter).floor() as usize + 1).max(1)
}

/// Lays out `free_seat_indices` as evenly-spaced rows of markers below the
/// table area, inside the card's reserved [`EMPTY_SEAT_ROW_HEIGHT`]-tall
/// strips — wrapping into additional rows, at most `max_per_row` markers
/// each (see [`empty_seat_row_capacity`]), so markers never overlap
/// regardless of how many seats are free.
fn build_empty_seat_row(
    free_seat_indices: &[usize],
    x: f32,
    y: f32,
    options: &RenderOptions,
    max_per_row: usize,
) -> Vec<LayoutSeat> {
    let (table_width, table_height) = card_size(options);
    let left = x + 24.0;
    let right = x + table_width - 24.0;
    free_seat_indices
        .chunks(max_per_row.max(1))
        .enumerate()
        .flat_map(|(row_index, chunk)| {
            let row_y = y + table_height + EMPTY_SEAT_ROW_HEIGHT * (row_index as f32 + 0.5);
            line_points(chunk.len(), left, right, row_y, row_y)
                .into_iter()
                .zip(chunk.iter())
                .map(|((seat_x, seat_y), &seat_index)| LayoutSeat {
                    seat_index,
                    x: seat_x,
                    y: seat_y,
                    person_name: None,
                })
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

/// Smallest center-to-center distance between any two seats at a table.
/// `None` when there are fewer than two seats to compare.
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
    use super::{
        LayoutSeat, LayoutTable, RenderOptions, SeatingAssignment, SeatingLayout, TableSurface,
        apportion, build_rectangular_seats, render_svg,
    };
    use crate::models::TableShape;

    /// `apportion` is the largest-remainder method: each side gets its exact
    /// proportional share (5|5|0|0 scaled to 6 total is exactly 3|3|0|0 with
    /// no remainder to distribute).
    #[test]
    fn rectangular_layout_apportions_occupied_seats_by_side() {
        assert_eq!(apportion(&[5, 5, 0, 0], 6), vec![3, 3, 0, 0]);
    }

    /// When the proportional shares don't divide evenly, the leftover units
    /// go to the buckets with the largest fractional remainder, and ties
    /// break to the lower index: `[3,3,2,2]` scaled to 5 gives exact shares
    /// `1.5, 1.5, 1, 1` — both `1.5`s round down to `1` with a tied 0.5
    /// remainder, so the single leftover unit goes to index 0.
    #[test]
    fn apportion_breaks_tied_remainders_by_lower_index() {
        assert_eq!(apportion(&[3, 3, 2, 2], 5), vec![2, 1, 1, 1]);
    }

    /// A `people_per_side` that is missing (empty slice, matching
    /// `people_per_side.unwrap_or(&[])` in [`super::build_seat_positions`])
    /// or otherwise malformed falls back to an even spread across the 4
    /// sides, which [`apportion`] then shrinks to the number actually
    /// occupied — 8 capacity, 5 occupied, even spread `[2,2,2,2]` apportions
    /// to `[2,1,1,1]` (see [`apportion_breaks_tied_remainders_by_lower_index`]
    /// for the tie-break), so the top side keeps 2 seats and the other 3
    /// sides get 1 each, walked top → right → bottom → left.
    #[test]
    fn rectangular_seats_fall_back_to_even_spread_when_people_per_side_is_missing() {
        let options = RenderOptions::default();
        let assignments: Vec<SeatingAssignment> = (0..5)
            .map(|seat_index| SeatingAssignment {
                table_number: 1,
                table_type: "rect_8".to_string(),
                seat_index,
                person_id: format!("p{seat_index}"),
                person_name: format!("Guest {seat_index}"),
            })
            .collect();
        let refs: Vec<&SeatingAssignment> = assignments.iter().collect();

        let seats = build_rectangular_seats(8, &[], &refs, 0.0, 0.0, &options);

        assert_eq!(seats.len(), 5);
        // Top side (2 seats): same y, left-to-right.
        assert_eq!(seats[0].y, seats[1].y);
        assert!(seats[0].x < seats[1].x);
        // Right side (1 seat): further right and further down than the top row.
        assert!(seats[2].x > seats[1].x);
        assert!(seats[2].y > seats[0].y);
        // Bottom side (1 seat): further down than the right side, and left of it.
        assert!(seats[3].y > seats[2].y);
        assert!(seats[3].x < seats[2].x);
        // Left side (1 seat): further left than the top row, at the same
        // mid-height as the right side (both single midpoints).
        assert!(seats[4].x < seats[0].x);
        assert_eq!(seats[4].y, seats[2].y);
    }

    /// Seat 0 is the first seat, so its rendered index text must read "1",
    /// not "0" (display is 1-based; `LayoutSeat::seat_index` itself stays
    /// 0-based).
    #[test]
    fn rendered_seat_index_is_one_based() {
        let layout = SeatingLayout {
            width: 400.0,
            height: 400.0,
            tables: vec![LayoutTable {
                table_number: 1,
                table_type: "round_8".to_string(),
                shape: TableShape::Round,
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 200.0,
                seats: vec![LayoutSeat {
                    seat_index: 0,
                    x: 100.0,
                    y: 100.0,
                    person_name: Some("Alice".to_string()),
                }],
                empty_seats: vec![],
                surface: TableSurface::Round {
                    cx: 100.0,
                    cy: 100.0,
                    radius: 60.0,
                },
            }],
        };

        let svg = render_svg(&layout, &RenderOptions::default());

        assert!(svg.contains(">1<"));
        assert!(!svg.contains(">0<"));
    }
}
