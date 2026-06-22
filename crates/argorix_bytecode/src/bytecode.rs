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
    ) && !program.providers.is_empty()
    {
        errors.push(BytecodeError::ContractsRequireV011);
    }
    if (!program.modules.is_empty() || !program.imports.is_empty())
        && !matches!(
            program.bytecode_version.as_str(),
            "0.16" | "0.17" | "0.18" | "0.19" | "0.20" | "0.21" | "0.22" | "0.23" | "0.24" | "0.25"
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
            "0.17" | "0.18" | "0.19" | "0.20" | "0.21" | "0.22" | "0.23" | "0.24" | "0.25"
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
            "0.22" | "0.23" | "0.24" | "0.25"
        )
    {
        errors.push(BytecodeError::AdaptersRequireV022);
    }
    if !program.adapter_profiles.is_empty()
        && !matches!(program.bytecode_version.as_str(), "0.23" | "0.24" | "0.25")
    {
        errors.push(BytecodeError::AdapterProfilesRequireV023);
    }
    if !program.cryptos.is_empty() && !matches!(program.bytecode_version.as_str(), "0.24" | "0.25")
    {
        errors.push(BytecodeError::CryptosRequireV024);
    }
    if !program.crypto_boundaries.is_empty() && program.bytecode_version != "0.25" {
        errors.push(BytecodeError::CryptoBoundariesRequireV025);
    }
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
            "0.18" | "0.19" | "0.20" | "0.21" | "0.22" | "0.23" | "0.24" | "0.25"
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
            "0.19" | "0.20" | "0.21" | "0.22" | "0.23" | "0.24" | "0.25"
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
            "0.20" | "0.21" | "0.22" | "0.23" | "0.24" | "0.25"
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
            "0.21" | "0.22" | "0.23" | "0.24" | "0.25"
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
            "0.21" | "0.22" | "0.23" | "0.24" | "0.25"
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
