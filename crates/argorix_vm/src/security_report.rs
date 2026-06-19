use crate::{
    evidence::canonical_digest, EventType, ExecutionOutcome, InjectedMessage,
    ProviderContractSummary, RuntimeStatus, VmError,
};
use argorix_bytecode::BytecodeProgram;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityReport {
    pub report_version: String,
    pub language: String,
    pub module: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<argorix_bytecode::BytecodeModule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<argorix_bytecode::BytecodeModuleImport>,
    pub bytecode_version: String,
    pub vm_version: String,
    pub execution: ExecutionSummary,
    pub policy: PolicySummary,
    pub provider_boundary: ProviderBoundarySummary,
    pub calls: CallSummary,
    pub intrinsics: IntrinsicSummary,
    pub ledger: LedgerSummary,
    pub verdict: SecurityVerdict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub status: String,
    pub completed: bool,
    pub failed: bool,
    pub halted: bool,
    pub steps: usize,
    pub injected_message: Option<InjectedMessageSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectedMessageSummary {
    pub from: String,
    pub to: String,
    pub act: String,
    pub message_type: String,
}

impl From<&InjectedMessage> for InjectedMessageSummary {
    fn from(message: &InjectedMessage) -> Self {
        Self {
            from: message.from.clone(),
            to: message.to.clone(),
            act: message.act.clone(),
            message_type: message.message_type.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySummary {
    pub evaluated: bool,
    pub passed: bool,
    pub assertions_total: usize,
    pub assertions_passed: usize,
    pub assertions_failed: usize,
    pub failed_assertions: Vec<String>,
    pub activated_failures: Vec<String>,
    pub policy_blocks_total: usize,
    pub policy_blocks_passed: usize,
    pub policy_blocks_failed: usize,
    pub require_rules_total: usize,
    pub deny_rules_total: usize,
    pub violations: Vec<crate::PolicyViolation>,
    pub actions: Vec<crate::PolicyActionResult>,
    pub review_required: bool,
    pub warning: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderBoundarySummary {
    pub executable_providers: Vec<String>,
    pub declarative_contracts: Vec<ProviderContractSummary>,
    pub external_contracts_total: usize,
    pub external_execution_blocked: bool,
    pub blocked_attempts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSummary {
    pub tool_calls_total: usize,
    pub model_calls_total: usize,
    pub provider_calls_total: usize,
    pub denied_calls_total: usize,
    pub simulated_calls_total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrinsicSummary {
    pub facu_checkpoints: usize,
    pub marron_guards: usize,
    pub intrinsic_events_total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerSummary {
    pub events_total: usize,
    pub event_kinds: BTreeMap<String, usize>,
    pub first_event: Option<String>,
    pub last_event: Option<String>,
    pub ledger_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityVerdict {
    pub passed: bool,
    pub severity: String,
    pub reasons: Vec<String>,
}

impl SecurityReport {
    pub fn from_outcome(bytecode: &BytecodeProgram, outcome: &ExecutionOutcome) -> Self {
        let events = &outcome.state.trace_ledger.events;
        let trace = outcome.result.as_ref().ok();
        let completed = outcome.state.status == RuntimeStatus::Completed;
        let failed = outcome.state.status == RuntimeStatus::Failed;
        let halted = trace
            .map(|trace| trace.steps.iter().any(|step| step.halted))
            .unwrap_or_else(|| has_event(events, EventType::VmHalted));
        let execution = ExecutionSummary {
            status: runtime_status(outcome.state.status).into(),
            completed,
            failed,
            halted,
            steps: trace
                .map(|trace| trace.steps.len())
                .unwrap_or(outcome.state.completed_steps),
            injected_message: trace.map(|trace| (&trace.injected).into()),
        };

        let policy = policy_summary(outcome);
        let blocked_attempts = count_event(events, EventType::ExternalProviderExecutionBlocked);
        let declarative_contracts = trace
            .map(|trace| trace.provider_contracts.clone())
            .unwrap_or_else(|| outcome.state.provider_contracts.clone());
        let provider_boundary = ProviderBoundarySummary {
            executable_providers: trace
                .map(|trace| {
                    trace
                        .providers
                        .iter()
                        .filter(|provider| provider.enabled)
                        .map(|provider| provider.name.clone())
                        .collect()
                })
                .unwrap_or_else(|| {
                    outcome
                        .state
                        .executable_providers
                        .iter()
                        .filter(|provider| provider.enabled)
                        .map(|provider| provider.name.clone())
                        .collect()
                }),
            external_contracts_total: declarative_contracts
                .iter()
                .filter(|contract| contract.kind == "external")
                .count(),
            declarative_contracts,
            external_execution_blocked: blocked_attempts > 0,
            blocked_attempts,
        };

        let denied_calls_total = events
            .iter()
            .filter(|event| {
                matches!(
                    event.event_type,
                    EventType::ToolCallDenied
                        | EventType::ModelCallDenied
                        | EventType::ProviderBoundaryDenied
                )
            })
            .count();
        let calls = CallSummary {
            tool_calls_total: outcome.state.tool_calls.len(),
            model_calls_total: outcome.state.model_calls.len(),
            provider_calls_total: outcome.state.provider_calls.len(),
            denied_calls_total,
            simulated_calls_total: outcome
                .state
                .provider_calls
                .iter()
                .filter(|call| call.simulated)
                .count(),
        };

        let facu_checkpoints = count_event(events, EventType::FacuStateCheckpoint);
        let marron_guards = count_event(events, EventType::MarronCausalGuard);
        let intrinsics = IntrinsicSummary {
            facu_checkpoints,
            marron_guards,
            intrinsic_events_total: facu_checkpoints + marron_guards,
        };
        let ledger = ledger_summary(events);
        let verdict = verdict(outcome, &policy, &provider_boundary, &calls);

        Self {
            report_version: "0.17".into(),
            language: bytecode.language.clone(),
            module: bytecode.module.clone(),
            modules: bytecode.modules.clone(),
            imports: bytecode.imports.clone(),
            bytecode_version: bytecode.bytecode_version.clone(),
            vm_version: trace
                .map(|trace| trace.vm_version.clone())
                .unwrap_or_else(|| "0.17".into()),
            execution,
            policy,
            provider_boundary,
            calls,
            intrinsics,
            ledger,
            verdict,
        }
    }
}

fn policy_summary(outcome: &ExecutionOutcome) -> PolicySummary {
    if let Ok(trace) = &outcome.result {
        let assertions_total = trace.policy_report.assertions.len();
        let assertions_passed = trace
            .policy_report
            .assertions
            .iter()
            .filter(|assertion| assertion.status == "passed")
            .count();
        let failed_assertions = trace
            .policy_report
            .assertions
            .iter()
            .filter(|assertion| assertion.status == "failed")
            .map(|assertion| assertion.name.clone())
            .collect::<Vec<_>>();
        let policy_blocks_total = trace.policy_report.policy_blocks.len();
        let policy_blocks_passed = trace
            .policy_report
            .policy_blocks
            .iter()
            .filter(|policy| policy.passed)
            .count();
        let violations = trace
            .policy_report
            .policy_blocks
            .iter()
            .flat_map(|policy| policy.violations.iter().cloned())
            .collect::<Vec<_>>();
        let require_rules_total = trace
            .policy_report
            .policy_blocks
            .iter()
            .map(|policy| policy.require_rules.len())
            .sum();
        let deny_rules_total = trace
            .policy_report
            .policy_blocks
            .iter()
            .map(|policy| policy.deny_rules.len())
            .sum();
        return PolicySummary {
            evaluated: assertions_total + policy_blocks_total > 0,
            passed: assertions_total + policy_blocks_total > 0
                && failed_assertions.is_empty()
                && violations.is_empty(),
            assertions_total,
            assertions_passed,
            assertions_failed: failed_assertions.len(),
            failed_assertions,
            activated_failures: trace
                .policy_report
                .failures
                .iter()
                .map(|failure| failure.name.clone())
                .collect(),
            policy_blocks_total,
            policy_blocks_passed,
            policy_blocks_failed: policy_blocks_total - policy_blocks_passed,
            require_rules_total,
            deny_rules_total,
            violations,
            actions: trace.policy_report.actions.clone(),
            review_required: trace
                .policy_report
                .actions
                .iter()
                .any(|action| action.action == "review"),
            warning: trace
                .policy_report
                .actions
                .iter()
                .any(|action| action.action == "warn"),
        };
    }

    let events = &outcome.state.trace_ledger.events;
    let assertions_passed = count_event(events, EventType::AssertionVerified);
    let failed_assertions = events
        .iter()
        .filter(|event| event.event_type == EventType::AssertionFailed)
        .filter_map(|event| extract_name(&event.details, "assertion ", " evaluated"))
        .collect::<Vec<_>>();
    let assertions_failed = count_event(events, EventType::AssertionFailed);
    PolicySummary {
        evaluated: assertions_passed + assertions_failed > 0,
        passed: assertions_passed + assertions_failed > 0 && assertions_failed == 0,
        assertions_total: assertions_passed + assertions_failed,
        assertions_passed,
        assertions_failed,
        failed_assertions,
        activated_failures: events
            .iter()
            .filter(|event| event.event_type == EventType::FailureModeActivated)
            .filter_map(|event| extract_name(&event.details, "failure mode ", " activated"))
            .collect(),
        policy_blocks_total: 0,
        policy_blocks_passed: 0,
        policy_blocks_failed: count_event(events, EventType::PolicyViolation),
        require_rules_total: 0,
        deny_rules_total: 0,
        violations: Vec::new(),
        actions: Vec::new(),
        review_required: events.iter().any(|event| {
            event.event_type == EventType::PolicyActionActivated
                && event.details.contains("action review")
        }),
        warning: events.iter().any(|event| {
            event.event_type == EventType::PolicyActionActivated
                && event.details.contains("action warn")
        }),
    }
}

fn ledger_summary(events: &[crate::ExecutionEvent]) -> LedgerSummary {
    let mut event_kinds = BTreeMap::new();
    for event in events {
        *event_kinds.entry(event_kind(event.event_type)).or_insert(0) += 1;
    }
    LedgerSummary {
        events_total: events.len(),
        event_kinds,
        first_event: events.first().map(|event| event_kind(event.event_type)),
        last_event: events.last().map(|event| event_kind(event.event_type)),
        ledger_digest: canonical_digest(events).expect("ledger serialization must succeed"),
    }
}

fn verdict(
    outcome: &ExecutionOutcome,
    policy: &PolicySummary,
    provider: &ProviderBoundarySummary,
    calls: &CallSummary,
) -> SecurityVerdict {
    let provider_boundary_failure = matches!(outcome.result, Err(VmError::ProviderBoundary { .. }))
        || has_event(
            &outcome.state.trace_ledger.events,
            EventType::ProviderBoundaryDenied,
        );
    if provider.external_execution_blocked || provider_boundary_failure {
        let mut reasons = Vec::new();
        if provider.external_execution_blocked {
            reasons.push("external provider execution blocked".into());
        }
        if provider_boundary_failure {
            reasons.push("provider boundary failure".into());
        }
        return SecurityVerdict {
            passed: false,
            severity: "high".into(),
            reasons,
        };
    }
    if policy
        .actions
        .iter()
        .any(|action| action.action == "block")
    {
        return SecurityVerdict {
            passed: false,
            severity: "high".into(),
            reasons: vec!["policy block action activated".into()],
        };
    }
    if outcome.state.status == RuntimeStatus::Failed {
        return SecurityVerdict {
            passed: false,
            severity: "high".into(),
            reasons: vec!["runtime failed".into()],
        };
    }
    if policy.review_required {
        return SecurityVerdict {
            passed: false,
            severity: "medium".into(),
            reasons: vec!["policy review required".into()],
        };
    }
    if policy.warning {
        return SecurityVerdict {
            passed: false,
            severity: "warning".into(),
            reasons: vec!["policy warning activated".into()],
        };
    }
    if policy.assertions_failed > 0 {
        return SecurityVerdict {
            passed: false,
            severity: "medium".into(),
            reasons: vec!["policy assertion failed".into()],
        };
    }
    if calls.denied_calls_total > 0 {
        return SecurityVerdict {
            passed: false,
            severity: "medium".into(),
            reasons: vec!["tool/model call denied".into()],
        };
    }
    if policy.policy_blocks_failed > 0 {
        return SecurityVerdict {
            passed: false,
            severity: "medium".into(),
            reasons: vec!["policy block violated".into()],
        };
    }
    if !policy.evaluated {
        return SecurityVerdict {
            passed: true,
            severity: "informational".into(),
            reasons: vec!["completed without policy assertions".into()],
        };
    }
    SecurityVerdict {
        passed: policy.passed,
        severity: "pass".into(),
        reasons: vec!["policy passed".into()],
    }
}

fn runtime_status(status: RuntimeStatus) -> &'static str {
    match status {
        RuntimeStatus::Initialized => "initialized",
        RuntimeStatus::Running => "running",
        RuntimeStatus::Completed => "completed",
        RuntimeStatus::Failed => "failed",
    }
}

fn count_event(events: &[crate::ExecutionEvent], event_type: EventType) -> usize {
    events
        .iter()
        .filter(|event| event.event_type == event_type)
        .count()
}

fn has_event(events: &[crate::ExecutionEvent], event_type: EventType) -> bool {
    events.iter().any(|event| event.event_type == event_type)
}

fn event_kind(event_type: EventType) -> String {
    serde_json::to_value(event_type)
        .expect("event type serialization must succeed")
        .as_str()
        .expect("event type must serialize as a string")
        .to_owned()
}

fn extract_name(details: &str, prefix: &str, suffix: &str) -> Option<String> {
    details
        .strip_prefix(prefix)
        .and_then(|value| value.strip_suffix(suffix))
        .map(str::to_owned)
}
