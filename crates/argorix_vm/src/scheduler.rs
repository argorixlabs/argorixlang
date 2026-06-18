use crate::{
    EventFields, EventType, MessageEnvelope, RuntimeState, RuntimeStatus, TraceStep, VmError,
};
use argorix_bytecode::{verify_bytecode, BytecodeProgram, Instruction};
use serde_json::json;
use std::collections::{HashMap, HashSet};

const EXTERNAL_ENTITIES: [&str; 5] = ["User", "System", "Runtime", "Memory", "Tool"];

#[derive(Debug, Default)]
pub struct Scheduler;

impl Scheduler {
    pub const fn new() -> Self {
        Self
    }

    pub fn run(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
    ) -> Result<Vec<TraceStep>, VmError> {
        state.trace_ledger.record(
            EventType::VmStarted,
            "ok",
            "deterministic scheduler started",
            EventFields::default(),
        );
        state.status = RuntimeStatus::Running;

        if let Err(errors) = verify_bytecode(bytecode) {
            let error = VmError::from_verification(errors);
            state.fail(error.to_string());
            return Err(error);
        }

        let approvals: HashMap<&str, &str> = bytecode
            .agents
            .iter()
            .map(|agent| (agent.name.as_str(), agent.approval.as_str()))
            .collect();
        let capabilities: HashSet<&str> = bytecode
            .capabilities
            .iter()
            .map(|capability| capability.name.as_str())
            .collect();
        let mut steps = Vec::new();
        let mut reached_end = false;

        for instruction in &bytecode.instructions {
            let result = match instruction {
                Instruction::RequireCapability { agent, capability } => {
                    if capabilities.contains(capability.as_str()) {
                        Ok(())
                    } else {
                        Err(VmError::UnknownCapability {
                            agent: agent.clone(),
                            capability: capability.clone(),
                        })
                    }
                }
                Instruction::RequireApproval { agent, capability } => {
                    if approvals.get(agent.as_str()) == Some(&"granted") {
                        Ok(())
                    } else {
                        Err(VmError::ApprovalDenied {
                            agent: agent.clone(),
                            capability: capability.clone(),
                        })
                    }
                }
                Instruction::SendMessage {
                    from,
                    to,
                    act,
                    message_type,
                } => self.schedule_message(state, &mut steps, from, to, act, message_type),
                Instruction::Halt { reason } => Err(VmError::Halted(reason.clone())),
                Instruction::End => {
                    reached_end = true;
                    break;
                }
                _ => Ok(()),
            };

            if let Err(error) = result {
                state.fail(error.to_string());
                return Err(error);
            }
        }

        if !reached_end {
            let error = VmError::MissingEnd;
            state.fail(error.to_string());
            return Err(error);
        }

        state.status = RuntimeStatus::Completed;
        state.trace_ledger.record(
            EventType::VmCompleted,
            "completed",
            format!("{} message steps completed", state.completed_steps),
            EventFields::default(),
        );
        Ok(steps)
    }

