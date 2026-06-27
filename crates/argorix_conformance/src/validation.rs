use crate::types::{ConformanceCase, ConformanceSuite};
use argorix_vm::parse_injection;
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

pub const STAGES: [&str; 14] = [
    "parse",
    "semantic_check",
    "emit_ir",
    "emit_bytecode",
    "verify_bytecode",
    "run_vm",
    "security_report",
    "trace_out",
    "evidence_bundle",
    "verify_evidence",
    "resolve_package",
    "check_package",
    "emit_ir_package",
    "graph_package",
];

pub const CATEGORIES: [&str; 36] = [
    "parser",
    "semantics",
    "ir",
    "bytecode",
    "vm",
    "policy",
    "policy_v2",
    "provider_boundary",
    "adapter_contracts",
    "allowlists",
    "security_report",
    "evidence_bundle",
    "offline_verification",
    "compatibility",
    "modules",
    "package",
    "module_graph",
    "multi_file_semantics",
    "typed_messages",
    "agent_passport",
    "provider_harness",
    "feature_flags",
    "secret_boundary",
    "adapter_framework",
    "crypto_registry",
    "crypto_boundaries",
    "did_methods",
    "atrust_boundaries",
    "atrust_identities",
    "atrust_credential_contracts",
    "atrust_handshakes",
    "trust_ledgers",
    "bridge_contracts",
    "atrust_evidence_maps",
    "governance_mappings",
    "public_conformance",
];

#[derive(Debug, Error)]
#[error("invalid conformance suite:\n{details}")]
pub struct ConformanceValidationError {
    details: String,
}

pub fn validate_suite(
    suite: &ConformanceSuite,
    suite_path: &Path,
) -> Result<(), ConformanceValidationError> {
    let mut errors = Vec::new();
    if !matches!(
        suite.suite_version.as_str(),
        "0.16"
            | "0.17"
            | "0.18"
            | "0.19"
            | "0.20"
            | "0.21"
            | "0.22"
            | "0.23"
            | "0.24"
            | "0.25"
            | "0.26"
            | "0.27"
            | "0.28"
            | "0.29"
            | "0.30"
            | "0.31"
            | "0.32"
            | "0.33"
            | "0.34"
    ) {
        errors.push(format!(
            "suite_version must be `0.16`..`0.34`, found `{}`",
            suite.suite_version
        ));
    }

    let mut ids = HashSet::new();
    let mut categories = HashSet::new();
    for case in &suite.cases {
        if !ids.insert(case.id.as_str()) {
            errors.push(format!("duplicate case id `{}`", case.id));
        }
        categories.insert(case.category.as_str());
        validate_case(case, suite_path, &mut errors);
    }
    for category in CATEGORIES {
        if suite.suite_version == "0.16" && category == "policy_v2" {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.18"
                | "0.19"
                | "0.20"
                | "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
        ) && category == "typed_messages"
        {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.19"
                | "0.20"
                | "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
        ) && category == "agent_passport"
        {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.20"
                | "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
        ) && category == "provider_harness"
        {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
        ) && matches!(category, "feature_flags" | "secret_boundary")
        {
            continue;
        }
        // crypto_registry was introduced in v0.24; crypto_boundaries in v0.25.
        if !matches!(
            suite.suite_version.as_str(),
            "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
        ) && category == "crypto_registry"
        {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.25" | "0.26" | "0.27" | "0.28" | "0.29" | "0.30" | "0.31" | "0.32" | "0.33" | "0.34"
        ) && category == "crypto_boundaries"
        {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.26" | "0.27" | "0.28" | "0.29" | "0.30" | "0.31" | "0.32" | "0.33" | "0.34"
        ) && matches!(category, "did_methods" | "atrust_boundaries")
        {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.27" | "0.28" | "0.29" | "0.30" | "0.31" | "0.32" | "0.33" | "0.34"
        ) && category == "atrust_identities"
        {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.28" | "0.29" | "0.30" | "0.31" | "0.32" | "0.33" | "0.34"
        ) && category == "atrust_credential_contracts"
        {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.29" | "0.30" | "0.31" | "0.32" | "0.33" | "0.34"
        ) && category == "atrust_handshakes"
        {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.30" | "0.31" | "0.32" | "0.33" | "0.34"
        ) && category == "trust_ledgers"
        {
            continue;
        }
        if !matches!(
            suite.suite_version.as_str(),
            "0.31" | "0.32" | "0.33" | "0.34"
        ) && category == "bridge_contracts"
        {
            continue;
        }
        if !matches!(suite.suite_version.as_str(), "0.32" | "0.33" | "0.34")
            && category == "atrust_evidence_maps"
        {
            continue;
        }
        if !matches!(suite.suite_version.as_str(), "0.33" | "0.34")
            && category == "governance_mappings"
        {
            continue;
        }
        if suite.suite_version != "0.34" && category == "public_conformance" {
            continue;
        }
        if !categories.contains(category) && category != "adapter_framework" {
            errors.push(format!("missing required category `{category}`"));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ConformanceValidationError {
            details: errors.join("\n"),
        })
    }
}

