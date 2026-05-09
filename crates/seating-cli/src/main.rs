use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use seating_core::{
    make_project, parse_seating_csv, score_solution, validate_project, write_seating_csv, HeuristicOptimizer,
    OptimizationConfig, SeatingOptimizer,
};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "wedding-seating")]
#[command(about = "Wedding seating validator/optimizer/scorer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Validate {
        #[arg(long)]
        people: PathBuf,
        #[arg(long)]
        closeness: PathBuf,
        #[arg(long)]
        tables: PathBuf,
    },
    Optimize {
        #[arg(long)]
        people: PathBuf,
        #[arg(long)]
        closeness: PathBuf,
        #[arg(long)]
        tables: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value_t = 1)]
        solutions: usize,
        #[arg(long, default_value_t = 200)]
        iterations: usize,
        #[arg(long, default_value_t = 1.0)]
        recommended_weight: f64,
    },
    Score {
        #[arg(long)]
        people: PathBuf,
        #[arg(long)]
        closeness: PathBuf,
        #[arg(long)]
        tables: PathBuf,
        #[arg(long)]
        seating: PathBuf,
        #[arg(long, default_value_t = 1.0)]
        recommended_weight: f64,
    },
}

fn read_file(path: &PathBuf, label: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed reading {label} file {}", path.display()))
}

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
            let optimizer = HeuristicOptimizer;
            let result = optimizer.optimize(
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
            let score =
                score_solution(&project, &assignments, recommended_weight).map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("Score: {score}");
        }
    }
    Ok(())
}