    fn schedule_message(
        &self,
        state: &mut RuntimeState,
        steps: &mut Vec<TraceStep>,
        from: &str,
        to: &str,
        act: &str,
        message_type: &str,
    ) -> Result<(), VmError> {
        if act.trim().is_empty() {
            return Err(VmError::EmptyAct);
        }
        if message_type.trim().is_empty() {
            return Err(VmError::EmptyMessageType);
        }

        let step = state.completed_steps + 1;
        let envelope = MessageEnvelope {
            id: format!("msg_{step:03}"),
            from: from.to_owned(),
            to: to.to_owned(),
            act: act.to_owned(),
            message_type: message_type.to_owned(),
            payload: json!({}),
        };
        state.pending_messages.push_back(envelope.clone());
        state.trace_ledger.record(
            EventType::MessageScheduled,
            "scheduled",
            envelope.id.clone(),
            EventFields::message(from, to, act, message_type),
        );

        if EXTERNAL_ENTITIES.contains(&to) {
            state.pending_messages.pop_front();
            state.completed_steps += 1;
            state.trace_ledger.record(
                EventType::MessageProcessed,
                "processed",
                format!("{} handed to external entity {to}", envelope.id),
                EventFields::message(from, to, act, message_type),
            );
            steps.push(TraceStep {
                index: state.completed_steps,
                from: from.to_owned(),
                to: to.to_owned(),
                act: act.to_owned(),
                message_type: message_type.to_owned(),
                status: "ok".to_owned(),
            });
            return Ok(());
        }

        let mailbox = state
            .mailboxes
            .get_mut(to)
            .ok_or_else(|| VmError::MissingMailbox(to.to_owned()))?;
        mailbox.push(envelope.clone());
        state.trace_ledger.record(
            EventType::MessageDelivered,
            "delivered",
            format!("{} delivered to {to}.mailbox", envelope.id),
            EventFields::message(from, to, act, message_type),
        );

        let processed = mailbox
            .pop()
            .ok_or_else(|| VmError::MailboxEmpty(to.to_owned()))?;
        state.pending_messages.pop_front();
        state.completed_steps += 1;
        state.trace_ledger.record(
            EventType::MessageProcessed,
            "processed",
            format!("{} processed by {to}", processed.id),
            EventFields::message(
                &processed.from,
                &processed.to,
                &processed.act,
                &processed.message_type,
            ),
        );
        steps.push(TraceStep {
            index: state.completed_steps,
            from: processed.from,
            to: processed.to,
            act: processed.act,
            message_type: processed.message_type,
            status: "ok".to_owned(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Scheduler;
    use crate::{EventType, RuntimeState, RuntimeStatus};
    use argorix_bytecode::{BytecodeAgent, BytecodeProgram, Instruction};

    fn prompt_defense_bytecode() -> BytecodeProgram {
        BytecodeProgram {
            bytecode_version: "0.3".into(),
            language: "Argorix Lang".into(),
            module: "Argorix.Security".into(),
            agents: ["PromptScanner", "PolicyJudge", "RuntimeGate"]
                .into_iter()
                .map(|name| BytecodeAgent {
                    name: name.into(),
                    approval: "granted".into(),
                })
                .collect(),
            capabilities: vec![],
            instructions: vec![
                Instruction::DeclareProtocol {
                    name: "PromptDefense".into(),
                },
                Instruction::SendMessage {
                    from: "User".into(),
                    to: "PromptScanner".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
                Instruction::SendMessage {
                    from: "PromptScanner".into(),
                    to: "PolicyJudge".into(),
                    act: "propose".into(),
                    message_type: "Finding".into(),
                },
                Instruction::SendMessage {
                    from: "PolicyJudge".into(),
                    to: "RuntimeGate".into(),
                    act: "commit".into(),
                    message_type: "Decision".into(),
                },
                Instruction::End,
            ],
        }
    }

    #[test]
    fn creates_delivers_and_processes_three_messages() {
        let bytecode = prompt_defense_bytecode();
        let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
        let steps = Scheduler::new().run(&bytecode, &mut state).unwrap();

        assert_eq!(steps.len(), 3);
        assert_eq!(state.completed_steps, 3);
        assert!(state.pending_messages.is_empty());
        assert_eq!(state.status, RuntimeStatus::Completed);
        for agent in ["PromptScanner", "PolicyJudge", "RuntimeGate"] {
            let mailbox = &state.mailboxes[agent];
            assert_eq!(mailbox.delivered(), 1);
            assert_eq!(mailbox.processed(), 1);
            assert!(mailbox.is_empty());
        }
        assert_eq!(
            state
                .trace_ledger
                .events
                .iter()
                .filter(|event| event.event_type == EventType::MessageScheduled)
                .count(),
            3
        );
    }

    #[test]
    fn rejects_unknown_receiver_and_records_failure() {
        let mut bytecode = prompt_defense_bytecode();
        if let Instruction::SendMessage { to, .. } = &mut bytecode.instructions[1] {
            *to = "MissingAgent".into();
        }
        let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
        let result = Scheduler::new().run(&bytecode, &mut state);

        assert!(result.is_err());
        assert_eq!(state.status, RuntimeStatus::Failed);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }
}
