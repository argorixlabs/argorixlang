use crate::span::Spanned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub module: Spanned<String>,
    pub imports: Vec<ImportDecl>,
    pub providers: Vec<ProviderDecl>,
    pub harnesses: Vec<ProviderHarnessDecl>,
    pub assertions: Vec<AssertionDecl>,
    pub policies: Vec<PolicyDecl>,
    pub failures: Vec<FailureDecl>,
    pub capabilities: Vec<CapabilityDecl>,
    pub enums: Vec<EnumDecl>,
    pub types: Vec<TypeDecl>,
    pub tools: Vec<ToolDecl>,
    pub models: Vec<ModelDecl>,
    pub agents: Vec<AgentDecl>,
    pub protocols: Vec<ProtocolDecl>,
    pub passports: Vec<PassportDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDecl {
    pub path: Spanned<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKindDecl {
    Simulated,
    External,
}

impl ProviderKindDecl {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Simulated => "simulated",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDecl {
    pub name: Spanned<String>,
    pub kind: Spanned<ProviderKindDecl>,
    pub enabled: Spanned<bool>,
    pub dry_run_only: Spanned<bool>,
    pub requires_feature_flag: bool,
    pub requires_explicit_approval: bool,
    pub allowed_targets: Vec<Spanned<String>>,
    pub allowed_capabilities: Vec<Spanned<String>>,
}

/// A top-level `harness` block describing declarative provider containment.
///
/// Harnesses are metadata only. They do not make providers executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderHarnessDecl {
    pub name: Spanned<String>,
    pub provider: Spanned<String>,
    pub mode: Spanned<HarnessMode>,
    pub network: Spanned<HarnessNetwork>,
    pub secrets: Spanned<HarnessSecrets>,
    pub filesystem: Spanned<HarnessFilesystem>,
    pub max_steps: Option<Spanned<u64>>,
    pub timeout_ms: Option<Spanned<u64>>,
    pub input_contract: Option<Spanned<String>>,
    pub output_contract: Option<Spanned<String>>,
    pub attestations: Vec<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessMode {
    DryRun,
    Simulated,
    Unknown(String),
}

impl HarnessMode {
    pub fn source_name(&self) -> &str {
        match self {
            Self::DryRun => "dry_run",
            Self::Simulated => "simulated",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessNetwork {
    Denied,
    Unknown(String),
}

impl HarnessNetwork {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Denied => "denied",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessSecrets {
    Denied,
    Unknown(String),
}

impl HarnessSecrets {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Denied => "denied",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessFilesystem {
    None,
    ReadOnly,
    Unknown(String),
}

impl HarnessFilesystem {
    pub fn source_name(&self) -> &str {
        match self {
            Self::None => "none",
            Self::ReadOnly => "read_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionDecl {
    pub name: Spanned<String>,
    pub argument: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecl {
    pub name: Spanned<String>,
    pub rules: Vec<PolicyRuleDecl>,
    pub violation: Option<PolicyViolationDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyRuleDecl {
    Require { rule: Spanned<PolicyRule> },
    Deny { rule: Spanned<PolicyRule> },
}

impl PolicyRuleDecl {
    pub const fn effect(&self) -> &'static str {
        match self {
            Self::Require { .. } => "require",
            Self::Deny { .. } => "deny",
        }
    }

    pub const fn rule(&self) -> &Spanned<PolicyRule> {
        match self {
            Self::Require { rule } | Self::Deny { rule } => rule,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PolicyRule {
    NoUnhandledMessages,
    AllToolCallsTraced,
    AllModelCallsTraced,
    AllIntrinsicsTraced,
    AllProviderCallsTraced,
    HaltRequiresTrace,
    RuntimeStatusCompleted,
    ProviderContractsDeclared,
    ProviderAllowlistsValid,
    ExternalExecution,
    EvidenceBundleVerified,
    SecurityReportGenerated,
    AgentPassportDeclared,
    AgentPassportAttested,
    AgentDataResidencyDeclared,
    AgentIdentityDeclared,
    Unknown(String),
}

impl PolicyRule {
    pub fn source_name(&self) -> String {
        match self {
            Self::NoUnhandledMessages => "no_unhandled_messages",
            Self::AllToolCallsTraced => "all_tool_calls_traced",
            Self::AllModelCallsTraced => "all_model_calls_traced",
            Self::AllIntrinsicsTraced => "all_intrinsics_traced",
            Self::AllProviderCallsTraced => "all_provider_calls_traced",
            Self::HaltRequiresTrace => "halt_requires_trace",
            Self::RuntimeStatusCompleted => "runtime_status completed",
            Self::ProviderContractsDeclared => "provider_contracts_declared",
            Self::ProviderAllowlistsValid => "provider_allowlists_valid",
            Self::ExternalExecution => "external_execution",
            Self::EvidenceBundleVerified => "evidence_bundle_verified",
            Self::SecurityReportGenerated => "security_report_generated",
            Self::AgentPassportDeclared => "agent_passport_declared",
            Self::AgentPassportAttested => "agent_passport_attested",
            Self::AgentDataResidencyDeclared => "agent_data_residency_declared",
            Self::AgentIdentityDeclared => "agent_identity_declared",
            Self::Unknown(value) => value,
        }
        .to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyViolationDecl {
    pub action: Spanned<PolicyViolationAction>,
    pub trace_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyViolationAction {
    Block,
    Review,
    Warn,
    Unknown(String),
}

impl PolicyViolationAction {
    pub fn source_name(&self) -> String {
        match self {
            Self::Block => "block",
            Self::Review => "review",
            Self::Warn => "warn",
            Self::Unknown(value) => value,
        }
        .to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureDecl {
    pub name: Spanned<String>,
    pub action: Spanned<String>,
    pub trace_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDecl {
    pub name: Spanned<String>,
    pub provider: Spanned<String>,
    pub capability: Spanned<String>,
    pub input: Spanned<String>,
    pub output: Spanned<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDecl {
    pub name: Spanned<String>,
    pub provider: Option<Spanned<String>>,
    pub capability: Spanned<String>,
    pub input: Spanned<String>,
    pub output: Spanned<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityLevel {
    Safe,
    Restricted,
    Dangerous,
}

impl CapabilityLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Restricted => "restricted",
            Self::Dangerous => "dangerous",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDecl {
    pub name: Spanned<String>,
    pub level: Spanned<CapabilityLevel>,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approval {
    Granted,
    Denied,
}

impl Approval {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDecl {
    pub name: Spanned<String>,
    pub variants: Vec<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDecl {
    pub name: Spanned<String>,
    pub fields: Vec<FieldDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDecl {
    pub name: Spanned<String>,
    pub field_type: Spanned<MessageFieldType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageFieldType {
    String,
    Bool,
    Int,
    Float,
    Unknown(String),
}

impl MessageFieldType {
    pub fn source_name(&self) -> &str {
        match self {
            Self::String => "string",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Float => "float",
            Self::Unknown(value) => value,
        }
    }

    pub const fn is_primitive(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDecl {
    pub name: Spanned<String>,
    pub approval: Option<Spanned<Approval>>,
    pub receives: Vec<ReceiveDecl>,
    pub sends: Vec<SendDecl>,
    pub capabilities: Vec<Spanned<String>>,
    pub tools: Vec<Spanned<String>>,
    pub models: Vec<Spanned<String>>,
    pub handlers: Vec<HandlerDecl>,
}

impl AgentDecl {
    pub fn effective_approval(&self) -> Approval {
        self.approval
            .as_ref()
            .map(|approval| approval.value)
            .unwrap_or(Approval::Denied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerDecl {
    pub message_type: Spanned<String>,
    pub binding: Spanned<String>,
    pub instructions: Vec<HandlerInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerInstruction {
    Emit {
        message_type: Spanned<String>,
        to: Spanned<String>,
    },
    Trace {
        binding: Spanned<String>,
    },
    Halt {
        span: crate::span::Span,
    },
    IntrinsicCall {
        name: Spanned<String>,
        argument: Spanned<String>,
    },
    CallTool {
        tool: Spanned<String>,
        binding: Spanned<String>,
    },
    AskModel {
        model: Spanned<String>,
        binding: Spanned<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiveDecl {
    pub message_type: Spanned<String>,
    pub from: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendDecl {
    pub message_type: Spanned<String>,
    pub to: Spanned<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolDecl {
    pub name: Spanned<String>,
    pub steps: Vec<ProtocolStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolStep {
    pub from: Spanned<String>,
    pub to: Spanned<String>,
    pub act: Spanned<String>,
    pub message_type: Spanned<String>,
}

/// A top-level `passport` block declaring sovereign agent identity metadata.
///
/// v0.19 passports are compilable, verifiable, auditable metadata only. They
/// perform no network resolution, DID verification, or ASN lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassportDecl {
    pub name: Spanned<String>,
    pub agent: Spanned<String>,
    pub agent_name: Spanned<String>,
    pub global_id: Spanned<String>,
    pub identity: Spanned<String>,
    pub provider: Spanned<String>,
    pub version: Spanned<String>,
    pub ans_name: Option<Spanned<String>>,
    pub country: Spanned<String>,
    pub jurisdiction: Spanned<String>,
    pub data_residency: Vec<Spanned<String>>,
    pub asn: Option<PassportAsnDecl>,
    pub model: Option<Spanned<String>>,
    pub risk_level: Spanned<String>,
    pub data_scope: Vec<Spanned<String>>,
    pub intent: Spanned<String>,
    pub intended_use: Vec<Spanned<String>>,
    pub prohibited_use: Vec<Spanned<String>>,
    pub attestations: Vec<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassportAsnDecl {
    pub registry: Spanned<String>,
    pub number: Spanned<String>,
    pub holder: Spanned<String>,
    pub country: Spanned<String>,
}
