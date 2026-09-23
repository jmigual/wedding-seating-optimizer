use seating_core::*;
use std::collections::BTreeMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn sample_tables_csv() -> &'static str {
    "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,4,2,2,\nrect_4,rectangular,4,4,2,1,1|1|1|1\n"
}

fn sample_people() -> Vec<Person> {
    vec![
        Person {
            id: "p1".to_string(),
            name: "Alice".to_string(),
            table_type: Some("round_4".to_string()),
            groups: vec!["family".to_string(), "friends".to_string()],
            locked_table: Some(1),
            locked_seat: Some(0),
        },
        Person {
            id: "p2".to_string(),
            name: "Bob".to_string(),
            table_type: None,
            groups: vec!["friends".to_string()],
            locked_table: None,
            locked_seat: None,
        },
    ]
}

/// Names/groups containing commas, quotes, and pipes — the characters CSV
/// quoting and the pipe-separated `groups` encoding must round-trip safely.
fn csv_hostile_people() -> Vec<Person> {
    vec![
        Person {
            id: "p1".to_string(),
            name: "O'Brien, \"Jay\"".to_string(),
            table_type: Some("round_4".to_string()),
            groups: vec!["family, close".to_string(), "friends \"inner\"".to_string()],
            locked_table: Some(1),
            locked_seat: Some(0),
        },
        Person {
            id: "p2".to_string(),
            name: "Bob \"The Builder\", Jr.".to_string(),
            table_type: None,
            groups: vec!["friends".to_string()],
            locked_table: None,
            locked_seat: None,
        },
    ]
}

fn sample_closeness_rules() -> Vec<ClosenessRule> {
    vec![
        ClosenessRule {
            left_id: "p1".to_string(),
            right_id: "p2".to_string(),
            score: 20.5,
        },
        ClosenessRule {
            left_id: "family".to_string(),
            right_id: "friends".to_string(),
            score: -2.0,
        },
    ]
}

fn sample_table_map() -> BTreeMap<TableTypeId, TableTypeConfig> {
    build_table_type_map(vec![
        (
            "round_4".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: Some(4),
                min_people: Some(2),
                number_of_tables: Some(1),
            },
        ),
        (
            "square_4".to_string(),
            TableTypeConfig {
                shape: TableShape::Square,
                people_per_side: Some(vec![1, 1, 1, 1]),
                max_people: 4,
                recommended_people: Some(4),
                min_people: Some(2),
                number_of_tables: Some(1),
            },
        ),
    ])
    .unwrap()
}

fn sample_project() -> ProjectInput {
    ProjectInput {
        people: sample_people(),
        closeness_rules: sample_closeness_rules(),
        table_types: sample_table_map(),
        table_order: Vec::new(),
    }
}

fn round_project() -> ProjectInput {
    ProjectInput {
        people: vec![
            Person {
                id: "p1".to_string(),
                name: "Alice".to_string(),
                table_type: Some("round_4".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p2".to_string(),
                name: "Bob".to_string(),
                table_type: Some("round_4".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p3".to_string(),
                name: "Cara".to_string(),
                table_type: Some("round_4".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p4".to_string(),
                name: "Dan".to_string(),
                table_type: Some("round_4".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
        ],
        closeness_rules: vec![],
        table_types: build_table_type_map(vec![(
            "round_4".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: Some(4),
                min_people: Some(2),
                number_of_tables: Some(1),
            },
        )])
        .unwrap(),
        table_order: Vec::new(),
    }
}

fn crowded_min_capacity_project() -> ProjectInput {
    let mut people = Vec::new();
    for index in 1..=6 {
        people.push(Person {
            id: format!("head_{index}"),
            name: format!("Head {index}"),
            table_type: Some("nupcial".to_string()),
            groups: Vec::new(),
            locked_table: None,
            locked_seat: None,
        });
    }
    for index in 1..=37 {
        people.push(Person {
            id: format!("guest_{index}"),
            name: format!("Guest {index}"),
            table_type: None,
            groups: Vec::new(),
            locked_table: None,
            locked_seat: None,
        });
    }

    let mut table_types = BTreeMap::new();
    table_types.insert(
        "nupcial".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 6,
            recommended_people: None,
            min_people: Some(4),
            number_of_tables: Some(1),
        },
    );
    table_types.insert(
        "rodona".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 10,
            recommended_people: None,
            min_people: Some(8),
            number_of_tables: None,
        },
    );

    ProjectInput {
        people,
        closeness_rules: Vec::new(),
        table_types,
        table_order: Vec::new(),
    }
}

fn round_assignments() -> Vec<SeatingAssignment> {
    vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "Alice".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "Bob".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 2,
            person_id: "p3".to_string(),
            person_name: "Cara".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 3,
            person_id: "p4".to_string(),
            person_name: "Dan".to_string(),
        },
    ]
}

fn square_project() -> ProjectInput {
    ProjectInput {
        people: vec![
            Person {
                id: "s1".to_string(),
                name: "North".to_string(),
                table_type: Some("square_4".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "s2".to_string(),
                name: "East".to_string(),
                table_type: Some("square_4".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "s3".to_string(),
                name: "South".to_string(),
                table_type: Some("square_4".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "s4".to_string(),
                name: "West".to_string(),
                table_type: Some("square_4".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
        ],
        closeness_rules: vec![],
        table_types: build_table_type_map(vec![(
            "square_4".to_string(),
            TableTypeConfig {
                shape: TableShape::Square,
                people_per_side: Some(vec![1, 1, 1, 1]),
                max_people: 4,
                recommended_people: Some(4),
                min_people: Some(2),
                number_of_tables: Some(1),
            },
        )])
        .unwrap(),
        table_order: Vec::new(),
    }
}

fn square_assignments() -> Vec<SeatingAssignment> {
    vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "square_4".to_string(),
            seat_index: 0,
            person_id: "s1".to_string(),
            person_name: "North".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "square_4".to_string(),
            seat_index: 1,
            person_id: "s2".to_string(),
            person_name: "East".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "square_4".to_string(),
            seat_index: 2,
            person_id: "s3".to_string(),
            person_name: "South".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "square_4".to_string(),
            seat_index: 3,
            person_id: "s4".to_string(),
            person_name: "West".to_string(),
        },
    ]
}

#[test]
fn people_csv_parsing_works() {
    let csv =
        "id,name,table_type,groups,locked_table,locked_seat\np1,Alice,round_4,family|friends,,\n";
    let people = parse_people_csv(csv).unwrap();
    assert_eq!(people.len(), 1);
    assert_eq!(people[0].groups, vec!["family", "friends"]);
}

#[test]
fn people_csv_dedupes_repeated_groups() {
    let csv =
        "id,name,table_type,groups,locked_table,locked_seat\np1,Alice,,family|family|friends,,\n";
    let people = parse_people_csv(csv).unwrap();
    assert_eq!(people[0].groups, vec!["family", "friends"]);
}

#[test]
fn structured_people_round_trip_csv_works() {
    let csv = write_people_csv(&sample_people()).unwrap();
    assert_eq!(parse_people_csv(&csv).unwrap(), sample_people());
}

#[test]
fn closeness_csv_parsing_works() {
    let csv = "left_id,right_id,score\np1,p2,20\nfamily,family,10\n";
    let closeness = parse_closeness_csv(csv).unwrap();
    assert_eq!(closeness.len(), 2);
    assert_eq!(closeness[0].score, 20.0);
}

#[test]
fn structured_closeness_round_trip_csv_works() {
    let csv = write_closeness_csv(&sample_closeness_rules()).unwrap();
    assert_eq!(parse_closeness_csv(&csv).unwrap(), sample_closeness_rules());
}

#[test]
fn tables_csv_parsing_works() {
    let tables = parse_tables_csv(sample_tables_csv()).unwrap();
    assert!(tables.contains_key("round_4"));
    assert_eq!(
        tables["rect_4"]
            .people_per_side
            .as_ref()
            .unwrap()
            .iter()
            .sum::<usize>(),
        4
    );
}

#[test]
fn structured_table_configs_round_trip_csv_works() {
    let csv = write_tables_csv(&sample_table_map()).unwrap();
    assert_eq!(parse_tables_csv(&csv).unwrap(), sample_table_map());
}

#[test]
fn csv_writers_emit_the_documented_headers() {
    assert_eq!(
        write_people_csv(&[]).unwrap().lines().next().unwrap(),
        PEOPLE_CSV_HEADER
    );
    assert_eq!(
        write_closeness_csv(&[]).unwrap().lines().next().unwrap(),
        CLOSENESS_CSV_HEADER
    );
    assert_eq!(
        write_tables_csv(&BTreeMap::new())
            .unwrap()
            .lines()
            .next()
            .unwrap(),
        TABLES_CSV_HEADER
    );
    assert_eq!(
        write_seating_csv(&[]).unwrap().lines().next().unwrap(),
        SEATING_CSV_HEADER
    );
}

#[test]
fn project_file_round_trip_works() {
    let project = sample_project();
    let project_file = ProjectFile::new(
        project.clone(),
        OptimizationConfig {
            seed: 77,
            attempts: 12,
            used_table_weight: 2.5,
            ..OptimizationConfig::default()
        },
        round_assignments(),
    );

    let json = write_project_file(&project_file).unwrap();
    let parsed = parse_project_file(&json).unwrap();

    assert_eq!(parsed.version, PROJECT_FILE_VERSION);
    assert_eq!(parsed.project_input().people, project.people);
    assert_eq!(
        parsed.project_input().closeness_rules,
        project.closeness_rules
    );
    assert_eq!(parsed.project_input().table_types, project.table_types);
    assert_eq!(parsed.optimization.seed, 77);
    assert_eq!(parsed.optimization.used_table_weight, 2.5);
    assert_eq!(parsed.seating, round_assignments());
}

#[test]
fn project_file_rejects_unsupported_version() {
    let json = r#"{
        "version": 999,
        "people": [],
        "closeness_rules": [],
        "table_types": {},
        "optimization": {},
        "seating": []
    }"#;

    let err = parse_project_file(json).unwrap_err();
    assert!(matches!(
        err,
        ValidationError::UnsupportedProjectVersion { found: 999, .. }
    ));
}

#[test]
fn project_file_preserves_csv_representations() {
    // Uses CSV-hostile names (commas, quotes) so this test actually exercises
    // csv-crate quoting/escaping, unlike a plain round-trip on alphanumeric
    // fixtures which every other round-trip test already covers.
    let people = csv_hostile_people();
    let project = make_project(
        &write_people_csv(&people).unwrap(),
        &write_closeness_csv(&sample_closeness_rules()).unwrap(),
        &write_tables_csv(&sample_table_map()).unwrap(),
    )
    .unwrap();
    let project_file = ProjectFile::new(project, OptimizationConfig::default(), Vec::new());
    let parsed = parse_project_file(&write_project_file(&project_file).unwrap()).unwrap();

    assert_eq!(
        parse_people_csv(&write_people_csv(&parsed.people).unwrap()).unwrap(),
        people
    );
    assert_eq!(
        parse_closeness_csv(&write_closeness_csv(&parsed.closeness_rules).unwrap()).unwrap(),
        sample_closeness_rules()
    );
    assert_eq!(
        parse_tables_csv(&write_tables_csv(&parsed.table_types).unwrap()).unwrap(),
        sample_table_map()
    );
}

#[test]
fn table_instance_generation_uses_configured_counts() {
    let project = ProjectInput {
        people: sample_people(),
        closeness_rules: vec![],
        table_types: sample_table_map(),
        table_order: Vec::new(),
    };
    let instances = generate_table_instances(&project);
    assert_eq!(instances.len(), 2);
    assert_eq!(instances[0].number, 1);
    assert_eq!(instances[0].table_type, "round_4");
    assert_eq!(instances[1].number, 2);
    assert_eq!(instances[1].table_type, "square_4");
}

#[test]
fn round_table_layout_generation_places_seats_circularly() {
    let layout = build_layout(&round_project(), &round_assignments()).unwrap();
    let table = &layout.tables[0];
    assert_eq!(table.shape, TableShape::Round);
    assert_eq!(table.seats.len(), 4);
    assert!(table.seats[0].y < table.seats[1].y);
    assert!(table.seats[0].y < table.seats[3].y);
    assert_eq!(table.seats[0].person_name.as_deref(), Some("Alice"));
}

/// Regression test: the canvas used to re-derive a round table's surface
/// center from the card rect instead of reusing the seat-ring center,
/// visibly offsetting the drawn table from its seats. `build_layout` must
/// expose one surface center that both the seat ring and the surface use.
#[test]
fn round_table_surface_center_matches_seat_ring_center() {
    let layout = build_layout(&round_project(), &round_assignments()).unwrap();
    let table = &layout.tables[0];
    let TableSurface::Round { cx, cy, .. } = &table.surface else {
        panic!("expected a round surface for a round table");
    };

    assert_eq!(table.seats.len(), 4);
    let distances: Vec<f32> = table
        .seats
        .iter()
        .map(|seat| ((seat.x - cx).powi(2) + (seat.y - cy).powi(2)).sqrt())
        .collect();
    let first = distances[0];
    for distance in &distances {
        assert!(
            (distance - first).abs() < 0.01,
            "seat distances from surface center should be equal: {distances:?}"
        );
    }
}

/// The seats of a regular n-gon are all equidistant from their two
/// neighbors and strictly farther from every other seat, so the smallest
/// center-to-center distance is exactly the chord length `2 * r * sin(pi/n)`
/// for the seat ring's radius `r`.
#[test]
fn min_seat_spacing_matches_the_regular_polygon_chord_length() {
    let layout = build_layout(&round_project(), &round_assignments()).unwrap();
    let table = &layout.tables[0];
    let TableSurface::Round { cx, cy, .. } = &table.surface else {
        panic!("expected a round surface for a round table");
    };

    let seat_count = table.seats.len();
    let seat_ring_radius =
        ((table.seats[0].x - cx).powi(2) + (table.seats[0].y - cy).powi(2)).sqrt();
    let expected_chord = 2.0 * seat_ring_radius * (std::f32::consts::PI / seat_count as f32).sin();

    let spacing = min_seat_spacing(&table.seats).unwrap();
    assert!(
        (spacing - expected_chord).abs() < 0.01,
        "expected chord length {expected_chord}, got {spacing}"
    );
}

#[test]
fn min_seat_spacing_is_none_below_two_seats() {
    let seat = LayoutSeat {
        seat_index: 0,
        x: 0.0,
        y: 0.0,
        person_name: None,
    };
    assert_eq!(min_seat_spacing(&[seat]), None);
    assert_eq!(min_seat_spacing(&[]), None);
}

#[test]
fn square_table_layout_generation_preserves_perimeter_order() {
    let layout = build_layout(&square_project(), &square_assignments()).unwrap();
    let table = &layout.tables[0];
    assert_eq!(table.shape, TableShape::Square);
    assert_eq!(table.seats.len(), 4);
    assert!(table.seats[0].y < table.seats[2].y);
    assert!(table.seats[1].x > table.seats[3].x);
    assert_eq!(table.seats[1].person_name.as_deref(), Some("East"));
}

fn semicircle_project() -> ProjectInput {
    let people = (1..=6)
        .map(|i| Person {
            id: format!("p{i}"),
            name: format!("Guest {i}"),
            table_type: Some("semi_6".to_string()),
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        })
        .collect();
    ProjectInput {
        people,
        closeness_rules: vec![],
        table_types: build_table_type_map(vec![(
            "semi_6".to_string(),
            TableTypeConfig {
                shape: TableShape::Semicircle,
                people_per_side: None,
                max_people: 6,
                recommended_people: None,
                min_people: None,
                number_of_tables: Some(1),
            },
        )])
        .unwrap(),
        table_order: Vec::new(),
    }
}

fn semicircle_assignments() -> Vec<SeatingAssignment> {
    (0..6)
        .map(|seat_index| SeatingAssignment {
            table_number: 1,
            table_type: "semi_6".to_string(),
            seat_index,
            person_id: format!("p{}", seat_index + 1),
            person_name: format!("Guest {}", seat_index + 1),
        })
        .collect()
}

#[test]
fn semicircle_layout_places_seats_on_the_arc_only() {
    let layout = build_layout(&semicircle_project(), &semicircle_assignments()).unwrap();
    let table = &layout.tables[0];
    assert_eq!(table.shape, TableShape::Semicircle);
    let TableSurface::Semicircle { cy, .. } = &table.surface else {
        panic!("expected a semicircle surface for a semicircle table");
    };

    assert_eq!(table.seats.len(), 6);
    for seat in &table.seats {
        assert!(
            seat.y < *cy,
            "seat {} should sit above the flat edge (y={}, cy={cy})",
            seat.seat_index,
            seat.y
        );
    }
    for pair in table.seats.windows(2) {
        assert!(
            pair[0].x < pair[1].x,
            "seat x should strictly increase with seat_index"
        );
    }

    let svg = render_svg(&layout, &RenderOptions::default());
    assert!(svg.contains("<path"));
}

/// Regression test: `build_semicircle_seats` used to reuse `round_table_metrics`
/// (sized for a full circle), so a semicircle's arc was half as tall as the
/// card allows and adjacent seat markers overlapped well before reaching a
/// realistic 10-12 person table.
#[test]
fn semicircle_seats_stay_separated_for_twelve_seats() {
    let people: Vec<Person> = (1..=12)
        .map(|i| Person {
            id: format!("p{i}"),
            name: format!("Guest {i}"),
            table_type: Some("semi_12".to_string()),
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        })
        .collect();
    let project = ProjectInput {
        people,
        closeness_rules: vec![],
        table_types: build_table_type_map(vec![(
            "semi_12".to_string(),
            TableTypeConfig {
                shape: TableShape::Semicircle,
                people_per_side: None,
                max_people: 12,
                recommended_people: None,
                min_people: None,
                number_of_tables: Some(1),
            },
        )])
        .unwrap(),
        table_order: Vec::new(),
    };
    let assignments: Vec<SeatingAssignment> = (0..12)
        .map(|seat_index| SeatingAssignment {
            table_number: 1,
            table_type: "semi_12".to_string(),
            seat_index,
            person_id: format!("p{}", seat_index + 1),
            person_name: format!("Guest {}", seat_index + 1),
        })
        .collect();

    let options = RenderOptions::default();
    let layout = build_layout(&project, &assignments).unwrap();
    let table = &layout.tables[0];
    assert_eq!(table.seats.len(), 12);

    for pair in table.seats.windows(2) {
        let dx = pair[1].x - pair[0].x;
        let dy = pair[1].y - pair[0].y;
        let distance = (dx * dx + dy * dy).sqrt();
        assert!(
            distance >= 2.0 * options.seat_radius,
            "adjacent semicircle seats should not overlap: distance={distance}, seat_radius={}",
            options.seat_radius
        );
    }
}

#[test]
fn svg_rendering_contains_table_labels_types_and_guest_names() {
    let layout = build_layout(&round_project(), &round_assignments()).unwrap();
    let svg = render_svg(&layout, &RenderOptions::default());
    assert!(svg.contains("Table 1 — round_4"));
    assert!(svg.contains("Shape: round"));
    assert!(svg.contains("Alice"));
    assert!(svg.contains("Dan"));
}

#[test]
fn layout_and_svg_skip_unused_tables_but_render_all_capacity_seats() {
    let project = ProjectInput {
        people: sample_people(),
        closeness_rules: vec![],
        table_types: sample_table_map(),
        table_order: Vec::new(),
    };
    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "Alice".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 3,
            person_id: "p2".to_string(),
            person_name: "Bob".to_string(),
        },
    ];

    let layout = build_layout(&project, &assignments).unwrap();
    assert_eq!(layout.tables.len(), 1);
    // round_4 has capacity 4: all four seat slots are rendered, not just the
    // two occupied ones (empty seats no longer vanish from the layout).
    assert_eq!(layout.tables[0].seats.len(), 4);
    let occupied = layout.tables[0]
        .seats
        .iter()
        .filter(|seat| seat.person_name.is_some())
        .count();
    assert_eq!(occupied, 2);

    let svg = render_svg(&layout, &RenderOptions::default());
    assert!(svg.contains("Alice"));
    assert!(svg.contains("Bob"));
    assert!(!svg.contains("Table 2"));
}

