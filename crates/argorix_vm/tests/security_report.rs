use argorix_bytecode::{BytecodeProgram, BytecodeProviderHarness};
use argorix_vm::{
    AssertionResult, EventFields, EventType, ExecutionOutcome, InjectedMessage, RuntimeState,
    RuntimeStatus, SecurityReport, Vm, VmError,
};

fn fixture() -> BytecodeProgram {
    serde_json::from_str(include_str!(
        "../../../examples/provider_allowlists_v013.argbc.json"
    ))
    .unwrap()
}

fn injection() -> InjectedMessage {
    InjectedMessage {
        from: "User".into(),
        to: "ResearchAgent".into(),
        act: "tell".into(),
        message_type: "UserPrompt".into(),
    }
}

fn policy_fixture(action: Option<&str>) -> BytecodeProgram {
    let mut bytecode: BytecodeProgram =
        serde_json::from_str(include_str!("../../../examples/policy_v017.argbc.json")).unwrap();
    bytecode.policies[1].rules[0].rule = "evidence_bundle_verified".into();
    bytecode.policies[1].on_violation =
        action.map(|action| argorix_bytecode::BytecodePolicyViolation {
            action: action.into(),
            trace_required: true,
        });
    bytecode
}

#[test]
fn successful_report_uses_real_runtime_evidence() {
    let bytecode = fixture();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    assert!(outcome.result.is_ok());
    assert_eq!(report.report_version, "1.0");
    assert_eq!(report.bytecode_version, "0.13");
    assert_eq!(report.vm_version, "1.0");
    assert!(report.execution.completed);
    assert!(!report.execution.failed);
    assert_eq!(report.execution.steps, 3);
    assert!(report.policy.evaluated);
    assert!(report.policy.passed);
    assert_eq!(report.verdict.severity, "pass");
    assert_eq!(report.verdict.reasons, ["policy passed"]);
    assert_eq!(report.intrinsics.facu_checkpoints, 3);
    assert_eq!(report.intrinsics.marron_guards, 3);
    assert_eq!(report.intrinsics.intrinsic_events_total, 6);
    assert_eq!(report.provider_boundary.executable_providers, ["simulated"]);
    assert_eq!(outcome.state.provider_contracts.len(), 1);
    assert_eq!(report.provider_boundary.declarative_contracts.len(), 1);
    assert_eq!(
        report.provider_boundary.declarative_contracts[0].allowed_targets,
        ["GuardModel"]
    );
    assert!(report.ledger.ledger_digest.starts_with("sha256:"));
    assert_eq!(report.ledger.ledger_digest.len(), 71);
}

#[test]
fn policy_v2_review_warn_and_unhandled_violation_have_evidence_based_verdicts() {
    for (action, severity, reason) in [
        (Some("review"), "medium", "policy review required"),
        (Some("warn"), "warning", "policy warning activated"),
        (None, "medium", "policy block violated"),
    ] {
        let bytecode = policy_fixture(action);
        let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
        let report = SecurityReport::from_outcome(&bytecode, &outcome);
        assert!(outcome.result.is_ok());
        assert_eq!(report.verdict.severity, severity);
        assert_eq!(report.verdict.reasons, [reason]);
        assert_eq!(report.policy.policy_blocks_failed, 1);
        assert_eq!(report.policy.violations.len(), 1);
    }
}

#[test]
fn typed_message_report_summarizes_contracts_without_payload_execution() {
    let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
        "../../../examples/typed_messages_v018.argbc.json"
    ))
    .unwrap();
    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        InjectedMessage {
            from: "User".into(),
            to: "ResearchAgent".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );
    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.message_contracts.total, 4);
    assert_eq!(report.message_contracts.typed, 2);
    assert_eq!(report.message_contracts.untyped, 2);
    assert_eq!(report.message_contracts.fields_total, 6);
    assert_eq!(outcome.result.unwrap().message_contracts, bytecode.types);
}

