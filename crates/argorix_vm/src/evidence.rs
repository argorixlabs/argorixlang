use crate::{ExecutionOutcome, ReactiveExecutionTrace, SecurityReport};
use argorix_bytecode::BytecodeProgram;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub bundle_version: String,
    pub language: String,
    pub module: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<argorix_bytecode::BytecodeModule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<argorix_bytecode::BytecodeModuleImport>,
    pub bytecode_version: String,
    pub vm_version: String,
    pub report_version: String,
    pub bytecode_digest: String,
    pub trace_digest: Option<String>,
    pub report_digest: String,
    pub ledger_digest: String,
    pub artifacts: EvidenceArtifacts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceArtifacts {
    pub bytecode_path: Option<String>,
    pub trace_path: Option<String>,
    pub security_report_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceVerificationResult {
    pub passed: bool,
    pub checks_total: usize,
    pub checks_passed: usize,
    pub checks_failed: usize,
    pub failures: Vec<String>,
}

#[derive(Debug, Error)]
pub enum EvidenceError {
    #[error("failed to serialize semantic evidence: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to access evidence artifact `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("evidence artifact `{0}` is outside the bundle portable tree")]
    OutsidePortableTree(PathBuf),
    #[error("evidence bundle path `{0}` has no parent directory")]
    MissingBundleParent(PathBuf),
}

impl EvidenceBundle {
    #[allow(clippy::too_many_arguments)]
    pub fn from_outcome(
        bytecode: &BytecodeProgram,
        outcome: &ExecutionOutcome,
        report: &SecurityReport,
        bundle_path: &Path,
        bytecode_path: Option<&Path>,
        trace_path: Option<&Path>,
        security_report_path: Option<&Path>,
    ) -> Result<Self, EvidenceError> {
        let trace = outcome.result.as_ref().ok();
        Ok(Self {
            bundle_version: "0.35".into(),
            language: bytecode.language.clone(),
            module: bytecode.module.clone(),
            modules: bytecode.modules.clone(),
            imports: bytecode.imports.clone(),
            bytecode_version: bytecode.bytecode_version.clone(),
            vm_version: report.vm_version.clone(),
            report_version: report.report_version.clone(),
            bytecode_digest: canonical_digest(bytecode)?,
            trace_digest: trace.map(canonical_digest).transpose()?,
            report_digest: canonical_digest(report)?,
            ledger_digest: report.ledger.ledger_digest.clone(),
            artifacts: EvidenceArtifacts {
                bytecode_path: portable_artifact_path(bundle_path, bytecode_path)?,
                trace_path: if trace.is_some() {
                    portable_artifact_path(bundle_path, trace_path)?
                } else {
                    None
                },
                security_report_path: portable_artifact_path(bundle_path, security_report_path)?,
            },
        })
    }
}

pub fn canonical_digest<T: Serialize + ?Sized>(value: &T) -> Result<String, EvidenceError> {
    let bytes = serde_json::to_vec(value)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

pub fn verify_evidence(bundle_path: &Path) -> Result<EvidenceVerificationResult, EvidenceError> {
    let source = read(bundle_path)?;
    let bundle: EvidenceBundle = serde_json::from_slice(&source)?;
    let cwd = std::env::current_dir().map_err(|source| EvidenceError::Io {
        path: PathBuf::from("."),
        source,
    })?;
    let absolute_bundle_path = absolute_lexical(&cwd, bundle_path);
    let base = absolute_bundle_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| EvidenceError::MissingBundleParent(absolute_bundle_path.clone()))?;
    let mut checks = Checks::default();

    checks.record(
        matches!(
            bundle.bundle_version.as_str(),
            "0.14"
                | "0.15"
                | "0.16"
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
                | "0.35"
        ),
        "unsupported bundle_version",
    );
    checks.record(
        valid_digest(&bundle.bytecode_digest),
        "bytecode_digest has invalid sha256 format",
    );
    checks.record(
        bundle.trace_digest.as_deref().is_none_or(valid_digest),
        "trace_digest has invalid sha256 format",
    );
    checks.record(
        valid_digest(&bundle.report_digest),
        "report_digest has invalid sha256 format",
    );
    checks.record(
        valid_digest(&bundle.ledger_digest),
        "ledger_digest has invalid sha256 format",
    );
    checks.record(
        bundle.artifacts.trace_path.is_none() || bundle.trace_digest.is_some(),
        "trace_path and trace_digest are inconsistent",
    );

    if let Some(path) = &bundle.artifacts.bytecode_path {
        if let Some(bytecode) =
            load_artifact::<BytecodeProgram>(base, path, "bytecode", &mut checks)
        {
            checks.record(
                canonical_digest(&bytecode)? == bundle.bytecode_digest,
                "bytecode_digest mismatch",
            );
            checks.record(
                bytecode.language == bundle.language,
                "bytecode language mismatch",
            );
            checks.record(bytecode.module == bundle.module, "bytecode module mismatch");
            checks.record(
                bytecode.bytecode_version == bundle.bytecode_version,
                "bytecode version mismatch",
            );
        }
    }

    if let Some(path) = &bundle.artifacts.trace_path {
        if let Some(trace) =
            load_artifact::<ReactiveExecutionTrace>(base, path, "trace", &mut checks)
        {
            if let Some(expected) = &bundle.trace_digest {
                checks.record(
                    canonical_digest(&trace)? == *expected,
                    "trace_digest mismatch",
                );
            }
            checks.record(
                canonical_digest(&trace.events)? == bundle.ledger_digest,
                "trace ledger_digest mismatch",
            );
            checks.record(
                trace.vm_version == bundle.vm_version,
                "trace vm_version mismatch",
            );
        }
    }

    if let Some(path) = &bundle.artifacts.security_report_path {
        if let Some(report) =
            load_artifact::<SecurityReport>(base, path, "security report", &mut checks)
        {
            checks.record(
                canonical_digest(&report)? == bundle.report_digest,
                "report_digest mismatch",
            );
            checks.record(
                report.ledger.ledger_digest == bundle.ledger_digest,
                "ledger_digest mismatch",
            );
            checks.record(
                report.report_version == bundle.report_version,
                "report_version mismatch",
            );
            checks.record(
                report.bytecode_version == bundle.bytecode_version,
                "report bytecode_version mismatch",
            );
            checks.record(
                report.vm_version == bundle.vm_version,
                "report vm_version mismatch",
            );
        }
    }

    Ok(checks.finish())
}

fn portable_artifact_path(
    bundle_path: &Path,
    artifact_path: Option<&Path>,
) -> Result<Option<String>, EvidenceError> {
    let Some(artifact_path) = artifact_path else {
        return Ok(None);
    };
    let cwd = std::env::current_dir().map_err(|source| EvidenceError::Io {
        path: PathBuf::from("."),
        source,
    })?;
    let bundle = absolute_lexical(&cwd, bundle_path);
    let artifact = absolute_lexical(&cwd, artifact_path);
    let bundle_dir = bundle
        .parent()
        .ok_or_else(|| EvidenceError::MissingBundleParent(bundle.clone()))?;
    if !shares_non_root_ancestor(bundle_dir, &artifact) {
        return Err(EvidenceError::OutsidePortableTree(artifact));
    }
    let relative = relative_path(bundle_dir, &artifact)
        .ok_or_else(|| EvidenceError::OutsidePortableTree(artifact.clone()))?;
    Ok(Some(normalized_slashes(&relative)))
}

fn absolute_lexical(cwd: &Path, path: &Path) -> PathBuf {
    normalize_lexical(if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    })
}

