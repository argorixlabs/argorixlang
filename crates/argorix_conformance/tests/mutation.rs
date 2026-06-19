use argorix_conformance::{
    run_suite,
    types::{ConformanceCase, ConformanceMutation, ConformanceSuite},
};
use serde_json::json;
use std::{fs, path::PathBuf};

const CATEGORIES: [&str; 13] = [
    "parser",
    "semantics",
    "ir",
    "bytecode",
    "vm",
    "policy",
    "provider_boundary",
    "adapter_contracts",
    "allowlists",
    "security_report",
    "evidence_bundle",
    "offline_verification",
    "compatibility",
];

fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "argorix-conformance-mutation-{}-{name}",
        std::process::id()
    ))
}

fn suite(root: &std::path::Path, special: ConformanceCase) -> (ConformanceSuite, PathBuf, Vec<u8>) {
    fs::create_dir_all(root.join("sources")).unwrap();
    fs::create_dir_all(root.join("bytecode")).unwrap();
    fs::write(root.join("sources/minimal.argx"), "module Minimal\n").unwrap();
    let fixture = include_bytes!("../../../examples/provider_allowlists_v014.argbc.json").to_vec();
    fs::write(root.join("bytecode/program.argbc.json"), &fixture).unwrap();
    let mut cases = CATEGORIES
        .iter()
        .enumerate()
        .map(|(index, category)| ConformanceCase {
            id: format!("basic-{index}"),
            name: format!("{category} coverage"),
            category: (*category).into(),
            source_path: Some("sources/minimal.argx".into()),
            bytecode_path: None,
            stages: vec!["parse".into()],
            injection: None,
            mutation: None,
            expected_failure_stage: None,
            expected_failure_contains: None,
        })
        .collect::<Vec<_>>();
    let index = cases
        .iter()
        .position(|case| case.category == special.category)
        .unwrap();
    cases[index] = special;
    (
        ConformanceSuite {
            suite_version: "0.15".into(),
            cases,
        },
        root.join("suite.v015.json"),
        fixture,
    )
}

fn tamper_case(
    artifact: &str,
    pointer: &str,
    value: serde_json::Value,
    expected: &str,
) -> ConformanceCase {
    ConformanceCase {
        id: format!("tamper-{artifact}"),
        name: format!("Tamper {artifact}"),
        category: "offline_verification".into(),
        source_path: None,
        bytecode_path: Some("bytecode/program.argbc.json".into()),
        stages: vec![
            "verify_bytecode".into(),
            "run_vm".into(),
            "security_report".into(),
            "trace_out".into(),
            "evidence_bundle".into(),
            "verify_evidence".into(),
        ],
        injection: Some("User:ResearchAgent:tell:UserPrompt".into()),
        mutation: Some(ConformanceMutation {
            before_stage: "verify_evidence".into(),
            artifact: artifact.into(),
            json_pointer: pointer.into(),
            value,
        }),
        expected_failure_stage: Some("verify_evidence".into()),
        expected_failure_contains: Some(expected.into()),
    }
}

#[test]
fn declaratively_mutates_report_bytecode_and_bundle_copies() {
    for (name, case) in [
        (
            "report",
            tamper_case(
                "security_report",
                "/module",
                json!("Tampered"),
                "report_digest mismatch",
            ),
        ),
        (
            "bytecode",
            tamper_case(
                "bytecode",
                "/module",
                json!("Tampered"),
                "bytecode_digest mismatch",
            ),
        ),
        (
            "bundle",
            tamper_case(
                "bundle",
                "/ledger_digest",
                json!("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                "ledger_digest mismatch",
            ),
        ),
    ] {
        let root = temp_root(name);
        let (suite, suite_path, fixture_before) = suite(&root, case);

        let result = run_suite(&suite, &suite_path, &root.join("work")).unwrap();

        assert!(result.passed, "{:?}", result.failures);
        assert_eq!(
            fs::read(root.join("bytecode/program.argbc.json")).unwrap(),
            fixture_before
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn missing_artifact_and_pointer_are_clear_expected_failures() {
    let root = temp_root("missing-artifact");
    let mut missing_artifact = tamper_case(
        "security_report",
        "/module",
        json!("Tampered"),
        "artifact `security_report` does not exist",
    );
    missing_artifact.stages = vec!["verify_bytecode".into()];
    missing_artifact.mutation.as_mut().unwrap().before_stage = "verify_bytecode".into();
    missing_artifact.expected_failure_stage = Some("verify_bytecode".into());
    let (suite_data, suite_path, _) = suite(&root, missing_artifact);
    let result = run_suite(&suite_data, &suite_path, &root.join("work")).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    fs::remove_dir_all(&root).unwrap();

    let root = temp_root("missing-pointer");
    let case = tamper_case(
        "bundle",
        "/does/not/exist",
        json!("Tampered"),
        "JSON Pointer `/does/not/exist` does not exist",
    );
    let (suite_data, suite_path, _) = suite(&root, case);
    let result = run_suite(&suite_data, &suite_path, &root.join("work")).unwrap();
    assert!(result.passed, "{:?}", result.failures);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn bytecode_mutation_before_run_vm_is_reloaded_and_reported_fail_closed() {
    let root = temp_root("runtime-block");
    let mut case = tamper_case("bytecode", "/models/0/provider", json!("OpenAI"), "unused");
    case.id = "runtime-block".into();
    case.name = "External provider runtime block".into();
    case.category = "provider_boundary".into();
    case.mutation.as_mut().unwrap().before_stage = "run_vm".into();
    case.expected_failure_stage = None;
    case.expected_failure_contains = None;
    let (suite_data, suite_path, _) = suite(&root, case);

    let result = run_suite(&suite_data, &suite_path, &root.join("work")).unwrap();

    assert!(result.passed, "{:?}", result.failures);
    let report: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("work/runtime-block/run.security.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        report["provider_boundary"]["external_execution_blocked"],
        true
    );
    assert_eq!(report["execution"]["failed"], true);
    fs::remove_dir_all(root).unwrap();
}