#[test]
fn passport_report_summarizes_identity_and_trace_preserves_passports() {
    let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
        "../../../examples/agent_passport_v019.argbc.json"
    ))
    .unwrap();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.report_version, "1.0");
    assert_eq!(report.agent_passports.total, 1);
    assert_eq!(report.agent_passports.linked_agents, 1);
    assert_eq!(report.agent_passports.countries, vec!["CL".to_string()]);
    assert_eq!(report.agent_passports.jurisdictions, vec!["CL".to_string()]);
    assert_eq!(
        report.agent_passports.data_residency,
        vec!["CL".to_string(), "EU".to_string()]
    );
    assert_eq!(report.agent_passports.attestations_total, 3);
    assert_eq!(
        report.agent_passports.intents,
        vec!["risk_analysis".to_string()]
    );
    assert_eq!(report.agent_passports.risk_levels.get("high"), Some(&1));
    // The verdict is not inflated by the presence of a passport.
    assert!(report.verdict.passed);
    // The VM trace preserves the passport metadata verbatim.
    assert_eq!(outcome.result.unwrap().passports, bytecode.passports);
}

#[test]
fn provider_harness_report_summarizes_structural_containment() {
    let mut bytecode = fixture();
    bytecode.bytecode_version = "0.20".into();
    let provider = bytecode.providers[0].name.clone();
    bytecode.provider_harnesses = vec![BytecodeProviderHarness {
        name: "OpenAIHarness".into(),
        provider: provider.clone(),
        feature: None,
        secret: None,
        mode: "dry_run".into(),
        network: "denied".into(),
        secrets: "denied".into(),
        filesystem: "read_only".into(),
        max_steps: Some(10),
        timeout_ms: Some(1000),
        input_contract: None,
        output_contract: None,
        attestations: vec!["dry-run".into(), "policy-check".into()],
    }];
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    assert_eq!(report.report_version, "1.0");
    assert_eq!(report.provider_harnesses.total, 1);
    assert_eq!(report.provider_harnesses.providers, vec![provider]);
    assert_eq!(report.provider_harnesses.modes.get("dry_run"), Some(&1));
    assert_eq!(report.provider_harnesses.network.get("denied"), Some(&1));
    assert_eq!(report.provider_harnesses.secrets.get("denied"), Some(&1));
    assert_eq!(
        report.provider_harnesses.filesystem.get("read_only"),
        Some(&1)
    );
    assert_eq!(report.provider_harnesses.attestations_total, 2);
    assert_eq!(
        outcome.result.unwrap().provider_harnesses,
        bytecode.provider_harnesses
    );
}

#[test]
fn ledger_digest_is_deterministic_and_changes_with_evidence() {
    let bytecode = fixture();
    let outcome_a = Vm::new().run_reactive_outcome(&bytecode, injection());
    let outcome_b = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report_a = SecurityReport::from_outcome(&bytecode, &outcome_a);
    let report_b = SecurityReport::from_outcome(&bytecode, &outcome_b);
    assert_eq!(report_a.ledger.ledger_digest, report_b.ledger.ledger_digest);

    let mut changed = outcome_b;
    changed.state.trace_ledger.events[0]
        .details
        .push_str(" changed");
    let changed_report = SecurityReport::from_outcome(&bytecode, &changed);
    assert_ne!(
        report_a.ledger.ledger_digest,
        changed_report.ledger.ledger_digest
    );
}
#[test]
fn failed_execution_report_preserves_ledger_and_reports_high_severity() {
    let mut bytecode = fixture();
    bytecode.instructions.pop();
    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    assert!(outcome.result.is_err());
    assert_eq!(report.execution.status, "failed");
    assert!(report.provider_boundary.declarative_contracts.is_empty());
    assert!(report.execution.failed);
    assert!(!report.execution.completed);
    assert_eq!(report.verdict.severity, "high");
    assert!(!report.verdict.passed);
    assert_eq!(report.verdict.reasons, ["runtime failed"]);
    assert_eq!(report.ledger.last_event.as_deref(), Some("VmFailed"));
    assert!(report.ledger.events_total > 0);
}

