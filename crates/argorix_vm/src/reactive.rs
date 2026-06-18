use crate::{
    EmittedMessage, EventFields, EventType, InjectedMessage, InvokedIntrinsic, MessageEnvelope,
    ReactiveStep, RuntimeState, RuntimeStatus, StateCheckpoint, VmError,
};
use argorix_bytecode::{verify_bytecode, BytecodeProgram, Instruction};
use serde_json::json;
use std::collections::{HashMap, HashSet};

const EXTERNAL_ENTITIES: [&str; 5] = ["User", "System", "Runtime", "Memory", "Tool"];

#[derive(Debug, Clone)]
enum HandlerOp {
    Emit { message_type: String, to: String },
    Trace { binding: String },
    Halt,
    Intrinsic { name: String, argument: String },
}

#[derive(Debug, Clone)]
struct Handler {
    binding: String,
    operations: Vec<HandlerOp>,
}

#[derive(Debug, Default)]
pub struct ReactiveScheduler;

impl ReactiveScheduler {
    pub const fn new() -> Self {
        Self
    }

    pub fn run(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
        injected: &InjectedMessage,
    ) -> Result<Vec<ReactiveStep>, VmError> {
        state.trace_ledger.record(
            EventType::VmStarted,
            "ok",
            "reactive deterministic scheduler started",
            EventFields::default(),
        );
        state.status = RuntimeStatus::Running;
        if let Err(errors) = verify_bytecode(bytecode) {
            let error = VmError::from_verification(errors);
            state.fail(error.to_string());
            return Err(error);
        }
        if !state.mailboxes.contains_key(&injected.to) {
            let error = VmError::MissingMailbox(injected.to.clone());
            state.fail(error.to_string());
            return Err(error);
        }

        self.verify_security(bytecode, state)?;
        let handlers = self.collect_handlers(bytecode);
        state.pending_messages.push_back(MessageEnvelope {
            id: "msg_001".into(),
            from: injected.from.clone(),
            to: injected.to.clone(),
            act: injected.act.clone(),
            message_type: injected.message_type.clone(),
            payload: json!({}),
        });
        state.trace_ledger.record(
            EventType::MessageScheduled,
            "injected",
            "initial message injected",
            EventFields::message(
                &injected.from,
                &injected.to,
                &injected.act,
                &injected.message_type,
            ),
        );

        let mut steps = Vec::new();
        let mut next_id = 2usize;
        let mut halted = false;
        while let Some(envelope) = state.pending_messages.pop_front() {
            let step_index = state.completed_steps + 1;
            if EXTERNAL_ENTITIES.contains(&envelope.to.as_str()) {
                state.completed_steps = step_index;
                state.trace_ledger.record(
                    EventType::MessageProcessed,
                    "processed",
                    format!("{} handed to external entity {}", envelope.id, envelope.to),
                    EventFields::message(
                        &envelope.from,
                        &envelope.to,
                        &envelope.act,
                        &envelope.message_type,
                    ),
                );
                continue;
            }
            let mailbox = state
                .mailboxes
                .get_mut(&envelope.to)
                .ok_or_else(|| VmError::MissingMailbox(envelope.to.clone()))?;
            mailbox.push(envelope.clone());
            state.trace_ledger.record(
                EventType::MessageDelivered,
                "delivered",
                format!("{} delivered to {}.mailbox", envelope.id, envelope.to),
                EventFields::message(
                    &envelope.from,
                    &envelope.to,
                    &envelope.act,
                    &envelope.message_type,
                ),
            );
            let processed = mailbox
                .pop()
                .ok_or_else(|| VmError::MailboxEmpty(envelope.to.clone()))?;
            let key = (processed.to.clone(), processed.message_type.clone());
            let handler = match handlers.get(&key) {
                Some(handler) => handler,
                None => {
                    let error = VmError::MissingHandler {
                        agent: processed.to,
                        message_type: processed.message_type,
                    };
                    state.fail(error.to_string());
                    return Err(error);
                }
            };
            state.trace_ledger.record(
                EventType::HandlerExecuted,
                "handled",
                format!("{} handled {}", key.0, key.1),
                EventFields::target(&key.0),
            );
            if let Some(agent_state) = state.agent_state.get_mut(&key.0) {
                agent_state.handled_count += 1;
                agent_state.last_message_id = Some(processed.id.clone());
                agent_state.last_message_type = Some(processed.message_type.clone());
            }

            let mut emitted = Vec::new();
            let mut traced = Vec::new();
            let mut invoked_intrinsics = Vec::new();
            let mut step_halted = false;
            for operation in &handler.operations {
                match operation {
                    HandlerOp::Emit { message_type, to } => {
                        let outgoing = MessageEnvelope {
                            id: format!("msg_{next_id:03}"),
                            from: key.0.clone(),
                            to: to.clone(),
                            act: "emit".into(),
                            message_type: message_type.clone(),
                            payload: json!({}),
                        };
                        next_id += 1;
                        state.pending_messages.push_back(outgoing);
                        emitted.push(EmittedMessage {
                            message_type: message_type.clone(),
                            to: to.clone(),
                        });
                        state.trace_ledger.record(
                            EventType::MessageEmitted,
                            "emitted",
                            format!("{} emitted {} to {to}", key.0, message_type),
                            EventFields::message(&key.0, to, "emit", message_type),
                        );
                    }
                    HandlerOp::Trace { binding } => {
                        traced.push(binding.clone());
                        state.trace_ledger.record(
                            EventType::ValueTraced,
                            "traced",
                            format!("{} traced {binding}", key.0),
                            EventFields::target(&key.0),
                        );
                    }
                    HandlerOp::Halt => {
                        halted = true;
                        step_halted = true;
                        state.trace_ledger.record(
                            EventType::VmHalted,
                            "halted",
                            format!("{} halted execution", key.0),
                            EventFields::target(&key.0),
                        );
                        break;
                    }
                    HandlerOp::Intrinsic { name, argument } => {
                        if let Err(error) = self.invoke_intrinsic(
                            state,
                            &key.0,
                            &handler.binding,
                            name,
                            argument,
                            &processed,
                        ) {
                            state.fail(error.to_string());
                            return Err(error);
                        }
                        invoked_intrinsics.push(InvokedIntrinsic {
                            name: name.clone(),
                            argument: argument.clone(),
                        });
                    }
                }
            }
            state.completed_steps = step_index;
            state.trace_ledger.record(
                EventType::MessageProcessed,
                "processed",
                format!("{} processed by {}", processed.id, key.0),
                EventFields::message(
                    &processed.from,
                    &processed.to,
                    &processed.act,
                    &processed.message_type,
                ),
            );
            steps.push(ReactiveStep {
                index: step_index,
                agent: key.0,
                handled: key.1,
                emitted,
                traced,
                halted: step_halted,
                intrinsics: invoked_intrinsics,
            });
            if halted {
                break;
            }
        }

        state.status = RuntimeStatus::Completed;
        state.trace_ledger.record(
            EventType::VmCompleted,
            "completed",
            if halted {
                "reactive execution completed by halt"
            } else {
                "reactive execution completed with no pending messages"
            },
            EventFields::default(),
        );
        Ok(steps)
    }

