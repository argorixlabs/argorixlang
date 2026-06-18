use crate::{
    AgentStateSummary, AssertionResult, EventFields, EventType, ExecutionTrace, FailureActivation,
    InjectedMessage, IntrinsicExecution, MailboxSummary, PolicyReport, ReactiveExecutionTrace,
    ReactiveScheduler, RuntimeState, RuntimeStatus, Scheduler, VmError,
};
use argorix_bytecode::{verify_bytecode, BytecodeProgram};
use argorix_provider::{AdapterContract, ProviderKind, ProviderRegistry};

pub struct Vm {
    providers: ProviderRegistry,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Self {
            providers: ProviderRegistry::default(),
        }
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

    pub fn load_provider_contracts(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
    ) -> Result<ProviderRegistry, VmError> {
        let mut registry = self.providers.execution_registry();
        for provider in &bytecode.providers {
            state.trace_ledger.record(
                EventType::ProviderContractDeclared,
                "declared",
                format!("provider contract {} declared", provider.name),
                EventFields::default(),
            );
            let kind = match provider.kind.as_str() {
                "simulated" => ProviderKind::Simulated,
                "external" => ProviderKind::External,
                other => {
                    let reason = format!("unsupported provider kind `{other}`");
                    self.reject_provider_contract(state, &provider.name, &reason);
                    return Err(VmError::ProviderBoundary {
                        provider: provider.name.clone(),
                        reason,
                    });
                }
            };
            let contract = AdapterContract {
                name: provider.name.clone(),
                kind,
                enabled: provider.enabled,
                dry_run_only: provider.dry_run_only,
                requires_feature_flag: provider.requires_feature_flag,
                requires_explicit_approval: provider.requires_explicit_approval,
                allowed_targets: provider.allowed_targets.clone(),
                allowed_capabilities: provider.allowed_capabilities.clone(),
            };
            if let Err(error) = registry.register_contract(contract) {
                let reason = error.to_string();
                self.reject_provider_contract(state, &provider.name, &reason);
                return Err(VmError::ProviderBoundary {
                    provider: provider.name.clone(),
                    reason,
                });
            }
            if let Err(error) = registry.validate_contract(&provider.name) {
                let reason = error.to_string();
                self.reject_provider_contract(state, &provider.name, &reason);
                return Err(VmError::ProviderBoundary {
                    provider: provider.name.clone(),
                    reason,
                });
            }
            state.trace_ledger.record(
                EventType::ProviderContractValidated,
                "validated",
                format!("provider contract {} validated", provider.name),
                EventFields::default(),
            );
        }
        Ok(registry)
    }

    fn reject_provider_contract(&self, state: &mut RuntimeState, name: &str, reason: &str) {
        state.activate_failure("ProviderContractRejected");
        state.trace_ledger.record(
            EventType::ProviderContractRejected,
            "rejected",
            format!("provider contract {name} rejected: {reason}"),
            EventFields::default(),
        );
        state.fail(format!("provider contract {name} rejected: {reason}"));
    }