pub fn resolve_fixture_path(
    suite_path: &Path,
    stored_path: &str,
) -> Result<PathBuf, ConformanceValidationError> {
    let suite_dir = suite_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let stored = Path::new(stored_path);
    if stored.is_absolute() {
        return Err(single_error(format!(
            "fixture path `{stored_path}` must be relative"
        )));
    }
    let root = lexical_absolute(suite_dir);
    let resolved = normalize_lexical(root.join(stored));
    if !resolved.starts_with(&root) {
        return Err(single_error(format!(
            "fixture path `{stored_path}` escapes the portable suite tree"
        )));
    }
    Ok(resolved)
}

fn validate_case(case: &ConformanceCase, suite_path: &Path, errors: &mut Vec<String>) {
    if case.id.trim().is_empty()
        || matches!(case.id.as_str(), "." | "..")
        || case.id.contains(['/', '\\'])
    {
        errors.push(format!(
            "case id `{}` must be a portable path segment",
            case.id
        ));
    }
    if case.name.trim().is_empty() {
        errors.push(format!("case `{}` has an empty name", case.id));
    }
    if case.category.trim().is_empty() {
        errors.push(format!("case `{}` has an empty category", case.id));
    }
    if case.stages.is_empty() {
        errors.push(format!("case `{}` must list at least one stage", case.id));
    }

    let mut seen = HashSet::new();
    for stage in &case.stages {
        if !STAGES.contains(&stage.as_str()) {
            errors.push(format!("case `{}` has unknown stage `{stage}`", case.id));
        }
        if !seen.insert(stage.as_str()) {
            errors.push(format!("case `{}` has duplicate stage `{stage}`", case.id));
        }
    }
    for (stage, dependency) in [
        ("semantic_check", "parse"),
        ("emit_ir", "semantic_check"),
        ("run_vm", "verify_bytecode"),
        ("security_report", "run_vm"),
        ("trace_out", "run_vm"),
        ("evidence_bundle", "security_report"),
        ("evidence_bundle", "trace_out"),
        ("verify_evidence", "evidence_bundle"),
        ("check_package", "resolve_package"),
        ("emit_ir_package", "check_package"),
        ("graph_package", "resolve_package"),
    ] {
        validate_dependency(case, stage, dependency, errors);
    }
    // `emit_bytecode` may be produced from a single-file IR or a package IR.
    validate_dependency_any(
        case,
        "emit_bytecode",
        &["emit_ir", "emit_ir_package"],
        errors,
    );
    if case.stages.iter().any(|stage| stage == "verify_bytecode")
        && !case.stages.iter().any(|stage| stage == "emit_bytecode")
        && case.bytecode_path.is_none()
    {
        errors.push(format!(
            "case `{}` verify_bytecode requires emit_bytecode or bytecode_path",
            case.id
        ));
    }
    if case
        .stages
        .iter()
        .any(|stage| matches!(stage.as_str(), "parse" | "semantic_check" | "emit_ir"))
        && case.source_path.is_none()
    {
        errors.push(format!(
            "case `{}` source stages require source_path",
            case.id
        ));
    }
    if case.stages.iter().any(|stage| {
        matches!(
            stage.as_str(),
            "resolve_package" | "check_package" | "emit_ir_package" | "graph_package"
        )
    }) && case.manifest_path.is_none()
    {
        errors.push(format!(
            "case `{}` package stages require manifest_path",
            case.id
        ));
    }

    if case.stages.iter().any(|stage| {
        matches!(
            stage.as_str(),
            "run_vm" | "security_report" | "trace_out" | "evidence_bundle" | "verify_evidence"
        )
    }) {
        match case.injection.as_deref() {
            Some(value) => {
                if let Err(error) = parse_injection(value) {
                    errors.push(format!("case `{}` has invalid injection: {error}", case.id));
                }
            }
            None => errors.push(format!("case `{}` requires injection", case.id)),
        }
    }

    match (
        case.expected_failure_stage.as_deref(),
        case.expected_failure_contains.as_deref(),
    ) {
        (Some(stage), _) if !case.stages.iter().any(|item| item == stage) => errors.push(format!(
            "case `{}` expected failure stage `{stage}` is not listed",
            case.id
        )),
        (None, Some(_)) => errors.push(format!(
            "case `{}` expected_failure_contains requires expected_failure_stage",
            case.id
        )),
        _ => {}
    }

    if let Some(mutation) = &case.mutation {
        if !case
            .stages
            .iter()
            .any(|stage| stage == &mutation.before_stage)
        {
            errors.push(format!(
                "case `{}` mutation stage `{}` is not listed",
                case.id, mutation.before_stage
            ));
        }
        if !matches!(
            mutation.artifact.as_str(),
            "bytecode" | "security_report" | "bundle"
        ) {
            errors.push(format!(
                "case `{}` has unsupported mutation artifact `{}`",
                case.id, mutation.artifact
            ));
        }
        if !mutation.json_pointer.starts_with('/') {
            errors.push(format!(
                "case `{}` mutation JSON Pointer must start with `/`",
                case.id
            ));
        }
    }

    for (label, path) in [
        ("source_path", case.source_path.as_deref()),
        ("bytecode_path", case.bytecode_path.as_deref()),
        ("manifest_path", case.manifest_path.as_deref()),
    ] {
        if let Some(path) = path {
            match resolve_fixture_path(suite_path, path) {
                Ok(resolved) if !resolved.exists() => errors.push(format!(
                    "case `{}` {label} `{path}` does not exist",
                    case.id
                )),
                Err(error) => errors.push(format!("case `{}` {error}", case.id)),
                _ => {}
            }
        }
    }
}

