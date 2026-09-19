//! Executable policy model for MAT-007.
//!
//! This module intentionally models lifecycle and policy decisions, not
//! cryptographic primitives. Product signatures continue to use maintained
//! Ed25519 implementations; MAT-012 owns runtime integration.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize)]
pub struct ModelPolicy {
    pub schema_version: u32,
    pub profile: String,
    pub identity_decisions: Vec<String>,
    pub key_states: Vec<String>,
    pub required_scenarios: Vec<String>,
    pub secret_forbidden_sinks: Vec<String>,
    pub recovery_quorum: usize,
    pub test_key_marker: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioFile {
    pub schema_version: u32,
    pub cases: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub operation: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub approvals: Vec<String>,
    #[serde(default)]
    pub payload: Option<String>,
    #[serde(default = "default_true")]
    pub subject_match: bool,
    #[serde(default)]
    pub condition: Option<String>,
    pub expected: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pending,
    Active,
    Suspended,
    Revoked,
    Expired,
    Compromised,
    Retired,
}

#[derive(Debug, Clone, Copy)]
pub struct AuthenticationInput {
    pub state: KeyState,
    pub anchor_known: bool,
    pub proof_valid: bool,
    pub purpose_matches: bool,
    pub challenge_fresh: bool,
}

pub fn authenticate(input: AuthenticationInput) -> &'static str {
    if !input.anchor_known {
        return "UNKNOWN";
    }
    if !input.proof_valid || !input.purpose_matches || !input.challenge_fresh {
        return "DENY";
    }
    match input.state {
        KeyState::Active => "VERIFIED",
        KeyState::Suspended => "REVIEW",
        KeyState::Pending
        | KeyState::Revoked
        | KeyState::Expired
        | KeyState::Compromised
        | KeyState::Retired => "DENY",
    }
}

fn key_input(name: &str) -> AuthenticationInput {
    let mut input = AuthenticationInput {
        state: KeyState::Active,
        anchor_known: true,
        proof_valid: true,
        purpose_matches: true,
        challenge_fresh: true,
    };
    match name {
        "active" => {}
        "foreign" => input.proof_valid = false,
        "expired" => input.state = KeyState::Expired,
        "revoked" => input.state = KeyState::Revoked,
        "unknown_anchor" => input.anchor_known = false,
        "release_only" => input.purpose_matches = false,
        "suspended" => input.state = KeyState::Suspended,
        "test" | "release" => {}
        _ => input.proof_valid = false,
    }
    input
}

pub fn recovery(old_key: &str, approvals: &[String]) -> &'static str {
    let unique: HashSet<_> = approvals.iter().collect();
    if old_key == "revoked" && unique.len() >= 2 {
        "VERIFIED"
    } else {
        "RECOVERY_PENDING"
    }
}

pub fn publish(payload: &str) -> &'static str {
    if payload.contains("ARGORIX_TEST_ONLY_CANARY_") {
        "BLOCKED_ROTATE"
    } else {
        "PUBLISHED"
    }
}

pub fn verify_provenance(key: &str, subject_match: bool) -> &'static str {
    if key == "release" && subject_match {
        "VERIFIED"
    } else {
        "DENY"
    }
}

pub fn run_scenario(case: &Scenario) -> String {
    match case.operation.as_str() {
        "authenticate" => authenticate(key_input(case.key.as_deref().unwrap_or(""))).into(),
        "metadata_auth" => "DENY".into(),
        "replay" => {
            let mut input = key_input(case.key.as_deref().unwrap_or(""));
            input.challenge_fresh = false;
            authenticate(input).into()
        }
        "recover" => recovery(case.key.as_deref().unwrap_or(""), &case.approvals).into(),
        "publish" => publish(case.payload.as_deref().unwrap_or("")).into(),
        "verify_provenance" => {
            verify_provenance(case.key.as_deref().unwrap_or(""), case.subject_match).into()
        }
        "limit" => match case.condition.as_deref() {
            Some("host_compromised") => "STOP_EXTERNAL_RECOVERY".into(),
            Some("anchor_compromised") => "STOP_REPLACE_ANCHOR".into(),
            _ => "UNKNOWN".into(),
        },
        _ => "UNKNOWN".into(),
    }
}

#[derive(Debug, Serialize)]
pub struct ScenarioResult {
    pub id: String,
    pub expected: String,
    pub observed: String,
    pub passed: bool,
}

pub fn run_all(file: &ScenarioFile) -> Vec<ScenarioResult> {
    file.cases
        .iter()
        .map(|case| {
            let observed = run_scenario(case);
            ScenarioResult {
                id: case.id.clone(),
                passed: observed == case.expected,
                expected: case.expected.clone(),
                observed,
            }
        })
        .collect()
}

/// Exhaust every key state and the four authentication gates.
pub fn exhaustive_authentication_failures() -> Vec<String> {
    let states = [
        KeyState::Pending,
        KeyState::Active,
        KeyState::Suspended,
        KeyState::Revoked,
        KeyState::Expired,
        KeyState::Compromised,
        KeyState::Retired,
    ];
    let mut failures = Vec::new();
    for state in states {
        for mask in 0_u8..16 {
            let input = AuthenticationInput {
                state,
                anchor_known: mask & 1 != 0,
                proof_valid: mask & 2 != 0,
                purpose_matches: mask & 4 != 0,
                challenge_fresh: mask & 8 != 0,
            };
            let observed = authenticate(input);
            let expected = if !input.anchor_known {
                "UNKNOWN"
            } else if !input.proof_valid || !input.purpose_matches || !input.challenge_fresh {
                "DENY"
            } else if state == KeyState::Active {
                "VERIFIED"
            } else if state == KeyState::Suspended {
                "REVIEW"
            } else {
                "DENY"
            };
            if observed != expected {
                failures.push(format!("{state:?}/{mask}: {observed} != {expected}"));
            }
        }
    }
    failures
}
