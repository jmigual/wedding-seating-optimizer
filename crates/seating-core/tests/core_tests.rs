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

#[test]
fn layout_with_empty_tables_includes_every_instance() {
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

    let instances = generate_table_instances(&project);
    let full_layout = build_layout_with_empty_tables(&project, &assignments).unwrap();
    let used_layout = build_layout(&project, &assignments).unwrap();

    assert_eq!(full_layout.tables.len(), instances.len());
    assert!(full_layout.tables.len() > used_layout.tables.len());

    let empty_table = full_layout
        .tables
        .iter()
        .find(|table| table.table_number == 2)
        .unwrap();
    assert_eq!(empty_table.seats.len(), 4);
    assert!(
        empty_table
            .seats
            .iter()
            .all(|seat| seat.person_name.is_none())
    );
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
fn svg_truncates_long_guest_labels_and_keeps_full_name_in_title() {
    let project = round_project();
    let mut assignments = round_assignments();
    assignments[0].person_name = "Alexandria Montgomery-Featherstonehaugh".to_string();

    let layout = build_layout(&project, &assignments).unwrap();
    let svg = render_svg(&layout, &RenderOptions::default());

    assert!(svg.contains("<title>Alexandria Montgomery-Featherstonehaugh</title>"));
    // The full name must appear only once (inside <title>) — the visible
    // guest label is truncated with an ellipsis, not the raw string.
    assert_eq!(
        svg.matches("Alexandria Montgomery-Featherstonehaugh")
            .count(),
        1
    );
    assert!(svg.contains('…'));
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

/// Compaction moves each type's used tables onto its lowest-numbered
/// instances, preserving seat indices, and is score-neutral because
/// same-type table instances are identical for scoring.
#[test]
fn compact_table_numbers_moves_used_tables_to_the_lowest_numbers() {
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

    let compacted = compact_table_numbers(&project, &assignments);

    let table_of = |compacted: &[SeatingAssignment], person_id: &str| {
        let a = compacted.iter().find(|a| a.person_id == person_id).unwrap();
        (a.table_number, a.seat_index)
    };
    assert_eq!(table_of(&compacted, "p1"), (1, 0));
    assert_eq!(table_of(&compacted, "p2"), (1, 1));
    assert_eq!(table_of(&compacted, "p3"), (2, 0));
    assert_eq!(table_of(&compacted, "p4"), (2, 1));
    assert_eq!(table_of(&compacted, "p5"), (4, 0));

    assert!(validate_seating_solution(&project, &compacted).is_ok());

    let config = OptimizationConfig::default();
    let score_before = score_solution(&project, &assignments, &config).unwrap();
    let score_after = score_solution(&project, &compacted, &config).unwrap();
    assert_eq!(score_before, score_after);
}

/// A table holding a guest with `locked_table` set is pinned: it keeps its
/// number, and the other used tables of that type fill the remaining lowest
/// non-pinned numbers.
#[test]
fn compact_table_numbers_keeps_locked_tables_in_place() {
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

    let compacted = compact_table_numbers(&project, &assignments);

    let table_of = |compacted: &[SeatingAssignment], person_id: &str| {
        compacted
            .iter()
            .find(|a| a.person_id == person_id)
            .unwrap()
            .table_number
    };
    assert_eq!(table_of(&compacted, "p1"), 1);
    assert_eq!(table_of(&compacted, "p2"), 1);
    assert_eq!(table_of(&compacted, "p3"), 3);
    assert_eq!(table_of(&compacted, "p4"), 3);
    assert_eq!(table_of(&compacted, "p5"), 4);

    assert!(validate_seating_solution(&project, &compacted).is_ok());
}

/// With a non-empty [`ProjectInput::table_order`] (`b`, then `a`, then `a` —
/// so `b` is #1 and `a`'s two instances are #2 and #3), compaction still
/// groups by table type: table 3 (type `a`, occupied) moves onto table 2
/// (type `a`, empty), never onto table 1, which belongs to type `b`.
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

    let compacted = compact_table_numbers(&project, &assignments);
    let table_of = |compacted: &[SeatingAssignment], person_id: &str| {
        compacted
            .iter()
            .find(|a| a.person_id == person_id)
            .unwrap()
            .table_number
    };
    assert_eq!(table_of(&compacted, "p1"), 1);
    assert_eq!(table_of(&compacted, "p2"), 2);

    assert!(validate_seating_solution(&project, &compacted).is_ok());

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
