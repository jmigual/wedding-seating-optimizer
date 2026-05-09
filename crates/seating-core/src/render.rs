//! Reusable seating-plan layout and rendering.
//!
//! The GUI and CLI both use this module to turn a validated seating solution
//! into a layout description and exportable SVG/PNG outputs.

use crate::models::{ProjectInput, SeatingAssignment, TableShape, ValidationReport};
use crate::validation::{generate_table_instances, validate_seating_solution};
use std::collections::HashMap;
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
        }
    }
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
    /// Concrete seat positions around the table.
    pub seats: Vec<LayoutSeat>,
}

/// One rendered seat marker within a table layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSeat {
    /// Zero-based seat index.
    pub seat_index: usize,
    /// Seat-center X coordinate.
    pub x: f32,
    /// Seat-center Y coordinate.
    pub y: f32,
    /// Occupant name when assigned.
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
pub fn build_layout(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
) -> Result<SeatingLayout, ValidationReport> {
    build_layout_with_options(project, assignments, &RenderOptions::default())
}

/// Build a reusable layout using caller-provided render options.
pub fn build_layout_with_options(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    options: &RenderOptions,
) -> Result<SeatingLayout, ValidationReport> {
    validate_seating_solution(project, assignments)?;

    let instances = generate_table_instances(project);
    let assignment_lookup: HashMap<(usize, usize), &SeatingAssignment> = assignments
        .iter()
        .map(|assignment| ((assignment.table_number, assignment.seat_index), assignment))
        .collect();

    let columns = columns_for(instances.len());
    let mut tables = Vec::new();

    for (index, table) in instances.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let x = options.margin + column as f32 * (options.table_width + options.column_gap);
        let y = options.margin + row as f32 * (options.table_height + options.row_gap);
        let config = &project.table_types[&table.table_type];
        let seats = build_seat_positions(
            table.shape.clone(),
            config.people_per_side.as_deref(),
            table.max_people,
            x,
            y,
            options,
            &assignment_lookup,
            table.number,
        );
        tables.push(LayoutTable {
            table_number: table.number,
            table_type: table.table_type.clone(),
            shape: table.shape.clone(),
            x,
            y,
            width: options.table_width,
            height: options.table_height,
            seats,
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
pub fn render_svg(layout: &SeatingLayout, options: &RenderOptions) -> String {
    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}" viewBox="0 0 {:.0} {:.0}">"#,
        layout.width, layout.height, layout.width, layout.height
    ));
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#10151c\"/>");
    svg.push_str(&format!(
        "<style>text {{ fill: #f5f7fa; font-family: Arial, Helvetica, sans-serif; font-size: {}px; }} .muted {{ fill: #b8c1cc; }} .seat-index {{ fill: #10151c; font-size: {}px; font-weight: bold; }} .guest {{ fill: #dbe7ff; font-size: {}px; }}</style>",
        options.font_size,
        options.font_size - 3.0,
        options.font_size - 1.0
    ));

    for table in &layout.tables {
        let label_x = table.x + table.width / 2.0;
        let title_y = table.y + 24.0;
        svg.push_str(&format!(
            "<g><rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"18\" fill=\"#17212b\" stroke=\"#3f5368\" stroke-width=\"1.5\"/>",
            table.x, table.y, table.width, table.height
        ));
        svg.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">Table {} — {}</text>",
            label_x,
            title_y,
            table.table_number,
            escape_xml(&table.table_type)
        ));
        svg.push_str(&format!(
            "<text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">Shape: {}</text>",
            label_x,
            title_y + 20.0,
            shape_label(&table.shape)
        ));

        match table.shape {
            TableShape::Round => svg.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"48\" fill=\"#355070\" stroke=\"#90e0ef\" stroke-width=\"2\"/>",
                table.x + table.width / 2.0,
                table.y + table.height / 2.0 + 6.0,
            )),
            TableShape::Rectangular | TableShape::Square => svg.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"12\" fill=\"#355070\" stroke=\"#90e0ef\" stroke-width=\"2\"/>",
                table.x + 60.0,
                table.y + 72.0,
                table.width - 120.0,
                table.height - 110.0,
            )),
        }

        for seat in &table.seats {
            svg.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"#f5f7fa\" stroke=\"#5c6773\" stroke-width=\"1.5\"/>",
                seat.x, seat.y, options.seat_radius
            ));
            svg.push_str(&format!(
                "<text class=\"seat-index\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" dominant-baseline=\"middle\">{}</text>",
                seat.x,
                seat.y + 0.5,
                seat.seat_index
            ));
            let guest_label = seat
                .person_name
                .as_deref()
                .map(escape_xml)
                .unwrap_or_else(|| format!("Seat {}", seat.seat_index));
            svg.push_str(&format!(
                "<text class=\"guest\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">{}</text>",
                seat.x,
                seat.y + options.seat_radius + 16.0,
                guest_label
            ));
        }
        svg.push_str("</g>");
    }

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
    let svg_options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&svg, &svg_options)
        .map_err(|error| RenderingError::SvgParse(error.to_string()))?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height()).ok_or(
        RenderingError::PixmapAllocation {
            width: size.width(),
            height: size.height(),
        },
    )?;
    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap_mut,
    );
    pixmap
        .save_png(path)
        .map_err(|error| RenderingError::WritePng(error.to_string()))
}

