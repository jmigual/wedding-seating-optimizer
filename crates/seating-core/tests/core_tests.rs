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
    assert!(err
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::NamespaceCollision(id) if id == "p1")));
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
fn multiple_group_maximum_rule_applies() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\na1,A1,,g1|g2,,\na2,A2,,g3|g4,,\n",
        "left_id,right_id,score\ng1,g3,5\ng2,g4,9\n",
        sample_tables_csv(),
    )
    .unwrap();
    let score =
        effective_person_pair_score(&project, &project.people[0], &project.people[1]).unwrap();
    assert_eq!(score, 9.0);
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
fn capacity_validation_catches_insufficient_space() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,g,,\np2,B,,g,,\np3,C,,g,,\np4,D,,g,,\np5,E,,g,,\n",
        "left_id,right_id,score\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,,,1,\n",
    )
    .unwrap();
    let err = validate_project(&project).unwrap_err();
    assert!(err
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::NotEnoughSeats { .. })));
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
    assert!(err
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::LockedTableDoesNotExist { .. })));
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
    assert!(err
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::LockedSeatOutOfRange { .. })));
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
                iterations: 200,
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
                iterations: 800,
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
        iterations: 100,
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
                iterations: 150,
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
    assert!(err
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::MissingOrDuplicatePerson(id) if id == "p1")));
    assert!(err
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::MissingOrDuplicatePerson(id) if id == "p4")));
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

#[test]
fn table_below_min_is_detected() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,,,\np2,B,,,,\np3,C,,,,\n",
        "left_id,right_id,score\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,,2,2,\n",
    )
    .unwrap();
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
    let err = validate_seating_solution(&project, &assignments).unwrap_err();
    assert!(err.errors.iter().any(|e| matches!(
        e,
        ValidationError::TableBelowMin {
            table_number: 2,
            count: 1,
            min: 2
        }
    )));
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
    let project = ProjectInput {
        people: vec![
            Person {
                id: "p1".to_string(),
                name: "A".to_string(),
                table_type: Some("round_6".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
            Person {
                id: "p2".to_string(),
                name: "B".to_string(),
                table_type: Some("round_6".to_string()),
                groups: vec![],
                locked_table: None,
                locked_seat: None,
            },
        ],
        closeness_rules: vec![ClosenessRule {
            left_id: "p1".to_string(),
            right_id: "p2".to_string(),
            score: 10.0,
        }],
        table_types,
    };
    let config = OptimizationConfig::default();

    let adjacent = vec![
        SeatingAssignment {
            table_number: 1,
            table_type: "round_6".to_string(),
            seat_index: 0,
            person_id: "p1".to_string(),
            person_name: "A".to_string(),
        },
        SeatingAssignment {
            table_number: 1,
            table_type: "round_6".to_string(),
            seat_index: 1,
            person_id: "p2".to_string(),
            person_name: "B".to_string(),
        },
    ];
    let mut distant = adjacent.clone();
    distant[1].seat_index = 3;

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
                iterations: 300,
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
fn infeasible_min_constraints_yield_error() {
    // 5 people, 2 tables of max=4/min=3: every split (5+0, 4+1, 3+2) either
    // exceeds a table's capacity or leaves a used table below its minimum —
    // genuinely infeasible for the heuristic, deterministically, regardless
    // of seed.
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

    let err = HeuristicOptimizer
        .optimize(&project, &OptimizationConfig::default())
        .unwrap_err();
    assert!(err
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::NoFeasibleAssignment)));
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
