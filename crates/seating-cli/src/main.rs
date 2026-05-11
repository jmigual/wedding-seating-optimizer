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
//! | `render`   | Render an existing seating CSV as SVG or PNG |
//!
//! ## Example
//!
//! ```text
//! # Validate inputs
//! wedding-seating validate \
//!     --people people.csv \
//!     --closeness closeness.csv \
//!     --tables tables.csv
//!
//! # Optimize and write result
//! wedding-seating optimize \
//!     --people people.csv \
//!     --closeness closeness.csv \
//!     --tables tables.csv \
//!     --output seating.csv \
//!     --seed 1234
//!
//! # Score a pre-existing seating
//! wedding-seating score \
//!     --people people.csv \
//!     --closeness closeness.csv \
//!     --tables tables.csv \
//!     --seating seating.csv
//!
//! # Render an existing seating plan
//! wedding-seating render \
//!     --people people.csv \
//!     --tables tables.csv \
//!     --seating seating.csv \
//!     --output seating-plan.svg
//! ```

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use seating_core::{
    build_layout, parse_people_csv, parse_seating_csv, parse_tables_csv, render_png, render_svg,
    score_solution, validate_project, write_seating_csv, HeuristicOptimizer, OptimizationConfig,
    ProjectInput, RenderOptions, SeatingOptimizer,
};
use std::fs;
use std::path::PathBuf;

/// Top-level CLI entry point.
#[derive(Parser, Debug)]
#[command(name = "wedding-seating")]
#[command(about = "Wedding seating validator, optimizer, scorer, and renderer")]
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
        /// Path to the tables CSV file.
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
        /// Path to the tables CSV file.
        #[arg(long)]
        tables: PathBuf,
        /// Destination path for the output seating CSV.
        #[arg(long)]
        output: PathBuf,
        /// RNG seed for reproducible runs.
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Number of independent random-restart attempts.
        #[arg(long, default_value_t = 10)]
        attempts: usize,
        /// Number of top solutions to keep (only the best is written to output).
        #[arg(long, default_value_t = 1)]
        solutions: usize,
        /// Local-improvement swap iterations per attempt.
        #[arg(long, default_value_t = 200)]
        iterations: usize,
        /// Global multiplier for closeness and proximity scoring.
        #[arg(long, default_value_t = 1.0)]
        proximity_weight: f64,
        /// Penalty subtracted for every used table.
        #[arg(long, default_value_t = 0.0)]
        used_table_weight: f64,
        /// Weight applied to the penalty for deviating from recommended table size.
        #[arg(long, default_value_t = 1.0)]
        optimal_table_size_weight: f64,
    },

    /// Score a pre-existing seating CSV and print the aggregate score.
    Score {
        /// Path to the people CSV file.
        #[arg(long)]
        people: PathBuf,
        /// Path to the closeness CSV file.
        #[arg(long)]
        closeness: PathBuf,
        /// Path to the tables CSV file.
        #[arg(long)]
        tables: PathBuf,
        /// Path to the seating CSV to score.
        #[arg(long)]
        seating: PathBuf,
        /// Global multiplier for closeness and proximity scoring.
        #[arg(long, default_value_t = 1.0)]
        proximity_weight: f64,
        /// Penalty subtracted for every used table.
        #[arg(long, default_value_t = 0.0)]
        used_table_weight: f64,
        /// Weight applied to the penalty for deviating from recommended table size.
        #[arg(long, default_value_t = 1.0)]
        optimal_table_size_weight: f64,
    },

    /// Render a seating CSV as an SVG or PNG seating plan.
    Render {
        /// Path to the people CSV file.
        #[arg(long)]
        people: PathBuf,
        /// Path to the tables CSV file.
        #[arg(long)]
        tables: PathBuf,
        /// Path to the seating CSV to render.
        #[arg(long)]
        seating: PathBuf,
        /// Destination `.svg` or `.png` output path.
        #[arg(long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Validate {
            people,
            closeness,
            tables,
        } => {
            let project = seating_core::make_project(
                &read_file(&people, "people")?,
                &read_file(&closeness, "closeness")?,
                &read_file(&tables, "tables")?,
            )?;
            validate_project(&project).map_err(|error| anyhow::anyhow!(error.to_string()))?;
            println!("Validation passed.");
        }
        Commands::Optimize {
            people,
            closeness,
            tables,
            output,
            seed,
            attempts,
            solutions,
            iterations,
            proximity_weight,
            used_table_weight,
            optimal_table_size_weight,
        } => {
            let project = seating_core::make_project(
                &read_file(&people, "people")?,
                &read_file(&closeness, "closeness")?,
                &read_file(&tables, "tables")?,
            )?;
            validate_project(&project).map_err(|error| anyhow::anyhow!(error.to_string()))?;
            let result = HeuristicOptimizer.optimize(
                &project,
                &OptimizationConfig {
                    seed,
                    attempts,
                    iterations,
                    solutions,
                    proximity_weight,
                    used_table_weight,
                    optimal_table_size_weight,
                },
            )?;
            let best = result
                .solutions
                .first()
                .ok_or_else(|| anyhow::anyhow!("optimizer returned no solutions"))?;
            fs::write(&output, write_seating_csv(&best.assignments)?)
                .with_context(|| format!("failed writing output {}", output.display()))?;
            println!(
                "Wrote seating to {} with score {}",
                output.display(),
                best.score
            );
        }
        Commands::Score {
            people,
            closeness,
            tables,
            seating,
            proximity_weight,
            used_table_weight,
            optimal_table_size_weight,
        } => {
            let project = seating_core::make_project(
                &read_file(&people, "people")?,
                &read_file(&closeness, "closeness")?,
                &read_file(&tables, "tables")?,
            )?;
            let assignments = parse_seating_csv(&read_file(&seating, "seating")?)?;
            let score = score_solution(
                &project,
                &assignments,
                &OptimizationConfig {
                    proximity_weight,
                    used_table_weight,
                    optimal_table_size_weight,
                    ..OptimizationConfig::default()
                },
            )
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            println!("Score: {score}");
        }
        Commands::Render {
            people,
            tables,
            seating,
            output,
        } => {
            let project = ProjectInput {
                people: parse_people_csv(&read_file(&people, "people")?)?,
                closeness_rules: Vec::new(),
                table_types: parse_tables_csv(&read_file(&tables, "tables")?)?,
            };
            let assignments = parse_seating_csv(&read_file(&seating, "seating")?)?;
            let layout = build_layout(&project, &assignments)
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            let options = RenderOptions::default();
            match output
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.to_ascii_lowercase())
                .as_deref()
            {
                Some("svg") => fs::write(&output, render_svg(&layout, &options))
                    .with_context(|| format!("failed writing output {}", output.display()))?,
                Some("png") => render_png(&layout, &options, &output)
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?,
                _ => return Err(anyhow::anyhow!("render output must end with .svg or .png")),
            }
            println!("Rendered seating plan to {}", output.display());
        }
    }
    Ok(())
}

/// Read a text file, providing a helpful error message on failure.
fn read_file(path: &PathBuf, label: &str) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("failed reading {label} file {}", path.display()))
}
