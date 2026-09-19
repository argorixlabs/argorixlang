use anyhow::{bail, Context, Result};
use argorix_conformance::core_spec::{validate_core_spec, CaseManifest, CoreSpec};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    task: &'static str,
    baseline_commit: String,
    scope: &'static str,
    spec_manifest_sha256: String,
    fixture_manifest_sha256: String,
    validation: argorix_conformance::core_spec::CoreValidation,
    mutation_control_detected: bool,
    syntax_execution: &'static str,
    overall_pass: bool,
    not_proven: Vec<&'static str>,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn main() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let spec_path = PathBuf::from(
        args.next()
            .context("usage: core-spec-check CORE_SPEC CASES SPEC_DIR FIXTURE_DIR OUTPUT")?,
    );
    let cases_path = PathBuf::from(
        args.next()
            .context("usage: core-spec-check CORE_SPEC CASES SPEC_DIR FIXTURE_DIR OUTPUT")?,
    );
    let spec_dir = PathBuf::from(
        args.next()
            .context("usage: core-spec-check CORE_SPEC CASES SPEC_DIR FIXTURE_DIR OUTPUT")?,
    );
    let fixture_dir = PathBuf::from(
        args.next()
            .context("usage: core-spec-check CORE_SPEC CASES SPEC_DIR FIXTURE_DIR OUTPUT")?,
    );
    let output = PathBuf::from(
        args.next()
            .context("usage: core-spec-check CORE_SPEC CASES SPEC_DIR FIXTURE_DIR OUTPUT")?,
    );
    if args.next().is_some() {
        bail!("usage: core-spec-check CORE_SPEC CASES SPEC_DIR FIXTURE_DIR OUTPUT");
    }
    let spec_raw = fs::read(&spec_path).with_context(|| format!("read {}", spec_path.display()))?;
    let cases_raw =
        fs::read(&cases_path).with_context(|| format!("read {}", cases_path.display()))?;
    let spec: CoreSpec = serde_json::from_slice(&spec_raw).context("parse Core spec")?;
    let cases: CaseManifest = serde_json::from_slice(&cases_raw).context("parse Core cases")?;
    let validation = validate_core_spec(&spec, &cases, &spec_dir, &fixture_dir);

    let mut mutated = spec.clone();
    mutated.required_constructs.push("mutation_canary".into());
    let mutation_control_detected =
        !validate_core_spec(&mutated, &cases, &spec_dir, &fixture_dir).passed;
    let overall_pass = validation.passed && mutation_control_detected;
    let report = Report {
        schema_version: 1,
        task: "ESP-004",
        baseline_commit: option_env!("GIT_COMMIT").unwrap_or("UNSPECIFIED").into(),
        scope: "Executable specification inventory and corpus contract; not a Core parser/compiler/runtime",
        spec_manifest_sha256: digest(&spec_raw),
        fixture_manifest_sha256: digest(&cases_raw),
        validation,
        mutation_control_detected,
        syntax_execution: "NOT_EXECUTED_UNTIL_ESP_006",
        overall_pass,
        not_proven: vec![
            "stage0 parses Core",
            "Core programs execute",
            "memory and host ABI are implementable",
            "self-hosting",
            "Rust independence",
            "native backend",
        ],
    };
    fs::write(
        &output,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )
    .with_context(|| format!("write {}", output.display()))?;
    println!(
        "ESP-004: docs={}; valid={}; invalid={}; constructs={}; mutation={}; overall_pass={}",
        report.validation.documents_checked,
        report.validation.valid_fixtures_checked,
        report.validation.invalid_fixtures_checked,
        report.validation.constructs_covered,
        report.mutation_control_detected,
        report.overall_pass
    );
    if !report.overall_pass {
        bail!(
            "ESP-004 specification validation failed: {:?}",
            report.validation.failures
        );
    }
    Ok(())
}
