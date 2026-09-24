//! Guest-name label wrapping and placement, shared by SVG export and the GUI
//! canvas so both put the same lines in the same box.
//!
//! A name only ever breaks after whitespace or after a separator character
//! (see [`SEPARATORS`]); a piece between break points is never split and the
//! name is never truncated. When a name does not fit its box, the font
//! shrinks instead (greedy, deterministic), down to [`MIN_LABEL_FONT_SIZE`].

use super::{
    Bounds, LayoutSeat, LayoutTable, RenderOptions, TableSurface, card_size, header_bottom,
};

/// Smallest font size [`seat_label`] shrinks a label to. At this size a
/// label is drawn with whole pieces even if it spills out of its box.
pub const MIN_LABEL_FONT_SIZE: f32 = 6.0;

/// Line height as a multiple of the font size.
const LINE_HEIGHT: f32 = 1.2;
/// Factor the font shrinks by on each fitting attempt.
const SHRINK: f32 = 0.9;
/// Inset of the label area from the card edges.
const INSET: f32 = 4.0;
/// Gap between a seat marker's edge and its label box.
const GAP: f32 = 2.0;
/// Characters a name may break after; each stays at the end of its line.
const SEPARATORS: &[char] = &['-', '_', '/', '.', ',', '·', '–', '—'];

/// Horizontal alignment of a [`SeatLabel`]'s lines relative to its `x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelAlign {
    /// Lines start at `x` (label to the right of its seat).
    Start,
    /// Lines are centered on `x` (label above or below its seat).
    Center,
    /// Lines end at `x` (label to the left of its seat).
    End,
}

/// A guest-name label fitted to the box outward of its seat, in layout units.
#[derive(Debug, Clone, PartialEq)]
pub struct SeatLabel {
    /// Wrapped lines, top to bottom. Rejoining them (with a space where the
    /// name had whitespace) gives back the full name.
    pub lines: Vec<String>,
    /// Font size to draw every line at.
    pub font_size: f32,
    /// Distance between the tops of consecutive lines (`font_size * 1.2`).
    pub line_height: f32,
    /// Anchor X coordinate; see `align`.
    pub x: f32,
    /// Y coordinate of the first line's top.
    pub top: f32,
    /// How each line is aligned relative to `x`.
    pub align: LabelAlign,
}

/// Which side of its seat a label sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Above,
    Below,
    Left,
    Right,
}

