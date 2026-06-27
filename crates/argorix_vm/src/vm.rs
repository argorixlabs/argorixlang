use crate::policy::{evaluate_rule, PolicyEvidenceContext};
use crate::{
    AgentStateSummary, AssertionResult, EventFields, EventType, ExecutionTrace, FailureActivation,
    InjectedMessage, IntrinsicExecution, MailboxSummary, PolicyActionResult, PolicyBlockResult,
    PolicyReport, PolicyRuleResult, PolicyViolation, ReactiveExecutionTrace, ReactiveScheduler,
    RuntimeState, RuntimeStatus, Scheduler, VmError,
};
use argorix_bytecode::{verify_bytecode, BytecodeError, BytecodeProgram};
use argorix_provider::{AdapterContract, ProviderKind, ProviderRegistry};
use serde::{Deserialize, Serialize};

pub struct Vm {
    providers: ProviderRegistry,
}

#[derive(Debug)]
pub struct ExecutionOutcome {
    pub state: RuntimeState,
    pub result: Result<ReactiveExecutionTrace, VmError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeExecutionRequest {
    pub runtime: String,
    pub adapter: Option<String>,
    pub operation: Option<String>,
    pub sandboxed_external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeExecutionResult {
    pub vm_version: String,
    pub runtime: String,
    pub mode: String,
    pub status: String,
    pub provider: String,
    pub adapter: Option<String>,
    pub operation: Option<String>,
    pub external_execution_enabled: bool,
    pub events: Vec<crate::ExecutionEvent>,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Self {
            providers: ProviderRegistry::default(),
        }
    }

    /// Execute the v1.0 governed runtime entrypoint.
    ///
    /// The default implementation never performs network I/O and never
    /// resolves endpoint or secret references. `sandboxed_external` validates
    /// and records a call plan only; an external executor is intentionally not
    /// part of the v1.0 core runtime.
    pub fn run_runtime_profile(
        &self,
        bytecode: &BytecodeProgram,
        request: RuntimeExecutionRequest,
    ) -> Result<RuntimeExecutionResult, VmError> {
        verify_bytecode(bytecode).map_err(VmError::from_verification)?;
        let profile = bytecode
            .runtime_execution_profiles
            .iter()
            .find(|profile| profile.name == request.runtime)
            .ok_or_else(|| VmError::RuntimeProfile {
                runtime: request.runtime.clone(),
                reason: "unknown runtime_execution_profile".into(),
            })?;
        let mut ledger = crate::TraceLedger::default();
        ledger.record(
            EventType::RuntimeExecutionProfileDeclared,
            "declared",
            format!(
                "runtime_execution_profile {} selected in {} mode",
                profile.name, profile.mode
            ),
            EventFields::default(),
        );
        ledger.record(
            EventType::RuntimeProfileGuardsValidated,
            "validated",
            format!(
                "runtime_execution_profile {} is policy, hardening, evidence, governance and fail-closed bound",
                profile.name
            ),
            EventFields::default(),
        );

        let mut result = RuntimeExecutionResult {
            vm_version: "1.0".into(),
            runtime: profile.name.clone(),
            mode: profile.mode.clone(),
            status: "completed".into(),
            provider: profile.provider.clone(),
            adapter: request.adapter.clone(),
            operation: request.operation.clone(),
            external_execution_enabled: false,
            events: Vec::new(),
        };
        match profile.mode.as_str() {
            "dry_run" => ledger.record(
                EventType::RuntimeDryRunCompleted,
                "completed",
                format!(
                    "runtime_execution_profile {} completed without provider execution",
                    profile.name
                ),
                EventFields::default(),
            ),
            "simulated" => {
                if profile.provider != "simulated" {
                    return Err(VmError::RuntimeProfile {
                        runtime: profile.name.clone(),
                        reason: "simulated mode requires the built-in `simulated` provider".into(),
                    });
                }
                ledger.record(
                    EventType::RuntimeSimulatedProviderExecuted,
                    "completed",
                    format!(
                        "runtime_execution_profile {} executed deterministic simulated provider",
                        profile.name
                    ),
                    EventFields::default(),
                );
            }
            "sandboxed_external" => {
                const REQUIRED_POLICY_RULES: &[&str] = &[
                    "runtime_execution_profiles_declared",
                    "runtime_profiles_provider_bound",
                    "runtime_profiles_hardening_bound",
                    "runtime_profiles_evidence_bound",
                    "runtime_profiles_governance_bound",
                    "runtime_profiles_fail_closed",
                    "runtime_profiles_external_execution_sandboxed",
                    "sandboxed_provider_adapters_declared",
                    "sandboxed_provider_adapters_provider_bound",
                    "sandboxed_provider_adapters_runtime_bound",
                    "sandboxed_provider_adapters_operations_bounded",
                    "sandboxed_provider_adapters_network_declared",
                    "sandboxed_provider_adapters_external_execution_sandboxed",
                    "sandboxed_provider_adapters_secret_refs_redacted",
                    "sandboxed_provider_adapters_fail_closed",
                ];
                let policy_rules: std::collections::HashSet<&str> = bytecode
                    .policies
                    .iter()
                    .flat_map(|policy| policy.rules.iter().map(|rule| rule.rule.as_str()))
                    .collect();
                if !REQUIRED_POLICY_RULES
                    .iter()
                    .all(|rule| policy_rules.contains(rule))
                {
                    return Err(VmError::RuntimeProfile {
                        runtime: profile.name.clone(),
                        reason: "sandboxed_external requires complete policy v2 authorization"
                            .into(),
                    });
                }
                let adapter_name =
                    request
                        .adapter
                        .as_deref()
                        .ok_or_else(|| VmError::RuntimeProfile {
                            runtime: profile.name.clone(),
                            reason: "sandboxed_external requires an explicit adapter".into(),
                        })?;
                let adapter = bytecode
                    .sandboxed_provider_adapters
                    .iter()
                    .find(|adapter| adapter.name == adapter_name)
                    .ok_or_else(|| VmError::RuntimeProfile {
                        runtime: profile.name.clone(),
                        reason: format!("unknown sandboxed_provider_adapter `{adapter_name}`"),
                    })?;
                let operation =
                    request
                        .operation
                        .as_deref()
                        .ok_or_else(|| VmError::RuntimeProfile {
                            runtime: profile.name.clone(),
                            reason: "sandboxed_external requires an explicit operation".into(),
                        })?;
                if adapter.runtime != profile.name
                    || adapter.provider != profile.provider
                    || !profile
                        .allowed_actions
                        .iter()
                        .any(|value| value == "model.generate")
                    || !adapter
                        .allowed_operations
                        .iter()
                        .any(|value| value == operation)
                    || adapter
                        .denied_operations
                        .iter()
                        .any(|value| value == operation)
                {
                    return Err(VmError::RuntimeProfile {
                        runtime: profile.name.clone(),
                        reason: "adapter/provider/operation boundary rejected".into(),
                    });
                }
                ledger.record(
                    EventType::SandboxedProviderAdapterDeclared,
                    "declared",
                    format!(
                        "sandboxed_provider_adapter {} selected for provider {}",
                        adapter.name, adapter.provider
                    ),
                    EventFields::default(),
                );
                ledger.record(
                    EventType::ProviderReferencesRedacted,
                    "redacted",
                    format!(
                        "sandboxed_provider_adapter {} retains reference names only; values are absent",
                        adapter.name
                    ),
                    EventFields::default(),
                );
                if !request.sandboxed_external {
                    result.status = "blocked".into();
                    ledger.record(
                        EventType::ExternalProviderExecutionBlocked,
                        "blocked",
                        format!(
                            "sandboxed external operation {} blocked without explicit flag",
                            operation
                        ),
                        EventFields::default(),
                    );
                } else {
                    result.status = "planned".into();
                    ledger.record(
                        EventType::SandboxedExternalCallPlanned,
                        "planned",
                        format!(
                            "sandboxed external operation {} planned; v1.0 core performs no network call",
                            operation
                        ),
                        EventFields::default(),
                    );
                }
            }
            other => {
                return Err(VmError::RuntimeProfile {
                    runtime: profile.name.clone(),
                    reason: format!("unsupported runtime mode `{other}`"),
                })
            }
        }
        result.events = ledger.events;
        Ok(result)
    }

    pub fn run_dry(&self, bytecode: &BytecodeProgram) -> Result<ExecutionTrace, VmError> {
        let mut state = RuntimeState::from_bytecode(bytecode)?;
        let steps = Scheduler::new().run(bytecode, &mut state)?;
        let mailboxes = state
            .agents
            .iter()
            .filter_map(|agent| {
                state.mailboxes.get(agent).map(|mailbox| MailboxSummary {
                    agent: agent.clone(),
                    delivered: mailbox.delivered(),
                    processed: mailbox.processed(),
                })
            })
            .collect::<Vec<_>>();

        Ok(ExecutionTrace {
            vm_version: "0.5".to_owned(),
            status: "completed".to_owned(),
            mode: "dry-run".to_owned(),
            scheduler: "deterministic".to_owned(),
            steps,
            mailboxes,
            events: state.trace_ledger.events,
            security_checks: "passed".to_owned(),
        })
    }

    pub fn initialize(&self, bytecode: &BytecodeProgram) -> Result<RuntimeState, VmError> {
        RuntimeState::from_bytecode(bytecode)
    }

    pub fn load_provider_contracts(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
    ) -> Result<ProviderRegistry, VmError> {
        let mut registry = self.providers.execution_registry();
        for provider in &bytecode.providers {
            state.trace_ledger.record(
                EventType::ProviderContractDeclared,
                "declared",
                format!("provider contract {} declared", provider.name),
                EventFields::default(),
            );
            let kind = match provider.kind.as_str() {
                "simulated" => ProviderKind::Simulated,
                "external" => ProviderKind::External,
                other => {
                    let reason = format!("unsupported provider kind `{other}`");
                    self.reject_provider_contract(state, &provider.name, &reason);
                    return Err(VmError::ProviderBoundary {
                        provider: provider.name.clone(),
                        reason,
                    });
                }
            };
            let contract = AdapterContract {
                name: provider.name.clone(),
                kind,
                enabled: provider.enabled,
                dry_run_only: provider.dry_run_only,
                requires_feature_flag: provider.requires_feature_flag,
                requires_explicit_approval: provider.requires_explicit_approval,
                allowed_targets: provider.allowed_targets.clone(),
                allowed_capabilities: provider.allowed_capabilities.clone(),
            };
            if let Err(error) = registry.register_contract(contract) {
                let reason = error.to_string();
                self.reject_provider_contract(state, &provider.name, &reason);
                return Err(VmError::ProviderBoundary {
                    provider: provider.name.clone(),
                    reason,
                });
            }
            if let Err(error) = registry.validate_contract(&provider.name) {
                let reason = error.to_string();
                self.reject_provider_contract(state, &provider.name, &reason);
                return Err(VmError::ProviderBoundary {
                    provider: provider.name.clone(),
                    reason,
                });
            }
            state.trace_ledger.record(
                EventType::ProviderContractValidated,
                "validated",
                format!("provider contract {} validated", provider.name),
                EventFields::default(),
            );
        }
        Ok(registry)
    }

    fn reject_provider_contract(&self, state: &mut RuntimeState, name: &str, reason: &str) {
        state.activate_failure("ProviderContractRejected");
        state.trace_ledger.record(
            EventType::ProviderContractRejected,
            "rejected",
            format!("provider contract {name} rejected: {reason}"),
            EventFields::default(),
        );
        state.fail(format!("provider contract {name} rejected: {reason}"));
    }

    pub fn run_reactive(
        &self,
        bytecode: &BytecodeProgram,
        injected: InjectedMessage,
    ) -> Result<ReactiveExecutionTrace, VmError> {
        let outcome = self.run_reactive_outcome(bytecode, injected);
        match outcome.result {
            Ok(trace) => {
                if let Some(action) = trace
                    .policy_report
                    .actions
                    .iter()
                    .find(|action| action.action == "block")
                {
                    Err(VmError::PolicyViolation {
                        policy: action.policy.clone(),
                    })
                } else {
                    Ok(trace)
                }
            }
            Err(error) => Err(error),
        }
    }

    pub fn run_reactive_outcome(
        &self,
        bytecode: &BytecodeProgram,
        injected: InjectedMessage,
    ) -> ExecutionOutcome {
        let mut state = RuntimeState::from_bytecode(bytecode)
            .expect("runtime state initialization is infallible for decoded bytecode");
        if let Err(errors) = verify_bytecode(bytecode) {
            if let Some(provider) = blocked_external_provider(bytecode, &errors) {
                state.trace_ledger.record(
                    EventType::ExternalProviderExecutionBlocked,
                    "blocked",
                    format!("external provider execution through {provider} blocked"),
                    EventFields::default(),
                );
                state.trace_ledger.record(
                    EventType::ProviderBoundaryDenied,
                    "denied",
                    format!("provider boundary denied external provider {provider}"),
                    EventFields::default(),
                );
                record_external_execution_policy_violations(bytecode, &mut state);
            }
            if errors.iter().any(|error| {
                matches!(
                    error,
                    BytecodeError::HarnessesRequireV020
                        | BytecodeError::DuplicateProviderHarness(_)
                        | BytecodeError::InvalidProviderHarness { .. }
                )
            }) {
                state.trace_ledger.record(
                    EventType::ProviderHarnessRejected,
                    "rejected",
                    "provider harness metadata rejected by bytecode verification",
                    EventFields::default(),
                );
            }
            let error = VmError::from_verification(errors);
            state.fail(error.to_string());
            return ExecutionOutcome {
                state,
                result: Err(error),
            };
        }
        for harness in &bytecode.provider_harnesses {
            state.trace_ledger.record(
                EventType::ProviderHarnessValidated,
                "validated",
                format!("provider harness {} validated", harness.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ProviderHarnessSandboxed,
                "sandboxed",
                format!(
                    "provider harness {} containment validated without execution",
                    harness.name
                ),
                EventFields::default(),
            );
        }
        for feature in &bytecode.features {
            state.trace_ledger.record(
                EventType::FeatureDeclared,
                "declared",
                format!("feature {} declared", feature.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::FeatureValidated,
                "validated",
                format!(
                    "feature {} validated as governance metadata without execution",
                    feature.name
                ),
                EventFields::default(),
            );
        }
        for secret in &bytecode.secrets {
            state.trace_ledger.record(
                EventType::SecretBoundaryDeclared,
                "declared",
                format!("secret boundary {} declared", secret.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::SecretBoundaryValidated,
                "validated",
                format!(
                    "secret boundary {} validated; handle is metadata, no secret value present",
                    secret.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::SecretAccessDenied,
                "denied",
                format!(
                    "secret boundary {} access denied; no env, vault, or network lookup performed",
                    secret.name
                ),
                EventFields::default(),
            );
        }
        // v0.31 MCP/A2A bridge contracts are declarative interoperability
        // surfaces. Declaring a bridge never connects it: no network is opened,
        // no MCP server starts, no tool runs, and no agent executes. We record
        // the closed boundary as evidence only.
        for c in &bytecode.mcp_bridge_contracts {
            state.trace_ledger.record(
                EventType::McpBridgeContractDeclared,
                "declared",
                format!("mcp_bridge_contract {} declared as metadata only", c.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::McpBridgeRuntimeDisabled,
                "disabled",
                format!(
                    "mcp_bridge_contract {} runtime disabled; no MCP server, no tool execution",
                    c.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::McpBridgeNetworkDenied,
                "denied",
                format!(
                    "mcp_bridge_contract {} network denied; no sockets opened",
                    c.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::McpBridgeToolExecutionDisabled,
                "disabled",
                format!("mcp_bridge_contract {} tool execution disabled", c.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::BridgeSecurityClaimsDenied,
                "none",
                format!(
                    "mcp_bridge_contract {} declares no security claims; declared != connected",
                    c.name
                ),
                EventFields::default(),
            );
        }
        for c in &bytecode.a2a_bridge_contracts {
            state.trace_ledger.record(
                EventType::A2ABridgeContractDeclared,
                "declared",
                format!("a2a_bridge_contract {} declared as metadata only", c.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::A2ABridgeRuntimeDisabled,
                "disabled",
                format!(
                    "a2a_bridge_contract {} runtime disabled; no agent messages sent",
                    c.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::A2ABridgeNetworkDenied,
                "denied",
                format!(
                    "a2a_bridge_contract {} network denied; no sockets opened",
                    c.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::A2AAgentExecutionDisabled,
                "disabled",
                format!("a2a_bridge_contract {} agent execution disabled", c.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::BridgeSecurityClaimsDenied,
                "none",
                format!(
                    "a2a_bridge_contract {} declares no security claims; declared != connected",
                    c.name
                ),
                EventFields::default(),
            );
        }
        for map in &bytecode.atrust_evidence_maps {
            state.trace_ledger.record(
                EventType::ATrustEvidenceMapDeclared,
                "declared",
                format!("atrust_evidence_map {} declared as metadata only", map.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ATrustEvidenceMapCoverageRequired,
                "required",
                format!("atrust_evidence_map {} coverage {}", map.name, map.coverage),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ATrustEvidenceMapLinksValidated,
                "validated",
                format!(
                    "atrust_evidence_map {} links identity, credential, handshake, ledger, bridges, policy and evidence metadata without verification",
                    map.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ATrustEvidenceMapRuntimeDisabled,
                "disabled",
                format!(
                    "atrust_evidence_map {} runtime disabled; no identity, credential, handshake, bridge, signature, or blockchain verification",
                    map.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ATrustEvidenceMapSecurityClaimsDenied,
                "none",
                format!(
                    "atrust_evidence_map {} declares security_claims none; mapped != verified",
                    map.name
                ),
                EventFields::default(),
            );
        }
        for profile in &bytecode.governance_profiles {
            state.trace_ledger.record(
                EventType::GovernanceProfileDeclared,
                "declared",
                format!(
                    "governance_profile {} declared as governance metadata; declaration does not establish compliance",
                    profile.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::GovernanceControlsMapped,
                "mapped",
                format!(
                    "governance_profile {} maps {} controls; mapped does not mean externally audited",
                    profile.name,
                    profile.controls.len()
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::GovernanceRuntimeDisabled,
                "disabled",
                format!(
                    "governance_profile {} runtime, network, external execution, keys and secrets disabled",
                    profile.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::GovernanceSecurityClaimsDenied,
                "none",
                format!(
                    "governance_profile {} security_claims none; risk level declared does not mean risk eliminated",
                    profile.name
                ),
                EventFields::default(),
            );
        }
        for mapping in &bytecode.regulatory_mappings {
            state.trace_ledger.record(
                EventType::RegulatoryMappingDeclared,
                "declared",
                format!(
                    "regulatory_mapping {} declared as an audit aid, not legal advice",
                    mapping.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::RegulatoryObligationsMapped,
                "mapped",
                format!(
                    "regulatory_mapping {} maps {} obligations; mapped does not mean legally satisfied",
                    mapping.name,
                    mapping.obligations.len()
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::LegalCertificationDenied,
                "none",
                format!(
                    "regulatory_mapping {} legal_claims none and certification none; no regulator approval",
                    mapping.name
                ),
                EventFields::default(),
            );
        }
        for verifier in &bytecode.third_party_verifiers {
            state.trace_ledger.record(
                EventType::ThirdPartyVerifierDeclared,
                "declared",
                format!(
                    "third_party_verifier {} declared as review metadata; identity and independence are not externally verified",
                    verifier.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::PublicConformanceRuntimeDisabled,
                "disabled",
                format!(
                    "third_party_verifier {} has network and external execution disabled",
                    verifier.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::PublicConformanceSecurityClaimsDenied,
                "none",
                format!(
                    "third_party_verifier {} has legal, certification and security claims denied",
                    verifier.name
                ),
                EventFields::default(),
            );
        }
        for report in &bytecode.public_conformance_reports {
            state.trace_ledger.record(
                EventType::PublicConformanceReportDeclared,
                "declared",
                format!(
                    "public_conformance_report {} declared with result {}; passed is not certification",
                    report.name, report.result
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::PublicConformanceArtifactsMapped,
                "mapped",
                format!(
                    "public_conformance_report {} maps suite, source, bytecode, evidence, governance, regulatory and ledger artifacts",
                    report.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::PublicConformanceReplayDeclared,
                "declared",
                format!(
                    "public_conformance_report {} reproducibility {}; declaration is not remote verification",
                    report.name, report.reproducibility
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::PublicConformanceRuntimeDisabled,
                "disabled",
                format!(
                    "public_conformance_report {} cannot execute, publish remotely, or call an auditor",
                    report.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::PublicConformanceSecurityClaimsDenied,
                "none",
                format!(
                    "public_conformance_report {} has security_claims none",
                    report.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::LegalCertificationDenied,
                "none",
                format!(
                    "public_conformance_report {} is not legal certification or regulator approval",
                    report.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::RemoteVerificationDenied,
                "denied",
                format!(
                    "public_conformance_report {} performs no remote audit or attestation",
                    report.name
                ),
                EventFields::default(),
            );
        }
        for profile in &bytecode.runtime_hardening_profiles {
            state.trace_ledger.record(
                EventType::RuntimeHardeningProfileDeclared,
                "declared",
                format!(
                    "runtime_hardening_profile {} declared as offline, fail-closed metadata",
                    profile.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::RuntimeDenyByDefaultDeclared,
                "required",
                format!(
                    "runtime_hardening_profile {} denies by default with enforcement {}",
                    profile.name, profile.enforcement
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::RuntimeSandboxRequired,
                "required",
                format!(
                    "runtime_hardening_profile {} requires sandbox {}",
                    profile.name, profile.sandbox
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::RuntimeNetworkDenied,
                "denied",
                format!(
                    "runtime_hardening_profile {} network {}",
                    profile.name, profile.network
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::RuntimeSecretsDenied,
                "denied",
                format!(
                    "runtime_hardening_profile {} secret material {} and environment access {}",
                    profile.name, profile.secret_material, profile.env_access
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::RuntimeExecutionDisabled,
                "disabled",
                format!(
                    "runtime_hardening_profile {} external execution, tools, agents, providers, MCP and A2A remain disabled",
                    profile.name
                ),
                EventFields::default(),
            );
        }
        for model in &bytecode.threat_models {
            state.trace_ledger.record(
                EventType::ThreatModelDeclared,
                "declared",
                format!(
                    "threat_model {} declared for hardening profile {}",
                    model.name, model.hardening_profile
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ThreatAssetsMapped,
                "mapped",
                format!(
                    "threat_model {} maps {} assets",
                    model.name,
                    model.assets.len()
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ThreatsMapped,
                "mapped",
                format!(
                    "threat_model {} maps {} threats",
                    model.name,
                    model.threats.len()
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::MitigationsMapped,
                "mapped",
                format!(
                    "threat_model {} maps {} mitigations",
                    model.name,
                    model.mitigations.len()
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ThreatModelRuntimeDisabled,
                "disabled",
                format!(
                    "threat_model {} performs no attack simulation, exploit execution, network access, or external verification",
                    model.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ThreatModelSecurityClaimsDenied,
                "none",
                format!(
                    "threat_model {} is declarative and does not certify security or eliminate risk",
                    model.name
                ),
                EventFields::default(),
            );
        }
        for freeze in &bytecode.spec_freezes {
            state.trace_ledger.record(
                EventType::SpecFreezeDeclared,
                "declared",
                format!(
                    "spec_freeze {} pins version {}",
                    freeze.name, freeze.version
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::SpecFreezeCompatibilityDeclared,
                "declared",
                format!(
                    "spec_freeze {} declares {} compatible versions and {} required suites",
                    freeze.name,
                    freeze.compatible_versions.len(),
                    freeze.required_suites.len()
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::SpecFreezeRuntimeDisabled,
                "disabled",
                format!(
                    "spec_freeze {} does not enable runtime, network, providers, tools, agents, secrets, keys, MCP or A2A",
                    freeze.name
                ),
                EventFields::default(),
            );
        }
        for candidate in &bytecode.release_candidates {
            state.trace_ledger.record(
                EventType::ReleaseCandidateDeclared,
                "declared",
                format!(
                    "release_candidate {} declares readiness {} without production certification",
                    candidate.name, candidate.readiness
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ReleaseCandidateArtifactsMapped,
                "mapped",
                format!(
                    "release_candidate {} maps {} artifacts and {} checks",
                    candidate.name,
                    candidate.required_artifacts.len(),
                    candidate.required_checks.len()
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ReleaseCandidateCompatibilityMapped,
                "mapped",
                format!(
                    "release_candidate {} maps {} compatibility rows",
                    candidate.name,
                    candidate.compatibility_matrix.len()
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ReleaseCandidateRuntimeDisabled,
                "disabled",
                format!(
                    "release_candidate {} keeps runtime and all external execution disabled",
                    candidate.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ReleaseCandidateSecurityClaimsDenied,
                "none",
                format!(
                    "release_candidate {} is not production, legal, compliance, or security certification",
                    candidate.name
                ),
                EventFields::default(),
            );
        }
        for profile in &bytecode.runtime_execution_profiles {
            state.trace_ledger.record(
                EventType::RuntimeExecutionProfileDeclared,
                "declared",
                format!(
                    "runtime_execution_profile {} declares governed mode {}",
                    profile.name, profile.mode
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::RuntimeProfileGuardsValidated,
                "validated",
                format!(
                    "runtime_execution_profile {} is hardening, evidence, governance, policy and fail-closed bound",
                    profile.name
                ),
                EventFields::default(),
            );
            if profile.mode == "sandboxed_external" {
                state.trace_ledger.record(
                    EventType::ExternalProviderExecutionBlocked,
                    "blocked",
                    format!(
                        "runtime_execution_profile {} external execution blocked without explicit sandboxed flag",
                        profile.name
                    ),
                    EventFields::default(),
                );
            }
        }
        for adapter in &bytecode.sandboxed_provider_adapters {
            state.trace_ledger.record(
                EventType::SandboxedProviderAdapterDeclared,
                "declared",
                format!(
                    "sandboxed_provider_adapter {} declared for provider {}",
                    adapter.name, adapter.provider
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ProviderReferencesRedacted,
                "redacted",
                format!(
                    "sandboxed_provider_adapter {} retains reference names only; endpoint and secret values are absent",
                    adapter.name
                ),
                EventFields::default(),
            );
        }
        let execution_providers = match self.load_provider_contracts(bytecode, &mut state) {
            Ok(providers) => providers,
            Err(error) => {
                return ExecutionOutcome {
                    state,
                    result: Err(error),
                };
            }
        };
        for (name, kind) in execution_providers.entries() {
            state.trace_ledger.record(
                EventType::ProviderRegistered,
                "registered",
                format!("provider {name} registered as {kind:?}"),
                EventFields::default(),
            );
        }
        state.executable_providers = execution_providers
            .entries()
            .into_iter()
            .map(|(name, kind)| crate::ProviderSummary {
                name: name.to_owned(),
                kind: match kind {
                    ProviderKind::Simulated => "simulated",
                    ProviderKind::External => "external",
                }
                .into(),
                enabled: true,
            })
            .collect();
        state.provider_contracts = execution_providers
            .contract_entries()
            .into_iter()
            .map(|contract| crate::ProviderContractSummary {
                name: contract.name.clone(),
                kind: match contract.kind {
                    ProviderKind::Simulated => "simulated",
                    ProviderKind::External => "external",
                }
                .into(),
                enabled: contract.enabled,
                dry_run_only: contract.dry_run_only,
                requires_feature_flag: contract.requires_feature_flag,
                requires_explicit_approval: contract.requires_explicit_approval,
                allowed_targets: contract.allowed_targets.clone(),
                allowed_capabilities: contract.allowed_capabilities.clone(),
            })
            .collect();
        let steps = match ReactiveScheduler::new().run_with_registry(
            bytecode,
            &mut state,
            &injected,
            &execution_providers,
        ) {
            Ok(steps) => steps,
            Err(error) => {
                return ExecutionOutcome {
                    state,
                    result: Err(error),
                };
            }
        };
        let policy_report = self.evaluate_policy(bytecode, &mut state, &steps);
        let mailboxes = state
            .agents
            .iter()
            .filter_map(|agent| {
                state.mailboxes.get(agent).map(|mailbox| MailboxSummary {
                    agent: agent.clone(),
                    delivered: mailbox.delivered(),
                    processed: mailbox.processed(),
                })
            })
            .collect();
        let agent_state = state
            .agents
            .iter()
            .filter_map(|agent| state.agent_state.get(agent))
            .map(|agent_state| AgentStateSummary {
                agent: agent_state.agent.clone(),
                handled_count: agent_state.handled_count,
                checkpoints: agent_state.checkpoints.len(),
                last_message_type: agent_state.last_message_type.clone(),
            })
            .collect();
        let intrinsics = steps
            .iter()
            .flat_map(|step| {
                step.intrinsics.iter().map(|intrinsic| IntrinsicExecution {
                    agent: step.agent.clone(),
                    name: intrinsic.name.clone(),
                    argument: intrinsic.argument.clone(),
                    status: "ok".into(),
                })
            })
            .collect();
        let tool_calls = state
            .tool_calls
            .iter()
            .map(|call| crate::ToolCallSummary {
                agent: call.agent.clone(),
                tool: call.tool.clone(),
                capability: call.capability.clone(),
                status: call.status.clone(),
                mode: "dry-run".into(),
            })
            .collect();
        let model_calls = state
            .model_calls
            .iter()
            .map(|call| crate::ModelCallSummary {
                agent: call.agent.clone(),
                model: call.model.clone(),
                provider: call.provider.clone(),
                capability: call.capability.clone(),
                status: call.status.clone(),
                mode: "dry-run".into(),
            })
            .collect();
        let providers = state.executable_providers.clone();
        let provider_contracts = state.provider_contracts.clone();
        let provider_calls = state.provider_calls.clone();
        let trace = ReactiveExecutionTrace {
            vm_version: "1.0".into(),
            status: match state.status {
                RuntimeStatus::Completed => "completed",
                RuntimeStatus::Failed => "failed",
                RuntimeStatus::Initialized => "initialized",
                RuntimeStatus::Running => "running",
            }
            .into(),
            mode: "reactive-dry-run".into(),
            scheduler: "deterministic".into(),
            modules: bytecode.modules.clone(),
            imports: bytecode.imports.clone(),
            message_contracts: bytecode.types.clone(),
            passports: bytecode.passports.clone(),
            provider_harnesses: bytecode.provider_harnesses.clone(),
            features: bytecode.features.clone(),
            secrets: bytecode.secrets.clone(),
            adapters: bytecode.adapters.clone(),
            adapter_profiles: bytecode.adapter_profiles.clone(),
            cryptos: bytecode.cryptos.clone(),
            atrust_evidence_maps: bytecode.atrust_evidence_maps.clone(),
            governance_profiles: bytecode.governance_profiles.clone(),
            regulatory_mappings: bytecode.regulatory_mappings.clone(),
            third_party_verifiers: bytecode.third_party_verifiers.clone(),
            public_conformance_reports: bytecode.public_conformance_reports.clone(),
            runtime_hardening_profiles: bytecode.runtime_hardening_profiles.clone(),
            threat_models: bytecode.threat_models.clone(),
            spec_freezes: bytecode.spec_freezes.clone(),
            release_candidates: bytecode.release_candidates.clone(),
            runtime_execution_profiles: bytecode.runtime_execution_profiles.clone(),
            sandboxed_provider_adapters: bytecode.sandboxed_provider_adapters.clone(),
            injected,
            steps,
            mailboxes,
            agent_state,
            intrinsics,
            tool_calls,
            model_calls,
            providers,
            provider_contracts,
            provider_calls,
            policy_report,
            events: state.trace_ledger.events.clone(),
            security_checks: "passed".into(),
        };
        ExecutionOutcome {
            state,
            result: Ok(trace),
        }
    }

    fn evaluate_policy(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
        steps: &[crate::ReactiveStep],
    ) -> PolicyReport {
        let evidence_context = policy_evidence_context(bytecode);
        let mut results = Vec::new();
        for assertion in &bytecode.assertions {
            let name = if assertion.name == "runtime_status" {
                "runtime_status completed"
            } else {
                assertion.name.as_str()
            };
            let mut evaluation = evaluate_rule(name, state, steps, evidence_context);
            if assertion.name == "runtime_status"
                && assertion.argument.as_deref() != Some("completed")
            {
                evaluation.passed = false;
                evaluation.reason = "runtime status assertion requires completed";
            }
            let passed = evaluation.passed;
            state.trace_ledger.record(
                if passed {
                    EventType::AssertionVerified
                } else {
                    EventType::AssertionFailed
                },
                if passed { "passed" } else { "failed" },
                format!("assertion {} evaluated", assertion.name),
                EventFields::default(),
            );
            results.push(AssertionResult {
                name: assertion.name.clone(),
                argument: assertion.argument.clone(),
                status: if passed { "passed" } else { "failed" }.into(),
                reason: (!passed).then(|| evaluation.reason.to_owned()),
            });
        }
        let legacy_failed = results.iter().any(|result| result.status == "failed");
        let mut failures = Vec::new();
        if legacy_failed {
            state.status = RuntimeStatus::Failed;
            let selected = bytecode
                .failures
                .iter()
                .find(|failure| failure.name == "PolicyViolation")
                .cloned()
                .unwrap_or(argorix_bytecode::BytecodeFailure {
                    name: "InternalPolicyViolation".into(),
                    action: "block".into(),
                    trace: "required".into(),
                });
            state.trace_ledger.record(
                EventType::FailureModeActivated,
                "active",
                format!("failure mode {} activated", selected.name),
                EventFields::default(),
            );
            failures.push(FailureActivation {
                name: selected.name,
                action: selected.action,
                trace: selected.trace,
            });
        }
        let mut policy_blocks = Vec::new();
        let mut actions = Vec::new();
        for policy in &bytecode.policies {
            let mut require_rules = Vec::new();
            let mut deny_rules = Vec::new();
            let mut violations = Vec::new();
            for declaration in &policy.rules {
                let evaluation = evaluate_rule(&declaration.rule, state, steps, evidence_context);
                let passed = if declaration.effect == "deny" {
                    !evaluation.passed
                } else {
                    evaluation.passed
                };
                let reason = (!passed).then(|| {
                    if declaration.effect == "deny" && declaration.rule == "external_execution" {
                        "external provider execution was attempted".to_owned()
                    } else if declaration.effect == "deny" {
                        format!("denied condition `{}` occurred", declaration.rule)
                    } else {
                        evaluation.reason.to_owned()
                    }
                });
                state.trace_ledger.record(
                    EventType::PolicyEvaluated,
                    if passed { "passed" } else { "failed" },
                    format!(
                        "policy {} {} {} evaluated",
                        policy.name, declaration.effect, declaration.rule
                    ),
                    EventFields::default(),
                );
                let result = PolicyRuleResult {
                    rule: declaration.rule.clone(),
                    effect: declaration.effect.clone(),
                    passed,
                    reason: reason.clone(),
                };
                if declaration.effect == "require" {
                    require_rules.push(result);
                } else {
                    deny_rules.push(result);
                }
                if let Some(reason) = reason {
                    state.trace_ledger.record(
                        EventType::PolicyViolation,
                        "violated",
                        format!(
                            "policy {} {} {} violated: {}",
                            policy.name, declaration.effect, declaration.rule, reason
                        ),
                        EventFields::default(),
                    );
                    violations.push(PolicyViolation {
                        rule: declaration.rule.clone(),
                        effect: declaration.effect.clone(),
                        reason,
                    });
                }
            }
            let passed = violations.is_empty();
            let action = (!passed)
                .then_some(policy.on_violation.as_ref())
                .flatten()
                .map(|violation| violation.action.clone());
            let trace_required = policy
                .on_violation
                .as_ref()
                .is_some_and(|violation| violation.trace_required);
            let status = if passed {
                "passed"
            } else {
                match action.as_deref() {
                    Some("block") => "failed",
                    Some("review") => "review_required",
                    Some("warn") => "warning",
                    _ => "violated",
                }
            };
            if let Some(action) = &action {
                state.trace_ledger.record(
                    EventType::PolicyActionActivated,
                    "active",
                    format!(
                        "policy {} action {} activated trace_required={trace_required}",
                        policy.name, action
                    ),
                    EventFields::default(),
                );
                actions.push(PolicyActionResult {
                    policy: policy.name.clone(),
                    action: action.clone(),
                    trace_required,
                });
            }
            policy_blocks.push(PolicyBlockResult {
                name: policy.name.clone(),
                passed,
                status: status.into(),
                require_rules,
                deny_rules,
                violations,
                action,
                trace_required,
            });
        }
        let has_block = actions.iter().any(|action| action.action == "block");
        let has_review = actions.iter().any(|action| action.action == "review");
        let has_warn = actions.iter().any(|action| action.action == "warn");
        let has_unhandled_violation = policy_blocks
            .iter()
            .any(|policy| !policy.passed && policy.action.is_none());
        let status = if legacy_failed || has_block {
            "failed"
        } else if has_review {
            "review_required"
        } else if has_warn {
            "warning"
        } else if has_unhandled_violation {
            "violated"
        } else {
            "passed"
        };
        state.trace_ledger.record(
            EventType::PolicyReportGenerated,
            status,
            "policy report generated",
            EventFields::default(),
        );
        if has_block {
            state.fail("policy block action activated");
        }
        PolicyReport {
            status: status.into(),
            assertions: results,
            policy_blocks,
            actions,
            failures,
        }
    }
}

fn record_external_execution_policy_violations(
    bytecode: &BytecodeProgram,
    state: &mut RuntimeState,
) {
    for policy in &bytecode.policies {
        if !policy
            .rules
            .iter()
            .any(|rule| rule.effect == "deny" && rule.rule == "external_execution")
        {
            continue;
        }
        state.trace_ledger.record(
            EventType::PolicyEvaluated,
            "failed",
            format!("policy {} deny external_execution evaluated", policy.name),
            EventFields::default(),
        );
        state.trace_ledger.record(
            EventType::PolicyViolation,
            "violated",
            format!(
                "policy {} deny external_execution violated: external provider execution was attempted",
                policy.name
            ),
            EventFields::default(),
        );
        if let Some(violation) = &policy.on_violation {
            state.trace_ledger.record(
                EventType::PolicyActionActivated,
                "active",
                format!(
                    "policy {} action {} activated trace_required={}",
                    policy.name, violation.action, violation.trace_required
                ),
                EventFields::default(),
            );
        }
    }
}

fn blocked_external_provider<'a>(
    _bytecode: &'a BytecodeProgram,
    errors: &'a [BytecodeError],
) -> Option<&'a str> {
    errors.iter().find_map(|error| {
        let provider = match error {
            BytecodeError::UnknownToolProvider(provider)
            | BytecodeError::UnknownModelProvider(provider) => provider.as_str(),
            _ => return None,
        };
        (provider != "simulated").then_some(provider)
    })
}

/// Derive offline passport policy evidence from the bytecode.
///
/// These booleans feed the v0.19 Policy v2 passport rules. They never resolve
/// DIDs, ASN registrations, or registries; they only inspect declared metadata.
fn policy_evidence_context(bytecode: &BytecodeProgram) -> PolicyEvidenceContext {
    let passports = &bytecode.passports;
    let agent_passport_declared = !bytecode.agents.is_empty()
        && bytecode.agents.iter().all(|agent| {
            passports
                .iter()
                .any(|passport| passport.agent == agent.name)
        });
    let harnesses = &bytecode.provider_harnesses;
    let features = &bytecode.features;
    let secrets = &bytecode.secrets;
    let evidence_maps = &bytecode.atrust_evidence_maps;
    let governance_profiles = &bytecode.governance_profiles;
    let regulatory_mappings = &bytecode.regulatory_mappings;
    let third_party_verifiers = &bytecode.third_party_verifiers;
    let public_conformance_reports = &bytecode.public_conformance_reports;
    let runtime_hardening_profiles = &bytecode.runtime_hardening_profiles;
    let threat_models = &bytecode.threat_models;
    let spec_freezes = &bytecode.spec_freezes;
    let release_candidates = &bytecode.release_candidates;
    let runtime_profiles = &bytecode.runtime_execution_profiles;
    let sandboxed_adapters = &bytecode.sandboxed_provider_adapters;
    PolicyEvidenceContext {
        agent_passport_declared,
        agent_passport_attested: !passports.is_empty()
            && passports
                .iter()
                .all(|passport| !passport.attestations.is_empty()),
        agent_data_residency_declared: !passports.is_empty()
            && passports
                .iter()
                .all(|passport| !passport.data_residency.is_empty()),
        agent_identity_declared: !passports.is_empty()
            && passports
                .iter()
                .all(|passport| !passport.identity.trim().is_empty()),
        provider_harness_declared: !harnesses.is_empty(),
        provider_harness_sandboxed: harnesses.iter().all(|harness| {
            matches!(harness.mode.as_str(), "dry_run" | "simulated")
                && harness.network == "denied"
                && harness.secrets == "denied"
                && matches!(harness.filesystem.as_str(), "none" | "read_only")
        }),
        provider_network_denied: harnesses.iter().all(|harness| harness.network == "denied"),
        provider_secrets_denied: harnesses.iter().all(|harness| harness.secrets == "denied"),
        provider_filesystem_restricted: harnesses
            .iter()
            .all(|harness| matches!(harness.filesystem.as_str(), "none" | "read_only")),
        external_provider_harnessed: bytecode
            .providers
            .iter()
            .filter(|provider| provider.kind == "external")
            .all(|provider| {
                harnesses
                    .iter()
                    .any(|harness| harness.provider == provider.name)
            }),
        feature_flags_declared: !features.is_empty(),
        features_default_disabled: !features.is_empty()
            && features.iter().all(|feature| feature.default == "disabled"),
        experimental_features_require_approval: features
            .iter()
            .filter(|feature| matches!(feature.status.as_str(), "experimental" | "preview"))
            .all(|feature| feature.requires_approval),
        secret_boundaries_declared: !secrets.is_empty(),
        secret_access_denied: !secrets.is_empty()
            && secrets.iter().all(|secret| secret.access == "denied"),
        // Secret declarations never carry secret material in v0.21; the bytecode
        // schema has no value field, so by construction values are always absent.
        secret_values_absent: true,
        external_provider_feature_gated: bytecode
            .providers
            .iter()
            .filter(|provider| provider.kind == "external")
            .all(|provider| {
                features.iter().any(|feature| {
                    feature.provider.as_deref() == Some(provider.name.as_str())
                        && feature.default == "disabled"
                        && feature.requires_approval
                })
            }),
        external_provider_secret_boundary_declared: bytecode
            .providers
            .iter()
            .filter(|provider| provider.kind == "external")
            .all(|provider| {
                secrets.iter().any(|secret| {
                    secret.provider.as_deref() == Some(provider.name.as_str())
                        && secret.access == "denied"
                        && secret.source == "none"
                })
            }),
        adapters_declared: !bytecode.adapters.is_empty(),
        adapters_execution_disabled: !bytecode.adapters.is_empty()
            && bytecode.adapters.iter().all(|a| a.execution == "disabled"),
        adapters_network_denied: !bytecode.adapters.is_empty()
            && bytecode.adapters.iter().all(|a| a.network == "denied"),
        adapters_secrets_denied: !bytecode.adapters.is_empty()
            && bytecode.adapters.iter().all(|a| a.secrets == "denied"),
        adapters_provider_harnessed: bytecode
            .providers
            .iter()
            .filter(|p| p.kind == "external")
            .all(|p| {
                bytecode
                    .adapters
                    .iter()
                    .filter(|a| a.provider == p.name)
                    .all(|a| a.harness.is_some())
            }),
        adapters_feature_gated: bytecode
            .providers
            .iter()
            .filter(|p| p.kind == "external")
            .all(|p| {
                bytecode
                    .adapters
                    .iter()
                    .filter(|a| a.provider == p.name)
                    .all(|a| {
                        if let Some(f) = &a.feature {
                            bytecode.features.iter().any(|feat| {
                                feat.name == *f
                                    && feat.default == "disabled"
                                    && feat.requires_approval
                            })
                        } else {
                            false
                        }
                    })
            }),
        adapters_secret_boundaried: bytecode
            .providers
            .iter()
            .filter(|p| p.kind == "external")
            .all(|p| {
                bytecode
                    .adapters
                    .iter()
                    .filter(|a| a.provider == p.name)
                    .all(|a| {
                        if let Some(s) = &a.secret {
                            bytecode.secrets.iter().any(|sec| {
                                sec.name == *s && sec.access == "denied" && sec.source == "none"
                            })
                        } else {
                            false
                        }
                    })
            }),
        adapters_conformance_declared: !bytecode.adapters.is_empty()
            && bytecode.adapters.iter().all(|a| !a.conformance.is_empty()),
        adapters_evidence_required: !bytecode.adapters.is_empty()
            && bytecode
                .adapters
                .iter()
                .filter(|a| !a.conformance.is_empty())
                .all(|a| a.conformance.iter().any(|c| c == "evidence-required")),
        adapter_profiles_declared: !bytecode.adapter_profiles.is_empty(),
        adapter_profiles_execution_disabled: !bytecode.adapter_profiles.is_empty()
            && bytecode
                .adapter_profiles
                .iter()
                .all(|p| p.execution == "disabled"),
        adapter_profiles_network_denied: !bytecode.adapter_profiles.is_empty()
            && bytecode
                .adapter_profiles
                .iter()
                .all(|p| p.network == "denied"),
        adapter_profiles_secrets_denied: !bytecode.adapter_profiles.is_empty()
            && bytecode
                .adapter_profiles
                .iter()
                .all(|p| p.secrets == "denied"),
        adapter_profiles_linked: !bytecode.adapter_profiles.is_empty()
            && bytecode.adapter_profiles.iter().all(|p| {
                bytecode.adapters.iter().any(|a| a.name == p.adapter)
                    && bytecode.providers.iter().any(|pr| pr.name == p.provider)
            }),
        adapter_profiles_conformance_declared: !bytecode.adapter_profiles.is_empty()
            && bytecode
                .adapter_profiles
                .iter()
                .all(|p| !p.required_conformance.is_empty()),
        vendor_profiles_declared: !bytecode.adapter_profiles.is_empty()
            && bytecode
                .adapter_profiles
                .iter()
                .any(|p| !p.vendor.trim().is_empty()),
        crypto_primitives_declared: !bytecode.cryptos.is_empty(),
        crypto_primitives_allowed: !bytecode.cryptos.is_empty()
            && bytecode.cryptos.iter().all(|c| c.status != "denied"),
        crypto_denied_not_used: !bytecode.cryptos.iter().any(|c| c.status == "denied"),
        crypto_post_quantum_candidates_declared: bytecode
            .cryptos
            .iter()
            .any(|c| c.strength == "post_quantum" || c.status == "post_quantum_candidate"),
        crypto_key_material_absent: true,
        crypto_secret_material_absent: true,
        crypto_execution_absent: true,
        crypto_boundaries_declared: !bytecode.crypto_boundaries.is_empty(),
        post_quantum_readiness_declared: bytecode
            .crypto_boundaries
            .iter()
            .any(|b| b.post_quantum_ready == Some(true)),
        atrust_evidence_maps_declared: !evidence_maps.is_empty(),
        atrust_evidence_map_agents_bound: !evidence_maps.is_empty()
            && evidence_maps
                .iter()
                .all(|m| bytecode.agents.iter().any(|a| a.name == m.agent)),
        atrust_evidence_map_passports_bound: !evidence_maps.is_empty()
            && evidence_maps
                .iter()
                .all(|m| bytecode.passports.iter().any(|p| p.name == m.passport)),
        atrust_evidence_map_identities_bound: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| {
                bytecode
                    .atrust_identities
                    .iter()
                    .any(|i| i.name == m.identity)
            }),
        atrust_evidence_map_credentials_bound: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| {
                bytecode
                    .atrust_credential_contracts
                    .iter()
                    .any(|c| c.name == m.credential_contract)
            }),
        atrust_evidence_map_handshakes_bound: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| {
                bytecode
                    .atrust_handshakes
                    .iter()
                    .any(|h| h.name == m.handshake)
            }),
        atrust_evidence_map_ledgers_bound: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| {
                bytecode
                    .trust_ledgers
                    .iter()
                    .any(|l| l.name == m.trust_ledger)
            }),
        atrust_evidence_map_bridges_bound: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| {
                !m.mcp_bridges.is_empty()
                    && !m.a2a_bridges.is_empty()
                    && m.mcp_bridges.iter().all(|name| {
                        bytecode
                            .mcp_bridge_contracts
                            .iter()
                            .any(|b| b.name == *name)
                    })
                    && m.a2a_bridges.iter().all(|name| {
                        bytecode
                            .a2a_bridge_contracts
                            .iter()
                            .any(|b| b.name == *name)
                    })
            }),
        atrust_evidence_map_policies_bound: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| {
                !m.policies.is_empty()
                    && m.policies
                        .iter()
                        .all(|name| bytecode.policies.iter().any(|p| p.name == *name))
            }),
        atrust_evidence_map_coverage_required: !evidence_maps.is_empty()
            && evidence_maps
                .iter()
                .all(|m| matches!(m.coverage.as_str(), "required" | "complete")),
        atrust_evidence_map_verification_non_verifying: !evidence_maps.is_empty()
            && evidence_maps
                .iter()
                .all(|m| matches!(m.verification.as_str(), "declared_only" | "disabled")),
        atrust_evidence_map_resolution_disabled: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| m.resolution == "disabled"),
        atrust_evidence_map_network_denied: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| m.network == "denied"),
        atrust_evidence_map_external_execution_disabled: !evidence_maps.is_empty()
            && evidence_maps
                .iter()
                .all(|m| m.external_execution == "disabled"),
        atrust_evidence_map_secret_material_denied: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| m.secret_material == "denied"),
        atrust_evidence_map_key_material_denied: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| m.key_material == "denied"),
        atrust_evidence_map_execution_disabled: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| m.execution == "disabled"),
        atrust_evidence_map_security_claims_absent: !evidence_maps.is_empty()
            && evidence_maps.iter().all(|m| m.security_claims == "none"),
        governance_profiles_declared: !governance_profiles.is_empty(),
        governance_profiles_evidence_bound: !governance_profiles.is_empty()
            && governance_profiles.iter().all(|profile| {
                bytecode
                    .atrust_evidence_maps
                    .iter()
                    .any(|map| map.name == profile.evidence_map)
                    && bytecode
                        .trust_ledgers
                        .iter()
                        .any(|ledger| ledger.name == profile.trust_ledger)
            }),
        governance_profiles_controls_mapped: !governance_profiles.is_empty()
            && governance_profiles.iter().all(|profile| {
                !profile.controls.is_empty()
                    && profile.controls.iter().all(|control| {
                        matches!(
                            control.status.as_str(),
                            "mapped" | "declared" | "pending_review" | "not_applicable"
                        )
                    })
            }),
        governance_profiles_runtime_disabled: !governance_profiles.is_empty()
            && governance_profiles.iter().all(|profile| {
                profile.network == "denied"
                    && profile.external_execution == "disabled"
                    && profile.secret_material == "denied"
                    && profile.key_material == "denied"
                    && profile.execution == "disabled"
            }),
        governance_profiles_security_claims_absent: !governance_profiles.is_empty()
            && governance_profiles
                .iter()
                .all(|profile| profile.security_claims == "none"),
        governance_profiles_no_legal_certification: !governance_profiles.is_empty()
            && governance_profiles.iter().all(|profile| {
                matches!(
                    profile.assurance.as_str(),
                    "declared_only" | "evidence_mapped" | "manually_reviewed"
                )
            }),
        regulatory_mappings_declared: !regulatory_mappings.is_empty(),
        regulatory_mappings_profiles_bound: !regulatory_mappings.is_empty()
            && regulatory_mappings.iter().all(|mapping| {
                governance_profiles.iter().any(|profile| {
                    profile.name == mapping.governance_profile
                        && profile.evidence_map == mapping.evidence_map
                })
            }),
        regulatory_mappings_obligations_mapped: !regulatory_mappings.is_empty()
            && regulatory_mappings.iter().all(|mapping| {
                !mapping.obligations.is_empty()
                    && mapping.obligations.iter().all(|obligation| {
                        matches!(
                            obligation.status.as_str(),
                            "mapped" | "pending_review" | "gap" | "not_applicable"
                        )
                    })
            }),
        regulatory_mappings_controls_bound: !regulatory_mappings.is_empty()
            && regulatory_mappings.iter().all(|mapping| {
                governance_profiles
                    .iter()
                    .find(|profile| profile.name == mapping.governance_profile)
                    .is_some_and(|profile| {
                        mapping.obligations.iter().all(|obligation| {
                            profile
                                .controls
                                .iter()
                                .any(|control| control.id == obligation.control)
                        })
                    })
            }),
        regulatory_mappings_legal_claims_absent: !regulatory_mappings.is_empty()
            && regulatory_mappings
                .iter()
                .all(|mapping| mapping.legal_claims == "none"),
        regulatory_mappings_certification_absent: !regulatory_mappings.is_empty()
            && regulatory_mappings
                .iter()
                .all(|mapping| mapping.certification == "none"),
        regulatory_mappings_runtime_disabled: !regulatory_mappings.is_empty()
            && regulatory_mappings.iter().all(|mapping| {
                mapping.network == "denied"
                    && mapping.external_execution == "disabled"
                    && mapping.secret_material == "denied"
                    && mapping.key_material == "denied"
                    && mapping.execution == "disabled"
            }),
        regulatory_mappings_security_claims_absent: !regulatory_mappings.is_empty()
            && regulatory_mappings
                .iter()
                .all(|mapping| mapping.security_claims == "none"),
        third_party_verifiers_declared: !third_party_verifiers.is_empty(),
        third_party_verifiers_identity_declared: !third_party_verifiers.is_empty()
            && third_party_verifiers.iter().all(|verifier| {
                !verifier.display_name.is_empty()
                    && !verifier.organization.is_empty()
                    && !verifier.jurisdiction.is_empty()
                    && matches!(
                        verifier.identity_mode.as_str(),
                        "declared_only" | "document_only"
                    )
            }),
        third_party_verifiers_scope_bounded: !third_party_verifiers.is_empty()
            && third_party_verifiers.iter().all(|verifier| {
                !verifier.allowed_scopes.is_empty() && !verifier.disallowed_claims.is_empty()
            }),
        third_party_verifiers_runtime_disabled: !third_party_verifiers.is_empty()
            && third_party_verifiers.iter().all(|verifier| {
                verifier.network == "denied"
                    && verifier.external_execution == "disabled"
                    && verifier.secret_material == "denied"
                    && verifier.key_material == "denied"
                    && verifier.execution == "disabled"
            }),
        third_party_verifiers_legal_claims_absent: !third_party_verifiers.is_empty()
            && third_party_verifiers
                .iter()
                .all(|verifier| verifier.legal_claims == "none"),
        third_party_verifiers_certification_absent: !third_party_verifiers.is_empty()
            && third_party_verifiers
                .iter()
                .all(|verifier| verifier.certification == "none"),
        third_party_verifiers_security_claims_absent: !third_party_verifiers.is_empty()
            && third_party_verifiers
                .iter()
                .all(|verifier| verifier.security_claims == "none"),
        public_conformance_reports_declared: !public_conformance_reports.is_empty(),
        public_conformance_reports_verifiers_bound: !public_conformance_reports.is_empty()
            && public_conformance_reports.iter().all(|report| {
                third_party_verifiers
                    .iter()
                    .any(|verifier| verifier.name == report.verifier)
            }),
        public_conformance_reports_artifacts_declared: !public_conformance_reports.is_empty()
            && public_conformance_reports.iter().all(|report| {
                !report.suite.is_empty()
                    && matches!(report.suite_version.as_str(), "0.34" | "0.35")
                    && !report.source_artifact.is_empty()
                    && !report.bytecode_artifact.is_empty()
            }),
        public_conformance_reports_evidence_bound: !public_conformance_reports.is_empty()
            && public_conformance_reports.iter().all(|report| {
                bytecode
                    .atrust_evidence_maps
                    .iter()
                    .any(|map| map.name == report.evidence_map)
                    && report.evidence_bundle == "required"
                    && report.security_report == "required"
                    && report.trace == "required"
            }),
        public_conformance_reports_governance_bound: !public_conformance_reports.is_empty()
            && public_conformance_reports.iter().all(|report| {
                governance_profiles
                    .iter()
                    .any(|profile| profile.name == report.governance_profile)
            }),
        public_conformance_reports_regulatory_bound: !public_conformance_reports.is_empty()
            && public_conformance_reports.iter().all(|report| {
                regulatory_mappings
                    .iter()
                    .any(|mapping| mapping.name == report.regulatory_mapping)
            }),
        public_conformance_reports_replayable: !public_conformance_reports.is_empty()
            && public_conformance_reports.iter().all(|report| {
                matches!(
                    report.reproducibility.as_str(),
                    "declared" | "replayable_locally" | "document_only"
                )
            }),
        public_conformance_reports_runtime_disabled: !public_conformance_reports.is_empty()
            && public_conformance_reports.iter().all(|report| {
                report.network == "denied"
                    && report.external_execution == "disabled"
                    && report.secret_material == "denied"
                    && report.key_material == "denied"
                    && report.execution == "disabled"
            }),
        public_conformance_reports_legal_claims_absent: !public_conformance_reports.is_empty()
            && public_conformance_reports
                .iter()
                .all(|report| report.legal_claims == "none"),
        public_conformance_reports_certification_absent: !public_conformance_reports.is_empty()
            && public_conformance_reports
                .iter()
                .all(|report| report.certification == "none"),
        public_conformance_reports_security_claims_absent: !public_conformance_reports.is_empty()
            && public_conformance_reports
                .iter()
                .all(|report| report.security_claims == "none"),
        runtime_hardening_profiles_declared: !runtime_hardening_profiles.is_empty(),
        runtime_hardening_evidence_bound: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles.iter().all(|profile| {
                bytecode
                    .atrust_evidence_maps
                    .iter()
                    .any(|map| map.name == profile.evidence_map)
                    && governance_profiles
                        .iter()
                        .any(|item| item.name == profile.governance_profile)
                    && public_conformance_reports
                        .iter()
                        .any(|item| item.name == profile.public_conformance_report)
                    && profile.evidence == "required"
            }),
        runtime_hardening_deny_by_default: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles.iter().all(|p| p.deny_by_default),
        runtime_hardening_sandbox_required: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.sandbox == "required"),
        runtime_hardening_network_denied: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.network == "denied"),
        runtime_hardening_external_providers_disabled: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.external_providers == "disabled"),
        runtime_hardening_tool_execution_disabled: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.tool_execution == "disabled"),
        runtime_hardening_agent_execution_disabled: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.agent_execution == "disabled"),
        runtime_hardening_filesystem_denied: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.filesystem_access == "denied"),
        runtime_hardening_env_denied: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.env_access == "denied"),
        runtime_hardening_secret_material_denied: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.secret_material == "denied"),
        runtime_hardening_key_material_denied: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.key_material == "denied"),
        runtime_hardening_audit_log_required: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.audit_log == "required"),
        runtime_hardening_security_claims_absent: !runtime_hardening_profiles.is_empty()
            && runtime_hardening_profiles
                .iter()
                .all(|p| p.security_claims == "none"),
        threat_models_declared: !threat_models.is_empty(),
        threat_models_hardening_bound: !threat_models.is_empty()
            && threat_models.iter().all(|model| {
                runtime_hardening_profiles
                    .iter()
                    .any(|profile| profile.name == model.hardening_profile)
            }),
        threat_models_assets_mapped: !threat_models.is_empty()
            && threat_models.iter().all(|model| !model.assets.is_empty()),
        threat_models_threats_mapped: !threat_models.is_empty()
            && threat_models.iter().all(|model| !model.threats.is_empty()),
        threat_models_mitigations_mapped: !threat_models.is_empty()
            && threat_models
                .iter()
                .all(|model| !model.mitigations.is_empty()),
        threat_models_runtime_disabled: !threat_models.is_empty()
            && threat_models.iter().all(|model| {
                model.network == "denied"
                    && model.external_execution == "disabled"
                    && model.tool_execution == "disabled"
                    && model.agent_execution == "disabled"
                    && model.execution == "disabled"
            }),
        threat_models_network_denied: !threat_models.is_empty()
            && threat_models.iter().all(|model| model.network == "denied"),
        threat_models_secret_material_denied: !threat_models.is_empty()
            && threat_models
                .iter()
                .all(|model| model.secret_material == "denied"),
        threat_models_key_material_denied: !threat_models.is_empty()
            && threat_models
                .iter()
                .all(|model| model.key_material == "denied"),
        threat_models_execution_disabled: !threat_models.is_empty()
            && threat_models
                .iter()
                .all(|model| model.execution == "disabled"),
        threat_models_security_claims_absent: !threat_models.is_empty()
            && threat_models
                .iter()
                .all(|model| model.security_claims == "none"),
        spec_freezes_declared: !spec_freezes.is_empty(),
        spec_freeze_versions_pinned: !spec_freezes.is_empty()
            && spec_freezes.iter().all(|freeze| freeze.version == "0.36"),
        spec_freeze_features_declared: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| !freeze.frozen_features.is_empty()),
        spec_freeze_compatibility_declared: !spec_freezes.is_empty()
            && spec_freezes.iter().all(|freeze| {
                ["0.34", "0.35", "0.36"]
                    .iter()
                    .all(|version| freeze.compatible_versions.iter().any(|v| v == version))
            }),
        spec_freeze_required_suites_declared: !spec_freezes.is_empty()
            && spec_freezes.iter().all(|freeze| {
                freeze
                    .required_suites
                    .iter()
                    .any(|suite| suite == "conformance/suite.v036.json")
            }),
        spec_freeze_runtime_disabled: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| freeze.runtime_status == "disabled"),
        spec_freeze_network_denied: !spec_freezes.is_empty()
            && spec_freezes.iter().all(|freeze| freeze.network == "denied"),
        spec_freeze_external_execution_disabled: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| freeze.external_execution == "disabled"),
        spec_freeze_provider_execution_disabled: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| freeze.provider_execution == "disabled"),
        spec_freeze_secret_material_denied: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| freeze.secret_material == "denied"),
        spec_freeze_key_material_denied: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| freeze.key_material == "denied"),
        spec_freeze_env_denied: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| freeze.env_access == "denied"),
        spec_freeze_filesystem_denied: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| freeze.filesystem_access == "denied"),
        spec_freeze_security_claims_absent: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| freeze.security_claims == "none"),
        spec_freeze_legal_claims_absent: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| freeze.legal_claims == "none"),
        spec_freeze_certification_absent: !spec_freezes.is_empty()
            && spec_freezes
                .iter()
                .all(|freeze| freeze.certification == "none"),
        release_candidates_declared: !release_candidates.is_empty(),
        release_candidates_spec_freeze_bound: !release_candidates.is_empty()
            && release_candidates.iter().all(|candidate| {
                spec_freezes
                    .iter()
                    .any(|freeze| freeze.name == candidate.spec_freeze)
            }),
        release_candidates_artifacts_declared: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| !candidate.required_artifacts.is_empty()),
        release_candidates_checks_declared: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| !candidate.required_checks.is_empty()),
        release_candidates_compatibility_matrix_declared: !release_candidates.is_empty()
            && release_candidates.iter().all(|candidate| {
                ["0.34", "0.35", "0.36"].iter().all(|version| {
                    candidate
                        .compatibility_matrix
                        .iter()
                        .any(|entry| entry.version == *version)
                })
            }),
        release_candidates_limitations_declared: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| !candidate.known_limitations.is_empty()),
        release_candidates_runtime_disabled: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.runtime_status == "disabled"),
        release_candidates_network_denied: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.network == "denied"),
        release_candidates_external_execution_disabled: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.external_execution == "disabled"),
        release_candidates_provider_execution_disabled: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.provider_execution == "disabled"),
        release_candidates_secret_material_denied: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.secret_material == "denied"),
        release_candidates_key_material_denied: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.key_material == "denied"),
        release_candidates_env_denied: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.env_access == "denied"),
        release_candidates_filesystem_denied: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.filesystem_access == "denied"),
        release_candidates_security_claims_absent: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.security_claims == "none"),
        release_candidates_legal_claims_absent: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.legal_claims == "none"),
        release_candidates_certification_absent: !release_candidates.is_empty()
            && release_candidates
                .iter()
                .all(|candidate| candidate.certification == "none"),
        runtime_execution_profiles_declared: !runtime_profiles.is_empty(),
        runtime_profiles_agents_bound: !runtime_profiles.is_empty()
            && runtime_profiles.iter().all(|profile| {
                !profile.agents.is_empty()
                    && profile
                        .agents
                        .iter()
                        .all(|name| bytecode.agents.iter().any(|agent| agent.name == *name))
            }),
        runtime_profiles_provider_bound: !runtime_profiles.is_empty()
            && runtime_profiles.iter().all(|profile| {
                profile.provider == "simulated"
                    || bytecode
                        .providers
                        .iter()
                        .any(|provider| provider.name == profile.provider)
            }),
        runtime_profiles_hardening_bound: !runtime_profiles.is_empty()
            && runtime_profiles.iter().all(|profile| {
                runtime_hardening_profiles
                    .iter()
                    .any(|hardening| hardening.name == profile.hardening)
            }),
        runtime_profiles_evidence_bound: !runtime_profiles.is_empty()
            && runtime_profiles.iter().all(|profile| {
                evidence_maps
                    .iter()
                    .any(|evidence| evidence.name == profile.evidence_map)
                    && profile.evidence == "required"
            }),
        runtime_profiles_governance_bound: !runtime_profiles.is_empty()
            && runtime_profiles.iter().all(|profile| {
                governance_profiles.iter().any(|governance| {
                    governance.name == profile.governance_profile
                        && governance.evidence_map == profile.evidence_map
                })
            }),
        runtime_profiles_fail_closed: !runtime_profiles.is_empty()
            && runtime_profiles.iter().all(|profile| profile.fail_closed),
        runtime_profiles_external_execution_sandboxed: !runtime_profiles.is_empty()
            && runtime_profiles.iter().all(|profile| {
                matches!(
                    profile.external_execution.as_str(),
                    "disabled" | "sandboxed"
                )
            }),
        runtime_profiles_tool_execution_disabled: !runtime_profiles.is_empty()
            && runtime_profiles
                .iter()
                .all(|profile| profile.tool_execution == "disabled"),
        runtime_profiles_agent_execution_disabled: !runtime_profiles.is_empty()
            && runtime_profiles
                .iter()
                .all(|profile| profile.agent_execution == "disabled"),
        runtime_profiles_secret_refs_only: !runtime_profiles.is_empty()
            && runtime_profiles
                .iter()
                .all(|profile| matches!(profile.secrets.as_str(), "denied" | "env_reference_only")),
        runtime_profiles_security_claims_absent: !runtime_profiles.is_empty()
            && runtime_profiles
                .iter()
                .all(|profile| profile.security_claims == "none"),
        sandboxed_provider_adapters_declared: !sandboxed_adapters.is_empty(),
        sandboxed_provider_adapters_provider_bound: !sandboxed_adapters.is_empty()
            && sandboxed_adapters.iter().all(|adapter| {
                bytecode
                    .providers
                    .iter()
                    .any(|provider| provider.name == adapter.provider)
            }),
        sandboxed_provider_adapters_runtime_bound: !sandboxed_adapters.is_empty()
            && sandboxed_adapters.iter().all(|adapter| {
                runtime_profiles.iter().any(|profile| {
                    profile.name == adapter.runtime && profile.provider == adapter.provider
                })
            }),
        sandboxed_provider_adapters_operations_bounded: !sandboxed_adapters.is_empty()
            && sandboxed_adapters.iter().all(|adapter| {
                !adapter.allowed_operations.is_empty()
                    && !adapter.denied_operations.is_empty()
                    && adapter
                        .allowed_operations
                        .iter()
                        .all(|operation| !adapter.denied_operations.contains(operation))
            }),
        sandboxed_provider_adapters_network_declared: !sandboxed_adapters.is_empty()
            && sandboxed_adapters
                .iter()
                .all(|adapter| matches!(adapter.network.as_str(), "denied" | "declared_only")),
        sandboxed_provider_adapters_external_execution_sandboxed: !sandboxed_adapters.is_empty()
            && sandboxed_adapters.iter().all(|adapter| {
                matches!(
                    adapter.external_execution.as_str(),
                    "disabled" | "sandboxed"
                )
            }),
        sandboxed_provider_adapters_tool_execution_disabled: !sandboxed_adapters.is_empty()
            && sandboxed_adapters
                .iter()
                .all(|adapter| adapter.tool_execution == "disabled"),
        sandboxed_provider_adapters_secret_refs_redacted: !sandboxed_adapters.is_empty()
            && sandboxed_adapters.iter().all(|adapter| {
                adapter.redacted
                    && adapter.endpoint_value.is_none()
                    && adapter.secret_value.is_none()
                    && (adapter.endpoint_ref.starts_with("env:")
                        || adapter.endpoint_ref.starts_with("config:"))
                    && (adapter.secret_ref.starts_with("env:")
                        || adapter.secret_ref.starts_with("secret_boundary:"))
            }),
        sandboxed_provider_adapters_fail_closed: !sandboxed_adapters.is_empty()
            && sandboxed_adapters.iter().all(|adapter| adapter.fail_closed),
        sandboxed_provider_adapters_security_claims_absent: !sandboxed_adapters.is_empty()
            && sandboxed_adapters
                .iter()
                .all(|adapter| adapter.security_claims == "none"),
        ..PolicyEvidenceContext::default()
    }
}

#[cfg(test)]
mod tests {
    use super::Vm;
    use crate::{EventType, InjectedMessage, RuntimeStatus, Scheduler};
    use argorix_bytecode::{
        BytecodeAgent, BytecodePolicy, BytecodePolicyRule, BytecodePolicyViolation,
        BytecodeProgram, BytecodeProviderContract, BytecodeProviderHarness, Instruction,
    };
    use serde_json::json;