/// `ensure_spare_tables` only prevents an *unlimited* type from running
/// dry — it never caps a *limited* type, which materializes every one of
/// its `number_of_tables` instances up front (see
/// `generate_table_instances`) and so can genuinely have several empty at
/// once as guests move out. "At most one empty table per type" in the
/// editor is therefore `build_editor_layout`'s own display filter, not
/// something `ensure_spare_tables` provides. Here `b` (limited to 3
/// instances) has one occupied and two empty; only the lower-numbered
/// empty one (table 2) should render.
#[test]
fn editor_layout_shows_one_empty_table_per_type() {
    let table_types = build_table_type_map(vec![
        (
            "a".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: None,
                min_people: None,
                number_of_tables: Some(1),
            },
        ),
        (
            "b".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: None,
                min_people: None,
                number_of_tables: Some(3),
            },
        ),
    ])
    .unwrap();
    let project = ProjectInput {
        people: vec![
            Person {
                id: "p1".to_string(),
                name: "Alice".to_string(),
                table_type: None,
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p2".to_string(),
                name: "Bob".to_string(),
                table_type: None,
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
        ],
        closeness_rules: vec![],
        table_types,
        table_order: Vec::new(),
    };
    // a = #1 (occupied); b = #2 (occupied), #3, #4 (both empty).
    assert_eq!(
        instance_types(&project),
        vec![
            (1, "a".to_string()),
            (2, "b".to_string()),
            (3, "b".to_string()),
            (4, "b".to_string()),
        ]
    );
    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "a".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "Alice".to_string(),
        },
        SeatingAssignment {
            table_number: 2,
            table_type: "b".to_string(),
            seat_index: 0,
            person_id: "p2".to_string(),
            person_name: "Bob".to_string(),
        },
    ];

    let full_layout = build_editor_layout(&project, &assignments, true).unwrap();

    // Both used tables, plus exactly one empty table (the lower-numbered
    // of `b`'s two empty instances, table 3) — never table 4.
    let numbers: Vec<usize> = full_layout
        .tables
        .iter()
        .map(|table| table.table_number)
        .collect();
    assert_eq!(numbers, vec![1, 2, 3]);
}

#[test]
fn round_table_layout_uses_capacity_not_occupant_count_for_seat_angles() {
    let table_types = build_table_type_map(vec![(
        "round_6".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 6,
            recommended_people: Some(6),
            min_people: Some(1),
            number_of_tables: Some(1),
        },
    )])
    .unwrap();
    let project = ProjectInput {
        people: vec![
            Person {
                id: "p1".to_string(),
                name: "Alice".to_string(),
                table_type: Some("round_6".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p2".to_string(),
                name: "Bob".to_string(),
                table_type: Some("round_6".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p3".to_string(),
                name: "Cara".to_string(),
                table_type: Some("round_6".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
        ],
        closeness_rules: vec![],
        table_types,
        table_order: Vec::new(),
    };
    // Only 3 of the 6 seats are occupied, and not the first three indices —
    // this is exactly the partially-filled case the geometry must not skew.
    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_6".to_string(),
            seat_index: 1,
            person_id: "p1".to_string(),
            person_name: "Alice".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_6".to_string(),
            seat_index: 3,
            person_id: "p2".to_string(),
            person_name: "Bob".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_6".to_string(),
            seat_index: 5,
            person_id: "p3".to_string(),
            person_name: "Cara".to_string(),
        },
    ];

    let layout = build_layout(&project, &assignments).unwrap();
    let table = &layout.tables[0];
    assert_eq!(table.seats.len(), 6);
    let occupied = table
        .seats
        .iter()
        .filter(|seat| seat.person_name.is_some())
        .count();
    assert_eq!(occupied, 3);

    // Six seats evenly spaced around a circle have a centroid equal to the
    // circle's true center, regardless of occupancy — so this doesn't need
    // to know render.rs's internal radius/offset constants.
    let center_x = table.seats.iter().map(|seat| seat.x).sum::<f32>() / table.seats.len() as f32;
    let center_y = table.seats.iter().map(|seat| seat.y).sum::<f32>() / table.seats.len() as f32;

    for seat in &table.seats {
        let expected_angle =
            std::f32::consts::TAU * seat.seat_index as f32 / 6.0 - std::f32::consts::FRAC_PI_2;
        let actual_angle = (seat.y - center_y).atan2(seat.x - center_x);
        let mut delta = actual_angle - expected_angle;
        delta =
            (delta + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        assert!(
            delta.abs() < 1e-3,
            "seat {} angle mismatch: expected {expected_angle}, got {actual_angle}",
            seat.seat_index
        );
    }
}

#[test]
fn svg_wraps_long_guest_labels_without_ellipsis() {
    let project = round_project();
    let mut assignments = round_assignments();
    assignments[0].person_name = "Alexandria Montgomery-Featherstonehaugh".to_string();

    let layout = build_layout(&project, &assignments).unwrap();
    let svg = render_svg(&layout, &RenderOptions::default());

    let title = "<title>Alexandria Montgomery-Featherstonehaugh</title>";
    assert!(svg.contains(title));
    // The guest label must never be truncated with an ellipsis.
    assert!(!svg.contains('…'));

    // Isolate the <text class="guest"> element for this specific guest (the
    // one immediately following their <title>), rather than counting
    // <tspan>s across the whole SVG — every seat emits at least one <tspan>,
    // so that alone wouldn't prove this particular long name wrapped.
    let after_title = &svg[svg.find(title).unwrap() + title.len()..];
    let guest_text_start = after_title.find("<text class=\"guest\"").unwrap();
    let guest_text = &after_title[guest_text_start..];
    let guest_text_end = guest_text.find("</text>").unwrap() + "</text>".len();
    let guest_text = &guest_text[..guest_text_end];

    assert!(
        guest_text.matches("<tspan").count() >= 2,
        "expected the long name to wrap onto multiple <tspan> lines, got: {guest_text}"
    );

    // Every <tspan>'s content, concatenated, must reproduce the full name
    // (ignoring the whitespace introduced/removed at wrap points).
    let mut joined = String::new();
    let mut rest = guest_text;
    while let Some(open) = rest.find("<tspan") {
        let after_open = &rest[open..];
        let content_start = after_open.find('>').unwrap() + 1;
        let content_and_rest = &after_open[content_start..];
        let close = content_and_rest.find("</tspan>").unwrap();
        joined.push_str(&content_and_rest[..close]);
        rest = &content_and_rest[close + "</tspan>".len()..];
    }
    let joined: String = joined.chars().filter(|c| !c.is_whitespace()).collect();
    let expected: String = "Alexandria Montgomery-Featherstonehaugh"
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert_eq!(joined, expected);
}

#[test]
fn svg_respects_label_budget_floor_on_tightly_spaced_seats() {
    // 20 seats evenly spaced around the default round-table ring put
    // adjacent seats ~19px apart (2 * 61px radius * sin(pi/20)) — well
    // below the ~71px floor `render_svg` guarantees the label budget, so
    // the wrap must use the floor instead of the raw (tiny) spacing.
    let table_types = build_table_type_map(vec![(
        "round_20".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 20,
            recommended_people: Some(20),
            min_people: Some(1),
            number_of_tables: Some(1),
        },
    )])
    .unwrap();

    let mut people = Vec::new();
    let mut assignments = Vec::new();
    for seat_index in 0..20 {
        let id = format!("p{seat_index}");
        let name = if seat_index == 0 {
            "Alexandria Montgomery-Featherstonehaugh".to_string()
        } else {
            format!("Guest{seat_index}")
        };
        people.push(Person {
            id: id.clone(),
            name: name.clone(),
            table_type: Some("round_20".to_string()),
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        });
        assignments.push(SeatingAssignment {
            table_number: 1,
            table_type: "round_20".to_string(),
            seat_index,
            person_id: id,
            person_name: name,
        });
    }
    let project = ProjectInput {
        people,
        closeness_rules: vec![],
        table_types,
        table_order: Vec::new(),
    };

    let layout = build_layout(&project, &assignments).unwrap();
    let svg = render_svg(&layout, &RenderOptions::default());

    let title = "<title>Alexandria Montgomery-Featherstonehaugh</title>";
    assert!(svg.contains(title));
    let after_title = &svg[svg.find(title).unwrap() + title.len()..];
    let guest_text_start = after_title.find("<text class=\"guest\"").unwrap();
    let guest_text = &after_title[guest_text_start..];
    let guest_text_end = guest_text.find("</text>").unwrap() + "</text>".len();
    let guest_text = &guest_text[..guest_text_end];

    let first_tspan_start = guest_text.find("<tspan").unwrap();
    let first_tspan = &guest_text[first_tspan_start..];
    let content_start = first_tspan.find('>').unwrap() + 1;
    let content_end = first_tspan.find("</tspan>").unwrap();
    let first_line = &first_tspan[content_start..content_end];

    // On 8041d8a (no budget floor), the raw ~19px spacing would only fit
    // 2 characters ("Al"); with the floor this whole first word fits.
    assert_eq!(
        first_line, "Alexandria",
        "expected the floored label budget to fit the whole first word, got {first_line:?} in {guest_text}"
    );
}

#[test]
fn svg_height_grows_to_fit_wrapped_labels_without_clipping() {
    let project = round_project();
    let mut assignments = round_assignments();
    // Seat index 2 sits at the bottom of the ring, closest to the image
    // edge — its wrapped label lines are the ones a fixed layout.height
    // would clip.
    assignments[2].person_name = "Alexandria Montgomery-Featherstonehaugh".to_string();

    let layout = build_layout(&project, &assignments).unwrap();
    let svg = render_svg(&layout, &RenderOptions::default());

    let height_attr: f32 = svg
        .split("height=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .and_then(|value| value.parse().ok())
        .unwrap();

    assert!(
        height_attr > layout.height,
        "expected the SVG canvas ({height_attr}) to grow past the base layout height \
         ({}) to fit the wrapped label instead of clipping it",
        layout.height
    );

    // The canvas must reach all the way down to the wrapped label's lowest
    // baseline, not just be taller than layout.height by some margin.
    let title = "<title>Alexandria Montgomery-Featherstonehaugh</title>";
    let after_title = &svg[svg.find(title).unwrap() + title.len()..];
    let guest_text_start = after_title.find("<text class=\"guest\"").unwrap();
    let guest_text = &after_title[guest_text_start..];
    let guest_text_end = guest_text.find("</text>").unwrap() + "</text>".len();
    let guest_text = &guest_text[..guest_text_end];

    let text_y_start = guest_text.find("y=\"").unwrap() + "y=\"".len();
    let text_y_rest = &guest_text[text_y_start..];
    let base_y: f32 = text_y_rest[..text_y_rest.find('"').unwrap()]
        .parse()
        .unwrap();

    let mut last_baseline = base_y;
    let mut rest = guest_text;
    while let Some(open) = rest.find("<tspan") {
        let after_open = &rest[open..];
        let dy_start = after_open.find("dy=\"").unwrap() + "dy=\"".len();
        let dy_rest = &after_open[dy_start..];
        let dy: f32 = dy_rest[..dy_rest.find('"').unwrap()].parse().unwrap();
        last_baseline += dy;
        let close = after_open.find("</tspan>").unwrap();
        rest = &after_open[close + "</tspan>".len()..];
    }

    assert!(
        height_attr >= last_baseline,
        "expected the SVG canvas ({height_attr}) to reach the wrapped label's last \
         baseline ({last_baseline})"
    );
}

#[test]
fn png_rendering_includes_guest_text_when_fonts_are_loaded() {
    let occupied_seat = LayoutSeat {
        seat_index: 0,
        x: 144.0,
        y: 100.0,
        person_name: Some("Alice".to_string()),
    };
    let table = LayoutTable {
        table_number: 1,
        table_type: "round_4".to_string(),
        shape: TableShape::Round,
        x: 24.0,
        y: 24.0,
        width: 240.0,
        height: 220.0,
        seats: vec![occupied_seat],
        surface: TableSurface::Round {
            cx: 144.0,
            cy: 134.0,
            radius: 80.0,
        },
    };
    let layout_with_name = SeatingLayout {
        width: 288.0,
        height: 268.0,
        tables: vec![table],
    };
    let mut layout_without_name = layout_with_name.clone();
    layout_without_name.tables[0].seats[0].person_name = Some(String::new());

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let with_name_path = std::env::temp_dir().join(format!("wedding-seating-font-{stamp}-a.png"));
    let without_name_path =
        std::env::temp_dir().join(format!("wedding-seating-font-{stamp}-b.png"));

    render_png(
        &layout_with_name,
        &RenderOptions::default(),
        &with_name_path,
    )
    .unwrap();
    render_png(
        &layout_without_name,
        &RenderOptions::default(),
        &without_name_path,
    )
    .unwrap();

    let with_name_bytes = fs::read(&with_name_path).unwrap();
    let without_name_bytes = fs::read(&without_name_path).unwrap();
    assert_ne!(
        with_name_bytes, without_name_bytes,
        "PNG output did not change when the guest name text changed — fonts are not being loaded"
    );

    let _ = fs::remove_file(with_name_path);
    let _ = fs::remove_file(without_name_path);
}

#[test]
fn png_rendering_writes_a_file() {
    let layout = build_layout(&round_project(), &round_assignments()).unwrap();
    let output = std::env::temp_dir().join(format!(
        "wedding-seating-{}.png",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    render_png(&layout, &RenderOptions::default(), &output).unwrap();
    assert!(output.exists());
    assert!(fs::metadata(&output).unwrap().len() > 0);
    let _ = fs::remove_file(output);
}

#[test]
fn invalid_people_per_side_validation_is_reported() {
    let project = ProjectInput {
        people: vec![],
        closeness_rules: vec![],
        table_types: build_table_type_map(vec![(
            "bad_rect".to_string(),
            TableTypeConfig {
                shape: TableShape::Rectangular,
                people_per_side: Some(vec![2, 2, 2]),
                max_people: 8,
                recommended_people: Some(6),
                min_people: Some(4),
                number_of_tables: Some(1),
            },
        )])
        .unwrap(),
        table_order: Vec::new(),
    };
    let report = validate_project(&project).unwrap_err();
    assert!(report.errors.iter().any(|error| {
        matches!(
            error,
            ValidationError::InvalidPeoplePerSideLength { table_type, len }
                if table_type == "bad_rect" && *len == 3
        )
    }));
}

#[test]
fn locked_table_and_seat_validation_after_gui_style_edits_is_reported() {
    let project = ProjectInput {
        people: vec![Person {
            id: "p1".to_string(),
            name: "Alice".to_string(),
            table_type: Some("round_4".to_string()),
            groups: vec![],
            locked_table: None,
            locked_seat: Some(2),
        }],
        closeness_rules: vec![],
        table_types: build_table_type_map(vec![(
            "round_4".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: Some(4),
                min_people: Some(2),
                number_of_tables: Some(1),
            },
        )])
        .unwrap(),
        table_order: Vec::new(),
    };
    let report = validate_project(&project).unwrap_err();
    assert!(report.errors.iter().any(|error| {
        matches!(error, ValidationError::LockedSeatRequiresLockedTable(id) if id == "p1")
    }));
}

#[test]
fn id_namespace_collision_is_rejected() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,Alice,,p1,,\n",
        "left_id,right_id,score\n",
        sample_tables_csv(),
    )
    .unwrap();
    let err = validate_project(&project).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::NamespaceCollision(id) if id == "p1"))
    );
}

