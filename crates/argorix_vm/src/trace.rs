use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub vm_version: String,
    pub status: String,
    pub mode: String,
    pub scheduler: String,
    pub steps: Vec<TraceStep>,
    pub mailboxes: Vec<MailboxSummary>,
    pub events: Vec<ExecutionEvent>,
    pub security_checks: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceStep {
    pub index: usize,
    pub from: String,
    pub to: String,
    pub act: String,
    pub message_type: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxSummary {
    pub agent: String,
    pub delivered: usize,
    pub processed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    VmStarted,
    AgentDeclared,
    CapabilityDeclared,
    MessageScheduled,
    MessageDelivered,
    MessageProcessed,
    HandlerExecuted,
    MessageEmitted,
    ValueTraced,
    VmHalted,
    FacuStateCheckpoint,
    MarronCausalGuard,
    ToolDeclared,
    ToolAuthorized,
    ToolCallRequested,
    ToolCallAllowed,
    ToolCallDenied,
    ToolCallDryRunResult,
    ModelDeclared,
    ModelAuthorized,
    ModelCallRequested,
    ModelCallAllowed,
    ModelCallDenied,
    ModelCallDryRunResult,
    ProviderRegistered,
    ProviderSelected,
    ProviderRequestCreated,
    ProviderResponseReceived,
    ProviderDryRunEnforced,
    ProviderBoundaryDenied,
    ProviderContractDeclared,
    ProviderContractValidated,
    ProviderContractRejected,
    ProviderHarnessDeclared,
    ProviderHarnessValidated,
    ProviderHarnessSandboxed,
    ProviderHarnessRejected,
    FeatureDeclared,
    FeatureValidated,
    SecretBoundaryDeclared,
    SecretBoundaryValidated,
    SecretAccessDenied,
    ExternalProviderExecutionBlocked,
    AssertionDeclared,
    FailureDeclared,
    AssertionVerified,
    AssertionFailed,
    PolicyDeclared,
    PolicyEvaluated,
    PolicyViolation,
    PolicyActionActivated,
    PolicyReportGenerated,
    FailureModeActivated,
    McpBridgeContractDeclared,
    McpBridgeRuntimeDisabled,
    McpBridgeNetworkDenied,
    McpBridgeToolExecutionDisabled,
    A2ABridgeContractDeclared,
    A2ABridgeRuntimeDisabled,
    A2ABridgeNetworkDenied,
    A2AAgentExecutionDisabled,
    BridgeSecurityClaimsDenied,
    ATrustEvidenceMapDeclared,
    ATrustEvidenceMapCoverageRequired,
    ATrustEvidenceMapLinksValidated,
    ATrustEvidenceMapRuntimeDisabled,
    ATrustEvidenceMapSecurityClaimsDenied,
    GovernanceProfileDeclared,
    GovernanceControlsMapped,
    RegulatoryMappingDeclared,
    RegulatoryObligationsMapped,
    GovernanceRuntimeDisabled,
    GovernanceSecurityClaimsDenied,
    LegalCertificationDenied,
    ThirdPartyVerifierDeclared,
    PublicConformanceReportDeclared,
    PublicConformanceArtifactsMapped,
    PublicConformanceReplayDeclared,
    PublicConformanceRuntimeDisabled,
    PublicConformanceSecurityClaimsDenied,
    RemoteVerificationDenied,
    RuntimeHardeningProfileDeclared,
    RuntimeDenyByDefaultDeclared,
    RuntimeSandboxRequired,
    RuntimeNetworkDenied,
    RuntimeSecretsDenied,
    RuntimeExecutionDisabled,
    ThreatModelDeclared,
    ThreatAssetsMapped,
    ThreatsMapped,
    MitigationsMapped,
    ThreatModelRuntimeDisabled,
    ThreatModelSecurityClaimsDenied,
    SpecFreezeDeclared,
    SpecFreezeCompatibilityDeclared,
    SpecFreezeRuntimeDisabled,
    ReleaseCandidateDeclared,
    ReleaseCandidateArtifactsMapped,
    ReleaseCandidateCompatibilityMapped,
    ReleaseCandidateRuntimeDisabled,
    ReleaseCandidateSecurityClaimsDenied,
    VmCompleted,
    VmFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjectedMessage {
    pub from: String,
    pub to: String,
    pub act: String,
    pub message_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmittedMessage {
    pub message_type: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactiveStep {
    pub index: usize,
    pub agent: String,
    pub handled: String,
    pub emitted: Vec<EmittedMessage>,
    pub traced: Vec<String>,
    pub halted: bool,
    pub intrinsics: Vec<InvokedIntrinsic>,
    pub tool_calls: Vec<String>,
    pub model_calls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvokedIntrinsic {
    pub name: String,
    pub argument: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentStateSummary {
    pub agent: String,
    pub handled_count: usize,
    pub checkpoints: usize,
    pub last_message_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrinsicExecution {
    pub agent: String,
    pub name: String,
    pub argument: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallSummary {
    pub agent: String,
    pub tool: String,
    pub capability: String,
    pub status: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCallSummary {
    pub agent: String,
    pub model: String,
    pub provider: String,
    pub capability: String,
    pub status: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactiveExecutionTrace {
    pub vm_version: String,
    pub status: String,
    pub mode: String,
    pub scheduler: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<argorix_bytecode::BytecodeModule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<argorix_bytecode::BytecodeModuleImport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub message_contracts: Vec<argorix_bytecode::BytecodeType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passports: Vec<argorix_bytecode::BytecodePassport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_harnesses: Vec<argorix_bytecode::BytecodeProviderHarness>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<argorix_bytecode::BytecodeFeature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<argorix_bytecode::BytecodeSecret>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapters: Vec<argorix_bytecode::BytecodeAdapter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapter_profiles: Vec<argorix_bytecode::BytecodeAdapterProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cryptos: Vec<argorix_bytecode::BytecodeCrypto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_evidence_maps: Vec<argorix_bytecode::BytecodeATrustEvidenceMap>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub governance_profiles: Vec<argorix_bytecode::BytecodeGovernanceProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regulatory_mappings: Vec<argorix_bytecode::BytecodeRegulatoryMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub third_party_verifiers: Vec<argorix_bytecode::BytecodeThirdPartyVerifier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_conformance_reports: Vec<argorix_bytecode::BytecodePublicConformanceReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_hardening_profiles: Vec<argorix_bytecode::BytecodeRuntimeHardeningProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threat_models: Vec<argorix_bytecode::BytecodeThreatModel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spec_freezes: Vec<argorix_bytecode::BytecodeSpecFreeze>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub release_candidates: Vec<argorix_bytecode::BytecodeReleaseCandidate>,
    pub injected: InjectedMessage,
    pub steps: Vec<ReactiveStep>,
    pub mailboxes: Vec<MailboxSummary>,
    pub agent_state: Vec<AgentStateSummary>,
    pub intrinsics: Vec<IntrinsicExecution>,
    pub tool_calls: Vec<ToolCallSummary>,
    pub model_calls: Vec<ModelCallSummary>,
    pub providers: Vec<ProviderSummary>,
    pub provider_contracts: Vec<ProviderContractSummary>,
    pub provider_calls: Vec<ProviderCallSummary>,
    pub policy_report: PolicyReport,
    pub events: Vec<ExecutionEvent>,
    pub security_checks: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderContractSummary {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub dry_run_only: bool,
    pub requires_feature_flag: bool,
    pub requires_explicit_approval: bool,
    pub allowed_targets: Vec<String>,
    pub allowed_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCallSummary {
    pub provider: String,
    pub kind: String,
    pub target: String,
    pub status: String,
    pub simulated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyReport {
    pub status: String,
    #[serde(rename = "legacy_assertions", alias = "assertions")]
    pub assertions: Vec<AssertionResult>,
    #[serde(default)]
    pub policy_blocks: Vec<PolicyBlockResult>,
    #[serde(default)]
    pub actions: Vec<PolicyActionResult>,
    pub failures: Vec<FailureActivation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyBlockResult {
    pub name: String,
    pub passed: bool,
    pub status: String,
    pub require_rules: Vec<PolicyRuleResult>,
    pub deny_rules: Vec<PolicyRuleResult>,
    pub violations: Vec<PolicyViolation>,
    pub action: Option<String>,
    pub trace_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRuleResult {
    pub rule: String,
    pub effect: String,
    pub passed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyViolation {
    pub rule: String,
    pub effect: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyActionResult {
    pub policy: String,
    pub action: String,
    pub trace_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionResult {
    pub name: String,
    pub argument: Option<String>,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureActivation {
    pub name: String,
    pub action: String,
    pub trace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub index: usize,
    pub event_type: EventType,
    pub from: Option<String>,
    pub to: Option<String>,
    pub act: Option<String>,
    pub message_type: Option<String>,
    pub status: String,
    pub details: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventFields {
    pub from: Option<String>,
    pub to: Option<String>,
    pub act: Option<String>,
    pub message_type: Option<String>,
}

impl EventFields {
    pub fn message(from: &str, to: &str, act: &str, message_type: &str) -> Self {
        Self {
            from: Some(from.to_owned()),
            to: Some(to.to_owned()),
            act: Some(act.to_owned()),
            message_type: Some(message_type.to_owned()),
        }
    }

    pub fn target(to: &str) -> Self {
        Self {
            to: Some(to.to_owned()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceLedger {
    pub events: Vec<ExecutionEvent>,
}

impl TraceLedger {
    pub fn record(
        &mut self,
        event_type: EventType,
        status: impl Into<String>,
        details: impl Into<String>,
        fields: EventFields,
    ) {
        self.events.push(ExecutionEvent {
            index: self.events.len() + 1,
            event_type,
            from: fields.from,
            to: fields.to,
            act: fields.act,
            message_type: fields.message_type,
            status: status.into(),
            details: details.into(),
        });
    }
}
