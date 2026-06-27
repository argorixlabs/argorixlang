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
    pub crypto_boundaries: Vec<CryptoBoundaryDecl>,
    pub did_methods: Vec<DidMethodDecl>,
    pub atrust_boundaries: Vec<ATrustBoundaryDecl>,
    pub atrust_identities: Vec<ATrustIdentityDecl>,
    pub atrust_credential_contracts: Vec<ATrustCredentialContractDecl>,
    pub atrust_handshakes: Vec<ATrustHandshakeDecl>,
    pub trust_ledgers: Vec<TrustLedgerDecl>,
    pub mcp_bridge_contracts: Vec<McpBridgeContractDecl>,
    pub a2a_bridge_contracts: Vec<A2ABridgeContractDecl>,
    pub atrust_evidence_maps: Vec<ATrustEvidenceMapDecl>,
    pub governance_profiles: Vec<GovernanceProfileDecl>,
    pub regulatory_mappings: Vec<RegulatoryMappingDecl>,
    pub third_party_verifiers: Vec<ThirdPartyVerifierDecl>,
    pub public_conformance_reports: Vec<PublicConformanceReportDecl>,
    pub runtime_hardening_profiles: Vec<RuntimeHardeningProfileDecl>,
    pub threat_models: Vec<ThreatModelDecl>,
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

