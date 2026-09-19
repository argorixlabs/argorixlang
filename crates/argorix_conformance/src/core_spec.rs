//! Structural validator for the executable Argorix Core 0.1 specification.
//!
//! ESP-004 freezes grammar, semantics and fixtures. It does not implement the
//! Core parser; ESP-006 owns that stage0 frontend.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
pub struct CoreSpec {
    pub schema_version: u32,
    pub language: String,
    pub version: String,
    pub required_header: String,
    pub documents: Vec<String>,
    pub required_grammar_rules: Vec<String>,
    pub required_constructs: Vec<String>,
    pub forbidden_magic: Vec<String>,
    pub deferred_features: Vec<String>,
    pub required_demonstrations: Vec<String>,
    pub fixture_manifest: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CaseManifest {
    pub schema_version: u32,
    pub core_version: String,
    pub valid_cases: Vec<FixtureCase>,
    pub invalid_cases: Vec<FixtureCase>,
    pub construct_coverage: Vec<ConstructCoverage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FixtureCase {
    pub id: String,
    pub file: String,
    pub expected: String,
    #[serde(default)]
    pub demonstrates: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConstructCoverage {
    pub construct: String,
    pub positive: String,
    pub negative: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoreValidation {
    pub passed: bool,
    pub failures: Vec<String>,
    pub documents_checked: usize,
    pub valid_fixtures_checked: usize,
    pub invalid_fixtures_checked: usize,
    pub constructs_covered: usize,
    pub demonstrations_covered: usize,
    pub forbidden_magic_control_detected: bool,
    pub artifact_sha256: BTreeMap<String, String>,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn safe_relative(value: &str) -> bool {
    let path = Path::new(value);
    !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
}

fn read_artifact(
    root: &Path,
    relative: &str,
    failures: &mut Vec<String>,
    hashes: &mut BTreeMap<String, String>,
) -> Option<String> {
    if !safe_relative(relative) {
        failures.push(format!("unsafe artifact path: {relative}"));
        return None;
    }
    let path = root.join(relative);
    match fs::read(&path) {
        Ok(bytes) if !bytes.is_empty() => {
            hashes.insert(relative.replace('\\', "/"), digest(&bytes));
            match String::from_utf8(bytes) {
                Ok(text) => Some(text),
                Err(_) => {
                    failures.push(format!("artifact is not UTF-8: {relative}"));
                    None
                }
            }
        }
        Ok(_) => {
            failures.push(format!("artifact is empty: {relative}"));
            None
        }
        Err(error) => {
            failures.push(format!("cannot read {relative}: {error}"));
            None
        }
    }
}

pub fn validate_core_spec(
    spec: &CoreSpec,
    cases: &CaseManifest,
    spec_dir: &Path,
    fixture_dir: &Path,
) -> CoreValidation {
    let mut failures = Vec::new();
    let mut hashes = BTreeMap::new();
    if spec.schema_version != 1 || cases.schema_version != 1 {
        failures.push("schema_version must be 1".into());
    }
    if spec.language != "Argorix Core" || spec.version != "0.1" || cases.core_version != "0.1" {
        failures.push("language/core version mismatch".into());
    }
    if spec.required_header != "core 0.1;" {
        failures.push("required Core header changed".into());
    }
    if spec.fixture_manifest.replace('\\', "/") != "tests/selfhost/spec/cases.json" {
        failures.push("fixture manifest path changed".into());
    }
    let mut grammar = None;
    for document in &spec.documents {
        let text = read_artifact(spec_dir, document, &mut failures, &mut hashes);
        if document == "grammar.ebnf" {
            grammar = text;
        }
    }
    if let Some(grammar) = grammar {
        for rule in &spec.required_grammar_rules {
            let found = grammar.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with(rule) && line[rule.len()..].trim_start().starts_with('=')
            });
            if !found {
                failures.push(format!("missing grammar rule {rule}"));
            }
        }
    }

    let valid_ids: BTreeSet<_> = cases
        .valid_cases
        .iter()
        .map(|case| case.id.as_str())
        .collect();
    let invalid_ids: BTreeSet<_> = cases
        .invalid_cases
        .iter()
        .map(|case| case.id.as_str())
        .collect();
    if valid_ids.len() != cases.valid_cases.len() || invalid_ids.len() != cases.invalid_cases.len()
    {
        failures.push("duplicate fixture id".into());
    }

    let forbidden = spec
        .forbidden_magic
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut demonstrations = BTreeSet::new();
    for case in &cases.valid_cases {
        if case.expected != "ACCEPT" || case.category.is_some() {
            failures.push(format!("valid case {} has invalid oracle", case.id));
        }
        demonstrations.extend(case.demonstrates.iter().cloned());
        if let Some(text) = read_artifact(fixture_dir, &case.file, &mut failures, &mut hashes) {
            if !text.trim_start().starts_with(&spec.required_header) {
                failures.push(format!("valid case {} lacks Core header", case.id));
            }
            let lower = text.to_ascii_lowercase();
            for magic in &forbidden {
                if lower.contains(magic) {
                    failures.push(format!(
                        "valid case {} contains forbidden magic {magic}",
                        case.id
                    ));
                }
            }
        }
    }

    let mut forbidden_control = false;
    for case in &cases.invalid_cases {
        if case.expected != "REJECT" || case.category.as_deref().unwrap_or("").is_empty() {
            failures.push(format!("invalid case {} lacks rejection oracle", case.id));
        }
        if let Some(text) = read_artifact(fixture_dir, &case.file, &mut failures, &mut hashes) {
            let lower = text.to_ascii_lowercase();
            if case.category.as_deref() == Some("ForbiddenHostEscape")
                && forbidden.iter().any(|magic| lower.contains(magic))
            {
                forbidden_control = true;
            }
        }
    }
    if !forbidden_control {
        failures.push("forbidden-magic positive control was not detected".into());
    }

    let required: BTreeSet<_> = spec.required_constructs.iter().cloned().collect();
    let covered: BTreeSet<_> = cases
        .construct_coverage
        .iter()
        .map(|entry| entry.construct.clone())
        .collect();
    if required != covered || covered.len() != cases.construct_coverage.len() {
        failures.push("construct coverage is missing, unexpected or duplicated".into());
    }
    for entry in &cases.construct_coverage {
        match cases
            .valid_cases
            .iter()
            .find(|case| case.id == entry.positive)
        {
            Some(case) if case.demonstrates.contains(&entry.construct) => {}
            Some(_) => failures.push(format!(
                "{} positive fixture does not claim the construct",
                entry.construct
            )),
            None => failures.push(format!("{} lacks positive fixture", entry.construct)),
        }
        if !invalid_ids.contains(entry.negative.as_str()) {
            failures.push(format!("{} lacks negative fixture", entry.construct));
        }
    }
    for required_demo in &spec.required_demonstrations {
        if !valid_ids.contains(required_demo.as_str()) || !demonstrations.contains(required_demo) {
            failures.push(format!("missing demonstration {required_demo}"));
        }
    }
    for feature in &spec.deferred_features {
        if feature.trim().is_empty() {
            failures.push("empty deferred feature".into());
        }
    }

    let referenced: BTreeSet<PathBuf> = cases
        .valid_cases
        .iter()
        .chain(&cases.invalid_cases)
        .map(|case| PathBuf::from(&case.file))
        .collect();
    let mut discovered = BTreeSet::new();
    for directory in ["valid", "invalid"] {
        let path = fixture_dir.join(directory);
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|value| value.to_str()) == Some("argx") {
                    discovered.insert(PathBuf::from(directory).join(entry.file_name()));
                }
            }
        }
    }
    if referenced != discovered {
        failures.push("fixture manifest does not exactly inventory .argx files".into());
    }

    CoreValidation {
        passed: failures.is_empty(),
        failures,
        documents_checked: spec.documents.len(),
        valid_fixtures_checked: cases.valid_cases.len(),
        invalid_fixtures_checked: cases.invalid_cases.len(),
        constructs_covered: covered.len(),
        demonstrations_covered: spec
            .required_demonstrations
            .iter()
            .filter(|item| demonstrations.contains(*item))
            .count(),
        forbidden_magic_control_detected: forbidden_control,
        artifact_sha256: hashes,
    }
}
