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
    pub governance_profiles_declared: bool,
    pub governance_profiles_evidence_bound: bool,
    pub governance_profiles_controls_mapped: bool,
    pub governance_profiles_runtime_disabled: bool,
    pub governance_profiles_security_claims_absent: bool,
    pub governance_profiles_no_legal_certification: bool,
    pub regulatory_mappings_declared: bool,
    pub regulatory_mappings_profiles_bound: bool,
    pub regulatory_mappings_obligations_mapped: bool,
    pub regulatory_mappings_controls_bound: bool,
    pub regulatory_mappings_legal_claims_absent: bool,
    pub regulatory_mappings_certification_absent: bool,
    pub regulatory_mappings_runtime_disabled: bool,
    pub regulatory_mappings_security_claims_absent: bool,
    pub third_party_verifiers_declared: bool,
    pub third_party_verifiers_identity_declared: bool,
    pub third_party_verifiers_scope_bounded: bool,
    pub third_party_verifiers_runtime_disabled: bool,
    pub third_party_verifiers_legal_claims_absent: bool,
    pub third_party_verifiers_certification_absent: bool,
    pub third_party_verifiers_security_claims_absent: bool,
    pub public_conformance_reports_declared: bool,
    pub public_conformance_reports_verifiers_bound: bool,
    pub public_conformance_reports_artifacts_declared: bool,
    pub public_conformance_reports_evidence_bound: bool,
    pub public_conformance_reports_governance_bound: bool,
    pub public_conformance_reports_regulatory_bound: bool,
    pub public_conformance_reports_replayable: bool,
    pub public_conformance_reports_runtime_disabled: bool,
    pub public_conformance_reports_legal_claims_absent: bool,
    pub public_conformance_reports_certification_absent: bool,
    pub public_conformance_reports_security_claims_absent: bool,
    pub runtime_hardening_profiles_declared: bool,
    pub runtime_hardening_evidence_bound: bool,
    pub runtime_hardening_deny_by_default: bool,
    pub runtime_hardening_sandbox_required: bool,
    pub runtime_hardening_network_denied: bool,
    pub runtime_hardening_external_providers_disabled: bool,
    pub runtime_hardening_tool_execution_disabled: bool,
    pub runtime_hardening_agent_execution_disabled: bool,
    pub runtime_hardening_filesystem_denied: bool,
    pub runtime_hardening_env_denied: bool,
    pub runtime_hardening_secret_material_denied: bool,
    pub runtime_hardening_key_material_denied: bool,
    pub runtime_hardening_audit_log_required: bool,
    pub runtime_hardening_security_claims_absent: bool,
    pub threat_models_declared: bool,
    pub threat_models_hardening_bound: bool,
    pub threat_models_assets_mapped: bool,
    pub threat_models_threats_mapped: bool,
    pub threat_models_mitigations_mapped: bool,
    pub threat_models_runtime_disabled: bool,
    pub threat_models_network_denied: bool,
    pub threat_models_secret_material_denied: bool,
    pub threat_models_key_material_denied: bool,
    pub threat_models_execution_disabled: bool,
    pub threat_models_security_claims_absent: bool,
    pub spec_freezes_declared: bool,
    pub spec_freeze_versions_pinned: bool,
    pub spec_freeze_features_declared: bool,
    pub spec_freeze_compatibility_declared: bool,
    pub spec_freeze_required_suites_declared: bool,
    pub spec_freeze_runtime_disabled: bool,
    pub spec_freeze_network_denied: bool,
    pub spec_freeze_external_execution_disabled: bool,
    pub spec_freeze_provider_execution_disabled: bool,
    pub spec_freeze_secret_material_denied: bool,
    pub spec_freeze_key_material_denied: bool,
    pub spec_freeze_env_denied: bool,
    pub spec_freeze_filesystem_denied: bool,
    pub spec_freeze_security_claims_absent: bool,
    pub spec_freeze_legal_claims_absent: bool,
    pub spec_freeze_certification_absent: bool,
    pub release_candidates_declared: bool,
    pub release_candidates_spec_freeze_bound: bool,
    pub release_candidates_artifacts_declared: bool,
    pub release_candidates_checks_declared: bool,
    pub release_candidates_compatibility_matrix_declared: bool,
    pub release_candidates_limitations_declared: bool,
    pub release_candidates_runtime_disabled: bool,
    pub release_candidates_network_denied: bool,
    pub release_candidates_external_execution_disabled: bool,
    pub release_candidates_provider_execution_disabled: bool,
    pub release_candidates_secret_material_denied: bool,
    pub release_candidates_key_material_denied: bool,
    pub release_candidates_env_denied: bool,
    pub release_candidates_filesystem_denied: bool,
    pub release_candidates_security_claims_absent: bool,
    pub release_candidates_legal_claims_absent: bool,
    pub release_candidates_certification_absent: bool,
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
        "governance_profiles_declared" => (
            context.governance_profiles_declared,
            "no governance_profile is declared",
        ),
        "governance_profiles_evidence_bound" => (
            context.governance_profiles_evidence_bound,
            "one or more governance profiles lack evidence or ledger bindings",
        ),
        "governance_profiles_controls_mapped" => (
            context.governance_profiles_controls_mapped,
            "one or more governance profiles lack mapped controls",
        ),
        "governance_profiles_runtime_disabled" => (
            context.governance_profiles_runtime_disabled,
            "one or more governance profiles enable runtime capabilities",
        ),
        "governance_profiles_security_claims_absent" => (
            context.governance_profiles_security_claims_absent,
            "one or more governance profiles declare security claims",
        ),
        "governance_profiles_no_legal_certification" => (
            context.governance_profiles_no_legal_certification,
            "one or more governance profiles claim legal certification",
        ),
        "regulatory_mappings_declared" => (
            context.regulatory_mappings_declared,
            "no regulatory_mapping is declared",
        ),
        "regulatory_mappings_profiles_bound" => (
            context.regulatory_mappings_profiles_bound,
            "one or more regulatory mappings lack profile/evidence bindings",
        ),
        "regulatory_mappings_obligations_mapped" => (
            context.regulatory_mappings_obligations_mapped,
            "one or more regulatory mappings lack obligations",
        ),
        "regulatory_mappings_controls_bound" => (
            context.regulatory_mappings_controls_bound,
            "one or more obligations reference an unknown control",
        ),
        "regulatory_mappings_legal_claims_absent" => (
            context.regulatory_mappings_legal_claims_absent,
            "one or more regulatory mappings declare legal claims",
        ),
        "regulatory_mappings_certification_absent" => (
            context.regulatory_mappings_certification_absent,
            "one or more regulatory mappings declare certification",
        ),
        "regulatory_mappings_runtime_disabled" => (
            context.regulatory_mappings_runtime_disabled,
            "one or more regulatory mappings enable runtime capabilities",
        ),
        "regulatory_mappings_security_claims_absent" => (
            context.regulatory_mappings_security_claims_absent,
            "one or more regulatory mappings declare security claims",
        ),
        "third_party_verifiers_declared" => (
            context.third_party_verifiers_declared,
            "no third_party_verifier is declared",
        ),
        "third_party_verifiers_identity_declared" => (
            context.third_party_verifiers_identity_declared,
            "one or more verifiers lack declared identity metadata",
        ),
        "third_party_verifiers_scope_bounded" => (
            context.third_party_verifiers_scope_bounded,
            "one or more verifiers lack bounded scopes or disallowed claims",
        ),
        "third_party_verifiers_runtime_disabled" => (
            context.third_party_verifiers_runtime_disabled,
            "one or more verifiers enable runtime capabilities",
        ),
        "third_party_verifiers_legal_claims_absent" => (
            context.third_party_verifiers_legal_claims_absent,
            "one or more verifiers declare legal claims",
        ),
        "third_party_verifiers_certification_absent" => (
            context.third_party_verifiers_certification_absent,
            "one or more verifiers declare certification",
        ),
        "third_party_verifiers_security_claims_absent" => (
            context.third_party_verifiers_security_claims_absent,
            "one or more verifiers declare security claims",
        ),
        "public_conformance_reports_declared" => (
            context.public_conformance_reports_declared,
            "no public_conformance_report is declared",
        ),
        "public_conformance_reports_verifiers_bound" => (
            context.public_conformance_reports_verifiers_bound,
            "one or more reports reference an unknown verifier",
        ),
        "public_conformance_reports_artifacts_declared" => (
            context.public_conformance_reports_artifacts_declared,
            "one or more reports lack suite/source/bytecode artifacts",
        ),
        "public_conformance_reports_evidence_bound" => (
            context.public_conformance_reports_evidence_bound,
            "one or more reports lack evidence bindings",
        ),
        "public_conformance_reports_governance_bound" => (
            context.public_conformance_reports_governance_bound,
            "one or more reports lack governance bindings",
        ),
        "public_conformance_reports_regulatory_bound" => (
            context.public_conformance_reports_regulatory_bound,
            "one or more reports lack regulatory bindings",
        ),
        "public_conformance_reports_replayable" => (
            context.public_conformance_reports_replayable,
            "one or more reports do not declare reproducibility",
        ),
        "public_conformance_reports_runtime_disabled" => (
            context.public_conformance_reports_runtime_disabled,
            "one or more reports enable runtime capabilities",
        ),
        "public_conformance_reports_legal_claims_absent" => (
            context.public_conformance_reports_legal_claims_absent,
            "one or more reports declare legal claims",
        ),
        "public_conformance_reports_certification_absent" => (
            context.public_conformance_reports_certification_absent,
            "one or more reports declare certification",
        ),
        "public_conformance_reports_security_claims_absent" => (
            context.public_conformance_reports_security_claims_absent,
            "one or more reports declare security claims",
        ),
        "runtime_hardening_profiles_declared" => (
            context.runtime_hardening_profiles_declared,
            "no runtime hardening profile is declared",
        ),
        "runtime_hardening_evidence_bound" => (
            context.runtime_hardening_evidence_bound,
            "one or more hardening profiles lack evidence bindings",
        ),
        "runtime_hardening_deny_by_default" => (
            context.runtime_hardening_deny_by_default,
            "one or more hardening profiles do not deny by default",
        ),
        "runtime_hardening_sandbox_required" => (
            context.runtime_hardening_sandbox_required,
            "one or more hardening profiles do not require a sandbox",
        ),
        "runtime_hardening_network_denied" => (
            context.runtime_hardening_network_denied,
            "one or more hardening profiles allow network access",
        ),
        "runtime_hardening_external_providers_disabled" => (
            context.runtime_hardening_external_providers_disabled,
            "one or more hardening profiles enable external providers",
        ),
        "runtime_hardening_tool_execution_disabled" => (
            context.runtime_hardening_tool_execution_disabled,
            "one or more hardening profiles enable tools",
        ),
        "runtime_hardening_agent_execution_disabled" => (
            context.runtime_hardening_agent_execution_disabled,
            "one or more hardening profiles enable agents",
        ),
        "runtime_hardening_filesystem_denied" => (
            context.runtime_hardening_filesystem_denied,
            "one or more hardening profiles allow filesystem access",
        ),
        "runtime_hardening_env_denied" => (
            context.runtime_hardening_env_denied,
            "one or more hardening profiles allow environment access",
        ),
        "runtime_hardening_secret_material_denied" => (
            context.runtime_hardening_secret_material_denied,
            "one or more hardening profiles allow secret material",
        ),
        "runtime_hardening_key_material_denied" => (
            context.runtime_hardening_key_material_denied,
            "one or more hardening profiles allow key material",
        ),
        "runtime_hardening_audit_log_required" => (
            context.runtime_hardening_audit_log_required,
            "one or more hardening profiles do not require audit logs",
        ),
        "runtime_hardening_security_claims_absent" => (
            context.runtime_hardening_security_claims_absent,
            "one or more hardening profiles declare security claims",
        ),
        "threat_models_declared" => (
            context.threat_models_declared,
            "no threat model is declared",
        ),
        "threat_models_hardening_bound" => (
            context.threat_models_hardening_bound,
            "one or more threat models lack hardening bindings",
        ),
        "threat_models_assets_mapped" => (
            context.threat_models_assets_mapped,
            "one or more threat models lack assets",
        ),
        "threat_models_threats_mapped" => (
            context.threat_models_threats_mapped,
            "one or more threat models lack threats",
        ),
        "threat_models_mitigations_mapped" => (
            context.threat_models_mitigations_mapped,
            "one or more threat models lack mitigations",
        ),
        "threat_models_runtime_disabled" => (
            context.threat_models_runtime_disabled,
            "one or more threat models enable runtime capabilities",
        ),
        "threat_models_network_denied" => (
            context.threat_models_network_denied,
            "one or more threat models allow network access",
        ),
        "threat_models_secret_material_denied" => (
            context.threat_models_secret_material_denied,
            "one or more threat models allow secret material",
        ),
        "threat_models_key_material_denied" => (
            context.threat_models_key_material_denied,
            "one or more threat models allow key material",
        ),
        "threat_models_execution_disabled" => (
            context.threat_models_execution_disabled,
            "one or more threat models enable execution",
        ),
        "threat_models_security_claims_absent" => (
            context.threat_models_security_claims_absent,
            "one or more threat models declare security claims",
        ),
        "spec_freezes_declared" => (context.spec_freezes_declared, "no spec freeze is declared"),
        "spec_freeze_versions_pinned" => (
            context.spec_freeze_versions_pinned,
            "spec freeze versions are not pinned",
        ),
        "spec_freeze_features_declared" => (
            context.spec_freeze_features_declared,
            "spec freeze features are not declared",
        ),
        "spec_freeze_compatibility_declared" => (
            context.spec_freeze_compatibility_declared,
            "spec freeze compatibility is incomplete",
        ),
        "spec_freeze_required_suites_declared" => (
            context.spec_freeze_required_suites_declared,
            "spec freeze required suites are incomplete",
        ),
        "spec_freeze_runtime_disabled" => (
            context.spec_freeze_runtime_disabled,
            "spec freeze enables runtime",
        ),
        "spec_freeze_network_denied" => (
            context.spec_freeze_network_denied,
            "spec freeze allows network",
        ),
        "spec_freeze_external_execution_disabled" => (
            context.spec_freeze_external_execution_disabled,
            "spec freeze enables external execution",
        ),
        "spec_freeze_provider_execution_disabled" => (
            context.spec_freeze_provider_execution_disabled,
            "spec freeze enables provider execution",
        ),
        "spec_freeze_secret_material_denied" => (
            context.spec_freeze_secret_material_denied,
            "spec freeze allows secret material",
        ),
        "spec_freeze_key_material_denied" => (
            context.spec_freeze_key_material_denied,
            "spec freeze allows key material",
        ),
        "spec_freeze_env_denied" => (
            context.spec_freeze_env_denied,
            "spec freeze allows environment access",
        ),
        "spec_freeze_filesystem_denied" => (
            context.spec_freeze_filesystem_denied,
            "spec freeze allows filesystem access",
        ),
        "spec_freeze_security_claims_absent" => (
            context.spec_freeze_security_claims_absent,
            "spec freeze declares security claims",
        ),
        "spec_freeze_legal_claims_absent" => (
            context.spec_freeze_legal_claims_absent,
            "spec freeze declares legal claims",
        ),
        "spec_freeze_certification_absent" => (
            context.spec_freeze_certification_absent,
            "spec freeze declares certification",
        ),
        "release_candidates_declared" => (
            context.release_candidates_declared,
            "no release candidate is declared",
        ),
        "release_candidates_spec_freeze_bound" => (
            context.release_candidates_spec_freeze_bound,
            "release candidate lacks spec freeze binding",
        ),
        "release_candidates_artifacts_declared" => (
            context.release_candidates_artifacts_declared,
            "release candidate artifacts are missing",
        ),
        "release_candidates_checks_declared" => (
            context.release_candidates_checks_declared,
            "release candidate checks are missing",
        ),
        "release_candidates_compatibility_matrix_declared" => (
            context.release_candidates_compatibility_matrix_declared,
            "release candidate compatibility matrix is incomplete",
        ),
        "release_candidates_limitations_declared" => (
            context.release_candidates_limitations_declared,
            "release candidate limitations are missing",
        ),
        "release_candidates_runtime_disabled" => (
            context.release_candidates_runtime_disabled,
            "release candidate enables runtime",
        ),
        "release_candidates_network_denied" => (
            context.release_candidates_network_denied,
            "release candidate allows network",
        ),
        "release_candidates_external_execution_disabled" => (
            context.release_candidates_external_execution_disabled,
            "release candidate enables external execution",
        ),
        "release_candidates_provider_execution_disabled" => (
            context.release_candidates_provider_execution_disabled,
            "release candidate enables provider execution",
        ),
        "release_candidates_secret_material_denied" => (
            context.release_candidates_secret_material_denied,
            "release candidate allows secret material",
        ),
        "release_candidates_key_material_denied" => (
            context.release_candidates_key_material_denied,
            "release candidate allows key material",
        ),
        "release_candidates_env_denied" => (
            context.release_candidates_env_denied,
            "release candidate allows environment access",
        ),
        "release_candidates_filesystem_denied" => (
            context.release_candidates_filesystem_denied,
            "release candidate allows filesystem access",
        ),
        "release_candidates_security_claims_absent" => (
            context.release_candidates_security_claims_absent,
            "release candidate declares security claims",
        ),
        "release_candidates_legal_claims_absent" => (
            context.release_candidates_legal_claims_absent,
            "release candidate declares legal claims",
        ),
        "release_candidates_certification_absent" => (
            context.release_candidates_certification_absent,
            "release candidate declares certification",
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
