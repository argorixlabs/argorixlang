use anyhow::{bail, Context, Result};
use argorix_conformance::identity_policy::{
    exhaustive_authentication_failures, run_all, ModelPolicy, ScenarioFile,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, path::PathBuf};

#[derive(Serialize)]
struct ContractCoverage {
    profile: String,
    decisions: usize,
    key_states: usize,
    forbidden_secret_sinks: usize,
    missing_scenarios: Vec<String>,
    unexpected_scenarios: Vec<String>,
    passed: bool,
}

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    task: &'static str,
    baseline_commit: String,
    model_scope: &'static str,
    policy_sha256: String,
    scenario_sha256: String,
    contract_coverage: ContractCoverage,
    scenarios_total: usize,
    scenarios_passed: usize,
    scenario_results: Vec<argorix_conformance::identity_policy::ScenarioResult>,
    exhaustive_authentication_cases: usize,
    exhaustive_failures: Vec<String>,
    secret_canary_blocked: bool,
    recovery_does_not_restore_old_key: bool,
    test_key_rejected_for_release: bool,
    overall_pass: bool,
    not_proven: Vec<&'static str>,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn main() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let policy_path = PathBuf::from(
        args.next()
            .context("usage: identity-policy-check POLICY SCENARIOS OUTPUT")?,
    );
    let scenarios_path = PathBuf::from(
        args.next()
            .context("usage: identity-policy-check POLICY SCENARIOS OUTPUT")?,
    );
    let output = PathBuf::from(
        args.next()
            .context("usage: identity-policy-check POLICY SCENARIOS OUTPUT")?,
    );
    if args.next().is_some() {
        bail!("usage: identity-policy-check POLICY SCENARIOS OUTPUT");
    }

    let policy_raw =
        fs::read(&policy_path).with_context(|| format!("read {}", policy_path.display()))?;
    let scenario_raw =
        fs::read(&scenarios_path).with_context(|| format!("read {}", scenarios_path.display()))?;
    let policy: ModelPolicy = serde_json::from_slice(&policy_raw).context("parse policy file")?;
    let file: ScenarioFile =
        serde_json::from_slice(&scenario_raw).context("parse scenario file")?;
    if policy.schema_version != 1 || file.schema_version != 1 {
        bail!("unsupported MAT-007 schema");
    }

    let required: BTreeSet<_> = policy.required_scenarios.iter().cloned().collect();
    let supplied: BTreeSet<_> = file.cases.iter().map(|case| case.id.clone()).collect();
    let missing_scenarios: Vec<_> = required.difference(&supplied).cloned().collect();
    let unexpected_scenarios: Vec<_> = supplied.difference(&required).cloned().collect();
    let coverage_passed = policy.identity_decisions.len() == 4
        && policy.key_states.len() == 7
        && policy.secret_forbidden_sinks.len() == 7
        && policy.recovery_quorum == 2
        && policy.test_key_marker == "ARGORIX_TEST_ONLY"
        && missing_scenarios.is_empty()
        && unexpected_scenarios.is_empty();
    let coverage = ContractCoverage {
        profile: policy.profile,
        decisions: policy.identity_decisions.len(),
        key_states: policy.key_states.len(),
        forbidden_secret_sinks: policy.secret_forbidden_sinks.len(),
        missing_scenarios,
        unexpected_scenarios,
        passed: coverage_passed,
    };

    let results = run_all(&file);
    let passed = results.iter().filter(|result| result.passed).count();
    let failures = exhaustive_authentication_failures();
    let secret_canary_blocked = argorix_conformance::identity_policy::publish(
        "authorization=ARGORIX_TEST_ONLY_CANARY_validation",
    ) == "BLOCKED_ROTATE";
    let recovery_does_not_restore_old_key = argorix_conformance::identity_policy::authenticate(
        argorix_conformance::identity_policy::AuthenticationInput {
            state: argorix_conformance::identity_policy::KeyState::Revoked,
            anchor_known: true,
            proof_valid: true,
            purpose_matches: true,
            challenge_fresh: true,
        },
    ) == "DENY";
    let test_key_rejected_for_release =
        argorix_conformance::identity_policy::verify_provenance("test", true) == "DENY";
    let overall_pass = coverage_passed
        && passed == results.len()
        && failures.is_empty()
        && secret_canary_blocked
        && recovery_does_not_restore_old_key
        && test_key_rejected_for_release;
    let report = Report {
        schema_version: 1,
        task: "MAT-007",
        baseline_commit: option_env!("GIT_COMMIT").unwrap_or("UNSPECIFIED").into(),
        model_scope: "Rust conformance policy model; no runtime authentication, secret broker, keystore, remote resolver or release promotion",
        policy_sha256: sha256(&policy_raw),
        scenario_sha256: sha256(&scenario_raw),
        contract_coverage: coverage,
        scenarios_total: results.len(),
        scenarios_passed: passed,
        scenario_results: results,
        exhaustive_authentication_cases: 7 * 16,
        exhaustive_failures: failures,
        secret_canary_blocked,
        recovery_does_not_restore_old_key,
        test_key_rejected_for_release,
        overall_pass,
        not_proven: vec![
            "private-key custody",
            "operating-system keystore integration",
            "runtime identity enforcement",
            "secret broker isolation",
            "remote revocation freshness",
            "release provenance pipeline",
            "compromised-host resistance",
        ],
    };
    fs::write(
        &output,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )
    .with_context(|| format!("write {}", output.display()))?;
    println!(
        "MAT-007: {passed}/{} scenarios; 112 exhaustive cases; coverage={coverage_passed}; overall_pass={overall_pass}",
        report.scenarios_total
    );
    if !overall_pass {
        bail!("MAT-007 policy validation failed");
    }
    Ok(())
}
