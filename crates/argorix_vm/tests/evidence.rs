use argorix_bytecode::BytecodeProgram;
use argorix_vm::{
    evidence::{canonical_digest, verify_evidence, EvidenceBundle},
    InjectedMessage, SecurityReport, Vm,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn fixture() -> BytecodeProgram {
    serde_json::from_str(include_str!(
        "../../../examples/provider_allowlists_v013.argbc.json"
    ))
    .unwrap()
}

fn injection() -> InjectedMessage {
    InjectedMessage {
        from: "User".into(),
        to: "ResearchAgent".into(),
        act: "tell".into(),
        message_type: "UserPrompt".into(),
    }
}

fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "argorix-v014-evidence-{}-{name}",
        std::process::id()
    ))
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
    )
    .unwrap();
}

#[test]
fn semantic_digests_are_deterministic_and_ignore_json_whitespace() {
    let bytecode = fixture();
    let compact = serde_json::to_string(&bytecode).unwrap();
    let pretty = serde_json::to_string_pretty(&bytecode).unwrap();
    let compact_value: BytecodeProgram = serde_json::from_str(&compact).unwrap();
    let pretty_value: BytecodeProgram = serde_json::from_str(&pretty).unwrap();

    let first = canonical_digest(&compact_value).unwrap();
    let second = canonical_digest(&pretty_value).unwrap();

    assert_eq!(first, second);
    assert!(first.starts_with("sha256:"));
    assert_eq!(first.len(), 71);
}

#[test]
fn semantic_digest_changes_when_bytecode_changes() {
    let bytecode = fixture();
    let mut changed = bytecode.clone();
    changed.module.push_str("Changed");

    assert_ne!(
        canonical_digest(&bytecode).unwrap(),
        canonical_digest(&changed).unwrap()
    );
}

#[test]
fn bundle_preserves_metadata_digests_and_relative_paths() {
    let root = temp_root("bundle");
    let bundle_path = root.join("reports/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let trace_path = root.join("reports/run.trace.json");
    let report_path = root.join("reports/run.security.json");
    let bytecode = fixture();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    let bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        Some(&trace_path),
        Some(&report_path),
    )
    .unwrap();

    assert!(matches!(bundle.bundle_version.as_str(), "1.0"));
    assert_eq!(bundle.language, bytecode.language);
    assert_eq!(bundle.module, bytecode.module);
    assert_eq!(bundle.bytecode_version, bytecode.bytecode_version);
    assert_eq!(bundle.report_version, report.report_version);
    assert_eq!(bundle.ledger_digest, report.ledger.ledger_digest);
    assert_eq!(
        bundle.artifacts.bytecode_path.as_deref(),
        Some("../examples/program.argbc.json")
    );
    assert_eq!(
        bundle.artifacts.trace_path.as_deref(),
        Some("run.trace.json")
    );
    assert_eq!(
        bundle.artifacts.security_report_path.as_deref(),
        Some("run.security.json")
    );
    assert!(bundle.trace_digest.is_some());
}

#[test]
fn failed_outcome_without_trace_uses_null_trace_evidence() {
    let root = temp_root("failed");
    let bundle_path = root.join("reports/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let report_path = root.join("reports/run.security.json");
    let mut bytecode = fixture();
    bytecode.instructions.pop();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    let bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        None,
        Some(&report_path),
    )
    .unwrap();

    assert!(outcome.result.is_err());
    assert_eq!(bundle.trace_digest, None);
    assert_eq!(bundle.artifacts.trace_path, None);
}

#[test]
fn absolute_artifact_outside_portable_tree_is_rejected() {
    let root = temp_root("external");
    let bundle_path = root.join("reports/run.bundle.json");
    let external = if cfg!(windows) {
        PathBuf::from(r"Z:\different-portable-tree\program.argbc.json")
    } else {
        PathBuf::from("/different-portable-tree/program.argbc.json")
    };
    let bytecode = fixture();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    let error = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&external),
        None,
        None,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("outside the bundle portable tree"));
}

