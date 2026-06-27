use argorix_parser::ast::{HandlerInstruction, Program};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrProgram {
    pub ir_version: String,
    pub language: String,
    pub module: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<IrModule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<IrModuleImport>,
    pub providers: Vec<IrProviderContract>,
    pub provider_harnesses: Vec<IrProviderHarness>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<IrFeature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<IrSecret>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapters: Vec<IrAdapter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapter_profiles: Vec<IrAdapterProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cryptos: Vec<IrCrypto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crypto_boundaries: Vec<IrCryptoBoundary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub did_methods: Vec<IrDidMethod>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_boundaries: Vec<IrATrustBoundary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_identities: Vec<IrATrustIdentity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_credential_contracts: Vec<IrATrustCredentialContract>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_handshakes: Vec<IrATrustHandshake>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trust_ledgers: Vec<IrTrustLedger>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_bridge_contracts: Vec<IrMcpBridgeContract>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub a2a_bridge_contracts: Vec<IrA2ABridgeContract>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_evidence_maps: Vec<IrATrustEvidenceMap>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub governance_profiles: Vec<IrGovernanceProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regulatory_mappings: Vec<IrRegulatoryMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub third_party_verifiers: Vec<IrThirdPartyVerifier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_conformance_reports: Vec<IrPublicConformanceReport>,
    pub assertions: Vec<IrAssertion>,
    pub policies: Vec<IrPolicy>,
    pub failures: Vec<IrFailure>,
    pub capabilities: Vec<IrCapability>,
    pub enums: Vec<IrEnum>,
    pub types: Vec<IrType>,
    pub tools: Vec<IrTool>,
    pub models: Vec<IrModel>,
    pub agents: Vec<IrAgent>,
    pub protocols: Vec<IrProtocol>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passports: Vec<IrPassport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPassport {
    pub name: String,
    pub agent: String,
    pub agent_name: String,
    pub global_id: String,
    pub identity: String,
    pub provider: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ans_name: Option<String>,
    pub country: String,
    pub jurisdiction: String,
    pub data_residency: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asn: Option<IrPassportAsn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub risk_level: String,
    pub data_scope: Vec<String>,
    pub intent: String,
    pub intended_use: Vec<String>,
    pub prohibited_use: Vec<String>,
    pub attestations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPassportAsn {
    pub registry: String,
    pub number: String,
    pub holder: String,
    pub country: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrModule {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrModuleImport {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrProviderContract {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub dry_run_only: bool,
    pub requires_feature_flag: bool,
    pub requires_explicit_approval: bool,
    pub allowed_targets: Vec<String>,
    pub allowed_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrFeature {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub status: String,
    pub default: String,
    pub requires_approval: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrSecret {
    pub name: String,
    pub handle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_by: Option<String>,
    pub scope: String,
    pub access: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrAdapter {
    pub name: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    pub mode: String,
    pub execution: String,
    pub network: String,
    pub secrets: String,
    pub filesystem: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<String>,
    pub conformance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrAdapterProfile {
    pub name: String,
    pub adapter: String,
    pub provider: String,
    pub vendor: String,
    pub family: String,
    pub api_style: String,
    pub auth: String,
    pub execution: String,
    pub network: String,
    pub secrets: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_contract: Option<String>,
    pub capabilities: Vec<String>,
    pub required_conformance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrCrypto {
    pub name: String,
    pub kind: String,
    pub status: String,
    pub strength: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_bits: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_key_bits: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrCryptoBoundary {
    pub name: String,
    #[serde(default)]
    pub allowed_hashes: Vec<String>,
    #[serde(default)]
    pub allowed_signatures: Vec<String>,
    #[serde(default)]
    pub allowed_kems: Vec<String>,
    #[serde(default)]
    pub allowed_aeads: Vec<String>,
    #[serde(default)]
    pub legacy_allowed: Vec<String>,
    #[serde(default)]
    pub denied: Vec<String>,
    #[serde(default)]
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_hash_bits: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_quantum_ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid_allowed: Option<bool>,
    pub key_material: String,
    pub secret_material: String,
    pub execution: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrDidMethod {
    pub name: String,
    pub status: String,
    pub resolution: String,
    pub ledger: String,
    pub crypto_boundary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance: Option<String>,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrATrustBoundary {
    pub name: String,
    pub crypto_boundary: String,
    pub did_methods: Vec<String>,
    pub identity_format: String,
    pub credential_mode: String,
    pub handshake: String,
    pub resolution: String,
    pub key_material: String,
    pub secret_material: String,
    pub execution: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_quantum_ready: Option<String>,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrATrustIdentity {
    pub name: String,
    pub subject: String,
    pub did: String,
    pub method: String,
    pub boundary: String,
    pub status: String,
    pub validation: String,
    pub resolution: String,
    pub key_material: String,
    pub secret_material: String,
    pub execution: String,
    pub evidence: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrATrustCredentialContract {
    pub name: String,
    pub subject: String,
    pub identity: String,
    pub boundary: String,
    pub method: String,
    pub issuer_did: String,
    pub holder_did: String,
    pub credential_type: String,
    pub schema: String,
    pub status: String,
    pub verification: String,
    pub presentation: String,
    pub resolution: String,
    pub key_material: String,
    pub secret_material: String,
    pub execution: String,
    pub evidence: String,
    pub security_claims: String,
    pub claims: Vec<String>,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrATrustHandshake {
    pub name: String,
    pub initiator: String,
    pub responder: String,
    pub initiator_identity: String,
    pub responder_identity: String,
    pub credential_contracts: Vec<String>,
    pub boundary: String,
    pub method: String,
    pub mode: String,
    pub direction: String,
    pub challenge: String,
    pub response: String,
    pub transcript: String,
    pub verification: String,
    pub resolution: String,
    pub network: String,
    pub key_material: String,
    pub secret_material: String,
    pub execution: String,
    pub evidence: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrTrustLedger {
    pub name: String,
    pub scope: String,
    pub mode: String,
    pub hash_algorithm: String,
    pub chain_policy: String,
    pub entries: Vec<IrTrustLedgerEntry>,
    pub chain_root: String,
    pub network: String,
    pub key_material: String,
    pub secret_material: String,
    pub execution: String,
    pub evidence: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrTrustLedgerEntry {
    pub id: String,
    pub kind: String,
    pub subject: String,
    pub previous_hash: String,
    pub entry_hash: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrMcpBridgeContract {
    pub name: String,
    pub agent: String,
    pub passport: String,
    pub identity: String,
    pub boundary: String,
    pub transport: String,
    pub protocol: String,
    pub direction: String,
    pub tools: Vec<String>,
    pub resources: Vec<String>,
    pub prompts: Vec<String>,
    pub network: String,
    pub external_execution: String,
    pub tool_execution: String,
    pub secret_material: String,
    pub key_material: String,
    pub authentication: String,
    pub authorization: String,
    pub evidence: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrA2ABridgeContract {
    pub name: String,
    pub initiator: String,
    pub responder: String,
    pub initiator_passport: String,
    pub responder_passport: String,
    pub initiator_identity: String,
    pub responder_identity: String,
    pub handshake: String,
    pub trust_ledger: String,
    pub boundary: String,
    pub protocol: String,
    pub transport: String,
    pub direction: String,
    pub message_contracts: Vec<String>,
    pub capabilities: Vec<String>,
    pub network: String,
    pub external_execution: String,
    pub agent_execution: String,
    pub secret_material: String,
    pub key_material: String,
    pub authentication: String,
    pub authorization: String,
    pub evidence: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrATrustEvidenceMap {
    pub name: String,
    pub agent: String,
    pub passport: String,
    pub identity: String,
    pub credential_contract: String,
    pub handshake: String,
    pub trust_ledger: String,
    pub mcp_bridges: Vec<String>,
    pub a2a_bridges: Vec<String>,
    pub policies: Vec<String>,
    pub coverage: String,
    pub mapping_mode: String,
    pub verification: String,
    pub resolution: String,
    pub evidence_bundle: String,
    pub security_report: String,
    pub trace: String,
    pub network: String,
    pub external_execution: String,
    pub secret_material: String,
    pub key_material: String,
    pub execution: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrGovernanceProfile {
    pub name: String,
    pub scope: String,
    pub level: String,
    pub domain: String,
    pub owner: String,
    pub jurisdiction: String,
    pub framework: String,
    pub evidence_map: String,
    pub trust_ledger: String,
    pub policies: Vec<String>,
    pub controls: Vec<IrGovernanceControl>,
    pub risk_level: String,
    pub review_status: String,
    pub assurance: String,
    pub network: String,
    pub external_execution: String,
    pub secret_material: String,
    pub key_material: String,
    pub execution: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrGovernanceControl {
    pub id: String,
    pub category: String,
    pub requirement: String,
    pub evidence_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrRegulatoryMapping {
    pub name: String,
    pub governance_profile: String,
    pub evidence_map: String,
    pub jurisdiction: String,
    pub framework: String,
    pub obligations: Vec<IrRegulatoryObligation>,
    pub coverage: String,
    pub assessment: String,
    pub legal_claims: String,
    pub certification: String,
    pub network: String,
    pub external_execution: String,
    pub secret_material: String,
    pub key_material: String,
    pub execution: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrRegulatoryObligation {
    pub id: String,
    pub source: String,
    pub requirement: String,
    pub control: String,
    pub evidence_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrThirdPartyVerifier {
    pub name: String,
    pub verifier_type: String,
    pub independence: String,
    pub identity_mode: String,
    pub verification_mode: String,
    pub display_name: String,
    pub organization: String,
    pub jurisdiction: String,
    pub allowed_scopes: Vec<String>,
    pub disallowed_claims: Vec<String>,
    pub network: String,
    pub external_execution: String,
    pub secret_material: String,
    pub key_material: String,
    pub execution: String,
    pub legal_claims: String,
    pub certification: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPublicConformanceReport {
    pub name: String,
    pub verifier: String,
    pub suite: String,
    pub suite_version: String,
    pub source_artifact: String,
    pub bytecode_artifact: String,
    pub evidence_map: String,
    pub governance_profile: String,
    pub regulatory_mapping: String,
    pub trust_ledger: String,
    pub security_report: String,
    pub evidence_bundle: String,
    pub trace: String,
    pub result: String,
    pub reproducibility: String,
    pub review_status: String,
    pub claims: Vec<IrPublicConformanceClaim>,
    pub network: String,
    pub external_execution: String,
    pub secret_material: String,
    pub key_material: String,
    pub execution: String,
    pub legal_claims: String,
    pub certification: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPublicConformanceClaim {
    pub id: String,
    pub category: String,
    pub statement: String,
    pub evidence_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrProviderHarness {
    pub name: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    pub mode: String,
    pub network: String,
    pub secrets: String,
    pub filesystem: String,
    pub max_steps: Option<u64>,
    pub timeout_ms: Option<u64>,
    pub input_contract: Option<String>,
    pub output_contract: Option<String>,
    pub attestations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrAssertion {
    pub name: String,
    pub argument: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPolicy {
    pub name: String,
    pub rules: Vec<IrPolicyRule>,
    pub on_violation: Option<IrPolicyViolation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPolicyRule {
    pub effect: String,
    pub rule: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPolicyViolation {
    pub action: String,
    pub trace_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrFailure {
    pub name: String,
    pub action: String,
    pub trace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrModel {
    pub name: String,
    pub provider: String,
    pub capability: String,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrTool {
    pub name: String,
    pub provider: String,
    pub capability: String,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrCapability {
    pub name: String,
    pub level: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrEnum {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrType {
    pub name: String,
    pub fields: Vec<IrField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrAgent {
    pub name: String,
    pub approval: String,
    pub receives: Vec<IrReceive>,
    pub sends: Vec<IrSend>,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
    pub models: Vec<String>,
    pub handlers: Vec<IrHandler>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrHandler {
    pub message_type: String,
    pub binding: String,
    pub instructions: Vec<IrHandlerInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum IrHandlerInstruction {
    Emit { message_type: String, to: String },
    Trace { binding: String },
    Halt,
    Intrinsic { name: String, argument: String },
    Call { tool: String, binding: String },
    Ask { model: String, binding: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrReceive {
    pub message_type: String,
    pub from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrSend {
    pub message_type: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrProtocol {
    pub name: String,
    pub steps: Vec<IrProtocolStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrProtocolStep {
    pub from: String,
    pub to: String,
    pub act: String,
    pub message_type: String,
}

impl From<&Program> for IrProgram {
    fn from(program: &Program) -> Self {
        Self {
            ir_version: "0.34".to_owned(),
            language: "Argorix Lang".to_owned(),
            module: program.module.value.clone(),
            modules: Vec::new(),
            imports: Vec::new(),
            providers: program
                .providers
                .iter()
                .map(|provider| IrProviderContract {
                    name: provider.name.value.clone(),
                    kind: provider.kind.value.as_str().into(),
                    enabled: provider.enabled.value,
                    dry_run_only: provider.dry_run_only.value,
                    requires_feature_flag: provider.requires_feature_flag,
                    requires_explicit_approval: provider.requires_explicit_approval,
                    allowed_targets: provider
                        .allowed_targets
                        .iter()
                        .map(|item| item.value.clone())
                        .collect(),
                    allowed_capabilities: provider
                        .allowed_capabilities
                        .iter()
                        .map(|item| item.value.clone())
                        .collect(),
                })
                .collect(),
            provider_harnesses: program
                .harnesses
                .iter()
                .map(|harness| IrProviderHarness {
                    name: harness.name.value.clone(),
                    provider: harness.provider.value.clone(),
                    feature: harness.feature.as_ref().map(|value| value.value.clone()),
                    secret: harness.secret.as_ref().map(|value| value.value.clone()),
                    mode: harness.mode.value.source_name().to_owned(),
                    network: harness.network.value.source_name().to_owned(),
                    secrets: harness.secrets.value.source_name().to_owned(),
                    filesystem: harness.filesystem.value.source_name().to_owned(),
                    max_steps: harness.max_steps.as_ref().map(|value| value.value),
                    timeout_ms: harness.timeout_ms.as_ref().map(|value| value.value),
                    input_contract: harness
                        .input_contract
                        .as_ref()
                        .map(|value| value.value.clone()),
                    output_contract: harness
                        .output_contract
                        .as_ref()
                        .map(|value| value.value.clone()),
                    attestations: spanned_values(&harness.attestations),
                })
                .collect(),
            features: program
                .features
                .iter()
                .map(|feature| IrFeature {
                    name: feature.name.value.clone(),
                    provider: feature.provider.as_ref().map(|value| value.value.clone()),
                    status: feature.status.value.source_name().to_owned(),
                    default: feature.default.value.source_name().to_owned(),
                    requires_approval: feature.requires_approval,
                    purpose: feature.purpose.as_ref().map(|value| value.value.clone()),
                })
                .collect(),
            secrets: program
                .secrets
                .iter()
                .map(|secret| IrSecret {
                    name: secret.name.value.clone(),
                    handle: secret.handle.value.clone(),
                    provider: secret.provider.as_ref().map(|value| value.value.clone()),
                    required_by: secret.required_by.as_ref().map(|value| value.value.clone()),
                    scope: secret.scope.value.source_name().to_owned(),
                    access: secret.access.value.source_name().to_owned(),
                    source: secret.source.value.source_name().to_owned(),
                })
                .collect(),
            adapters: program
                .adapters
                .iter()
                .map(|adapter| IrAdapter {
                    name: adapter.name.value.clone(),
                    provider: adapter.provider.value.clone(),
                    feature: adapter.feature.as_ref().map(|v| v.value.clone()),
                    secret: adapter.secret.as_ref().map(|v| v.value.clone()),
                    harness: adapter.harness.as_ref().map(|v| v.value.clone()),
                    kind: adapter
                        .kind
                        .as_ref()
                        .map(|v| v.value.source_name().to_owned()),
                    vendor: adapter.vendor.as_ref().map(|v| v.value.clone()),
                    mode: adapter.mode.value.source_name().to_owned(),
                    execution: adapter.execution.value.source_name().to_owned(),
                    network: adapter.network.value.source_name().to_owned(),
                    secrets: adapter.secrets.value.source_name().to_owned(),
                    filesystem: adapter.filesystem.value.source_name().to_owned(),
                    input_contract: adapter.input_contract.as_ref().map(|v| v.value.clone()),
                    output_contract: adapter.output_contract.as_ref().map(|v| v.value.clone()),
                    conformance: adapter
                        .conformance
                        .iter()
                        .map(|v| v.value.clone())
                        .collect(),
                })
                .collect(),
            adapter_profiles: program
                .adapter_profiles
                .iter()
                .map(|p| IrAdapterProfile {
                    name: p.name.value.clone(),
                    adapter: p.adapter.value.clone(),
                    provider: p.provider.value.clone(),
                    vendor: p.vendor.value.clone(),
                    family: p.family.value.source_name().to_owned(),
                    api_style: p.api_style.value.source_name().to_owned(),
                    auth: p.auth.value.source_name().to_owned(),
                    execution: p.execution.value.source_name().to_owned(),
                    network: p.network.value.source_name().to_owned(),
                    secrets: p.secrets.value.source_name().to_owned(),
                    request_contract: p.request_contract.as_ref().map(|v| v.value.clone()),
                    response_contract: p.response_contract.as_ref().map(|v| v.value.clone()),
                    capabilities: p.capabilities.iter().map(|v| v.value.clone()).collect(),
                    required_conformance: p
                        .required_conformance
                        .iter()
                        .map(|v| v.value.clone())
                        .collect(),
                })
                .collect(),
            cryptos: program
                .cryptos
                .iter()
                .map(|c| IrCrypto {
                    name: c.name.value.clone(),
                    kind: c.kind.value.source_name().to_owned(),
                    status: c.status.value.source_name().to_owned(),
                    strength: c.strength.value.source_name().to_owned(),
                    purpose: c.purpose.iter().map(|v| v.value.clone()).collect(),
                    output_bits: c.output_bits.as_ref().map(|v| v.value),
                    min_key_bits: c.min_key_bits.as_ref().map(|v| v.value),
                    security_level: c.security_level.as_ref().map(|v| v.value.clone()),
                    notes: c.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            crypto_boundaries: program
                .crypto_boundaries
                .iter()
                .map(|b| IrCryptoBoundary {
                    name: b.name.value.clone(),
                    allowed_hashes: b.allowed_hashes.iter().map(|v| v.value.clone()).collect(),
                    allowed_signatures: b
                        .allowed_signatures
                        .iter()
                        .map(|v| v.value.clone())
                        .collect(),
                    allowed_kems: b.allowed_kems.iter().map(|v| v.value.clone()).collect(),
                    allowed_aeads: b.allowed_aeads.iter().map(|v| v.value.clone()).collect(),
                    legacy_allowed: b.legacy_allowed.iter().map(|v| v.value.clone()).collect(),
                    denied: b.denied.iter().map(|v| v.value.clone()).collect(),
                    purpose: b.purpose.iter().map(|v| v.value.clone()).collect(),
                    min_hash_bits: b.min_hash_bits.as_ref().map(|v| v.value),
                    post_quantum_ready: b.post_quantum_ready.as_ref().map(|v| v.value),
                    hybrid_allowed: b.hybrid_allowed.as_ref().map(|v| v.value),
                    key_material: b.key_material.value.clone(),
                    secret_material: b.secret_material.value.clone(),
                    execution: b.execution.value.clone(),
                })
                .collect(),
            did_methods: program
                .did_methods
                .iter()
                .map(|d| IrDidMethod {
                    name: d.name.value.clone(),
                    status: d.status.value.source_name().to_owned(),
                    resolution: d.resolution.value.source_name().to_owned(),
                    ledger: d.ledger.value.source_name().to_owned(),
                    crypto_boundary: d.crypto_boundary.value.clone(),
                    governance: d.governance.as_ref().map(|v| v.value.clone()),
                    purpose: d.purpose.iter().map(|v| v.value.clone()).collect(),
                    notes: d.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            atrust_boundaries: program
                .atrust_boundaries
                .iter()
                .map(|a| IrATrustBoundary {
                    name: a.name.value.clone(),
                    crypto_boundary: a.crypto_boundary.value.clone(),
                    did_methods: a.did_methods.iter().map(|v| v.value.clone()).collect(),
                    identity_format: a.identity_format.value.source_name().to_owned(),
                    credential_mode: a.credential_mode.value.source_name().to_owned(),
                    handshake: a.handshake.value.source_name().to_owned(),
                    resolution: a.resolution.value.source_name().to_owned(),
                    key_material: a.key_material.value.source_name().to_owned(),
                    secret_material: a.secret_material.value.source_name().to_owned(),
                    execution: a.execution.value.source_name().to_owned(),
                    post_quantum_ready: a
                        .post_quantum_ready
                        .as_ref()
                        .map(|v| v.value.source_name().to_owned()),
                    security_claims: a.security_claims.value.source_name().to_owned(),
                    purpose: a.purpose.iter().map(|v| v.value.clone()).collect(),
                    notes: a.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            atrust_identities: program
                .atrust_identities
                .iter()
                .map(|i| IrATrustIdentity {
                    name: i.name.value.clone(),
                    subject: i.subject.value.clone(),
                    did: i.did.value.clone(),
                    method: i.method.value.clone(),
                    boundary: i.boundary.value.clone(),
                    status: i.status.value.source_name().to_owned(),
                    validation: i.validation.value.source_name().to_owned(),
                    resolution: i.resolution.value.source_name().to_owned(),
                    key_material: i.key_material.value.source_name().to_owned(),
                    secret_material: i.secret_material.value.source_name().to_owned(),
                    execution: i.execution.value.source_name().to_owned(),
                    evidence: i.evidence.value.source_name().to_owned(),
                    security_claims: i.security_claims.value.source_name().to_owned(),
                    purpose: i.purpose.iter().map(|v| v.value.clone()).collect(),
                    notes: i.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            atrust_credential_contracts: program
                .atrust_credential_contracts
                .iter()
                .map(|c| IrATrustCredentialContract {
                    name: c.name.value.clone(),
                    subject: c.subject.value.clone(),
                    identity: c.identity.value.clone(),
                    boundary: c.boundary.value.clone(),
                    method: c.method.value.clone(),
                    issuer_did: c.issuer_did.value.clone(),
                    holder_did: c.holder_did.value.clone(),
                    credential_type: c.credential_type.value.clone(),
                    schema: c.schema.value.clone(),
                    status: c.status.value.source_name().to_owned(),
                    verification: c.verification.value.source_name().to_owned(),
                    presentation: c.presentation.value.source_name().to_owned(),
                    resolution: c.resolution.value.source_name().to_owned(),
                    key_material: c.key_material.value.source_name().to_owned(),
                    secret_material: c.secret_material.value.source_name().to_owned(),
                    execution: c.execution.value.source_name().to_owned(),
                    evidence: c.evidence.value.source_name().to_owned(),
                    security_claims: c.security_claims.value.source_name().to_owned(),
                    claims: c.claims.iter().map(|v| v.value.clone()).collect(),
                    purpose: c.purpose.iter().map(|v| v.value.clone()).collect(),
                    notes: c.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            atrust_handshakes: program
                .atrust_handshakes
                .iter()
                .map(|h| IrATrustHandshake {
                    name: h.name.value.clone(),
                    initiator: h.initiator.value.clone(),
                    responder: h.responder.value.clone(),
                    initiator_identity: h.initiator_identity.value.clone(),
                    responder_identity: h.responder_identity.value.clone(),
                    credential_contracts: h
                        .credential_contracts
                        .iter()
                        .map(|v| v.value.clone())
                        .collect(),
                    boundary: h.boundary.value.clone(),
                    method: h.method.value.clone(),
                    mode: h.mode.value.source_name().to_owned(),
                    direction: h.direction.value.source_name().to_owned(),
                    challenge: h.challenge.value.source_name().to_owned(),
                    response: h.response.value.source_name().to_owned(),
                    transcript: h.transcript.value.source_name().to_owned(),
                    verification: h.verification.value.source_name().to_owned(),
                    resolution: h.resolution.value.source_name().to_owned(),
                    network: h.network.value.source_name().to_owned(),
                    key_material: h.key_material.value.source_name().to_owned(),
                    secret_material: h.secret_material.value.source_name().to_owned(),
                    execution: h.execution.value.source_name().to_owned(),
                    evidence: h.evidence.value.source_name().to_owned(),
                    security_claims: h.security_claims.value.source_name().to_owned(),
                    purpose: h.purpose.iter().map(|v| v.value.clone()).collect(),
                    notes: h.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            trust_ledgers: program
                .trust_ledgers
                .iter()
                .map(|l| IrTrustLedger {
                    name: l.name.value.clone(),
                    scope: l.scope.value.source_name().to_owned(),
                    mode: l.mode.value.source_name().to_owned(),
                    hash_algorithm: l.hash_algorithm.value.clone(),
                    chain_policy: l.chain_policy.value.source_name().to_owned(),
                    entries: l
                        .entries
                        .iter()
                        .map(|e| IrTrustLedgerEntry {
                            id: e.id.value.clone(),
                            kind: e.kind.value.source_name().to_owned(),
                            subject: e.subject.value.clone(),
                            previous_hash: e.previous_hash.value.clone(),
                            entry_hash: e.entry_hash.value.clone(),
                            evidence_ref: e.evidence_ref.value.clone(),
                        })
                        .collect(),
                    chain_root: l.chain_root.value.clone(),
                    network: l.network.value.source_name().to_owned(),
                    key_material: l.key_material.value.source_name().to_owned(),
                    secret_material: l.secret_material.value.source_name().to_owned(),
                    execution: l.execution.value.source_name().to_owned(),
                    evidence: l.evidence.value.source_name().to_owned(),
                    security_claims: l.security_claims.value.source_name().to_owned(),
                    purpose: l.purpose.iter().map(|v| v.value.clone()).collect(),
                    notes: l.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            mcp_bridge_contracts: program
                .mcp_bridge_contracts
                .iter()
                .map(|c| IrMcpBridgeContract {
                    name: c.name.value.clone(),
                    agent: c.agent.value.clone(),
                    passport: c.passport.value.clone(),
                    identity: c.identity.value.clone(),
                    boundary: c.boundary.value.clone(),
                    transport: c.transport.value.source_name().to_owned(),
                    protocol: c.protocol.value.source_name().to_owned(),
                    direction: c.direction.value.source_name().to_owned(),
                    tools: c.tools.iter().map(|v| v.value.clone()).collect(),
                    resources: c.resources.iter().map(|v| v.value.clone()).collect(),
                    prompts: c.prompts.iter().map(|v| v.value.clone()).collect(),
                    network: c.network.value.source_name().to_owned(),
                    external_execution: c.external_execution.value.source_name().to_owned(),
                    tool_execution: c.tool_execution.value.source_name().to_owned(),
                    secret_material: c.secret_material.value.source_name().to_owned(),
                    key_material: c.key_material.value.source_name().to_owned(),
                    authentication: c.authentication.value.source_name().to_owned(),
                    authorization: c.authorization.value.source_name().to_owned(),
                    evidence: c.evidence.value.source_name().to_owned(),
                    security_claims: c.security_claims.value.source_name().to_owned(),
                    purpose: c.purpose.iter().map(|v| v.value.clone()).collect(),
                    notes: c.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            a2a_bridge_contracts: program
                .a2a_bridge_contracts
                .iter()
                .map(|c| IrA2ABridgeContract {
                    name: c.name.value.clone(),
                    initiator: c.initiator.value.clone(),
                    responder: c.responder.value.clone(),
                    initiator_passport: c.initiator_passport.value.clone(),
                    responder_passport: c.responder_passport.value.clone(),
                    initiator_identity: c.initiator_identity.value.clone(),
                    responder_identity: c.responder_identity.value.clone(),
                    handshake: c.handshake.value.clone(),
                    trust_ledger: c.trust_ledger.value.clone(),
                    boundary: c.boundary.value.clone(),
                    protocol: c.protocol.value.source_name().to_owned(),
                    transport: c.transport.value.source_name().to_owned(),
                    direction: c.direction.value.source_name().to_owned(),
                    message_contracts: c
                        .message_contracts
                        .iter()
                        .map(|v| v.value.clone())
                        .collect(),
                    capabilities: c.capabilities.iter().map(|v| v.value.clone()).collect(),
                    network: c.network.value.source_name().to_owned(),
                    external_execution: c.external_execution.value.source_name().to_owned(),
                    agent_execution: c.agent_execution.value.source_name().to_owned(),
                    secret_material: c.secret_material.value.source_name().to_owned(),
                    key_material: c.key_material.value.source_name().to_owned(),
                    authentication: c.authentication.value.source_name().to_owned(),
                    authorization: c.authorization.value.source_name().to_owned(),
                    evidence: c.evidence.value.source_name().to_owned(),
                    security_claims: c.security_claims.value.source_name().to_owned(),
                    purpose: c.purpose.iter().map(|v| v.value.clone()).collect(),
                    notes: c.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            atrust_evidence_maps: program
                .atrust_evidence_maps
                .iter()
                .map(|m| IrATrustEvidenceMap {
                    name: m.name.value.clone(),
                    agent: m.agent.value.clone(),
                    passport: m.passport.value.clone(),
                    identity: m.identity.value.clone(),
                    credential_contract: m.credential_contract.value.clone(),
                    handshake: m.handshake.value.clone(),
                    trust_ledger: m.trust_ledger.value.clone(),
                    mcp_bridges: spanned_values(&m.mcp_bridges),
                    a2a_bridges: spanned_values(&m.a2a_bridges),
                    policies: spanned_values(&m.policies),
                    coverage: m.coverage.value.source_name().to_owned(),
                    mapping_mode: m.mapping_mode.value.source_name().to_owned(),
                    verification: m.verification.value.source_name().to_owned(),
                    resolution: m.resolution.value.source_name().to_owned(),
                    evidence_bundle: m.evidence_bundle.value.source_name().to_owned(),
                    security_report: m.security_report.value.source_name().to_owned(),
                    trace: m.trace.value.source_name().to_owned(),
                    network: m.network.value.source_name().to_owned(),
                    external_execution: m.external_execution.value.source_name().to_owned(),
                    secret_material: m.secret_material.value.source_name().to_owned(),
                    key_material: m.key_material.value.source_name().to_owned(),
                    execution: m.execution.value.source_name().to_owned(),
                    security_claims: m.security_claims.value.source_name().to_owned(),
                    purpose: spanned_values(&m.purpose),
                    notes: m.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            governance_profiles: program
                .governance_profiles
                .iter()
                .map(|p| IrGovernanceProfile {
                    name: p.name.value.clone(),
                    scope: p.scope.value.source_name().to_owned(),
                    level: p.level.value.source_name().to_owned(),
                    domain: p.domain.value.source_name().to_owned(),
                    owner: p.owner.value.clone(),
                    jurisdiction: p.jurisdiction.value.clone(),
                    framework: p.framework.value.clone(),
                    evidence_map: p.evidence_map.value.clone(),
                    trust_ledger: p.trust_ledger.value.clone(),
                    policies: spanned_values(&p.policies),
                    controls: p
                        .controls
                        .iter()
                        .map(|c| IrGovernanceControl {
                            id: c.id.value.clone(),
                            category: c.category.value.source_name().to_owned(),
                            requirement: c.requirement.value.clone(),
                            evidence_ref: c.evidence_ref.value.clone(),
                            status: c.status.value.source_name().to_owned(),
                        })
                        .collect(),
                    risk_level: p.risk_level.value.source_name().to_owned(),
                    review_status: p.review_status.value.source_name().to_owned(),
                    assurance: p.assurance.value.source_name().to_owned(),
                    network: p.network.value.source_name().to_owned(),
                    external_execution: p.external_execution.value.source_name().to_owned(),
                    secret_material: p.secret_material.value.source_name().to_owned(),
                    key_material: p.key_material.value.source_name().to_owned(),
                    execution: p.execution.value.source_name().to_owned(),
                    security_claims: p.security_claims.value.source_name().to_owned(),
                    purpose: spanned_values(&p.purpose),
                    notes: p.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            regulatory_mappings: program
                .regulatory_mappings
                .iter()
                .map(|m| IrRegulatoryMapping {
                    name: m.name.value.clone(),
                    governance_profile: m.governance_profile.value.clone(),
                    evidence_map: m.evidence_map.value.clone(),
                    jurisdiction: m.jurisdiction.value.clone(),
                    framework: m.framework.value.clone(),
                    obligations: m
                        .obligations
                        .iter()
                        .map(|o| IrRegulatoryObligation {
                            id: o.id.value.clone(),
                            source: o.source.value.clone(),
                            requirement: o.requirement.value.clone(),
                            control: o.control.value.clone(),
                            evidence_ref: o.evidence_ref.value.clone(),
                            status: o.status.value.source_name().to_owned(),
                        })
                        .collect(),
                    coverage: m.coverage.value.source_name().to_owned(),
                    assessment: m.assessment.value.source_name().to_owned(),
                    legal_claims: m.legal_claims.value.clone(),
                    certification: m.certification.value.clone(),
                    network: m.network.value.source_name().to_owned(),
                    external_execution: m.external_execution.value.source_name().to_owned(),
                    secret_material: m.secret_material.value.source_name().to_owned(),
                    key_material: m.key_material.value.source_name().to_owned(),
                    execution: m.execution.value.source_name().to_owned(),
                    security_claims: m.security_claims.value.source_name().to_owned(),
                    purpose: spanned_values(&m.purpose),
                    notes: m.notes.as_ref().map(|v| v.value.clone()),
                })
                .collect(),
            third_party_verifiers: program
                .third_party_verifiers
                .iter()
                .map(|v| IrThirdPartyVerifier {
                    name: v.name.value.clone(),
                    verifier_type: v.verifier_type.value.source_name().to_owned(),
                    independence: v.independence.value.source_name().to_owned(),
                    identity_mode: v.identity_mode.value.source_name().to_owned(),
                    verification_mode: v.verification_mode.value.source_name().to_owned(),
                    display_name: v.display_name.value.clone(),
                    organization: v.organization.value.clone(),
                    jurisdiction: v.jurisdiction.value.clone(),
                    allowed_scopes: spanned_values(&v.allowed_scopes),
                    disallowed_claims: spanned_values(&v.disallowed_claims),
                    network: v.network.value.source_name().to_owned(),
                    external_execution: v.external_execution.value.source_name().to_owned(),
                    secret_material: v.secret_material.value.source_name().to_owned(),
                    key_material: v.key_material.value.source_name().to_owned(),
                    execution: v.execution.value.source_name().to_owned(),
                    legal_claims: v.legal_claims.value.clone(),
                    certification: v.certification.value.clone(),
                    security_claims: v.security_claims.value.source_name().to_owned(),
                    purpose: spanned_values(&v.purpose),
                    notes: v.notes.as_ref().map(|value| value.value.clone()),
                })
                .collect(),
            public_conformance_reports: program
                .public_conformance_reports
                .iter()
                .map(|r| IrPublicConformanceReport {
                    name: r.name.value.clone(),
                    verifier: r.verifier.value.clone(),
                    suite: r.suite.value.clone(),
                    suite_version: r.suite_version.value.clone(),
                    source_artifact: r.source_artifact.value.clone(),
                    bytecode_artifact: r.bytecode_artifact.value.clone(),
                    evidence_map: r.evidence_map.value.clone(),
                    governance_profile: r.governance_profile.value.clone(),
                    regulatory_mapping: r.regulatory_mapping.value.clone(),
                    trust_ledger: r.trust_ledger.value.clone(),
                    security_report: r.security_report.value.clone(),
                    evidence_bundle: r.evidence_bundle.value.clone(),
                    trace: r.trace.value.clone(),
                    result: r.result.value.source_name().to_owned(),
                    reproducibility: r.reproducibility.value.source_name().to_owned(),
                    review_status: r.review_status.value.source_name().to_owned(),
                    claims: r
                        .claims
                        .iter()
                        .map(|c| IrPublicConformanceClaim {
                            id: c.id.value.clone(),
                            category: c.category.value.source_name().to_owned(),
                            statement: c.statement.value.clone(),
                            evidence_ref: c.evidence_ref.value.clone(),
                            status: c.status.value.source_name().to_owned(),
                        })
                        .collect(),
                    network: r.network.value.source_name().to_owned(),
                    external_execution: r.external_execution.value.source_name().to_owned(),
                    secret_material: r.secret_material.value.source_name().to_owned(),
                    key_material: r.key_material.value.source_name().to_owned(),
                    execution: r.execution.value.source_name().to_owned(),
                    legal_claims: r.legal_claims.value.clone(),
                    certification: r.certification.value.clone(),
                    security_claims: r.security_claims.value.source_name().to_owned(),
                    purpose: spanned_values(&r.purpose),
                    notes: r.notes.as_ref().map(|value| value.value.clone()),
                })
                .collect(),
            assertions: program
                .assertions
                .iter()
                .map(|assertion| IrAssertion {
                    name: assertion.name.value.clone(),
                    argument: assertion.argument.as_ref().map(|value| value.value.clone()),
                })
                .collect(),
            policies: program
                .policies
                .iter()
                .map(|policy| IrPolicy {
                    name: policy.name.value.clone(),
                    rules: policy
                        .rules
                        .iter()
                        .map(|declaration| IrPolicyRule {
                            effect: declaration.effect().into(),
                            rule: declaration.rule().value.source_name(),
                        })
                        .collect(),
                    on_violation: policy
                        .violation
                        .as_ref()
                        .map(|violation| IrPolicyViolation {
                            action: violation.action.value.source_name(),
                            trace_required: violation.trace_required,
                        }),
                })
                .collect(),
            failures: program
                .failures
                .iter()
                .map(|failure| IrFailure {
                    name: failure.name.value.clone(),
                    action: failure.action.value.clone(),
                    trace: "required".into(),
                })
                .collect(),
            capabilities: program
                .capabilities
                .iter()
                .map(|capability| IrCapability {
                    name: capability.name.value.clone(),
                    level: capability.level.value.as_str().to_owned(),
                    requires_approval: capability.requires_approval,
                })
                .collect(),
            enums: program
                .enums
                .iter()
                .map(|item| IrEnum {
                    name: item.name.value.clone(),
                    variants: item
                        .variants
                        .iter()
                        .map(|variant| variant.value.clone())
                        .collect(),
                })
                .collect(),
            types: program
                .types
                .iter()
                .map(|item| IrType {
                    name: item.name.value.clone(),
                    fields: item
                        .fields
                        .iter()
                        .map(|field| IrField {
                            name: field.name.value.clone(),
                            field_type: field.field_type.value.source_name().to_owned(),
                        })
                        .collect(),
                })
                .collect(),
            tools: program
                .tools
                .iter()
                .map(|tool| IrTool {
                    name: tool.name.value.clone(),
                    provider: resolved_provider(
                        tool.provider
                            .as_ref()
                            .map(|provider| provider.value.as_str()),
                    )
                    .to_owned(),
                    capability: tool.capability.value.clone(),
                    input: tool.input.value.clone(),
                    output: tool.output.value.clone(),
                })
                .collect(),
            models: program
                .models
                .iter()
                .map(|model| IrModel {
                    name: model.name.value.clone(),
                    provider: model.provider.value.clone(),
                    capability: model.capability.value.clone(),
                    input: model.input.value.clone(),
                    output: model.output.value.clone(),
                })
                .collect(),
            agents: program
                .agents
                .iter()
                .map(|agent| IrAgent {
                    name: agent.name.value.clone(),
                    approval: agent.effective_approval().as_str().to_owned(),
                    receives: agent
                        .receives
                        .iter()
                        .map(|receive| IrReceive {
                            message_type: receive.message_type.value.clone(),
                            from: receive.from.as_ref().map(|from| from.value.clone()),
                        })
                        .collect(),
                    sends: agent
                        .sends
                        .iter()
                        .map(|send| IrSend {
                            message_type: send.message_type.value.clone(),
                            to: send.to.value.clone(),
                        })
                        .collect(),
                    capabilities: agent
                        .capabilities
                        .iter()
                        .map(|capability| capability.value.clone())
                        .collect(),
                    tools: agent.tools.iter().map(|tool| tool.value.clone()).collect(),
                    models: agent
                        .models
                        .iter()
                        .map(|model| model.value.clone())
                        .collect(),
                    handlers: agent
                        .handlers
                        .iter()
                        .map(|handler| IrHandler {
                            message_type: handler.message_type.value.clone(),
                            binding: handler.binding.value.clone(),
                            instructions: handler
                                .instructions
                                .iter()
                                .map(|instruction| match instruction {
                                    HandlerInstruction::Emit { message_type, to } => {
                                        IrHandlerInstruction::Emit {
                                            message_type: message_type.value.clone(),
                                            to: to.value.clone(),
                                        }
                                    }
                                    HandlerInstruction::Trace { binding } => {
                                        IrHandlerInstruction::Trace {
                                            binding: binding.value.clone(),
                                        }
                                    }
                                    HandlerInstruction::Halt { .. } => IrHandlerInstruction::Halt,
                                    HandlerInstruction::IntrinsicCall { name, argument } => {
                                        IrHandlerInstruction::Intrinsic {
                                            name: name.value.clone(),
                                            argument: argument.value.clone(),
                                        }
                                    }
                                    HandlerInstruction::CallTool { tool, binding } => {
                                        IrHandlerInstruction::Call {
                                            tool: tool.value.clone(),
                                            binding: binding.value.clone(),
                                        }
                                    }
                                    HandlerInstruction::AskModel { model, binding } => {
                                        IrHandlerInstruction::Ask {
                                            model: model.value.clone(),
                                            binding: binding.value.clone(),
                                        }
                                    }
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
            protocols: program
                .protocols
                .iter()
                .map(|protocol| IrProtocol {
                    name: protocol.name.value.clone(),
                    steps: protocol
                        .steps
                        .iter()
                        .map(|step| IrProtocolStep {
                            from: step.from.value.clone(),
                            to: step.to.value.clone(),
                            act: step.act.value.clone(),
                            message_type: step.message_type.value.clone(),
                        })
                        .collect(),
                })
                .collect(),
            passports: program
                .passports
                .iter()
                .map(|passport| IrPassport {
                    name: passport.name.value.clone(),
                    agent: passport.agent.value.clone(),
                    agent_name: passport.agent_name.value.clone(),
                    global_id: passport.global_id.value.clone(),
                    identity: passport.identity.value.clone(),
                    provider: passport.provider.value.clone(),
                    version: passport.version.value.clone(),
                    ans_name: passport.ans_name.as_ref().map(|value| value.value.clone()),
                    country: passport.country.value.clone(),
                    jurisdiction: passport.jurisdiction.value.clone(),
                    data_residency: spanned_values(&passport.data_residency),
                    asn: passport.asn.as_ref().map(|asn| IrPassportAsn {
                        registry: asn.registry.value.clone(),
                        number: asn.number.value.clone(),
                        holder: asn.holder.value.clone(),
                        country: asn.country.value.clone(),
                    }),
                    model: passport.model.as_ref().map(|value| value.value.clone()),
                    risk_level: passport.risk_level.value.clone(),
                    data_scope: spanned_values(&passport.data_scope),
                    intent: passport.intent.value.clone(),
                    intended_use: spanned_values(&passport.intended_use),
                    prohibited_use: spanned_values(&passport.prohibited_use),
                    attestations: spanned_values(&passport.attestations),
                })
                .collect(),
        }
    }
}

fn spanned_values(values: &[argorix_parser::span::Spanned<String>]) -> Vec<String> {
    values.iter().map(|value| value.value.clone()).collect()
}

pub fn resolved_provider(provider: Option<&str>) -> &str {
    provider.unwrap_or("simulated")
}

#[cfg(test)]
mod tests {
    use super::IrProgram;
    use argorix_parser::parse_source;

    #[test]
    fn lowers_policy_v2_metadata_to_ir_017() {
        let program = parse_source(
            r#"
            module main
            assert all_tool_calls_traced
            policy ProviderSafety {
                deny external_execution
                on violation { action block trace required }
            }
            "#,
        )
        .unwrap();
        let ir = IrProgram::from(&program);
        assert_eq!(ir.ir_version, "0.34");
        assert_eq!(ir.assertions.len(), 1);
        assert_eq!(ir.policies[0].name, "ProviderSafety");
        assert_eq!(ir.policies[0].rules[0].effect, "deny");
        assert_eq!(ir.policies[0].rules[0].rule, "external_execution");
        assert_eq!(
            ir.policies[0].on_violation.as_ref().unwrap().action,
            "block"
        );
        assert!(ir.policies[0].on_violation.as_ref().unwrap().trace_required);
    }

    #[test]
    fn ir_019_preserves_passport_metadata() {
        let program = parse_source(
            r#"
            module main
            agent ResearchAgent {}
            passport RiskAnalyzerPassport {
                agent ResearchAgent
                agent_name "Risk Analyzer"
                global_id "argx:agent:1"
                identity "did:argorix:risk-v1"
                provider "Argorix"
                version "1.0.0"
                ans_name "argx://risk.v1.sovereign"
                country "CL"
                jurisdiction "CL"
                data_residency ["CL", "EU"]
                asn { registry "LACNIC" number "AS-PLACEHOLDER" holder "Argorix Labs" country "CL" }
                risk_level "high"
                intent "risk_analysis"
                attestations ["redteam"]
            }
            "#,
        )
        .unwrap();
        let ir = IrProgram::from(&program);
        assert_eq!(ir.ir_version, "0.34");
        assert_eq!(ir.passports.len(), 1);
        assert_eq!(ir.passports[0].agent, "ResearchAgent");
        assert_eq!(ir.passports[0].data_residency, vec!["CL", "EU"]);
        assert_eq!(ir.passports[0].asn.as_ref().unwrap().registry, "LACNIC");
        assert_eq!(
            ir.passports[0].ans_name.as_deref(),
            Some("argx://risk.v1.sovereign")
        );
    }

    #[test]
    fn ir_020_preserves_provider_harness_metadata() {
        let program = parse_source(
            r#"
            module main
            provider OpenAI { kind external enabled false dry_run_only true requires feature_flag requires approval }
            type UserPrompt { content: string }
            type DraftAnswer { content: string }
            harness OpenAIHarness {
                provider OpenAI
                mode dry_run
                network denied
                secrets denied
                filesystem none
                max_steps 10
                timeout_ms 1000
                input_contract UserPrompt
                output_contract DraftAnswer
                attestations ["dry-run"]
            }
            "#,
        )
        .unwrap();
        let ir = IrProgram::from(&program);
        assert_eq!(ir.ir_version, "0.34");
        assert_eq!(ir.provider_harnesses.len(), 1);
        let harness = &ir.provider_harnesses[0];
        assert_eq!(harness.name, "OpenAIHarness");
        assert_eq!(harness.mode, "dry_run");
        assert_eq!(harness.max_steps, Some(10));
        assert_eq!(harness.input_contract.as_deref(), Some("UserPrompt"));
        assert_eq!(harness.attestations, vec!["dry-run"]);
    }
}