    fn collect_handlers(&self, bytecode: &BytecodeProgram) -> HashMap<(String, String), Handler> {
        let mut handlers = HashMap::new();
        let mut current: Option<(String, String, Handler)> = None;
        for instruction in &bytecode.instructions {
            match instruction {
                Instruction::DeclareHandler {
                    agent,
                    message_type,
                    binding,
                } => {
                    if let Some((agent, message, handler)) = current.take() {
                        handlers.insert((agent, message), handler);
                    }
                    current = Some((
                        agent.clone(),
                        message_type.clone(),
                        Handler {
                            binding: binding.clone(),
                            operations: Vec::new(),
                        },
                    ));
                }
                Instruction::EmitMessage {
                    message_type, to, ..
                } => {
                    if let Some((_, _, handler)) = &mut current {
                        handler.operations.push(HandlerOp::Emit {
                            message_type: message_type.clone(),
                            to: to.clone(),
                        });
                    }
                }
                Instruction::TraceValue { binding, .. } => {
                    if let Some((_, _, handler)) = &mut current {
                        debug_assert_eq!(&handler.binding, binding);
                        handler.operations.push(HandlerOp::Trace {
                            binding: binding.clone(),
                        });
                    }
                }
                Instruction::HandlerHalt { .. } => {
                    if let Some((_, _, handler)) = &mut current {
                        handler.operations.push(HandlerOp::Halt);
                    }
                }
                Instruction::InvokeIntrinsic { name, argument, .. } => {
                    if let Some((_, _, handler)) = &mut current {
                        handler.operations.push(HandlerOp::Intrinsic {
                            name: name.clone(),
                            argument: argument.clone(),
                        });
                    }
                }
                Instruction::EndHandler => {
                    if let Some((agent, message, handler)) = current.take() {
                        handlers.insert((agent, message), handler);
                    }
                }
                _ => {}
            }
        }
        if let Some((agent, message, handler)) = current {
            handlers.insert((agent, message), handler);
        }
        handlers
    }

