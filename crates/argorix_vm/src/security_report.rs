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
    pub message_contracts: MessageContractSummary,
    pub agent_passports: AgentPassportSummary,
    #[serde(default)]
    pub provider_harnesses: ProviderHarnessSummary,
    #[serde(default)]
    pub feature_flags: FeatureFlagsSummary,
    #[serde(default)]
    pub secret_boundaries: SecretBoundariesSummary,
    #[serde(default)]
    pub adapters: AdapterSummary,
    #[serde(default)]
    pub adapter_profiles: AdapterProfileSummary,
    #[serde(default)]
    pub crypto_registry: CryptoRegistrySummary,
    #[serde(default)]
    pub crypto_boundaries: CryptoBoundariesSummary,
    #[serde(default)]
    pub atrust_credential_contracts: usize,
    #[serde(default)]
    pub atrust_handshakes: usize,
    #[serde(default)]
    pub trust_ledgers: usize,
    #[serde(default)]
    pub mcp_bridge_contracts: McpBridgeContractsSummary,
    #[serde(default)]
    pub a2a_bridge_contracts: A2ABridgeContractsSummary,
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

/// Summary of declared MCP Bridge Contracts (v0.31).
///
/// A declared bridge is NOT a connected bridge: no MCP server exists, no network
/// is opened, and no tools are executed. The verdict is never inflated by the
/// presence of bridge contracts. The `network`, `external_execution`,
/// `tool_execution`, and `security_claims` maps make the closed boundary
/// explicit in the evidence.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpBridgeContractsSummary {
    pub total: usize,
    pub names: Vec<String>,
    pub protocols: BTreeMap<String, usize>,
    pub directions: BTreeMap<String, usize>,
    pub network: BTreeMap<String, usize>,
    pub external_execution: BTreeMap<String, usize>,
    pub tool_execution: BTreeMap<String, usize>,
    pub security_claims: BTreeMap<String, usize>,
}

/// Summary of declared A2A Bridge Contracts (v0.31).
///
/// A declared bridge is NOT a connected bridge: no agent communication occurred,
/// no network is opened, and no agent is executed. The verdict is never inflated
/// by the presence of bridge contracts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct A2ABridgeContractsSummary {
    pub total: usize,
    pub names: Vec<String>,
    pub protocols: BTreeMap<String, usize>,
    pub directions: BTreeMap<String, usize>,
    pub network: BTreeMap<String, usize>,
    pub external_execution: BTreeMap<String, usize>,
    pub agent_execution: BTreeMap<String, usize>,
    pub security_claims: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageContractSummary {
    pub total: usize,
    pub typed: usize,
    pub untyped: usize,
    pub fields_total: usize,
}

/// Summary of declared Agent Passports (v0.19).
///
/// Passports improve traceability, declared identity, and structural evidence.
/// They are not proof of real-world safety; the verdict is not inflated by their
/// presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentPassportSummary {
    pub total: usize,
    pub linked_agents: usize,
    pub countries: Vec<String>,
    pub jurisdictions: Vec<String>,
    pub data_residency: Vec<String>,
    pub risk_levels: BTreeMap<String, usize>,
    pub attestations_total: usize,
    pub intents: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderHarnessSummary {
    pub total: usize,
    pub providers: Vec<String>,
    pub modes: BTreeMap<String, usize>,
    pub network: BTreeMap<String, usize>,
    pub secrets: BTreeMap<String, usize>,
    pub filesystem: BTreeMap<String, usize>,
    pub attestations_total: usize,
    pub input_contracts: Vec<String>,
    pub output_contracts: Vec<String>,
}

/// Summary of declared v0.21 feature flags.
///
/// Feature flags are governance metadata. Their presence does not enable real
/// provider execution; the verdict is not inflated by their declaration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureFlagsSummary {
    pub total: usize,
    pub statuses: BTreeMap<String, usize>,
    pub defaults: BTreeMap<String, usize>,
    pub requires_approval: usize,
    pub linked_providers: Vec<String>,
}

