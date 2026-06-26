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
    pub feature_flags_declared: bool,
    pub features_default_disabled: bool,
    pub experimental_features_require_approval: bool,
    pub secret_boundaries_declared: bool,
    pub secret_access_denied: bool,
    pub secret_values_absent: bool,
    pub external_provider_feature_gated: bool,
    pub external_provider_secret_boundary_declared: bool,
    pub adapters_declared: bool,
    pub adapters_execution_disabled: bool,
    pub adapters_network_denied: bool,
    pub adapters_secrets_denied: bool,
    pub adapters_provider_harnessed: bool,
    pub adapters_feature_gated: bool,
    pub adapters_secret_boundaried: bool,
    pub adapters_conformance_declared: bool,
    pub adapters_evidence_required: bool,
    pub adapter_profiles_declared: bool,
    pub adapter_profiles_execution_disabled: bool,
    pub adapter_profiles_network_denied: bool,
    pub adapter_profiles_secrets_denied: bool,
    pub adapter_profiles_linked: bool,
    pub adapter_profiles_conformance_declared: bool,
    pub vendor_profiles_declared: bool,
    pub crypto_primitives_declared: bool,
    pub crypto_primitives_allowed: bool,
    pub crypto_denied_not_used: bool,
    pub crypto_post_quantum_candidates_declared: bool,
    pub crypto_key_material_absent: bool,
    pub crypto_secret_material_absent: bool,
    pub crypto_execution_absent: bool,
    pub crypto_boundaries_declared: bool,
    pub post_quantum_readiness_declared: bool,
    pub atrust_evidence_maps_declared: bool,
    pub atrust_evidence_map_agents_bound: bool,
    pub atrust_evidence_map_passports_bound: bool,
    pub atrust_evidence_map_identities_bound: bool,
    pub atrust_evidence_map_credentials_bound: bool,
    pub atrust_evidence_map_handshakes_bound: bool,
    pub atrust_evidence_map_ledgers_bound: bool,
    pub atrust_evidence_map_bridges_bound: bool,
    pub atrust_evidence_map_policies_bound: bool,
    pub atrust_evidence_map_coverage_required: bool,
    pub atrust_evidence_map_verification_non_verifying: bool,
    pub atrust_evidence_map_resolution_disabled: bool,
    pub atrust_evidence_map_network_denied: bool,
    pub atrust_evidence_map_external_execution_disabled: bool,
    pub atrust_evidence_map_secret_material_denied: bool,
    pub atrust_evidence_map_key_material_denied: bool,
    pub atrust_evidence_map_execution_disabled: bool,
    pub atrust_evidence_map_security_claims_absent: bool,
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
        "feature_flags_declared" => (
            context.feature_flags_declared,
            "no feature flag is declared",
        ),
        "features_default_disabled" => (
            context.features_default_disabled,
            "one or more features do not default to disabled",
        ),
        "experimental_features_require_approval" => (
            context.experimental_features_require_approval,
            "one or more experimental or preview features lack required approval",
        ),
        "secret_boundaries_declared" => (
            context.secret_boundaries_declared,
            "no secret boundary is declared",
        ),
        "secret_access_denied" => (
            context.secret_access_denied,
            "one or more secret boundaries do not deny access",
        ),
        "secret_values_absent" => (
            context.secret_values_absent,
            "one or more secret declarations contain secret material",
        ),
        "external_provider_feature_gated" => (
            context.external_provider_feature_gated,
            "one or more external providers lack a disabled, approval-gated feature",
        ),
        "external_provider_secret_boundary_declared" => (
            context.external_provider_secret_boundary_declared,
            "one or more external providers lack a denied/none secret boundary",
        ),
        "adapters_declared" => (context.adapters_declared, "no adapter is declared"),
        "adapters_execution_disabled" => (
            context.adapters_execution_disabled,
            "one or more adapters do not have execution disabled",
        ),
        "adapters_network_denied" => (
            context.adapters_network_denied,
            "one or more adapters do not deny network access",
        ),
        "adapters_secrets_denied" => (
            context.adapters_secrets_denied,
            "one or more adapters do not deny secret access",
        ),
        "adapters_provider_harnessed" => (
            context.adapters_provider_harnessed,
            "one or more external-provider adapters lack a harness",
        ),
        "adapters_feature_gated" => (
            context.adapters_feature_gated,
            "one or more external-provider adapters lack a disabled approval-gated feature",
        ),
        "adapters_secret_boundaried" => (
            context.adapters_secret_boundaried,
            "one or more external-provider adapters lack a denied/none secret boundary",
        ),
        "adapters_conformance_declared" => (
            context.adapters_conformance_declared,
            "one or more adapters lack a non-empty conformance list",
        ),
        "adapters_evidence_required" => (
            context.adapters_evidence_required,
            "one or more adapters with conformance do not require evidence",
        ),
        "adapter_profiles_declared" => (
            context.adapter_profiles_declared,
            "no adapter profile is declared",
        ),
        "adapter_profiles_execution_disabled" => (
            context.adapter_profiles_execution_disabled,
            "one or more adapter profiles do not have execution disabled",
        ),
        "adapter_profiles_network_denied" => (
            context.adapter_profiles_network_denied,
            "one or more adapter profiles do not deny network access",
        ),
        "adapter_profiles_secrets_denied" => (
            context.adapter_profiles_secrets_denied,
            "one or more adapter profiles do not deny secret access",
        ),
        "adapter_profiles_linked" => (
            context.adapter_profiles_linked,
            "one or more adapter profiles have mismatched or missing adapter/provider links",
        ),
        "adapter_profiles_conformance_declared" => (
            context.adapter_profiles_conformance_declared,
            "one or more adapter profiles lack required_conformance",
        ),
        "vendor_profiles_declared" => (
            context.vendor_profiles_declared,
            "no vendor profile is declared",
        ),
        "crypto_primitives_declared" => (
            context.crypto_primitives_declared,
            "no crypto primitive is declared",
        ),
        "crypto_primitives_allowed" => (
            context.crypto_primitives_allowed,
            "one or more crypto primitives are denied or invalid",
        ),
        "crypto_denied_not_used" => (
            context.crypto_denied_not_used,
            "one or more denied crypto primitives are referenced",
        ),
        "crypto_post_quantum_candidates_declared" => (
            context.crypto_post_quantum_candidates_declared,
            "no post-quantum candidate crypto primitive is declared",
        ),
        "crypto_key_material_absent" => (
            context.crypto_key_material_absent,
            "crypto key material is present",
        ),
        "crypto_secret_material_absent" => (
            context.crypto_secret_material_absent,
            "crypto secret material is present",
        ),
        "crypto_execution_absent" => (
            context.crypto_execution_absent,
            "crypto execution primitives or operations are present",
        ),
        "crypto_boundaries_declared" => (
            context.crypto_boundaries_declared,
            "no crypto boundary is declared",
        ),
        "post_quantum_readiness_declared" => (
            context.post_quantum_readiness_declared,
            "no crypto boundary declares post-quantum readiness",
        ),
        "atrust_evidence_maps_declared" => (
            context.atrust_evidence_maps_declared,
            "no atrust_evidence_map is declared",
        ),
        "atrust_evidence_map_agents_bound" => (
            context.atrust_evidence_map_agents_bound,
            "one or more evidence maps lack an agent binding",
        ),
        "atrust_evidence_map_passports_bound" => (
            context.atrust_evidence_map_passports_bound,
            "one or more evidence maps lack a passport binding",
        ),
        "atrust_evidence_map_identities_bound" => (
            context.atrust_evidence_map_identities_bound,
            "one or more evidence maps lack an identity binding",
        ),
        "atrust_evidence_map_credentials_bound" => (
            context.atrust_evidence_map_credentials_bound,
            "one or more evidence maps lack a credential binding",
        ),
        "atrust_evidence_map_handshakes_bound" => (
            context.atrust_evidence_map_handshakes_bound,
            "one or more evidence maps lack a handshake binding",
        ),
        "atrust_evidence_map_ledgers_bound" => (
            context.atrust_evidence_map_ledgers_bound,
            "one or more evidence maps lack a ledger binding",
        ),
        "atrust_evidence_map_bridges_bound" => (
            context.atrust_evidence_map_bridges_bound,
            "one or more evidence maps lack bridge bindings",
        ),
        "atrust_evidence_map_policies_bound" => (
            context.atrust_evidence_map_policies_bound,
            "one or more evidence maps lack policy bindings",
        ),
        "atrust_evidence_map_coverage_required" => (
            context.atrust_evidence_map_coverage_required,
            "one or more evidence maps do not require coverage",
        ),
        "atrust_evidence_map_verification_non_verifying" => (
            context.atrust_evidence_map_verification_non_verifying,
            "one or more evidence maps claim real verification",
        ),
        "atrust_evidence_map_resolution_disabled" => (
            context.atrust_evidence_map_resolution_disabled,
            "one or more evidence maps allow resolution",
        ),
        "atrust_evidence_map_network_denied" => (
            context.atrust_evidence_map_network_denied,
            "one or more evidence maps do not deny network access",
        ),
        "atrust_evidence_map_external_execution_disabled" => (
            context.atrust_evidence_map_external_execution_disabled,
            "one or more evidence maps allow external execution",
        ),
        "atrust_evidence_map_secret_material_denied" => (
            context.atrust_evidence_map_secret_material_denied,
            "one or more evidence maps allow secret material",
        ),
        "atrust_evidence_map_key_material_denied" => (
            context.atrust_evidence_map_key_material_denied,
            "one or more evidence maps allow key material",
        ),
        "atrust_evidence_map_execution_disabled" => (
            context.atrust_evidence_map_execution_disabled,
            "one or more evidence maps allow execution",
        ),
        "atrust_evidence_map_security_claims_absent" => (
            context.atrust_evidence_map_security_claims_absent,
            "one or more evidence maps declare security claims",
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