/// Fit the guest label of `seat` (a seat of `table`, laid out with the same
/// `options`) into the box outward of the seat (see [`label_side`]).
/// `measure(text, font_size)` returns the drawn width of `text` in layout
/// units.
///
/// The box stays inside the card (inset by 4), below the header, and above
/// the table-area bottom (clear of the editor's free-seat rows). Its extent
/// along the row of labels is the distance to the nearest seat whose label
/// sits on the same side (see [`same_side_spacing`]):
/// - above/below the seat, it is that wide, centered on the seat;
/// - left/right of the seat, it runs to the card edge and is that tall.
///
/// Starts at `options.label_font_size` and shrinks by 10% per attempt until
/// the wrapped lines fit, stopping at [`MIN_LABEL_FONT_SIZE`]. Returns
/// `None` for a seat without a `person_name`.
pub fn seat_label(
    table: &LayoutTable,
    seat: &LayoutSeat,
    options: &RenderOptions,
    measure: impl Fn(&str, f32) -> f32,
) -> Option<SeatLabel> {
    let name = seat.person_name.as_deref()?;
    let inner = Bounds {
        left: table.x + INSET,
        top: header_bottom(table.y, options),
        right: table.x + table.width - INSET,
        // The table area ends above the editor's free-seat rows, and never
        // below the card itself (e.g. a layout built with other options).
        bottom: (table.y + card_size(options).1).min(table.y + table.height) - INSET,
    };
    let side = label_side(&table.surface, seat);
    let spacing = same_side_spacing(table, seat, side);
    let offset = options.seat_radius + GAP;

    let (lines, font_size, x, top, align) = if matches!(side, Side::Left | Side::Right) {
        let span = spacing.unwrap_or(inner.bottom - inner.top);
        let box_top = (seat.y - span / 2.0).max(inner.top);
        let box_bottom = (seat.y + span / 2.0).min(inner.bottom);
        let (x, width, align) = if side == Side::Right {
            let x = seat.x + offset;
            (x, inner.right - x, LabelAlign::Start)
        } else {
            let x = seat.x - offset;
            (x, x - inner.left, LabelAlign::End)
        };
        let (lines, font_size) = fit(
            name,
            width,
            box_bottom - box_top,
            options.label_font_size,
            &measure,
        );
        let block = lines.len() as f32 * LINE_HEIGHT * font_size;
        // Centered on the seat, but kept inside the box; a block that
        // spills (only at the minimum font) spills downward, never into
        // the header.
        let top = (seat.y - block / 2.0).min(box_bottom - block).max(box_top);
        (lines, font_size, x, top, align)
    } else {
        let span = spacing.unwrap_or(inner.right - inner.left);
        let half_width = (span / 2.0)
            .min(seat.x - inner.left)
            .min(inner.right - seat.x);
        let (box_top, box_bottom) = if side == Side::Below {
            (seat.y + offset, inner.bottom)
        } else {
            (inner.top, seat.y - offset)
        };
        let (lines, font_size) = fit(
            name,
            2.0 * half_width,
            box_bottom - box_top,
            options.label_font_size,
            &measure,
        );
        // Hug the seat: a label below starts at the box top, a label above
        // ends at the box bottom. A block too tall for the box (only at the
        // minimum font) spills downward, never into the header.
        let top = if side == Side::Below {
            box_top
        } else {
            (box_bottom - lines.len() as f32 * LINE_HEIGHT * font_size).max(box_top)
        };
        (lines, font_size, seat.x, top, LabelAlign::Center)
    };

    Some(SeatLabel {
        lines,
        font_size,
        line_height: LINE_HEIGHT * font_size,
        x,
        top,
        align,
    })
}

/// The side of `seat` its label sits on: away from the table center along
/// the axis where the seat is relatively furthest out, measured against the
/// surface's half extents (center and radius for a round or semicircle
/// table, whose ring center is the middle of a semicircle's flat edge). On a
/// rectangular table this is the side the seat is on, corner seats included.
fn label_side(surface: &TableSurface, seat: &LayoutSeat) -> Side {
    let (center_x, center_y, half_width, half_height) = match surface {
        TableSurface::Round { cx, cy, radius } | TableSurface::Semicircle { cx, cy, radius } => {
            (*cx, *cy, *radius, *radius)
        }
        TableSurface::Rect {
            x,
            y,
            width,
            height,
        } => (x + width / 2.0, y + height / 2.0, width / 2.0, height / 2.0),
    };
    let dx = (seat.x - center_x) / half_width;
    let dy = (seat.y - center_y) / half_height;
    if dx.abs() > dy.abs() {
        if dx >= 0.0 { Side::Right } else { Side::Left }
    } else if dy >= 0.0 {
        Side::Below
    } else {
        Side::Above
    }
}

/// Distance from `seat` to the nearest other seat of `table` whose label
/// sits on the same `side` — the neighbours its label could run into. Seats
/// labelled on another side (e.g. the end seat of a rectangle's adjacent
/// side) don't constrain it. A seat alone on its side falls back to its
/// nearest seat on any side, so its label stays in proportion to the table
/// instead of spanning the whole card. `None` for a lone seat.
fn same_side_spacing(table: &LayoutTable, seat: &LayoutSeat, side: Side) -> Option<f32> {
    let nearest = |same_side_only: bool| {
        table
            .seats
            .iter()
            .filter(|other| {
                other.seat_index != seat.seat_index
                    && (!same_side_only || label_side(&table.surface, other) == side)
            })
            .map(|other| (other.x - seat.x).hypot(other.y - seat.y))
            .min_by(f32::total_cmp)
    };
    nearest(true).or_else(|| nearest(false))
}

