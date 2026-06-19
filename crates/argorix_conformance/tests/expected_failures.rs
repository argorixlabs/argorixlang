use argorix_conformance::{
    run_suite,
    types::{ConformanceCase, ConformanceSuite},
};
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
        "argorix-conformance-failures-{}-{name}",
        std::process::id()
    ))
}

fn suite(root: &std::path::Path, special: ConformanceCase) -> (ConformanceSuite, PathBuf) {
    fs::create_dir_all(root.join("sources")).unwrap();
    fs::write(root.join("sources/good.argx"), "module Good\n").unwrap();
    fs::write(
        root.join("sources/unknown.argx"),
        "module Broken\ntype Ping { value: string }\nagent Worker { receives Ping capabilities { missing.capability } }\n",
    )
    .unwrap();
    fs::write(root.join("sources/parse-bad.argx"), "module\n").unwrap();
    let mut cases = CATEGORIES
        .iter()
        .enumerate()
        .map(|(index, category)| ConformanceCase {
            id: format!("basic-{index}"),
            name: format!("{category} coverage"),
            category: (*category).into(),
            source_path: Some("sources/good.argx".into()),
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
    )
}

fn negative_case(source: &str, expected_contains: &str) -> ConformanceCase {
    ConformanceCase {
        id: "negative".into(),
        name: "Expected semantic failure".into(),
        category: "semantics".into(),
        source_path: Some(source.into()),
        bytecode_path: None,
        stages: vec!["parse".into(), "semantic_check".into(), "emit_ir".into()],
        injection: None,
        mutation: None,
        expected_failure_stage: Some("semantic_check".into()),
        expected_failure_contains: Some(expected_contains.into()),
    }
}

#[test]
fn expected_failure_keeps_stage_failed_skips_later_and_passes_case() {
    let root = temp_root("match");
    let (suite, suite_path) = suite(
        &root,
        negative_case("sources/unknown.argx", "Unknown capability"),
    );

    let result = run_suite(&suite, &suite_path, &root.join("work")).unwrap();
    let case = result
        .case_results
        .iter()
        .find(|case| case.id == "negative")
        .unwrap();

    assert!(case.passed);
    assert_eq!(case.stages[0].status, "passed");
    assert_eq!(case.stages[1].status, "failed");
    assert_eq!(case.stages[2].status, "skipped");
    assert!(result.failures.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn diagnostic_mismatch_fails_case() {
    let root = temp_root("mismatch");
    let (suite, suite_path) = suite(
        &root,
        negative_case("sources/unknown.argx", "different diagnostic"),
    );

    let result = run_suite(&suite, &suite_path, &root.join("work")).unwrap();

    assert!(!result.passed);
    assert!(result.failures[0]
        .reason
        .contains("did not contain expected text"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn expected_stage_passing_fails_case_and_skips_later_stages() {
    let root = temp_root("unexpected-pass");
    let (suite, suite_path) = suite(
        &root,
        negative_case("sources/good.argx", "unknown capability"),
    );

    let result = run_suite(&suite, &suite_path, &root.join("work")).unwrap();
    let case = result
        .case_results
        .iter()
        .find(|case| case.id == "negative")
        .unwrap();

    assert!(!case.passed);
    assert_eq!(case.stages[1].status, "passed");
    assert_eq!(case.stages[2].status, "skipped");
    assert!(result.failures[0].reason.contains("to fail, but it passed"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn earlier_unexpected_failure_fails_case() {
    let root = temp_root("earlier");
    let (suite, suite_path) = suite(
        &root,
        negative_case("sources/parse-bad.argx", "unknown capability"),
    );

    let result = run_suite(&suite, &suite_path, &root.join("work")).unwrap();
    let case = result
        .case_results
        .iter()
        .find(|case| case.id == "negative")
        .unwrap();

    assert!(!case.passed);
    assert_eq!(case.stages[0].status, "failed");
    assert_eq!(case.stages[1].status, "skipped");
    assert_eq!(result.failures[0].stage, "parse");
    fs::remove_dir_all(root).unwrap();
}
