use argorix_conformance::core_spec::{validate_core_spec, CaseManifest, CoreSpec};
use std::path::PathBuf;

fn roots() -> (PathBuf, PathBuf) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    (repo.join("spec/core"), repo.join("tests/selfhost/spec"))
}

fn documents() -> (CoreSpec, CaseManifest) {
    let spec = serde_json::from_str(include_str!("../../../spec/core/core-spec.json"))
        .expect("Core spec must parse");
    let cases = serde_json::from_str(include_str!("../../../tests/selfhost/spec/cases.json"))
        .expect("Core cases must parse");
    (spec, cases)
}

#[test]
fn core_spec_and_corpus_are_complete() {
    let (spec, cases) = documents();
    let (spec_dir, fixture_dir) = roots();
    let result = validate_core_spec(&spec, &cases, &spec_dir, &fixture_dir);
    assert!(result.passed, "{:#?}", result.failures);
    assert_eq!(result.documents_checked, 5);
    assert_eq!(result.valid_fixtures_checked, 4);
    assert_eq!(result.invalid_fixtures_checked, 12);
    assert_eq!(result.constructs_covered, 32);
    assert_eq!(result.demonstrations_covered, 3);
    assert!(result.forbidden_magic_control_detected);
}

#[test]
fn missing_construct_is_detected() {
    let (mut spec, cases) = documents();
    let (spec_dir, fixture_dir) = roots();
    spec.required_constructs.push("mutation_canary".into());
    let result = validate_core_spec(&spec, &cases, &spec_dir, &fixture_dir);
    assert!(!result.passed);
    assert!(result
        .failures
        .iter()
        .any(|failure| failure.contains("construct coverage")));
}

#[test]
fn unsafe_fixture_path_is_rejected() {
    let (spec, mut cases) = documents();
    let (spec_dir, fixture_dir) = roots();
    cases.valid_cases[0].file = "../outside.argx".into();
    let result = validate_core_spec(&spec, &cases, &spec_dir, &fixture_dir);
    assert!(!result.passed);
    assert!(result
        .failures
        .iter()
        .any(|failure| failure.contains("unsafe artifact path")));
}
