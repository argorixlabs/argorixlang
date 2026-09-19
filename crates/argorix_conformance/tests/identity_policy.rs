use argorix_conformance::identity_policy::{
    authenticate, exhaustive_authentication_failures, publish, recovery, run_all,
    verify_provenance, AuthenticationInput, KeyState, ModelPolicy, ScenarioFile,
};

fn scenarios() -> ScenarioFile {
    serde_json::from_str(include_str!("../../../conformance/identity/scenarios.json"))
        .expect("MAT-007 scenarios must parse")
}

fn policy() -> ModelPolicy {
    serde_json::from_str(include_str!(
        "../../../conformance/identity/model-policy.json"
    ))
    .expect("MAT-007 policy must parse")
}

#[test]
fn all_mat_007_scenarios_match_the_contract() {
    let file = scenarios();
    let policy = policy();
    assert_eq!(file.schema_version, 1);
    assert_eq!(policy.schema_version, 1);
    let mut required = policy.required_scenarios;
    let mut supplied: Vec<_> = file.cases.iter().map(|case| case.id.clone()).collect();
    required.sort();
    supplied.sort();
    assert_eq!(supplied, required);
    let results = run_all(&file);
    assert_eq!(results.len(), 15);
    assert!(results.iter().all(|result| result.passed), "{results:#?}");
}

#[test]
fn authentication_matrix_fails_closed() {
    assert!(exhaustive_authentication_failures().is_empty());
}

#[test]
fn recovery_never_reactivates_a_revoked_key() {
    assert_eq!(
        recovery("revoked", &["offline".into(), "operator_b".into()]),
        "VERIFIED"
    );
    assert_eq!(
        authenticate(AuthenticationInput {
            state: KeyState::Revoked,
            anchor_known: true,
            proof_valid: true,
            purpose_matches: true,
            challenge_fresh: true,
        }),
        "DENY"
    );
}

#[test]
fn secrets_and_test_keys_cannot_reach_publishable_artifacts() {
    assert_eq!(
        publish("token=ARGORIX_TEST_ONLY_CANARY_unit"),
        "BLOCKED_ROTATE"
    );
    assert_eq!(verify_provenance("test", true), "DENY");
    assert_eq!(verify_provenance("release", true), "VERIFIED");
}
