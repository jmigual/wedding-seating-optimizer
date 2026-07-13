//! # seating-core
//!
//! Core library for the wedding seating optimizer.
//!
//! This crate contains all domain logic and is shared by the CLI (`seating-cli`)
//! and the native GUI (`seating-gui`).  Neither of those crates contains any
//! business logic of its own.
//!
//! ## Module layout
//!
//! | Module | Responsibility |
//! |--------|---------------|
//! | [`models`] | Domain types: guests, table configs, closeness rules, solutions, errors |
//! | [`io`] | CSV parsing and serialization |
//! | [`validation`] | Pre-optimization project validation and seating-solution validation |
//! | [`scoring`] | Distance functions, proximity weights, pairwise and solution scoring |
//! | [`optimizer`] | [`SeatingOptimizer`] trait and [`HeuristicOptimizer`] implementation |
//!
//! ## Typical usage
//!
//! ```no_run
//! use seating_core::{
//!     make_project, validate_project, HeuristicOptimizer, OptimizationConfig,
//!     SeatingOptimizer, write_seating_csv,
//! };
//!
//! let project = make_project(
//!     include_str!("../../../examples/people.csv"),
//!     include_str!("../../../examples/closeness.csv"),
//!     include_str!("../../../examples/tables.csv"),
//! )
//! .expect("parse inputs");
//!
//! validate_project(&project).expect("valid inputs");
//!
//! let result = HeuristicOptimizer.optimize(&project, &OptimizationConfig::default())
//!     .expect("optimize");
//!
//! let csv = write_seating_csv(&result.solutions[0].assignments).expect("serialize");
//! println!("{csv}");
//! ```

pub mod editing;
pub mod io;
pub mod models;
pub mod optimizer;
pub mod render;
pub mod scoring;
pub mod validation;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use editing::{
    apply_seat_drop, build_table_type_map, collect_group_ids, parse_f64_value,
    parse_optional_usize_value, parse_people_per_side, parse_required_usize_value,
    reference_id_options, reference_label, reference_matches, remove_group, rename_group,
    rules_match, ReferenceIdOption, SeatDropOutcome,
};
pub use io::{
    make_project, parse_closeness_csv, parse_people_csv, parse_project_file, parse_seating_csv,
    parse_tables_csv, write_closeness_csv, write_people_csv, write_project_file, write_seating_csv,
    write_tables_csv,
};
pub use models::{
    ClosenessRule, GroupId, OptimizationConfig, OptimizationResult, Person, PersonId, ProjectFile,
    ProjectInput, SeatIndex, SeatingAssignment, SeatingSolution, TableInstance, TableShape,
    TableTypeConfig, TableTypeId, ValidationError, ValidationReport, PROJECT_FILE_VERSION,
};
pub use optimizer::{HeuristicOptimizer, SeatingOptimizer};
pub use render::{
    build_layout, render_png, render_svg, LayoutSeat, LayoutTable, RenderOptions, RenderingError,
    SeatingLayout, TableSurface, COLOR_BACKGROUND, COLOR_CARD, COLOR_GUEST_TEXT, COLOR_MUTED,
    COLOR_SEAT_FILL, COLOR_SEAT_STROKE, COLOR_STROKE, COLOR_TABLE_FILL, COLOR_TABLE_STROKE,
};
pub use scoring::{
    circular_distance, default_proximity_weight, effective_person_pair_score, perimeter_distance,
    score_solution,
};
pub use validation::{generate_table_instances, validate_project, validate_seating_solution};