#[test]
fn group_pair_scores_apply() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\na1,A1,,family,,\na2,A2,,family,,\n",
        "left_id,right_id,score\nfamily,family,10\n",
        sample_tables_csv(),
    )
    .unwrap();
    let score =
        effective_person_pair_score(&project, &project.people[0], &project.people[1]).unwrap();
    assert_eq!(score, 10.0);
}

#[test]
fn multiple_group_rules_are_summed() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\na1,A1,,g1|g2,,\na2,A2,,g3|g4,,\n",
        "left_id,right_id,score\ng1,g3,5\ng2,g4,9\n",
        sample_tables_csv(),
    )
    .unwrap();
    let score =
        effective_person_pair_score(&project, &project.people[0], &project.people[1]).unwrap();
    assert_eq!(score, 14.0);
}

#[test]
fn overlapping_group_rules_are_summed() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,P1,,town|church|family,,\np2,P2,,town|church|family,,\np3,P3,,town|church,,\n",
        "left_id,right_id,score\ntown,town,4\nchurch,church,5\nfamily,family,5\n",
        sample_tables_csv(),
    )
    .unwrap();
    let score_p1_p2 =
        effective_person_pair_score(&project, &project.people[0], &project.people[1]).unwrap();
    assert_eq!(score_p1_p2, 14.0);
    let score_p1_p3 =
        effective_person_pair_score(&project, &project.people[0], &project.people[2]).unwrap();
    assert_eq!(score_p1_p3, 9.0);

    // A negative direct rule nets against the summed group rules.
    let project_with_direct = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,P1,,town|church|family,,\np2,P2,,town|church|family,,\np3,P3,,town|church,,\n",
        "left_id,right_id,score\ntown,town,4\nchurch,church,5\nfamily,family,5\np1,p3,-10\n",
        sample_tables_csv(),
    )
    .unwrap();
    let score_p1_p3_negative = effective_person_pair_score(
        &project_with_direct,
        &project_with_direct.people[0],
        &project_with_direct.people[2],
    )
    .unwrap();
    assert_eq!(score_p1_p3_negative, -1.0);
}

#[test]
fn person_pair_score_adds_to_group_score() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\na1,A1,,fam,,\na2,A2,,fam,,\n",
        "left_id,right_id,score\na1,a2,5\nfam,fam,10\n",
        sample_tables_csv(),
    )
    .unwrap();
    let score =
        effective_person_pair_score(&project, &project.people[0], &project.people[1]).unwrap();
    assert_eq!(score, 15.0);
}

#[test]
fn cross_group_rule_counts_once_for_overlapping_membership() {
    // Both people belong to both G and H, so the cross-group rule `G,H,7`
    // must be counted once, not once per (G,H)/(H,G) enumeration order.
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,P1,,G|H,,\np2,P2,,G|H,,\n",
        "left_id,right_id,score\nG,H,7\n",
        sample_tables_csv(),
    )
    .unwrap();
    let score =
        effective_person_pair_score(&project, &project.people[0], &project.people[1]).unwrap();
    assert_eq!(score, 7.0);

    // Asymmetric membership: only one orientation of the cross-group rule is
    // enumerable, plus the shared group's self-rule, both counted once.
    let project_asymmetric = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,P1,,G|H,,\np2,P2,,H,,\n",
        "left_id,right_id,score\nG,H,7\nH,H,2\n",
        sample_tables_csv(),
    )
    .unwrap();
    let score_asymmetric = effective_person_pair_score(
        &project_asymmetric,
        &project_asymmetric.people[0],
        &project_asymmetric.people[1],
    )
    .unwrap();
    assert_eq!(score_asymmetric, 9.0);
}

#[test]
fn round_distance_is_circular() {
    assert_eq!(circular_distance(0, 1, 5), 1);
    assert_eq!(circular_distance(0, 4, 5), 1);
    assert_eq!(circular_distance(0, 3, 5), 2);
}

#[test]
fn perimeter_distance_is_circular_default() {
    assert_eq!(perimeter_distance(0, 3, 8), 3);
    assert_eq!(perimeter_distance(0, 7, 8), 1);
}

#[test]
fn semicircle_distance_is_linear_along_the_arc() {
    assert_eq!(linear_distance(0, 7), 7);
    assert_eq!(seat_distance(&TableShape::Semicircle, 0, 7, 8), 7);
    assert_eq!(seat_distance(&TableShape::Round, 0, 7, 8), 1);
}

#[test]
fn capacity_validation_catches_insufficient_space() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,g,,\np2,B,,g,,\np3,C,,g,,\np4,D,,g,,\np5,E,,g,,\n",
        "left_id,right_id,score\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,,,1,\n",
    )
    .unwrap();
    let err = validate_project(&project).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::NotEnoughSeats { .. }))
    );
}

#[test]
fn locked_table_validation_works() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,g,999,\n",
        "left_id,right_id,score\n",
        sample_tables_csv(),
    )
    .unwrap();
    let err = validate_project(&project).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::LockedTableDoesNotExist { .. }))
    );
}

#[test]
fn locked_seat_validation_works() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,round_4,g,1,99\n",
        "left_id,right_id,score\n",
        sample_tables_csv(),
    )
    .unwrap();
    let err = validate_project(&project).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::LockedSeatOutOfRange { .. }))
    );
}

#[test]
fn impossible_assignment_is_detected() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,missing,g,,\n",
        "left_id,right_id,score\n",
        sample_tables_csv(),
    )
    .unwrap();
    let err = validate_project(&project).unwrap_err();
    assert!(err.errors.iter().any(|e| {
        matches!(
            e,
            ValidationError::UnknownTableTypeForPerson {
                person_id,
                table_type
            } if person_id == "p1" && table_type == "missing"
        )
    }));
}

#[test]
fn known_seating_scoring_works() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,g,,\np2,B,,g,,\np3,C,,x,,\n",
        "left_id,right_id,score\np1,p2,10\ng,g,5\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,3,,1,\n",
    )
    .unwrap();

    validate_project(&project).unwrap();
    let seating = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 2,
            person_id: "p3".to_string(),
            person_name: "C".to_string(),
        },
    ];

    let score = score_solution(&project, &seating, &OptimizationConfig::default()).unwrap();
    assert!(score > 10.0);

    let no_proximity_score = score_solution(
        &project,
        &seating,
        &OptimizationConfig {
            proximity_weight: 0.0,
            ..OptimizationConfig::default()
        },
    )
    .unwrap();
    assert!(score > no_proximity_score);
}

#[test]
fn integration_style_optimization_test() {
    // 2 tables of 2 (rather than 1 table of 4): every feasible solution on a
    // single table puts all 4 people together regardless of scoring, so that
    // fixture cannot distinguish a working optimizer from one that ignores
    // closeness entirely. With two tables, the strong same-group score (15)
    // vs. cross-group penalty (-2) has an actual outcome to get right: p1/p2
    // must end up co-tabled, and so must p3/p4.
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,fam,,\np2,B,,fam,,\np3,C,,work,,\np4,D,,work,,\n",
        "left_id,right_id,score\np1,p2,15\np3,p4,15\nfam,work,-2\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_2,round,2,2,,2,\n",
    )
    .unwrap();

    validate_project(&project).unwrap();
    let optimizer = HeuristicOptimizer;
    let result = optimizer
        .optimize(
            &project,
            &OptimizationConfig {
                seed: 1234,
                attempts: 20,
                steps: 200,
                solutions: 1,
                optimal_table_size_weight: 0.5,
                ..OptimizationConfig::default()
            },
        )
        .unwrap();

    assert_eq!(result.solutions.len(), 1);
    let assignments = &result.solutions[0].assignments;
    assert_eq!(assignments.len(), 4);
    assert!(result.solutions[0].score.is_finite());

    let table_of = |id: &str| {
        assignments
            .iter()
            .find(|a| a.person_id == id)
            .unwrap()
            .table_number
    };
    assert_eq!(table_of("p1"), table_of("p2"));
    assert_eq!(table_of("p3"), table_of("p4"));
    assert_ne!(table_of("p1"), table_of("p3"));
}

#[test]
fn used_table_penalty_prefers_needed_table_count() {
    let people_csv = (1..=8)
        .map(|index| format!("p{index},Person {index},round_2,,,\n"))
        .fold(
            "id,name,table_type,groups,locked_table,locked_seat\n".to_string(),
            |mut csv, row| {
                csv.push_str(&row);
                csv
            },
        );
    let project = make_project(
        &people_csv,
        "left_id,right_id,score\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_2,round,2,2,,5,\n",
    )
    .unwrap();
    let result = HeuristicOptimizer
        .optimize(
            &project,
            &OptimizationConfig {
                seed: 9,
                attempts: 30,
                steps: 800,
                solutions: 1,
                used_table_weight: 10.0,
                optimal_table_size_weight: 2.0,
                ..OptimizationConfig::default()
            },
        )
        .unwrap();
    let used_tables = result.solutions[0]
        .assignments
        .iter()
        .map(|assignment| assignment.table_number)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(used_tables.len(), 4);
}

// ── Determinism ────────────────────────────────────────────────────────────

#[test]
fn same_seed_produces_identical_solutions() {
    let project = round_project();
    let config = OptimizationConfig {
        seed: 4242,
        attempts: 8,
        steps: 100,
        solutions: 1,
        ..OptimizationConfig::default()
    };

    let result1 = HeuristicOptimizer.optimize(&project, &config).unwrap();
    let result2 = HeuristicOptimizer.optimize(&project, &config).unwrap();

    // Exact equality (not just equal scores): same seed + same input must
    // produce bitwise-identical assignments and scores every time. This is
    // the load-bearing check for CLAUDE.md's determinism contract — a
    // `thread_rng()` slip or a `HashMap`-iteration-order-dependent sum would
    // fail this test even though it might still "look right" on one run.
    assert_eq!(result1.solutions, result2.solutions);
}

// ── Locked guests ────────────────────────────────────────────────────────

#[test]
fn optimizer_honors_locked_table_and_seat() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,,1,2\np2,B,,,,\np3,C,,,,\np4,D,,,,\n",
        "left_id,right_id,score\np1,p3,50\np1,p4,50\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,,,2,\n",
    )
    .unwrap();

    let result = HeuristicOptimizer
        .optimize(
            &project,
            &OptimizationConfig {
                seed: 7,
                attempts: 15,
                steps: 150,
                solutions: 1,
                ..OptimizationConfig::default()
            },
        )
        .unwrap();

    let solution = &result.solutions[0];
    let p1 = solution
        .assignments
        .iter()
        .find(|a| a.person_id == "p1")
        .unwrap();
    assert_eq!(p1.table_number, 1);
    assert_eq!(p1.seat_index, 2);
    assert!(validate_seating_solution(&project, &solution.assignments).is_ok());
}

// ── validate_seating_solution invariants ─────────────────────────────────

#[test]
fn seat_collision_is_detected() {
    let project = round_project();
    let mut assignments = round_assignments();
    assignments[1].seat_index = assignments[0].seat_index; // p1 and p2 both at seat 0
    let err = validate_seating_solution(&project, &assignments).unwrap_err();
    assert!(err.errors.iter().any(|e| matches!(
        e,
        ValidationError::SeatCollision {
            table_number: 1,
            seat: 0
        }
    )));
}

#[test]
fn missing_and_duplicate_person_is_detected() {
    let project = round_project();
    let mut assignments = round_assignments();
    // p4's slot is overwritten with a second p1 entry: p1 now appears twice
    // (duplicate) and p4 never appears (missing) — both directions of the
    // same invariant in one solution.
    assignments[3].person_id = "p1".to_string();
    assignments[3].person_name = "Alice".to_string();
    let err = validate_seating_solution(&project, &assignments).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingOrDuplicatePerson(id) if id == "p1"))
    );
    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingOrDuplicatePerson(id) if id == "p4"))
    );
}