    fn add_external_contract(bytecode: &mut BytecodeProgram, enabled: bool) {
        bytecode.bytecode_version = "0.11".into();
        let contract = BytecodeProviderContract {
            name: "OpenAI".into(),
            kind: "external".into(),
            enabled,
            dry_run_only: true,
            requires_feature_flag: true,
            requires_explicit_approval: true,
            allowed_targets: vec![],
            allowed_capabilities: vec![],
        };
        bytecode.providers.push(contract.clone());
        bytecode.instructions.insert(
            0,
            Instruction::DeclareProviderContract {
                name: contract.name,
                kind: contract.kind,
                enabled: contract.enabled,
                dry_run_only: contract.dry_run_only,
                requires_feature_flag: contract.requires_feature_flag,
                requires_explicit_approval: contract.requires_explicit_approval,
                allowed_targets: contract.allowed_targets,
                allowed_capabilities: contract.allowed_capabilities,
            },
        );
    }

    fn valid_bytecode() -> BytecodeProgram {
        BytecodeProgram {
            bytecode_version: "0.3".into(),
            language: "Argorix Lang".into(),
            module: "Example".into(),
            modules: vec![],
            imports: vec![],
            providers: vec![],
            provider_harnesses: vec![],
            features: vec![],
            secrets: vec![],
            adapters: vec![],
            adapter_profiles: vec![],
            cryptos: vec![],
            crypto_boundaries: vec![],
            did_methods: vec![],
            atrust_boundaries: vec![],
            atrust_identities: vec![],
            atrust_credential_contracts: vec![],
            atrust_handshakes: vec![],
            trust_ledgers: vec![],
            mcp_bridge_contracts: vec![],
            a2a_bridge_contracts: vec![],
            atrust_evidence_maps: vec![],
            governance_profiles: vec![],
            regulatory_mappings: vec![],
            third_party_verifiers: vec![],
            public_conformance_reports: vec![],
            runtime_hardening_profiles: vec![],
            threat_models: vec![],
            spec_freezes: vec![],
            release_candidates: vec![],
            runtime_execution_profiles: vec![],
            sandboxed_provider_adapters: vec![],
            assertions: vec![],
            policies: vec![],
            types: vec![],
            enums: vec![],
            failures: vec![],
            passports: vec![],
            agents: vec![BytecodeAgent {
                name: "Worker".into(),
                approval: "denied".into(),
            }],
            capabilities: vec![],
            tools: vec![],
            models: vec![],
            instructions: vec![
                Instruction::DeclareProtocol {
                    name: "Flow".into(),
                },
                Instruction::SendMessage {
                    from: "User".into(),
                    to: "Worker".into(),
                    act: "tell".into(),
                    message_type: "Ping".into(),
                },
                Instruction::End,
            ],
        }
    }