    pub fn run_reactive(
        &self,
        bytecode: &BytecodeProgram,
        injected: InjectedMessage,
    ) -> Result<ReactiveExecutionTrace, VmError> {
        let mut state = RuntimeState::from_bytecode(bytecode)?;
        if let Err(errors) = verify_bytecode(bytecode) {
            let error = VmError::from_verification(errors);
            state.fail(error.to_string());
            return Err(error);
        }
        let execution_providers = self.load_provider_contracts(bytecode, &mut state)?;
        for (name, kind) in execution_providers.entries() {
            state.trace_ledger.record(
                EventType::ProviderRegistered,
                "registered",
                format!("provider {name} registered as {kind:?}"),
                EventFields::default(),
            );
        }
        let steps = ReactiveScheduler::new().run_with_registry(
            bytecode,
            &mut state,
            &injected,
            &execution_providers,
        )?;
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
        let providers = execution_providers
            .entries()
            .into_iter()
            .map(|(name, kind)| crate::ProviderSummary {
                name: name.to_owned(),
                kind: match kind {
                    argorix_provider::ProviderKind::Simulated => "simulated",
                    argorix_provider::ProviderKind::External => "external",
                }
                .into(),
                enabled: true,
            })
            .collect();
        let provider_contracts = execution_providers
            .contract_entries()
            .into_iter()
            .map(|contract| crate::ProviderContractSummary {
                name: contract.name.clone(),
                kind: match contract.kind {
                    ProviderKind::Simulated => "simulated",
                    ProviderKind::External => "external",
                }
                .into(),
                enabled: contract.enabled,
                dry_run_only: contract.dry_run_only,
                requires_feature_flag: contract.requires_feature_flag,
                requires_explicit_approval: contract.requires_explicit_approval,
                allowed_targets: contract.allowed_targets.clone(),
                allowed_capabilities: contract.allowed_capabilities.clone(),
            })
            .collect();
        let provider_calls = state.provider_calls.clone();
        Ok(ReactiveExecutionTrace {
            vm_version: "0.12".into(),
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
            providers,
            provider_contracts,
            provider_calls,
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
    use argorix_bytecode::{BytecodeAgent, BytecodeProgram, BytecodeProviderContract, Instruction};
    use serde_json::json;

    fn add_external_contract(bytecode: &mut BytecodeProgram, enabled: bool) {
        bytecode.bytecode_version = "0.11".into();
        let contract = BytecodeProviderContract {
            name: "OpenAI".into(),
            kind: "external".into(),
            enabled,
            dry_run_only: true,
            requires_feature_flag: true,
            requires_explicit_approval: true,
            allowed_targets: vec![],
            allowed_capabilities: vec![],
        };
        bytecode.providers.push(contract.clone());
        bytecode.instructions.insert(
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
    }

    fn valid_bytecode() -> BytecodeProgram {
        BytecodeProgram {
            bytecode_version: "0.3".into(),
            language: "Argorix Lang".into(),
            module: "Example".into(),
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
        assert_eq!(json["vm_version"], "0.12");
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
        assert_eq!(json["vm_version"], "0.12");
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
        assert_eq!(json["vm_version"], "0.12");
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
    fn provider_boundary_routes_tool_and_model_calls() {
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

        assert_eq!(trace.vm_version, "0.12");
        assert_eq!(trace.providers[0].name, "simulated");
        assert_eq!(trace.providers[0].kind, "simulated");
        assert_eq!(trace.provider_calls.len(), 2);
        assert_eq!(trace.provider_calls[0].kind, "tool");
        assert_eq!(trace.provider_calls[0].target, "WebSearch");
        assert_eq!(trace.provider_calls[1].kind, "model");
        assert_eq!(trace.provider_calls[1].target, "GuardModel");
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderSelected));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderRegistered));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderRequestCreated));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderResponseReceived));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderDryRunEnforced));

        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["providers"][0]["name"], "simulated");
        assert_eq!(json["provider_calls"][0]["kind"], "tool");
        assert_eq!(json["provider_calls"][1]["kind"], "model");
    }

    #[test]
    fn vm_loads_and_validates_provider_contracts_before_execution() {
        let mut bytecode = valid_bytecode();
        add_external_contract(&mut bytecode, false);
        let vm = Vm::new();
        let mut state = vm.initialize(&bytecode).unwrap();
        let registry = vm.load_provider_contracts(&bytecode, &mut state).unwrap();

        assert!(registry.contains("simulated"));
        assert!(registry.contains_contract("OpenAI"));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderContractDeclared));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderContractValidated));
    }

    #[test]
    fn vm_json_preserves_populated_provider_allowlists() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v012.argbc.json"
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
        assert_eq!(trace.vm_version, "0.12");
        assert_eq!(
            trace.provider_contracts[0].allowed_targets,
            vec!["GuardModel"]
        );
        assert_eq!(
            trace.provider_contracts[0].allowed_capabilities,
            vec!["model.invoke"]
        );
    }
    #[test]
    fn rejected_provider_contract_preserves_runtime_ledger() {
        let mut bytecode = valid_bytecode();
        add_external_contract(&mut bytecode, true);
        let vm = Vm::new();
        let mut state = vm.initialize(&bytecode).unwrap();
        let result = vm.load_provider_contracts(&bytecode, &mut state);

        assert!(result.is_err());
        assert_eq!(state.status, RuntimeStatus::Failed);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderContractRejected));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }

    #[test]
    fn reactive_json_lists_executable_providers_and_declarative_contracts_separately() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_boundary_v010.argbc.json"
        ))
        .unwrap();
        add_external_contract(&mut bytecode, false);
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

        assert_eq!(json["vm_version"], "0.12");
        assert_eq!(json["providers"][0]["name"], "simulated");
        assert_eq!(json["providers"][0]["enabled"], true);
        assert_eq!(json["provider_contracts"][0]["name"], "OpenAI");
        assert_eq!(json["provider_contracts"][0]["allowed_targets"], json!([]));
        assert_eq!(
            json["provider_contracts"][0]["allowed_capabilities"],
            json!([])
        );
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
