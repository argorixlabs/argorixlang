use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

const EXTERNAL_ENTITIES: [&str; 5] = ["User", "System", "Runtime", "Memory", "Tool"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BytecodeProgram {
    pub bytecode_version: String,
    pub language: String,
    pub module: String,
    pub agents: Vec<BytecodeAgent>,
    pub capabilities: Vec<BytecodeCapability>,
    pub instructions: Vec<Instruction>,
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
}

pub fn verify_bytecode(program: &BytecodeProgram) -> Result<(), Vec<BytecodeError>> {
    let mut errors = Vec::new();
    if program.bytecode_version.trim().is_empty() {
        errors.push(BytecodeError::MissingVersion);
    } else if !matches!(program.bytecode_version.as_str(), "0.3" | "0.5" | "0.6") {
        errors.push(BytecodeError::UnsupportedVersion(
            program.bytecode_version.clone(),
        ));
    }
    if program.agents.is_empty() {
        errors.push(BytecodeError::NoAgents);
    }

    let agents: HashSet<&str> = program
        .agents
        .iter()
        .map(|agent| agent.name.as_str())
        .collect();
    let mut has_protocol_or_message = false;
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

#[cfg(test)]
mod tests {
    use super::{verify_bytecode, BytecodeAgent, BytecodeError, BytecodeProgram, Instruction};

    fn valid_program() -> BytecodeProgram {
        BytecodeProgram {
            bytecode_version: "0.3".into(),
            language: "Argorix Lang".into(),
            module: "Test".into(),
            agents: vec![BytecodeAgent {
                name: "Worker".into(),
                approval: "denied".into(),
            }],
            capabilities: vec![],
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
}