#[test]
fn capacity_exceeded_is_detected() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,,,\np2,B,,,,\np3,C,,,,\n",
        "left_id,right_id,score\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_2,round,2,,,2,\n",
    )
    .unwrap();
    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_2".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_2".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_2".to_string(),
            seat_index: 1,
            person_id: "p3".to_string(),
            person_name: "C".to_string(),
        },
    ];
    let err = validate_seating_solution(&project, &assignments).unwrap_err();
    assert!(err.errors.iter().any(|e| matches!(
        e,
        ValidationError::TableCapacityExceeded {
            table_number: 1,
            count: 3,
            capacity: 2
        }
    )));
}

/// Fixture reused by [`min_people_shortfall_is_penalized_not_rejected`] and
/// [`apply_seat_drop_allows_dropping_onto_an_empty_table_below_min`]: 3
/// people, one `round_4` type (min 2) with 2 instances.
fn min_shortfall_project() -> ProjectInput {
    make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,,,\np2,B,,,,\np3,C,,,,\n",
        "left_id,right_id,score\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,,2,2,\n",
    )
    .unwrap()
}

#[test]
fn min_people_shortfall_is_penalized_not_rejected() {
    let project = min_shortfall_project();
    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
        SeatingAssignment {
            table_number: 2,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p3".to_string(),
            person_name: "C".to_string(),
        },
    ];
    validate_seating_solution(&project, &assignments).unwrap();

    let breakdown =
        score_solution_breakdown(&project, &assignments, &OptimizationConfig::default()).unwrap();
    // No closeness rules (proximity = 0), no recommended_people (size_penalty
    // = 0), default used_table_weight = 0 (used_table_penalty = 0). Table 2
    // has 1 guest against min_people = 2, so the shortfall of 1 is penalized
    // at the default min_people_weight of 1000.0.
    assert_eq!(breakdown.min_people_penalty, 1000.0);
    assert_eq!(
        breakdown.total,
        breakdown.proximity
            - breakdown.used_table_penalty
            - breakdown.size_penalty
            - breakdown.min_people_penalty
    );
    assert_eq!(breakdown.total, -1000.0);
}

#[test]
fn seating_violating_locked_table_is_detected() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,,1,\np2,B,,,,\n",
        "left_id,right_id,score\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,,,2,\n",
    )
    .unwrap();
    let assignments = vec![
        SeatingAssignment {
            table_number: 2,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
    ];
    let err = validate_seating_solution(&project, &assignments).unwrap_err();
    assert!(err.errors.iter().any(|e| matches!(
        e,
        ValidationError::SeatingViolatesLockedTable {
            person_id,
            locked_table: 1,
            assigned_table: 2,
        } if person_id == "p1"
    )));
}

// ── Scoring ──────────────────────────────────────────────────────────────

#[test]
fn adjacent_seating_scores_higher_than_distant() {
    let table_types = build_table_type_map(vec![(
        "round_6".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 6,
            recommended_people: None,
            min_people: None,
            number_of_tables: Some(1),
        },
    )])
    .unwrap();
    // Six guests fill all six seats (no empty seats), so each guest's rank
    // among occupied seats equals its raw seat index — this fixture is about
    // the distance rule for a fully occupied table, not the empty-seat rule.
    let people: Vec<Person> = (1..=6)
        .map(|i| Person {
            id: format!("p{i}"),
            name: format!("Guest {i}"),
            table_type: Some("round_6".to_string()),
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        })
        .collect();
    let project = ProjectInput {
        people,
        closeness_rules: vec![ClosenessRule {
            left_id: "p1".to_string(),
            right_id: "p2".to_string(),
            score: 10.0,
        }],
        table_types,
        table_order: Vec::new(),
    };
    let config = OptimizationConfig::default();

    let seat_assignment = |seat_indices: [usize; 6]| -> Vec<SeatingAssignment> {
        (1..=6)
            .zip(seat_indices)
            .map(|(i, seat_index)| SeatingAssignment {
                table_number: 1,
                table_type: "round_6".to_string(),
                seat_index,
                person_id: format!("p{i}"),
                person_name: format!("Guest {i}"),
            })
            .collect()
    };

    // p1 at seat 0, p2 at seat 1: adjacent.
    let adjacent = seat_assignment([0, 1, 2, 3, 4, 5]);
    // p1 at seat 0, p2 at seat 3: three seats apart, still every seat filled.
    let distant = seat_assignment([0, 3, 1, 2, 4, 5]);

    let adjacent_score = score_solution(&project, &adjacent, &config).unwrap();
    let distant_score = score_solution(&project, &distant, &config).unwrap();
    assert!(adjacent_score > distant_score);
}

#[test]
fn default_proximity_weight_profile() {
    assert_eq!(default_proximity_weight(0), 1.0);
    assert_eq!(default_proximity_weight(1), 1.0);
    assert_eq!(default_proximity_weight(2), 0.75);
    assert_eq!(default_proximity_weight(3), 0.5);
    assert_eq!(default_proximity_weight(4), 0.25);
    assert_eq!(default_proximity_weight(5), 0.2);
}

#[test]
fn used_table_and_size_penalties_apply() {
    let table_types = build_table_type_map(vec![(
        "round_4".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 4,
            recommended_people: Some(2),
            min_people: None,
            number_of_tables: Some(1),
        },
    )])
    .unwrap();
    let project = ProjectInput {
        people: vec![
            Person {
                id: "p1".to_string(),
                name: "A".to_string(),
                table_type: None,
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p2".to_string(),
                name: "B".to_string(),
                table_type: None,
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p3".to_string(),
                name: "C".to_string(),
                table_type: None,
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
        ],
        closeness_rules: vec![ClosenessRule {
            left_id: "p1".to_string(),
            right_id: "p2".to_string(),
            score: 5.0,
        }],
        table_types,
        table_order: Vec::new(),
    };
    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 2,
            person_id: "p3".to_string(),
            person_name: "C".to_string(),
        },
    ];
    let config = OptimizationConfig {
        used_table_weight: 3.0,
        optimal_table_size_weight: 2.0,
        ..OptimizationConfig::default()
    };

    // Hand-computed expectation:
    //   p1-p2 (distance 1, weight 1.0): 5.0 * 1.0 * 1.0 = 5.0
    //   p1-p3 (distance 2, weight 0.75) and p2-p3 (distance 1): no rule, 0.0
    //   used_table_weight: 1 table * 3.0 = -3.0
    //   size penalty: |3 - 2| * 2.0 = -2.0
    //   total: 5.0 - 3.0 - 2.0 = 0.0
    let score = score_solution(&project, &assignments, &config).unwrap();
    assert!((score - 0.0).abs() < 1e-9, "expected 0.0, got {score}");
}

/// Shared fixture for score breakdown tests: one round table (max 4,
/// recommended 2) seating 3 people, with a single closeness rule between two
/// adjacent seats. Mirrors `used_table_and_size_penalties_apply` above.
fn breakdown_fixture() -> (ProjectInput, Vec<SeatingAssignment>, OptimizationConfig) {
    let table_types = build_table_type_map(vec![(
        "round_4".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 4,
            recommended_people: Some(2),
            min_people: None,
            number_of_tables: Some(1),
        },
    )])
    .unwrap();
    let project = ProjectInput {
        people: vec![
            Person {
                id: "p1".to_string(),
                name: "A".to_string(),
                table_type: None,
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p2".to_string(),
                name: "B".to_string(),
                table_type: None,
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p3".to_string(),
                name: "C".to_string(),
                table_type: None,
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
        ],
        closeness_rules: vec![ClosenessRule {
            left_id: "p1".to_string(),
            right_id: "p2".to_string(),
            score: 5.0,
        }],
        table_types,
        table_order: Vec::new(),
    };
    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 2,
            person_id: "p3".to_string(),
            person_name: "C".to_string(),
        },
    ];
    let config = OptimizationConfig {
        used_table_weight: 3.0,
        optimal_table_size_weight: 2.0,
        ..OptimizationConfig::default()
    };
    (project, assignments, config)
}

#[test]
fn score_solution_breakdown_total_matches_score_solution() {
    let (project, assignments, config) = breakdown_fixture();
    let breakdown = score_solution_breakdown(&project, &assignments, &config).unwrap();
    let score = score_solution(&project, &assignments, &config).unwrap();
    assert_eq!(breakdown.total, score);
}

#[test]
fn score_solution_breakdown_reports_expected_components() {
    let (project, assignments, config) = breakdown_fixture();
    let breakdown = score_solution_breakdown(&project, &assignments, &config).unwrap();
    // Same hand-computed expectation as `used_table_and_size_penalties_apply`:
    //   proximity: p1-p2 (distance 1, weight 1.0) => 5.0 * 1.0 * 1.0 = 5.0
    //   used_table_penalty: 1 table * 3.0 = 3.0
    //   size_penalty: |3 - 2| * 2.0 = 2.0
    assert!((breakdown.proximity - 5.0).abs() < 1e-9);
    assert!((breakdown.used_table_penalty - 3.0).abs() < 1e-9);
    assert!((breakdown.size_penalty - 2.0).abs() < 1e-9);
    assert!((breakdown.total - 0.0).abs() < 1e-9);
    assert!(
        (breakdown.total
            - (breakdown.proximity
                - breakdown.used_table_penalty
                - breakdown.size_penalty
                - breakdown.min_people_penalty))
            .abs()
            < 1e-9
    );
}

#[test]
fn semicircle_scores_arc_ends_as_far_not_adjacent() {
    // All four seats filled, so ranks equal raw seat indices: this fixture
    // is about the arc's no-wrap rule for a fully occupied table, not the
    // empty-seat rule (see the 2-guest case below for that).
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,,,\np2,B,,,,\np3,C,,,,\np4,D,,,,\n",
        "left_id,right_id,score\np1,p2,10\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nsemi_4,semicircle,4,,,1,\n",
    )
    .unwrap();
    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "semi_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "semi_4".to_string(),
            seat_index: 1,
            person_id: "p3".to_string(),
            person_name: "C".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "semi_4".to_string(),
            seat_index: 2,
            person_id: "p4".to_string(),
            person_name: "D".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "semi_4".to_string(),
            seat_index: 3,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
    ];
    let breakdown =
        score_solution_breakdown(&project, &assignments, &OptimizationConfig::default()).unwrap();
    // On a round table, seats 0 and 3 of 4 are adjacent (circular_distance = 1,
    // weight 1.0, contribution 10.0); on a semicircle the arc doesn't wrap, so
    // the two ends are the farthest apart (linear_distance = 3, weight 0.5).
    assert_eq!(breakdown.proximity, 10.0 * default_proximity_weight(3));
    assert_eq!(breakdown.proximity, 5.0);
}

/// Empty-seat rule for a semicircle: with only 2 of 4 seats occupied, the two
/// guests are tablemates 0 and 1 (of 2 occupied) — adjacent, not two raw
/// seats apart — even though the arc itself never wraps.
#[test]
fn semicircle_scores_two_guests_with_empty_seats_between_as_adjacent() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,,,\np2,B,,,,\n",
        "left_id,right_id,score\np1,p2,10\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nsemi_4,semicircle,4,,,1,\n",
    )
    .unwrap();
    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "semi_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "semi_4".to_string(),
            seat_index: 3,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
    ];
    let breakdown =
        score_solution_breakdown(&project, &assignments, &OptimizationConfig::default()).unwrap();
    assert_eq!(breakdown.proximity, 10.0);
}

// ── min_people through the optimizer ─────────────────────────────────────

#[test]
fn optimizer_respects_min_people() {
    let people_csv = (1..=6)
        .map(|index| format!("p{index},Person {index},,,,\n"))
        .fold(
            "id,name,table_type,groups,locked_table,locked_seat\n".to_string(),
            |mut csv, row| {
                csv.push_str(&row);
                csv
            },
        );
    let project = make_project(
        &people_csv,
        "left_id,right_id,score\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,,3,3,\n",
    )
    .unwrap();

    let result = HeuristicOptimizer
        .optimize(
            &project,
            &OptimizationConfig {
                seed: 5,
                attempts: 20,
                steps: 300,
                solutions: 1,
                ..OptimizationConfig::default()
            },
        )
        .unwrap();

    let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
    for assignment in &result.solutions[0].assignments {
        *counts.entry(assignment.table_number).or_insert(0) += 1;
    }
    for count in counts.values() {
        assert!(*count >= 3, "table below min_people: {count}");
    }
}

#[test]
fn optimizer_minimizes_min_shortfall_when_no_feasible_split_exists() {
    // 5 people, 2 tables of max=4/min=3: every split (4+1, 3+2) leaves a
    // used table below its minimum. Min is soft, so the optimizer must still
    // return a solution and pick the smallest shortfall (3+2, one guest
    // short) rather than fail or settle for 4+1 (two short).
    let people_csv = (1..=5)
        .map(|index| format!("p{index},Person {index},,,,\n"))
        .fold(
            "id,name,table_type,groups,locked_table,locked_seat\n".to_string(),
            |mut csv, row| {
                csv.push_str(&row);
                csv
            },
        );
    let project = make_project(
        &people_csv,
        "left_id,right_id,score\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,,3,2,\n",
    )
    .unwrap();

    let result = HeuristicOptimizer
        .optimize(
            &project,
            &OptimizationConfig {
                steps: 500,
                ..OptimizationConfig::default()
            },
        )
        .unwrap();
    let solution = &result.solutions[0];
    validate_seating_solution(&project, &solution.assignments).unwrap();

    let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
    for assignment in &solution.assignments {
        *counts.entry(assignment.table_number).or_insert(0) += 1;
    }
    let mut sizes: Vec<usize> = counts.into_values().collect();
    sizes.sort_unstable();
    assert_eq!(sizes, [2, 3]);
    // No closeness rules (proximity 0), no recommended_people (size penalty
    // 0), default used_table_weight 0; the only term is one missing guest
    // on the 2-seat table at the default min_people_weight.
    assert_eq!(
        solution.score,
        -OptimizationConfig::default().min_people_weight
    );
}

#[test]
fn optimizer_moves_whole_group_onto_the_table_that_fits_it() {
    // 11-person group `G` plus 9 singles; two 10-seat tables (1, 2) and one
    // 11-seat table (3), all min 8. The only arrangement seating all of `G`
    // together uses table 3 — which random construction never opens (it
    // fills tables in number order) and a search that cannot open or close
    // a table never reaches. p1/p2 additionally want to be adjacent.
    let mut people_csv = "id,name,table_type,groups,locked_table,locked_seat\n".to_string();
    for index in 1..=11 {
        people_csv.push_str(&format!("g{index},G {index},,G,,\n"));
    }
    for index in 1..=9 {
        people_csv.push_str(&format!("s{index},S {index},,,,\n"));
    }
    let project = make_project(
        &people_csv,
        "left_id,right_id,score\nG,G,5\ng1,g2,10\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\na,round,10,,8,2,\nb,round,11,,8,1,\n",
    )
    .unwrap();
    let config = OptimizationConfig {
        seed: 7,
        attempts: 10,
        steps: 50_000,
        time_limit_secs: 0,
        ..OptimizationConfig::default()
    };

    let run1 = HeuristicOptimizer
        .optimize_timed(&project, &config, None)
        .unwrap();
    let run2 = HeuristicOptimizer
        .optimize_timed(&project, &config, None)
        .unwrap();
    assert_eq!(run1.solutions, run2.solutions);

    let assignments = &run1.solutions[0].assignments;
    validate_seating_solution(&project, assignments).unwrap();
    let seat_of = |id: &str| assignments.iter().find(|a| a.person_id == id).unwrap();

    let group_table = seat_of("g1").table_number;
    assert!(
        (1..=11).all(|index| seat_of(&format!("g{index}")).table_number == group_table),
        "group G is split across tables"
    );
    assert_eq!(seat_of("g1").table_type, "b");
    assert_eq!(
        circular_distance(seat_of("g1").seat_index, seat_of("g2").seat_index, 11),
        1
    );

    let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
    for assignment in assignments {
        *counts.entry(assignment.table_number).or_insert(0) += 1;
    }
    assert!(counts.values().all(|count| *count >= 8), "{counts:?}");
}