    #[test]
    fn initializes_runtime_state_from_bytecode() {
        let state = Vm::new().initialize(&valid_bytecode()).unwrap();
        assert_eq!(state.status, RuntimeStatus::Initialized);
        assert!(state.mailboxes.contains_key("Worker"));
    }

    #[test]
    fn dry_run_generates_protocol_trace_and_mailbox_summary() {
        let trace = Vm::new().run_dry(&valid_bytecode()).unwrap();
        assert_eq!(trace.status, "completed");
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.steps[0].message_type, "Ping");
        assert_eq!(trace.mailboxes[0].delivered, 1);
        assert_eq!(trace.mailboxes[0].processed, 1);
    }

    #[test]
    fn ledger_records_lifecycle_and_delivery_events() {
        let trace = Vm::new().run_dry(&valid_bytecode()).unwrap();
        for expected in [
            EventType::VmStarted,
            EventType::MessageDelivered,
            EventType::VmCompleted,
        ] {
            assert!(trace
                .events
                .iter()
                .any(|event| event.event_type == expected));
        }
    }

    #[test]
    fn failure_keeps_trace_ledger_in_runtime_state() {
        let mut invalid = valid_bytecode();
        invalid.instructions.pop();
        let mut state = Vm::new().initialize(&invalid).unwrap();
        let result = Scheduler::new().run(&invalid, &mut state);

        assert!(result.is_err());
        assert_eq!(state.status, RuntimeStatus::Failed);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }

