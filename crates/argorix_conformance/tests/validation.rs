use argorix_conformance::{
    resolve_fixture_path,
    types::{ConformanceCase, ConformanceMutation, ConformanceSuite},
    validate_suite,
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
        "argorix-conformance-validation-{}-{name}",
        std::process::id()
    ))
}

fn case(id: &str, category: &str) -> ConformanceCase {
    ConformanceCase {
        id: id.into(),
        name: format!("{category} case"),
        category: category.into(),
        source_path: Some("sources/minimal.argx".into()),
        bytecode_path: None,
        stages: vec!["parse".into()],
        injection: None,
        mutation: None,
        expected_failure_stage: None,
        expected_failure_contains: None,
    }
}

fn valid_suite(root: &std::path::Path) -> (ConformanceSuite, PathBuf) {
    fs::create_dir_all(root.join("sources")).unwrap();
    fs::write(root.join("sources/minimal.argx"), "module Minimal\n").unwrap();
    let suite_path = root.join("suite.v015.json");
    let suite = ConformanceSuite {
        suite_version: "0.15".into(),
        cases: CATEGORIES
            .iter()
            .enumerate()
            .map(|(index, category)| case(&format!("case-{index}"), category))
            .collect(),
    };
    (suite, suite_path)
}

#[test]
fn accepts_valid_portable_suite_and_resolves_from_suite_parent() {
    let root = temp_root("valid");
    let (suite, suite_path) = valid_suite(&root);

    validate_suite(&suite, &suite_path).unwrap();
    let resolved = resolve_fixture_path(&suite_path, "sources/minimal.argx").unwrap();

    assert_eq!(resolved, root.join("sources/minimal.argx"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_unknown_duplicate_and_out_of_order_stages() {
    let root = temp_root("stages");
    let (mut suite, suite_path) = valid_suite(&root);
    suite.cases[0].stages = vec!["parse".into(), "parse".into(), "blade".into()];
    suite.cases[1].stages = vec!["semantic_check".into(), "parse".into()];

    let error = validate_suite(&suite, &suite_path).unwrap_err().to_string();

    assert!(error.contains("duplicate stage `parse`"));
    assert!(error.contains("unknown stage `blade`"));
    assert!(error.contains("semantic_check requires earlier stage `parse`"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_failure_injection_and_mutation_contracts() {
    let root = temp_root("contracts");
    let (mut suite, suite_path) = valid_suite(&root);
    let case = &mut suite.cases[0];
    case.stages = vec![
        "parse".into(),
        "semantic_check".into(),
        "emit_ir".into(),
        "emit_bytecode".into(),
        "verify_bytecode".into(),
        "run_vm".into(),
    ];
    case.expected_failure_stage = Some("missing".into());
    case.expected_failure_contains = Some("boom".into());
    case.mutation = Some(ConformanceMutation {
        before_stage: "verify_evidence".into(),
        artifact: "trace".into(),
        json_pointer: "not-a-pointer".into(),
        value: json!("changed"),
    });

    let error = validate_suite(&suite, &suite_path).unwrap_err().to_string();

    assert!(error.contains("requires injection"));
    assert!(error.contains("expected failure stage `missing` is not listed"));
    assert!(error.contains("mutation stage `verify_evidence` is not listed"));
    assert!(error.contains("unsupported mutation artifact `trace`"));
    assert!(error.contains("JSON Pointer must start with `/`"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_duplicate_ids_missing_categories_and_paths_outside_suite() {
    let root = temp_root("identity");
    let (mut suite, suite_path) = valid_suite(&root);
    suite.cases[1].id = suite.cases[0].id.clone();
    suite.cases.pop();
    suite.cases[0].source_path = Some("../outside.argx".into());

    let error = validate_suite(&suite, &suite_path).unwrap_err().to_string();

    assert!(error.contains("duplicate case id"));
    assert!(error.contains("missing required category `compatibility`"));
    assert!(error.contains("escapes the portable suite tree"));
    fs::remove_dir_all(root).unwrap();
}