#[test]
fn optimizer_splits_a_table_across_smaller_tables_when_that_scores_better() {
    // 12 people in two mutually-hostile groups `G`/`H` of 6 each. One big
    // table seats all 12 (and is what random construction fills first, since
    // it fills already-used tables before opening a new one); two min-6
    // tables seat each group separately with no cross-group penalty.
    // `big`'s `recommended_people` is set to its full capacity (12): under
    // rank-based seat distance, a group of 6 sitting on any 6 seats of the
    // 12-seat big table scores identically to sitting on a dedicated 6-seat
    // table (distance only depends on how many seats are occupied, not the
    // table's capacity or which seats), so without this penalty "G stays
    // on the big table, only H moves off" ties the fully-split solution.
    // Recommending `big` at full capacity makes leaving it half-empty cost
    // `|occupancy - 12| * optimal_table_size_weight`, while leaving it
    // completely unused (the intended split) incurs no size penalty at all
    // (deviation is only scored for *used* tables) — breaking the tie in
    // favor of the split.
    let mut people_csv = "id,name,table_type,groups,locked_table,locked_seat\n".to_string();
    for index in 1..=6 {
        people_csv.push_str(&format!("g{index},G {index},,G,,\n"));
    }
    for index in 1..=6 {
        people_csv.push_str(&format!("h{index},H {index},,H,,\n"));
    }
    let project = make_project(
        &people_csv,
        "left_id,right_id,score\nG,G,5\nH,H,5\nG,H,-5\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nbig,round,12,12,0,1,\nsmall,round,6,,6,2,\n",
    )
    .unwrap();
    let config = OptimizationConfig {
        seed: 3,
        attempts: 8,
        steps: 50_000,
        time_limit_secs: 0,
        ..OptimizationConfig::default()
    };

    let run1 = HeuristicOptimizer
        .optimize_timed(&project, &config, None)
        .unwrap();
    let run2 = HeuristicOptimizer
        .optimize_timed(&project, &config, None)
        .unwrap();
    assert_eq!(run1.solutions, run2.solutions);

    let assignments = &run1.solutions[0].assignments;
    validate_seating_solution(&project, assignments).unwrap();
    let seat_of = |id: &str| assignments.iter().find(|a| a.person_id == id).unwrap();

    let g_table = seat_of("g1").table_number;
    let h_table = seat_of("h1").table_number;
    assert_ne!(g_table, h_table, "G and H ended up on the same table");
    assert!(
        (1..=6).all(|index| seat_of(&format!("g{index}")).table_number == g_table),
        "group G is split across tables"
    );
    assert!(
        (1..=6).all(|index| seat_of(&format!("h{index}")).table_number == h_table),
        "group H is split across tables"
    );
    assert_eq!(seat_of("g1").table_type, "small");
    assert_eq!(seat_of("h1").table_type, "small");

    let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
    for assignment in assignments {
        *counts.entry(assignment.table_number).or_insert(0) += 1;
    }
    assert_eq!(
        counts.get(&1).copied().unwrap_or(0),
        0,
        "big table 1 not empty: {counts:?}"
    );

    // Guard against the tie described above coming back: assert the
    // intended split strictly outscores "leave G on the big table, only
    // move H off" — not just that the optimizer happens to prefer the
    // split for this seed.
    let expected_score = score_solution(&project, assignments, &config).unwrap();
    let tied_alternative: Vec<SeatingAssignment> = (1..=6)
        .map(|index| SeatingAssignment {
            table_number: 1,
            table_type: "big".to_string(),
            seat_index: index - 1,
            person_id: format!("g{index}"),
            person_name: format!("G {index}"),
        })
        .chain((1..=6).map(|index| SeatingAssignment {
            table_number: 2,
            table_type: "small".to_string(),
            seat_index: index - 1,
            person_id: format!("h{index}"),
            person_name: format!("H {index}"),
        }))
        .collect();
    let tied_alternative_score = score_solution(&project, &tied_alternative, &config).unwrap();
    assert!(
        expected_score > tied_alternative_score,
        "expected split (score {expected_score}) should strictly beat leaving G on the big table (score {tied_alternative_score})"
    );
}

#[test]
fn timed_optimizer_finds_compacted_min_capacity_solution() {
    let project = crowded_min_capacity_project();
    let config = OptimizationConfig {
        steps: 300,
        time_limit_secs: 2,
        ..OptimizationConfig::default()
    };
    let result = HeuristicOptimizer
        .optimize_timed(&project, &config, None)
        .unwrap();

    assert!(result.attempts_completed >= config.attempts);
    validate_seating_solution(&project, &result.solutions[0].assignments).unwrap();

    let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
    for assignment in &result.solutions[0].assignments {
        *counts.entry(assignment.table_number).or_insert(0) += 1;
    }

    assert_eq!(counts.remove(&1), Some(6));

    let mut remaining: Vec<usize> = counts.values().copied().collect();
    remaining.sort_unstable();
    assert_eq!(remaining.len(), 4);
    assert_eq!(remaining.iter().sum::<usize>(), 37);
    assert!(remaining.iter().all(|count| (8..=10).contains(count)));
}

#[test]
fn zero_time_limit_run_equals_exact_attempts_run() {
    let project = round_project();
    let config = OptimizationConfig {
        steps: 300,
        time_limit_secs: 0,
        ..OptimizationConfig::default()
    };

    let timed = HeuristicOptimizer
        .optimize_timed(&project, &config, None)
        .unwrap();
    let exact = HeuristicOptimizer.optimize(&project, &config).unwrap();

    assert_eq!(timed.solutions, exact.solutions);
    assert_eq!(timed.attempts_completed, config.attempts);
}

#[test]
fn warm_start_never_returns_worse_and_is_deterministic() {
    let project = round_project();
    let initial = round_assignments();
    let config = OptimizationConfig {
        steps: 300,
        time_limit_secs: 0,
        ..OptimizationConfig::default()
    };

    let run1 = HeuristicOptimizer
        .optimize_timed(&project, &config, Some(&initial))
        .unwrap();
    let run2 = HeuristicOptimizer
        .optimize_timed(&project, &config, Some(&initial))
        .unwrap();
    assert_eq!(run1.solutions, run2.solutions);

    let initial_score = score_solution(&project, &initial, &config).unwrap();
    assert!(run1.solutions[0].score >= initial_score);

    // With no search steps at all, only a real warm start returns the
    // initial seating unchanged (a random start would not).
    let untouched = HeuristicOptimizer
        .optimize_timed(
            &project,
            &OptimizationConfig {
                steps: 0,
                ..config.clone()
            },
            Some(&initial),
        )
        .unwrap();
    assert_eq!(untouched.solutions[0].assignments, initial);
}

#[test]
fn warm_start_rejects_invalid_initial() {
    let project = round_project();
    let mut invalid_initial = round_assignments();
    invalid_initial[1].seat_index = invalid_initial[0].seat_index; // seat collision

    let err = HeuristicOptimizer
        .optimize_timed(
            &project,
            &OptimizationConfig {
                steps: 300,
                ..OptimizationConfig::default()
            },
            Some(&invalid_initial),
        )
        .unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::SeatCollision { .. }))
    );
}

// ── CSV error paths ───────────────────────────────────────────────────────

#[test]
fn people_csv_rejects_non_numeric_locked_table() {
    let csv = "id,name,table_type,groups,locked_table,locked_seat\np1,Alice,,,abc,\n";
    let err = parse_people_csv(csv).unwrap_err();
    assert!(matches!(err, ValidationError::MalformedInput(_)));
}

#[test]
fn closeness_csv_rejects_non_numeric_score() {
    let csv = "left_id,right_id,score\np1,p2,abc\n";
    let err = parse_closeness_csv(csv).unwrap_err();
    assert!(matches!(err, ValidationError::MalformedInput(_)));
}

#[test]
fn tables_csv_rejects_unknown_shape() {
    let csv = "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,hexagon,4,,,1,\n";
    let err = parse_tables_csv(csv).unwrap_err();
    assert!(matches!(err, ValidationError::MalformedInput(_)));
}

#[test]
fn tables_csv_rejects_missing_max_people() {
    let csv = "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,,,,1,\n";
    let err = parse_tables_csv(csv).unwrap_err();
    assert!(matches!(err, ValidationError::MalformedInput(_)));
}

#[test]
fn tables_csv_rejects_duplicate_type_id() {
    let csv = "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,,,1,\nround_4,round,6,,,1,\n";
    let err = parse_tables_csv(csv).unwrap_err();
    assert!(matches!(err, ValidationError::DuplicateTableTypeId(id) if id == "round_4"));
}

#[test]
fn tables_csv_rejects_empty_type_id() {
    let csv = "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\n,round,4,,,1,\n";
    let err = parse_tables_csv(csv).unwrap_err();
    assert!(matches!(err, ValidationError::EmptyTableTypeId));
}

#[test]
fn tables_csv_round_trips_semicircle_shape() {
    let tables = parse_tables_csv(
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nsemi_6,semicircle,6,,,1,\n",
    )
    .unwrap();
    assert_eq!(tables["semi_6"].shape, TableShape::Semicircle);

    let csv = write_tables_csv(&tables).unwrap();
    assert!(csv.contains("semicircle"));

    let project = ProjectInput {
        people: vec![],
        closeness_rules: vec![],
        table_types: tables,
        table_order: Vec::new(),
    };
    let project_file = ProjectFile::new(project, OptimizationConfig::default(), Vec::new());
    let parsed = parse_project_file(&write_project_file(&project_file).unwrap()).unwrap();
    assert_eq!(parsed.table_types["semi_6"].shape, TableShape::Semicircle);
}

// ── SVG escaping ───────────────────────────────────────────────────────────

#[test]
fn svg_escapes_special_characters_in_names() {
    let table_types = build_table_type_map(vec![(
        "round & 4".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 2,
            recommended_people: None,
            min_people: None,
            number_of_tables: Some(1),
        },
    )])
    .unwrap();
    let project = ProjectInput {
        people: vec![Person {
            id: "p1".to_string(),
            name: "A & B <VIP>".to_string(),
            table_type: Some("round & 4".to_string()),
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        }],
        closeness_rules: vec![],
        table_types,
        table_order: Vec::new(),
    };
    let assignments = vec![SeatingAssignment {
        table_number: 1,
        table_type: "round & 4".to_string(),
        seat_index: 0,
        person_id: "p1".to_string(),
        person_name: "A & B <VIP>".to_string(),
    }];

    let layout = build_layout(&project, &assignments).unwrap();
    let svg = render_svg(&layout, &RenderOptions::default());
    assert!(svg.contains("A &amp; B &lt;VIP&gt;"));
    assert!(svg.contains("round &amp; 4"));
    assert!(!svg.contains("A & B <VIP>"));

    // The escaped SVG must still be well-formed enough for resvg to parse.
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let output = std::env::temp_dir().join(format!("wedding-seating-escape-{stamp}.png"));
    render_png(&layout, &RenderOptions::default(), &output).unwrap();
    assert!(output.exists());
    let _ = fs::remove_file(output);
}

// ── Seating CSV round-trip ───────────────────────────────────────────────

// ── apply_seat_drop ───────────────────────────────────────────────────────

/// Fixture covering the drag-and-drop scenarios below: two round_4 tables
/// (1, 2) and one square_4 table (3). p3/p4 are locked at table 1 seats 0/1;
/// p1 (table 1 seat 2) and table 1 seat 3 are free to move; p2 sits alone at
/// table 2; p5 requires the square table type.
fn drop_project() -> ProjectInput {
    let people = vec![
        Person {
            id: "p1".to_string(),
            name: "Alice".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        },
        Person {
            id: "p2".to_string(),
            name: "Bob".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        },
        Person {
            id: "p3".to_string(),
            name: "Cara".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: Some(1),
            locked_seat: Some(0),
        },
        Person {
            id: "p4".to_string(),
            name: "Dan".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: Some(1),
            locked_seat: Some(1),
        },
        Person {
            id: "p5".to_string(),
            name: "Eve".to_string(),
            table_type: Some("square_4".to_string()),
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        },
    ];
    let table_types = build_table_type_map(vec![
        (
            "round_4".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: None,
                min_people: None,
                number_of_tables: Some(2),
            },
        ),
        (
            "square_4".to_string(),
            TableTypeConfig {
                shape: TableShape::Square,
                people_per_side: Some(vec![1, 1, 1, 1]),
                max_people: 4,
                recommended_people: None,
                min_people: None,
                number_of_tables: Some(1),
            },
        ),
    ])
    .unwrap();
    ProjectInput {
        people,
        closeness_rules: vec![],
        table_types,
        table_order: Vec::new(),
    }
}

fn drop_assignments() -> Vec<SeatingAssignment> {
    vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 2,
            person_id: "p1".to_string(),
            person_name: "Alice".to_string(),
        },
        SeatingAssignment {
            table_number: 2,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p2".to_string(),
            person_name: "Bob".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p3".to_string(),
            person_name: "Cara".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p4".to_string(),
            person_name: "Dan".to_string(),
        },
        SeatingAssignment {
            table_number: 3,
            table_type: "square_4".to_string(),
            seat_index: 0,
            person_id: "p5".to_string(),
            person_name: "Eve".to_string(),
        },
    ]
}

#[test]
fn apply_seat_drop_moves_into_empty_seat() {
    let project = drop_project();
    let assignments = drop_assignments();

    let (updated, outcome) = apply_seat_drop(&project, &assignments, "p1", 1, 3).unwrap();

    assert_eq!(outcome, SeatDropOutcome::Moved);
    let p1 = updated.iter().find(|a| a.person_id == "p1").unwrap();
    assert_eq!((p1.table_number, p1.seat_index), (1, 3));
    // Original slice is untouched.
    assert_eq!(assignments, drop_assignments());
}

#[test]
fn apply_seat_drop_swaps_occupied_seat() {
    let project = drop_project();
    let assignments = drop_assignments();

    let (updated, outcome) = apply_seat_drop(&project, &assignments, "p1", 2, 0).unwrap();

    assert_eq!(outcome, SeatDropOutcome::Swapped);
    let p1 = updated.iter().find(|a| a.person_id == "p1").unwrap();
    let p2 = updated.iter().find(|a| a.person_id == "p2").unwrap();
    assert_eq!((p1.table_number, p1.seat_index), (2, 0));
    assert_eq!((p2.table_number, p2.seat_index), (1, 2));
}

#[test]
fn apply_seat_drop_onto_own_seat_is_a_no_op() {
    let project = drop_project();
    let assignments = drop_assignments();

    let (updated, outcome) = apply_seat_drop(&project, &assignments, "p1", 1, 2).unwrap();

    assert_eq!(outcome, SeatDropOutcome::Moved);
    assert_eq!(updated, assignments);
}

#[test]
fn apply_seat_drop_refuses_moving_a_locked_seat_guest() {
    let project = drop_project();
    let assignments = drop_assignments();

    let err = apply_seat_drop(&project, &assignments, "p3", 1, 3).unwrap_err();

    assert!(err.errors.iter().any(|e| matches!(
        e,
        ValidationError::SeatingViolatesLockedSeat {
            person_id,
            locked_seat: 0,
            assigned_seat: 3,
        } if person_id == "p3"
    )));
}

#[test]
fn apply_seat_drop_refuses_swap_that_breaks_other_guests_locked_seat() {
    let project = drop_project();
    let assignments = drop_assignments();

    // p4 is locked to (table 1, seat 1); swapping p1 onto p4's seat would
    // relocate p4 to p1's old seat (table 1, seat 2), breaking their lock.
    let err = apply_seat_drop(&project, &assignments, "p1", 1, 1).unwrap_err();

    assert!(err.errors.iter().any(|e| matches!(
        e,
        ValidationError::SeatingViolatesLockedSeat {
            person_id,
            locked_seat: 1,
            assigned_seat: 2,
        } if person_id == "p4"
    )));
}