#[test]
fn nested_bundle_can_reference_project_sibling_artifacts() {
    let root = temp_root("nested");
    let bundle_path = root.join("reports/nested/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let bytecode = fixture();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    let bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        None,
        None,
    )
    .unwrap();

    assert_eq!(
        bundle.artifacts.bytecode_path.as_deref(),
        Some("../../examples/program.argbc.json")
    );
}

#[test]
fn offline_verification_passes_intact_semantic_artifacts() {
    let root = temp_root("verify-pass");
    let bundle_path = root.join("reports/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let trace_path = root.join("reports/run.trace.json");
    let report_path = root.join("reports/run.security.json");
    let bytecode = fixture();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let trace = outcome.result.as_ref().unwrap();
    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    let bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        Some(&trace_path),
        Some(&report_path),
    )
    .unwrap();
    write_json(&bytecode_path, &bytecode);
    write_json(&trace_path, trace);
    write_json(&report_path, &report);
    write_json(&bundle_path, &bundle);

    let result = verify_evidence(&bundle_path).unwrap();

    assert!(result.passed, "{:?}", result.failures);
    assert_eq!(result.checks_failed, 0);
    assert_eq!(result.checks_total, result.checks_passed);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn offline_verification_reports_tampering_missing_artifacts_and_bad_ledger() {
    let root = temp_root("verify-fail");
    let bundle_path = root.join("reports/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let report_path = root.join("reports/run.security.json");
    let bytecode = fixture();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    let mut bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        None,
        Some(&report_path),
    )
    .unwrap();
    let mut changed = bytecode.clone();
    changed.module.push_str("Tampered");
    let mut report_json = serde_json::to_value(&report).unwrap();
    report_json["ledger"]["ledger_digest"] = Value::String("sha256:deadbeef".into());
    write_json(&bytecode_path, &changed);
    write_json(&report_path, &report_json);
    bundle.artifacts.trace_path = Some("missing.trace.json".into());
    bundle.trace_digest = None;
    write_json(&bundle_path, &bundle);

    let result = verify_evidence(&bundle_path).unwrap();

    assert!(!result.passed);
    assert!(result
        .failures
        .iter()
        .any(|failure| failure.contains("bytecode_digest mismatch")));
    assert!(result
        .failures
        .iter()
        .any(|failure| failure.contains("trace_path and trace_digest")));
    assert!(result
        .failures
        .iter()
        .any(|failure| failure.contains("ledger_digest")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn offline_verification_accepts_v014_bundle_and_report() {
    let root = temp_root("compat-v014");
    let bundle_path = root.join("reports/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let trace_path = root.join("reports/run.trace.json");
    let report_path = root.join("reports/run.security.json");
    let bytecode = fixture();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let trace = outcome.result.as_ref().unwrap();
    let mut report = SecurityReport::from_outcome(&bytecode, &outcome);
    report.report_version = "0.14".into();
    let mut bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        Some(&trace_path),
        Some(&report_path),
    )
    .unwrap();
    bundle.bundle_version = "0.14".into();
    write_json(&bytecode_path, &bytecode);
    write_json(&trace_path, trace);
    write_json(&report_path, &report);
    write_json(&bundle_path, &bundle);

    let result = verify_evidence(&bundle_path).unwrap();

    assert!(result.passed, "{:?}", result.failures);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn offline_verification_accepts_v034_bundle_and_report() {
    let root = temp_root("compat-v034");
    let bundle_path = root.join("reports/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let trace_path = root.join("reports/run.trace.json");
    let report_path = root.join("reports/run.security.json");
    let bytecode = fixture();
    let mut outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    outcome.result.as_mut().unwrap().vm_version = "0.34".into();
    let trace = outcome.result.as_ref().unwrap();
    let mut report = SecurityReport::from_outcome(&bytecode, &outcome);
    report.report_version = "0.34".into();
    report.vm_version = "0.34".into();
    let mut bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        Some(&trace_path),
        Some(&report_path),
    )
    .unwrap();
    bundle.bundle_version = "0.34".into();
    write_json(&bytecode_path, &bytecode);
    write_json(&trace_path, trace);
    write_json(&report_path, &report);
    write_json(&bundle_path, &bundle);
    let result = verify_evidence(&bundle_path).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn offline_verification_accepts_v035_bundle_and_report() {
    let root = temp_root("compat-v035");
    let bundle_path = root.join("reports/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let trace_path = root.join("reports/run.trace.json");
    let report_path = root.join("reports/run.security.json");
    let bytecode = fixture();
    let mut outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    outcome.result.as_mut().unwrap().vm_version = "0.35".into();
    let trace = outcome.result.as_ref().unwrap();
    let mut report = SecurityReport::from_outcome(&bytecode, &outcome);
    report.report_version = "0.35".into();
    report.vm_version = "0.35".into();
    let mut bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        Some(&trace_path),
        Some(&report_path),
    )
    .unwrap();
    bundle.bundle_version = "0.35".into();
    write_json(&bytecode_path, &bytecode);
    write_json(&trace_path, trace);
    write_json(&report_path, &report);
    write_json(&bundle_path, &bundle);
    let result = verify_evidence(&bundle_path).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn offline_verification_accepts_v036_bundle_and_report() {
    let root = temp_root("compat-v036");
    let bundle_path = root.join("reports/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let trace_path = root.join("reports/run.trace.json");
    let report_path = root.join("reports/run.security.json");
    let bytecode = fixture();
    let mut outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    outcome.result.as_mut().unwrap().vm_version = "0.36".into();
    let trace = outcome.result.as_ref().unwrap();
    let mut report = SecurityReport::from_outcome(&bytecode, &outcome);
    report.report_version = "0.36".into();
    report.vm_version = "0.36".into();
    let mut bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        Some(&trace_path),
        Some(&report_path),
    )
    .unwrap();
    bundle.bundle_version = "0.36".into();
    write_json(&bytecode_path, &bytecode);
    write_json(&trace_path, trace);
    write_json(&report_path, &report);
    write_json(&bundle_path, &bundle);
    let result = verify_evidence(&bundle_path).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn offline_verification_accepts_v019_bundle_without_harness_summary_field() {
    let root = temp_root("compat-v019");
    let bundle_path = root.join("reports/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let trace_path = root.join("reports/run.trace.json");
    let report_path = root.join("reports/run.security.json");
    let bytecode = fixture();
    let mut outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    outcome.result.as_mut().unwrap().vm_version = "0.19".into();
    let trace = outcome.result.as_ref().unwrap();
    let mut report = SecurityReport::from_outcome(&bytecode, &outcome);
    report.report_version = "0.19".into();
    report.vm_version = "0.19".into();
    let mut bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        Some(&trace_path),
        Some(&report_path),
    )
    .unwrap();
    bundle.bundle_version = "0.19".into();

    let mut old_report_json = serde_json::to_value(&report).unwrap();
    old_report_json
        .as_object_mut()
        .unwrap()
        .remove("provider_harnesses");
    write_json(&bytecode_path, &bytecode);
    write_json(&trace_path, trace);
    write_json(&report_path, &old_report_json);
    write_json(&bundle_path, &bundle);

    let result = verify_evidence(&bundle_path).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn offline_verification_rejects_unsupported_bundle_version() {
    let root = temp_root("unsupported-bundle");
    let bundle_path = root.join("reports/run.bundle.json");
    let bytecode_path = root.join("examples/program.argbc.json");
    let trace_path = root.join("reports/run.trace.json");
    let report_path = root.join("reports/run.security.json");
    let bytecode = fixture();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let trace = outcome.result.as_ref().unwrap();
    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    let mut bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        &bundle_path,
        Some(&bytecode_path),
        Some(&trace_path),
        Some(&report_path),
    )
    .unwrap();
    bundle.bundle_version = "0.13".into();
    write_json(&bytecode_path, &bytecode);
    write_json(&trace_path, trace);
    write_json(&report_path, &report);
    write_json(&bundle_path, &bundle);

    let result = verify_evidence(&bundle_path).unwrap();

    assert!(!result.passed);
    assert!(result
        .failures
        .iter()
        .any(|failure| failure.contains("unsupported bundle_version")));
    fs::remove_dir_all(root).unwrap();
}
