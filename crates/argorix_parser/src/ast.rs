use crate::span::Spanned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub module: Spanned<String>,
    pub imports: Vec<ImportDecl>,
    pub providers: Vec<ProviderDecl>,
    pub harnesses: Vec<ProviderHarnessDecl>,
    pub features: Vec<FeatureDecl>,
    pub secrets: Vec<SecretDecl>,
    pub adapters: Vec<AdapterDecl>,
    pub adapter_profiles: Vec<AdapterProfileDecl>,
    pub cryptos: Vec<CryptoDecl>,
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
    pub feature: Option<Spanned<String>>,
    pub secret: Option<Spanned<String>>,
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

/// A top-level `feature` block declaring an experimental or future capability.
///
/// v0.21 feature flags are governance metadata only. A feature flag never makes a
/// provider executable; it records that an experimental capability exists, whether
/// it is disabled by default, and whether it requires approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureDecl {
    pub name: Spanned<String>,
    pub provider: Option<Spanned<String>>,
    pub status: Spanned<FeatureStatus>,
    pub default: Spanned<FeatureDefault>,
    pub requires_approval: bool,
    pub purpose: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureStatus {
    Experimental,
    Preview,
    Stable,
    Deprecated,
    Unknown(String),
}

impl FeatureStatus {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Experimental => "experimental",
            Self::Preview => "preview",
            Self::Stable => "stable",
            Self::Deprecated => "deprecated",
            Self::Unknown(value) => value,
        }
    }

    pub const fn is_gated(&self) -> bool {
        matches!(self, Self::Experimental | Self::Preview)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureDefault {
    Disabled,
    Enabled,
    Unknown(String),
}

impl FeatureDefault {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::Enabled => "enabled",
            Self::Unknown(value) => value,
        }
    }
}

/// A top-level `secret` block declaring a secret boundary.
///
/// v0.21 secret declarations record only the boundary metadata of a future secret:
/// its handle, scope, denied access, and `none` source. They never contain secret
/// material, never read environment variables, and never open a vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretDecl {
    pub name: Spanned<String>,
    pub handle: Spanned<String>,
    pub provider: Option<Spanned<String>>,
    pub required_by: Option<Spanned<String>>,
    pub scope: Spanned<SecretScope>,
    pub access: Spanned<SecretAccess>,
    pub source: Spanned<SecretSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretScope {
    Provider,
    Adapter,
    Model,
    Tool,
    Runtime,
    Unknown(String),
}

