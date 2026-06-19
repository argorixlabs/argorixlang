use argorix_conformance::types::{ConformanceCase, ConformanceSuite};
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};

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
        "argorix-conformance-cli-{}-{name}",
        std::process::id()
    ))
}

fn write_suite(root: &std::path::Path, failing: bool) -> PathBuf {
    fs::create_dir_all(root.join("sources")).unwrap();
    fs::write(root.join("sources/minimal.argx"), "module Minimal\n").unwrap();
    let mut cases = CATEGORIES
        .iter()
        .enumerate()
        .map(|(index, category)| ConformanceCase {
            id: format!("case-{index}"),
            name: format!("{category} case"),
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
    if failing {
        cases[0].expected_failure_stage = Some("parse".into());
        cases[0].expected_failure_contains = Some("expected failure".into());
    }
    let suite = ConformanceSuite {
        suite_version: "0.15".into(),
        cases,
    };
    let path = root.join("suite.v015.json");
    fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&suite).unwrap()),
    )
    .unwrap();
    path
}

#[test]
fn text_mode_uses_default_workdir_and_prints_clear_summary() {
    let root = temp_root("text");
    let suite = write_suite(&root, false);
    let output = Command::new(env!("CARGO_BIN_EXE_argorix-conformance"))
        .current_dir(&root)
        .args(["run"])
        .arg(&suite)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Argorix Conformance Suite v0.15"));
    assert!(stdout.contains("Cases: 13"));
    assert!(stdout.contains("Passed: 13"));
    assert!(stdout.contains("Conformance: passed"));
    assert!(root.join("target/argorix-conformance/case-0").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn json_mode_is_clean_and_honors_custom_workdir() {
    let root = temp_root("json");
    let suite = write_suite(&root, false);
    let workdir = root.join("custom-work");
    let output = Command::new(env!("CARGO_BIN_EXE_argorix-conformance"))
        .args(["run"])
        .arg(&suite)
        .arg("--workdir")
        .arg(&workdir)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["passed"], true);
    assert_eq!(result["cases_total"], 13);
    assert!(output.stderr.is_empty());
    assert!(workdir.join("case-0").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_result_exits_nonzero_but_still_prints_result_json() {
    let root = temp_root("failed");
    let suite = write_suite(&root, true);
    let output = Command::new(env!("CARGO_BIN_EXE_argorix-conformance"))
        .args(["run"])
        .arg(&suite)
        .arg("--workdir")
        .arg(root.join("work"))
        .arg("--json")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["passed"], false);
    assert_eq!(result["cases_failed"], 1);
    fs::remove_dir_all(root).unwrap();
}