/// Wrap `name` into a `width` x `height` box, starting at `font_size` and
/// shrinking by [`SHRINK`] until the widest line fits and the lines fit the
/// height, or the font reaches [`MIN_LABEL_FONT_SIZE`] (where the lines are
/// returned as-is, spilling if they must). Greedy, not optimal: it takes the
/// first font size that fits, not the best-balanced wrap.
fn fit(
    name: &str,
    width: f32,
    height: f32,
    font_size: f32,
    measure: &impl Fn(&str, f32) -> f32,
) -> (Vec<String>, f32) {
    let mut font_size = font_size.max(MIN_LABEL_FONT_SIZE);
    loop {
        let lines = wrap(name, width, font_size, measure);
        let fits = lines.iter().all(|line| measure(line, font_size) <= width)
            && lines.len() as f32 * LINE_HEIGHT * font_size <= height;
        if fits || font_size <= MIN_LABEL_FONT_SIZE {
            return (lines, font_size);
        }
        font_size = (font_size * SHRINK).max(MIN_LABEL_FONT_SIZE);
    }
}

/// Greedily pack the unbreakable [`pieces`] of `name` into lines no wider
/// than `width` at `font_size`. A piece wider than `width` gets a line of its
/// own rather than being split. An empty name yields one empty line.
fn wrap(
    name: &str,
    width: f32,
    font_size: f32,
    measure: &impl Fn(&str, f32) -> f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut joiner = "";
    for (piece, spaced) in pieces(name) {
        if current.is_empty() {
            current = piece;
        } else {
            let candidate = format!("{current}{joiner}{piece}");
            if measure(&candidate, font_size) <= width {
                current = candidate;
            } else {
                lines.push(std::mem::replace(&mut current, piece));
            }
        }
        joiner = if spaced { " " } else { "" };
    }
    lines.push(current);
    lines
}

