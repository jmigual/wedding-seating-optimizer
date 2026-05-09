use seating_core::*;

fn sample_tables_json() -> &'static str {
    r#"{
      "round_4": {
        "shape": "round",
        "max_people": 4,
        "recommended_people": 4,
        "min_people": 2,
        "number_of_tables": 2
      },
      "rect_4": {
        "shape": "rectangular",
        "people_per_side": [1,1,1,1],
        "max_people": 4,
        "recommended_people": 4,
        "min_people": 2,
        "number_of_tables": 1
      }
    }"#
}

#[test]
fn people_csv_parsing_works() {
    let csv = "id,name,table_type,groups,locked_table,locked_seat\np1,Alice,round_4,family|friends,,\n";
    let people = parse_people_csv(csv).unwrap();
    assert_eq!(people.len(), 1);
    assert_eq!(people[0].groups, vec!["family", "friends"]);
}

#[test]
fn closeness_csv_parsing_works() {
    let csv = "left_id,right_id,score\np1,p2,20\nfamily,family,10\n";
    let closeness = parse_closeness_csv(csv).unwrap();
    assert_eq!(closeness.len(), 2);
    assert_eq!(closeness[0].score, 20.0);
}

#[test]
fn tables_json_parsing_works() {
    let tables = parse_tables_json(sample_tables_json()).unwrap();
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
fn id_namespace_collision_is_rejected() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,Alice,,p1,,\n",
        "left_id,right_id,score\n",
        sample_tables_json(),
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
        sample_tables_json(),
    )
    .unwrap();
    let score = effective_person_pair_score(&project, &project.people[0], &project.people[1]).unwrap();
    assert_eq!(score, 10.0);
}

#[test]
fn multiple_group_maximum_rule_applies() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\na1,A1,,g1|g2,,\na2,A2,,g3|g4,,\n",
        "left_id,right_id,score\ng1,g3,5\ng2,g4,9\n",
        sample_tables_json(),
    )
    .unwrap();
    let score = effective_person_pair_score(&project, &project.people[0], &project.people[1]).unwrap();
    assert_eq!(score, 9.0);
}

#[test]
fn person_pair_score_adds_to_group_score() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\na1,A1,,fam,,\na2,A2,,fam,,\n",
        "left_id,right_id,score\na1,a2,5\nfam,fam,10\n",
        sample_tables_json(),
    )
    .unwrap();
    let score = effective_person_pair_score(&project, &project.people[0], &project.people[1]).unwrap();
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
        r#"{
          "round_4": {"shape":"round","max_people":4,"number_of_tables":1}
        }"#,
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
        sample_tables_json(),
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
        sample_tables_json(),
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
        sample_tables_json(),
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
        r#"{
          "round_4": {"shape":"round","max_people":4,"recommended_people":3,"number_of_tables":1}
        }"#,
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

    let score = score_solution(&project, &seating, 1.0).unwrap();
    assert!(score > 10.0);
}

#[test]
fn integration_style_optimization_test() {
    let project = make_project(
        "id,name,table_type,groups,locked_table,locked_seat\np1,A,,fam,,\np2,B,,fam,,\np3,C,,work,,\np4,D,,work,,\n",
        "left_id,right_id,score\np1,p2,15\np3,p4,15\nfam,work,-2\n",
        r#"{
          "round_4": {"shape":"round","max_people":4,"recommended_people":4,"number_of_tables":1}
        }"#,
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
                recommended_capacity_weight: 0.5,
            },
        )
        .unwrap();

    assert_eq!(result.solutions.len(), 1);
    assert_eq!(result.solutions[0].assignments.len(), 4);
    assert!(result.solutions[0].score.is_finite());
}