/// A declared cryptographic trust boundary: which primitives are allowed/denied
/// across a perimeter, and whether key material, secret material, or execution
/// may cross it. Declarative only — no key material is stored or read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoBoundaryDecl {
    pub name: Spanned<String>,
    pub allowed_hashes: Vec<Spanned<String>>,
    pub allowed_signatures: Vec<Spanned<String>>,
    pub allowed_kems: Vec<Spanned<String>>,
    pub allowed_aeads: Vec<Spanned<String>>,
    pub legacy_allowed: Vec<Spanned<String>>,
    pub denied: Vec<Spanned<String>>,
    pub purpose: Vec<Spanned<String>>,
    pub min_hash_bits: Option<Spanned<u64>>,
    pub post_quantum_ready: Option<Spanned<bool>>,
    pub hybrid_allowed: Option<Spanned<bool>>,
    pub key_material: Spanned<String>,
    pub secret_material: Spanned<String>,
    pub execution: Spanned<String>,
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
    CryptoBoundariesDeclared,
    PostQuantumReadinessDeclared,
    ATrustBoundariesDeclared,
    ATrustDidMethodsDeclared,
    ATrustDidMethodAllowed,
    ATrustIdentityFormatDeclared,
    ATrustCredentialModeDeclared,
    ATrustHandshakeDisabled,
    ATrustResolutionDisabled,
    ATrustKeyMaterialDenied,
    ATrustSecretMaterialDenied,
    ATrustExecutionDisabled,
    ATrustPostQuantumReadinessDeclared,
    ATrustSecurityClaimsNone,
    ATrustIdentityDeclared,
    ATrustIdentitySubjectValid,
    ATrustIdentityDidMethodValid,
    ATrustIdentityBoundaryValid,
    ATrustIdentityStatusActive,
    ATrustIdentityValidationDryRun,
    ATrustIdentityResolutionDisabled,
    ATrustIdentityKeyMaterialDenied,
    ATrustIdentitySecretMaterialDenied,
    ATrustIdentityExecutionDisabled,
    ATrustIdentityEvidenceRequired,
    ATrustIdentitySecurityClaimsAbsent,
    ATrustIdentityPassportConsistent,
    ATrustCredentialContractDeclared,
    ATrustCredentialIssuerDidDeclared,
    ATrustCredentialHolderDidDeclared,
    ATrustCredentialTypeDeclared,
    ATrustCredentialSchemaDeclared,
    ATrustCredentialClaimsDeclared,
    ATrustCredentialVerificationDeclaredOnly,
    ATrustCredentialPresentationDisabled,
    ATrustCredentialResolutionDisabled,
    ATrustCredentialKeyMaterialDenied,
    ATrustCredentialSecretMaterialDenied,
    ATrustCredentialExecutionDisabled,
    ATrustCredentialEvidenceRequired,
    ATrustCredentialSecurityClaimsAbsent,
    ATrustHandshakeDeclared,
    ATrustHandshakeInitiatorResponderValid,
    ATrustHandshakeIdentitiesValid,
    ATrustHandshakeCredentialContractsValid,
    ATrustHandshakeBoundaryMethodValid,
    ATrustHandshakeModeDryRun,
    ATrustHandshakeDirectionValid,
    ATrustHandshakeChallengeDeclaredOnly,
    ATrustHandshakeResponseDeclaredOnly,
    ATrustHandshakeTranscriptEvidenceOnly,
    ATrustHandshakeVerificationDeclaredOnly,
    ATrustHandshakeResolutionDisabled,
    ATrustHandshakeNetworkDenied,
    ATrustHandshakeKeyMaterialDenied,
    ATrustHandshakeSecretMaterialDenied,
    ATrustHandshakeExecutionDisabled,
    ATrustHandshakeEvidenceRequired,
    ATrustHandshakeSecurityClaimsAbsent,
    TrustLedgersDeclared,
    TrustLedgerHashAlgorithmDeclared,
    TrustLedgerChainValid,
    TrustLedgerEntriesBound,
    TrustLedgerAppendOnly,
    TrustLedgerNetworkDenied,
    TrustLedgerKeyMaterialDenied,
    TrustLedgerSecretMaterialDenied,
    TrustLedgerExecutionDisabled,
    TrustLedgerEvidenceRequired,
    TrustLedgerSecurityClaimsAbsent,
    TrustLedgerBlockchainAbsent,
    TrustLedgerSignatureAbsent,
    McpBridgeContractsDeclared,
    McpBridgeAgentsBound,
    McpBridgePassportsBound,
    McpBridgeIdentitiesBound,
    McpBridgeBoundariesBound,
    McpBridgeNetworkDenied,
    McpBridgeExternalExecutionDisabled,
    McpBridgeToolExecutionDisabled,
    McpBridgeSecretMaterialDenied,
    McpBridgeKeyMaterialDenied,
    McpBridgeAuthenticationNonSecret,
    McpBridgeSecurityClaimsAbsent,
    A2ABridgeContractsDeclared,
    A2ABridgeAgentsBound,
    A2ABridgePassportsBound,
    A2ABridgeIdentitiesBound,
    A2ABridgeHandshakesBound,
    A2ABridgeTrustLedgersBound,
    A2ABridgeMessageContractsBound,
    A2ABridgeNetworkDenied,
    A2ABridgeExternalExecutionDisabled,
    A2ABridgeAgentExecutionDisabled,
    A2ABridgeSecretMaterialDenied,
    A2ABridgeKeyMaterialDenied,
    A2ABridgeAuthenticationNonSecret,
    A2ABridgeSecurityClaimsAbsent,
    ATrustEvidenceMapsDeclared,
    ATrustEvidenceMapAgentsBound,
    ATrustEvidenceMapPassportsBound,
    ATrustEvidenceMapIdentitiesBound,
    ATrustEvidenceMapCredentialsBound,
    ATrustEvidenceMapHandshakesBound,
    ATrustEvidenceMapLedgersBound,
    ATrustEvidenceMapBridgesBound,
    ATrustEvidenceMapPoliciesBound,
    ATrustEvidenceMapCoverageRequired,
    ATrustEvidenceMapVerificationNonVerifying,
    ATrustEvidenceMapResolutionDisabled,
    ATrustEvidenceMapNetworkDenied,
    ATrustEvidenceMapExternalExecutionDisabled,
    ATrustEvidenceMapSecretMaterialDenied,
    ATrustEvidenceMapKeyMaterialDenied,
    ATrustEvidenceMapExecutionDisabled,
    ATrustEvidenceMapSecurityClaimsAbsent,
    GovernanceProfilesDeclared,
    GovernanceProfilesEvidenceBound,
    GovernanceProfilesControlsMapped,
    GovernanceProfilesRuntimeDisabled,
    GovernanceProfilesSecurityClaimsAbsent,
    GovernanceProfilesNoLegalCertification,
    RegulatoryMappingsDeclared,
    RegulatoryMappingsProfilesBound,
    RegulatoryMappingsObligationsMapped,
    RegulatoryMappingsControlsBound,
    RegulatoryMappingsLegalClaimsAbsent,
    RegulatoryMappingsCertificationAbsent,
    RegulatoryMappingsRuntimeDisabled,
    RegulatoryMappingsSecurityClaimsAbsent,
    ThirdPartyVerifiersDeclared,
    ThirdPartyVerifiersIdentityDeclared,
    ThirdPartyVerifiersScopeBounded,
    ThirdPartyVerifiersRuntimeDisabled,
    ThirdPartyVerifiersLegalClaimsAbsent,
    ThirdPartyVerifiersCertificationAbsent,
    ThirdPartyVerifiersSecurityClaimsAbsent,
    PublicConformanceReportsDeclared,
    PublicConformanceReportsVerifiersBound,
    PublicConformanceReportsArtifactsDeclared,
    PublicConformanceReportsEvidenceBound,
    PublicConformanceReportsGovernanceBound,
    PublicConformanceReportsRegulatoryBound,
    PublicConformanceReportsReplayable,
    PublicConformanceReportsRuntimeDisabled,
    PublicConformanceReportsLegalClaimsAbsent,
    PublicConformanceReportsCertificationAbsent,
    PublicConformanceReportsSecurityClaimsAbsent,
    RuntimeHardeningProfilesDeclared,
    RuntimeHardeningEvidenceBound,
    RuntimeHardeningDenyByDefault,
    RuntimeHardeningSandboxRequired,
    RuntimeHardeningNetworkDenied,
    RuntimeHardeningExternalProvidersDisabled,
    RuntimeHardeningToolExecutionDisabled,
    RuntimeHardeningAgentExecutionDisabled,
    RuntimeHardeningFilesystemDenied,
    RuntimeHardeningEnvDenied,
    RuntimeHardeningSecretMaterialDenied,
    RuntimeHardeningKeyMaterialDenied,
    RuntimeHardeningAuditLogRequired,
    RuntimeHardeningSecurityClaimsAbsent,
    ThreatModelsDeclared,
    ThreatModelsHardeningBound,
    ThreatModelsAssetsMapped,
    ThreatModelsThreatsMapped,
    ThreatModelsMitigationsMapped,
    ThreatModelsRuntimeDisabled,
    ThreatModelsNetworkDenied,
    ThreatModelsSecretMaterialDenied,
    ThreatModelsKeyMaterialDenied,
    ThreatModelsExecutionDisabled,
    ThreatModelsSecurityClaimsAbsent,
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
            Self::CryptoBoundariesDeclared => "crypto_boundaries_declared",
            Self::PostQuantumReadinessDeclared => "post_quantum_readiness_declared",
            Self::ATrustBoundariesDeclared => "atrust_boundaries_declared",
            Self::ATrustDidMethodsDeclared => "atrust_did_methods_declared",
            Self::ATrustDidMethodAllowed => "atrust_did_method_allowed",
            Self::ATrustIdentityFormatDeclared => "atrust_identity_format_declared",
            Self::ATrustCredentialModeDeclared => "atrust_credential_mode_declared",
            Self::ATrustHandshakeDisabled => "atrust_handshake_disabled",
            Self::ATrustResolutionDisabled => "atrust_resolution_disabled",
            Self::ATrustKeyMaterialDenied => "atrust_key_material_denied",
            Self::ATrustSecretMaterialDenied => "atrust_secret_material_denied",
            Self::ATrustExecutionDisabled => "atrust_execution_disabled",
            Self::ATrustPostQuantumReadinessDeclared => "atrust_post_quantum_readiness_declared",
            Self::ATrustSecurityClaimsNone => "atrust_security_claims_none",
            Self::ATrustIdentityDeclared => "atrust_identity_declared",
            Self::ATrustIdentitySubjectValid => "atrust_identity_subject_valid",
            Self::ATrustIdentityDidMethodValid => "atrust_identity_did_method_valid",
            Self::ATrustIdentityBoundaryValid => "atrust_identity_boundary_valid",
            Self::ATrustIdentityStatusActive => "atrust_identity_status_active",
            Self::ATrustIdentityValidationDryRun => "atrust_identity_validation_dry_run",
            Self::ATrustIdentityResolutionDisabled => "atrust_identity_resolution_disabled",
            Self::ATrustIdentityKeyMaterialDenied => "atrust_identity_key_material_denied",
            Self::ATrustIdentitySecretMaterialDenied => "atrust_identity_secret_material_denied",
            Self::ATrustIdentityExecutionDisabled => "atrust_identity_execution_disabled",
            Self::ATrustIdentityEvidenceRequired => "atrust_identity_evidence_required",
            Self::ATrustIdentitySecurityClaimsAbsent => "atrust_identity_security_claims_absent",
            Self::ATrustIdentityPassportConsistent => "atrust_identity_passport_consistent",
            Self::ATrustCredentialContractDeclared => "atrust_credential_contract_declared",
            Self::ATrustCredentialIssuerDidDeclared => "atrust_credential_issuer_did_declared",
            Self::ATrustCredentialHolderDidDeclared => "atrust_credential_holder_did_declared",
            Self::ATrustCredentialTypeDeclared => "atrust_credential_type_declared",
            Self::ATrustCredentialSchemaDeclared => "atrust_credential_schema_declared",
            Self::ATrustCredentialClaimsDeclared => "atrust_credential_claims_declared",
            Self::ATrustCredentialVerificationDeclaredOnly => {
                "atrust_credential_verification_declared_only"
            }
            Self::ATrustCredentialPresentationDisabled => "atrust_credential_presentation_disabled",
            Self::ATrustCredentialResolutionDisabled => "atrust_credential_resolution_disabled",
            Self::ATrustCredentialKeyMaterialDenied => "atrust_credential_key_material_denied",
            Self::ATrustCredentialSecretMaterialDenied => {
                "atrust_credential_secret_material_denied"
            }
            Self::ATrustCredentialExecutionDisabled => "atrust_credential_execution_disabled",
            Self::ATrustCredentialEvidenceRequired => "atrust_credential_evidence_required",
            Self::ATrustCredentialSecurityClaimsAbsent => {
                "atrust_credential_security_claims_absent"
            }
            Self::ATrustHandshakeDeclared => "atrust_handshake_declared",
            Self::ATrustHandshakeInitiatorResponderValid => {
                "atrust_handshake_initiator_responder_valid"
            }
            Self::ATrustHandshakeIdentitiesValid => "atrust_handshake_identities_valid",
            Self::ATrustHandshakeCredentialContractsValid => {
                "atrust_handshake_credential_contracts_valid"
            }
            Self::ATrustHandshakeBoundaryMethodValid => "atrust_handshake_boundary_method_valid",
            Self::ATrustHandshakeModeDryRun => "atrust_handshake_mode_dry_run",
            Self::ATrustHandshakeDirectionValid => "atrust_handshake_direction_valid",
            Self::ATrustHandshakeChallengeDeclaredOnly => {
                "atrust_handshake_challenge_declared_only"
            }
            Self::ATrustHandshakeResponseDeclaredOnly => "atrust_handshake_response_declared_only",
            Self::ATrustHandshakeTranscriptEvidenceOnly => {
                "atrust_handshake_transcript_evidence_only"
            }
            Self::ATrustHandshakeVerificationDeclaredOnly => {
                "atrust_handshake_verification_declared_only"
            }
            Self::ATrustHandshakeResolutionDisabled => "atrust_handshake_resolution_disabled",
            Self::ATrustHandshakeNetworkDenied => "atrust_handshake_network_denied",
            Self::ATrustHandshakeKeyMaterialDenied => "atrust_handshake_key_material_denied",
            Self::ATrustHandshakeSecretMaterialDenied => "atrust_handshake_secret_material_denied",
            Self::ATrustHandshakeExecutionDisabled => "atrust_handshake_execution_disabled",
            Self::ATrustHandshakeEvidenceRequired => "atrust_handshake_evidence_required",
            Self::ATrustHandshakeSecurityClaimsAbsent => "atrust_handshake_security_claims_absent",
            Self::TrustLedgersDeclared => "trust_ledgers_declared",
            Self::TrustLedgerHashAlgorithmDeclared => "trust_ledger_hash_algorithm_declared",
            Self::TrustLedgerChainValid => "trust_ledger_chain_valid",
            Self::TrustLedgerEntriesBound => "trust_ledger_entries_bound",
            Self::TrustLedgerAppendOnly => "trust_ledger_append_only",
            Self::TrustLedgerNetworkDenied => "trust_ledger_network_denied",
            Self::TrustLedgerKeyMaterialDenied => "trust_ledger_key_material_denied",
            Self::TrustLedgerSecretMaterialDenied => "trust_ledger_secret_material_denied",
            Self::TrustLedgerExecutionDisabled => "trust_ledger_execution_disabled",
            Self::TrustLedgerEvidenceRequired => "trust_ledger_evidence_required",
            Self::TrustLedgerSecurityClaimsAbsent => "trust_ledger_security_claims_absent",
            Self::TrustLedgerBlockchainAbsent => "trust_ledger_blockchain_absent",
            Self::TrustLedgerSignatureAbsent => "trust_ledger_signature_absent",
            Self::McpBridgeContractsDeclared => "mcp_bridge_contracts_declared",
            Self::McpBridgeAgentsBound => "mcp_bridge_agents_bound",
            Self::McpBridgePassportsBound => "mcp_bridge_passports_bound",
            Self::McpBridgeIdentitiesBound => "mcp_bridge_identities_bound",
            Self::McpBridgeBoundariesBound => "mcp_bridge_boundaries_bound",
            Self::McpBridgeNetworkDenied => "mcp_bridge_network_denied",
            Self::McpBridgeExternalExecutionDisabled => "mcp_bridge_external_execution_disabled",
            Self::McpBridgeToolExecutionDisabled => "mcp_bridge_tool_execution_disabled",
            Self::McpBridgeSecretMaterialDenied => "mcp_bridge_secret_material_denied",
            Self::McpBridgeKeyMaterialDenied => "mcp_bridge_key_material_denied",
            Self::McpBridgeAuthenticationNonSecret => "mcp_bridge_authentication_non_secret",
            Self::McpBridgeSecurityClaimsAbsent => "mcp_bridge_security_claims_absent",
            Self::A2ABridgeContractsDeclared => "a2a_bridge_contracts_declared",
            Self::A2ABridgeAgentsBound => "a2a_bridge_agents_bound",
            Self::A2ABridgePassportsBound => "a2a_bridge_passports_bound",
            Self::A2ABridgeIdentitiesBound => "a2a_bridge_identities_bound",
            Self::A2ABridgeHandshakesBound => "a2a_bridge_handshakes_bound",
            Self::A2ABridgeTrustLedgersBound => "a2a_bridge_trust_ledgers_bound",
            Self::A2ABridgeMessageContractsBound => "a2a_bridge_message_contracts_bound",
            Self::A2ABridgeNetworkDenied => "a2a_bridge_network_denied",
            Self::A2ABridgeExternalExecutionDisabled => "a2a_bridge_external_execution_disabled",
            Self::A2ABridgeAgentExecutionDisabled => "a2a_bridge_agent_execution_disabled",
            Self::A2ABridgeSecretMaterialDenied => "a2a_bridge_secret_material_denied",
            Self::A2ABridgeKeyMaterialDenied => "a2a_bridge_key_material_denied",
            Self::A2ABridgeAuthenticationNonSecret => "a2a_bridge_authentication_non_secret",
            Self::A2ABridgeSecurityClaimsAbsent => "a2a_bridge_security_claims_absent",
            Self::ATrustEvidenceMapsDeclared => "atrust_evidence_maps_declared",
            Self::ATrustEvidenceMapAgentsBound => "atrust_evidence_map_agents_bound",
            Self::ATrustEvidenceMapPassportsBound => "atrust_evidence_map_passports_bound",
            Self::ATrustEvidenceMapIdentitiesBound => "atrust_evidence_map_identities_bound",
            Self::ATrustEvidenceMapCredentialsBound => "atrust_evidence_map_credentials_bound",
            Self::ATrustEvidenceMapHandshakesBound => "atrust_evidence_map_handshakes_bound",
            Self::ATrustEvidenceMapLedgersBound => "atrust_evidence_map_ledgers_bound",
            Self::ATrustEvidenceMapBridgesBound => "atrust_evidence_map_bridges_bound",
            Self::ATrustEvidenceMapPoliciesBound => "atrust_evidence_map_policies_bound",
            Self::ATrustEvidenceMapCoverageRequired => "atrust_evidence_map_coverage_required",
            Self::ATrustEvidenceMapVerificationNonVerifying => {
                "atrust_evidence_map_verification_non_verifying"
            }
            Self::ATrustEvidenceMapResolutionDisabled => "atrust_evidence_map_resolution_disabled",
            Self::ATrustEvidenceMapNetworkDenied => "atrust_evidence_map_network_denied",
            Self::ATrustEvidenceMapExternalExecutionDisabled => {
                "atrust_evidence_map_external_execution_disabled"
            }
            Self::ATrustEvidenceMapSecretMaterialDenied => {
                "atrust_evidence_map_secret_material_denied"
            }
            Self::ATrustEvidenceMapKeyMaterialDenied => "atrust_evidence_map_key_material_denied",
            Self::ATrustEvidenceMapExecutionDisabled => "atrust_evidence_map_execution_disabled",
            Self::ATrustEvidenceMapSecurityClaimsAbsent => {
                "atrust_evidence_map_security_claims_absent"
            }
            Self::GovernanceProfilesDeclared => "governance_profiles_declared",
            Self::GovernanceProfilesEvidenceBound => "governance_profiles_evidence_bound",
            Self::GovernanceProfilesControlsMapped => "governance_profiles_controls_mapped",
            Self::GovernanceProfilesRuntimeDisabled => "governance_profiles_runtime_disabled",
            Self::GovernanceProfilesSecurityClaimsAbsent => {
                "governance_profiles_security_claims_absent"
            }
            Self::GovernanceProfilesNoLegalCertification => {
                "governance_profiles_no_legal_certification"
            }
            Self::RegulatoryMappingsDeclared => "regulatory_mappings_declared",
            Self::RegulatoryMappingsProfilesBound => "regulatory_mappings_profiles_bound",
            Self::RegulatoryMappingsObligationsMapped => "regulatory_mappings_obligations_mapped",
            Self::RegulatoryMappingsControlsBound => "regulatory_mappings_controls_bound",
            Self::RegulatoryMappingsLegalClaimsAbsent => "regulatory_mappings_legal_claims_absent",
            Self::RegulatoryMappingsCertificationAbsent => {
                "regulatory_mappings_certification_absent"
            }
            Self::RegulatoryMappingsRuntimeDisabled => "regulatory_mappings_runtime_disabled",
            Self::RegulatoryMappingsSecurityClaimsAbsent => {
                "regulatory_mappings_security_claims_absent"
            }
            Self::ThirdPartyVerifiersDeclared => "third_party_verifiers_declared",
            Self::ThirdPartyVerifiersIdentityDeclared => "third_party_verifiers_identity_declared",
            Self::ThirdPartyVerifiersScopeBounded => "third_party_verifiers_scope_bounded",
            Self::ThirdPartyVerifiersRuntimeDisabled => "third_party_verifiers_runtime_disabled",
            Self::ThirdPartyVerifiersLegalClaimsAbsent => {
                "third_party_verifiers_legal_claims_absent"
            }
            Self::ThirdPartyVerifiersCertificationAbsent => {
                "third_party_verifiers_certification_absent"
            }
            Self::ThirdPartyVerifiersSecurityClaimsAbsent => {
                "third_party_verifiers_security_claims_absent"
            }
            Self::PublicConformanceReportsDeclared => "public_conformance_reports_declared",
            Self::PublicConformanceReportsVerifiersBound => {
                "public_conformance_reports_verifiers_bound"
            }
            Self::PublicConformanceReportsArtifactsDeclared => {
                "public_conformance_reports_artifacts_declared"
            }
            Self::PublicConformanceReportsEvidenceBound => {
                "public_conformance_reports_evidence_bound"
            }
            Self::PublicConformanceReportsGovernanceBound => {
                "public_conformance_reports_governance_bound"
            }
            Self::PublicConformanceReportsRegulatoryBound => {
                "public_conformance_reports_regulatory_bound"
            }
            Self::PublicConformanceReportsReplayable => "public_conformance_reports_replayable",
            Self::PublicConformanceReportsRuntimeDisabled => {
                "public_conformance_reports_runtime_disabled"
            }
            Self::PublicConformanceReportsLegalClaimsAbsent => {
                "public_conformance_reports_legal_claims_absent"
            }
            Self::PublicConformanceReportsCertificationAbsent => {
                "public_conformance_reports_certification_absent"
            }
            Self::PublicConformanceReportsSecurityClaimsAbsent => {
                "public_conformance_reports_security_claims_absent"
            }
            Self::RuntimeHardeningProfilesDeclared => "runtime_hardening_profiles_declared",
            Self::RuntimeHardeningEvidenceBound => "runtime_hardening_evidence_bound",
            Self::RuntimeHardeningDenyByDefault => "runtime_hardening_deny_by_default",
            Self::RuntimeHardeningSandboxRequired => "runtime_hardening_sandbox_required",
            Self::RuntimeHardeningNetworkDenied => "runtime_hardening_network_denied",
            Self::RuntimeHardeningExternalProvidersDisabled => {
                "runtime_hardening_external_providers_disabled"
            }
            Self::RuntimeHardeningToolExecutionDisabled => {
                "runtime_hardening_tool_execution_disabled"
            }
            Self::RuntimeHardeningAgentExecutionDisabled => {
                "runtime_hardening_agent_execution_disabled"
            }
            Self::RuntimeHardeningFilesystemDenied => "runtime_hardening_filesystem_denied",
            Self::RuntimeHardeningEnvDenied => "runtime_hardening_env_denied",
            Self::RuntimeHardeningSecretMaterialDenied => {
                "runtime_hardening_secret_material_denied"
            }
            Self::RuntimeHardeningKeyMaterialDenied => "runtime_hardening_key_material_denied",
            Self::RuntimeHardeningAuditLogRequired => "runtime_hardening_audit_log_required",
            Self::RuntimeHardeningSecurityClaimsAbsent => {
                "runtime_hardening_security_claims_absent"
            }
            Self::ThreatModelsDeclared => "threat_models_declared",
            Self::ThreatModelsHardeningBound => "threat_models_hardening_bound",
            Self::ThreatModelsAssetsMapped => "threat_models_assets_mapped",
            Self::ThreatModelsThreatsMapped => "threat_models_threats_mapped",
            Self::ThreatModelsMitigationsMapped => "threat_models_mitigations_mapped",
            Self::ThreatModelsRuntimeDisabled => "threat_models_runtime_disabled",
            Self::ThreatModelsNetworkDenied => "threat_models_network_denied",
            Self::ThreatModelsSecretMaterialDenied => "threat_models_secret_material_denied",
            Self::ThreatModelsKeyMaterialDenied => "threat_models_key_material_denied",
            Self::ThreatModelsExecutionDisabled => "threat_models_execution_disabled",
            Self::ThreatModelsSecurityClaimsAbsent => "threat_models_security_claims_absent",
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

