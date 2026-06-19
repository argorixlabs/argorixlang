use anyhow::{Context, Result};
use argorix_conformance::{run_suite, types::ConformanceSuite};
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "argorix-conformance",
    version,
    about = "Argorix Lang Conformance Suite runner"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run {
        suite: PathBuf,
        #[arg(long, default_value = "target/argorix-conformance")]
        workdir: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<bool> {
    match Cli::parse().command {
        Command::Run {
            suite,
            workdir,
            json,
        } => {
            let source = fs::read_to_string(&suite)
                .with_context(|| format!("failed to read suite `{}`", suite.display()))?;
            let suite_data: ConformanceSuite = serde_json::from_str(&source)
                .with_context(|| format!("invalid suite JSON in `{}`", suite.display()))?;
            let result = run_suite(&suite_data, &suite, &workdir)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("Argorix Conformance Suite v{}", result.suite_version);
                println!("Cases: {}", result.cases_total);
                println!("Passed: {}", result.cases_passed);
                println!("Failed: {}", result.cases_failed);
                println!(
                    "Conformance: {}",
                    if result.passed { "passed" } else { "failed" }
                );
                for failure in &result.failures {
                    println!(
                        "- {} [{}]: {}",
                        failure.case_id, failure.stage, failure.reason
                    );
                }
            }
            Ok(result.passed)
        }
    }
}