#[test]
fn apply_seat_drop_refuses_table_type_mismatch() {
    let project = drop_project();
    let assignments = drop_assignments();

    // p5 requires square_4 but the drop target is a round_4 table.
    let err = apply_seat_drop(&project, &assignments, "p5", 1, 3).unwrap_err();

    assert!(err
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::SeatingPersonTableTypeMismatch { person_id, .. } if person_id == "p5")));
}

#[test]
fn apply_seat_drop_refuses_out_of_range_seat_index() {
    let project = drop_project();
    let assignments = drop_assignments();

    let err = apply_seat_drop(&project, &assignments, "p1", 1, 99).unwrap_err();

    assert!(err.errors.iter().any(|e| matches!(
        e,
        ValidationError::SeatIndexOutOfRange {
            person_id,
            seat: 99,
            capacity: 4,
        } if person_id == "p1"
    )));
}

#[test]
fn apply_seat_drop_allows_dropping_onto_an_empty_table_below_min() {
    let project = min_shortfall_project();
    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 2,
            person_id: "p3".to_string(),
            person_name: "C".to_string(),
        },
    ];

    // Table 2's min_people is 2, but dropping a single guest onto it (an
    // empty table) must succeed now that min_people is a soft penalty.
    let (updated, outcome) = apply_seat_drop(&project, &assignments, "p3", 2, 0).unwrap();

    assert_eq!(outcome, SeatDropOutcome::Moved);
    let moved = updated.iter().find(|a| a.person_id == "p3").unwrap();
    assert_eq!(moved.table_number, 2);
    assert_eq!(moved.seat_index, 0);
}

#[test]
fn partial_validation_accepts_missing_people_but_rejects_duplicates() {
    let project = drop_project();
    let assignments: Vec<SeatingAssignment> = drop_assignments()
        .into_iter()
        .filter(|a| a.person_id != "p1")
        .collect();

    // p1 is missing entirely: fine under the partial check.
    validate_partial_seating_solution(&project, &assignments).unwrap();

    // A duplicated assignment for an already-seated person is still an error.
    let mut duplicated = assignments.clone();
    let dup_id = duplicated[0].person_id.clone();
    duplicated.push(duplicated[0].clone());
    let err = validate_partial_seating_solution(&project, &duplicated).unwrap_err();
    assert!(err.errors.iter().any(
        |e| matches!(e, ValidationError::MissingOrDuplicatePerson(id) if id.as_str() == dup_id)
    ));
}

#[test]
fn apply_seat_drop_places_an_unassigned_guest() {
    let project = drop_project();
    let assignments: Vec<SeatingAssignment> = drop_assignments()
        .into_iter()
        .filter(|a| a.person_id != "p1")
        .collect();

    let (updated, outcome) = apply_seat_drop(&project, &assignments, "p1", 1, 3).unwrap();

    assert_eq!(outcome, SeatDropOutcome::Moved);
    let p1 = updated.iter().find(|a| a.person_id == "p1").unwrap();
    assert_eq!((p1.table_number, p1.seat_index), (1, 3));
    assert_eq!(p1.table_type, "round_4");
}

#[test]
fn apply_seat_drop_from_unassigned_onto_occupied_seat_unassigns_the_occupant() {
    let project = drop_project();
    let assignments: Vec<SeatingAssignment> = drop_assignments()
        .into_iter()
        .filter(|a| a.person_id != "p1")
        .collect();

    // p1 (currently unassigned) drops onto p2's occupied seat.
    let (updated, outcome) = apply_seat_drop(&project, &assignments, "p1", 2, 0).unwrap();

    assert_eq!(outcome, SeatDropOutcome::Swapped);
    let p1 = updated.iter().find(|a| a.person_id == "p1").unwrap();
    assert_eq!((p1.table_number, p1.seat_index), (2, 0));
    assert!(!updated.iter().any(|a| a.person_id == "p2"));
}

/// An unassigned mover must never bump a *locked* occupant out of their
/// seat — p4 is locked to (table 1, seat 1).
#[test]
fn apply_seat_drop_refuses_to_displace_a_locked_occupant() {
    let project = drop_project();
    let assignments: Vec<SeatingAssignment> = drop_assignments()
        .into_iter()
        .filter(|a| a.person_id != "p1")
        .collect();

    let err = apply_seat_drop(&project, &assignments, "p1", 1, 1).unwrap_err();

    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::LockedGuestDisplaced(id) if id == "p4"))
    );
}

/// A mover that is neither in `assignments` nor in `project.people` is
/// unknown, not merely unassigned.
#[test]
fn apply_seat_drop_unknown_person_not_in_people_or_assignments() {
    let project = drop_project();
    let assignments = drop_assignments();

    let err = apply_seat_drop(&project, &assignments, "ghost", 1, 3).unwrap_err();

    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::UnknownPersonInSeating(id) if id == "ghost"))
    );
}

/// Dropping an unassigned but *locked* guest onto a seat other than their
/// own still violates their lock — being unassigned doesn't waive it.
#[test]
fn apply_seat_drop_unassigned_locked_guest_off_their_seat_fails() {
    let project = drop_project();
    // p3 is locked to (table 1, seat 0); remove them so they're unassigned.
    let assignments: Vec<SeatingAssignment> = drop_assignments()
        .into_iter()
        .filter(|a| a.person_id != "p3")
        .collect();

    let err = apply_seat_drop(&project, &assignments, "p3", 2, 1).unwrap_err();

    assert!(err.errors.iter().any(|e| matches!(
        e,
        ValidationError::SeatingViolatesLockedSeat { person_id, locked_seat: 0, .. }
            if person_id == "p3"
    )));
}

/// `validate_partial_seating_solution` still enforces a locked guest's seat
/// even though every other guest is missing.
#[test]
fn partial_validation_still_enforces_locked_seat() {
    let project = drop_project();
    // p3 is locked to (table 1, seat 0) but placed at seat 3; p1, p2, p4, p5
    // are simply missing.
    let assignments = vec![SeatingAssignment {
        table_number: 1,
        table_type: "round_4".to_string(),
        seat_index: 3,
        person_id: "p3".to_string(),
        person_name: "Cara".to_string(),
    }];

    let err = validate_partial_seating_solution(&project, &assignments).unwrap_err();

    assert!(err.errors.iter().any(|e| matches!(
        e,
        ValidationError::SeatingViolatesLockedSeat { person_id, locked_seat: 0, assigned_seat: 3 }
            if person_id == "p3"
    )));
}

/// `validate_partial_seating_solution` still enforces seat collisions even
/// though every other guest is missing.
#[test]
fn partial_validation_still_enforces_seat_collision() {
    let project = drop_project();
    // p1 and p2 (both unlocked) both placed at (table 2, seat 0); p3, p4, p5
    // are missing.
    let assignments = vec![
        SeatingAssignment {
            table_number: 2,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "Alice".to_string(),
        },
        SeatingAssignment {
            table_number: 2,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p2".to_string(),
            person_name: "Bob".to_string(),
        },
    ];

    let err = validate_partial_seating_solution(&project, &assignments).unwrap_err();

    assert!(err.errors.iter().any(|e| matches!(
        e,
        ValidationError::SeatCollision {
            table_number: 2,
            seat: 0
        }
    )));
}

#[test]
fn editor_layout_accepts_partial_seating() {
    let project = drop_project();
    let assignments: Vec<SeatingAssignment> = drop_assignments()
        .into_iter()
        .filter(|a| a.person_id != "p1")
        .collect();

    let strict_err = build_layout(&project, &assignments).unwrap_err();
    assert!(
        strict_err
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingOrDuplicatePerson(id) if id == "p1"))
    );

    let layout = build_editor_layout(&project, &assignments, false).unwrap();
    assert!(
        !layout
            .tables
            .iter()
            .flat_map(|table| &table.seats)
            .any(|seat| seat.person_name.as_deref() == Some("Alice"))
    );
}

#[test]
fn unassigned_people_preserves_project_order() {
    let people = drop_project().people;
    let assignments: Vec<SeatingAssignment> = drop_assignments()
        .into_iter()
        .filter(|a| a.person_id != "p1" && a.person_id != "p4")
        .collect();

    let unassigned = unassigned_people(&people, &assignments);

    assert_eq!(
        unassigned.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
        vec!["p1", "p4"]
    );
}

#[test]
fn unassign_person_removes_their_assignment() {
    let project = drop_project();
    let assignments = drop_assignments();

    let updated = unassign_person(&project, &assignments, "p1").unwrap();

    assert!(!updated.iter().any(|a| a.person_id == "p1"));
    assert_eq!(updated.len(), assignments.len() - 1);
}

#[test]
fn unassign_person_refuses_a_locked_guest() {
    let project = drop_project();
    let assignments = drop_assignments();

    let err = unassign_person(&project, &assignments, "p3").unwrap_err();

    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::LockedGuestUnassigned(id) if id == "p3"))
    );
}

/// A `locked_table`-only guest (no `locked_seat`) is refused just like a
/// fully seat-locked one — the lock, not the seat specifically, is what's
/// enforced.
#[test]
fn unassign_person_refuses_a_table_only_locked_guest() {
    let mut project = drop_project();
    let p1 = project
        .people
        .iter_mut()
        .find(|p| p.id == "p1")
        .expect("p1 exists in drop_project");
    p1.locked_table = Some(2);
    let assignments = drop_assignments();

    let err = unassign_person(&project, &assignments, "p1").unwrap_err();

    assert!(
        err.errors
            .iter()
            .any(|e| matches!(e, ValidationError::LockedGuestUnassigned(id) if id == "p1"))
    );
}

/// A locked guest who is already unassigned is a no-op, not an error — the
/// lock only prevents *unassigning* a currently seated guest.
#[test]
fn unassign_person_locked_guest_already_unassigned_is_a_no_op() {
    let project = drop_project();
    let assignments: Vec<SeatingAssignment> = drop_assignments()
        .into_iter()
        .filter(|a| a.person_id != "p3")
        .collect();

    let updated = unassign_person(&project, &assignments, "p3").unwrap();

    assert_eq!(updated, assignments);
}

#[test]
fn unassign_person_already_unassigned_is_a_no_op() {
    let project = drop_project();
    let assignments: Vec<SeatingAssignment> = drop_assignments()
        .into_iter()
        .filter(|a| a.person_id != "p1")
        .collect();

    let updated = unassign_person(&project, &assignments, "p1").unwrap();

    assert_eq!(updated, assignments);
}

#[test]
fn seating_csv_round_trip_is_sorted() {
    let shuffled = vec![
        SeatingAssignment {
            table_number: 2,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p3".to_string(),
            person_name: "C".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
    ];
    let csv = write_seating_csv(&shuffled).unwrap();
    let parsed = parse_seating_csv(&csv).unwrap();

    let expected = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
        SeatingAssignment {
            table_number: 2,
            table_type: "round_4".to_string(),
            seat_index: 1,
            person_id: "p3".to_string(),
            person_name: "C".to_string(),
        },
    ];
    assert_eq!(parsed, expected);
}

/// Bumping `number_of_tables` on a type that isn't lexicographically last
/// renumbers every later type's instances. `table_number_remap` must track
/// each instance by `(table_type, ordinal)` so assignments and locked tables
/// can be carried forward instead of failing validation after the bump.
#[test]
fn table_number_remap_tracks_type_ordinal_after_a_type_grows() {
    let table_types_with_a_count = |a_count: usize| {
        build_table_type_map(vec![
            (
                "a".to_string(),
                TableTypeConfig {
                    shape: TableShape::Round,
                    people_per_side: None,
                    max_people: 4,
                    recommended_people: None,
                    min_people: None,
                    number_of_tables: Some(a_count),
                },
            ),
            (
                "b".to_string(),
                TableTypeConfig {
                    shape: TableShape::Round,
                    people_per_side: None,
                    max_people: 4,
                    recommended_people: None,
                    min_people: None,
                    number_of_tables: Some(1),
                },
            ),
        ])
        .unwrap()
    };

    let people = vec![
        Person {
            id: "p1".to_string(),
            name: "Alice".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        },
        Person {
            id: "p2".to_string(),
            name: "Bob".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: Some(2),
            locked_seat: None,
        },
    ];

    let before = ProjectInput {
        people: people.clone(),
        closeness_rules: vec![],
        table_types: table_types_with_a_count(1),
        table_order: Vec::new(),
    };
    let old_instances = generate_table_instances(&before);
    assert_eq!(
        old_instances
            .iter()
            .find(|t| t.table_type == "b")
            .unwrap()
            .number,
        2
    );

    let after = ProjectInput {
        people: people.clone(),
        closeness_rules: vec![],
        table_types: table_types_with_a_count(2),
        table_order: Vec::new(),
    };
    let new_instances = generate_table_instances(&after);
    assert_eq!(
        new_instances
            .iter()
            .find(|t| t.table_type == "b")
            .unwrap()
            .number,
        3
    );

    let remap = table_number_remap(&old_instances, &new_instances);
    assert_eq!(remap.get(&2).copied(), Some(3));

    let mut assignments = vec![
        SeatingAssignment {
            table_number: 2,
            table_type: "b".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "Alice".to_string(),
        },
        SeatingAssignment {
            table_number: 2,
            table_type: "b".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "Bob".to_string(),
        },
    ];
    for a in assignments.iter_mut() {
        if let Some(&new_number) = remap.get(&a.table_number) {
            a.table_number = new_number;
        }
    }
    assert_eq!(assignments[0].table_number, 3);
    assert_eq!(assignments[1].table_number, 3);

    let mut remapped_project = after;
    if let Some(locked) = remapped_project.people[1].locked_table {
        remapped_project.people[1].locked_table = remap.get(&locked).copied();
    }
    assert_eq!(remapped_project.people[1].locked_table, Some(3));

    assert!(validate_seating_solution(&remapped_project, &assignments).is_ok());
}

fn compact_table_types() -> BTreeMap<TableTypeId, TableTypeConfig> {
    build_table_type_map(vec![
        (
            "a".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: None,
                min_people: Some(0),
                number_of_tables: Some(3),
            },
        ),
        (
            "b".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: None,
                min_people: Some(0),
                number_of_tables: Some(2),
            },
        ),
    ])
    .unwrap()
}

fn compact_people() -> Vec<Person> {
    ["p1", "p2", "p3", "p4", "p5"]
        .into_iter()
        .map(|id| Person {
            id: id.to_string(),
            name: id.to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        })
        .collect()
}

/// Table 1 (type `a`, occupied), table 3 (type `a`, occupied), table 5
/// (type `b`, occupied) — tables 2 and 4 are empty.
fn compact_assignments() -> Vec<SeatingAssignment> {
    vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "a".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "p1".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "a".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "p2".to_string(),
        },
        SeatingAssignment {
            table_number: 3,
            table_type: "a".to_string(),
            seat_index: 0,
            person_id: "p3".to_string(),
            person_name: "p3".to_string(),
        },
        SeatingAssignment {
            table_number: 3,
            table_type: "a".to_string(),
            seat_index: 1,
            person_id: "p4".to_string(),
            person_name: "p4".to_string(),
        },
        SeatingAssignment {
            table_number: 5,
            table_type: "b".to_string(),
            seat_index: 0,
            person_id: "p5".to_string(),
            person_name: "p5".to_string(),
        },
    ]
}

