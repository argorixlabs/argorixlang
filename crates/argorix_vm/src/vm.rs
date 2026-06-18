use crate::{
    AgentStateSummary, AssertionResult, EventFields, EventType, ExecutionTrace, FailureActivation,
    InjectedMessage, IntrinsicExecution, MailboxSummary, PolicyReport, ReactiveExecutionTrace,
    ReactiveScheduler, RuntimeState, RuntimeStatus, Scheduler, VmError,
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
        let policy_report = self.evaluate_policy(bytecode, &mut state, &steps);
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
        let tool_calls = state
            .tool_calls
            .iter()
            .map(|call| crate::ToolCallSummary {
                agent: call.agent.clone(),
                tool: call.tool.clone(),
                capability: call.capability.clone(),
                status: call.status.clone(),
                mode: "dry-run".into(),
            })
            .collect();
        let model_calls = state
            .model_calls
            .iter()
            .map(|call| crate::ModelCallSummary {
                agent: call.agent.clone(),
                model: call.model.clone(),
                provider: call.provider.clone(),
                capability: call.capability.clone(),
                status: call.status.clone(),
                mode: "dry-run".into(),
            })
            .collect();
        Ok(ReactiveExecutionTrace {
            vm_version: "0.9".into(),
            status: if policy_report.status == "passed" {
                "completed".into()
            } else {
                "failed".into()
            },
            mode: "reactive-dry-run".into(),
            scheduler: "deterministic".into(),
            injected,
            steps,
            mailboxes,
            agent_state,
            intrinsics,
            tool_calls,
            model_calls,
            policy_report,
            events: state.trace_ledger.events,
            security_checks: "passed".into(),
        })
    }

    fn evaluate_policy(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
        steps: &[crate::ReactiveStep],
    ) -> PolicyReport {
        let mut results = Vec::new();
        for assertion in &bytecode.assertions {
            let (passed, reason) = match assertion.name.as_str() {
                "no_unhandled_messages" => {
                    let passed = state.pending_messages.is_empty()
                        && state.mailboxes.values().all(|mailbox| mailbox.is_empty());
                    (passed, "mailbox contains unprocessed messages")
                }
                "all_tool_calls_traced" => (
                    state
                        .trace_ledger
                        .events
                        .iter()
                        .filter(|event| event.event_type == EventType::ToolCallDryRunResult)
                        .count()
                        == state.tool_calls.len(),
                    "one or more tool calls lack a dry-run trace result",
                ),
                "all_model_calls_traced" => (
                    state
                        .trace_ledger
                        .events
                        .iter()
                        .filter(|event| event.event_type == EventType::ModelCallDryRunResult)
                        .count()
                        == state.model_calls.len(),
                    "one or more model calls lack a dry-run trace result",
                ),
                "all_intrinsics_traced" => (
                    state
                        .trace_ledger
                        .events
                        .iter()
                        .filter(|event| {
                            matches!(
                                event.event_type,
                                EventType::FacuStateCheckpoint | EventType::MarronCausalGuard
                            )
                        })
                        .count()
                        == steps
                            .iter()
                            .map(|step| step.intrinsics.len())
                            .sum::<usize>(),
                    "one or more intrinsic invocations lack trace events",
                ),
                "halt_requires_trace" => (
                    !state
                        .trace_ledger
                        .events
                        .iter()
                        .any(|event| event.event_type == EventType::VmHalted)
                        || state
                            .trace_ledger
                            .events
                            .iter()
                            .any(|event| event.event_type == EventType::ValueTraced),
                    "halt occurred without a preceding trace",
                ),
                "runtime_status" => (
                    assertion.argument.as_deref() == Some("completed")
                        && state.status == RuntimeStatus::Completed,
                    "runtime status is not completed",
                ),
                _ => (false, "unknown assertion"),
            };
            state.trace_ledger.record(
                if passed {
                    EventType::AssertionVerified
                } else {
                    EventType::AssertionFailed
                },
                if passed { "passed" } else { "failed" },
                format!("assertion {} evaluated", assertion.name),
                EventFields::default(),
            );
            results.push(AssertionResult {
                name: assertion.name.clone(),
                argument: assertion.argument.clone(),
                status: if passed { "passed" } else { "failed" }.into(),
                reason: (!passed).then(|| reason.to_owned()),
            });
        }
        let failed = results.iter().any(|result| result.status == "failed");
        let mut failures = Vec::new();
        if failed {
            state.status = RuntimeStatus::Failed;
            let selected = bytecode
                .failures
                .iter()
                .find(|failure| failure.name == "PolicyViolation")
                .cloned()
                .unwrap_or(argorix_bytecode::BytecodeFailure {
                    name: "InternalPolicyViolation".into(),
                    action: "block".into(),
                    trace: "required".into(),
                });
            state.trace_ledger.record(
                EventType::FailureModeActivated,
                "active",
                format!("failure mode {} activated", selected.name),
                EventFields::default(),
            );
            failures.push(FailureActivation {
                name: selected.name,
                action: selected.action,
                trace: selected.trace,
            });
        }
        state.trace_ledger.record(
            EventType::PolicyReportGenerated,
            if failed { "failed" } else { "passed" },
            "policy report generated",
            EventFields::default(),
        );
        PolicyReport {
            status: if failed { "failed" } else { "passed" }.into(),
            assertions: results,
            failures,
        }
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
        assert_eq!(json["vm_version"], "0.9");
        assert_eq!(json["agent_state"].as_array().unwrap().len(), 3);
        assert_eq!(json["intrinsics"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn v07_json_contains_tool_calls() {
        let bytecode: BytecodeProgram =
            serde_json::from_str(include_str!("../../../examples/tool_call_v07.argbc.json"))
                .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "0.9");
        assert_eq!(json["tool_calls"][0]["tool"], "WebSearch");
        assert_eq!(json["tool_calls"][0]["mode"], "dry-run");
    }

    #[test]
    fn v08_json_contains_model_calls() {
        let bytecode: BytecodeProgram =
            serde_json::from_str(include_str!("../../../examples/model_call_v08.argbc.json"))
                .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "0.9");
        assert_eq!(json["model_calls"][0]["model"], "GuardModel");
        assert_eq!(json["model_calls"][0]["provider"], "simulated");
    }

    #[test]
    fn v09_policy_report_verifies_global_assertions() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(trace.status, "completed");
        assert_eq!(trace.policy_report.status, "passed");
        assert_eq!(trace.policy_report.assertions.len(), 6);
        assert!(trace
            .policy_report
            .assertions
            .iter()
            .all(|assertion| assertion.status == "passed"));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyReportGenerated));
    }

    #[test]
    fn v09_policy_failure_activates_declared_failure_mode() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        bytecode
            .assertions
            .iter_mut()
            .find(|assertion| assertion.name == "runtime_status")
            .unwrap()
            .argument = Some("failed".into());

        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(trace.status, "failed");
        assert_eq!(trace.policy_report.status, "failed");
        assert_eq!(trace.policy_report.failures[0].name, "PolicyViolation");
        assert_eq!(trace.policy_report.failures[0].action, "block");
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::AssertionFailed));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::FailureModeActivated));
    }
}
