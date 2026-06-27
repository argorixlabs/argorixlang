use argorix_conformance::{run_suite, types::ConformanceSuite};
use std::{fs, path::PathBuf};

fn temp_workdir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "argorix-official-conformance-{}",
        std::process::id()
    ))
}

#[test]
fn official_v016_suite_passes_and_records_blocked_external_execution() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v016.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    // Use a dedicated subdir: tests in this binary run concurrently and the
    // bare temp_workdir() base is the parent of every other test's workdir.
    let workdir = temp_workdir().join("v016");

    let result = run_suite(&suite, &suite_path, &workdir).unwrap();

    assert!(result.passed, "{:?}", result.failures);
    assert_eq!(result.cases_total, 44);
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(workdir.join("external-provider-blocked-reportable/run.security.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        report["provider_boundary"]["external_execution_blocked"],
        true
    );
    assert_eq!(report["execution"]["failed"], true);
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v017_suite_passes_with_policy_v2_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v017.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v017");

    let result = run_suite(&suite, &suite_path, &workdir).unwrap();

    assert!(result.passed, "{:?}", result.failures);
    assert_eq!(result.cases_total, 26);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "policy_v2")
        .all(|case| case.passed));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v018_suite_passes_with_typed_message_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v018.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v018");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert_eq!(result.cases_total, 24);
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v019_suite_passes_with_agent_passport_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v019.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v019");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert_eq!(result.cases_total, 45);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "agent_passport")
        .all(|case| case.passed));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v020_suite_passes_with_provider_harness_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v020.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v020");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "provider_harness")
        .all(|case| case.passed));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v021_suite_passes_with_feature_and_secret_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v021.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v021");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| matches!(case.category.as_str(), "feature_flags" | "secret_boundary"))
        .all(|case| case.passed));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v025_suite_passes_with_crypto_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v025.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v025");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| matches!(
            case.category.as_str(),
            "crypto_registry" | "crypto_boundaries"
        ))
        .all(|case| case.passed));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v029_suite_passes_with_atrust_handshake_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v029.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v029");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "atrust_handshakes")
        .all(|case| case.passed));
    assert!(result
        .case_results
        .iter()
        .any(|case| case.category == "atrust_handshakes"));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v030_suite_passes_with_trust_ledger_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v030.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v030");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "trust_ledgers")
        .all(|case| case.passed));
    assert!(result
        .case_results
        .iter()
        .any(|case| case.category == "trust_ledgers"));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v031_suite_passes_with_bridge_contract_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v031.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v031");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "bridge_contracts")
        .all(|case| case.passed));
    assert!(result
        .case_results
        .iter()
        .any(|case| case.category == "bridge_contracts"));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v032_suite_passes_with_atrust_evidence_map_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v032.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v032");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "atrust_evidence_maps")
        .all(|case| case.passed));
    assert!(result
        .case_results
        .iter()
        .any(|case| case.category == "atrust_evidence_maps"));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v034_suite_passes_with_public_conformance_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v034.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v034");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "public_conformance")
        .all(|case| case.passed));
    assert!(result
        .case_results
        .iter()
        .any(|case| case.category == "public_conformance"));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v035_suite_passes_with_runtime_hardening_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v035.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v035");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "runtime_hardening")
        .all(|case| case.passed));
    assert!(result
        .case_results
        .iter()
        .any(|case| case.category == "runtime_hardening"));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v036_suite_passes_with_spec_freeze_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v036.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v036");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "spec_freeze")
        .all(|case| case.passed));
    assert!(result
        .case_results
        .iter()
        .any(|case| case.category == "spec_freeze"));
    fs::remove_dir_all(workdir).unwrap();
}

#[test]
fn official_v100_suite_passes_with_runtime_mvp_cases() {
    let suite_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/suite.v100.json");
    let suite: ConformanceSuite = serde_json::from_slice(&fs::read(&suite_path).unwrap()).unwrap();
    let workdir = temp_workdir().join("v100");
    let result = run_suite(&suite, &suite_path, &workdir).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    assert!(result
        .case_results
        .iter()
        .filter(|case| case.category == "runtime_mvp")
        .all(|case| case.passed));
    assert!(result
        .case_results
        .iter()
        .any(|case| case.category == "runtime_mvp"));
    fs::remove_dir_all(workdir).unwrap();
}