/// Summary of declared v0.21 secret boundaries.
///
/// Secret boundaries record handles and denied access only. `values_present` is
/// always false: Argorix declares boundaries, never secret material.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretBoundariesSummary {
    pub total: usize,
    pub scopes: BTreeMap<String, usize>,
    pub access: BTreeMap<String, usize>,
    pub sources: BTreeMap<String, usize>,
    pub linked_providers: Vec<String>,
    pub required_by: Vec<String>,
    pub values_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AdapterSummary {
    pub total: usize,
    pub providers: Vec<String>,
    pub features: Vec<String>,
    pub secrets: Vec<String>,
    pub harnesses: Vec<String>,
    pub kinds: BTreeMap<String, usize>,
    pub modes: BTreeMap<String, usize>,
    pub execution: BTreeMap<String, usize>,
    pub network: BTreeMap<String, usize>,
    pub secrets_access: BTreeMap<String, usize>,
    pub filesystem: BTreeMap<String, usize>,
    pub conformance_total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AdapterProfileSummary {
    pub total: usize,
    pub vendors: Vec<String>,
    pub families: BTreeMap<String, usize>,
    pub api_styles: BTreeMap<String, usize>,
    pub auth: BTreeMap<String, usize>,
    pub execution: BTreeMap<String, usize>,
    pub network: BTreeMap<String, usize>,
    pub secrets: BTreeMap<String, usize>,
    pub capabilities_total: usize,
    pub required_conformance_total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CryptoRegistrySummary {
    pub total: usize,
    pub kinds: BTreeMap<String, usize>,
    pub statuses: BTreeMap<String, usize>,
    pub strength: BTreeMap<String, usize>,
    pub purposes_total: usize,
    pub post_quantum_candidates: usize,
    pub denied: usize,
    pub key_material_present: bool,
    pub secret_material_present: bool,
    pub execution_present: bool,
}

fn adapter_profile_summary(
    profiles: &[argorix_bytecode::BytecodeAdapterProfile],
) -> AdapterProfileSummary {
    use std::collections::BTreeSet;
    let mut vendors = BTreeSet::new();
    let mut families = BTreeMap::new();
    let mut api_styles = BTreeMap::new();
    let mut auth = BTreeMap::new();
    let mut execution = BTreeMap::new();
    let mut network = BTreeMap::new();
    let mut secrets = BTreeMap::new();
    let mut caps_total = 0usize;
    let mut req_conf_total = 0usize;

    for p in profiles {
        vendors.insert(p.vendor.clone());
        *families.entry(p.family.clone()).or_insert(0) += 1;
        *api_styles.entry(p.api_style.clone()).or_insert(0) += 1;
        *auth.entry(p.auth.clone()).or_insert(0) += 1;
        *execution.entry(p.execution.clone()).or_insert(0) += 1;
        *network.entry(p.network.clone()).or_insert(0) += 1;
        *secrets.entry(p.secrets.clone()).or_insert(0) += 1;
        caps_total += p.capabilities.len();
        req_conf_total += p.required_conformance.len();
    }

    AdapterProfileSummary {
        total: profiles.len(),
        vendors: vendors.into_iter().collect(),
        families,
        api_styles,
        auth,
        execution,
        network,
        secrets,
        capabilities_total: caps_total,
        required_conformance_total: req_conf_total,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CryptoBoundariesSummary {
    pub total: usize,
    pub allowed_hashes_total: usize,
    pub allowed_signatures_total: usize,
    pub allowed_kems_total: usize,
    pub allowed_aeads_total: usize,
    pub legacy_allowed_total: usize,
    pub denied_total: usize,
    pub post_quantum_ready: usize,
    pub hybrid_allowed: usize,
    pub key_material: std::collections::BTreeMap<String, usize>,
    pub secret_material: std::collections::BTreeMap<String, usize>,
    pub execution: std::collections::BTreeMap<String, usize>,
    pub security_claims: std::collections::BTreeMap<String, bool>,
}

fn crypto_boundaries_summary(
    boundaries: &[argorix_bytecode::BytecodeCryptoBoundary],
) -> CryptoBoundariesSummary {
    use std::collections::BTreeMap;
    let mut key_material = BTreeMap::new();
    let mut secret_material = BTreeMap::new();
    let mut execution = BTreeMap::new();
    let mut allowed_hashes_total = 0usize;
    let mut allowed_signatures_total = 0usize;
    let mut allowed_kems_total = 0usize;
    let mut allowed_aeads_total = 0usize;
    let mut legacy_allowed_total = 0usize;
    let mut denied_total = 0usize;
    let mut post_quantum_ready = 0usize;
    let mut hybrid_allowed = 0usize;

    for b in boundaries {
        *key_material.entry(b.key_material.clone()).or_insert(0) += 1;
        *secret_material
            .entry(b.secret_material.clone())
            .or_insert(0) += 1;
        *execution.entry(b.execution.clone()).or_insert(0) += 1;
        allowed_hashes_total += b.allowed_hashes.len();
        allowed_signatures_total += b.allowed_signatures.len();
        allowed_kems_total += b.allowed_kems.len();
        allowed_aeads_total += b.allowed_aeads.len();
        legacy_allowed_total += b.legacy_allowed.len();
        denied_total += b.denied.len();
        if b.post_quantum_ready == Some(true) {
            post_quantum_ready += 1;
        }
        if b.hybrid_allowed == Some(true) {
            hybrid_allowed += 1;
        }
    }

    CryptoBoundariesSummary {
        total: boundaries.len(),
        allowed_hashes_total,
        allowed_signatures_total,
        allowed_kems_total,
        allowed_aeads_total,
        legacy_allowed_total,
        denied_total,
        post_quantum_ready,
        hybrid_allowed,
        key_material,
        secret_material,
        execution,
        security_claims: {
            let mut m = BTreeMap::new();
            m.insert("post_quantum_secure".to_string(), false);
            m
        },
    }
}

fn crypto_registry_summary(cryptos: &[argorix_bytecode::BytecodeCrypto]) -> CryptoRegistrySummary {
    let mut kinds = BTreeMap::new();
    let mut statuses = BTreeMap::new();
    let mut strength = BTreeMap::new();
    let mut purposes_total = 0usize;
    let mut post_quantum_candidates = 0usize;
    let mut denied = 0usize;

    for c in cryptos {
        *kinds.entry(c.kind.clone()).or_insert(0) += 1;
        *statuses.entry(c.status.clone()).or_insert(0) += 1;
        *strength.entry(c.strength.clone()).or_insert(0) += 1;
        purposes_total += c.purpose.len();
        if c.status == "post_quantum_candidate" || c.strength == "post_quantum" {
            post_quantum_candidates += 1;
        }
        if c.status == "denied" {
            denied += 1;
        }
    }

    CryptoRegistrySummary {
        total: cryptos.len(),
        kinds,
        statuses,
        strength,
        purposes_total,
        post_quantum_candidates,
        denied,
        key_material_present: false,
        secret_material_present: false,
        execution_present: false,
    }
}

fn adapter_summary(adapters: &[argorix_bytecode::BytecodeAdapter]) -> AdapterSummary {
    use std::collections::BTreeSet;
    let mut kinds = BTreeMap::new();
    let mut modes = BTreeMap::new();
    let mut execution = BTreeMap::new();
    let mut network = BTreeMap::new();
    let mut secrets_access = BTreeMap::new();
    let mut filesystem = BTreeMap::new();
    let mut providers = BTreeSet::new();
    let mut features = BTreeSet::new();
    let mut secrets = BTreeSet::new();
    let mut harnesses = BTreeSet::new();
    let mut conformance_total = 0usize;

    for a in adapters {
        *kinds
            .entry(a.kind.clone().unwrap_or_else(|| "unspecified".into()))
            .or_insert(0) += 1;
        *modes.entry(a.mode.clone()).or_insert(0) += 1;
        *execution.entry(a.execution.clone()).or_insert(0) += 1;
        *network.entry(a.network.clone()).or_insert(0) += 1;
        *secrets_access.entry(a.secrets.clone()).or_insert(0) += 1;
        *filesystem.entry(a.filesystem.clone()).or_insert(0) += 1;
        providers.insert(a.provider.clone());
        if let Some(f) = &a.feature {
            features.insert(f.clone());
        }
        if let Some(s) = &a.secret {
            secrets.insert(s.clone());
        }
        if let Some(h) = &a.harness {
            harnesses.insert(h.clone());
        }
        conformance_total += a.conformance.len();
    }

    AdapterSummary {
        total: adapters.len(),
        providers: providers.into_iter().collect(),
        features: features.into_iter().collect(),
        secrets: secrets.into_iter().collect(),
        harnesses: harnesses.into_iter().collect(),
        kinds,
        modes,
        execution,
        network,
        secrets_access,
        filesystem,
        conformance_total,
    }
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
            report_version: "0.31".into(),
            language: bytecode.language.clone(),
            module: bytecode.module.clone(),
            modules: bytecode.modules.clone(),
            imports: bytecode.imports.clone(),
            bytecode_version: bytecode.bytecode_version.clone(),
            vm_version: trace
                .map(|trace| trace.vm_version.clone())
                .unwrap_or_else(|| "0.31".into()),
            execution,
            message_contracts: message_contract_summary(&bytecode.types),
            agent_passports: agent_passport_summary(&bytecode.passports),
            provider_harnesses: provider_harness_summary(&bytecode.provider_harnesses),
            feature_flags: feature_flags_summary(&bytecode.features),
            secret_boundaries: secret_boundaries_summary(&bytecode.secrets),
            adapters: adapter_summary(&bytecode.adapters),
            adapter_profiles: adapter_profile_summary(&bytecode.adapter_profiles),
            crypto_registry: crypto_registry_summary(&bytecode.cryptos),
            crypto_boundaries: crypto_boundaries_summary(&bytecode.crypto_boundaries),
            atrust_credential_contracts: bytecode.atrust_credential_contracts.len(),
            atrust_handshakes: bytecode.atrust_handshakes.len(),
            trust_ledgers: bytecode.trust_ledgers.len(),
            mcp_bridge_contracts: mcp_bridge_contracts_summary(&bytecode.mcp_bridge_contracts),
            a2a_bridge_contracts: a2a_bridge_contracts_summary(&bytecode.a2a_bridge_contracts),
            policy,
            provider_boundary,
            calls,
            intrinsics,
            ledger,
            verdict,
        }
    }
}

fn mcp_bridge_contracts_summary(
    contracts: &[argorix_bytecode::BytecodeMcpBridgeContract],
) -> McpBridgeContractsSummary {
    let mut summary = McpBridgeContractsSummary {
        total: contracts.len(),
        ..Default::default()
    };
    for c in contracts {
        summary.names.push(c.name.clone());
        *summary.protocols.entry(c.protocol.clone()).or_insert(0) += 1;
        *summary.directions.entry(c.direction.clone()).or_insert(0) += 1;
        *summary.network.entry(c.network.clone()).or_insert(0) += 1;
        *summary
            .external_execution
            .entry(c.external_execution.clone())
            .or_insert(0) += 1;
        *summary
            .tool_execution
            .entry(c.tool_execution.clone())
            .or_insert(0) += 1;
        *summary
            .security_claims
            .entry(c.security_claims.clone())
            .or_insert(0) += 1;
    }
    summary
}

fn a2a_bridge_contracts_summary(
    contracts: &[argorix_bytecode::BytecodeA2ABridgeContract],
) -> A2ABridgeContractsSummary {
    let mut summary = A2ABridgeContractsSummary {
        total: contracts.len(),
        ..Default::default()
    };
    for c in contracts {
        summary.names.push(c.name.clone());
        *summary.protocols.entry(c.protocol.clone()).or_insert(0) += 1;
        *summary.directions.entry(c.direction.clone()).or_insert(0) += 1;
        *summary.network.entry(c.network.clone()).or_insert(0) += 1;
        *summary
            .external_execution
            .entry(c.external_execution.clone())
            .or_insert(0) += 1;
        *summary
            .agent_execution
            .entry(c.agent_execution.clone())
            .or_insert(0) += 1;
        *summary
            .security_claims
            .entry(c.security_claims.clone())
            .or_insert(0) += 1;
    }
    summary
}

fn provider_harness_summary(
    harnesses: &[argorix_bytecode::BytecodeProviderHarness],
) -> ProviderHarnessSummary {
    use std::collections::BTreeSet;
    let mut providers = BTreeSet::new();
    let mut modes = BTreeMap::new();
    let mut network = BTreeMap::new();
    let mut secrets = BTreeMap::new();
    let mut filesystem = BTreeMap::new();
    let mut input_contracts = BTreeSet::new();
    let mut output_contracts = BTreeSet::new();
    let mut attestations_total = 0;
    for harness in harnesses {
        providers.insert(harness.provider.clone());
        *modes.entry(harness.mode.clone()).or_insert(0) += 1;
        *network.entry(harness.network.clone()).or_insert(0) += 1;
        *secrets.entry(harness.secrets.clone()).or_insert(0) += 1;
        *filesystem.entry(harness.filesystem.clone()).or_insert(0) += 1;
        if let Some(contract) = &harness.input_contract {
            input_contracts.insert(contract.clone());
        }
        if let Some(contract) = &harness.output_contract {
            output_contracts.insert(contract.clone());
        }
        attestations_total += harness.attestations.len();
    }
    ProviderHarnessSummary {
        total: harnesses.len(),
        providers: providers.into_iter().collect(),
        modes,
        network,
        secrets,
        filesystem,
        attestations_total,
        input_contracts: input_contracts.into_iter().collect(),
        output_contracts: output_contracts.into_iter().collect(),
    }
}

fn feature_flags_summary(features: &[argorix_bytecode::BytecodeFeature]) -> FeatureFlagsSummary {
    use std::collections::BTreeSet;
    let mut statuses = BTreeMap::new();
    let mut defaults = BTreeMap::new();
    let mut linked_providers = BTreeSet::new();
    let mut requires_approval = 0;
    for feature in features {
        *statuses.entry(feature.status.clone()).or_insert(0) += 1;
        *defaults.entry(feature.default.clone()).or_insert(0) += 1;
        if feature.requires_approval {
            requires_approval += 1;
        }
        if let Some(provider) = &feature.provider {
            linked_providers.insert(provider.clone());
        }
    }
    FeatureFlagsSummary {
        total: features.len(),
        statuses,
        defaults,
        requires_approval,
        linked_providers: linked_providers.into_iter().collect(),
    }
}

fn secret_boundaries_summary(
    secrets: &[argorix_bytecode::BytecodeSecret],
) -> SecretBoundariesSummary {
    use std::collections::BTreeSet;
    let mut scopes = BTreeMap::new();
    let mut access = BTreeMap::new();
    let mut sources = BTreeMap::new();
    let mut linked_providers = BTreeSet::new();
    let mut required_by = BTreeSet::new();
    for secret in secrets {
        *scopes.entry(secret.scope.clone()).or_insert(0) += 1;
        *access.entry(secret.access.clone()).or_insert(0) += 1;
        *sources.entry(secret.source.clone()).or_insert(0) += 1;
        if let Some(provider) = &secret.provider {
            linked_providers.insert(provider.clone());
        }
        if let Some(feature) = &secret.required_by {
            required_by.insert(feature.clone());
        }
    }
    SecretBoundariesSummary {
        total: secrets.len(),
        scopes,
        access,
        sources,
        linked_providers: linked_providers.into_iter().collect(),
        required_by: required_by.into_iter().collect(),
        // Argorix v0.21 declares secret boundaries, never secret material.
        values_present: false,
    }
}

fn message_contract_summary(
    contracts: &[argorix_bytecode::BytecodeType],
) -> MessageContractSummary {
    let typed = contracts
        .iter()
        .filter(|contract| {
            !contract.fields.is_empty()
                && contract.fields.iter().all(|field| {
                    matches!(
                        field.field_type.as_str(),
                        "string" | "bool" | "int" | "float"
                    )
                })
        })
        .count();
    MessageContractSummary {
        total: contracts.len(),
        typed,
        untyped: contracts.len() - typed,
        fields_total: contracts.iter().map(|contract| contract.fields.len()).sum(),
    }
}

fn agent_passport_summary(
    passports: &[argorix_bytecode::BytecodePassport],
) -> AgentPassportSummary {
    use std::collections::BTreeSet;
    let mut countries = BTreeSet::new();
    let mut jurisdictions = BTreeSet::new();
    let mut data_residency = BTreeSet::new();
    let mut intents = BTreeSet::new();
    let mut linked_agents = BTreeSet::new();
    let mut risk_levels = BTreeMap::new();
    let mut attestations_total = 0;
    for passport in passports {
        if !passport.country.trim().is_empty() {
            countries.insert(passport.country.clone());
        }
        if !passport.jurisdiction.trim().is_empty() {
            jurisdictions.insert(passport.jurisdiction.clone());
        }
        for entry in &passport.data_residency {
            data_residency.insert(entry.clone());
        }
        if !passport.intent.trim().is_empty() {
            intents.insert(passport.intent.clone());
        }
        if !passport.agent.trim().is_empty() {
            linked_agents.insert(passport.agent.clone());
        }
        if !passport.risk_level.trim().is_empty() {
            *risk_levels.entry(passport.risk_level.clone()).or_insert(0) += 1;
        }
        attestations_total += passport.attestations.len();
    }
    AgentPassportSummary {
        total: passports.len(),
        linked_agents: linked_agents.len(),
        countries: countries.into_iter().collect(),
        jurisdictions: jurisdictions.into_iter().collect(),
        data_residency: data_residency.into_iter().collect(),
        risk_levels,
        attestations_total,
        intents: intents.into_iter().collect(),
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
    let actions = events
        .iter()
        .filter(|event| event.event_type == EventType::PolicyActionActivated)
        .filter_map(|event| parse_policy_action(&event.details))
        .collect::<Vec<_>>();
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
        policy_blocks_total: count_event(events, EventType::PolicyDeclared),
        policy_blocks_passed: 0,
        policy_blocks_failed: count_event(events, EventType::PolicyViolation),
        require_rules_total: 0,
        deny_rules_total: 0,
        violations: Vec::new(),
        review_required: actions.iter().any(|action| action.action == "review"),
        warning: actions.iter().any(|action| action.action == "warn"),
        actions,
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
    if policy.actions.iter().any(|action| action.action == "block") {
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

fn parse_policy_action(details: &str) -> Option<crate::PolicyActionResult> {
    let words = details.split_whitespace().collect::<Vec<_>>();
    if words.len() < 6 || words.first().copied() != Some("policy") {
        return None;
    }
    let trace_required = words
        .iter()
        .find_map(|word| word.strip_prefix("trace_required="))
        == Some("true");
    Some(crate::PolicyActionResult {
        policy: words[1].to_owned(),
        action: words[3].to_owned(),
        trace_required,
    })
}