fn normalize_lexical(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
    let from_components = from.components().collect::<Vec<_>>();
    let to_components = to.components().collect::<Vec<_>>();
    let common = from_components
        .iter()
        .zip(&to_components)
        .take_while(|(left, right)| left == right)
        .count();
    if common == 0 {
        return None;
    }
    let mut relative = PathBuf::new();
    for _ in common..from_components.len() {
        relative.push("..");
    }
    for component in &to_components[common..] {
        relative.push(component.as_os_str());
    }
    Some(relative)
}

fn normalized_slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn shares_non_root_ancestor(left: &Path, right: &Path) -> bool {
    let left_components = left.components().collect::<Vec<_>>();
    let right_components = right.components().collect::<Vec<_>>();
    let common = left_components
        .iter()
        .zip(&right_components)
        .take_while(|(left, right)| left == right)
        .count();
    let first_normal = left_components
        .iter()
        .position(|component| matches!(component, Component::Normal(_)))
        .unwrap_or(left_components.len());
    common > first_normal
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn read(path: &Path) -> Result<Vec<u8>, EvidenceError> {
    fs::read(path).map_err(|source| EvidenceError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn resolve_artifact(base: &Path, stored: &str) -> Option<PathBuf> {
    let path = Path::new(stored);
    if path.is_absolute() {
        return None;
    }
    let resolved = normalize_lexical(base.join(path));
    shares_non_root_ancestor(base, &resolved).then_some(resolved)
}

fn load_artifact<T: for<'de> Deserialize<'de>>(
    base: &Path,
    stored: &str,
    label: &str,
    checks: &mut Checks,
) -> Option<T> {
    let Some(path) = resolve_artifact(base, stored) else {
        checks.record(
            false,
            format!("{label} path must be relative and inside the portable tree"),
        );
        return None;
    };
    let bytes = match fs::read(&path) {
        Ok(bytes) => {
            checks.record(true, format!("{label} artifact is missing"));
            bytes
        }
        Err(error) => {
            checks.record(
                false,
                format!("{label} artifact `{}` is missing: {error}", path.display()),
            );
            return None;
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(value) => {
            checks.record(true, format!("{label} artifact is invalid JSON"));
            Some(value)
        }
        Err(error) => {
            checks.record(false, format!("{label} artifact is invalid JSON: {error}"));
            None
        }
    }
}

#[derive(Default)]
struct Checks {
    total: usize,
    passed: usize,
    failures: Vec<String>,
}

impl Checks {
    fn record(&mut self, passed: bool, failure: impl Into<String>) {
        self.total += 1;
        if passed {
            self.passed += 1;
        } else {
            self.failures.push(failure.into());
        }
    }

    fn finish(self) -> EvidenceVerificationResult {
        let failed = self.failures.len();
        EvidenceVerificationResult {
            passed: failed == 0,
            checks_total: self.total,
            checks_passed: self.passed,
            checks_failed: failed,
            failures: self.failures,
        }
    }
}
