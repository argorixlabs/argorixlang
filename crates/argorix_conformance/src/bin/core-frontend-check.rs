use anyhow::{bail, Context, Result};
use argorix_parser::core::{lex_core, parse_core_source, CoreDiagnostic};
use argorix_semantics::{check_core_program, CoreCheckOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::{env, fs, path::PathBuf};

#[derive(Deserialize)]
struct CaseManifest {
    core_version: String,
    valid_cases: Vec<Case>,
    invalid_cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    file: String,
    category: Option<String>,
}

#[derive(Serialize)]
struct CaseResult {
    id: String,
    file: String,
    expected_category: String,
    observed_categories: Vec<String>,
    token_count: usize,
    spans_valid: bool,
    passed: bool,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    task: &'static str,
    baseline_commit: String,
    scope: &'static str,
    core_version: String,
    cases_manifest_sha256: String,
    parser_sha256: String,
    semantics_sha256: String,
    valid_cases: Vec<CaseResult>,
    invalid_cases: Vec<CaseResult>,
    accepted_valid: usize,
    rejected_invalid_with_expected_category: usize,
    phases_exercised: Vec<&'static str>,
    fixture_shortcuts_found: Vec<String>,
    mutation_control_detected: bool,
    unsafe_blocks_in_frontend: usize,
    overall_pass: bool,
    not_proven: Vec<&'static str>,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn categories(errors: &[CoreDiagnostic]) -> Vec<String> {
    let mut result = errors
        .iter()
        .map(|error| error.code.clone())
        .collect::<Vec<_>>();
    result.sort();
    result.dedup();
    result
}

fn main() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let cases_path = PathBuf::from(args.next().context(
        "usage: core-frontend-check CASES FIXTURE_DIR PARSER SEMANTICS BASELINE OUTPUT",
    )?);
    let fixture_dir = PathBuf::from(args.next().context(
        "usage: core-frontend-check CASES FIXTURE_DIR PARSER SEMANTICS BASELINE OUTPUT",
    )?);
    let parser_path = PathBuf::from(args.next().context(
        "usage: core-frontend-check CASES FIXTURE_DIR PARSER SEMANTICS BASELINE OUTPUT",
    )?);
    let semantics_path = PathBuf::from(args.next().context(
        "usage: core-frontend-check CASES FIXTURE_DIR PARSER SEMANTICS BASELINE OUTPUT",
    )?);
    let baseline_commit = args
        .next()
        .context("usage: core-frontend-check CASES FIXTURE_DIR PARSER SEMANTICS BASELINE OUTPUT")?
        .to_string_lossy()
        .into_owned();
    let output = PathBuf::from(args.next().context(
        "usage: core-frontend-check CASES FIXTURE_DIR PARSER SEMANTICS BASELINE OUTPUT",
    )?);
    if args.next().is_some() {
        bail!("usage: core-frontend-check CASES FIXTURE_DIR PARSER SEMANTICS BASELINE OUTPUT");
    }

    let cases_raw = fs::read(&cases_path)?;
    let parser_raw = fs::read(&parser_path)?;
    let semantics_raw = fs::read(&semantics_path)?;
    let manifest: CaseManifest = serde_json::from_slice(&cases_raw)?;

    let mut valid_programs = Vec::new();
    for case in &manifest.valid_cases {
        let source = fs::read_to_string(fixture_dir.join(&case.file))?;
        if let Ok(program) = parse_core_source(&source) {
            valid_programs.push((case, source, program));
        }
    }
    let available_modules = valid_programs
        .iter()
        .map(|(_, _, program)| program.module.value.clone())
        .collect::<BTreeSet<_>>();
    let options = CoreCheckOptions { available_modules };

    let mut valid_results = Vec::new();
    for case in &manifest.valid_cases {
        let source = fs::read_to_string(fixture_dir.join(&case.file))?;
        let tokens = lex_core(&source);
        let token_count = tokens.as_ref().map_or(0, Vec::len);
        let spans_valid = tokens.as_ref().is_ok_and(|tokens| {
            tokens
                .iter()
                .all(|token| token.span.start <= token.span.end && token.span.end <= source.len())
        });
        let observed = match parse_core_source(&source) {
            Err(errors) => categories(&errors),
            Ok(program) => match check_core_program(&program, &options) {
                Ok(()) => Vec::new(),
                Err(errors) => categories(&errors),
            },
        };
        valid_results.push(CaseResult {
            id: case.id.clone(),
            file: case.file.clone(),
            expected_category: "ACCEPT".into(),
            passed: observed.is_empty() && spans_valid,
            observed_categories: observed,
            token_count,
            spans_valid,
        });
    }

    let mut invalid_results = Vec::new();
    for case in &manifest.invalid_cases {
        let source = fs::read_to_string(fixture_dir.join(&case.file))?;
        let tokens = lex_core(&source);
        let token_count = tokens.as_ref().map_or(0, Vec::len);
        let spans_valid = tokens.as_ref().map_or(true, |tokens| {
            tokens
                .iter()
                .all(|token| token.span.start <= token.span.end && token.span.end <= source.len())
        });
        let observed = match parse_core_source(&source) {
            Err(errors) => categories(&errors),
            Ok(program) => match check_core_program(&program, &options) {
                Ok(()) => Vec::new(),
                Err(errors) => categories(&errors),
            },
        };
        let expected = case
            .category
            .clone()
            .context("invalid case missing category")?;
        invalid_results.push(CaseResult {
            id: case.id.clone(),
            file: case.file.clone(),
            expected_category: expected.clone(),
            passed: observed.contains(&expected) && spans_valid,
            observed_categories: observed,
            token_count,
            spans_valid,
        });
    }

    let implementation = format!(
        "{}\n{}",
        String::from_utf8_lossy(&parser_raw),
        String::from_utf8_lossy(&semantics_raw)
    );
    let mut fixture_shortcuts_found = Vec::new();
    for case in manifest.valid_cases.iter().chain(&manifest.invalid_cases) {
        if implementation.contains(&case.file) {
            fixture_shortcuts_found.push(case.file.clone());
        }
        let source = fs::read_to_string(fixture_dir.join(&case.file))?;
        if let Ok(program) = parse_core_source(&source) {
            if implementation.contains(&program.module.value) {
                fixture_shortcuts_found.push(program.module.value);
            }
        }
    }
    fixture_shortcuts_found.sort();
    fixture_shortcuts_found.dedup();
    let mutation_control_detected = {
        let source = fs::read_to_string(fixture_dir.join(&manifest.valid_cases[0].file))?;
        let mutated = source.replacen("core 0.1;", "core 9.9;", 1);
        parse_core_source(&mutated)
            .expect_err("version mutation was accepted")
            .iter()
            .any(|error| error.code == "VersionUnsupported")
    };
    let accepted_valid = valid_results.iter().filter(|case| case.passed).count();
    let rejected_invalid = invalid_results.iter().filter(|case| case.passed).count();
    let unsafe_blocks = implementation.matches("unsafe {").count();
    let overall_pass = accepted_valid == manifest.valid_cases.len()
        && rejected_invalid == manifest.invalid_cases.len()
        && fixture_shortcuts_found.is_empty()
        && mutation_control_detected
        && unsafe_blocks == 0;
    let report = Report {
        schema_version: 1,
        task: "ESP-006",
        baseline_commit,
        scope: "Rust stage0 Core 0.1 lexer/parser/resolution/semantics; no lowering, execution, self-hosting, or Rust independence",
        core_version: manifest.core_version,
        cases_manifest_sha256: digest(&cases_raw),
        parser_sha256: digest(&parser_raw),
        semantics_sha256: digest(&semantics_raw),
        valid_cases: valid_results,
        invalid_cases: invalid_results,
        accepted_valid,
        rejected_invalid_with_expected_category: rejected_invalid,
        phases_exercised: vec!["lexical", "syntax", "resolution", "semantic"],
        fixture_shortcuts_found,
        mutation_control_detected,
        unsafe_blocks_in_frontend: unsafe_blocks,
        overall_pass,
        not_proven: vec![
            "Core lowering or execution",
            "self-hosted frontend",
            "stage0/stage1 equivalence",
            "native backend",
            "Rust independence",
            "production security or performance",
        ],
    };
    fs::write(
        &output,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    println!(
        "ESP-006: valid={}/{} invalid={}/{} mutation={} shortcuts={} overall_pass={}",
        report.accepted_valid,
        report.valid_cases.len(),
        report.rejected_invalid_with_expected_category,
        report.invalid_cases.len(),
        report.mutation_control_detected,
        report.fixture_shortcuts_found.len(),
        report.overall_pass
    );
    if !report.overall_pass {
        bail!("ESP-006 frontend validation failed");
    }
    Ok(())
}
