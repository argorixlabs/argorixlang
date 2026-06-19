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
        "argorix-conformance-pipeline-{}-{name}",
        std::process::id()
    ))
}

fn basic_case(id: &str, category: &str) -> ConformanceCase {
    ConformanceCase {
        id: id.into(),
        name: format!("{category} coverage"),
        category: category.into(),
        source_path: Some("sources/program.argx".into()),
        bytecode_path: None,
        stages: vec!["parse".into()],
        injection: None,
        mutation: None,
        expected_failure_stage: None,
        expected_failure_contains: None,
    }
}

fn suite_with(root: &std::path::Path, special: ConformanceCase) -> (ConformanceSuite, PathBuf) {
    fs::create_dir_all(root.join("sources")).unwrap();
    fs::write(
        root.join("sources/program.argx"),
        include_str!("../../../examples/provider_allowlists_v014.argx"),
    )
    .unwrap();
    let mut cases = CATEGORIES
        .iter()
        .enumerate()
        .map(|(index, category)| basic_case(&format!("basic-{index}"), category))
        .collect::<Vec<_>>();
    let category_index = cases
        .iter()
        .position(|case| case.category == special.category)
        .unwrap();
    cases[category_index] = special;
    (
        ConformanceSuite {
            suite_version: "0.15".into(),
            cases,
        },
        root.join("suite.v015.json"),
    )
}

#[test]
fn executes_source_pipeline_and_writes_ir_and_bytecode_in_case_workdir() {
    let root = temp_root("compiler");
    let workdir = root.join("work");
    let case = ConformanceCase {
        id: "compiler-pipeline".into(),
        name: "Compiler pipeline".into(),
        category: "bytecode".into(),
        source_path: Some("sources/program.argx".into()),
        bytecode_path: None,
        stages: vec![
            "parse".into(),
            "semantic_check".into(),
            "emit_ir".into(),
            "emit_bytecode".into(),
            "verify_bytecode".into(),
        ],
        injection: None,
        mutation: None,
        expected_failure_stage: None,
        expected_failure_contains: None,
    };
    let (suite, suite_path) = suite_with(&root, case);

    let result = run_suite(&suite, &suite_path, &workdir).unwrap();

    let case_result = result
        .case_results
        .iter()
        .find(|case| case.id == "compiler-pipeline")
        .unwrap();
    assert!(case_result.passed);
    assert!(case_result
        .stages
        .iter()
        .all(|stage| stage.status == "passed"));
    assert!(workdir.join("compiler-pipeline/ir.json").exists());
    assert!(workdir
        .join("compiler-pipeline/program.argbc.json")
        .exists());
    let result_json = serde_json::to_string(&result).unwrap();
    assert!(!result_json.contains(&root.to_string_lossy().to_string()));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn executes_vm_report_trace_bundle_and_offline_verification() {
    let root = temp_root("evidence");
    let workdir = root.join("work");
    let case = ConformanceCase {
        id: "evidence-pipeline".into(),
        name: "Evidence pipeline".into(),
        category: "evidence_bundle".into(),
        source_path: Some("sources/program.argx".into()),
        bytecode_path: None,
        stages: vec![
            "parse".into(),
            "semantic_check".into(),
            "emit_ir".into(),
            "emit_bytecode".into(),
            "verify_bytecode".into(),
            "run_vm".into(),
            "security_report".into(),
            "trace_out".into(),
            "evidence_bundle".into(),
            "verify_evidence".into(),
        ],
        injection: Some("User:ResearchAgent:tell:UserPrompt".into()),
        mutation: None,
        expected_failure_stage: None,
        expected_failure_contains: None,
    };
    let (suite, suite_path) = suite_with(&root, case);

    let result = run_suite(&suite, &suite_path, &workdir).unwrap();

    let case_result = result
        .case_results
        .iter()
        .find(|case| case.id == "evidence-pipeline")
        .unwrap();
    assert!(case_result.passed, "{:?}", result.failures);
    let case_dir = workdir.join("evidence-pipeline");
    for artifact in [
        "ir.json",
        "program.argbc.json",
        "run.security.json",
        "run.trace.json",
        "run.bundle.json",
    ] {
        assert!(case_dir.join(artifact).exists(), "{artifact}");
    }
    let bundle: serde_json::Value =
        serde_json::from_slice(&fs::read(case_dir.join("run.bundle.json")).unwrap()).unwrap();
    assert_eq!(bundle["artifacts"]["bytecode_path"], "program.argbc.json");
    assert_eq!(bundle["artifacts"]["trace_path"], "run.trace.json");
    assert_eq!(
        bundle["artifacts"]["security_report_path"],
        "run.security.json"
    );
    fs::remove_dir_all(root).unwrap();
}
