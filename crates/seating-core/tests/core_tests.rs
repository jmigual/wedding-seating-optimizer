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
    assert!(
        matches!(err, ValidationError::MalformedInput(message) if message.contains("unsupported project file version"))
    );
}

#[test]
fn project_file_preserves_csv_representations() {
    let project = make_project(
        &write_people_csv(&sample_people()).unwrap(),
        &write_closeness_csv(&sample_closeness_rules()).unwrap(),
        &write_tables_csv(&sample_table_map()).unwrap(),
    )
    .unwrap();
    let project_file = ProjectFile::new(project, OptimizationConfig::default(), Vec::new());
    let parsed = parse_project_file(&write_project_file(&project_file).unwrap()).unwrap();

    assert_eq!(
        parse_people_csv(&write_people_csv(&parsed.people).unwrap()).unwrap(),
        sample_people()
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
fn layout_and_svg_skip_unused_tables_and_empty_seats() {
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
    assert_eq!(layout.tables[0].seats.len(), 2);

    let svg = render_svg(&layout, &RenderOptions::default());
    assert!(svg.contains("Alice"));
    assert!(svg.contains("Bob"));
    assert!(!svg.contains("Table 2"));
    assert!(!svg.contains(">Seat "));
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
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,fam,,\np2,B,,fam,,\np3,C,,work,,\np4,D,,work,,\n",
        "left_id,right_id,score\np1,p2,15\np3,p4,15\nfam,work,-2\n",
        "table_type_id,shape,max_people,recommended_people,min_people,number_of_tables,people_per_side\nround_4,round,4,4,,1,\n",
    )
    .unwrap();

    validate_project(&project).unwrap();
    let optimizer = HeuristicOptimizer;
    let result = optimizer
        .optimize(
            &project,
            &OptimizationConfig {
                seed: 1234,
                attempts: 10,
                iterations: 80,
                solutions: 1,
                optimal_table_size_weight: 0.5,
                ..OptimizationConfig::default()
            },
        )
        .unwrap();

    assert_eq!(result.solutions.len(), 1);
    assert_eq!(result.solutions[0].assignments.len(), 4);
    assert!(result.solutions[0].score.is_finite());
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