fn validate_dependency(
    case: &ConformanceCase,
    stage: &str,
    dependency: &str,
    errors: &mut Vec<String>,
) {
    let Some(stage_index) = case.stages.iter().position(|item| item == stage) else {
        return;
    };
    let dependency_index = case.stages.iter().position(|item| item == dependency);
    if dependency_index.is_none_or(|index| index >= stage_index) {
        errors.push(format!(
            "case `{}` {stage} requires earlier stage `{dependency}`",
            case.id
        ));
    }
}

/// Validate that `stage` is preceded by at least one of the alternative producers.
fn validate_dependency_any(
    case: &ConformanceCase,
    stage: &str,
    dependencies: &[&str],
    errors: &mut Vec<String>,
) {
    let Some(stage_index) = case.stages.iter().position(|item| item == stage) else {
        return;
    };
    let satisfied = dependencies.iter().any(|dependency| {
        case.stages
            .iter()
            .position(|item| item == dependency)
            .is_some_and(|index| index < stage_index)
    });
    if !satisfied {
        errors.push(format!(
            "case `{}` {stage} requires an earlier stage among [{}]",
            case.id,
            dependencies.join(", ")
        ));
    }
}

fn lexical_absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_lexical(path.to_path_buf())
    } else {
        normalize_lexical(
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path),
        )
    }
}

fn normalize_lexical(path: PathBuf) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

fn single_error(message: String) -> ConformanceValidationError {
    ConformanceValidationError { details: message }
}