/// A top-level `did_method` block describing a DID method that may be referenced
/// by ATrust boundaries. Declarative only — no DID resolution is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DidMethodDecl {
    pub name: Spanned<String>,
    pub status: Spanned<DidMethodStatus>,
    pub resolution: Spanned<DidResolutionMode>,
    pub ledger: Spanned<DidLedgerMode>,
    pub crypto_boundary: Spanned<String>,
    pub governance: Option<Spanned<String>>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DidMethodStatus {
    Experimental,
    Preview,
    Stable,
    Deprecated,
    Denied,
    Unknown(String),
}

impl DidMethodStatus {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Experimental => "experimental",
            Self::Preview => "preview",
            Self::Stable => "stable",
            Self::Deprecated => "deprecated",
            Self::Denied => "denied",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DidResolutionMode {
    Disabled,
    Embedded,
    Local,
    Unknown(String),
}

impl DidResolutionMode {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::Embedded => "embedded",
            Self::Local => "local",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DidLedgerMode {
    None,
    Local,
    Embedded,
    Custom,
    Unknown(String),
}

impl DidLedgerMode {
    pub fn source_name(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Local => "local",
            Self::Embedded => "embedded",
            Self::Custom => "custom",
            Self::Unknown(value) => value,
        }
    }
}

/// A top-level `atrust_boundary` block declaring an ATrust boundary contract.
/// Metadata only — no identity resolution, credential verification, handshake,
/// signing, or key operations are performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ATrustBoundaryDecl {
    pub name: Spanned<String>,
    pub crypto_boundary: Spanned<String>,
    pub did_methods: Vec<Spanned<String>>,
    pub identity_format: Spanned<ATrustIdentityFormat>,
    pub credential_mode: Spanned<ATrustCredentialMode>,
    pub handshake: Spanned<ATrustHandshakeMode>,
    pub resolution: Spanned<ATrustResolutionMode>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub execution: Spanned<ATrustExecution>,
    pub post_quantum_ready: Option<Spanned<ATrustPostQuantumRequirement>>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustIdentityFormat {
    Did,
    Opaque,
    Custom,
    Unknown(String),
}

impl ATrustIdentityFormat {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Did => "did",
            Self::Opaque => "opaque",
            Self::Custom => "custom",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustCredentialMode {
    Disabled,
    DeclaredOnly,
    Unknown(String),
}

impl ATrustCredentialMode {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustHandshakeMode {
    Disabled,
    DeclaredOnly,
    Unknown(String),
}

impl ATrustHandshakeMode {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustResolutionMode {
    Disabled,
    Embedded,
    Local,
    Unknown(String),
}

impl ATrustResolutionMode {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::Embedded => "embedded",
            Self::Local => "local",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustMaterialBoundary {
    Denied,
    Unknown(String),
}

impl ATrustMaterialBoundary {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Denied => "denied",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustExecution {
    Disabled,
    Unknown(String),
}

impl ATrustExecution {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustPostQuantumRequirement {
    Required,
    Optional,
    NotRequired,
    Unknown(String),
}

impl ATrustPostQuantumRequirement {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Required => "required",
            Self::Optional => "optional",
            Self::NotRequired => "not_required",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustSecurityClaims {
    None,
    Unknown(String),
}

impl ATrustSecurityClaims {
    pub fn source_name(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Unknown(value) => value,
        }
    }
}

/// A top-level `atrust_identity` block declaring a dry-run ATrust identity for an agent.
/// v0.27 is simulation + evidence only. No real DID resolution, VC verification,
/// signing, or network access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ATrustIdentityDecl {
    pub name: Spanned<String>,
    pub subject: Spanned<String>,
    pub did: Spanned<String>,
    pub method: Spanned<String>,
    pub boundary: Spanned<String>,
    pub status: Spanned<ATrustIdentityStatus>,
    pub validation: Spanned<ATrustIdentityValidation>,
    pub resolution: Spanned<ATrustResolutionMode>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub execution: Spanned<ATrustExecution>,
    pub evidence: Spanned<ATrustEvidenceRequirement>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustIdentityStatus {
    Active,
    Suspended,
    Revoked,
    Unknown(String),
}

impl ATrustIdentityStatus {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Revoked => "revoked",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustIdentityValidation {
    DryRun,
    Unknown(String),
}

impl ATrustIdentityValidation {
    pub fn source_name(&self) -> &str {
        match self {
            Self::DryRun => "dry_run",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustEvidenceRequirement {
    Required,
    Unknown(String),
}

impl ATrustEvidenceRequirement {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Required => "required",
            Self::Unknown(value) => value,
        }
    }
}

/// A top-level `atrust_credential_contract` block declaring an ATrust credential contract.
/// v0.28 is credential contract metadata only. No VC parsing, no credential verification,
/// no signing, no DID resolution, no network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ATrustCredentialContractDecl {
    pub name: Spanned<String>,
    pub subject: Spanned<String>,
    pub identity: Spanned<String>,
    pub boundary: Spanned<String>,
    pub method: Spanned<String>,
    pub issuer_did: Spanned<String>,
    pub holder_did: Spanned<String>,
    pub credential_type: Spanned<String>,
    pub schema: Spanned<String>,
    pub status: Spanned<ATrustCredentialStatus>,
    pub verification: Spanned<ATrustCredentialVerification>,
    pub presentation: Spanned<ATrustCredentialPresentation>,
    pub resolution: Spanned<ATrustResolutionMode>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub execution: Spanned<ATrustExecution>,
    pub evidence: Spanned<ATrustEvidenceRequirement>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub claims: Vec<Spanned<String>>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustCredentialStatus {
    Declared,
    Active,
    Suspended,
    Revoked,
    Unknown(String),
}

impl ATrustCredentialStatus {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Declared => "declared",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Revoked => "revoked",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustCredentialVerification {
    DeclaredOnly,
    Unknown(String),
}

impl ATrustCredentialVerification {
    pub fn source_name(&self) -> &str {
        match self {
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustCredentialPresentation {
    Disabled,
    DeclaredOnly,
    Unknown(String),
}

impl ATrustCredentialPresentation {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}

/// A top-level `atrust_handshake` block declaring an ATrust handshake dry-run contract.
/// v0.29 is handshake dry-run metadata + evidence only. No real crypto, network, nonces,
/// signatures, or live challenge-response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ATrustHandshakeDecl {
    pub name: Spanned<String>,
    pub initiator: Spanned<String>,
    pub responder: Spanned<String>,
    pub initiator_identity: Spanned<String>,
    pub responder_identity: Spanned<String>,
    pub credential_contracts: Vec<Spanned<String>>,
    pub boundary: Spanned<String>,
    pub method: Spanned<String>,
    pub mode: Spanned<ATrustHandshakeDryRunMode>,
    pub direction: Spanned<ATrustHandshakeDirection>,
    pub challenge: Spanned<ATrustHandshakeChallenge>,
    pub response: Spanned<ATrustHandshakeResponse>,
    pub transcript: Spanned<ATrustHandshakeTranscript>,
    pub verification: Spanned<ATrustHandshakeVerification>,
    pub resolution: Spanned<ATrustResolutionMode>,
    pub network: Spanned<ATrustNetworkBoundary>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub execution: Spanned<ATrustExecution>,
    pub evidence: Spanned<ATrustEvidenceRequirement>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustHandshakeDryRunMode {
    DryRun,
    Unknown(String),
}

impl ATrustHandshakeDryRunMode {
    pub fn source_name(&self) -> &str {
        match self {
            Self::DryRun => "dry_run",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustHandshakeDirection {
    OneWay,
    Mutual,
    Unknown(String),
}

impl ATrustHandshakeDirection {
    pub fn source_name(&self) -> &str {
        match self {
            Self::OneWay => "one_way",
            Self::Mutual => "mutual",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustHandshakeChallenge {
    Disabled,
    DeclaredOnly,
    Unknown(String),
}

impl ATrustHandshakeChallenge {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustHandshakeResponse {
    Disabled,
    DeclaredOnly,
    Unknown(String),
}

impl ATrustHandshakeResponse {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustHandshakeTranscript {
    MetadataOnly,
    EvidenceOnly,
    Unknown(String),
}

impl ATrustHandshakeTranscript {
    pub fn source_name(&self) -> &str {
        match self {
            Self::MetadataOnly => "metadata_only",
            Self::EvidenceOnly => "evidence_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustHandshakeVerification {
    Disabled,
    DeclaredOnly,
    Unknown(String),
}

impl ATrustHandshakeVerification {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Disabled => "disabled",
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustNetworkBoundary {
    Denied,
    Unknown(String),
}

impl ATrustNetworkBoundary {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Denied => "denied",
            Self::Unknown(value) => value,
        }
    }
}

/// A top-level `trust_ledger` block declaring a Trust Ledger Hash Chain.
/// v0.30 is hash-chain metadata + evidence only. It is NOT a blockchain: no
/// consensus, mining, signing, signature verification, network, or key/secret
/// handling. It preserves an ordered, auditable relation of trust evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustLedgerDecl {
    pub name: Spanned<String>,
    pub scope: Spanned<TrustLedgerScope>,
    pub mode: Spanned<TrustLedgerMode>,
    pub hash_algorithm: Spanned<String>,
    pub chain_policy: Spanned<TrustLedgerChainPolicy>,
    pub entries: Vec<TrustLedgerEntryDecl>,
    pub chain_root: Spanned<String>,
    pub network: Spanned<ATrustNetworkBoundary>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub execution: Spanned<ATrustExecution>,
    pub evidence: Spanned<ATrustEvidenceRequirement>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustLedgerEntryDecl {
    pub id: Spanned<String>,
    pub kind: Spanned<TrustLedgerEntryKind>,
    pub subject: Spanned<String>,
    pub previous_hash: Spanned<String>,
    pub entry_hash: Spanned<String>,
    pub evidence_ref: Spanned<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustLedgerScope {
    Local,
    Package,
    Bundle,
    Unknown(String),
}

impl TrustLedgerScope {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Local => "local",
            Self::Package => "package",
            Self::Bundle => "bundle",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustLedgerMode {
    DryRun,
    DeclaredOnly,
    Unknown(String),
}

impl TrustLedgerMode {
    pub fn source_name(&self) -> &str {
        match self {
            Self::DryRun => "dry_run",
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustLedgerChainPolicy {
    AppendOnly,
    DeclaredOnly,
    Unknown(String),
}

impl TrustLedgerChainPolicy {
    pub fn source_name(&self) -> &str {
        match self {
            Self::AppendOnly => "append_only",
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustLedgerEntryKind {
    Identity,
    Credential,
    Handshake,
    Evidence,
    Policy,
    Custom,
    Unknown(String),
}

impl TrustLedgerEntryKind {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Identity => "identity",
            Self::Credential => "credential",
            Self::Handshake => "handshake",
            Self::Evidence => "evidence",
            Self::Policy => "policy",
            Self::Custom => "custom",
            Self::Unknown(value) => value,
        }
    }
}

/// A top-level `mcp_bridge_contract` block declaring an MCP interoperability
/// surface. v0.31 is bridge-contract metadata + evidence only. It declares how
/// an agent *could* interoperate with MCP tools/resources/prompts; it does NOT
/// open network access, start an MCP server, execute tools, or read secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpBridgeContractDecl {
    pub name: Spanned<String>,
    pub agent: Spanned<String>,
    pub passport: Spanned<String>,
    pub identity: Spanned<String>,
    pub boundary: Spanned<String>,
    pub transport: Spanned<BridgeTransport>,
    pub protocol: Spanned<McpProtocol>,
    pub direction: Spanned<BridgeDirection>,
    pub tools: Vec<Spanned<String>>,
    pub resources: Vec<Spanned<String>>,
    pub prompts: Vec<Spanned<String>>,
    pub network: Spanned<ATrustNetworkBoundary>,
    pub external_execution: Spanned<ATrustExecution>,
    pub tool_execution: Spanned<ATrustExecution>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub authentication: Spanned<BridgeAuthentication>,
    pub authorization: Spanned<BridgeAuthorization>,
    pub evidence: Spanned<ATrustEvidenceRequirement>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

/// A top-level `a2a_bridge_contract` block declaring an agent-to-agent
/// interoperability surface. v0.31 is bridge-contract metadata + evidence only.
/// It declares how two agents *could* interoperate; it does NOT open network
/// access, execute agents, send messages, or read secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2ABridgeContractDecl {
    pub name: Spanned<String>,
    pub initiator: Spanned<String>,
    pub responder: Spanned<String>,
    pub initiator_passport: Spanned<String>,
    pub responder_passport: Spanned<String>,
    pub initiator_identity: Spanned<String>,
    pub responder_identity: Spanned<String>,
    pub handshake: Spanned<String>,
    pub trust_ledger: Spanned<String>,
    pub boundary: Spanned<String>,
    pub protocol: Spanned<A2AProtocol>,
    pub transport: Spanned<BridgeTransport>,
    pub direction: Spanned<BridgeDirection>,
    pub message_contracts: Vec<Spanned<String>>,
    pub capabilities: Vec<Spanned<String>>,
    pub network: Spanned<ATrustNetworkBoundary>,
    pub external_execution: Spanned<ATrustExecution>,
    pub agent_execution: Spanned<ATrustExecution>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub authentication: Spanned<BridgeAuthentication>,
    pub authorization: Spanned<BridgeAuthorization>,
    pub evidence: Spanned<ATrustEvidenceRequirement>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

/// A top-level `atrust_evidence_map` block linking ATrust identity,
/// credential, handshake, ledger and bridge metadata. v0.32 is mapping only:
/// it records declared evidence coverage without real verification, network,
/// signing, key access, or bridge runtime connectivity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ATrustEvidenceMapDecl {
    pub name: Spanned<String>,
    pub agent: Spanned<String>,
    pub passport: Spanned<String>,
    pub identity: Spanned<String>,
    pub credential_contract: Spanned<String>,
    pub handshake: Spanned<String>,
    pub trust_ledger: Spanned<String>,
    pub mcp_bridges: Vec<Spanned<String>>,
    pub a2a_bridges: Vec<Spanned<String>>,
    pub policies: Vec<Spanned<String>>,
    pub coverage: Spanned<ATrustEvidenceMapCoverage>,
    pub mapping_mode: Spanned<ATrustEvidenceMapMappingMode>,
    pub verification: Spanned<ATrustHandshakeVerification>,
    pub resolution: Spanned<ATrustResolutionMode>,
    pub evidence_bundle: Spanned<ATrustEvidenceRequirement>,
    pub security_report: Spanned<ATrustEvidenceRequirement>,
    pub trace: Spanned<ATrustEvidenceRequirement>,
    pub network: Spanned<ATrustNetworkBoundary>,
    pub external_execution: Spanned<ATrustExecution>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub execution: Spanned<ATrustExecution>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustEvidenceMapCoverage {
    Required,
    Complete,
    Unknown(String),
}

impl ATrustEvidenceMapCoverage {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Required => "required",
            Self::Complete => "complete",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ATrustEvidenceMapMappingMode {
    DeclaredOnly,
    EvidenceOnly,
    Unknown(String),
}

impl ATrustEvidenceMapMappingMode {
    pub fn source_name(&self) -> &str {
        match self {
            Self::DeclaredOnly => "declared_only",
            Self::EvidenceOnly => "evidence_only",
            Self::Unknown(value) => value,
        }
    }
}

macro_rules! source_enum {
    ($name:ident { $($variant:ident => $source:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $name {
            $($variant,)+
            Unknown(String),
        }

        impl $name {
            pub fn source_name(&self) -> &str {
                match self {
                    $(Self::$variant => $source,)+
                    Self::Unknown(value) => value,
                }
            }
        }
    };
}

source_enum!(GovernanceScope {
    Agent => "agent",
    System => "system",
    Package => "package",
    Organization => "organization",
});
source_enum!(GovernanceLevel {
    Baseline => "baseline",
    Audit => "audit",
    Regulated => "regulated",
    Experimental => "experimental",
});
source_enum!(GovernanceDomain {
    AiAgent => "ai_agent",
    Security => "security",
    Compliance => "compliance",
    Privacy => "privacy",
    Safety => "safety",
    Custom => "custom",
});
source_enum!(GovernanceRiskLevel {
    Low => "low",
    Moderate => "moderate",
    High => "high",
    Critical => "critical",
    UnknownRisk => "unknown",
});
source_enum!(GovernanceReviewStatus {
    Draft => "draft",
    Reviewed => "reviewed",
    ApprovedInternal => "approved_internal",
    Deprecated => "deprecated",
});
source_enum!(GovernanceAssurance {
    DeclaredOnly => "declared_only",
    EvidenceMapped => "evidence_mapped",
    ManuallyReviewed => "manually_reviewed",
});
source_enum!(GovernanceControlCategory {
    Identity => "identity",
    Credential => "credential",
    Handshake => "handshake",
    Ledger => "ledger",
    Bridge => "bridge",
    Evidence => "evidence",
    RuntimeBoundary => "runtime_boundary",
    Policy => "policy",
    Security => "security",
    Privacy => "privacy",
    Safety => "safety",
    Compliance => "compliance",
    Custom => "custom",
});
source_enum!(GovernanceControlStatus {
    Mapped => "mapped",
    Declared => "declared",
    PendingReview => "pending_review",
    NotApplicable => "not_applicable",
});
source_enum!(RegulatoryCoverage {
    Mapped => "mapped",
    Partial => "partial",
    PendingReview => "pending_review",
});
source_enum!(RegulatoryAssessment {
    DeclaredOnly => "declared_only",
    EvidenceMapped => "evidence_mapped",
    ManualReviewRequired => "manual_review_required",
});
source_enum!(RegulatoryObligationStatus {
    Mapped => "mapped",
    PendingReview => "pending_review",
    Gap => "gap",
    NotApplicable => "not_applicable",
});

/// Declarative governance metadata. It is not a compliance certification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceProfileDecl {
    pub name: Spanned<String>,
    pub scope: Spanned<GovernanceScope>,
    pub level: Spanned<GovernanceLevel>,
    pub domain: Spanned<GovernanceDomain>,
    pub owner: Spanned<String>,
    pub jurisdiction: Spanned<String>,
    pub framework: Spanned<String>,
    pub evidence_map: Spanned<String>,
    pub trust_ledger: Spanned<String>,
    pub policies: Vec<Spanned<String>>,
    pub controls: Vec<GovernanceControlDecl>,
    pub risk_level: Spanned<GovernanceRiskLevel>,
    pub review_status: Spanned<GovernanceReviewStatus>,
    pub assurance: Spanned<GovernanceAssurance>,
    pub network: Spanned<ATrustNetworkBoundary>,
    pub external_execution: Spanned<ATrustExecution>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub execution: Spanned<ATrustExecution>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceControlDecl {
    pub id: Spanned<String>,
    pub category: Spanned<GovernanceControlCategory>,
    pub requirement: Spanned<String>,
    pub evidence_ref: Spanned<String>,
    pub status: Spanned<GovernanceControlStatus>,
}

/// Audit-oriented regulatory mapping metadata. It is not legal advice,
/// certification, regulatory approval, or proof that obligations are satisfied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegulatoryMappingDecl {
    pub name: Spanned<String>,
    pub governance_profile: Spanned<String>,
    pub evidence_map: Spanned<String>,
    pub jurisdiction: Spanned<String>,
    pub framework: Spanned<String>,
    pub obligations: Vec<RegulatoryObligationDecl>,
    pub coverage: Spanned<RegulatoryCoverage>,
    pub assessment: Spanned<RegulatoryAssessment>,
    pub legal_claims: Spanned<String>,
    pub certification: Spanned<String>,
    pub network: Spanned<ATrustNetworkBoundary>,
    pub external_execution: Spanned<ATrustExecution>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub execution: Spanned<ATrustExecution>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegulatoryObligationDecl {
    pub id: Spanned<String>,
    pub source: Spanned<String>,
    pub requirement: Spanned<String>,
    pub control: Spanned<String>,
    pub evidence_ref: Spanned<String>,
    pub status: Spanned<RegulatoryObligationStatus>,
}

source_enum!(ThirdPartyVerifierType {
    Internal => "internal",
    Community => "community",
    Academic => "academic",
    Vendor => "vendor",
    IndependentLab => "independent_lab",
    Custom => "custom",
});
source_enum!(VerifierIndependence {
    Declared => "declared",
    SelfAttested => "self_attested",
    IndependentDeclared => "independent_declared",
});
source_enum!(VerifierIdentityMode {
    DeclaredOnly => "declared_only",
    DocumentOnly => "document_only",
});
source_enum!(VerifierVerificationMode {
    ReproducibleArtifacts => "reproducible_artifacts",
    DocumentReview => "document_review",
    ConformanceReplay => "conformance_replay",
});
source_enum!(PublicConformanceResult {
    Passed => "passed",
    Failed => "failed",
    PendingReview => "pending_review",
});
source_enum!(PublicConformanceReproducibility {
    Declared => "declared",
    ReplayableLocally => "replayable_locally",
    DocumentOnly => "document_only",
});
source_enum!(PublicConformanceReviewStatus {
    Draft => "draft",
    Reviewed => "reviewed",
    Published => "published",
    Deprecated => "deprecated",
});
source_enum!(PublicConformanceClaimCategory {
    Conformance => "conformance",
    Evidence => "evidence",
    SecurityReport => "security_report",
    Governance => "governance",
    RegulatoryMapping => "regulatory_mapping",
    Bytecode => "bytecode",
    Source => "source",
    Policy => "policy",
    RuntimeBoundary => "runtime_boundary",
    Custom => "custom",
});
source_enum!(PublicConformanceClaimStatus {
    Mapped => "mapped",
    Declared => "declared",
    PendingReview => "pending_review",
    NotApplicable => "not_applicable",
});

/// Declared reviewer metadata only; it does not prove identity, independence,
/// certification, or cryptographic endorsement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThirdPartyVerifierDecl {
    pub name: Spanned<String>,
    pub verifier_type: Spanned<ThirdPartyVerifierType>,
    pub independence: Spanned<VerifierIndependence>,
    pub identity_mode: Spanned<VerifierIdentityMode>,
    pub verification_mode: Spanned<VerifierVerificationMode>,
    pub display_name: Spanned<String>,
    pub organization: Spanned<String>,
    pub jurisdiction: Spanned<String>,
    pub allowed_scopes: Vec<Spanned<String>>,
    pub disallowed_claims: Vec<Spanned<String>>,
    pub network: Spanned<ATrustNetworkBoundary>,
    pub external_execution: Spanned<ATrustExecution>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub execution: Spanned<ATrustExecution>,
    pub legal_claims: Spanned<String>,
    pub certification: Spanned<String>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

/// Reproducible public conformance metadata. It is not legal certification,
/// regulator approval, or evidence that external execution occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicConformanceReportDecl {
    pub name: Spanned<String>,
    pub verifier: Spanned<String>,
    pub suite: Spanned<String>,
    pub suite_version: Spanned<String>,
    pub source_artifact: Spanned<String>,
    pub bytecode_artifact: Spanned<String>,
    pub evidence_map: Spanned<String>,
    pub governance_profile: Spanned<String>,
    pub regulatory_mapping: Spanned<String>,
    pub trust_ledger: Spanned<String>,
    pub security_report: Spanned<String>,
    pub evidence_bundle: Spanned<String>,
    pub trace: Spanned<String>,
    pub result: Spanned<PublicConformanceResult>,
    pub reproducibility: Spanned<PublicConformanceReproducibility>,
    pub review_status: Spanned<PublicConformanceReviewStatus>,
    pub claims: Vec<PublicConformanceClaimDecl>,
    pub network: Spanned<ATrustNetworkBoundary>,
    pub external_execution: Spanned<ATrustExecution>,
    pub secret_material: Spanned<ATrustMaterialBoundary>,
    pub key_material: Spanned<ATrustMaterialBoundary>,
    pub execution: Spanned<ATrustExecution>,
    pub legal_claims: Spanned<String>,
    pub certification: Spanned<String>,
    pub security_claims: Spanned<ATrustSecurityClaims>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicConformanceClaimDecl {
    pub id: Spanned<String>,
    pub category: Spanned<PublicConformanceClaimCategory>,
    pub statement: Spanned<String>,
    pub evidence_ref: Spanned<String>,
    pub status: Spanned<PublicConformanceClaimStatus>,
}

/// Declarative pre-runtime hardening metadata. No field enables runtime
/// execution or active enforcement in v0.35.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHardeningProfileDecl {
    pub name: Spanned<String>,
    pub scope: Spanned<String>,
    pub mode: Spanned<String>,
    pub enforcement: Spanned<String>,
    pub sandbox: Spanned<String>,
    pub provider_execution: Spanned<String>,
    pub external_providers: Spanned<String>,
    pub network: Spanned<String>,
    pub tool_execution: Spanned<String>,
    pub agent_execution: Spanned<String>,
    pub filesystem_access: Spanned<String>,
    pub env_access: Spanned<String>,
    pub secret_material: Spanned<String>,
    pub key_material: Spanned<String>,
    pub allowlist: Spanned<String>,
    pub deny_by_default: Spanned<bool>,
    pub approval: Spanned<String>,
    pub audit_log: Spanned<String>,
    pub evidence: Spanned<String>,
    pub incident_response: Spanned<String>,
    pub evidence_map: Spanned<String>,
    pub governance_profile: Spanned<String>,
    pub public_conformance_report: Spanned<String>,
    pub protected_assets: Vec<Spanned<String>>,
    pub runtime_boundaries: Vec<Spanned<String>>,
    pub residual_risk: Spanned<String>,
    pub review_status: Spanned<String>,
    pub assurance: Spanned<String>,
    pub security_claims: Spanned<String>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

/// Declarative threat-model metadata. It cannot execute attacks or establish
/// that a system is secure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreatModelDecl {
    pub name: Spanned<String>,
    pub hardening_profile: Spanned<String>,
    pub evidence_map: Spanned<String>,
    pub governance_profile: Spanned<String>,
    pub public_conformance_report: Spanned<String>,
    pub methodology: Spanned<String>,
    pub scope: Spanned<String>,
    pub review_status: Spanned<String>,
    pub assets: Vec<ThreatAssetDecl>,
    pub threats: Vec<ThreatDecl>,
    pub mitigations: Vec<ThreatMitigationDecl>,
    pub residual_risk: Spanned<String>,
    pub risk_acceptance: Spanned<String>,
    pub network: Spanned<String>,
    pub external_execution: Spanned<String>,
    pub tool_execution: Spanned<String>,
    pub agent_execution: Spanned<String>,
    pub secret_material: Spanned<String>,
    pub key_material: Spanned<String>,
    pub execution: Spanned<String>,
    pub security_claims: Spanned<String>,
    pub purpose: Vec<Spanned<String>>,
    pub notes: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreatAssetDecl {
    pub id: Spanned<String>,
    pub category: Spanned<String>,
    pub description: Spanned<String>,
    pub sensitivity: Spanned<String>,
    pub evidence_ref: Spanned<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreatDecl {
    pub id: Spanned<String>,
    pub category: Spanned<String>,
    pub target: Spanned<String>,
    pub impact: Spanned<String>,
    pub mitigation: Spanned<String>,
    pub status: Spanned<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreatMitigationDecl {
    pub id: Spanned<String>,
    pub category: Spanned<String>,
    pub control_ref: Spanned<String>,
    pub evidence_ref: Spanned<String>,
    pub status: Spanned<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeTransport {
    DeclaredOnly,
    Disabled,
    Unknown(String),
}

impl BridgeTransport {
    pub fn source_name(&self) -> &str {
        match self {
            Self::DeclaredOnly => "declared_only",
            Self::Disabled => "disabled",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpProtocol {
    Mcp,
    Unknown(String),
}

impl McpProtocol {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Mcp => "mcp",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A2AProtocol {
    A2A,
    Unknown(String),
}

impl A2AProtocol {
    pub fn source_name(&self) -> &str {
        match self {
            Self::A2A => "a2a",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeDirection {
    Inbound,
    Outbound,
    Bidirectional,
    Unknown(String),
}

impl BridgeDirection {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
            Self::Bidirectional => "bidirectional",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeAuthentication {
    None,
    DeclaredOnly,
    Unknown(String),
}

impl BridgeAuthentication {
    pub fn source_name(&self) -> &str {
        match self {
            Self::None => "none",
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeAuthorization {
    PolicyBound,
    DeclaredOnly,
    Unknown(String),
}

impl BridgeAuthorization {
    pub fn source_name(&self) -> &str {
        match self {
            Self::PolicyBound => "policy_bound",
            Self::DeclaredOnly => "declared_only",
            Self::Unknown(value) => value,
        }
    }
}