/// Split `name` into the pieces a label may break between, each paired with
/// whether whitespace followed it (rejoined as one space) or not (a piece
/// ending in a separator, rejoined with nothing).
fn pieces(name: &str) -> Vec<(String, bool)> {
    let mut pieces: Vec<(String, bool)> = Vec::new();
    let mut current = String::new();
    for c in name.chars() {
        if c.is_whitespace() {
            if !current.is_empty() {
                pieces.push((std::mem::take(&mut current), true));
            } else if let Some(last) = pieces.last_mut() {
                last.1 = true;
            }
        } else {
            current.push(c);
            if SEPARATORS.contains(&c) {
                pieces.push((std::mem::take(&mut current), false));
            }
        }
    }
    if !current.is_empty() {
        pieces.push((current, false));
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::super::build_layout;
    use super::*;
    use crate::build_table_type_map;
    use crate::models::{Person, ProjectInput, SeatingAssignment, TableShape, TableTypeConfig};

    /// The SVG renderer's width estimate: `0.55 * font_size` per character.
    fn estimate(text: &str, font_size: f32) -> f32 {
        text.chars().count() as f32 * 0.55 * font_size
    }

    #[test]
    fn wrap_breaks_after_separators() {
        assert_eq!(
            wrap("Montgomery-Featherstonehaugh", 60.0, 9.0, &estimate),
            vec!["Montgomery-", "Featherstonehaugh"]
        );
        assert_eq!(wrap("a_b", 5.0, 9.0, &estimate), vec!["a_", "b"]);
    }

    /// Wide enough for a whole piece but not two: one piece per line, never
    /// a piece split mid-word.
    #[test]
    fn wrap_breaks_on_word_boundaries() {
        assert_eq!(
            wrap("Ana Maria Lopez", 40.0, 9.0, &estimate),
            vec!["Ana", "Maria", "Lopez"]
        );
    }

    /// Rejoining the lines (ignoring whitespace dropped at breaks) gives back
    /// every character of the name — nothing dropped, no ellipsis.
    #[test]
    fn wrap_rejoin_preserves_all_characters() {
        let name = "Alexandria  Montgomery-Featherstonehaugh";
        let lines = wrap(name, 40.0, 9.0, &estimate);
        assert!(lines.len() >= 2);
        let rejoined: String = lines
            .concat()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let original: String = name.chars().filter(|c| !c.is_whitespace()).collect();
        assert_eq!(rejoined, original);
    }

    /// Multi-byte characters count as one each: "Ångström" is 8 chars (44
    /// wide at 10, fits 45) but 10 bytes (55, would not).
    #[test]
    fn wrap_handles_multi_byte_names() {
        assert_eq!(
            wrap("Núñez Ångström", 45.0, 10.0, &estimate),
            vec!["Núñez", "Ångström"]
        );
    }

    #[test]
    fn wrap_of_empty_name_returns_one_empty_line() {
        assert_eq!(wrap("", 90.0, 9.0, &estimate), vec![String::new()]);
    }

    /// "Featherstonehaugh" (17 chars) is 84.15 wide at 9 but the box is 70:
    /// the font shrinks until it fits, instead of splitting the word.
    #[test]
    fn fit_shrinks_font_instead_of_splitting() {
        let (lines, font_size) = fit("Featherstonehaugh", 70.0, 100.0, 9.0, &estimate);
        assert_eq!(lines, vec!["Featherstonehaugh"]);
        assert!(font_size < 9.0);
        assert!(estimate(&lines[0], font_size) <= 70.0);
    }

    /// A box nothing fits in: the font stops at the minimum and every piece
    /// stays whole, even though it spills.
    #[test]
    fn fit_stops_at_min_font_and_keeps_tokens_whole() {
        let (lines, font_size) = fit("Featherstonehaugh Montgomery", 10.0, 5.0, 9.0, &estimate);
        assert_eq!(font_size, MIN_LABEL_FONT_SIZE);
        assert_eq!(lines, vec!["Featherstonehaugh", "Montgomery"]);
    }

    /// One table of `shape` (with `people_per_side` for a rectangular one)
    /// seating `names` in order.
    fn one_table_project(
        shape: TableShape,
        people_per_side: Option<&[usize]>,
        names: &[&str],
    ) -> (ProjectInput, Vec<SeatingAssignment>) {
        let people: Vec<Person> = names
            .iter()
            .enumerate()
            .map(|(index, name)| Person {
                id: format!("p{index}"),
                name: name.to_string(),
                table_type: Some("t".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            })
            .collect();
        let assignments = people
            .iter()
            .enumerate()
            .map(|(seat_index, person)| SeatingAssignment {
                table_number: 1,
                table_type: "t".to_string(),
                seat_index,
                person_id: person.id.clone(),
                person_name: person.name.clone(),
            })
            .collect();
        let project = ProjectInput {
            people,
            closeness_rules: vec![],
            table_types: build_table_type_map(vec![(
                "t".to_string(),
                TableTypeConfig {
                    shape,
                    people_per_side: people_per_side.map(<[usize]>::to_vec),
                    max_people: names.len(),
                    recommended_people: None,
                    min_people: None,
                    number_of_tables: Some(1),
                },
            )])
            .unwrap(),
            table_order: Vec::new(),
        };
        (project, assignments)
    }

    /// Every guest label block of `shape` with `names` sits inside the card
    /// (inset by 4), below the header and above the table-area bottom; with
    /// `keep_font`, no label shrank below the requested size.
    fn assert_labels_inside(
        shape: TableShape,
        people_per_side: Option<&[usize]>,
        names: &[&str],
        options: &RenderOptions,
        keep_font: bool,
    ) {
        let (project, assignments) = one_table_project(shape.clone(), people_per_side, names);
        let layout = build_layout(&project, &assignments, options).unwrap();
        let table = &layout.tables[0];
        let header = header_bottom(table.y, options);
        let table_bottom = table.y + card_size(options).1;
        let tolerance = 1e-3;
        for seat in &table.seats {
            let label = seat_label(table, seat, options, estimate).unwrap();
            let context = format!("{shape:?} {:?} at {}", label.lines, options.label_font_size);
            let widest = label
                .lines
                .iter()
                .map(|line| estimate(line, label.font_size))
                .fold(0.0, f32::max);
            let (left, right) = match label.align {
                LabelAlign::Start => (label.x, label.x + widest),
                LabelAlign::Center => (label.x - widest / 2.0, label.x + widest / 2.0),
                LabelAlign::End => (label.x - widest, label.x),
            };
            let bottom = label.top + label.lines.len() as f32 * label.line_height;
            assert!(label.top >= header - tolerance, "{context} overlaps header");
            assert!(
                bottom <= table_bottom - INSET + tolerance,
                "{context} below table area"
            );
            assert!(
                left >= table.x + INSET - tolerance,
                "{context} left of card"
            );
            assert!(
                right <= table.x + table.width - INSET + tolerance,
                "{context} right of card"
            );
            if keep_font {
                assert_eq!(label.font_size, options.label_font_size, "{context} shrank");
            }
        }
    }

    /// Guest names from the user's screenshot, including a three-line one.
    const ROUND_9: [&str; 9] = [
        "Evan Mendiburt",
        "Maria Victoria Martinez",
        "Guillermo Cueva",
        "Elena Mendiburt",
        "Marius Ritisan",
        "Laura Martínez",
        "Joan pare",
        "Patricia Ritisan",
        "Maria Victoria Martinez",
    ];

    /// The default label size and a larger one.
    fn label_size_options() -> [RenderOptions; 2] {
        [RenderOptions::default().label_font_size, 14.0].map(|label_font_size| RenderOptions {
            label_font_size,
            ..RenderOptions::default()
        })
    }

    fn twelve_names() -> Vec<&'static str> {
        ROUND_9.iter().chain(&ROUND_9[..3]).copied().collect()
    }

    #[test]
    fn seat_labels_stay_inside_card_and_below_header() {
        let semicircle_6 = [
            "Joan pare",
            "Patricia Ritisan",
            "Guillermo Cueva",
            "Elena Mendiburt",
            "Marius Ritisan",
            "Laura Martínez",
        ];
        for options in label_size_options() {
            assert_labels_inside(TableShape::Round, None, &ROUND_9, &options, true);
            assert_labels_inside(TableShape::Semicircle, None, &semicircle_6, &options, true);
            for shape in [TableShape::Round, TableShape::Semicircle] {
                assert_labels_inside(shape, None, &twelve_names(), &options, false);
            }
        }
    }

    /// A name too long for the box above the top seat, even at the minimum
    /// font, spills downward toward its seat instead of up into the card
    /// title.
    #[test]
    fn overflowing_label_above_a_seat_never_climbs_into_the_header() {
        let mut names = twelve_names();
        names[0] = "Maria Victoria Martinez de la Fuente Rodriguez";
        let (project, assignments) = one_table_project(TableShape::Round, None, &names);
        let options = RenderOptions::default();
        let layout = build_layout(&project, &assignments, &options).unwrap();
        let table = &layout.tables[0];
        let top_seat = &table.seats[0];
        assert_eq!(label_side(&table.surface, top_seat), Side::Above);

        let label = seat_label(table, top_seat, &options, estimate).unwrap();

        assert_eq!(label.font_size, MIN_LABEL_FONT_SIZE);
        assert!(label.top >= header_bottom(table.y, &options));
    }

    /// On a rectangular table each label is sized from the seats along its
    /// own side, so the end seat of an adjacent side (pointing another way)
    /// doesn't squeeze it down to the minimum font: every label keeps the
    /// requested size, on every side, and stays inside the card.
    #[test]
    fn rect_seat_labels_keep_the_requested_font_on_every_side() {
        for options in label_size_options() {
            for people_per_side in [[3, 3, 3, 3], [4, 2, 4, 2]] {
                assert_labels_inside(
                    TableShape::Rectangular,
                    Some(&people_per_side),
                    &twelve_names(),
                    &options,
                    true,
                );
            }
        }
    }

    /// Every seat on one side of a rectangular table labels outward from that
    /// side, its end seats included.
    #[test]
    fn rect_seats_label_outward_from_their_own_side() {
        let (project, assignments) = one_table_project(
            TableShape::Rectangular,
            Some(&[3, 3, 3, 3]),
            &twelve_names(),
        );
        let layout = build_layout(&project, &assignments, &RenderOptions::default()).unwrap();
        let table = &layout.tables[0];
        let sides: Vec<Side> = table
            .seats
            .iter()
            .map(|seat| label_side(&table.surface, seat))
            .collect();
        // Seats walk top -> right -> bottom -> left, three per side.
        let expected = [Side::Above, Side::Right, Side::Below, Side::Left]
            .into_iter()
            .flat_map(|side| [side; 3])
            .collect::<Vec<_>>();
        assert_eq!(sides, expected);
    }
}
