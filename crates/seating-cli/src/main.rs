//! # wedding-seating CLI
//!
//! Command-line interface for the wedding seating optimizer.
//!
//! All business logic lives in the `seating-core` crate; this binary is
//! responsible only for argument parsing, file I/O, and formatting output.
//!
//! ## Subcommands
//!
//! | Command    | Purpose |
//! |------------|---------|
//! | `validate` | Parse and validate the three input files, reporting all errors |
//! | `optimize` | Run the optimizer and write the seating CSV |
//! | `score`    | Score an existing seating CSV against the input files |
//!
//! ## Example
//!
//! ```text
//! # Validate inputs
//! wedding-seating validate \
//!     --people people.csv \
//!     --closeness closeness.csv \
//!     --tables tables.json
//!
//! # Optimize and write result
//! wedding-seating optimize \
//!     --people people.csv \
//!     --closeness closeness.csv \
//!     --tables tables.json \
//!     --output seating.csv \
//!     --seed 1234
//!
//! # Score a pre-existing seating
//! wedding-seating score \
//!     --people people.csv \
//!     --closeness closeness.csv \
//!     --tables tables.json \
//!     --seating seating.csv
//! ```

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use seating_core::{
    make_project, parse_seating_csv, score_solution, validate_project, write_seating_csv,
    HeuristicOptimizer, OptimizationConfig, SeatingOptimizer,
};
use std::fs;
use std::path::PathBuf;

// ── CLI definition ────────────────────────────────────────────────────────────

/// Top-level CLI entry point.
#[derive(Parser, Debug)]
#[command(name = "wedding-seating")]
#[command(about = "Wedding seating validator, optimizer, and scorer")]
#[command(long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Parse and validate the input files, printing any errors found.
    ///
    /// Exits with a non-zero status code when validation fails.
    Validate {
        /// Path to the people CSV file.
        #[arg(long)]
        people: PathBuf,
        /// Path to the closeness CSV file.
        #[arg(long)]
        closeness: PathBuf,
        /// Path to the tables JSON file.
        #[arg(long)]
        tables: PathBuf,
    },

    /// Optimize the seating assignment and write the result CSV.
    ///
    /// Exits with a non-zero status code when optimization fails.
    Optimize {
        /// Path to the people CSV file.
        #[arg(long)]
        people: PathBuf,
        /// Path to the closeness CSV file.
        #[arg(long)]
        closeness: PathBuf,
        /// Path to the tables JSON file.
        #[arg(long)]
        tables: PathBuf,
        /// Destination path for the output seating CSV.
        #[arg(long)]
        output: PathBuf,
        /// RNG seed for reproducible runs.
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Number of top solutions to keep (only the best is written to output).
        #[arg(long, default_value_t = 1)]
        solutions: usize,
        /// Local-improvement iterations per optimization attempt.
        #[arg(long, default_value_t = 200)]
        iterations: usize,
        /// Weight applied to the penalty for deviating from recommended table size.
        #[arg(long, default_value_t = 1.0)]
        recommended_weight: f64,
    },

    /// Score a pre-existing seating CSV and print the aggregate score.
    Score {
        /// Path to the people CSV file.
        #[arg(long)]
        people: PathBuf,
        /// Path to the closeness CSV file.
        #[arg(long)]
        closeness: PathBuf,
        /// Path to the tables JSON file.
        #[arg(long)]
        tables: PathBuf,
        /// Path to the seating CSV to score.
        #[arg(long)]
        seating: PathBuf,
        /// Weight applied to the penalty for deviating from recommended table size.
        #[arg(long, default_value_t = 1.0)]
        recommended_weight: f64,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Validate {
            people,
            closeness,
            tables,
        } => {
            let project = make_project(
                &read_file(&people, "people")?,
                &read_file(&closeness, "closeness")?,
                &read_file(&tables, "tables")?,
            )?;
            validate_project(&project).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("Validation passed.");
        }

        Commands::Optimize {
            people,
            closeness,
            tables,
            output,
            seed,
            solutions,
            iterations,
            recommended_weight,
        } => {
            let project = make_project(
                &read_file(&people, "people")?,
                &read_file(&closeness, "closeness")?,
                &read_file(&tables, "tables")?,
            )?;
            validate_project(&project).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let result = HeuristicOptimizer.optimize(
                &project,
                &OptimizationConfig {
                    seed,
                    iterations,
                    solutions,
                    recommended_capacity_weight: recommended_weight,
                },
            )?;
            let best = result
                .solutions
                .first()
                .ok_or_else(|| anyhow::anyhow!("optimizer returned no solutions"))?;
            fs::write(&output, write_seating_csv(&best.assignments)?)
                .with_context(|| format!("failed writing output {}", output.display()))?;
            println!("Wrote seating to {} with score {}", output.display(), best.score);
        }

        Commands::Score {
            people,
            closeness,
            tables,
            seating,
            recommended_weight,
        } => {
            let project = make_project(
                &read_file(&people, "people")?,
                &read_file(&closeness, "closeness")?,
                &read_file(&tables, "tables")?,
            )?;
            let assignments = parse_seating_csv(&read_file(&seating, "seating")?)?;
            let score = score_solution(&project, &assignments, recommended_weight)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("Score: {score}");
        }
    }
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Read a text file, providing a helpful error message on failure.
fn read_file(path: &PathBuf, label: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed reading {label} file {}", path.display()))
}
