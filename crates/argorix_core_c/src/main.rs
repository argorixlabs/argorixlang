//! ESP-008.R harness: run the Core runtime cases through the transitional C
//! backend and check that the resulting executables do not depend on Rust.
//!
//! Emission and execution are separate subcommands so execution can be shown
//! on a host with no Rust toolchain. See `conformance/core_c/README.md`.

mod elf;
mod gaps;
mod generate;
mod harness;
mod policy;
mod run;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "core-c-harness",
    about = "Execute Argorix Core cases through the transitional C backend (ESP-008.R)"
)]
struct Cli {
    /// Repository root; defaults to the working directory.
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    /// Where bundles, executables and reports are written.
    #[arg(long, global = true, default_value = "target/core-c")]
    work_dir: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Emit C for every case into a bundle (needs argorixc).
    Emit {
        #[arg(long)]
        argorixc: PathBuf,
        #[arg(long)]
        bundle: Option<PathBuf>,
        #[arg(long)]
        cases: Option<PathBuf>,
    },
    /// Compile, execute and inspect a bundle (needs only a C compiler).
    Run {
        #[arg(long)]
        bundle: Option<PathBuf>,
        #[arg(long)]
        cases: Option<PathBuf>,
        /// Allow-listed compiler name or an absolute path.
        #[arg(long)]
        cc: String,
        #[arg(long)]
        report: Option<PathBuf>,
        /// Run the cases only; the report then cannot pass.
        #[arg(long)]
        skip_negative_controls: bool,
        /// Compile and run with AddressSanitizer and UndefinedBehaviorSanitizer.
        #[arg(long)]
        sanitize: bool,
        /// Run each executable this many times and require identical output.
        #[arg(long, default_value_t = 1)]
        repeat: u32,
        /// Fail if rustc, cargo or rustup is on PATH of this host.
        #[arg(long)]
        require_rust_free_host: bool,
    },
    /// Emit and run in one process, as CI does.
    All {
        #[arg(long)]
        argorixc: PathBuf,
        #[arg(long)]
        bundle: Option<PathBuf>,
        #[arg(long)]
        cases: Option<PathBuf>,
        #[arg(long)]
        cc: String,
        #[arg(long)]
        report: Option<PathBuf>,
        #[arg(long)]
        skip_negative_controls: bool,
        /// Compile and run with AddressSanitizer and UndefinedBehaviorSanitizer.
        #[arg(long)]
        sanitize: bool,
        /// Run each executable this many times and require identical output.
        #[arg(long, default_value_t = 1)]
        repeat: u32,
    },
    /// Check the known-gap corpus (issue #27).
    Gaps {
        #[arg(long)]
        argorixc: PathBuf,
        #[arg(long)]
        cc: String,
        #[arg(long)]
        report: Option<PathBuf>,
        /// Print ::notice lines for gaps that are fixed or changed.
        #[arg(long)]
        github_annotations: bool,
    },
    /// Generate random programs with spec-derived expectations.
    Generate {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 40)]
        count: usize,
        #[arg(long)]
        out: PathBuf,
    },
}

fn write_report<T: serde::Serialize>(path: &Path, report: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(report)? + "\n")
        .with_context(|| format!("failed to write {}", path.display()))?;
    println!("report: {}", path.display());
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = match cli.root {
        Some(path) => path,
        None => std::env::current_dir()?,
    };
    let work = if cli.work_dir.is_absolute() {
        cli.work_dir.clone()
    } else {
        root.join(&cli.work_dir)
    };
    let default_cases = root.join("tests/selfhost/runtime/cases.json");

    let code = match cli.command {
        Command::Generate { seed, count, out } => {
            let out = if out.is_absolute() {
                out
            } else {
                root.join(out)
            };
            let manifest = generate::build_corpus(seed, count, &out)?;
            // The mix matters: a program that traps stops there, so a corpus
            // that is nearly all traps checks little of what follows one.
            let mut outcomes: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for case in &manifest.cases {
                let outcome = if case.expected_exit == 70 {
                    case.expected_stderr
                        .strip_prefix("ARGORIX_TRAP:")
                        .unwrap_or("TRAP")
                } else {
                    "RESULT"
                };
                *outcomes.entry(outcome).or_default() += 1;
            }
            let mix = outcomes
                .iter()
                .map(|(outcome, count)| format!("{outcome} {count}"))
                .collect::<Vec<_>>()
                .join(", ");
            println!(
                "generated {} cases in {} (seed {seed}, {} attempts): {mix}",
                manifest.cases.len(),
                out.display(),
                manifest.attempts
            );
            i32::from(manifest.cases.is_empty())
        }
        Command::Emit {
            argorixc,
            bundle,
            cases,
        } => {
            let cases = cases.unwrap_or(default_cases);
            let bundle = bundle.unwrap_or_else(|| work.join("bundle"));
            let manifest = harness::emit(&argorixc, &root, &cases, &bundle)?;
            let emitted = manifest
                .cases
                .iter()
                .filter(|case| case.emit_exit == 0)
                .count();
            println!(
                "emitted {emitted}/{} cases into {}",
                manifest.cases.len(),
                bundle.display()
            );
            i32::from(emitted != manifest.cases.len())
        }
        Command::Run {
            bundle,
            cases,
            cc,
            report,
            skip_negative_controls,
            sanitize,
            repeat,
            require_rust_free_host,
        } => {
            let cases = cases.unwrap_or(default_cases);
            let bundle = bundle.unwrap_or_else(|| work.join("bundle"));
            let outcome = run::run(run::RunOptions {
                root: &root,
                bundle_dir: &bundle,
                cases_path: &cases,
                compiler_name: &cc,
                work: &work.join("build"),
                with_controls: !skip_negative_controls,
                sanitize,
                repeat,
                require_rust_free: require_rust_free_host,
            })?;
            run::print_summary(&outcome);
            write_report(
                &report.unwrap_or_else(|| work.join("report.json")),
                &outcome,
            )?;
            i32::from(!outcome.overall_pass)
        }
        Command::All {
            argorixc,
            bundle,
            cases,
            cc,
            report,
            skip_negative_controls,
            sanitize,
            repeat,
        } => {
            let cases = cases.unwrap_or(default_cases);
            let bundle = bundle.unwrap_or_else(|| work.join("bundle"));
            let emitted = harness::emit(&argorixc, &root, &cases, &bundle)?;
            println!(
                "emitted {}/{} cases into {}",
                emitted
                    .cases
                    .iter()
                    .filter(|case| case.emit_exit == 0)
                    .count(),
                emitted.cases.len(),
                bundle.display()
            );
            let outcome = run::run(run::RunOptions {
                root: &root,
                bundle_dir: &bundle,
                cases_path: &cases,
                compiler_name: &cc,
                work: &work.join("build"),
                with_controls: !skip_negative_controls,
                sanitize,
                repeat,
                require_rust_free: false,
            })?;
            run::print_summary(&outcome);
            write_report(
                &report.unwrap_or_else(|| work.join("report.json")),
                &outcome,
            )?;
            i32::from(!outcome.overall_pass)
        }
        Command::Gaps {
            argorixc,
            cc,
            report,
            github_annotations,
        } => {
            let outcome = gaps::check(&argorixc, &root, &cc, &work.join("gaps"))?;
            gaps::print_summary(&outcome, github_annotations);
            write_report(
                &report.unwrap_or_else(|| work.join("gaps-report.json")),
                &outcome,
            )?;
            i32::from(!outcome.overall_pass)
        }
    };
    std::process::exit(code);
}