    #[test]
    fn reactive_outcome_preserves_failed_runtime_state_and_ledger() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/prompt_defense_v05.argbc.json"
        ))
        .unwrap();
        bytecode.instructions.pop();

        let outcome = Vm::new().run_reactive_outcome(
            &bytecode,
            InjectedMessage {
                from: "User".into(),
                to: "PromptScanner".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
        );

        assert!(outcome.result.is_err());
        assert_eq!(outcome.state.status, RuntimeStatus::Failed);
        assert!(outcome
            .state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }

    #[test]
    fn json_trace_contains_mailboxes() {
        let trace = Vm::new().run_dry(&valid_bytecode()).unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "0.5");
        assert_eq!(json["scheduler"], "deterministic");
        assert_eq!(json["mailboxes"][0]["agent"], "Worker");
    }

    #[test]
    fn reactive_json_trace_has_expected_mode() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/prompt_defense_v05.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "PromptScanner".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["mode"], "reactive-dry-run");
        assert_eq!(json["steps"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn v06_json_contains_agent_state_and_intrinsics() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/prompt_defense_v06.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "PromptScanner".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "1.0");
        assert_eq!(json["agent_state"].as_array().unwrap().len(), 3);
        assert_eq!(json["intrinsics"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn v07_json_contains_tool_calls() {
        let bytecode: BytecodeProgram =
            serde_json::from_str(include_str!("../../../examples/tool_call_v07.argbc.json"))
                .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "1.0");
        assert_eq!(json["tool_calls"][0]["tool"], "WebSearch");
        assert_eq!(json["tool_calls"][0]["mode"], "dry-run");
    }

    #[test]
    fn v08_json_contains_model_calls() {
        let bytecode: BytecodeProgram =
            serde_json::from_str(include_str!("../../../examples/model_call_v08.argbc.json"))
                .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "1.0");
        assert_eq!(json["model_calls"][0]["model"], "GuardModel");
        assert_eq!(json["model_calls"][0]["provider"], "simulated");
    }

    #[test]
    fn v09_policy_report_verifies_global_assertions() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(trace.status, "completed");
        assert_eq!(trace.policy_report.status, "passed");
        assert_eq!(trace.policy_report.assertions.len(), 6);
        assert!(trace
            .policy_report
            .assertions
            .iter()
            .all(|assertion| assertion.status == "passed"));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyReportGenerated));
    }

    #[test]
    fn provider_boundary_routes_tool_and_model_calls() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(trace.vm_version, "1.0");
        assert_eq!(trace.providers[0].name, "simulated");
        assert_eq!(trace.providers[0].kind, "simulated");
        assert_eq!(trace.provider_calls.len(), 2);
        assert_eq!(trace.provider_calls[0].kind, "tool");
        assert_eq!(trace.provider_calls[0].target, "WebSearch");
        assert_eq!(trace.provider_calls[1].kind, "model");
        assert_eq!(trace.provider_calls[1].target, "GuardModel");
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderSelected));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderRegistered));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderRequestCreated));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderResponseReceived));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderDryRunEnforced));

        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["providers"][0]["name"], "simulated");
        assert_eq!(json["provider_calls"][0]["kind"], "tool");
        assert_eq!(json["provider_calls"][1]["kind"], "model");
    }

    #[test]
    fn vm_loads_and_validates_provider_contracts_before_execution() {
        let mut bytecode = valid_bytecode();
        add_external_contract(&mut bytecode, false);
        let vm = Vm::new();
        let mut state = vm.initialize(&bytecode).unwrap();
        let registry = vm.load_provider_contracts(&bytecode, &mut state).unwrap();

        assert!(registry.contains("simulated"));
        assert!(registry.contains_contract("OpenAI"));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderContractDeclared));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderContractValidated));
    }

    #[test]
    fn vm_json_preserves_populated_provider_allowlists() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v012.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        assert_eq!(trace.vm_version, "1.0");
        assert_eq!(
            trace.provider_contracts[0].allowed_targets,
            vec!["GuardModel"]
        );
        assert_eq!(
            trace.provider_contracts[0].allowed_capabilities,
            vec!["model.invoke"]
        );
    }
    #[test]
    fn rejected_provider_contract_preserves_runtime_ledger() {
        let mut bytecode = valid_bytecode();
        add_external_contract(&mut bytecode, true);
        let vm = Vm::new();
        let mut state = vm.initialize(&bytecode).unwrap();
        let result = vm.load_provider_contracts(&bytecode, &mut state);

        assert!(result.is_err());
        assert_eq!(state.status, RuntimeStatus::Failed);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderContractRejected));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }

    #[test]
    fn reactive_json_lists_executable_providers_and_declarative_contracts_separately() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_boundary_v010.argbc.json"
        ))
        .unwrap();
        add_external_contract(&mut bytecode, false);
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();

        assert_eq!(json["vm_version"], "1.0");
        assert_eq!(json["providers"][0]["name"], "simulated");
        assert_eq!(json["providers"][0]["enabled"], true);
        assert_eq!(json["provider_contracts"][0]["name"], "OpenAI");
        assert_eq!(json["provider_contracts"][0]["allowed_targets"], json!([]));
        assert_eq!(
            json["provider_contracts"][0]["allowed_capabilities"],
            json!([])
        );
    }

    #[test]
    fn v09_policy_failure_activates_declared_failure_mode() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        bytecode
            .assertions
            .iter_mut()
            .find(|assertion| assertion.name == "runtime_status")
            .unwrap()
            .argument = Some("failed".into());

        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(trace.status, "failed");
        assert_eq!(trace.policy_report.status, "failed");
        assert_eq!(trace.policy_report.failures[0].name, "PolicyViolation");
        assert_eq!(trace.policy_report.failures[0].action, "block");
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::AssertionFailed));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::FailureModeActivated));
    }

    #[test]
    fn policy_v2_separates_legacy_assertions_and_policy_blocks() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        bytecode.bytecode_version = "0.17".into();
        bytecode.policies = vec![BytecodePolicy {
            name: "ProviderSafety".into(),
            rules: vec![BytecodePolicyRule {
                effect: "deny".into(),
                rule: "external_execution".into(),
            }],
            on_violation: Some(BytecodePolicyViolation {
                action: "block".into(),
                trace_required: true,
            }),
        }];
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        assert_eq!(trace.policy_report.assertions.len(), 6);
        assert_eq!(trace.policy_report.policy_blocks.len(), 1);
        assert!(trace.policy_report.policy_blocks[0].passed);
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyDeclared));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyEvaluated));
    }

    #[test]
    fn policy_v2_actions_apply_block_review_warn_and_no_action() {
        for (action, expected_status, expected_error) in [
            (Some("block"), "failed", true),
            (Some("review"), "review_required", false),
            (Some("warn"), "warning", false),
            (None, "violated", false),
        ] {
            let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
                "../../../examples/policy_assertions_v09.argbc.json"
            ))
            .unwrap();
            bytecode.bytecode_version = "0.17".into();
            bytecode.policies = vec![BytecodePolicy {
                name: format!("Policy{expected_status}"),
                rules: vec![BytecodePolicyRule {
                    effect: "require".into(),
                    rule: "evidence_bundle_verified".into(),
                }],
                on_violation: action.map(|action| BytecodePolicyViolation {
                    action: action.into(),
                    trace_required: true,
                }),
            }];
            let injection = InjectedMessage {
                from: "User".into(),
                to: "ResearchAgent".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            };
            let outcome = Vm::new().run_reactive_outcome(&bytecode, injection.clone());
            let trace = outcome.result.unwrap();
            assert_eq!(trace.policy_report.status, expected_status);
            assert_eq!(
                trace.status,
                if expected_error {
                    "failed"
                } else {
                    "completed"
                }
            );
            assert_eq!(
                Vm::new().run_reactive(&bytecode, injection).is_err(),
                expected_error
            );
            assert!(trace
                .events
                .iter()
                .any(|event| event.event_type == EventType::PolicyViolation));
        }
    }

    #[test]
    fn deny_external_execution_records_violation_for_blocked_attempt() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        bytecode.bytecode_version = "0.17".into();
        bytecode.models[0].provider = "OpenAI".into();
        bytecode.policies = vec![BytecodePolicy {
            name: "ProviderSafety".into(),
            rules: vec![BytecodePolicyRule {
                effect: "deny".into(),
                rule: "external_execution".into(),
            }],
            on_violation: Some(BytecodePolicyViolation {
                action: "block".into(),
                trace_required: true,
            }),
        }];
        let outcome = Vm::new().run_reactive_outcome(
            &bytecode,
            InjectedMessage {
                from: "User".into(),
                to: "ResearchAgent".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
        );
        assert!(outcome.result.is_err());
        assert!(outcome
            .state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ExternalProviderExecutionBlocked));
        assert!(outcome
            .state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyViolation));
        assert!(outcome
            .state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyActionActivated));
    }

    #[test]
    fn provider_harness_metadata_is_traced_and_policy_evaluated_offline() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        add_external_contract(&mut bytecode, false);
        bytecode.bytecode_version = "0.20".into();
        bytecode.provider_harnesses = vec![BytecodeProviderHarness {
            name: "OpenAIHarness".into(),
            provider: "OpenAI".into(),
            feature: None,
            secret: None,
            mode: "dry_run".into(),
            network: "denied".into(),
            secrets: "denied".into(),
            filesystem: "none".into(),
            max_steps: Some(10),
            timeout_ms: Some(1000),
            input_contract: None,
            output_contract: None,
            attestations: vec!["policy-check".into()],
        }];
        bytecode.policies = vec![BytecodePolicy {
            name: "HarnessPolicy".into(),
            rules: [
                "provider_harness_declared",
                "provider_harness_sandboxed",
                "provider_network_denied",
                "provider_secrets_denied",
                "provider_filesystem_restricted",
                "external_provider_harnessed",
            ]
            .into_iter()
            .map(|rule| BytecodePolicyRule {
                effect: "require".into(),
                rule: rule.into(),
            })
            .collect(),
            on_violation: Some(BytecodePolicyViolation {
                action: "review".into(),
                trace_required: true,
            }),
        }];

        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(trace.vm_version, "1.0");
        assert_eq!(trace.provider_harnesses, bytecode.provider_harnesses);
        assert_eq!(trace.policy_report.status, "passed");
        for expected in [
            EventType::ProviderHarnessDeclared,
            EventType::ProviderHarnessValidated,
            EventType::ProviderHarnessSandboxed,
        ] {
            assert!(trace
                .events
                .iter()
                .any(|event| event.event_type == expected));
        }
    }

    #[test]
    fn provider_harness_policies_fail_when_declaration_or_coverage_is_missing() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        add_external_contract(&mut bytecode, false);
        bytecode.bytecode_version = "0.20".into();
        bytecode.policies = vec![BytecodePolicy {
            name: "HarnessPolicy".into(),
            rules: ["provider_harness_declared", "external_provider_harnessed"]
                .into_iter()
                .map(|rule| BytecodePolicyRule {
                    effect: "require".into(),
                    rule: rule.into(),
                })
                .collect(),
            on_violation: Some(BytecodePolicyViolation {
                action: "review".into(),
                trace_required: true,
            }),
        }];
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        assert_eq!(trace.policy_report.status, "review_required");
        let violations = &trace.policy_report.policy_blocks[0].violations;
        assert!(violations
            .iter()
            .any(|violation| violation.rule == "provider_harness_declared"));
        assert!(violations
            .iter()
            .any(|violation| violation.rule == "external_provider_harnessed"));
    }

    #[test]
    fn harness_does_not_make_external_provider_executable() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        add_external_contract(&mut bytecode, false);
        bytecode.bytecode_version = "0.20".into();
        bytecode.provider_harnesses = vec![BytecodeProviderHarness {
            name: "OpenAIHarness".into(),
            provider: "OpenAI".into(),
            feature: None,
            secret: None,
            mode: "dry_run".into(),
            network: "denied".into(),
            secrets: "denied".into(),
            filesystem: "none".into(),
            max_steps: None,
            timeout_ms: None,
            input_contract: None,
            output_contract: None,
            attestations: vec![],
        }];
        bytecode.models[0].provider = "OpenAI".into();
        let outcome = Vm::new().run_reactive_outcome(
            &bytecode,
            InjectedMessage {
                from: "User".into(),
                to: "ResearchAgent".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
        );
        assert!(outcome.result.is_err());
        assert!(outcome
            .state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ExternalProviderExecutionBlocked));
    }
}
