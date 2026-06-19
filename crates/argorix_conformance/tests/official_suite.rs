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
    let workdir = temp_workdir();

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
