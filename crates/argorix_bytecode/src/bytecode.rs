use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

const EXTERNAL_ENTITIES: [&str; 5] = ["User", "System", "Runtime", "Memory", "Tool"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeProgram {
    pub bytecode_version: String,
    pub language: String,
    pub module: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<BytecodeModule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<BytecodeModuleImport>,
    #[serde(default)]
    pub providers: Vec<BytecodeProviderContract>,
    #[serde(default)]
    pub provider_harnesses: Vec<BytecodeProviderHarness>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<BytecodeFeature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<BytecodeSecret>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapters: Vec<BytecodeAdapter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapter_profiles: Vec<BytecodeAdapterProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cryptos: Vec<BytecodeCrypto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crypto_boundaries: Vec<BytecodeCryptoBoundary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub did_methods: Vec<BytecodeDidMethod>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_boundaries: Vec<BytecodeATrustBoundary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_identities: Vec<BytecodeATrustIdentity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_credential_contracts: Vec<BytecodeATrustCredentialContract>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_handshakes: Vec<BytecodeATrustHandshake>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trust_ledgers: Vec<BytecodeTrustLedger>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_bridge_contracts: Vec<BytecodeMcpBridgeContract>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub a2a_bridge_contracts: Vec<BytecodeA2ABridgeContract>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub atrust_evidence_maps: Vec<BytecodeATrustEvidenceMap>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub governance_profiles: Vec<BytecodeGovernanceProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regulatory_mappings: Vec<BytecodeRegulatoryMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub third_party_verifiers: Vec<BytecodeThirdPartyVerifier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_conformance_reports: Vec<BytecodePublicConformanceReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_hardening_profiles: Vec<BytecodeRuntimeHardeningProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threat_models: Vec<BytecodeThreatModel>,
    #[serde(default)]
    pub assertions: Vec<BytecodeAssertion>,
    #[serde(default)]
    pub policies: Vec<BytecodePolicy>,
    #[serde(default)]
    pub types: Vec<BytecodeType>,
    #[serde(default)]
    pub enums: Vec<String>,
    #[serde(default)]
    pub failures: Vec<BytecodeFailure>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passports: Vec<BytecodePassport>,
    pub agents: Vec<BytecodeAgent>,
    pub capabilities: Vec<BytecodeCapability>,
    #[serde(default)]
    pub tools: Vec<BytecodeTool>,
    #[serde(default)]
    pub models: Vec<BytecodeModel>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeModule {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeModuleImport {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeProviderContract {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub dry_run_only: bool,
    pub requires_feature_flag: bool,
    pub requires_explicit_approval: bool,
    #[serde(default)]
    pub allowed_targets: Vec<String>,
    #[serde(default)]
    pub allowed_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeFeature {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub status: String,
    pub default: String,
    pub requires_approval: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeSecret {
    pub name: String,
    pub handle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_by: Option<String>,
    pub scope: String,
    pub access: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeAdapter {
    pub name: String,
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    pub mode: String,
    pub execution: String,
    pub network: String,
    pub secrets: String,
    pub filesystem: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_contract: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<String>,
    #[serde(default)]
    pub conformance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeAdapterProfile {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_contract: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_contract: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub required_conformance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeCrypto {
    pub name: String,
    pub kind: String,
    pub status: String,
    pub strength: String,
    pub purpose: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_bits: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_key_bits: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeCryptoBoundary {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_hash_bits: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_quantum_ready: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hybrid_allowed: Option<bool>,
    pub key_material: String,
    pub secret_material: String,
    pub execution: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeDidMethod {
    pub name: String,
    pub status: String,
    pub resolution: String,
    pub ledger: String,
    pub crypto_boundary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance: Option<String>,
    pub purpose: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeATrustBoundary {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_quantum_ready: Option<String>,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeATrustIdentity {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeATrustCredentialContract {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeATrustHandshake {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeTrustLedger {
    pub name: String,
    pub scope: String,
    pub mode: String,
    pub hash_algorithm: String,
    pub chain_policy: String,
    pub entries: Vec<BytecodeTrustLedgerEntry>,
    pub chain_root: String,
    pub network: String,
    pub key_material: String,
    pub secret_material: String,
    pub execution: String,
    pub evidence: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeTrustLedgerEntry {
    pub id: String,
    pub kind: String,
    pub subject: String,
    pub previous_hash: String,
    pub entry_hash: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeMcpBridgeContract {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeA2ABridgeContract {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeATrustEvidenceMap {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeGovernanceProfile {
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
    pub controls: Vec<BytecodeGovernanceControl>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeGovernanceControl {
    pub id: String,
    pub category: String,
    pub requirement: String,
    pub evidence_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeRegulatoryMapping {
    pub name: String,
    pub governance_profile: String,
    pub evidence_map: String,
    pub jurisdiction: String,
    pub framework: String,
    pub obligations: Vec<BytecodeRegulatoryObligation>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeRegulatoryObligation {
    pub id: String,
    pub source: String,
    pub requirement: String,
    pub control: String,
    pub evidence_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeThirdPartyVerifier {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodePublicConformanceReport {
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
    pub claims: Vec<BytecodePublicConformanceClaim>,
    pub network: String,
    pub external_execution: String,
    pub secret_material: String,
    pub key_material: String,
    pub execution: String,
    pub legal_claims: String,
    pub certification: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodePublicConformanceClaim {
    pub id: String,
    pub category: String,
    pub statement: String,
    pub evidence_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeRuntimeHardeningProfile {
    pub name: String,
    pub scope: String,
    pub mode: String,
    pub enforcement: String,
    pub sandbox: String,
    pub provider_execution: String,
    pub external_providers: String,
    pub network: String,
    pub tool_execution: String,
    pub agent_execution: String,
    pub filesystem_access: String,
    pub env_access: String,
    pub secret_material: String,
    pub key_material: String,
    pub allowlist: String,
    pub deny_by_default: bool,
    pub approval: String,
    pub audit_log: String,
    pub evidence: String,
    pub incident_response: String,
    pub evidence_map: String,
    pub governance_profile: String,
    pub public_conformance_report: String,
    pub protected_assets: Vec<String>,
    pub runtime_boundaries: Vec<String>,
    pub residual_risk: String,
    pub review_status: String,
    pub assurance: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeThreatModel {
    pub name: String,
    pub hardening_profile: String,
    pub evidence_map: String,
    pub governance_profile: String,
    pub public_conformance_report: String,
    pub methodology: String,
    pub scope: String,
    pub review_status: String,
    pub assets: Vec<BytecodeThreatAsset>,
    pub threats: Vec<BytecodeThreat>,
    pub mitigations: Vec<BytecodeThreatMitigation>,
    pub residual_risk: String,
    pub risk_acceptance: String,
    pub network: String,
    pub external_execution: String,
    pub tool_execution: String,
    pub agent_execution: String,
    pub secret_material: String,
    pub key_material: String,
    pub execution: String,
    pub security_claims: String,
    pub purpose: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeThreatAsset {
    pub id: String,
    pub category: String,
    pub description: String,
    pub sensitivity: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeThreat {
    pub id: String,
    pub category: String,
    pub target: String,
    pub impact: String,
    pub mitigation: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeThreatMitigation {
    pub id: String,
    pub category: String,
    pub control_ref: String,
    pub evidence_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeProviderHarness {
    pub name: String,
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    pub mode: String,
    pub network: String,
    pub secrets: String,
    pub filesystem: String,
    pub max_steps: Option<u64>,
    pub timeout_ms: Option<u64>,
    pub input_contract: Option<String>,
    pub output_contract: Option<String>,
    #[serde(default)]
    pub attestations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeAssertion {
    pub name: String,
    pub argument: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodePolicy {
    pub name: String,
    pub rules: Vec<BytecodePolicyRule>,
    pub on_violation: Option<BytecodePolicyViolation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodePolicyRule {
    pub effect: String,
    pub rule: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodePolicyViolation {
    pub action: String,
    pub trace_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeType {
    pub name: String,
    pub fields: Vec<BytecodeTypeField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeTypeField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeFailure {
    pub name: String,
    pub action: String,
    pub trace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodePassport {
    pub name: String,
    pub agent: String,
    pub agent_name: String,
    pub global_id: String,
    pub identity: String,
    pub provider: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ans_name: Option<String>,
    pub country: String,
    pub jurisdiction: String,
    #[serde(default)]
    pub data_residency: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asn: Option<BytecodePassportAsn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub risk_level: String,
    #[serde(default)]
    pub data_scope: Vec<String>,
    pub intent: String,
    #[serde(default)]
    pub intended_use: Vec<String>,
    #[serde(default)]
    pub prohibited_use: Vec<String>,
    #[serde(default)]
    pub attestations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodePassportAsn {
    pub registry: String,
    pub number: String,
    pub holder: String,
    pub country: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeModel {
    pub name: String,
    pub provider: String,
    pub capability: String,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeTool {
    pub name: String,
    #[serde(default = "default_provider")]
    pub provider: String,
    pub capability: String,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeAgent {
    pub name: String,
    pub approval: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeCapability {
    pub name: String,
    pub level: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum Instruction {
    DeclareProviderContract {
        name: String,
        kind: String,
        enabled: bool,
        dry_run_only: bool,
        requires_feature_flag: bool,
        requires_explicit_approval: bool,
        #[serde(default)]
        allowed_targets: Vec<String>,
        #[serde(default)]
        allowed_capabilities: Vec<String>,
    },
    DeclareAgent {
        name: String,
        approval: String,
    },
    DeclareCapability {
        name: String,
        level: String,
        requires_approval: bool,
    },
    DeclareProtocol {
        name: String,
    },
    DeclareAssertion {
        name: String,
        argument: Option<String>,
    },
    DeclareFailure {
        name: String,
        action: String,
        trace: String,
    },
    VerifyAssertion {
        name: String,
        argument: Option<String>,
    },
    PolicyReport,
    DeclareTool {
        name: String,
        #[serde(default = "default_provider")]
        provider: String,
        capability: String,
        input: String,
        output: String,
    },
    AuthorizeTool {
        agent: String,
        tool: String,
    },
    DeclareModel {
        name: String,
        provider: String,
        capability: String,
        input: String,
        output: String,
    },
    AuthorizeModel {
        agent: String,
        model: String,
    },
    DeclareHandler {
        agent: String,
        message_type: String,
        binding: String,
    },
    EmitMessage {
        agent: String,
        message_type: String,
        to: String,
    },
    TraceValue {
        agent: String,
        binding: String,
    },
    HandlerHalt {
        agent: String,
    },
    InvokeIntrinsic {
        agent: String,
        name: String,
        argument: String,
    },
    CallTool {
        agent: String,
        tool: String,
        binding: String,
    },
    AskModel {
        agent: String,
        model: String,
        binding: String,
    },
    EndHandler,
    SendMessage {
        from: String,
        to: String,
        act: String,
        message_type: String,
    },
    RequireCapability {
        agent: String,
        capability: String,
    },
    RequireApproval {
        agent: String,
        capability: String,
    },
    Trace {
        message: String,
    },
    Halt {
        reason: String,
    },
    End,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BytecodeError {
    #[error("bytecode_version is required")]
    MissingVersion,
    #[error("unsupported bytecode version `{0}`")]
    UnsupportedVersion(String),
    #[error("bytecode must declare at least one agent")]
    NoAgents,
    #[error("bytecode must declare at least one protocol, handler, or SendMessage instruction")]
    NoProtocolOrMessages,
    #[error("unknown sender `{0}` in SendMessage instruction")]
    UnknownSender(String),
    #[error("unknown receiver `{0}` in SendMessage instruction")]
    UnknownReceiver(String),
    #[error("SendMessage field `{0}` must not be empty")]
    EmptySendField(&'static str),
    #[error("RequireApproval references unknown agent `{0}`")]
    UnknownApprovalAgent(String),
    #[error("RequireCapability references unknown agent `{0}`")]
    UnknownCapabilityAgent(String),
    #[error("bytecode must terminate with an End instruction")]
    MissingEnd,
    #[error("bytecode contains an unknown instruction")]
    UnknownInstruction,
    #[error("handler instruction references unknown agent `{0}`")]
    UnknownHandlerAgent(String),
    #[error("bytecode tool `{0}` is not declared")]
    UnknownTool(String),
    #[error("bytecode tool `{tool}` references unknown capability `{capability}`")]
    UnknownToolCapability { tool: String, capability: String },
    #[error("unsupported tool provider `{0}`")]
    UnknownToolProvider(String),
    #[error("bytecode model `{0}` is not declared or has invalid capability")]
    UnknownModel(String),
    #[error("unsupported model provider `{0}`")]
    UnknownModelProvider(String),
    #[error("provider contracts require bytecode version 0.11 through 0.15")]
    ContractsRequireV011,
    #[error("duplicate provider contract `{0}`")]
    DuplicateProviderContract(String),
    #[error("invalid provider contract `{name}`: {reason}")]
    InvalidProviderContract { name: String, reason: String },
    #[error("provider contract declarations do not match the top-level providers collection")]
    ProviderContractDeclarationMismatch,
    #[error("module metadata requires bytecode version 0.16")]
    ModulesRequireV016,
    #[error("module import edge references unknown module `{0}`")]
    UnknownModuleImport(String),
    #[error("policy metadata requires bytecode version 0.17")]
    PoliciesRequireV017,
    #[error("duplicate policy `{0}`")]
    DuplicatePolicy(String),
    #[error("invalid policy `{name}`: {reason}")]
    InvalidPolicy { name: String, reason: String },
    #[error("message contracts require bytecode version 0.18")]
    MessageContractsRequireV018,
    #[error("invalid message contract `{name}`: {reason}")]
    InvalidMessageContract { name: String, reason: String },
    #[error("agent passports require bytecode version 0.19")]
    PassportsRequireV019,
    #[error("invalid passport `{name}`: {reason}")]
    InvalidPassport { name: String, reason: String },
    #[error("provider harness metadata requires bytecode version 0.20 or later")]
    HarnessesRequireV020,
    #[error("duplicate provider harness `{0}`")]
    DuplicateProviderHarness(String),
    #[error("invalid provider harness `{name}`: {reason}")]
    InvalidProviderHarness { name: String, reason: String },
    #[error("feature flag metadata requires bytecode version 0.21")]
    FeaturesRequireV021,
    #[error("duplicate feature `{0}`")]
    DuplicateFeature(String),
    #[error("invalid feature `{name}`: {reason}")]
    InvalidFeature { name: String, reason: String },
    #[error("secret boundary metadata requires bytecode version 0.21")]
    SecretsRequireV021,
    #[error("duplicate secret `{0}`")]
    DuplicateSecret(String),
    #[error("invalid secret `{name}`: {reason}")]
    InvalidSecret { name: String, reason: String },
    #[error("adapter framework metadata requires bytecode version 0.22")]
    AdaptersRequireV022,
    #[error("duplicate adapter `{0}`")]
    DuplicateAdapter(String),
    #[error("invalid adapter `{name}`: {reason}")]
    InvalidAdapter { name: String, reason: String },
    #[error("adapter profile metadata requires bytecode version 0.23")]
    AdapterProfilesRequireV023,
    #[error("duplicate adapter profile `{0}`")]
    DuplicateAdapterProfile(String),
    #[error("invalid adapter profile `{name}`: {reason}")]
    InvalidAdapterProfile { name: String, reason: String },
    #[error("crypto primitive metadata requires bytecode version 0.24")]
    CryptosRequireV024,
    #[error("duplicate crypto `{0}`")]
    DuplicateCrypto(String),
    #[error("invalid crypto `{name}`: {reason}")]
    InvalidCrypto { name: String, reason: String },
    #[error("crypto boundary metadata requires bytecode version 0.25")]
    CryptoBoundariesRequireV025,
    #[error("duplicate crypto boundary `{0}`")]
    DuplicateCryptoBoundary(String),
    #[error("invalid crypto boundary `{name}`: {reason}")]
    InvalidCryptoBoundary { name: String, reason: String },
    #[error("duplicate atrust handshake `{0}`")]
    DuplicateATrustHandshake(String),
    #[error("invalid atrust handshake `{name}`: {reason}")]
    InvalidATrustHandshake { name: String, reason: String },
    #[error("duplicate trust ledger `{0}`")]
    DuplicateTrustLedger(String),
    #[error("invalid trust ledger `{name}`: {reason}")]
    InvalidTrustLedger { name: String, reason: String },
    #[error("duplicate mcp bridge contract `{0}`")]
    DuplicateMcpBridgeContract(String),
    #[error("invalid mcp bridge contract `{name}`: {reason}")]
    InvalidMcpBridgeContract { name: String, reason: String },
    #[error("duplicate a2a bridge contract `{0}`")]
    DuplicateA2ABridgeContract(String),
    #[error("invalid a2a bridge contract `{name}`: {reason}")]
    InvalidA2ABridgeContract { name: String, reason: String },
    #[error("atrust_evidence_maps require bytecode_version 0.32")]
    ATrustEvidenceMapsRequireV032,
    #[error("duplicate atrust_evidence_map `{0}`")]
    DuplicateATrustEvidenceMap(String),
    #[error("invalid atrust_evidence_map `{name}`: {reason}")]
    InvalidATrustEvidenceMap { name: String, reason: String },
    #[error("governance_profiles and regulatory_mappings require bytecode_version 0.33")]
    GovernanceMappingsRequireV033,
    #[error("duplicate governance_profile `{0}`")]
    DuplicateGovernanceProfile(String),
    #[error("invalid governance_profile `{name}`: {reason}")]
    InvalidGovernanceProfile { name: String, reason: String },
    #[error("duplicate regulatory_mapping `{0}`")]
    DuplicateRegulatoryMapping(String),
    #[error("invalid regulatory_mapping `{name}`: {reason}")]
    InvalidRegulatoryMapping { name: String, reason: String },
    #[error("third_party_verifiers and public_conformance_reports require bytecode_version 0.34")]
    PublicConformanceRequiresV034,
    #[error("duplicate third_party_verifier `{0}`")]
    DuplicateThirdPartyVerifier(String),
    #[error("invalid third_party_verifier `{name}`: {reason}")]
    InvalidThirdPartyVerifier { name: String, reason: String },
    #[error("duplicate public_conformance_report `{0}`")]
    DuplicatePublicConformanceReport(String),
    #[error("invalid public_conformance_report `{name}`: {reason}")]
    InvalidPublicConformanceReport { name: String, reason: String },
    #[error("runtime_hardening_profiles and threat_models require bytecode_version 0.35")]
    RuntimeHardeningRequiresV035,
    #[error("duplicate runtime_hardening_profile `{0}`")]
    DuplicateRuntimeHardeningProfile(String),
    #[error("invalid runtime_hardening_profile `{name}`: {reason}")]
    InvalidRuntimeHardeningProfile { name: String, reason: String },
    #[error("duplicate threat_model `{0}`")]
    DuplicateThreatModel(String),
    #[error("invalid threat_model `{name}`: {reason}")]
    InvalidThreatModel { name: String, reason: String },
}

pub fn verify_bytecode(program: &BytecodeProgram) -> Result<(), Vec<BytecodeError>> {
    let mut errors = Vec::new();
    if program.bytecode_version.trim().is_empty() {
        errors.push(BytecodeError::MissingVersion);
    } else if !matches!(
        program.bytecode_version.as_str(),
        "0.3"
            | "0.5"
            | "0.6"
            | "0.7"
            | "0.8"
            | "0.9"
            | "0.10"
            | "0.11"
            | "0.12"
            | "0.13"
            | "0.14"
            | "0.15"
            | "0.16"
            | "0.17"
            | "0.18"
            | "0.19"
            | "0.20"
            | "0.21"
            | "0.22"
            | "0.23"
            | "0.24"
            | "0.25"
            | "0.26"
            | "0.27"
            | "0.28"
            | "0.29"
            | "0.30"
            | "0.31"
            | "0.32"
            | "0.33"
            | "0.34"
            | "0.35"
    ) {
        errors.push(BytecodeError::UnsupportedVersion(
            program.bytecode_version.clone(),
        ));
    }
    if program.agents.is_empty() {
        errors.push(BytecodeError::NoAgents);
    }
    if !matches!(
        program.bytecode_version.as_str(),
        "0.21"
            | "0.20"
            | "0.19"
            | "0.18"
            | "0.17"
            | "0.16"
            | "0.11"
            | "0.12"
            | "0.13"
            | "0.14"
            | "0.15"
            | "0.22"
            | "0.23"
            | "0.24"
            | "0.25"
            | "0.26"
            | "0.27"
            | "0.28"
            | "0.29"
            | "0.30"
            | "0.31"
            | "0.32"
            | "0.33"
            | "0.34"
            | "0.35"
    ) && !program.providers.is_empty()
    {
        errors.push(BytecodeError::ContractsRequireV011);
    }
    if (!program.modules.is_empty() || !program.imports.is_empty())
        && !matches!(
            program.bytecode_version.as_str(),
            "0.16"
                | "0.17"
                | "0.18"
                | "0.19"
                | "0.20"
                | "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::ModulesRequireV016);
    }
    let module_names: HashSet<&str> = program
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    for import in &program.imports {
        for endpoint in [import.from.as_str(), import.to.as_str()] {
            if !module_names.contains(endpoint) {
                errors.push(BytecodeError::UnknownModuleImport(endpoint.to_owned()));
            }
        }
    }
    if !program.policies.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.17"
                | "0.18"
                | "0.19"
                | "0.20"
                | "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::PoliciesRequireV017);
    }
    validate_policies(program, &mut errors);
    validate_message_contracts(program, &mut errors);
    validate_passports(program, &mut errors);
    validate_provider_harnesses(program, &mut errors);
    validate_features(program, &mut errors);
    validate_secrets(program, &mut errors);
    if !program.adapters.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::AdaptersRequireV022);
    }
    if !program.adapter_profiles.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::AdapterProfilesRequireV023);
    }
    if !program.cryptos.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::CryptosRequireV024);
    }
    if !program.crypto_boundaries.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::CryptoBoundariesRequireV025);
    }
    if !program.atrust_credential_contracts.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.28" | "0.29" | "0.30" | "0.31" | "0.32" | "0.33" | "0.34" | "0.35"
        )
    {
        errors.push(BytecodeError::UnsupportedVersion(
            program.bytecode_version.clone(),
        ));
    }
    if !program.atrust_handshakes.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.29" | "0.30" | "0.31" | "0.32" | "0.33" | "0.34" | "0.35"
        )
    {
        errors.push(BytecodeError::UnsupportedVersion(
            program.bytecode_version.clone(),
        ));
    }
    validate_atrust_handshakes(program, &mut errors);
    if !program.trust_ledgers.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.30" | "0.31" | "0.32" | "0.33" | "0.34" | "0.35"
        )
    {
        errors.push(BytecodeError::UnsupportedVersion(
            program.bytecode_version.clone(),
        ));
    }
    validate_trust_ledgers(program, &mut errors);
    if (!program.mcp_bridge_contracts.is_empty() || !program.a2a_bridge_contracts.is_empty())
        && !matches!(
            program.bytecode_version.as_str(),
            "0.31" | "0.32" | "0.33" | "0.34" | "0.35"
        )
    {
        errors.push(BytecodeError::UnsupportedVersion(
            program.bytecode_version.clone(),
        ));
    }
    validate_mcp_bridge_contracts(program, &mut errors);
    validate_a2a_bridge_contracts(program, &mut errors);
    if !program.atrust_evidence_maps.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.32" | "0.33" | "0.34" | "0.35"
        )
    {
        errors.push(BytecodeError::ATrustEvidenceMapsRequireV032);
    }
    validate_atrust_evidence_maps(program, &mut errors);
    if (!program.governance_profiles.is_empty() || !program.regulatory_mappings.is_empty())
        && !matches!(program.bytecode_version.as_str(), "0.33" | "0.34" | "0.35")
    {
        errors.push(BytecodeError::GovernanceMappingsRequireV033);
    }
    validate_governance_profiles(program, &mut errors);
    validate_regulatory_mappings(program, &mut errors);
    if (!program.third_party_verifiers.is_empty() || !program.public_conformance_reports.is_empty())
        && !matches!(program.bytecode_version.as_str(), "0.34" | "0.35")
    {
        errors.push(BytecodeError::PublicConformanceRequiresV034);
    }
    validate_third_party_verifiers(program, &mut errors);
    validate_public_conformance_reports(program, &mut errors);
    if (!program.runtime_hardening_profiles.is_empty() || !program.threat_models.is_empty())
        && program.bytecode_version != "0.35"
    {
        errors.push(BytecodeError::RuntimeHardeningRequiresV035);
    }
    validate_runtime_hardening_profiles(program, &mut errors);
    validate_threat_models(program, &mut errors);
    validate_adapters(program, &mut errors);
    validate_adapter_profiles(program, &mut errors);
    validate_cryptos(program, &mut errors);
    validate_crypto_boundaries(program, &mut errors);
    let mut provider_contract_names = HashSet::new();
    for contract in &program.providers {
        if !provider_contract_names.insert(contract.name.as_str()) {
            errors.push(BytecodeError::DuplicateProviderContract(
                contract.name.clone(),
            ));
        }
        if let Some(reason) = invalid_contract_reason(contract) {
            errors.push(BytecodeError::InvalidProviderContract {
                name: contract.name.clone(),
                reason: reason.into(),
            });
        }
    }

    let agents: HashSet<&str> = program
        .agents
        .iter()
        .map(|agent| agent.name.as_str())
        .collect();
    let capabilities: HashSet<&str> = program
        .capabilities
        .iter()
        .map(|capability| capability.name.as_str())
        .collect();
    let tools: HashSet<&str> = program
        .tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect();
    let models: HashSet<&str> = program
        .models
        .iter()
        .map(|model| model.name.as_str())
        .collect();
    for tool in &program.tools {
        if tool.provider != "simulated" {
            errors.push(BytecodeError::UnknownToolProvider(tool.provider.clone()));
        }
        if !capabilities.contains(tool.capability.as_str()) {
            errors.push(BytecodeError::UnknownToolCapability {
                tool: tool.name.clone(),
                capability: tool.capability.clone(),
            });
        }
    }
    for model in &program.models {
        if model.provider != "simulated" {
            errors.push(BytecodeError::UnknownModelProvider(model.provider.clone()));
        }
        if !capabilities.contains(model.capability.as_str()) {
            errors.push(BytecodeError::UnknownModel(model.name.clone()));
        }
    }
    validate_contract_allowlists(program, &capabilities, &mut errors);
    let mut has_protocol_or_message = false;
    let contract_declarations = program
        .instructions
        .iter()
        .filter_map(instruction_contract)
        .collect::<Vec<_>>();
    if contract_declarations != program.providers {
        errors.push(BytecodeError::ProviderContractDeclarationMismatch);
    }
    for instruction in &program.instructions {
        match instruction {
            Instruction::DeclareProtocol { .. } => has_protocol_or_message = true,
            Instruction::DeclareHandler { agent, .. } => {
                has_protocol_or_message = true;
                if !agents.contains(agent.as_str()) {
                    errors.push(BytecodeError::UnknownHandlerAgent(agent.clone()));
                }
            }
            Instruction::SendMessage {
                from,
                to,
                act,
                message_type,
            } => {
                has_protocol_or_message = true;
                for (field, value) in [
                    ("from", from.as_str()),
                    ("to", to.as_str()),
                    ("act", act.as_str()),
                    ("message_type", message_type.as_str()),
                ] {
                    if value.trim().is_empty() {
                        errors.push(BytecodeError::EmptySendField(field));
                    }
                }
                if !agents.contains(from.as_str()) && !EXTERNAL_ENTITIES.contains(&from.as_str()) {
                    errors.push(BytecodeError::UnknownSender(from.clone()));
                }
                if !agents.contains(to.as_str()) && !EXTERNAL_ENTITIES.contains(&to.as_str()) {
                    errors.push(BytecodeError::UnknownReceiver(to.clone()));
                }
            }
            Instruction::RequireApproval { agent, .. } if !agents.contains(agent.as_str()) => {
                errors.push(BytecodeError::UnknownApprovalAgent(agent.clone()));
            }
            Instruction::RequireCapability { agent, .. } if !agents.contains(agent.as_str()) => {
                errors.push(BytecodeError::UnknownCapabilityAgent(agent.clone()));
            }
            Instruction::AuthorizeTool { agent, tool } => {
                if !agents.contains(agent.as_str()) {
                    errors.push(BytecodeError::UnknownHandlerAgent(agent.clone()));
                }
                if !tools.contains(tool.as_str()) {
                    errors.push(BytecodeError::UnknownTool(tool.clone()));
                }
            }
            Instruction::CallTool { agent, tool, .. } => {
                if !agents.contains(agent.as_str()) {
                    errors.push(BytecodeError::UnknownHandlerAgent(agent.clone()));
                }
                if !tools.contains(tool.as_str()) {
                    errors.push(BytecodeError::UnknownTool(tool.clone()));
                }
            }
            Instruction::AuthorizeModel { agent, model }
            | Instruction::AskModel { agent, model, .. } => {
                if !agents.contains(agent.as_str()) {
                    errors.push(BytecodeError::UnknownHandlerAgent(agent.clone()));
                }
                if !models.contains(model.as_str()) {
                    errors.push(BytecodeError::UnknownModel(model.clone()));
                }
            }
            Instruction::EmitMessage { agent, .. }
            | Instruction::TraceValue { agent, .. }
            | Instruction::HandlerHalt { agent }
            | Instruction::InvokeIntrinsic { agent, .. }
                if !agents.contains(agent.as_str()) =>
            {
                errors.push(BytecodeError::UnknownHandlerAgent(agent.clone()));
            }
            Instruction::Unknown => errors.push(BytecodeError::UnknownInstruction),
            _ => {}
        }
    }

    if !has_protocol_or_message {
        errors.push(BytecodeError::NoProtocolOrMessages);
    }
    if !matches!(program.instructions.last(), Some(Instruction::End)) {
        errors.push(BytecodeError::MissingEnd);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn default_provider() -> String {
    "simulated".into()
}

fn invalid_contract_reason(contract: &BytecodeProviderContract) -> Option<&'static str> {
    if contract.name.trim().is_empty() {
        Some("name must not be empty")
    } else if contract.name == "simulated" {
        Some("`simulated` is reserved for the executable provider")
    } else if contract.kind != "external" {
        Some("kind must be external")
    } else if contract.enabled {
        Some("external contracts must be disabled")
    } else if !contract.dry_run_only {
        Some("external contracts must be dry-run-only")
    } else if !contract.requires_feature_flag {
        Some("external contracts require a feature flag")
    } else if !contract.requires_explicit_approval {
        Some("external contracts require explicit approval")
    } else {
        None
    }
}

fn validate_contract_allowlists(
    program: &BytecodeProgram,
    capabilities: &HashSet<&str>,
    errors: &mut Vec<BytecodeError>,
) {
    for contract in &program.providers {
        if program.bytecode_version == "0.11"
            && (!contract.allowed_targets.is_empty() || !contract.allowed_capabilities.is_empty())
        {
            errors.push(BytecodeError::InvalidProviderContract {
                name: contract.name.clone(),
                reason: "Bytecode 0.11 provider allowlists must be empty".into(),
            });
            continue;
        }
        if !matches!(
            program.bytecode_version.as_str(),
            "0.12" | "0.13" | "0.14" | "0.15" | "0.16" | "0.17" | "0.18" | "0.19" | "0.20" | "0.21"
        ) {
            continue;
        }
        let mut seen_targets = HashSet::new();
        for target in &contract.allowed_targets {
            if !seen_targets.insert(target.as_str()) {
                errors.push(BytecodeError::InvalidProviderContract {
                    name: contract.name.clone(),
                    reason: format!("duplicate allowed target `{target}`"),
                });
                continue;
            }
            let tool = program.tools.iter().find(|item| item.name == *target);
            let model = program.models.iter().find(|item| item.name == *target);
            let required = match (tool, model) {
                (None, None) => {
                    errors.push(BytecodeError::InvalidProviderContract {
                        name: contract.name.clone(),
                        reason: format!("unknown allowlist target `{target}`"),
                    });
                    None
                }
                (Some(_), Some(_)) => {
                    errors.push(BytecodeError::InvalidProviderContract {
                        name: contract.name.clone(),
                        reason: format!("ambiguous allowlist target `{target}`"),
                    });
                    None
                }
                (Some(tool), None) => Some(tool.capability.as_str()),
                (None, Some(model)) => Some(model.capability.as_str()),
            };
            if let Some(required) = required {
                if !contract.allowed_capabilities.is_empty()
                    && !contract
                        .allowed_capabilities
                        .iter()
                        .any(|item| item == required)
                {
                    errors.push(BytecodeError::InvalidProviderContract {
                        name: contract.name.clone(),
                        reason: format!(
                            "allowlist target `{target}` requires capability `{required}`"
                        ),
                    });
                }
            }
        }
        let mut seen_capabilities = HashSet::new();
        for capability in &contract.allowed_capabilities {
            if !seen_capabilities.insert(capability.as_str()) {
                errors.push(BytecodeError::InvalidProviderContract {
                    name: contract.name.clone(),
                    reason: format!("duplicate allowed capability `{capability}`"),
                });
            }
            if !capabilities.contains(capability.as_str()) {
                errors.push(BytecodeError::InvalidProviderContract {
                    name: contract.name.clone(),
                    reason: format!("unknown allowlist capability `{capability}`"),
                });
            }
        }
    }
}

fn validate_message_contracts(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    if (!program.types.is_empty() || !program.enums.is_empty())
        && !matches!(
            program.bytecode_version.as_str(),
            "0.18"
                | "0.19"
                | "0.20"
                | "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::MessageContractsRequireV018);
    }
    let type_names = program
        .types
        .iter()
        .map(|contract| contract.name.as_str())
        .chain(program.enums.iter().map(String::as_str))
        .collect::<HashSet<_>>();
    let mut names = HashSet::new();
    for contract in &program.types {
        if !names.insert(contract.name.as_str()) {
            errors.push(BytecodeError::InvalidMessageContract {
                name: contract.name.clone(),
                reason: "duplicate type name".into(),
            });
        }
        let mut fields = HashSet::new();
        for field in &contract.fields {
            if !fields.insert(field.name.as_str()) {
                errors.push(BytecodeError::InvalidMessageContract {
                    name: contract.name.clone(),
                    reason: format!("duplicate field `{}`", field.name),
                });
            }
            if !matches!(
                field.field_type.as_str(),
                "string" | "bool" | "int" | "float"
            ) && !type_names.contains(field.field_type.as_str())
            {
                errors.push(BytecodeError::InvalidMessageContract {
                    name: contract.name.clone(),
                    reason: format!("unknown field type `{}`", field.field_type),
                });
            }
        }
    }
}

fn validate_passports(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    const RISK_LEVELS: [&str; 4] = ["low", "medium", "high", "critical"];
    const ASN_REGISTRIES: [&str; 6] = ["LACNIC", "ARIN", "RIPE", "APNIC", "AFRINIC", "UNKNOWN"];

    if !program.passports.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.19"
                | "0.20"
                | "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::PassportsRequireV019);
    }
    let agents: HashSet<&str> = program
        .agents
        .iter()
        .map(|agent| agent.name.as_str())
        .collect();
    let mut names = HashSet::new();
    let mut passport_agents = HashSet::new();
    for passport in &program.passports {
        if !names.insert(passport.name.as_str()) {
            errors.push(BytecodeError::InvalidPassport {
                name: passport.name.clone(),
                reason: "duplicate passport name".into(),
            });
        }
        for (label, value) in [
            ("agent", &passport.agent),
            ("agent_name", &passport.agent_name),
            ("global_id", &passport.global_id),
            ("identity", &passport.identity),
            ("provider", &passport.provider),
            ("version", &passport.version),
            ("country", &passport.country),
            ("jurisdiction", &passport.jurisdiction),
            ("intent", &passport.intent),
            ("risk_level", &passport.risk_level),
        ] {
            if value.trim().is_empty() {
                errors.push(BytecodeError::InvalidPassport {
                    name: passport.name.clone(),
                    reason: format!("missing required field `{label}`"),
                });
            }
        }
        if passport.data_residency.is_empty() {
            errors.push(BytecodeError::InvalidPassport {
                name: passport.name.clone(),
                reason: "data_residency must not be empty".into(),
            });
        }
        if !passport.agent.trim().is_empty() {
            if !agents.contains(passport.agent.as_str()) {
                errors.push(BytecodeError::InvalidPassport {
                    name: passport.name.clone(),
                    reason: format!("unknown agent `{}`", passport.agent),
                });
            } else if !passport_agents.insert(passport.agent.as_str()) {
                errors.push(BytecodeError::InvalidPassport {
                    name: passport.name.clone(),
                    reason: format!("agent `{}` already has a passport", passport.agent),
                });
            }
        }
        if !passport.risk_level.trim().is_empty()
            && !RISK_LEVELS.contains(&passport.risk_level.as_str())
        {
            errors.push(BytecodeError::InvalidPassport {
                name: passport.name.clone(),
                reason: format!("invalid risk_level `{}`", passport.risk_level),
            });
        }
        if let Some(asn) = &passport.asn {
            if !ASN_REGISTRIES.contains(&asn.registry.as_str()) {
                errors.push(BytecodeError::InvalidPassport {
                    name: passport.name.clone(),
                    reason: format!("invalid asn registry `{}`", asn.registry),
                });
            }
        }
    }
}

fn validate_provider_harnesses(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    if !program.provider_harnesses.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.20"
                | "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::HarnessesRequireV020);
    }
    let providers = program
        .providers
        .iter()
        .map(|provider| provider.name.as_str())
        .collect::<HashSet<_>>();
    let feature_names = program
        .features
        .iter()
        .map(|feature| feature.name.as_str())
        .collect::<HashSet<_>>();
    let secret_names = program
        .secrets
        .iter()
        .map(|secret| secret.name.as_str())
        .collect::<HashSet<_>>();
    let types = program
        .types
        .iter()
        .map(|contract| contract.name.as_str())
        .collect::<HashSet<_>>();
    let mut names = HashSet::new();
    for harness in &program.provider_harnesses {
        if !names.insert(harness.name.as_str()) {
            errors.push(BytecodeError::DuplicateProviderHarness(
                harness.name.clone(),
            ));
        }
        let invalid = |reason: String, errors: &mut Vec<BytecodeError>| {
            errors.push(BytecodeError::InvalidProviderHarness {
                name: harness.name.clone(),
                reason,
            });
        };
        if harness.name.trim().is_empty() {
            invalid("name must not be empty".into(), errors);
        }
        if harness.provider.trim().is_empty() {
            invalid("provider must not be empty".into(), errors);
        } else if !providers.contains(harness.provider.as_str()) {
            invalid(format!("unknown provider `{}`", harness.provider), errors);
        }
        if let Some(feature) = &harness.feature {
            if !feature_names.is_empty() && !feature_names.contains(feature.as_str()) {
                invalid(format!("unknown feature `{feature}`"), errors);
            }
        }
        if let Some(secret) = &harness.secret {
            if !secret_names.is_empty() && !secret_names.contains(secret.as_str()) {
                invalid(format!("unknown secret `{secret}`"), errors);
            }
        }
        if !matches!(harness.mode.as_str(), "dry_run" | "simulated") {
            invalid(format!("invalid mode `{}`", harness.mode), errors);
        }
        if harness.network != "denied" {
            invalid("network must be denied".into(), errors);
        }
        if harness.secrets != "denied" {
            invalid("secrets must be denied".into(), errors);
        }
        if !matches!(harness.filesystem.as_str(), "none" | "read_only") {
            invalid(
                format!("invalid filesystem `{}`", harness.filesystem),
                errors,
            );
        }
        if harness.max_steps == Some(0) {
            invalid("max_steps must be positive".into(), errors);
        }
        if harness.timeout_ms == Some(0) {
            invalid("timeout_ms must be positive".into(), errors);
        }
        for (label, contract) in [
            ("input_contract", harness.input_contract.as_ref()),
            ("output_contract", harness.output_contract.as_ref()),
        ] {
            if let Some(contract) = contract {
                if !types.contains(contract.as_str()) {
                    invalid(format!("unknown {label} `{contract}`"), errors);
                }
            }
        }
        if harness
            .attestations
            .iter()
            .any(|attestation| attestation.trim().is_empty())
        {
            invalid("attestation entries must not be empty".into(), errors);
        }
    }
}

fn validate_features(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    if !program.features.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::FeaturesRequireV021);
    }
    let providers = program
        .providers
        .iter()
        .map(|provider| (provider.name.as_str(), provider.kind.as_str()))
        .collect::<std::collections::HashMap<_, _>>();
    let mut names = HashSet::new();
    for feature in &program.features {
        if !names.insert(feature.name.as_str()) {
            errors.push(BytecodeError::DuplicateFeature(feature.name.clone()));
        }
        let invalid = |reason: String, errors: &mut Vec<BytecodeError>| {
            errors.push(BytecodeError::InvalidFeature {
                name: feature.name.clone(),
                reason,
            });
        };
        if feature.name.trim().is_empty() {
            invalid("name must not be empty".into(), errors);
        }
        if !matches!(
            feature.status.as_str(),
            "experimental" | "preview" | "stable" | "deprecated"
        ) {
            invalid(format!("invalid status `{}`", feature.status), errors);
        }
        if !matches!(feature.default.as_str(), "disabled" | "enabled") {
            invalid(format!("invalid default `{}`", feature.default), errors);
        }
        if matches!(feature.status.as_str(), "experimental" | "preview")
            && !feature.requires_approval
        {
            invalid(
                format!("status `{}` requires approval", feature.status),
                errors,
            );
        }
        if let Some(provider) = &feature.provider {
            if providers.get(provider.as_str()) == Some(&"external")
                && feature.default != "disabled"
            {
                invalid(
                    format!(
                        "feature linked to external provider `{provider}` must default to disabled"
                    ),
                    errors,
                );
            }
        }
    }
}

fn validate_secrets(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    if !program.secrets.is_empty()
        && !matches!(
            program.bytecode_version.as_str(),
            "0.21"
                | "0.22"
                | "0.23"
                | "0.24"
                | "0.25"
                | "0.26"
                | "0.27"
                | "0.28"
                | "0.29"
                | "0.30"
                | "0.31"
                | "0.32"
                | "0.33"
                | "0.34"
                | "0.35"
        )
    {
        errors.push(BytecodeError::SecretsRequireV021);
    }
    let feature_names = program
        .features
        .iter()
        .map(|feature| feature.name.as_str())
        .collect::<HashSet<_>>();
    let mut names = HashSet::new();
    for secret in &program.secrets {
        if !names.insert(secret.name.as_str()) {
            errors.push(BytecodeError::DuplicateSecret(secret.name.clone()));
        }
        let invalid = |reason: String, errors: &mut Vec<BytecodeError>| {
            errors.push(BytecodeError::InvalidSecret {
                name: secret.name.clone(),
                reason,
            });
        };
        if secret.name.trim().is_empty() {
            invalid("name must not be empty".into(), errors);
        }
        if secret.handle.trim().is_empty() {
            invalid("handle must not be empty".into(), errors);
        }
        if !matches!(
            secret.scope.as_str(),
            "provider" | "adapter" | "model" | "tool" | "runtime"
        ) {
            invalid(format!("invalid scope `{}`", secret.scope), errors);
        }
        if secret.access != "denied" {
            invalid("access must be denied".into(), errors);
        }
        if secret.source != "none" {
            invalid("source must be none".into(), errors);
        }
        if let Some(required_by) = &secret.required_by {
            if !feature_names.is_empty() && !feature_names.contains(required_by.as_str()) {
                invalid(
                    format!("unknown required_by feature `{required_by}`"),
                    errors,
                );
            }
        }
    }
}

fn validate_adapters(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let mut names = HashSet::new();
    let provider_names: HashSet<_> = program.providers.iter().map(|p| p.name.as_str()).collect();
    let feature_names: HashSet<_> = program.features.iter().map(|f| f.name.as_str()).collect();
    let secret_names: HashSet<_> = program.secrets.iter().map(|s| s.name.as_str()).collect();
    let harness_names: HashSet<_> = program
        .provider_harnesses
        .iter()
        .map(|h| h.name.as_str())
        .collect();

    for adapter in &program.adapters {
        if !names.insert(adapter.name.as_str()) {
            errors.push(BytecodeError::DuplicateAdapter(adapter.name.clone()));
            continue;
        }
        if adapter.provider.trim().is_empty() {
            errors.push(BytecodeError::InvalidAdapter {
                name: adapter.name.clone(),
                reason: "missing provider".into(),
            });
        } else if !provider_names.contains(adapter.provider.as_str()) {
            errors.push(BytecodeError::InvalidAdapter {
                name: adapter.name.clone(),
                reason: format!("unknown provider `{}`", adapter.provider),
            });
        }

        if let Some(f) = &adapter.feature {
            if !feature_names.contains(f.as_str()) {
                errors.push(BytecodeError::InvalidAdapter {
                    name: adapter.name.clone(),
                    reason: format!("unknown feature `{f}`"),
                });
            }
        }
        if let Some(s) = &adapter.secret {
            if !secret_names.contains(s.as_str()) {
                errors.push(BytecodeError::InvalidAdapter {
                    name: adapter.name.clone(),
                    reason: format!("unknown secret `{s}`"),
                });
            }
        }
        if let Some(h) = &adapter.harness {
            if !harness_names.contains(h.as_str()) {
                errors.push(BytecodeError::InvalidAdapter {
                    name: adapter.name.clone(),
                    reason: format!("unknown harness `{h}`"),
                });
            }
        }

        // required fields values
        if adapter.mode.trim().is_empty() || adapter.mode == "unknown" {
            errors.push(BytecodeError::InvalidAdapter {
                name: adapter.name.clone(),
                reason: "missing or invalid mode".into(),
            });
        }
        if adapter.execution != "disabled" {
            errors.push(BytecodeError::InvalidAdapter {
                name: adapter.name.clone(),
                reason: "execution must be disabled".into(),
            });
        }
        if adapter.network != "denied" {
            errors.push(BytecodeError::InvalidAdapter {
                name: adapter.name.clone(),
                reason: "network must be denied".into(),
            });
        }
        if adapter.secrets != "denied" {
            errors.push(BytecodeError::InvalidAdapter {
                name: adapter.name.clone(),
                reason: "secrets must be denied".into(),
            });
        }
        if adapter.filesystem != "none" && adapter.filesystem != "read_only" {
            errors.push(BytecodeError::InvalidAdapter {
                name: adapter.name.clone(),
                reason: "filesystem must be none or read_only".into(),
            });
        }

        for c in &adapter.conformance {
            if c.trim().is_empty() {
                errors.push(BytecodeError::InvalidAdapter {
                    name: adapter.name.clone(),
                    reason: "empty conformance item".into(),
                });
            }
        }

        // basic link coherence if harness present
        if let Some(hname) = &adapter.harness {
            if let Some(h) = program
                .provider_harnesses
                .iter()
                .find(|hh| hh.name == *hname)
            {
                if h.provider != adapter.provider {
                    errors.push(BytecodeError::InvalidAdapter {
                        name: adapter.name.clone(),
                        reason: "provider mismatch with harness".into(),
                    });
                }
            }
        }
    }
}

fn validate_adapter_profiles(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let mut names = HashSet::new();
    let adapter_names: HashSet<_> = program.adapters.iter().map(|a| a.name.as_str()).collect();
    let provider_names: HashSet<_> = program.providers.iter().map(|p| p.name.as_str()).collect();

    for p in &program.adapter_profiles {
        if !names.insert(p.name.as_str()) {
            errors.push(BytecodeError::DuplicateAdapterProfile(p.name.clone()));
            continue;
        }
        if p.adapter.trim().is_empty() {
            errors.push(BytecodeError::InvalidAdapterProfile {
                name: p.name.clone(),
                reason: "missing adapter".into(),
            });
        } else if !adapter_names.contains(p.adapter.as_str()) {
            errors.push(BytecodeError::InvalidAdapterProfile {
                name: p.name.clone(),
                reason: format!("unknown adapter `{}`", p.adapter),
            });
        }
        if p.provider.trim().is_empty() {
            errors.push(BytecodeError::InvalidAdapterProfile {
                name: p.name.clone(),
                reason: "missing provider".into(),
            });
        } else if !provider_names.contains(p.provider.as_str()) {
            errors.push(BytecodeError::InvalidAdapterProfile {
                name: p.name.clone(),
                reason: format!("unknown provider `{}`", p.provider),
            });
        }
        if p.vendor.trim().is_empty() {
            errors.push(BytecodeError::InvalidAdapterProfile {
                name: p.name.clone(),
                reason: "missing vendor".into(),
            });
        }
        if p.execution != "disabled" {
            errors.push(BytecodeError::InvalidAdapterProfile {
                name: p.name.clone(),
                reason: "execution must be disabled".into(),
            });
        }
        if p.network != "denied" {
            errors.push(BytecodeError::InvalidAdapterProfile {
                name: p.name.clone(),
                reason: "network must be denied".into(),
            });
        }
        if p.secrets != "denied" {
            errors.push(BytecodeError::InvalidAdapterProfile {
                name: p.name.clone(),
                reason: "secrets must be denied".into(),
            });
        }
        for c in &p.capabilities {
            if c.trim().is_empty() {
                errors.push(BytecodeError::InvalidAdapterProfile {
                    name: p.name.clone(),
                    reason: "empty capability".into(),
                });
            }
        }
        for c in &p.required_conformance {
            if c.trim().is_empty() {
                errors.push(BytecodeError::InvalidAdapterProfile {
                    name: p.name.clone(),
                    reason: "empty required_conformance item".into(),
                });
            }
        }
    }
}

fn validate_cryptos(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let mut names = HashSet::new();
    for c in &program.cryptos {
        if !names.insert(c.name.as_str()) {
            errors.push(BytecodeError::DuplicateCrypto(c.name.clone()));
            continue;
        }
        if c.kind.trim().is_empty() {
            errors.push(BytecodeError::InvalidCrypto {
                name: c.name.clone(),
                reason: "missing kind".into(),
            });
        }
        if c.status.trim().is_empty() {
            errors.push(BytecodeError::InvalidCrypto {
                name: c.name.clone(),
                reason: "missing status".into(),
            });
        }
        if c.strength.trim().is_empty() {
            errors.push(BytecodeError::InvalidCrypto {
                name: c.name.clone(),
                reason: "missing strength".into(),
            });
        }
        if c.purpose.is_empty() {
            errors.push(BytecodeError::InvalidCrypto {
                name: c.name.clone(),
                reason: "missing or empty purpose".into(),
            });
        } else {
            for p in &c.purpose {
                if p.trim().is_empty() {
                    errors.push(BytecodeError::InvalidCrypto {
                        name: c.name.clone(),
                        reason: "empty purpose item".into(),
                    });
                }
            }
        }
        if let Some(ob) = c.output_bits {
            if ob == 0 {
                errors.push(BytecodeError::InvalidCrypto {
                    name: c.name.clone(),
                    reason: "output_bits must be > 0".into(),
                });
            }
        }
        if let Some(mk) = c.min_key_bits {
            if mk == 0 {
                errors.push(BytecodeError::InvalidCrypto {
                    name: c.name.clone(),
                    reason: "min_key_bits must be > 0".into(),
                });
            }
        }
    }
}

fn validate_crypto_boundaries(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    const DISPOSITIONS: [&str; 3] = ["denied", "allowed", "required"];
    let mut names = HashSet::new();
    for b in &program.crypto_boundaries {
        if !names.insert(b.name.as_str()) {
            errors.push(BytecodeError::DuplicateCryptoBoundary(b.name.clone()));
            continue;
        }
        if b.name.trim().is_empty() {
            errors.push(BytecodeError::InvalidCryptoBoundary {
                name: b.name.clone(),
                reason: "name must not be empty".into(),
            });
        }
        // A trust boundary must not let key or secret material cross, and must
        // keep crypto execution disabled — the declarative-only guarantee.
        if !DISPOSITIONS.contains(&b.key_material.as_str()) {
            errors.push(BytecodeError::InvalidCryptoBoundary {
                name: b.name.clone(),
                reason: format!("invalid key_material `{}`", b.key_material),
            });
        }
        if !DISPOSITIONS.contains(&b.secret_material.as_str()) {
            errors.push(BytecodeError::InvalidCryptoBoundary {
                name: b.name.clone(),
                reason: format!("invalid secret_material `{}`", b.secret_material),
            });
        }
        if b.execution != "disabled" {
            errors.push(BytecodeError::InvalidCryptoBoundary {
                name: b.name.clone(),
                reason: "execution must be disabled".into(),
            });
        }
        if let Some(bits) = b.min_hash_bits {
            if bits == 0 {
                errors.push(BytecodeError::InvalidCryptoBoundary {
                    name: b.name.clone(),
                    reason: "min_hash_bits must be > 0".into(),
                });
            }
        }
        for list in [
            &b.allowed_hashes,
            &b.allowed_signatures,
            &b.allowed_kems,
            &b.allowed_aeads,
            &b.legacy_allowed,
            &b.denied,
            &b.purpose,
        ] {
            for item in list {
                if item.trim().is_empty() {
                    errors.push(BytecodeError::InvalidCryptoBoundary {
                        name: b.name.clone(),
                        reason: "empty list item".into(),
                    });
                }
            }
        }
    }
}

fn validate_atrust_handshakes(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let mut names = HashSet::new();
    for h in &program.atrust_handshakes {
        if !names.insert(h.name.as_str()) {
            errors.push(BytecodeError::DuplicateATrustHandshake(h.name.clone()));
            continue;
        }
        let mut invalid = |reason: &str| {
            errors.push(BytecodeError::InvalidATrustHandshake {
                name: h.name.clone(),
                reason: reason.to_owned(),
            });
        };
        if h.name.trim().is_empty() {
            invalid("name must not be empty");
        }
        if h.initiator.trim().is_empty() {
            invalid("initiator must not be empty");
        }
        if h.responder.trim().is_empty() {
            invalid("responder must not be empty");
        }
        if h.initiator_identity.trim().is_empty() {
            invalid("initiator_identity must not be empty");
        }
        if h.responder_identity.trim().is_empty() {
            invalid("responder_identity must not be empty");
        }
        if h.boundary.trim().is_empty() {
            invalid("boundary must not be empty");
        }
        if h.method.trim().is_empty() {
            invalid("method must not be empty");
        }
        if h.credential_contracts.iter().any(|c| c.trim().is_empty()) {
            invalid("credential_contracts must not contain empty items");
        }
        if h.purpose.iter().any(|p| p.trim().is_empty()) {
            invalid("purpose must not contain empty items");
        }
        if h.mode != "dry_run" {
            invalid("mode must be dry_run");
        }
        if !matches!(h.direction.as_str(), "one_way" | "mutual") {
            invalid("direction must be one_way or mutual");
        }
        if !matches!(h.challenge.as_str(), "disabled" | "declared_only") {
            invalid("challenge must be disabled or declared_only");
        }
        if !matches!(h.response.as_str(), "disabled" | "declared_only") {
            invalid("response must be disabled or declared_only");
        }
        if !matches!(h.transcript.as_str(), "metadata_only" | "evidence_only") {
            invalid("transcript must be metadata_only or evidence_only");
        }
        if !matches!(h.verification.as_str(), "disabled" | "declared_only") {
            invalid("verification must be disabled or declared_only");
        }
        if !matches!(h.resolution.as_str(), "disabled" | "embedded" | "local") {
            invalid("resolution must be disabled, embedded, or local");
        }
        if h.network != "denied" {
            invalid("network must be denied");
        }
        if h.key_material != "denied" {
            invalid("key_material must be denied");
        }
        if h.secret_material != "denied" {
            invalid("secret_material must be denied");
        }
        if h.execution != "disabled" {
            invalid("execution must be disabled");
        }
        if h.evidence != "required" {
            invalid("evidence must be required");
        }
        if h.security_claims != "none" {
            invalid("security_claims must be none");
        }
    }
}

fn validate_trust_ledgers(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let hash_cryptos: std::collections::HashSet<&str> = program
        .cryptos
        .iter()
        .filter(|c| c.kind == "hash" && c.status != "denied")
        .map(|c| c.name.as_str())
        .collect();

    let mut names = HashSet::new();
    for l in &program.trust_ledgers {
        if !names.insert(l.name.as_str()) {
            errors.push(BytecodeError::DuplicateTrustLedger(l.name.clone()));
            continue;
        }
        let mut invalid = |reason: &str| {
            errors.push(BytecodeError::InvalidTrustLedger {
                name: l.name.clone(),
                reason: reason.to_owned(),
            });
        };
        if l.name.trim().is_empty() {
            invalid("name must not be empty");
        }
        if !matches!(l.scope.as_str(), "local" | "package" | "bundle") {
            invalid("scope must be local, package, or bundle");
        }
        if !matches!(l.mode.as_str(), "dry_run" | "declared_only") {
            invalid("mode must be dry_run or declared_only");
        }
        if l.hash_algorithm.trim().is_empty() {
            invalid("hash_algorithm must not be empty");
        } else if !hash_cryptos.contains(l.hash_algorithm.as_str()) {
            invalid("hash_algorithm must reference a declared, non-denied crypto of kind hash");
        }
        if !matches!(l.chain_policy.as_str(), "append_only" | "declared_only") {
            invalid("chain_policy must be append_only or declared_only");
        }
        if l.entries.is_empty() {
            invalid("entries must not be empty");
        }
        if l.purpose.iter().any(|p| p.trim().is_empty()) {
            invalid("purpose must not contain empty items");
        }
        if l.network != "denied" {
            invalid("network must be denied");
        }
        if l.key_material != "denied" {
            invalid("key_material must be denied");
        }
        if l.secret_material != "denied" {
            invalid("secret_material must be denied");
        }
        if l.execution != "disabled" {
            invalid("execution must be disabled");
        }
        if l.evidence != "required" {
            invalid("evidence must be required");
        }
        if l.security_claims != "none" {
            invalid("security_claims must be none");
        }

        let prefix = format!("{}:", l.hash_algorithm);
        let mut entry_ids = HashSet::new();
        let mut previous_entry_hash: Option<&str> = None;
        for (index, e) in l.entries.iter().enumerate() {
            if !entry_ids.insert(e.id.as_str()) {
                invalid("entry ids must be unique");
            }
            if e.id.trim().is_empty() {
                invalid("entry id must not be empty");
            }
            if !matches!(
                e.kind.as_str(),
                "identity" | "credential" | "handshake" | "evidence" | "policy" | "custom"
            ) {
                invalid("entry kind is invalid");
            }
            if e.subject.trim().is_empty() {
                invalid("entry subject must not be empty");
            }
            if e.evidence_ref.trim().is_empty() {
                invalid("entry evidence_ref must not be empty");
            }
            if e.entry_hash.trim().is_empty() {
                invalid("entry entry_hash must not be empty");
            } else if !l.hash_algorithm.trim().is_empty() && !e.entry_hash.starts_with(&prefix) {
                invalid("entry_hash must use the declared hash_algorithm prefix");
            }
            if e.previous_hash.trim().is_empty() {
                invalid("entry previous_hash must not be empty");
            } else if index == 0 {
                if e.previous_hash != "GENESIS" {
                    invalid("first entry previous_hash must be GENESIS");
                }
            } else if let Some(prev) = previous_entry_hash {
                if e.previous_hash != prev {
                    invalid("entry previous_hash must match the prior entry_hash");
                }
            }
            previous_entry_hash = Some(e.entry_hash.as_str());
        }
        if let Some(last) = l.entries.last() {
            if l.chain_root != last.entry_hash {
                invalid("chain_root must match the final entry_hash");
            }
        } else if l.chain_root.trim().is_empty() {
            invalid("chain_root must not be empty");
        }
    }
}

fn validate_mcp_bridge_contracts(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let mut names = HashSet::new();
    for c in &program.mcp_bridge_contracts {
        if !names.insert(c.name.as_str()) {
            errors.push(BytecodeError::DuplicateMcpBridgeContract(c.name.clone()));
            continue;
        }
        let mut invalid = |reason: &str| {
            errors.push(BytecodeError::InvalidMcpBridgeContract {
                name: c.name.clone(),
                reason: reason.to_owned(),
            });
        };
        if c.name.trim().is_empty() {
            invalid("name must not be empty");
        }
        if c.agent.trim().is_empty() {
            invalid("agent must not be empty");
        }
        if c.passport.trim().is_empty() {
            invalid("passport must not be empty");
        }
        if c.identity.trim().is_empty() {
            invalid("identity must not be empty");
        }
        if c.boundary.trim().is_empty() {
            invalid("boundary must not be empty");
        }
        if !matches!(c.transport.as_str(), "declared_only" | "disabled") {
            invalid("transport must be declared_only or disabled");
        }
        if c.protocol != "mcp" {
            invalid("protocol must be mcp");
        }
        if !matches!(
            c.direction.as_str(),
            "inbound" | "outbound" | "bidirectional"
        ) {
            invalid("direction must be inbound, outbound, or bidirectional");
        }
        if c.tools.iter().any(|t| t.trim().is_empty()) {
            invalid("tools must not contain empty items");
        }
        if c.resources.iter().any(|t| t.trim().is_empty()) {
            invalid("resources must not contain empty items");
        }
        if c.prompts.iter().any(|t| t.trim().is_empty()) {
            invalid("prompts must not contain empty items");
        }
        if c.network != "denied" {
            invalid("network must be denied");
        }
        if c.external_execution != "disabled" {
            invalid("external_execution must be disabled");
        }
        if c.tool_execution != "disabled" {
            invalid("tool_execution must be disabled");
        }
        if c.secret_material != "denied" {
            invalid("secret_material must be denied");
        }
        if c.key_material != "denied" {
            invalid("key_material must be denied");
        }
        if !matches!(c.authentication.as_str(), "none" | "declared_only") {
            invalid("authentication must be none or declared_only");
        }
        if !matches!(c.authorization.as_str(), "policy_bound" | "declared_only") {
            invalid("authorization must be policy_bound or declared_only");
        }
        if c.evidence != "required" {
            invalid("evidence must be required");
        }
        if c.security_claims != "none" {
            invalid("security_claims must be none");
        }
        if c.purpose.is_empty() {
            invalid("purpose must not be empty");
        }
        if c.purpose.iter().any(|p| p.trim().is_empty()) {
            invalid("purpose must not contain empty items");
        }
    }
}

fn validate_a2a_bridge_contracts(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let mut names = HashSet::new();
    for c in &program.a2a_bridge_contracts {
        if !names.insert(c.name.as_str()) {
            errors.push(BytecodeError::DuplicateA2ABridgeContract(c.name.clone()));
            continue;
        }
        let mut invalid = |reason: &str| {
            errors.push(BytecodeError::InvalidA2ABridgeContract {
                name: c.name.clone(),
                reason: reason.to_owned(),
            });
        };
        if c.name.trim().is_empty() {
            invalid("name must not be empty");
        }
        for (field, value) in [
            ("initiator", &c.initiator),
            ("responder", &c.responder),
            ("initiator_passport", &c.initiator_passport),
            ("responder_passport", &c.responder_passport),
            ("initiator_identity", &c.initiator_identity),
            ("responder_identity", &c.responder_identity),
            ("handshake", &c.handshake),
            ("trust_ledger", &c.trust_ledger),
            ("boundary", &c.boundary),
        ] {
            if value.trim().is_empty() {
                invalid(&format!("{field} must not be empty"));
            }
        }
        if !c.initiator.trim().is_empty() && c.initiator == c.responder {
            invalid("initiator and responder must be distinct");
        }
        if c.protocol != "a2a" {
            invalid("protocol must be a2a");
        }
        if !matches!(c.transport.as_str(), "declared_only" | "disabled") {
            invalid("transport must be declared_only or disabled");
        }
        if !matches!(
            c.direction.as_str(),
            "inbound" | "outbound" | "bidirectional"
        ) {
            invalid("direction must be inbound, outbound, or bidirectional");
        }
        if c.message_contracts.is_empty() {
            invalid("message_contracts must not be empty");
        }
        if c.message_contracts.iter().any(|m| m.trim().is_empty()) {
            invalid("message_contracts must not contain empty items");
        }
        if c.capabilities.iter().any(|t| t.trim().is_empty()) {
            invalid("capabilities must not contain empty items");
        }
        if c.network != "denied" {
            invalid("network must be denied");
        }
        if c.external_execution != "disabled" {
            invalid("external_execution must be disabled");
        }
        if c.agent_execution != "disabled" {
            invalid("agent_execution must be disabled");
        }
        if c.secret_material != "denied" {
            invalid("secret_material must be denied");
        }
        if c.key_material != "denied" {
            invalid("key_material must be denied");
        }
        if !matches!(c.authentication.as_str(), "none" | "declared_only") {
            invalid("authentication must be none or declared_only");
        }
        if !matches!(c.authorization.as_str(), "policy_bound" | "declared_only") {
            invalid("authorization must be policy_bound or declared_only");
        }
        if c.evidence != "required" {
            invalid("evidence must be required");
        }
        if c.security_claims != "none" {
            invalid("security_claims must be none");
        }
        if c.purpose.is_empty() {
            invalid("purpose must not be empty");
        }
        if c.purpose.iter().any(|p| p.trim().is_empty()) {
            invalid("purpose must not contain empty items");
        }
    }
}

fn validate_atrust_evidence_maps(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let mut names = HashSet::new();
    for map in &program.atrust_evidence_maps {
        if !names.insert(map.name.as_str()) {
            errors.push(BytecodeError::DuplicateATrustEvidenceMap(map.name.clone()));
        }
        let required_scalar = [
            ("agent", map.agent.as_str()),
            ("passport", map.passport.as_str()),
            ("identity", map.identity.as_str()),
            ("credential_contract", map.credential_contract.as_str()),
            ("handshake", map.handshake.as_str()),
            ("trust_ledger", map.trust_ledger.as_str()),
        ];
        for (field, value) in required_scalar {
            if value.trim().is_empty() {
                errors.push(BytecodeError::InvalidATrustEvidenceMap {
                    name: map.name.clone(),
                    reason: format!("missing required field `{field}`"),
                });
            }
        }
        for (field, values) in [
            ("mcp_bridges", &map.mcp_bridges),
            ("a2a_bridges", &map.a2a_bridges),
            ("policies", &map.policies),
            ("purpose", &map.purpose),
        ] {
            if values.is_empty() {
                errors.push(BytecodeError::InvalidATrustEvidenceMap {
                    name: map.name.clone(),
                    reason: format!("missing required field `{field}`"),
                });
            }
            if values.iter().any(|value| value.trim().is_empty()) {
                errors.push(BytecodeError::InvalidATrustEvidenceMap {
                    name: map.name.clone(),
                    reason: format!("`{field}` must not contain empty strings"),
                });
            }
        }
        if !matches!(map.coverage.as_str(), "required" | "complete") {
            errors.push(BytecodeError::InvalidATrustEvidenceMap {
                name: map.name.clone(),
                reason: "coverage must be required or complete".into(),
            });
        }
        if !matches!(map.mapping_mode.as_str(), "declared_only" | "evidence_only") {
            errors.push(BytecodeError::InvalidATrustEvidenceMap {
                name: map.name.clone(),
                reason: "mapping_mode must be declared_only or evidence_only".into(),
            });
        }
        if !matches!(map.verification.as_str(), "declared_only" | "disabled") {
            errors.push(BytecodeError::InvalidATrustEvidenceMap {
                name: map.name.clone(),
                reason: "verification must be declared_only or disabled".into(),
            });
        }
        if map.resolution != "disabled" {
            errors.push(BytecodeError::InvalidATrustEvidenceMap {
                name: map.name.clone(),
                reason: "resolution must be disabled".into(),
            });
        }
        for (field, value) in [
            ("evidence_bundle", map.evidence_bundle.as_str()),
            ("security_report", map.security_report.as_str()),
            ("trace", map.trace.as_str()),
        ] {
            if value != "required" {
                errors.push(BytecodeError::InvalidATrustEvidenceMap {
                    name: map.name.clone(),
                    reason: format!("{field} must be required"),
                });
            }
        }
        if map.network != "denied" {
            errors.push(BytecodeError::InvalidATrustEvidenceMap {
                name: map.name.clone(),
                reason: "network must be denied".into(),
            });
        }
        if map.external_execution != "disabled" {
            errors.push(BytecodeError::InvalidATrustEvidenceMap {
                name: map.name.clone(),
                reason: "external_execution must be disabled".into(),
            });
        }
        if map.secret_material != "denied" {
            errors.push(BytecodeError::InvalidATrustEvidenceMap {
                name: map.name.clone(),
                reason: "secret_material must be denied".into(),
            });
        }
        if map.key_material != "denied" {
            errors.push(BytecodeError::InvalidATrustEvidenceMap {
                name: map.name.clone(),
                reason: "key_material must be denied".into(),
            });
        }
        if map.execution != "disabled" {
            errors.push(BytecodeError::InvalidATrustEvidenceMap {
                name: map.name.clone(),
                reason: "execution must be disabled".into(),
            });
        }
        if map.security_claims != "none" {
            errors.push(BytecodeError::InvalidATrustEvidenceMap {
                name: map.name.clone(),
                reason: "security_claims must be none".into(),
            });
        }
        if let Some(notes) = &map.notes {
            if notes.trim().is_empty() {
                errors.push(BytecodeError::InvalidATrustEvidenceMap {
                    name: map.name.clone(),
                    reason: "notes must not be empty".into(),
                });
            }
        }
    }
}

fn validate_governance_profiles(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let evidence_maps: HashSet<&str> = program
        .atrust_evidence_maps
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let ledgers: HashSet<&str> = program
        .trust_ledgers
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let policies: HashSet<&str> = program
        .policies
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let mut names = HashSet::new();
    for profile in &program.governance_profiles {
        if !names.insert(profile.name.as_str()) {
            errors.push(BytecodeError::DuplicateGovernanceProfile(
                profile.name.clone(),
            ));
        }
        let mut invalid = |reason: &str| {
            errors.push(BytecodeError::InvalidGovernanceProfile {
                name: profile.name.clone(),
                reason: reason.into(),
            });
        };
        for (field, value) in [
            ("name", profile.name.as_str()),
            ("owner", profile.owner.as_str()),
            ("jurisdiction", profile.jurisdiction.as_str()),
            ("framework", profile.framework.as_str()),
            ("evidence_map", profile.evidence_map.as_str()),
            ("trust_ledger", profile.trust_ledger.as_str()),
        ] {
            if value.trim().is_empty() {
                invalid(&format!("{field} must not be empty"));
            }
        }
        if !matches!(
            profile.scope.as_str(),
            "agent" | "system" | "package" | "organization"
        ) {
            invalid("invalid scope");
        }
        if !matches!(
            profile.level.as_str(),
            "baseline" | "audit" | "regulated" | "experimental"
        ) {
            invalid("invalid level");
        }
        if !matches!(
            profile.domain.as_str(),
            "ai_agent" | "security" | "compliance" | "privacy" | "safety" | "custom"
        ) {
            invalid("invalid domain");
        }
        if !evidence_maps.contains(profile.evidence_map.as_str()) {
            invalid("unknown evidence_map");
        }
        if !ledgers.contains(profile.trust_ledger.as_str()) {
            invalid("unknown trust_ledger");
        }
        if profile.policies.is_empty()
            || profile
                .policies
                .iter()
                .any(|value| value.is_empty() || !policies.contains(value.as_str()))
        {
            invalid("policies must be non-empty and reference declared policies");
        }
        if profile.controls.is_empty() {
            invalid("controls must not be empty");
        }
        let mut control_ids = HashSet::new();
        for control in &profile.controls {
            if !control_ids.insert(control.id.as_str()) {
                invalid("duplicate control id");
            }
            if control.id.trim().is_empty()
                || control.requirement.trim().is_empty()
                || control.evidence_ref.trim().is_empty()
            {
                invalid("control fields must not be empty");
            }
            if !matches!(
                control.category.as_str(),
                "identity"
                    | "credential"
                    | "handshake"
                    | "ledger"
                    | "bridge"
                    | "evidence"
                    | "runtime_boundary"
                    | "policy"
                    | "security"
                    | "privacy"
                    | "safety"
                    | "compliance"
                    | "custom"
            ) {
                invalid("invalid control category");
            }
            if !matches!(
                control.status.as_str(),
                "mapped" | "declared" | "pending_review" | "not_applicable"
            ) {
                invalid("invalid control status");
            }
        }
        if !matches!(
            profile.risk_level.as_str(),
            "low" | "moderate" | "high" | "critical" | "unknown"
        ) {
            invalid("invalid risk_level");
        }
        if !matches!(
            profile.review_status.as_str(),
            "draft" | "reviewed" | "approved_internal" | "deprecated"
        ) {
            invalid("invalid review_status");
        }
        if !matches!(
            profile.assurance.as_str(),
            "declared_only" | "evidence_mapped" | "manually_reviewed"
        ) {
            invalid("invalid assurance");
        }
        if profile.network != "denied"
            || profile.external_execution != "disabled"
            || profile.secret_material != "denied"
            || profile.key_material != "denied"
            || profile.execution != "disabled"
            || profile.security_claims != "none"
        {
            invalid("runtime boundaries must remain denied/disabled and security_claims none");
        }
        if profile.purpose.is_empty() || profile.purpose.iter().any(|value| value.is_empty()) {
            invalid("purpose must not be empty");
        }
        if profile.notes.as_ref().is_some_and(|value| value.is_empty()) {
            invalid("notes must not be empty");
        }
    }
}

fn validate_regulatory_mappings(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let profiles: std::collections::HashMap<&str, &BytecodeGovernanceProfile> = program
        .governance_profiles
        .iter()
        .map(|value| (value.name.as_str(), value))
        .collect();
    let evidence_maps: HashSet<&str> = program
        .atrust_evidence_maps
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let mut names = HashSet::new();
    for mapping in &program.regulatory_mappings {
        if !names.insert(mapping.name.as_str()) {
            errors.push(BytecodeError::DuplicateRegulatoryMapping(
                mapping.name.clone(),
            ));
        }
        let mut invalid = |reason: &str| {
            errors.push(BytecodeError::InvalidRegulatoryMapping {
                name: mapping.name.clone(),
                reason: reason.into(),
            });
        };
        let profile = profiles.get(mapping.governance_profile.as_str()).copied();
        if profile.is_none() {
            invalid("unknown governance_profile");
        }
        if !evidence_maps.contains(mapping.evidence_map.as_str()) {
            invalid("unknown evidence_map");
        }
        if profile.is_some_and(|profile| profile.evidence_map != mapping.evidence_map) {
            invalid("evidence_map does not match governance_profile");
        }
        if mapping.jurisdiction.trim().is_empty() || mapping.framework.trim().is_empty() {
            invalid("jurisdiction and framework must not be empty");
        }
        if mapping.obligations.is_empty() {
            invalid("obligations must not be empty");
        }
        let controls: HashSet<&str> = profile
            .map(|value| {
                value
                    .controls
                    .iter()
                    .map(|control| control.id.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let mut obligation_ids = HashSet::new();
        for obligation in &mapping.obligations {
            if !obligation_ids.insert(obligation.id.as_str()) {
                invalid("duplicate obligation id");
            }
            if obligation.id.trim().is_empty()
                || obligation.source.trim().is_empty()
                || obligation.requirement.trim().is_empty()
                || obligation.control.trim().is_empty()
                || obligation.evidence_ref.trim().is_empty()
            {
                invalid("obligation fields must not be empty");
            }
            if !controls.contains(obligation.control.as_str()) {
                invalid("obligation references unknown control");
            }
            if !matches!(
                obligation.status.as_str(),
                "mapped" | "pending_review" | "gap" | "not_applicable"
            ) {
                invalid("invalid obligation status");
            }
        }
        if !matches!(
            mapping.coverage.as_str(),
            "mapped" | "partial" | "pending_review"
        ) {
            invalid("invalid coverage");
        }
        if !matches!(
            mapping.assessment.as_str(),
            "declared_only" | "evidence_mapped" | "manual_review_required"
        ) {
            invalid("invalid assessment");
        }
        if mapping.legal_claims != "none" || mapping.certification != "none" {
            invalid("legal_claims and certification must be none");
        }
        if mapping.network != "denied"
            || mapping.external_execution != "disabled"
            || mapping.secret_material != "denied"
            || mapping.key_material != "denied"
            || mapping.execution != "disabled"
            || mapping.security_claims != "none"
        {
            invalid("runtime boundaries must remain denied/disabled and security_claims none");
        }
        if mapping.purpose.is_empty() || mapping.purpose.iter().any(|value| value.is_empty()) {
            invalid("purpose must not be empty");
        }
        if mapping.notes.as_ref().is_some_and(|value| value.is_empty()) {
            invalid("notes must not be empty");
        }
    }
}

fn validate_third_party_verifiers(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let mut names = HashSet::new();
    for verifier in &program.third_party_verifiers {
        if !names.insert(verifier.name.as_str()) {
            errors.push(BytecodeError::DuplicateThirdPartyVerifier(
                verifier.name.clone(),
            ));
        }
        let mut reasons = Vec::new();
        if !matches!(
            verifier.verifier_type.as_str(),
            "internal" | "community" | "academic" | "vendor" | "independent_lab" | "custom"
        ) {
            reasons.push("invalid verifier_type");
        }
        if !matches!(
            verifier.independence.as_str(),
            "declared" | "self_attested" | "independent_declared"
        ) {
            reasons.push("invalid independence");
        }
        if !matches!(
            verifier.identity_mode.as_str(),
            "declared_only" | "document_only"
        ) {
            reasons.push("invalid identity_mode");
        }
        if !matches!(
            verifier.verification_mode.as_str(),
            "reproducible_artifacts" | "document_review" | "conformance_replay"
        ) {
            reasons.push("invalid verification_mode");
        }
        if verifier.display_name.trim().is_empty()
            || verifier.organization.trim().is_empty()
            || verifier.jurisdiction.trim().is_empty()
        {
            reasons.push("identity metadata must not be empty");
        }
        if verifier.allowed_scopes.is_empty()
            || verifier.allowed_scopes.iter().any(|value| value.is_empty())
            || verifier.disallowed_claims.is_empty()
            || verifier
                .disallowed_claims
                .iter()
                .any(|value| value.is_empty())
            || verifier.purpose.is_empty()
            || verifier.purpose.iter().any(|value| value.is_empty())
        {
            reasons.push("scope, disallowed claims, and purpose must not be empty");
        }
        if verifier.network != "denied"
            || verifier.external_execution != "disabled"
            || verifier.secret_material != "denied"
            || verifier.key_material != "denied"
            || verifier.execution != "disabled"
            || verifier.legal_claims != "none"
            || verifier.certification != "none"
            || verifier.security_claims != "none"
        {
            reasons.push("runtime, legal, certification, and security claims must remain denied");
        }
        if verifier
            .notes
            .as_ref()
            .is_some_and(|value| value.is_empty())
        {
            reasons.push("notes must not be empty");
        }
        for reason in reasons {
            errors.push(BytecodeError::InvalidThirdPartyVerifier {
                name: verifier.name.clone(),
                reason: reason.into(),
            });
        }
    }
}

fn validate_public_conformance_reports(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let verifiers: HashSet<&str> = program
        .third_party_verifiers
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let profiles: std::collections::HashMap<&str, &BytecodeGovernanceProfile> = program
        .governance_profiles
        .iter()
        .map(|value| (value.name.as_str(), value))
        .collect();
    let mappings: std::collections::HashMap<&str, &BytecodeRegulatoryMapping> = program
        .regulatory_mappings
        .iter()
        .map(|value| (value.name.as_str(), value))
        .collect();
    let evidence_maps: HashSet<&str> = program
        .atrust_evidence_maps
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let ledgers: HashSet<&str> = program
        .trust_ledgers
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let mut names = HashSet::new();
    for report in &program.public_conformance_reports {
        if !names.insert(report.name.as_str()) {
            errors.push(BytecodeError::DuplicatePublicConformanceReport(
                report.name.clone(),
            ));
        }
        let mut reasons = Vec::new();
        let profile = profiles.get(report.governance_profile.as_str()).copied();
        let mapping = mappings.get(report.regulatory_mapping.as_str()).copied();
        if !verifiers.contains(report.verifier.as_str()) {
            reasons.push("unknown third_party_verifier");
        }
        if !evidence_maps.contains(report.evidence_map.as_str()) {
            reasons.push("unknown evidence_map");
        }
        if profile.is_none() {
            reasons.push("unknown governance_profile");
        }
        if mapping.is_none() {
            reasons.push("unknown regulatory_mapping");
        }
        if !ledgers.contains(report.trust_ledger.as_str()) {
            reasons.push("unknown trust_ledger");
        }
        if profile.is_some_and(|value| value.evidence_map != report.evidence_map) {
            reasons.push("governance/evidence_map mismatch");
        }
        if mapping.is_some_and(|value| value.governance_profile != report.governance_profile) {
            reasons.push("regulatory/governance mismatch");
        }
        if mapping.is_some_and(|value| value.evidence_map != report.evidence_map) {
            reasons.push("regulatory/evidence_map mismatch");
        }
        if report.suite.trim().is_empty()
            || report.suite_version != "0.34"
            || report.source_artifact.trim().is_empty()
            || report.bytecode_artifact.trim().is_empty()
        {
            reasons.push("suite and artifacts must be non-empty and suite_version 0.34");
        }
        if !matches!(
            report.result.as_str(),
            "passed" | "failed" | "pending_review"
        ) || !matches!(
            report.reproducibility.as_str(),
            "declared" | "replayable_locally" | "document_only"
        ) || !matches!(
            report.review_status.as_str(),
            "draft" | "reviewed" | "published" | "deprecated"
        ) {
            reasons.push("invalid result, reproducibility, or review_status");
        }
        if report.security_report != "required"
            || report.evidence_bundle != "required"
            || report.trace != "required"
        {
            reasons.push("security_report, evidence_bundle, and trace must be required");
        }
        if report.claims.is_empty() {
            reasons.push("claims must not be empty");
        }
        let mut claim_ids = HashSet::new();
        for claim in &report.claims {
            if !claim_ids.insert(claim.id.as_str()) {
                reasons.push("duplicate claim id");
            }
            if claim.id.is_empty() || claim.statement.is_empty() || claim.evidence_ref.is_empty() {
                reasons.push("claim fields must not be empty");
            }
            if !matches!(
                claim.category.as_str(),
                "conformance"
                    | "evidence"
                    | "security_report"
                    | "governance"
                    | "regulatory_mapping"
                    | "bytecode"
                    | "source"
                    | "policy"
                    | "runtime_boundary"
                    | "custom"
            ) || !matches!(
                claim.status.as_str(),
                "mapped" | "declared" | "pending_review" | "not_applicable"
            ) {
                reasons.push("invalid claim category or status");
            }
        }
        if report.network != "denied"
            || report.external_execution != "disabled"
            || report.secret_material != "denied"
            || report.key_material != "denied"
            || report.execution != "disabled"
            || report.legal_claims != "none"
            || report.certification != "none"
            || report.security_claims != "none"
        {
            reasons.push("runtime, legal, certification, and security claims must remain denied");
        }
        if report.purpose.is_empty() || report.purpose.iter().any(|value| value.is_empty()) {
            reasons.push("purpose must not be empty");
        }
        if report.notes.as_ref().is_some_and(|value| value.is_empty()) {
            reasons.push("notes must not be empty");
        }
        for reason in reasons {
            errors.push(BytecodeError::InvalidPublicConformanceReport {
                name: report.name.clone(),
                reason: reason.into(),
            });
        }
    }
}

fn validate_runtime_hardening_profiles(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let evidence_maps: HashSet<&str> = program
        .atrust_evidence_maps
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let governance: HashSet<&str> = program
        .governance_profiles
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let reports: HashSet<&str> = program
        .public_conformance_reports
        .iter()
        .map(|value| value.name.as_str())
        .collect();
    let mut names = HashSet::new();
    for profile in &program.runtime_hardening_profiles {
        if !names.insert(profile.name.as_str()) {
            errors.push(BytecodeError::DuplicateRuntimeHardeningProfile(
                profile.name.clone(),
            ));
        }
        let mut reasons = Vec::new();
        for (value, allowed, reason) in [
            (
                profile.scope.as_str(),
                &["agent", "system", "package", "organization"][..],
                "invalid scope",
            ),
            (
                profile.mode.as_str(),
                &["declared_only", "evidence_only"][..],
                "invalid mode",
            ),
            (
                profile.enforcement.as_str(),
                &["declared_only", "evidence_only"][..],
                "invalid enforcement",
            ),
            (
                profile.sandbox.as_str(),
                &["required", "declared"][..],
                "invalid sandbox",
            ),
            (
                profile.allowlist.as_str(),
                &["required", "declared"][..],
                "invalid allowlist",
            ),
            (
                profile.approval.as_str(),
                &["required", "declared"][..],
                "invalid approval",
            ),
            (
                profile.incident_response.as_str(),
                &["declared", "required"][..],
                "invalid incident_response",
            ),
            (
                profile.residual_risk.as_str(),
                &["low", "moderate", "high", "critical", "unknown"][..],
                "invalid residual_risk",
            ),
            (
                profile.review_status.as_str(),
                &["draft", "reviewed", "approved_internal", "deprecated"][..],
                "invalid review_status",
            ),
            (
                profile.assurance.as_str(),
                &["declared_only", "evidence_mapped", "manually_reviewed"][..],
                "invalid assurance",
            ),
        ] {
            if !allowed.contains(&value) {
                reasons.push(reason);
            }
        }
        if profile.provider_execution != "disabled"
            || profile.external_providers != "disabled"
            || profile.network != "denied"
            || profile.tool_execution != "disabled"
            || profile.agent_execution != "disabled"
            || profile.filesystem_access != "denied"
            || profile.env_access != "denied"
            || profile.secret_material != "denied"
            || profile.key_material != "denied"
            || !profile.deny_by_default
            || profile.audit_log != "required"
            || profile.evidence != "required"
            || profile.security_claims != "none"
        {
            reasons.push(
                "runtime boundaries must remain denied, disabled, required, and deny-by-default",
            );
        }
        if !evidence_maps.contains(profile.evidence_map.as_str())
            || !governance.contains(profile.governance_profile.as_str())
            || !reports.contains(profile.public_conformance_report.as_str())
        {
            reasons.push("unknown evidence, governance, or public conformance binding");
        }
        if profile.protected_assets.is_empty()
            || profile
                .protected_assets
                .iter()
                .any(|value| value.is_empty())
            || profile.runtime_boundaries.is_empty()
            || profile
                .runtime_boundaries
                .iter()
                .any(|value| value.is_empty())
            || profile.purpose.is_empty()
            || profile.purpose.iter().any(|value| value.is_empty())
        {
            reasons.push("protected assets, runtime boundaries, and purpose must not be empty");
        }
        if profile.notes.as_ref().is_some_and(|value| value.is_empty()) {
            reasons.push("notes must not be empty");
        }
        for reason in reasons {
            errors.push(BytecodeError::InvalidRuntimeHardeningProfile {
                name: profile.name.clone(),
                reason: reason.into(),
            });
        }
    }
}

fn validate_threat_models(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    let profiles: std::collections::HashMap<&str, &BytecodeRuntimeHardeningProfile> = program
        .runtime_hardening_profiles
        .iter()
        .map(|value| (value.name.as_str(), value))
        .collect();
    let mut names = HashSet::new();
    for model in &program.threat_models {
        if !names.insert(model.name.as_str()) {
            errors.push(BytecodeError::DuplicateThreatModel(model.name.clone()));
        }
        let mut reasons = Vec::new();
        let profile = profiles.get(model.hardening_profile.as_str()).copied();
        if profile.is_none() {
            reasons.push("unknown runtime_hardening_profile");
        }
        if profile.is_some_and(|profile| {
            profile.evidence_map != model.evidence_map
                || profile.governance_profile != model.governance_profile
                || profile.public_conformance_report != model.public_conformance_report
        }) {
            reasons.push("hardening references mismatch");
        }
        for (value, allowed, reason) in [
            (
                model.methodology.as_str(),
                &["declared", "structured", "internal_review"][..],
                "invalid methodology",
            ),
            (
                model.scope.as_str(),
                &["agent", "system", "package", "organization"][..],
                "invalid scope",
            ),
            (
                model.review_status.as_str(),
                &["draft", "reviewed", "approved_internal", "deprecated"][..],
                "invalid review_status",
            ),
            (
                model.residual_risk.as_str(),
                &["low", "moderate", "high", "critical", "unknown"][..],
                "invalid residual_risk",
            ),
            (
                model.risk_acceptance.as_str(),
                &["declared_only", "pending_review", "manually_reviewed"][..],
                "invalid risk_acceptance",
            ),
        ] {
            if !allowed.contains(&value) {
                reasons.push(reason);
            }
        }
        if model.network != "denied"
            || model.external_execution != "disabled"
            || model.tool_execution != "disabled"
            || model.agent_execution != "disabled"
            || model.secret_material != "denied"
            || model.key_material != "denied"
            || model.execution != "disabled"
            || model.security_claims != "none"
        {
            reasons.push("threat model runtime and security boundaries must remain denied");
        }
        validate_bytecode_threat_collections(model, &mut reasons);
        if model.purpose.is_empty() || model.purpose.iter().any(|value| value.is_empty()) {
            reasons.push("purpose must not be empty");
        }
        if model.notes.as_ref().is_some_and(|value| value.is_empty()) {
            reasons.push("notes must not be empty");
        }
        for reason in reasons {
            errors.push(BytecodeError::InvalidThreatModel {
                name: model.name.clone(),
                reason: reason.into(),
            });
        }
    }
}

fn validate_bytecode_threat_collections(
    model: &BytecodeThreatModel,
    reasons: &mut Vec<&'static str>,
) {
    if model.assets.is_empty() || model.threats.is_empty() || model.mitigations.is_empty() {
        reasons.push("assets, threats, and mitigations must not be empty");
    }
    let mut ids = HashSet::new();
    for asset in &model.assets {
        if !ids.insert(asset.id.as_str()) {
            reasons.push("duplicate asset id");
        }
        if asset.id.is_empty()
            || asset.description.is_empty()
            || asset.evidence_ref.is_empty()
            || !matches!(
                asset.category.as_str(),
                "secret"
                    | "key"
                    | "identity"
                    | "credential"
                    | "handshake"
                    | "ledger"
                    | "bridge"
                    | "evidence"
                    | "policy"
                    | "runtime"
                    | "provider"
                    | "user_data"
                    | "custom"
            )
            || !matches!(
                asset.sensitivity.as_str(),
                "low" | "moderate" | "high" | "critical" | "unknown"
            )
        {
            reasons.push("invalid asset");
        }
    }
    ids.clear();
    for threat in &model.threats {
        if !ids.insert(threat.id.as_str()) {
            reasons.push("duplicate threat id");
        }
        if threat.id.is_empty()
            || threat.target.is_empty()
            || threat.mitigation.is_empty()
            || !matches!(
                threat.category.as_str(),
                "prompt_injection"
                    | "secret_leakage"
                    | "tool_abuse"
                    | "network_exfiltration"
                    | "identity_spoofing"
                    | "credential_misuse"
                    | "handshake_replay"
                    | "bridge_misuse"
                    | "evidence_tampering"
                    | "policy_bypass"
                    | "provider_misuse"
                    | "runtime_escape"
                    | "supply_chain"
                    | "custom"
            )
            || !matches!(
                threat.impact.as_str(),
                "low" | "moderate" | "high" | "critical" | "unknown"
            )
            || !matches!(
                threat.status.as_str(),
                "declared"
                    | "mitigated_declared"
                    | "pending_review"
                    | "accepted_risk"
                    | "not_applicable"
            )
        {
            reasons.push("invalid threat");
        }
    }
    ids.clear();
    for mitigation in &model.mitigations {
        if !ids.insert(mitigation.id.as_str()) {
            reasons.push("duplicate mitigation id");
        }
        if mitigation.id.is_empty()
            || mitigation.control_ref.is_empty()
            || mitigation.evidence_ref.is_empty()
            || !matches!(
                mitigation.category.as_str(),
                "network_boundary"
                    | "secret_boundary"
                    | "key_boundary"
                    | "tool_boundary"
                    | "agent_boundary"
                    | "provider_boundary"
                    | "policy_enforcement"
                    | "audit_logging"
                    | "evidence_mapping"
                    | "sandboxing"
                    | "review_process"
                    | "custom"
            )
            || !matches!(
                mitigation.status.as_str(),
                "mapped" | "declared" | "pending_review" | "not_applicable"
            )
        {
            reasons.push("invalid mitigation");
        }
    }
}

fn validate_policies(program: &BytecodeProgram, errors: &mut Vec<BytecodeError>) {
    const RULES: &[&str] = &[
        "no_unhandled_messages",
        "all_tool_calls_traced",
        "all_model_calls_traced",
        "all_intrinsics_traced",
        "all_provider_calls_traced",
        "halt_requires_trace",
        "runtime_status completed",
        "provider_contracts_declared",
        "provider_allowlists_valid",
        "external_execution",
        "evidence_bundle_verified",
        "security_report_generated",
        "agent_passport_declared",
        "agent_passport_attested",
        "agent_data_residency_declared",
        "agent_identity_declared",
        "provider_harness_declared",
        "provider_harness_sandboxed",
        "provider_network_denied",
        "provider_secrets_denied",
        "provider_filesystem_restricted",
        "external_provider_harnessed",
        "feature_flags_declared",
        "features_default_disabled",
        "experimental_features_require_approval",
        "secret_boundaries_declared",
        "secret_access_denied",
        "secret_values_absent",
        "external_provider_feature_gated",
        "external_provider_secret_boundary_declared",
        "adapters_declared",
        "adapters_execution_disabled",
        "adapters_network_denied",
        "adapters_secrets_denied",
        "adapters_provider_harnessed",
        "adapters_feature_gated",
        "adapters_secret_boundaried",
        "adapters_conformance_declared",
        "adapters_evidence_required",
        "adapter_profiles_declared",
        "adapter_profiles_execution_disabled",
        "adapter_profiles_network_denied",
        "adapter_profiles_secrets_denied",
        "adapter_profiles_linked",
        "adapter_profiles_conformance_declared",
        "vendor_profiles_declared",
        "crypto_primitives_declared",
        "crypto_primitives_allowed",
        "crypto_denied_not_used",
        "crypto_post_quantum_candidates_declared",
        "crypto_key_material_absent",
        "crypto_secret_material_absent",
        "crypto_execution_absent",
        "crypto_boundaries_declared",
        "post_quantum_readiness_declared",
        "atrust_boundaries_declared",
        "atrust_did_methods_declared",
        "atrust_did_method_allowed",
        "atrust_identity_format_declared",
        "atrust_credential_mode_declared",
        "atrust_handshake_disabled",
        "atrust_resolution_disabled",
        "atrust_key_material_denied",
        "atrust_secret_material_denied",
        "atrust_execution_disabled",
        "atrust_post_quantum_readiness_declared",
        "atrust_security_claims_none",
        "atrust_identity_declared",
        "atrust_identity_subject_valid",
        "atrust_identity_did_method_valid",
        "atrust_identity_boundary_valid",
        "atrust_identity_status_active",
        "atrust_identity_validation_dry_run",
        "atrust_identity_resolution_disabled",
        "atrust_identity_key_material_denied",
        "atrust_identity_secret_material_denied",
        "atrust_identity_execution_disabled",
        "atrust_identity_evidence_required",
        "atrust_identity_security_claims_absent",
        "atrust_identity_passport_consistent",
        "atrust_credential_contract_declared",
        "atrust_credential_issuer_did_declared",
        "atrust_credential_holder_did_declared",
        "atrust_credential_type_declared",
        "atrust_credential_schema_declared",
        "atrust_credential_claims_declared",
        "atrust_credential_verification_declared_only",
        "atrust_credential_presentation_disabled",
        "atrust_credential_resolution_disabled",
        "atrust_credential_key_material_denied",
        "atrust_credential_secret_material_denied",
        "atrust_credential_execution_disabled",
        "atrust_credential_evidence_required",
        "atrust_credential_security_claims_absent",
        "atrust_handshake_declared",
        "atrust_handshake_initiator_responder_valid",
        "atrust_handshake_identities_valid",
        "atrust_handshake_credential_contracts_valid",
        "atrust_handshake_boundary_method_valid",
        "atrust_handshake_mode_dry_run",
        "atrust_handshake_direction_valid",
        "atrust_handshake_challenge_declared_only",
        "atrust_handshake_response_declared_only",
        "atrust_handshake_transcript_evidence_only",
        "atrust_handshake_verification_declared_only",
        "atrust_handshake_resolution_disabled",
        "atrust_handshake_network_denied",
        "atrust_handshake_key_material_denied",
        "atrust_handshake_secret_material_denied",
        "atrust_handshake_execution_disabled",
        "atrust_handshake_evidence_required",
        "atrust_handshake_security_claims_absent",
        "trust_ledgers_declared",
        "trust_ledger_hash_algorithm_declared",
        "trust_ledger_chain_valid",
        "trust_ledger_entries_bound",
        "trust_ledger_append_only",
        "trust_ledger_network_denied",
        "trust_ledger_key_material_denied",
        "trust_ledger_secret_material_denied",
        "trust_ledger_execution_disabled",
        "trust_ledger_evidence_required",
        "trust_ledger_security_claims_absent",
        "trust_ledger_blockchain_absent",
        "trust_ledger_signature_absent",
        "mcp_bridge_contracts_declared",
        "mcp_bridge_agents_bound",
        "mcp_bridge_passports_bound",
        "mcp_bridge_identities_bound",
        "mcp_bridge_boundaries_bound",
        "mcp_bridge_network_denied",
        "mcp_bridge_external_execution_disabled",
        "mcp_bridge_tool_execution_disabled",
        "mcp_bridge_secret_material_denied",
        "mcp_bridge_key_material_denied",
        "mcp_bridge_authentication_non_secret",
        "mcp_bridge_security_claims_absent",
        "a2a_bridge_contracts_declared",
        "a2a_bridge_agents_bound",
        "a2a_bridge_passports_bound",
        "a2a_bridge_identities_bound",
        "a2a_bridge_handshakes_bound",
        "a2a_bridge_trust_ledgers_bound",
        "a2a_bridge_message_contracts_bound",
        "a2a_bridge_network_denied",
        "a2a_bridge_external_execution_disabled",
        "a2a_bridge_agent_execution_disabled",
        "a2a_bridge_secret_material_denied",
        "a2a_bridge_key_material_denied",
        "a2a_bridge_authentication_non_secret",
        "a2a_bridge_security_claims_absent",
        "atrust_evidence_maps_declared",
        "atrust_evidence_map_agents_bound",
        "atrust_evidence_map_passports_bound",
        "atrust_evidence_map_identities_bound",
        "atrust_evidence_map_credentials_bound",
        "atrust_evidence_map_handshakes_bound",
        "atrust_evidence_map_ledgers_bound",
        "atrust_evidence_map_bridges_bound",
        "atrust_evidence_map_policies_bound",
        "atrust_evidence_map_coverage_required",
        "atrust_evidence_map_verification_non_verifying",
        "atrust_evidence_map_resolution_disabled",
        "atrust_evidence_map_network_denied",
        "atrust_evidence_map_external_execution_disabled",
        "atrust_evidence_map_secret_material_denied",
        "atrust_evidence_map_key_material_denied",
        "atrust_evidence_map_execution_disabled",
        "atrust_evidence_map_security_claims_absent",
        "governance_profiles_declared",
        "governance_profiles_evidence_bound",
        "governance_profiles_controls_mapped",
        "governance_profiles_runtime_disabled",
        "governance_profiles_security_claims_absent",
        "governance_profiles_no_legal_certification",
        "regulatory_mappings_declared",
        "regulatory_mappings_profiles_bound",
        "regulatory_mappings_obligations_mapped",
        "regulatory_mappings_controls_bound",
        "regulatory_mappings_legal_claims_absent",
        "regulatory_mappings_certification_absent",
        "regulatory_mappings_runtime_disabled",
        "regulatory_mappings_security_claims_absent",
        "third_party_verifiers_declared",
        "third_party_verifiers_identity_declared",
        "third_party_verifiers_scope_bounded",
        "third_party_verifiers_runtime_disabled",
        "third_party_verifiers_legal_claims_absent",
        "third_party_verifiers_certification_absent",
        "third_party_verifiers_security_claims_absent",
        "public_conformance_reports_declared",
        "public_conformance_reports_verifiers_bound",
        "public_conformance_reports_artifacts_declared",
        "public_conformance_reports_evidence_bound",
        "public_conformance_reports_governance_bound",
        "public_conformance_reports_regulatory_bound",
        "public_conformance_reports_replayable",
        "public_conformance_reports_runtime_disabled",
        "public_conformance_reports_legal_claims_absent",
        "public_conformance_reports_certification_absent",
        "public_conformance_reports_security_claims_absent",
        "runtime_hardening_profiles_declared",
        "runtime_hardening_evidence_bound",
        "runtime_hardening_deny_by_default",
        "runtime_hardening_sandbox_required",
        "runtime_hardening_network_denied",
        "runtime_hardening_external_providers_disabled",
        "runtime_hardening_tool_execution_disabled",
        "runtime_hardening_agent_execution_disabled",
        "runtime_hardening_filesystem_denied",
        "runtime_hardening_env_denied",
        "runtime_hardening_secret_material_denied",
        "runtime_hardening_key_material_denied",
        "runtime_hardening_audit_log_required",
        "runtime_hardening_security_claims_absent",
        "threat_models_declared",
        "threat_models_hardening_bound",
        "threat_models_assets_mapped",
        "threat_models_threats_mapped",
        "threat_models_mitigations_mapped",
        "threat_models_runtime_disabled",
        "threat_models_network_denied",
        "threat_models_secret_material_denied",
        "threat_models_key_material_denied",
        "threat_models_execution_disabled",
        "threat_models_security_claims_absent",
    ];
    let mut names = HashSet::new();
    for policy in &program.policies {
        if policy.name.trim().is_empty() {
            errors.push(BytecodeError::InvalidPolicy {
                name: policy.name.clone(),
                reason: "name must not be empty".into(),
            });
        } else if !names.insert(policy.name.as_str()) {
            errors.push(BytecodeError::DuplicatePolicy(policy.name.clone()));
        }
        let mut required = HashSet::new();
        let mut denied = HashSet::new();
        for rule in &policy.rules {
            if !matches!(rule.effect.as_str(), "require" | "deny") {
                errors.push(BytecodeError::InvalidPolicy {
                    name: policy.name.clone(),
                    reason: format!("unknown effect `{}`", rule.effect),
                });
                continue;
            }
            if !RULES.contains(&rule.rule.as_str()) {
                errors.push(BytecodeError::InvalidPolicy {
                    name: policy.name.clone(),
                    reason: format!("unknown rule `{}`", rule.rule),
                });
                continue;
            }
            let (current, opposite) = if rule.effect == "require" {
                (&mut required, &denied)
            } else {
                (&mut denied, &required)
            };
            if !current.insert(rule.rule.as_str()) {
                errors.push(BytecodeError::InvalidPolicy {
                    name: policy.name.clone(),
                    reason: format!("duplicate {} rule `{}`", rule.effect, rule.rule),
                });
            }
            if opposite.contains(rule.rule.as_str()) {
                errors.push(BytecodeError::InvalidPolicy {
                    name: policy.name.clone(),
                    reason: format!("contradictory rule `{}`", rule.rule),
                });
            }
        }
        if let Some(violation) = &policy.on_violation {
            if !matches!(violation.action.as_str(), "block" | "review" | "warn") {
                errors.push(BytecodeError::InvalidPolicy {
                    name: policy.name.clone(),
                    reason: format!("unknown action `{}`", violation.action),
                });
            }
        }
    }
}

fn instruction_contract(instruction: &Instruction) -> Option<BytecodeProviderContract> {
    match instruction {
        Instruction::DeclareProviderContract {
            name,
            kind,
            enabled,
            dry_run_only,
            requires_feature_flag,
            requires_explicit_approval,
            allowed_targets,
            allowed_capabilities,
        } => Some(BytecodeProviderContract {
            name: name.clone(),
            kind: kind.clone(),
            enabled: *enabled,
            dry_run_only: *dry_run_only,
            requires_feature_flag: *requires_feature_flag,
            requires_explicit_approval: *requires_explicit_approval,
            allowed_targets: allowed_targets.clone(),
            allowed_capabilities: allowed_capabilities.clone(),
        }),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::{
        verify_bytecode, BytecodeAgent, BytecodeError, BytecodeProgram, BytecodeProviderContract,
        Instruction,
    };

    fn valid_program() -> BytecodeProgram {
        BytecodeProgram {
            bytecode_version: "0.3".into(),
            language: "Argorix Lang".into(),
            module: "Test".into(),
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
    fn accepts_valid_bytecode() {
        verify_bytecode(&valid_program()).unwrap();
    }

    #[test]
    fn rejects_missing_end() {
        let mut program = valid_program();
        program.instructions.pop();
        let errors = verify_bytecode(&program).unwrap_err();
        assert!(errors.contains(&BytecodeError::MissingEnd));
    }

    #[test]
    fn rejects_unknown_instruction() {
        let mut program = valid_program();
        program.instructions.insert(1, Instruction::Unknown);
        let errors = verify_bytecode(&program).unwrap_err();
        assert!(errors.contains(&BytecodeError::UnknownInstruction));
    }

    #[test]
    fn validates_v011_provider_contracts_and_declarations() {
        let mut program = valid_program();
        program.bytecode_version = "0.11".into();
        let contract = BytecodeProviderContract {
            name: "OpenAI".into(),
            kind: "external".into(),
            enabled: false,
            dry_run_only: true,
            requires_feature_flag: true,
            requires_explicit_approval: true,
            allowed_targets: vec![],
            allowed_capabilities: vec![],
        };
        program.providers.push(contract.clone());
        program.instructions.insert(
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
        verify_bytecode(&program).unwrap();

        program.providers[0].enabled = true;
        let errors = verify_bytecode(&program).unwrap_err();
        assert!(errors.iter().any(|error| matches!(
            error,
            BytecodeError::InvalidProviderContract { name, .. } if name == "OpenAI"
        )));
        assert!(errors.contains(&BytecodeError::ProviderContractDeclarationMismatch));
    }

    #[test]
    fn accepts_v012_populated_provider_allowlists() {
        let program: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v012.argbc.json"
        ))
        .unwrap();
        verify_bytecode(&program).unwrap();
        assert_eq!(program.providers[0].allowed_targets, vec!["GuardModel"]);
        assert_eq!(
            program.providers[0].allowed_capabilities,
            vec!["model.invoke"]
        );
    }

    #[test]
    fn accepts_v013_populated_provider_allowlists_and_retains_v012_compatibility() {
        let v013: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v013.argbc.json"
        ))
        .unwrap();
        verify_bytecode(&v013).unwrap();
        assert_eq!(v013.bytecode_version, "0.13");

        let v012: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v012.argbc.json"
        ))
        .unwrap();
        verify_bytecode(&v012).unwrap();
        assert_eq!(v012.bytecode_version, "0.12");
    }

    #[test]
    fn accepts_v014_and_retains_v013_compatibility() {
        let mut v014: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v013.argbc.json"
        ))
        .unwrap();
        v014.bytecode_version = "0.14".into();
        verify_bytecode(&v014).unwrap();

        let v013: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v013.argbc.json"
        ))
        .unwrap();
        verify_bytecode(&v013).unwrap();
    }

    #[test]
    fn accepts_v015_and_retains_v014_compatibility() {
        let mut v015: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v014.argbc.json"
        ))
        .unwrap();
        v015.bytecode_version = "0.15".into();
        verify_bytecode(&v015).unwrap();

        let v014: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v014.argbc.json"
        ))
        .unwrap();
        verify_bytecode(&v014).unwrap();
    }

    #[test]
    fn rejects_provider_declarations_in_a_different_order_than_top_level_contracts() {
        let mut program: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v012.argbc.json"
        ))
        .unwrap();
        let mut second = program.providers[0].clone();
        second.name = "Anthropic".into();
        program.providers.push(second);
        let mut declaration = program.instructions[0].clone();
        if let Instruction::DeclareProviderContract { name, .. } = &mut declaration {
            *name = "Anthropic".into();
        }
        program.instructions.insert(0, declaration);
        let errors = verify_bytecode(&program).unwrap_err();
        assert!(errors.contains(&BytecodeError::ProviderContractDeclarationMismatch));
    }
    #[test]
    fn rejects_populated_allowlists_in_v011_even_when_declaration_matches() {
        let mut program: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v012.argbc.json"
        ))
        .unwrap();
        program.bytecode_version = "0.11".into();
        let errors = verify_bytecode(&program).unwrap_err();
        assert!(errors.iter().any(|error| matches!(
            error,
            BytecodeError::InvalidProviderContract { reason, .. }
                if reason.contains("0.11 provider allowlists must be empty")
        )));
    }
    #[test]
    fn accepts_v010_without_provider_contracts() {
        let mut program = valid_program();
        program.bytecode_version = "0.10".into();
        verify_bytecode(&program).unwrap();
    }

    #[test]
    fn accepts_v016_module_metadata() {
        let mut program = valid_program();
        program.bytecode_version = "0.16".into();
        program.modules = vec![
            super::BytecodeModule {
                name: "main".into(),
                path: "src/main.argx".into(),
            },
            super::BytecodeModule {
                name: "agents.worker".into(),
                path: "src/agents/worker.argx".into(),
            },
        ];
        program.imports = vec![super::BytecodeModuleImport {
            from: "main".into(),
            to: "agents.worker".into(),
        }];
        verify_bytecode(&program).unwrap();
    }

    #[test]
    fn rejects_module_metadata_below_v016() {
        let mut program = valid_program();
        program.modules = vec![super::BytecodeModule {
            name: "main".into(),
            path: "src/main.argx".into(),
        }];
        let errors = verify_bytecode(&program).unwrap_err();
        assert!(errors.contains(&BytecodeError::ModulesRequireV016));
    }

    #[test]
    fn rejects_unknown_module_import_edge() {
        let mut program = valid_program();
        program.bytecode_version = "0.16".into();
        program.modules = vec![super::BytecodeModule {
            name: "main".into(),
            path: "src/main.argx".into(),
        }];
        program.imports = vec![super::BytecodeModuleImport {
            from: "main".into(),
            to: "agents.missing".into(),
        }];
        let errors = verify_bytecode(&program).unwrap_err();
        assert!(errors
            .iter()
            .any(|error| matches!(error, BytecodeError::UnknownModuleImport(name) if name == "agents.missing")));
    }

    #[test]
    fn accepts_v017_policy_metadata_and_retains_v016_compatibility() {
        let mut v017 = valid_program();
        v017.bytecode_version = "0.17".into();
        v017.policies = vec![super::BytecodePolicy {
            name: "ProviderSafety".into(),
            rules: vec![super::BytecodePolicyRule {
                effect: "deny".into(),
                rule: "external_execution".into(),
            }],
            on_violation: Some(super::BytecodePolicyViolation {
                action: "block".into(),
                trace_required: true,
            }),
        }];
        verify_bytecode(&v017).unwrap();

        let mut v016 = valid_program();
        v016.bytecode_version = "0.16".into();
        verify_bytecode(&v016).unwrap();
    }

    #[test]
    fn rejects_policy_metadata_below_v017_and_invalid_rules() {
        let mut program = valid_program();
        program.bytecode_version = "0.16".into();
        program.policies = vec![super::BytecodePolicy {
            name: "Bad".into(),
            rules: vec![super::BytecodePolicyRule {
                effect: "require".into(),
                rule: "future_rule".into(),
            }],
            on_violation: None,
        }];
        let errors = verify_bytecode(&program).unwrap_err();
        assert!(errors.contains(&BytecodeError::PoliciesRequireV017));
        assert!(errors.iter().any(|error| matches!(
            error,
            BytecodeError::InvalidPolicy { reason, .. }
                if reason.contains("unknown rule `future_rule`")
        )));
    }

    #[test]
    fn validates_v018_message_contracts_and_accepts_v017() {
        let mut v018 = valid_program();
        v018.bytecode_version = "0.18".into();
        v018.enums = vec!["Risk".into()];
        v018.types = vec![super::BytecodeType {
            name: "Message".into(),
            fields: vec![
                super::BytecodeTypeField {
                    name: "content".into(),
                    field_type: "string".into(),
                },
                super::BytecodeTypeField {
                    name: "risk".into(),
                    field_type: "Risk".into(),
                },
            ],
        }];
        verify_bytecode(&v018).unwrap();

        let mut v017 = valid_program();
        v017.bytecode_version = "0.17".into();
        verify_bytecode(&v017).unwrap();
    }

    #[test]
    fn validates_v019_passports_and_accepts_v018() {
        let mut v019 = valid_program();
        v019.bytecode_version = "0.19".into();
        v019.passports = vec![super::BytecodePassport {
            name: "WorkerPassport".into(),
            agent: "Worker".into(),
            agent_name: "Worker".into(),
            global_id: "argx:agent:1".into(),
            identity: "did:argorix:worker".into(),
            provider: "Argorix".into(),
            version: "1.0.0".into(),
            ans_name: None,
            country: "CL".into(),
            jurisdiction: "CL".into(),
            data_residency: vec!["CL".into()],
            asn: None,
            model: None,
            risk_level: "high".into(),
            data_scope: vec![],
            intent: "x".into(),
            intended_use: vec![],
            prohibited_use: vec![],
            attestations: vec!["redteam".into()],
        }];
        verify_bytecode(&v019).unwrap();

        // Passports below 0.19 are rejected.
        let mut v018 = v019.clone();
        v018.bytecode_version = "0.18".into();
        assert!(verify_bytecode(&v018)
            .unwrap_err()
            .contains(&BytecodeError::PassportsRequireV019));

        // Unknown agent and invalid risk level are rejected.
        let mut invalid = v019.clone();
        invalid.passports[0].agent = "Ghost".into();
        invalid.passports[0].risk_level = "extreme".into();
        let errors = verify_bytecode(&invalid).unwrap_err();
        assert!(errors.iter().any(|error| matches!(
            error,
            BytecodeError::InvalidPassport { reason, .. } if reason.contains("unknown agent `Ghost`")
        )));
        assert!(errors.iter().any(|error| matches!(
            error,
            BytecodeError::InvalidPassport { reason, .. } if reason.contains("invalid risk_level")
        )));

        // A v0.18 program without passports still verifies.
        let mut plain_v018 = valid_program();
        plain_v018.bytecode_version = "0.18".into();
        verify_bytecode(&plain_v018).unwrap();
    }

    #[test]
    fn validates_v020_provider_harnesses_and_accepts_v019() {
        let mut v020 = valid_program();
        v020.bytecode_version = "0.20".into();
        v020.types = vec![super::BytecodeType {
            name: "Prompt".into(),
            fields: vec![],
        }];
        let contract = BytecodeProviderContract {
            name: "OpenAI".into(),
            kind: "external".into(),
            enabled: false,
            dry_run_only: true,
            requires_feature_flag: true,
            requires_explicit_approval: true,
            allowed_targets: vec![],
            allowed_capabilities: vec![],
        };
        v020.providers.push(contract.clone());
        v020.instructions.insert(
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
        v020.provider_harnesses = vec![super::BytecodeProviderHarness {
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
            input_contract: Some("Prompt".into()),
            output_contract: None,
            attestations: vec![],
        }];
        verify_bytecode(&v020).unwrap();

        let mut v019 = valid_program();
        v019.bytecode_version = "0.19".into();
        verify_bytecode(&v019).unwrap();

        let mut below = v020.clone();
        below.bytecode_version = "0.19".into();
        assert!(verify_bytecode(&below)
            .unwrap_err()
            .contains(&BytecodeError::HarnessesRequireV020));

        let mut invalid = v020;
        invalid.provider_harnesses[0].network = "allowed".into();
        invalid.provider_harnesses[0].max_steps = Some(0);
        invalid.provider_harnesses[0].attestations = vec!["".into()];
        let errors = verify_bytecode(&invalid).unwrap_err();
        assert!(errors.iter().any(|error| matches!(
            error,
            BytecodeError::InvalidProviderHarness { reason, .. }
                if reason.contains("network must be denied")
        )));
        assert!(errors.iter().any(|error| matches!(
            error,
            BytecodeError::InvalidProviderHarness { reason, .. }
                if reason.contains("max_steps must be positive")
        )));
        assert!(errors.iter().any(|error| matches!(
            error,
            BytecodeError::InvalidProviderHarness { reason, .. }
                if reason.contains("attestation entries")
        )));
    }
}
