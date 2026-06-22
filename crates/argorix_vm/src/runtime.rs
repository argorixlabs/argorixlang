use crate::{
    AgentMailbox, EventFields, EventType, MessageEnvelope, ProviderCallSummary,
    ProviderContractSummary, ProviderSummary, TraceLedger, VmError,
};
use argorix_bytecode::BytecodeProgram;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateCheckpoint {
    pub index: usize,
    pub intrinsic: String,
    pub binding: String,
    pub message_id: String,
    pub message_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentState {
    pub agent: String,
    pub handled_count: usize,
    pub last_binding: Option<String>,
    pub last_message_id: Option<String>,
    pub last_message_type: Option<String>,
    pub last_checkpoint_source: Option<String>,
    pub checkpoints: Vec<StateCheckpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallEnvelope {
    pub id: String,
    pub agent: String,
    pub tool: String,
    pub capability: String,
    pub input_binding: String,
    pub input_message_id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCallEnvelope {
    pub id: String,
    pub agent: String,
    pub model: String,
    pub provider: String,
    pub capability: String,
    pub input_binding: String,
    pub input_message_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeStatus {
    Initialized,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct RuntimeState {
    pub agents: Vec<String>,
    pub mailboxes: HashMap<String, AgentMailbox>,
    pub agent_state: HashMap<String, AgentState>,
    pub pending_messages: VecDeque<MessageEnvelope>,
    pub tool_calls: Vec<ToolCallEnvelope>,
    pub model_calls: Vec<ModelCallEnvelope>,
    pub provider_calls: Vec<ProviderCallSummary>,
    pub executable_providers: Vec<ProviderSummary>,
    pub provider_contracts: Vec<ProviderContractSummary>,
    pub failure_modes: Vec<argorix_bytecode::BytecodeFailure>,
    pub completed_steps: usize,
    pub trace_ledger: TraceLedger,
    pub status: RuntimeStatus,
}

impl RuntimeState {
    pub fn from_bytecode(bytecode: &BytecodeProgram) -> Result<Self, VmError> {
        let agents = bytecode
            .agents
            .iter()
            .map(|agent| agent.name.clone())
            .collect::<Vec<_>>();
        let mailboxes = agents
            .iter()
            .map(|agent| (agent.clone(), AgentMailbox::new()))
            .collect::<HashMap<_, _>>();
        let agent_state = agents
            .iter()
            .map(|agent| {
                (
                    agent.clone(),
                    AgentState {
                        agent: agent.clone(),
                        handled_count: 0,
                        last_binding: None,
                        last_message_id: None,
                        last_message_type: None,
                        last_checkpoint_source: None,
                        checkpoints: Vec::new(),
                    },
                )
            })
            .collect();
        let mut trace_ledger = TraceLedger::default();
        for agent in &agents {
            trace_ledger.record(
                EventType::AgentDeclared,
                "ok",
                format!("mailbox initialized for {agent}"),
                EventFields::target(agent),
            );
        }
        for capability in &bytecode.capabilities {
            trace_ledger.record(
                EventType::CapabilityDeclared,
                "ok",
                format!(
                    "capability {} declared at level {}",
                    capability.name, capability.level
                ),
                EventFields::default(),
            );
        }
        for assertion in &bytecode.assertions {
            trace_ledger.record(
                EventType::AssertionDeclared,
                "ok",
                format!("assertion {} declared", assertion.name),
                EventFields::default(),
            );
        }
        for policy in &bytecode.policies {
            trace_ledger.record(
                EventType::PolicyDeclared,
                "ok",
                format!("policy {} declared", policy.name),
                EventFields::default(),
            );
        }
        for harness in &bytecode.provider_harnesses {
            trace_ledger.record(
                EventType::ProviderHarnessDeclared,
                "declared",
                format!(
                    "provider harness {} declared for {}",
                    harness.name, harness.provider
                ),
                EventFields::default(),
            );
        }
        for failure in &bytecode.failures {
            trace_ledger.record(
                EventType::FailureDeclared,
                "ok",
                format!("failure {} declared", failure.name),
                EventFields::default(),
            );
        }
        for tool in &bytecode.tools {
            trace_ledger.record(
                EventType::ToolDeclared,
                "ok",
                format!("tool {} declared", tool.name),
                EventFields::default(),
            );
        }
        for instruction in &bytecode.instructions {
            if let argorix_bytecode::Instruction::AuthorizeTool { agent, tool } = instruction {
                trace_ledger.record(
                    EventType::ToolAuthorized,
                    "ok",
                    format!("tool {tool} authorized for {agent}"),
                    EventFields::target(agent),
                );
            }
        }
        for model in &bytecode.models {
            trace_ledger.record(
                EventType::ModelDeclared,
                "ok",
                format!("model {} declared", model.name),
                EventFields::default(),
            );
        }
        for instruction in &bytecode.instructions {
            if let argorix_bytecode::Instruction::AuthorizeModel { agent, model } = instruction {
                trace_ledger.record(
                    EventType::ModelAuthorized,
                    "ok",
                    format!("model {model} authorized for {agent}"),
                    EventFields::target(agent),
                );
            }
        }

        Ok(Self {
            agents,
            mailboxes,
            agent_state,
            pending_messages: VecDeque::new(),
            tool_calls: Vec::new(),
            model_calls: Vec::new(),
            provider_calls: Vec::new(),
            executable_providers: Vec::new(),
            provider_contracts: Vec::new(),
            failure_modes: bytecode.failures.clone(),
            completed_steps: 0,
            trace_ledger,
            status: RuntimeStatus::Initialized,
        })
    }

    pub fn fail(&mut self, details: impl Into<String>) {
        let details = details.into();
        self.status = RuntimeStatus::Failed;
        self.trace_ledger.record(
            EventType::VmFailed,
            "failed",
            details,
            EventFields::default(),
        );
    }

    pub fn activate_failure(&mut self, preferred: &str) {
        let name = self
            .failure_modes
            .iter()
            .find(|failure| failure.name == preferred)
            .map(|failure| failure.name.clone())
            .unwrap_or_else(|| "InternalFailure".into());
        self.trace_ledger.record(
            EventType::FailureModeActivated,
            "active",
            format!("failure mode {name} activated"),
            EventFields::default(),
        );
    }
}