    fn invoke_intrinsic(
        &self,
        state: &mut RuntimeState,
        agent: &str,
        binding: &str,
        name: &str,
        argument: &str,
        message: &MessageEnvelope,
    ) -> Result<(), VmError> {
        if argument != binding {
            return Err(VmError::IntrinsicBindingMismatch {
                intrinsic: name.to_owned(),
                argument: argument.to_owned(),
                binding: binding.to_owned(),
            });
        }
        match name {
            "facu" => {
                let agent_state = state
                    .agent_state
                    .get_mut(agent)
                    .ok_or_else(|| VmError::MissingMailbox(agent.to_owned()))?;
                agent_state.last_binding = Some(binding.to_owned());
                agent_state.last_message_id = Some(message.id.clone());
                agent_state.last_message_type = Some(message.message_type.clone());
                agent_state.last_checkpoint_source = Some("reactive-handler".into());
                agent_state.checkpoints.push(StateCheckpoint {
                    index: agent_state.checkpoints.len() + 1,
                    intrinsic: "facu".into(),
                    binding: binding.to_owned(),
                    message_id: message.id.clone(),
                    message_type: message.message_type.clone(),
                });
                state.trace_ledger.record(
                    EventType::FacuStateCheckpoint,
                    "ok",
                    format!("{agent} checkpointed {}", message.id),
                    EventFields::target(agent),
                );
                Ok(())
            }
            "marron" => {
                let complete = [
                    message.id.as_str(),
                    message.from.as_str(),
                    message.to.as_str(),
                    message.act.as_str(),
                    message.message_type.as_str(),
                ]
                .iter()
                .all(|value| !value.trim().is_empty());
                let delivered = state.trace_ledger.events.iter().any(|event| {
                    event.event_type == EventType::MessageDelivered
                        && event.to.as_deref() == Some(agent)
                        && event.message_type.as_deref() == Some(message.message_type.as_str())
                        && event.details.contains(&message.id)
                });
                if !complete
                    || !delivered
                    || message.to != agent
                    || message.message_type.trim().is_empty()
                {
                    return Err(VmError::CausalGuardFailed(format!(
                        "message `{}` is incomplete, orphaned, or outside the active handler",
                        message.id
                    )));
                }
                state.trace_ledger.record(
                    EventType::MarronCausalGuard,
                    "ok",
                    format!("{agent} verified causal message {}", message.id),
                    EventFields::message(
                        &message.from,
                        &message.to,
                        &message.act,
                        &message.message_type,
                    ),
                );
                Ok(())
            }
            other => Err(VmError::CausalGuardFailed(format!(
                "unknown runtime intrinsic `{other}`"
            ))),
        }
    }

    fn verify_security(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
    ) -> Result<(), VmError> {
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
        for instruction in &bytecode.instructions {
            let error = match instruction {
                Instruction::RequireCapability { agent, capability }
                    if !capabilities.contains(capability.as_str()) =>
                {
                    Some(VmError::UnknownCapability {
                        agent: agent.clone(),
                        capability: capability.clone(),
                    })
                }
                Instruction::RequireApproval { agent, capability }
                    if approvals.get(agent.as_str()) != Some(&"granted") =>
                {
                    Some(VmError::ApprovalDenied {
                        agent: agent.clone(),
                        capability: capability.clone(),
                    })
                }
                _ => None,
            };
            if let Some(error) = error {
                state.fail(error.to_string());
                return Err(error);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ReactiveScheduler;
    use crate::{EventType, InjectedMessage, RuntimeState};
    use argorix_bytecode::{BytecodeProgram, Instruction};

    fn fixture() -> BytecodeProgram {
        serde_json::from_str(include_str!(
            "../../../examples/prompt_defense_v05.argbc.json"
        ))
        .unwrap()
    }

    fn v06_fixture() -> BytecodeProgram {
        serde_json::from_str(include_str!(
            "../../../examples/prompt_defense_v06.argbc.json"
        ))
        .unwrap()
    }

    #[test]
    fn executes_three_handlers_emits_messages_and_halts() {
        let bytecode = fixture();
        let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
        let steps = ReactiveScheduler::new()
            .run(
                &bytecode,
                &mut state,
                &InjectedMessage {
                    from: "User".into(),
                    to: "PromptScanner".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].emitted[0].message_type, "Finding");
        assert_eq!(steps[1].emitted[0].message_type, "Decision");
        assert!(steps[2].halted);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmHalted));
    }

    #[test]
    fn intrinsics_create_checkpoints_and_causal_events() {
        let bytecode = v06_fixture();
        let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
        let steps = ReactiveScheduler::new()
            .run(
                &bytecode,
                &mut state,
                &InjectedMessage {
                    from: "User".into(),
                    to: "PromptScanner".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(state.agent_state["PromptScanner"].checkpoints.len(), 1);
        assert_eq!(state.agent_state["PolicyJudge"].handled_count, 1);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::FacuStateCheckpoint));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::MarronCausalGuard));
    }

    #[test]
    fn failed_marron_preserves_ledger() {
        let mut bytecode = v06_fixture();
        for instruction in &mut bytecode.instructions {
            if let Instruction::InvokeIntrinsic { name, argument, .. } = instruction {
                if name == "marron" {
                    *argument = "wrong".into();
                    break;
                }
            }
        }
        let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
        let result = ReactiveScheduler::new().run(
            &bytecode,
            &mut state,
            &InjectedMessage {
                from: "User".into(),
                to: "PromptScanner".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
        );
        assert!(result.is_err());
        assert_eq!(state.status, crate::RuntimeStatus::Failed);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }
}