fn columns_for(table_count: usize) -> usize {
    match table_count {
        0 => 1,
        1..=4 => table_count,
        _ => (table_count as f32).sqrt().ceil() as usize,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_seat_positions(
    shape: TableShape,
    people_per_side: Option<&[usize]>,
    seat_count: usize,
    x: f32,
    y: f32,
    options: &RenderOptions,
    assignment_lookup: &HashMap<(usize, usize), &SeatingAssignment>,
    table_number: usize,
) -> Vec<LayoutSeat> {
    match shape {
        TableShape::Round => {
            build_round_seats(seat_count, x, y, options, assignment_lookup, table_number)
        }
        TableShape::Rectangular | TableShape::Square => build_rectangular_seats(
            people_per_side.unwrap_or(&[]),
            seat_count,
            x,
            y,
            options,
            assignment_lookup,
            table_number,
        ),
    }
}

fn build_round_seats(
    seat_count: usize,
    x: f32,
    y: f32,
    options: &RenderOptions,
    assignment_lookup: &HashMap<(usize, usize), &SeatingAssignment>,
    table_number: usize,
) -> Vec<LayoutSeat> {
    let center_x = x + options.table_width / 2.0;
    let center_y = y + options.table_height / 2.0 + 6.0;
    let radius = 78.0;
    (0..seat_count)
        .map(|seat_index| {
            let angle = std::f32::consts::TAU * seat_index as f32 / seat_count.max(1) as f32
                - std::f32::consts::FRAC_PI_2;
            LayoutSeat {
                seat_index,
                x: center_x + radius * angle.cos(),
                y: center_y + radius * angle.sin(),
                person_name: assignment_lookup
                    .get(&(table_number, seat_index))
                    .map(|assignment| assignment.person_name.clone()),
            }
        })
        .collect()
}

fn build_rectangular_seats(
    people_per_side: &[usize],
    seat_count: usize,
    x: f32,
    y: f32,
    options: &RenderOptions,
    assignment_lookup: &HashMap<(usize, usize), &SeatingAssignment>,
    table_number: usize,
) -> Vec<LayoutSeat> {
    let mut points = Vec::new();
    let left = x + 52.0;
    let right = x + options.table_width - 52.0;
    let top = y + 68.0;
    let bottom = y + options.table_height - 58.0;
    let counts = if people_per_side.len() == 4 {
        people_per_side.to_vec()
    } else {
        spread_evenly(seat_count)
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
        .take(seat_count)
        .enumerate()
        .map(|(seat_index, (seat_x, seat_y))| LayoutSeat {
            seat_index,
            x: seat_x,
            y: seat_y,
            person_name: assignment_lookup
                .get(&(table_number, seat_index))
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
    }
}
