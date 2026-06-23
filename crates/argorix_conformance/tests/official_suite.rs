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
