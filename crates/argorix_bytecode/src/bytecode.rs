use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

const EXTERNAL_ENTITIES: [&str; 5] = ["User", "System", "Runtime", "Memory", "Tool"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeProgram {
    pub bytecode_version: String,
    pub language: String,
    pub module: String,
    #[serde(default)]
    pub providers: Vec<BytecodeProviderContract>,
    #[serde(default)]
    pub assertions: Vec<BytecodeAssertion>,
    #[serde(default)]
    pub failures: Vec<BytecodeFailure>,
    pub agents: Vec<BytecodeAgent>,
    pub capabilities: Vec<BytecodeCapability>,
    #[serde(default)]
    pub tools: Vec<BytecodeTool>,
    #[serde(default)]
    pub models: Vec<BytecodeModel>,
    pub instructions: Vec<Instruction>,
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
pub struct BytecodeAssertion {
    pub name: String,
    pub argument: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeFailure {
    pub name: String,
    pub action: String,
    pub trace: String,
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
    #[error("provider contracts require bytecode version 0.11, 0.12, or 0.13")]
    ContractsRequireV011,
    #[error("duplicate provider contract `{0}`")]
    DuplicateProviderContract(String),
    #[error("invalid provider contract `{name}`: {reason}")]
    InvalidProviderContract { name: String, reason: String },
    #[error("provider contract declarations do not match the top-level providers collection")]
    ProviderContractDeclarationMismatch,
}

pub fn verify_bytecode(program: &BytecodeProgram) -> Result<(), Vec<BytecodeError>> {
    let mut errors = Vec::new();
    if program.bytecode_version.trim().is_empty() {
        errors.push(BytecodeError::MissingVersion);
    } else if !matches!(
        program.bytecode_version.as_str(),
        "0.3" | "0.5" | "0.6" | "0.7" | "0.8" | "0.9" | "0.10" | "0.11" | "0.12" | "0.13"
    ) {
        errors.push(BytecodeError::UnsupportedVersion(
            program.bytecode_version.clone(),
        ));
    }
    if program.agents.is_empty() {
        errors.push(BytecodeError::NoAgents);
    }
    if !matches!(program.bytecode_version.as_str(), "0.11" | "0.12" | "0.13")
        && !program.providers.is_empty()
    {
        errors.push(BytecodeError::ContractsRequireV011);
    }
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
        if !matches!(program.bytecode_version.as_str(), "0.12" | "0.13") {
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
            providers: vec![],
            assertions: vec![],
            failures: vec![],
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
}