#[test]
fn external_provider_block_event_has_highest_verdict_precedence() {
    let bytecode = fixture();
    let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
    state.trace_ledger.record(
        EventType::ExternalProviderExecutionBlocked,
        "blocked",
        "external provider execution blocked",
        EventFields::default(),
    );
    state.fail("provider boundary denied call through `OpenAI`");
    let outcome = ExecutionOutcome {
        state,
        result: Err(VmError::ProviderBoundary {
            provider: "OpenAI".into(),
            reason: "external contracts are not executable".into(),
        }),
    };
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    assert_eq!(report.provider_boundary.blocked_attempts, 1);
    assert!(report.provider_boundary.external_execution_blocked);
    assert_eq!(report.verdict.severity, "high");
    assert_eq!(
        report.verdict.reasons,
        [
            "external provider execution blocked",
            "provider boundary failure"
        ]
    );
}

#[test]
fn completed_execution_without_assertions_is_informational() {
    let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
        "../../../examples/prompt_defense_v05.argbc.json"
    ))
    .unwrap();
    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        InjectedMessage {
            from: "User".into(),
            to: "PromptScanner".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    assert_eq!(outcome.state.status, RuntimeStatus::Completed);
    assert!(!report.policy.evaluated);
    assert_eq!(report.verdict.severity, "informational");
    assert_eq!(
        report.verdict.reasons,
        ["completed without policy assertions"]
    );
}
#[test]
fn failed_assertion_on_completed_runtime_has_medium_verdict() {
    let bytecode = fixture();
    let mut outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    outcome.state.status = RuntimeStatus::Completed;
    let trace = outcome.result.as_mut().unwrap();
    trace.policy_report.status = "failed".into();
    trace.policy_report.assertions = vec![AssertionResult {
        name: "runtime_status".into(),
        argument: Some("completed".into()),
        status: "failed".into(),
        reason: Some("test evidence".into()),
    }];
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    assert_eq!(report.policy.assertions_failed, 1);
    assert_eq!(report.verdict.severity, "medium");
    assert_eq!(report.verdict.reasons, ["policy assertion failed"]);
}

#[test]
fn denied_call_on_completed_runtime_has_medium_verdict() {
    let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
        "../../../examples/prompt_defense_v05.argbc.json"
    ))
    .unwrap();
    let mut outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        InjectedMessage {
            from: "User".into(),
            to: "PromptScanner".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );
    outcome.state.trace_ledger.record(
        EventType::ToolCallDenied,
        "denied",
        "tool call denied",
        EventFields::default(),
    );
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    assert_eq!(report.calls.denied_calls_total, 1);
    assert_eq!(report.verdict.severity, "medium");
    assert_eq!(report.verdict.reasons, ["tool/model call denied"]);
}

#[test]
fn failed_execution_reconstructs_policy_only_from_ledger_evidence() {
    let bytecode = fixture();
    let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
    state.trace_ledger.record(
        EventType::AssertionFailed,
        "failed",
        "assertion runtime_status evaluated",
        EventFields::default(),
    );
    state.trace_ledger.record(
        EventType::FailureModeActivated,
        "active",
        "failure mode PolicyViolation activated",
        EventFields::default(),
    );
    state.fail("runtime failed after policy evaluation");
    let outcome = ExecutionOutcome {
        state,
        result: Err(VmError::Halted("policy failure".into())),
    };
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    assert!(report.policy.evaluated);
    assert_eq!(report.policy.failed_assertions, ["runtime_status"]);
    assert_eq!(report.policy.activated_failures, ["PolicyViolation"]);
}

#[test]
fn early_external_provider_rejection_records_blocked_evidence_for_reports() {
    let mut bytecode = fixture();
    bytecode.models[0].provider = "OpenAI".into();

    let outcome = Vm::new().run_reactive_outcome(&bytecode, injection());
    let report = SecurityReport::from_outcome(&bytecode, &outcome);

    assert!(outcome.result.is_err());
    assert!(outcome
        .state
        .trace_ledger
        .events
        .iter()
        .any(|event| event.event_type == EventType::ExternalProviderExecutionBlocked));
    assert!(report.provider_boundary.external_execution_blocked);
    assert_eq!(report.provider_boundary.blocked_attempts, 1);
}