/// Apply a [`compact_table_numbers`] result the way a caller must: the order
/// on the project, the number map on every assignment and every locked
/// guest — mirrors the [`swap_table_numbers`]/[`move_table_number`] tests.
fn apply_compaction(
    project: &ProjectInput,
    assignments: &[SeatingAssignment],
    order: Vec<TableTypeId>,
    map: &BTreeMap<usize, usize>,
) -> (ProjectInput, Vec<SeatingAssignment>) {
    let mut project = project.clone();
    project.table_order = order;
    for person in project.people.iter_mut() {
        if let Some(number) = person.locked_table
            && let Some(&new_number) = map.get(&number)
        {
            person.locked_table = Some(new_number);
        }
    }
    let assignments = assignments
        .iter()
        .map(|a| SeatingAssignment {
            table_number: map[&a.table_number],
            ..a.clone()
        })
        .collect();
    (project, assignments)
}

/// Compaction sorts *all* used tables before *all* empty ones, across every
/// type — not per type, so table 5 (type `b`, occupied) lands before table 2
/// (type `a`, empty) even though it's a different, later-numbered type.
/// Preserves seat indices and is score-neutral because same-type table
/// instances are identical for scoring.
#[test]
fn compact_table_numbers_moves_all_empty_tables_to_the_end() {
    // Closeness rules on same-table pairs make the proximity term non-zero,
    // so the score-neutral assertion below is not a tautology.
    let project = ProjectInput {
        people: compact_people(),
        closeness_rules: vec![
            ClosenessRule {
                left_id: "p1".to_string(),
                right_id: "p2".to_string(),
                score: 5.0,
            },
            ClosenessRule {
                left_id: "p3".to_string(),
                right_id: "p4".to_string(),
                score: 5.0,
            },
        ],
        table_types: compact_table_types(),
        table_order: Vec::new(),
    };
    let assignments = compact_assignments();

    let (order, map) = compact_table_numbers(&project, &assignments);
    assert_eq!(
        order,
        vec![
            "a".to_string(),
            "a".to_string(),
            "b".to_string(),
            "a".to_string(),
            "b".to_string(),
        ]
    );
    assert_eq!(
        map,
        BTreeMap::from([(1, 1), (3, 2), (5, 3), (2, 4), (4, 5)])
    );

    let (compacted_project, compacted) = apply_compaction(&project, &assignments, order, &map);

    let table_of = |compacted: &[SeatingAssignment], person_id: &str| {
        let a = compacted.iter().find(|a| a.person_id == person_id).unwrap();
        (a.table_number, a.seat_index)
    };
    assert_eq!(table_of(&compacted, "p1"), (1, 0));
    assert_eq!(table_of(&compacted, "p2"), (1, 1));
    assert_eq!(table_of(&compacted, "p3"), (2, 0));
    assert_eq!(table_of(&compacted, "p4"), (2, 1));
    assert_eq!(table_of(&compacted, "p5"), (3, 0));

    assert!(validate_seating_solution(&compacted_project, &compacted).is_ok());

    let config = OptimizationConfig::default();
    let score_before = score_solution(&project, &assignments, &config).unwrap();
    let score_after = score_solution(&compacted_project, &compacted, &config).unwrap();
    assert!((score_before - score_after).abs() < 1e-9);
}

/// A guest's `locked_table` moves with the map like any other table
/// reference — compaction no longer pins a locked table's number in place,
/// matching [`swap_table_numbers`]/[`move_table_number`].
#[test]
fn compact_table_numbers_map_carries_locked_tables() {
    let mut people = compact_people();
    people
        .iter_mut()
        .find(|p| p.id == "p3")
        .unwrap()
        .locked_table = Some(3);
    let project = ProjectInput {
        people,
        closeness_rules: vec![],
        table_types: compact_table_types(),
        table_order: Vec::new(),
    };
    let assignments = compact_assignments();

    let (order, map) = compact_table_numbers(&project, &assignments);
    let (compacted_project, compacted) = apply_compaction(&project, &assignments, order, &map);

    let table_of = |compacted: &[SeatingAssignment], person_id: &str| {
        compacted
            .iter()
            .find(|a| a.person_id == person_id)
            .unwrap()
            .table_number
    };
    assert_eq!(table_of(&compacted, "p1"), 1);
    assert_eq!(table_of(&compacted, "p2"), 1);
    assert_eq!(table_of(&compacted, "p3"), 2);
    assert_eq!(table_of(&compacted, "p4"), 2);
    assert_eq!(table_of(&compacted, "p5"), 3);
    assert_eq!(
        compacted_project
            .people
            .iter()
            .find(|p| p.id == "p3")
            .unwrap()
            .locked_table,
        Some(2)
    );

    assert!(validate_seating_solution(&compacted_project, &compacted).is_ok());
}

/// With a non-empty [`ProjectInput::table_order`] (`b`, then `a`, then `a` —
/// so `b` is #1 and `a`'s two instances are #2 and #3), the occupied table
/// (3) moves onto the empty one (2); it can never land on table 1, since
/// that one is occupied too — not because it belongs to a different type.
#[test]
fn compact_table_numbers_respects_a_nonempty_table_order() {
    let table_types = build_table_type_map(vec![
        (
            "a".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: None,
                min_people: Some(0),
                number_of_tables: Some(2),
            },
        ),
        (
            "b".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: None,
                min_people: Some(0),
                number_of_tables: Some(1),
            },
        ),
    ])
    .unwrap();

    let project = ProjectInput {
        people: compact_people()[..2].to_vec(),
        closeness_rules: vec![],
        table_types,
        table_order: vec!["b".to_string(), "a".to_string(), "a".to_string()],
    };
    assert_eq!(
        instance_types(&project),
        vec![
            (1, "b".to_string()),
            (2, "a".to_string()),
            (3, "a".to_string()),
        ]
    );

    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "b".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "p1".to_string(),
        },
        SeatingAssignment {
            table_number: 3,
            table_type: "a".to_string(),
            seat_index: 0,
            person_id: "p2".to_string(),
            person_name: "p2".to_string(),
        },
    ];

    let (order, map) = compact_table_numbers(&project, &assignments);
    let (compacted_project, compacted) = apply_compaction(&project, &assignments, order, &map);
    let table_of = |compacted: &[SeatingAssignment], person_id: &str| {
        compacted
            .iter()
            .find(|a| a.person_id == person_id)
            .unwrap()
            .table_number
    };
    assert_eq!(table_of(&compacted, "p1"), 1);
    assert_eq!(table_of(&compacted, "p2"), 2);

    assert!(validate_seating_solution(&compacted_project, &compacted).is_ok());

    // Growing `a` to three instances: the order only names two `a` entries,
    // so the third is appended in derived order as #4, leaving 1/2/3
    // untouched.
    let mut grown = project.clone();
    grown.table_types.get_mut("a").unwrap().number_of_tables = Some(3);
    let old_instances = generate_table_instances(&project);
    let new_instances = generate_table_instances(&grown);
    let remap = table_number_remap(&old_instances, &new_instances);
    assert_eq!(remap, BTreeMap::from([(1, 1), (2, 2), (3, 3)]));
    assert_eq!(
        new_instances
            .last()
            .map(|t| (t.number, t.table_type.as_str())),
        Some((4, "a"))
    );
}

/// Ratchet mitigation: an unlimited type grown to 3 instances via
/// `table_order`, with guests removed so 2 of its 3 are empty — compaction
/// drops the extra spare entirely (not just reorders it), leaving exactly
/// one spare. A different, limited type's table moves to a new number but
/// keeps its type.
#[test]
fn compact_table_numbers_drops_extra_unlimited_spares() {
    let table_types = build_table_type_map(vec![
        (
            "round_4".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: None,
                min_people: None,
                number_of_tables: None,
            },
        ),
        (
            "sq".to_string(),
            TableTypeConfig {
                shape: TableShape::Square,
                people_per_side: Some(vec![1, 1, 1, 1]),
                max_people: 4,
                recommended_people: None,
                min_people: None,
                number_of_tables: Some(1),
            },
        ),
    ])
    .unwrap();
    let people = vec![
        Person {
            id: "p1".to_string(),
            name: "A".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        },
        Person {
            id: "p2".to_string(),
            name: "B".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        },
    ];
    let project = ProjectInput {
        people,
        closeness_rules: vec![],
        table_types,
        table_order: vec![
            "round_4".to_string(),
            "round_4".to_string(),
            "round_4".to_string(),
            "sq".to_string(),
        ],
    };
    assert_eq!(
        instance_types(&project),
        vec![
            (1, "round_4".to_string()),
            (2, "round_4".to_string()),
            (3, "round_4".to_string()),
            (4, "sq".to_string()),
        ]
    );

    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_4".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 4,
            table_type: "sq".to_string(),
            seat_index: 0,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
    ];

    let (order, map) = compact_table_numbers(&project, &assignments);

    // Table 3 (the second empty `round_4`) is dropped entirely; table 2
    // survives as the one remaining spare.
    assert_eq!(
        order,
        vec![
            "round_4".to_string(),
            "sq".to_string(),
            "round_4".to_string()
        ]
    );
    assert_eq!(map, BTreeMap::from([(1, 1), (4, 2), (2, 3)]));
    assert!(!map.contains_key(&3));

    let (compacted_project, compacted) = apply_compaction(&project, &assignments, order, &map);
    // `sq`'s table moved from 4 to 2, but it's still type `sq`.
    assert_eq!(
        instance_types(&compacted_project),
        vec![
            (1, "round_4".to_string()),
            (2, "sq".to_string()),
            (3, "round_4".to_string()),
        ]
    );
    assert!(validate_seating_solution(&compacted_project, &compacted).is_ok());
}

/// A guest locked to a table number created only by the locked-table
/// shortfall fill (see `generate_table_instances`) still has their lock
/// correctly remapped by compaction.
#[test]
fn compact_table_numbers_remaps_a_lock_created_by_the_shortfall_fill() {
    let table_types = build_table_type_map(vec![(
        "a".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 4,
            recommended_people: None,
            min_people: None,
            number_of_tables: None,
        },
    )])
    .unwrap();
    let people = vec![
        Person {
            id: "p1".to_string(),
            name: "A".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        },
        Person {
            id: "p2".to_string(),
            name: "B".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: Some(3),
            locked_seat: None,
        },
    ];
    let project = ProjectInput {
        people,
        closeness_rules: vec![],
        table_types,
        table_order: Vec::new(),
    };

    // Derived count alone (ceil(2/4) = 1) falls short of the locked table
    // number 3; the shortfall fill pads type `a` up to 3 instances so the
    // lock resolves.
    assert_eq!(
        instance_types(&project),
        vec![
            (1, "a".to_string()),
            (2, "a".to_string()),
            (3, "a".to_string()),
        ]
    );

    let assignments = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "a".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 3,
            table_type: "a".to_string(),
            seat_index: 0,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
    ];

    let (order, map) = compact_table_numbers(&project, &assignments);
    let (compacted_project, compacted) = apply_compaction(&project, &assignments, order, &map);

    assert_eq!(
        compacted_project
            .people
            .iter()
            .find(|p| p.id == "p2")
            .unwrap()
            .locked_table,
        Some(2)
    );
    assert!(validate_seating_solution(&compacted_project, &compacted).is_ok());
}

// ── ensure_spare_tables ────────────────────────────────────────────────────

/// An unlimited type with no unoccupied instance left gets one appended to
/// the table order.
#[test]
fn ensure_spare_tables_appends_when_an_unlimited_type_is_full() {
    let table_types = build_table_type_map(vec![(
        "round_4".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 4,
            recommended_people: None,
            min_people: None,
            number_of_tables: None,
        },
    )])
    .unwrap();
    let project = ProjectInput {
        people: vec![Person {
            id: "p1".to_string(),
            name: "A".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        }],
        closeness_rules: vec![],
        table_types,
        table_order: Vec::new(),
    };
    let assignments = vec![SeatingAssignment {
        table_number: 1,
        table_type: "round_4".to_string(),
        seat_index: 0,
        person_id: "p1".to_string(),
        person_name: "A".to_string(),
    }];

    let order = ensure_spare_tables(&project, &assignments).unwrap();
    assert_eq!(order, vec!["round_4".to_string(), "round_4".to_string()]);

    // The append never renumbers the existing (occupied) table.
    let mut grown = project.clone();
    grown.table_order = order;
    assert_eq!(
        instance_types(&grown),
        vec![(1, "round_4".to_string()), (2, "round_4".to_string())]
    );
}

/// With two unlimited types and no explicit `table_order`, only the full
/// one (`a`) grows; `b`'s untouched spare is left alone, and every
/// existing table number keeps its type after the append.
#[test]
fn ensure_spare_tables_grows_one_full_unlimited_type_leaving_others_untouched() {
    let table_types = build_table_type_map(vec![
        (
            "a".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: None,
                min_people: None,
                number_of_tables: None,
            },
        ),
        (
            "b".to_string(),
            TableTypeConfig {
                shape: TableShape::Round,
                people_per_side: None,
                max_people: 4,
                recommended_people: None,
                min_people: None,
                number_of_tables: None,
            },
        ),
    ])
    .unwrap();
    let project = ProjectInput {
        people: vec![Person {
            id: "p1".to_string(),
            name: "A".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        }],
        closeness_rules: vec![],
        table_types,
        table_order: Vec::new(),
    };
    // Derived: one instance per type (`a` = #1, `b` = #2, lexicographic).
    assert_eq!(
        instance_types(&project),
        vec![(1, "a".to_string()), (2, "b".to_string())]
    );
    let assignments = vec![SeatingAssignment {
        table_number: 1,
        table_type: "a".to_string(),
        seat_index: 0,
        person_id: "p1".to_string(),
        person_name: "A".to_string(),
    }];

    let order = ensure_spare_tables(&project, &assignments).unwrap();
    assert_eq!(
        order,
        vec!["a".to_string(), "b".to_string(), "a".to_string()]
    );

    let mut grown = project.clone();
    grown.table_order = order;
    assert_eq!(
        instance_types(&grown),
        vec![
            (1, "a".to_string()),
            (2, "b".to_string()),
            (3, "a".to_string()),
        ]
    );
}

/// A type that already has an unoccupied instance needs no growth.
#[test]
fn ensure_spare_tables_is_none_when_a_spare_exists() {
    let project = ProjectInput {
        people: compact_people(),
        closeness_rules: vec![],
        table_types: compact_table_types(),
        table_order: Vec::new(),
    };
    let assignments = compact_assignments();

    assert_eq!(ensure_spare_tables(&project, &assignments), None);
}

/// A *limited* type at its `number_of_tables` cap never grows, even with
/// every instance occupied — the cap is a hard ceiling.
#[test]
fn ensure_spare_tables_is_none_when_a_limited_type_is_at_its_limit() {
    let table_types = build_table_type_map(vec![(
        "round_4".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 4,
            recommended_people: None,
            min_people: None,
            number_of_tables: Some(1),
        },
    )])
    .unwrap();
    let project = ProjectInput {
        people: vec![Person {
            id: "p1".to_string(),
            name: "A".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        }],
        closeness_rules: vec![],
        table_types,
        table_order: Vec::new(),
    };
    let assignments = vec![SeatingAssignment {
        table_number: 1,
        table_type: "round_4".to_string(),
        seat_index: 0,
        person_id: "p1".to_string(),
        person_name: "A".to_string(),
    }];

    // A limited type's instance count is always exactly `number_of_tables`
    // (see `generate_table_instances`) — there is no "below the limit" state
    // for `ensure_spare_tables` to grow into; the whole cap is already
    // materialized from the start, and once full, it stays full.
    assert_eq!(ensure_spare_tables(&project, &assignments), None);
}

// ── Table order ───────────────────────────────────────────────────────────

/// Two single-instance types whose lexicographic order (`rodona` before
/// `square`) is the derived table numbering a user may want to override.
fn order_project() -> ProjectInput {
    ProjectInput {
        people: vec![],
        closeness_rules: vec![],
        table_types: build_table_type_map(vec![
            (
                "rodona".to_string(),
                TableTypeConfig {
                    shape: TableShape::Round,
                    people_per_side: None,
                    max_people: 10,
                    recommended_people: None,
                    min_people: None,
                    number_of_tables: Some(1),
                },
            ),
            (
                "square".to_string(),
                TableTypeConfig {
                    shape: TableShape::Square,
                    people_per_side: Some(vec![1, 1, 1, 1]),
                    max_people: 4,
                    recommended_people: None,
                    min_people: None,
                    number_of_tables: Some(1),
                },
            ),
        ])
        .unwrap(),
        table_order: Vec::new(),
    }
}

