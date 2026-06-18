use crate::{AgentMailbox, EventFields, EventType, MessageEnvelope, TraceLedger, VmError};
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

        Ok(Self {
            agents,
            mailboxes,
            agent_state,
            pending_messages: VecDeque::new(),
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
}
