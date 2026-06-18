use crate::{
    AgentStateSummary, ExecutionTrace, InjectedMessage, IntrinsicExecution, MailboxSummary,
    ReactiveExecutionTrace, ReactiveScheduler, RuntimeState, Scheduler, VmError,
};
use argorix_bytecode::BytecodeProgram;

#[derive(Debug, Default)]
pub struct Vm;

impl Vm {
    pub const fn new() -> Self {
        Self
    }

    pub fn run_dry(&self, bytecode: &BytecodeProgram) -> Result<ExecutionTrace, VmError> {
        let mut state = RuntimeState::from_bytecode(bytecode)?;
        let steps = Scheduler::new().run(bytecode, &mut state)?;
        let mailboxes = state
            .agents
            .iter()
            .filter_map(|agent| {
                state.mailboxes.get(agent).map(|mailbox| MailboxSummary {
                    agent: agent.clone(),
                    delivered: mailbox.delivered(),
                    processed: mailbox.processed(),
                })
            })
            .collect::<Vec<_>>();

        Ok(ExecutionTrace {
            vm_version: "0.5".to_owned(),
            status: "completed".to_owned(),
            mode: "dry-run".to_owned(),
            scheduler: "deterministic".to_owned(),
            steps,
            mailboxes,
            events: state.trace_ledger.events,
            security_checks: "passed".to_owned(),
        })
    }

    pub fn initialize(&self, bytecode: &BytecodeProgram) -> Result<RuntimeState, VmError> {
        RuntimeState::from_bytecode(bytecode)
    }

    pub fn run_reactive(
        &self,
        bytecode: &BytecodeProgram,
        injected: InjectedMessage,
    ) -> Result<ReactiveExecutionTrace, VmError> {
        let mut state = RuntimeState::from_bytecode(bytecode)?;
        let steps = ReactiveScheduler::new().run(bytecode, &mut state, &injected)?;
        let mailboxes = state
            .agents
            .iter()
            .filter_map(|agent| {
                state.mailboxes.get(agent).map(|mailbox| MailboxSummary {
                    agent: agent.clone(),
                    delivered: mailbox.delivered(),
                    processed: mailbox.processed(),
                })
            })
            .collect();
        let agent_state = state
            .agents
            .iter()
            .filter_map(|agent| state.agent_state.get(agent))
            .map(|agent_state| AgentStateSummary {
                agent: agent_state.agent.clone(),
                handled_count: agent_state.handled_count,
                checkpoints: agent_state.checkpoints.len(),
                last_message_type: agent_state.last_message_type.clone(),
            })
            .collect();
        let intrinsics = steps
            .iter()
            .flat_map(|step| {
                step.intrinsics.iter().map(|intrinsic| IntrinsicExecution {
                    agent: step.agent.clone(),
                    name: intrinsic.name.clone(),
                    argument: intrinsic.argument.clone(),
                    status: "ok".into(),
                })
            })
            .collect();
        Ok(ReactiveExecutionTrace {
            vm_version: "0.6".into(),
            status: "completed".into(),
            mode: "reactive-dry-run".into(),
            scheduler: "deterministic".into(),
            injected,
            steps,
            mailboxes,
            agent_state,
            intrinsics,
            events: state.trace_ledger.events,
            security_checks: "passed".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Vm;
    use crate::{EventType, InjectedMessage, RuntimeStatus, Scheduler};
    use argorix_bytecode::{BytecodeAgent, BytecodeProgram, Instruction};

    fn valid_bytecode() -> BytecodeProgram {
        BytecodeProgram {
            bytecode_version: "0.3".into(),
            language: "Argorix Lang".into(),
            module: "Example".into(),
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
    fn initializes_runtime_state_from_bytecode() {
        let state = Vm::new().initialize(&valid_bytecode()).unwrap();
        assert_eq!(state.status, RuntimeStatus::Initialized);
        assert!(state.mailboxes.contains_key("Worker"));
    }

    #[test]
    fn dry_run_generates_protocol_trace_and_mailbox_summary() {
        let trace = Vm::new().run_dry(&valid_bytecode()).unwrap();
        assert_eq!(trace.status, "completed");
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.steps[0].message_type, "Ping");
        assert_eq!(trace.mailboxes[0].delivered, 1);
        assert_eq!(trace.mailboxes[0].processed, 1);
    }

    #[test]
    fn ledger_records_lifecycle_and_delivery_events() {
        let trace = Vm::new().run_dry(&valid_bytecode()).unwrap();
        for expected in [
            EventType::VmStarted,
            EventType::MessageDelivered,
            EventType::VmCompleted,
        ] {
            assert!(trace
                .events
                .iter()
                .any(|event| event.event_type == expected));
        }
    }

    #[test]
    fn failure_keeps_trace_ledger_in_runtime_state() {
        let mut invalid = valid_bytecode();
        invalid.instructions.pop();
        let mut state = Vm::new().initialize(&invalid).unwrap();
        let result = Scheduler::new().run(&invalid, &mut state);

        assert!(result.is_err());
        assert_eq!(state.status, RuntimeStatus::Failed);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }

    #[test]
    fn json_trace_contains_mailboxes() {
        let trace = Vm::new().run_dry(&valid_bytecode()).unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "0.5");
        assert_eq!(json["scheduler"], "deterministic");
        assert_eq!(json["mailboxes"][0]["agent"], "Worker");
    }

    #[test]
    fn reactive_json_trace_has_expected_mode() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/prompt_defense_v05.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "PromptScanner".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["mode"], "reactive-dry-run");
        assert_eq!(json["steps"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn v06_json_contains_agent_state_and_intrinsics() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/prompt_defense_v06.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "PromptScanner".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "0.6");
        assert_eq!(json["agent_state"].as_array().unwrap().len(), 3);
        assert_eq!(json["intrinsics"].as_array().unwrap().len(), 5);
    }
}