fn instance_types(project: &ProjectInput) -> Vec<(usize, String)> {
    generate_table_instances(project)
        .into_iter()
        .map(|instance| (instance.number, instance.table_type))
        .collect()
}

#[test]
fn table_order_controls_instance_numbering() {
    let derived = order_project();
    let instances = generate_table_instances(&derived);
    assert_eq!(instances[0].table_type, "rodona");
    assert_eq!(instances[1].table_type, "square");

    let ordered = ProjectInput {
        table_order: vec!["square".to_string(), "rodona".to_string()],
        ..derived
    };
    let instances = generate_table_instances(&ordered);

    // The whole configuration follows the type, not the number.
    assert_eq!(instances[0].number, 1);
    assert_eq!(instances[0].table_type, "square");
    assert_eq!(instances[0].shape, TableShape::Square);
    assert_eq!(instances[0].max_people, 4);
    assert_eq!(instances[1].number, 2);
    assert_eq!(instances[1].table_type, "rodona");
    assert_eq!(instances[1].shape, TableShape::Round);
    assert_eq!(instances[1].max_people, 10);
}

/// A stored order that no longer matches the configuration heals itself: it
/// never drops an instance and never errors.
#[test]
fn table_order_self_heals_when_counts_change() {
    let mut project = order_project();
    project
        .table_types
        .get_mut("rodona")
        .unwrap()
        .number_of_tables = Some(3);
    project.table_order = vec!["square".to_string(), "rodona".to_string()];

    let expected = vec![
        (1, "square".to_string()),
        (2, "rodona".to_string()),
        (3, "rodona".to_string()),
        (4, "rodona".to_string()),
    ];
    // The two extra `rodona` instances are appended in derived order.
    assert_eq!(instance_types(&project), expected);

    // An entry naming a type that no longer exists is skipped.
    let removed = ProjectInput {
        table_order: vec!["gone".to_string(), "square".to_string()],
        ..project
    };
    assert_eq!(instance_types(&removed), expected);

    // Count SHRANK: `rodona` still has one instance, but the order names it
    // three times. The extra entries are skipped (not duplicated onto later
    // numbers), leaving exactly one instance per type and no duplicate
    // numbers.
    let shrunk = ProjectInput {
        table_order: vec![
            "rodona".to_string(),
            "rodona".to_string(),
            "square".to_string(),
            "rodona".to_string(),
        ],
        ..order_project()
    };
    assert_eq!(
        instance_types(&shrunk),
        vec![(1, "rodona".to_string()), (2, "square".to_string())]
    );
}

/// An *unlimited* type's (`number_of_tables: None`) instance count grows to
/// match how many times it appears in [`ProjectInput::table_order`] when
/// that's larger than its derived count — the mechanism
/// [`ensure_spare_tables`] relies on to grow a full type on demand. A
/// *limited* type never grows this way (see `table_order_self_heals...`'s
/// "Count SHRANK" case).
#[test]
fn unlimited_type_grows_to_match_table_order_entries() {
    let table_types = build_table_type_map(vec![(
        "round_4".to_string(),
        TableTypeConfig {
            shape: TableShape::Round,
            people_per_side: None,
            max_people: 4,
            recommended_people: None,
            min_people: None,
            number_of_tables: None,
        },
    )])
    .unwrap();
    let project = ProjectInput {
        people: vec![Person {
            id: "p1".to_string(),
            name: "A".to_string(),
            table_type: None,
            groups: vec![],
            locked_table: None,
            locked_seat: None,
        }],
        closeness_rules: vec![],
        table_types,
        table_order: Vec::new(),
    };

    // Derived count alone is ceil(1 / 4) = 1 instance.
    assert_eq!(instance_types(&project), vec![(1, "round_4".to_string())]);

    // Naming it twice in `table_order` grows it to 2 instances.
    let grown = ProjectInput {
        table_order: vec!["round_4".to_string(), "round_4".to_string()],
        ..project
    };
    assert_eq!(
        instance_types(&grown),
        vec![(1, "round_4".to_string()), (2, "round_4".to_string())]
    );
}

fn swap_project() -> ProjectInput {
    let mut people: Vec<Person> = (1..=4)
        .map(|n| Person {
            id: format!("p{n}"),
            name: format!("Guest {n}"),
            table_type: None,
            groups: vec!["familia".to_string()],
            locked_table: None,
            locked_seat: None,
        })
        .collect();
    people[0].table_type = Some("rodona".to_string());
    people[0].locked_table = Some(1);
    people[0].locked_seat = Some(0);
    people.extend((5..=6).map(|n| Person {
        id: format!("p{n}"),
        name: format!("Guest {n}"),
        table_type: None,
        groups: vec![],
        locked_table: None,
        locked_seat: None,
    }));

    ProjectInput {
        people,
        closeness_rules: vec![
            ClosenessRule {
                left_id: "familia".to_string(),
                right_id: "familia".to_string(),
                score: 8.0,
            },
            ClosenessRule {
                left_id: "p5".to_string(),
                right_id: "p6".to_string(),
                score: 6.0,
            },
        ],
        ..order_project()
    }
}

fn swap_assignments() -> Vec<SeatingAssignment> {
    let mut assignments: Vec<SeatingAssignment> = (1..=4)
        .map(|n| SeatingAssignment {
            table_number: 1,
            table_type: "rodona".to_string(),
            seat_index: n - 1,
            person_id: format!("p{n}"),
            person_name: format!("Guest {n}"),
        })
        .collect();
    assignments.extend((5..=6).map(|n| SeatingAssignment {
        table_number: 2,
        table_type: "square".to_string(),
        seat_index: n - 5,
        person_id: format!("p{n}"),
        person_name: format!("Guest {n}"),
    }));
    assignments
}

/// Swapping table 1 and table 2 moves each table whole — type, shape and
/// guests — so the solution stays valid and scores identically even though
/// the two tables have different capacities.
#[test]
fn swap_table_numbers_moves_the_whole_table_with_its_guests() {
    let project = swap_project();
    let assignments = swap_assignments();
    let config = OptimizationConfig::default();
    let score_before = score_solution(&project, &assignments, &config).unwrap();
    assert_ne!(score_before, 0.0);

    let (order, remap) = swap_table_numbers(&project, 1, 2).unwrap();
    assert_eq!(order, ["square".to_string(), "rodona".to_string()]);
    assert_eq!(remap, BTreeMap::from([(1, 2), (2, 1)]));

    // Apply the swap the way a caller must: the order on the project, the
    // number map on the assignments and on every locked guest.
    let mut swapped = project.clone();
    swapped.table_order = order;
    for person in swapped.people.iter_mut() {
        if let Some(number) = person.locked_table {
            person.locked_table = remap.get(&number).copied();
        }
    }
    let moved: Vec<SeatingAssignment> = assignments
        .iter()
        .map(|a| SeatingAssignment {
            table_number: remap[&a.table_number],
            ..a.clone()
        })
        .collect();

    assert_eq!(
        instance_types(&swapped),
        vec![(1, "square".to_string()), (2, "rodona".to_string())]
    );
    let guests_at = |number: usize| {
        let mut ids: Vec<&str> = moved
            .iter()
            .filter(|a| a.table_number == number)
            .map(|a| a.person_id.as_str())
            .collect();
        ids.sort_unstable();
        ids
    };
    assert_eq!(guests_at(2), ["p1", "p2", "p3", "p4"]);
    assert_eq!(guests_at(1), ["p5", "p6"]);
    assert_eq!(
        swapped
            .people
            .iter()
            .find(|p| p.id == "p1")
            .unwrap()
            .locked_table,
        Some(2)
    );

    assert!(validate_seating_solution(&swapped, &moved).is_ok());
    // Bitwise equality here relies on the fixture's closeness scores (8.0,
    // 6.0) being integers — exact in f64 regardless of summation order —
    // not on score-preservation being bitwise in general.
    assert_eq!(
        score_solution(&swapped, &moved, &config).unwrap(),
        score_before
    );
}

#[test]
fn move_table_number_shifts_the_tables_between() {
    let simple = |count: usize| TableTypeConfig {
        shape: TableShape::Round,
        people_per_side: None,
        max_people: 4,
        recommended_people: None,
        min_people: None,
        number_of_tables: Some(count),
    };
    let project = ProjectInput {
        people: vec![],
        closeness_rules: vec![],
        table_types: build_table_type_map(vec![
            ("a".to_string(), simple(2)),
            ("b".to_string(), simple(1)),
            ("c".to_string(), simple(1)),
        ])
        .unwrap(),
        table_order: Vec::new(),
    };
    assert_eq!(instance_types(&project).len(), 4);

    let (order, remap) = move_table_number(&project, 4, 1).unwrap();

    let order_types: Vec<&str> = order.iter().map(String::as_str).collect();
    assert_eq!(order_types, ["c", "a", "a", "b"]);
    assert_eq!(remap, BTreeMap::from([(4, 1), (1, 2), (2, 3), (3, 4)]));
    assert!(move_table_number(&project, 5, 1).is_none());
    assert!(swap_table_numbers(&project, 1, 0).is_none());

    let moved = ProjectInput {
        table_order: order,
        ..project
    };
    assert_eq!(
        instance_types(&moved),
        vec![
            (1, "c".to_string()),
            (2, "a".to_string()),
            (3, "a".to_string()),
            (4, "b".to_string()),
        ]
    );
}

#[test]
fn project_file_round_trips_table_order() {
    let project = order_project();
    let empty_order_json = write_project_file(&ProjectFile::new(
        project.clone(),
        OptimizationConfig::default(),
        Vec::new(),
    ))
    .unwrap();
    // An empty order is not persisted, so existing files stay byte-identical.
    assert!(!empty_order_json.contains("table_order"));

    let ordered = ProjectInput {
        table_order: vec!["square".to_string(), "rodona".to_string()],
        ..project
    };
    let json = write_project_file(&ProjectFile::new(
        ordered.clone(),
        OptimizationConfig::default(),
        Vec::new(),
    ))
    .unwrap();
    let parsed = parse_project_file(&json).unwrap();
    assert_eq!(parsed.project_input().table_order, ordered.table_order);

    // A project file written before tables could be reordered still loads.
    let legacy = format!(
        r#"{{"version": {PROJECT_FILE_VERSION}, "people": [], "closeness_rules": [],
            "table_types": {{}}, "optimization": {{}}, "seating": []}}"#
    );
    assert!(parse_project_file(&legacy).unwrap().table_order.is_empty());
}

// ── Cluster exchange: multi-step moves ───────────────────────────────────

/// Four groups of 2 (`A`/`B`/`C`/`D`), each with a strong self-closeness (10)
/// and a weak cross-closeness to one other group (`A`-`C` and `B`-`D`, both
/// 3). Two round tables of exactly 4 (min = max = recommended, so table use
/// and size penalties never vary). Warm-started from `A+B` / `C+D` (the
/// within-table pairs already adjacent), the strictly better arrangement is
/// `A+C` / `B+D`: it gains the two cross-group bonuses on both tables
/// without ever giving up a same-group bonus. Reaching it from `A+B`/`C+D`
/// requires exchanging a coherent 2-person cluster from each table in a
/// single move — swapping `{a1,a2}` for `{d1,d2}` (or, symmetrically,
/// `{b1,b2}` for `{c1,c2}`), never `{a1,a2}` for `{c1,c2}` directly, since
/// `A` and `C` already sit on different tables. No sequence of single-guest
/// swaps or joins passes through a strictly-improving intermediate state,
/// since moving `a1` alone off table 1 breaks the `A`-`A` pair before any
/// `A`-`C` bonus is gained.
#[test]
fn optimizer_exchanges_coherent_clusters_between_tables() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\n\
         a1,A1,,A,,\na2,A2,,A,,\nb1,B1,,B,,\nb2,B2,,B,,\n\
         c1,C1,,C,,\nc2,C2,,C,,\nd1,D1,,D,,\nd2,D2,,D,,\n",
        "left_id,right_id,score\nA,A,10\nB,B,10\nC,C,10\nD,D,10\nA,C,3\nB,D,3\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround4,round,4,4,4,2,\n",
    )
    .unwrap();

    let initial: Vec<SeatingAssignment> = [
        ("a1", 1, 0),
        ("a2", 1, 1),
        ("b1", 1, 2),
        ("b2", 1, 3),
        ("c1", 2, 0),
        ("c2", 2, 1),
        ("d1", 2, 2),
        ("d2", 2, 3),
    ]
    .into_iter()
    .map(|(person_id, table_number, seat_index)| SeatingAssignment {
        table_number,
        table_type: "round4".to_string(),
        seat_index,
        person_id: person_id.to_string(),
        person_name: person_id.to_string(),
    })
    .collect();

    let config = OptimizationConfig {
        seed: 1,
        attempts: 1,
        steps: 5_000,
        time_limit_secs: 0,
        ..OptimizationConfig::default()
    };

    let result = HeuristicOptimizer
        .optimize_timed(&project, &config, Some(&initial))
        .unwrap();
    let assignments = &result.solutions[0].assignments;
    validate_seating_solution(&project, assignments).unwrap();

    let seat_of = |id: &str| assignments.iter().find(|a| a.person_id == id).unwrap();
    let table = seat_of("a1").table_number;
    assert_eq!(seat_of("a2").table_number, table);
    assert_eq!(
        seat_of("c1").table_number,
        table,
        "a1/a2 and c1/c2 did not end up sharing a table"
    );
    assert_eq!(seat_of("c2").table_number, table);
}

/// Eight guests (spare capacity: two `round6` tables, no warm start, so the
/// random initial split varies by seed) with unequal-size groups (`P`/`S`
/// singles, `Q`/`R` triples) and one locked guest per table sharing its
/// group with unlocked table-mates: `lt1` is locked to table 1 (no locked
/// seat) inside group `Q`, `ls1` is locked to table 2 seat 0 inside group
/// `R`. Whenever `propose_cluster_exchange` forms a cluster around a `Q` or
/// `R` member seated on the locked guest's own table, that cluster includes
/// the locked guest, and `may_sit_at` must refuse moving it off that table;
/// a `Q`/`R` member seated on the *other* table forms a lock-free cluster
/// instead, exercising the free-seat branch of the move. This must hold
/// across many seeds without ever corrupting the solution or the locks.
#[test]
fn optimizer_keeps_locked_guests_in_place_across_seeds_with_cluster_exchange() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\n\
         p1,P1,,P,,\nq1,Q1,,Q,,\nq2,Q2,,Q,,\nlt1,LT1,,Q,1,\n\
         s1,S1,,S,,\nr1,R1,,R,,\nr2,R2,,R,,\nls1,LS1,,R,2,0\n",
        "left_id,right_id,score\nQ,Q,5\nR,R,5\nP,S,2\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround6,round,6,,,2,\n",
    )
    .unwrap();

    for seed in 1..=5u64 {
        let config = OptimizationConfig {
            seed,
            attempts: 2,
            steps: 2_000,
            time_limit_secs: 0,
            ..OptimizationConfig::default()
        };
        let result = HeuristicOptimizer.optimize(&project, &config).unwrap();
        let assignments = &result.solutions[0].assignments;
        validate_seating_solution(&project, assignments).unwrap();

        let seat_of = |id: &str| assignments.iter().find(|a| a.person_id == id).unwrap();
        assert_eq!(seat_of("lt1").table_number, 1, "seed {seed}");
        assert_eq!(seat_of("ls1").table_number, 2, "seed {seed}");
        assert_eq!(seat_of("ls1").seat_index, 0, "seed {seed}");
    }
}
