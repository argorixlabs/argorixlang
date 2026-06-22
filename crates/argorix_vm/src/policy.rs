use crate::{EventType, ReactiveStep, RuntimeState, RuntimeStatus};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PolicyEvidenceContext {
    pub security_report_generated: bool,
    pub evidence_bundle_verified: bool,
    pub agent_passport_declared: bool,
    pub agent_passport_attested: bool,
    pub agent_data_residency_declared: bool,
    pub agent_identity_declared: bool,
    pub provider_harness_declared: bool,
    pub provider_harness_sandboxed: bool,
    pub provider_network_denied: bool,
    pub provider_secrets_denied: bool,
    pub provider_filesystem_restricted: bool,
    pub external_provider_harnessed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleEvaluation {
    pub passed: bool,
    pub reason: &'static str,
}

pub fn evaluate_rule(
    rule: &str,
    state: &RuntimeState,
    steps: &[ReactiveStep],
    context: PolicyEvidenceContext,
) -> RuleEvaluation {
    let (passed, reason) = match rule {
        "no_unhandled_messages" => (
            state.pending_messages.is_empty()
                && state.mailboxes.values().all(|mailbox| mailbox.is_empty()),
            "mailbox contains unprocessed messages",
        ),
        "all_tool_calls_traced" => (
            count(state, EventType::ToolCallDryRunResult) == state.tool_calls.len(),
            "one or more tool calls lack a dry-run trace result",
        ),
        "all_model_calls_traced" => (
            count(state, EventType::ModelCallDryRunResult) == state.model_calls.len(),
            "one or more model calls lack a dry-run trace result",
        ),
        "all_intrinsics_traced" => (
            count_any(
                state,
                &[EventType::FacuStateCheckpoint, EventType::MarronCausalGuard],
            ) == steps
                .iter()
                .map(|step| step.intrinsics.len())
                .sum::<usize>(),
            "one or more intrinsic invocations lack trace events",
        ),
        "all_provider_calls_traced" => (
            count_any(
                state,
                &[
                    EventType::ProviderResponseReceived,
                    EventType::ProviderBoundaryDenied,
                ],
            ) >= state.provider_calls.len(),
            "one or more provider calls lack boundary trace evidence",
        ),
        "halt_requires_trace" => (
            count(state, EventType::VmHalted) == 0 || count(state, EventType::ValueTraced) > 0,
            "halt occurred without a preceding trace",
        ),
        "runtime_status completed" | "runtime_status" => (
            state.status == RuntimeStatus::Completed,
            "runtime status is not completed",
        ),
        "provider_contracts_declared" => (
            state
                .provider_calls
                .iter()
                .filter(|call| call.provider != "simulated")
                .all(|call| {
                    state
                        .provider_contracts
                        .iter()
                        .any(|contract| contract.name == call.provider)
                }),
            "one or more external provider references lack a declared contract",
        ),
        "provider_allowlists_valid" => (
            true,
            "provider allowlists were not accepted by bytecode verification",
        ),
        "external_execution" => (
            count(state, EventType::ExternalProviderExecutionBlocked) > 0,
            "external provider execution was not attempted",
        ),
        "evidence_bundle_verified" => (
            context.evidence_bundle_verified,
            "no prior verified evidence bundle was supplied",
        ),
        "security_report_generated" => (
            context.security_report_generated,
            "no prior generated security report was supplied",
        ),
        "agent_passport_declared" => (
            context.agent_passport_declared,
            "one or more agents lack a declared passport",
        ),
        "agent_passport_attested" => (
            context.agent_passport_attested,
            "one or more passports lack an attestation",
        ),
        "agent_data_residency_declared" => (
            context.agent_data_residency_declared,
            "one or more passports lack declared data residency",
        ),
        "agent_identity_declared" => (
            context.agent_identity_declared,
            "one or more passports lack a declared identity",
        ),
        "provider_harness_declared" => (
            context.provider_harness_declared,
            "no provider harness is declared",
        ),
        "provider_harness_sandboxed" => (
            context.provider_harness_sandboxed,
            "one or more provider harnesses are not sandboxed",
        ),
        "provider_network_denied" => (
            context.provider_network_denied,
            "one or more provider harnesses do not deny network access",
        ),
        "provider_secrets_denied" => (
            context.provider_secrets_denied,
            "one or more provider harnesses do not deny secret access",
        ),
        "provider_filesystem_restricted" => (
            context.provider_filesystem_restricted,
            "one or more provider harnesses have unrestricted filesystem access",
        ),
        "external_provider_harnessed" => (
            context.external_provider_harnessed,
            "one or more external providers lack an associated harness",
        ),
        _ => (false, "unknown policy rule"),
    };
    RuleEvaluation { passed, reason }
}

fn count(state: &RuntimeState, kind: EventType) -> usize {
    state
        .trace_ledger
        .events
        .iter()
        .filter(|event| event.event_type == kind)
        .count()
}

fn count_any(state: &RuntimeState, kinds: &[EventType]) -> usize {
    state
        .trace_ledger
        .events
        .iter()
        .filter(|event| kinds.contains(&event.event_type))
        .count()
}
