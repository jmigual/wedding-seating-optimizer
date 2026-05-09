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
//! | [`io`] | CSV/JSON parsing and serialization |
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
//!     include_str!("../../../examples/tables.json"),
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

pub mod io;
pub mod models;
pub mod optimizer;
pub mod scoring;
pub mod validation;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use io::{
    make_project, parse_closeness_csv, parse_people_csv, parse_seating_csv, parse_tables_json,
    write_closeness_csv, write_people_csv, write_seating_csv, write_tables_json,
};
pub use models::{
    ClosenessRule, GroupId, OptimizationConfig, OptimizationResult, Person, PersonId,
    ProjectInput, SeatingAssignment, SeatingSolution, SeatIndex, TableInstance, TableShape,
    TableTypeConfig, TableTypeId, ValidationError, ValidationReport,
};
pub use optimizer::{HeuristicOptimizer, SeatingOptimizer};
pub use scoring::{
    circular_distance, default_proximity_weight, effective_person_pair_score, perimeter_distance,
    score_solution,
};
pub use validation::{generate_table_instances, validate_project, validate_seating_solution};