impl SecretScope {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Provider => "provider",
            Self::Adapter => "adapter",
            Self::Model => "model",
            Self::Tool => "tool",
            Self::Runtime => "runtime",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretAccess {
    Denied,
    Unknown(String),
}

impl SecretAccess {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Denied => "denied",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretSource {
    None,
    Unknown(String),
}

impl SecretSource {
    pub fn source_name(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Unknown(value) => value,
        }
    }
}

/// A top-level `adapter` block declaring a future or experimental integration profile.
///
/// v0.22 adapters are governance / conformance metadata only. They declare how a
/// future adapter would connect provider contracts, feature flags, secret boundaries
/// and harnesses. Adapters do not execute, do not call external systems, and do not
/// read secrets or environment variables. `simulated` remains the only executable provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterDecl {
    pub name: Spanned<String>,
    pub provider: Spanned<String>,
    pub feature: Option<Spanned<String>>,
    pub secret: Option<Spanned<String>>,
    pub harness: Option<Spanned<String>>,
    pub kind: Option<Spanned<AdapterKind>>,
    pub vendor: Option<Spanned<String>>,
    pub mode: Spanned<AdapterMode>,
    pub execution: Spanned<AdapterExecution>,
    pub network: Spanned<AdapterNetwork>,
    pub secrets: Spanned<AdapterSecrets>,
    pub filesystem: Spanned<AdapterFilesystem>,
    pub input_contract: Option<Spanned<String>>,
    pub output_contract: Option<Spanned<String>>,
    pub conformance: Vec<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterKind {
    Llm,
    Tool,
    Bridge,
    Registry,
    Identity,
    Payment,
    Storage,
    Custom,
    Unknown(String),
}

impl AdapterKind {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Llm => "llm",
            Self::Tool => "tool",
            Self::Bridge => "bridge",
            Self::Registry => "registry",
            Self::Identity => "identity",
            Self::Payment => "payment",
            Self::Storage => "storage",
            Self::Custom => "custom",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterMode {
    Experimental,
    Preview,
    Stable,
    Deprecated,
    Unknown(String),
}

impl AdapterMode {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Experimental => "experimental",
            Self::Preview => "preview",
            Self::Stable => "stable",
            Self::Deprecated => "deprecated",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterExecution {
    Disabled,
    Unknown(String),
}

impl AdapterExecution {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterNetwork {
    Denied,
    Unknown(String),
}

impl AdapterNetwork {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Denied => "denied",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterSecrets {
    Denied,
    Unknown(String),
}

impl AdapterSecrets {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Denied => "denied",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterFilesystem {
    None,
    ReadOnly,
    Unknown(String),
}

impl AdapterFilesystem {
    pub fn source_name(&self) -> &str {
        match self {
            Self::None => "none",
            Self::ReadOnly => "read_only",
            Self::Unknown(value) => value,
        }
    }
}

/// A top-level `adapter_profile` block declaring a declarative vendor/protocol profile.
///
/// v0.23 adapter profiles are governance metadata describing expected vendor integrations
/// (e.g. OpenAI, Anthropic, generic). They declare contracts, capabilities, auth style,
/// and conformance requirements. Profiles do not execute, do not call APIs, and do not
/// read secrets or environment variables. `simulated` remains the only executable provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterProfileDecl {
    pub name: Spanned<String>,
    pub adapter: Spanned<String>,
    pub provider: Spanned<String>,
    pub vendor: Spanned<String>,
    pub family: Spanned<AdapterProfileFamily>,
    pub api_style: Spanned<AdapterProfileApiStyle>,
    pub auth: Spanned<AdapterProfileAuth>,
    pub execution: Spanned<AdapterProfileExecution>,
    pub network: Spanned<AdapterProfileNetwork>,
    pub secrets: Spanned<AdapterProfileSecrets>,
    pub request_contract: Option<Spanned<String>>,
    pub response_contract: Option<Spanned<String>>,
    pub capabilities: Vec<Spanned<String>>,
    pub required_conformance: Vec<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterProfileFamily {
    Llm,
    Tool,
    Bridge,
    Registry,
    Identity,
    Payment,
    Storage,
    Custom,
    Unknown(String),
}

impl AdapterProfileFamily {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Llm => "llm",
            Self::Tool => "tool",
            Self::Bridge => "bridge",
            Self::Registry => "registry",
            Self::Identity => "identity",
            Self::Payment => "payment",
            Self::Storage => "storage",
            Self::Custom => "custom",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterProfileApiStyle {
    Responses,
    Messages,
    Chat,
    Completion,
    ToolCall,
    Mcp,
    A2a,
    Rest,
    Custom,
    Unknown(String),
}

impl AdapterProfileApiStyle {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Responses => "responses",
            Self::Messages => "messages",
            Self::Chat => "chat",
            Self::Completion => "completion",
            Self::ToolCall => "tool_call",
            Self::Mcp => "mcp",
            Self::A2a => "a2a",
            Self::Rest => "rest",
            Self::Custom => "custom",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterProfileAuth {
    None,
    SecretBoundary,
    Did,
    Credential,
    Custom,
    Unknown(String),
}

impl AdapterProfileAuth {
    pub fn source_name(&self) -> &str {
        match self {
            Self::None => "none",
            Self::SecretBoundary => "secret_boundary",
            Self::Did => "did",
            Self::Credential => "credential",
            Self::Custom => "custom",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterProfileExecution {
    Disabled,
    Unknown(String),
}

impl AdapterProfileExecution {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterProfileNetwork {
    Denied,
    Unknown(String),
}

impl AdapterProfileNetwork {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Denied => "denied",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterProfileSecrets {
    Denied,
    Unknown(String),
}

impl AdapterProfileSecrets {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Denied => "denied",
            Self::Unknown(value) => value,
        }
    }
}

/// A top-level `crypto` block declaring a cryptographic primitive as metadata only.
///
/// v0.24 crypto primitives are governance / conformance metadata. They describe
/// which algorithms (hash, signature, KEM, AEAD, etc.) are allowed, legacy,
/// deprecated, denied, experimental or post-quantum candidates. They do not
/// execute any cryptography, do not generate keys, do not sign, do not verify
/// signatures, do not encrypt, do not decrypt. `simulated` remains the only
/// executable provider. No key material or secret material is stored or read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoDecl {
    pub name: Spanned<String>,
    pub kind: Spanned<CryptoKind>,
    pub status: Spanned<CryptoStatus>,
    pub strength: Spanned<CryptoStrength>,
    pub purpose: Vec<Spanned<String>>,
    pub output_bits: Option<Spanned<u64>>,
    pub min_key_bits: Option<Spanned<u64>>,
    pub security_level: Option<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoKind {
    Hash,
    Signature,
    Kem,
    Aead,
    Mac,
    Kdf,
    Commitment,
    Randomness,
    Custom,
    Unknown(String),
}

impl CryptoKind {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Hash => "hash",
            Self::Signature => "signature",
            Self::Kem => "kem",
            Self::Aead => "aead",
            Self::Mac => "mac",
            Self::Kdf => "kdf",
            Self::Commitment => "commitment",
            Self::Randomness => "randomness",
            Self::Custom => "custom",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoStatus {
    Allowed,
    Legacy,
    Deprecated,
    Denied,
    Experimental,
    PostQuantumCandidate,
    Unknown(String),
}

impl CryptoStatus {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Allowed => "allowed",
            Self::Legacy => "legacy",
            Self::Deprecated => "deprecated",
            Self::Denied => "denied",
            Self::Experimental => "experimental",
            Self::PostQuantumCandidate => "post_quantum_candidate",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoStrength {
    Classical,
    PostQuantum,
    Hybrid,
    Unknown,
    UnknownValue(String),
}

impl CryptoStrength {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Classical => "classical",
            Self::PostQuantum => "post_quantum",
            Self::Hybrid => "hybrid",
            Self::Unknown => "unknown",
            Self::UnknownValue(value) => value,
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
    ProviderHarnessDeclared,
    ProviderHarnessSandboxed,
    ProviderNetworkDenied,
    ProviderSecretsDenied,
    ProviderFilesystemRestricted,
    ExternalProviderHarnessed,
    FeatureFlagsDeclared,
    FeaturesDefaultDisabled,
    ExperimentalFeaturesRequireApproval,
    SecretBoundariesDeclared,
    SecretAccessDenied,
    SecretValuesAbsent,
    ExternalProviderFeatureGated,
    ExternalProviderSecretBoundaryDeclared,
    AdaptersDeclared,
    AdaptersExecutionDisabled,
    AdaptersNetworkDenied,
    AdaptersSecretsDenied,
    AdaptersProviderHarnessed,
    AdaptersFeatureGated,
    AdaptersSecretBoundaried,
    AdaptersConformanceDeclared,
    AdaptersEvidenceRequired,
    AdapterProfilesDeclared,
    AdapterProfilesExecutionDisabled,
    AdapterProfilesNetworkDenied,
    AdapterProfilesSecretsDenied,
    AdapterProfilesLinked,
    AdapterProfilesConformanceDeclared,
    VendorProfilesDeclared,
    CryptoPrimitivesDeclared,
    CryptoPrimitivesAllowed,
    CryptoDeniedNotUsed,
    CryptoPostQuantumCandidatesDeclared,
    CryptoKeyMaterialAbsent,
    CryptoSecretMaterialAbsent,
    CryptoExecutionAbsent,
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
            Self::ProviderHarnessDeclared => "provider_harness_declared",
            Self::ProviderHarnessSandboxed => "provider_harness_sandboxed",
            Self::ProviderNetworkDenied => "provider_network_denied",
            Self::ProviderSecretsDenied => "provider_secrets_denied",
            Self::ProviderFilesystemRestricted => "provider_filesystem_restricted",
            Self::ExternalProviderHarnessed => "external_provider_harnessed",
            Self::FeatureFlagsDeclared => "feature_flags_declared",
            Self::FeaturesDefaultDisabled => "features_default_disabled",
            Self::ExperimentalFeaturesRequireApproval => "experimental_features_require_approval",
            Self::SecretBoundariesDeclared => "secret_boundaries_declared",
            Self::SecretAccessDenied => "secret_access_denied",
            Self::SecretValuesAbsent => "secret_values_absent",
            Self::ExternalProviderFeatureGated => "external_provider_feature_gated",
            Self::ExternalProviderSecretBoundaryDeclared => {
                "external_provider_secret_boundary_declared"
            }
            Self::AdaptersDeclared => "adapters_declared",
            Self::AdaptersExecutionDisabled => "adapters_execution_disabled",
            Self::AdaptersNetworkDenied => "adapters_network_denied",
            Self::AdaptersSecretsDenied => "adapters_secrets_denied",
            Self::AdaptersProviderHarnessed => "adapters_provider_harnessed",
            Self::AdaptersFeatureGated => "adapters_feature_gated",
            Self::AdaptersSecretBoundaried => "adapters_secret_boundaried",
            Self::AdaptersConformanceDeclared => "adapters_conformance_declared",
            Self::AdaptersEvidenceRequired => "adapters_evidence_required",
            Self::AdapterProfilesDeclared => "adapter_profiles_declared",
            Self::AdapterProfilesExecutionDisabled => "adapter_profiles_execution_disabled",
            Self::AdapterProfilesNetworkDenied => "adapter_profiles_network_denied",
            Self::AdapterProfilesSecretsDenied => "adapter_profiles_secrets_denied",
            Self::AdapterProfilesLinked => "adapter_profiles_linked",
            Self::AdapterProfilesConformanceDeclared => "adapter_profiles_conformance_declared",
            Self::VendorProfilesDeclared => "vendor_profiles_declared",
            Self::CryptoPrimitivesDeclared => "crypto_primitives_declared",
            Self::CryptoPrimitivesAllowed => "crypto_primitives_allowed",
            Self::CryptoDeniedNotUsed => "crypto_denied_not_used",
            Self::CryptoPostQuantumCandidatesDeclared => "crypto_post_quantum_candidates_declared",
            Self::CryptoKeyMaterialAbsent => "crypto_key_material_absent",
            Self::CryptoSecretMaterialAbsent => "crypto_secret_material_absent",
            Self::CryptoExecutionAbsent => "crypto_execution_absent",
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
