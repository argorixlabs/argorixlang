use crate::{
    EmittedMessage, EventFields, EventType, InjectedMessage, InvokedIntrinsic, MessageEnvelope,
    ModelCallEnvelope, ReactiveStep, RuntimeState, RuntimeStatus, StateCheckpoint,
    ToolCallEnvelope, VmError,
};
use argorix_bytecode::{verify_bytecode, BytecodeProgram, Instruction};
use argorix_provider::{
    ModelProviderRequest, ProviderCallStatus, ProviderRegistry, ToolProviderRequest,
};
use serde_json::json;
use std::collections::{HashMap, HashSet};

const EXTERNAL_ENTITIES: [&str; 5] = ["User", "System", "Runtime", "Memory", "Tool"];

#[derive(Debug, Clone)]
enum HandlerOp {
    Emit { message_type: String, to: String },
    Trace { binding: String },
    Halt,
    Intrinsic { name: String, argument: String },
    ToolCall { tool: String, binding: String },
    ModelCall { model: String, binding: String },
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
        self.run_with_registry(bytecode, state, injected, &ProviderRegistry::default())
    }

    pub fn run_with_registry(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
        injected: &InjectedMessage,
        providers: &ProviderRegistry,
    ) -> Result<Vec<ReactiveStep>, VmError> {
        state.trace_ledger.record(
            EventType::VmStarted,
            "ok",
            "reactive deterministic scheduler started",
            EventFields::default(),
        );
        state.status = RuntimeStatus::Running;
        self.block_declared_external_execution(bytecode, state, providers)?;
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
        let tools = bytecode
            .tools
            .iter()
            .map(|tool| (tool.name.as_str(), tool))
            .collect::<HashMap<_, _>>();
        let models = bytecode
            .models
            .iter()
            .map(|model| (model.name.as_str(), model))
            .collect::<HashMap<_, _>>();
        let authorized = bytecode
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                Instruction::AuthorizeTool { agent, tool } => Some((agent.as_str(), tool.as_str())),
                _ => None,
            })
            .collect::<HashSet<_>>();
        let authorized_models = bytecode
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                Instruction::AuthorizeModel { agent, model } => {
                    Some((agent.as_str(), model.as_str()))
                }
                _ => None,
            })
            .collect::<HashSet<_>>();
        let agent_capabilities = bytecode
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                Instruction::RequireCapability { agent, capability } => {
                    Some((agent.as_str(), capability.as_str()))
                }
                _ => None,
            })
            .collect::<HashSet<_>>();
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
            let mut called_tools = Vec::new();
            let mut called_models = Vec::new();
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
                    HandlerOp::ToolCall { tool, binding } => {
                        if let Err(error) = self.call_tool(
                            state,
                            &tools,
                            &authorized,
                            &agent_capabilities,
                            &key.0,
                            &handler.binding,
                            tool,
                            binding,
                            &processed,
                            providers,
                        ) {
                            state.fail(error.to_string());
                            return Err(error);
                        }
                        called_tools.push(tool.clone());
                    }
                    HandlerOp::ModelCall { model, binding } => {
                        if let Err(error) = self.call_model(
                            state,
                            &models,
                            &authorized_models,
                            &agent_capabilities,
                            &key.0,
                            &handler.binding,
                            model,
                            binding,
                            &processed,
                            providers,
                        ) {
                            state.fail(error.to_string());
                            return Err(error);
                        }
                        called_models.push(model.clone());
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
                tool_calls: called_tools,
                model_calls: called_models,
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

    fn block_declared_external_execution(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
        providers: &ProviderRegistry,
    ) -> Result<(), VmError> {
        for instruction in &bytecode.instructions {
            let (provider, target, kind) = match instruction {
                Instruction::CallTool { tool, .. } => {
                    let Some(declaration) = bytecode
                        .tools
                        .iter()
                        .find(|candidate| candidate.name == *tool)
                    else {
                        continue;
                    };
                    (declaration.provider.as_str(), tool.as_str(), "tool")
                }
                Instruction::AskModel { model, .. } => {
                    let Some(declaration) = bytecode
                        .models
                        .iter()
                        .find(|candidate| candidate.name == *model)
                    else {
                        continue;
                    };
                    (declaration.provider.as_str(), model.as_str(), "model")
                }
                _ => continue,
            };
            if providers.contains_contract(provider) {
                let reason = format!(
                    "external provider contract `{provider}` cannot execute {kind} {target}"
                );
                state.activate_failure(if kind == "tool" {
                    "ToolDenied"
                } else {
                    "ModelDenied"
                });
                state.trace_ledger.record(
                    EventType::ExternalProviderExecutionBlocked,
                    "blocked",
                    reason.clone(),
                    EventFields::default(),
                );
                state.fail(reason.clone());
                return Err(VmError::ProviderBoundary {
                    provider: provider.into(),
                    reason,
                });
            }
        }
        Ok(())
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
                Instruction::CallTool { tool, binding, .. } => {
                    if let Some((_, _, handler)) = &mut current {
                        handler.operations.push(HandlerOp::ToolCall {
                            tool: tool.clone(),
                            binding: binding.clone(),
                        });
                    }
                }
                Instruction::AskModel { model, binding, .. } => {
                    if let Some((_, _, handler)) = &mut current {
                        handler.operations.push(HandlerOp::ModelCall {
                            model: model.clone(),
                            binding: binding.clone(),
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

    #[allow(clippy::too_many_arguments)]
    fn call_tool(
        &self,
        state: &mut RuntimeState,
        tools: &HashMap<&str, &argorix_bytecode::BytecodeTool>,
        authorized: &HashSet<(&str, &str)>,
        agent_capabilities: &HashSet<(&str, &str)>,
        agent: &str,
        handler_binding: &str,
        tool_name: &str,
        binding: &str,
        message: &MessageEnvelope,
        providers: &ProviderRegistry,
    ) -> Result<(), VmError> {
        if binding != handler_binding {
            return Err(VmError::ToolBindingMismatch {
                argument: binding.to_owned(),
                binding: handler_binding.to_owned(),
            });
        }
        let tool = tools
            .get(tool_name)
            .ok_or_else(|| VmError::UnknownTool(tool_name.to_owned()))?;
        state.trace_ledger.record(
            EventType::ToolCallRequested,
            "requested",
            format!("{agent} requested tool {tool_name}"),
            EventFields::target(agent),
        );
        if !authorized.contains(&(agent, tool_name))
            || !agent_capabilities.contains(&(agent, tool.capability.as_str()))
        {
            state.activate_failure("ToolDenied");
            state.trace_ledger.record(
                EventType::ToolCallDenied,
                "denied",
                format!("{agent} denied tool {tool_name}"),
                EventFields::target(agent),
            );
            return Err(VmError::ToolNotAuthorized {
                agent: agent.to_owned(),
                tool: tool_name.to_owned(),
            });
        }
        state.trace_ledger.record(
            EventType::ToolCallAllowed,
            "allowed",
            format!("tool {tool_name} allowed by capability {}", tool.capability),
            EventFields::target(agent),
        );
        let call_id = format!("tool_{:03}", state.tool_calls.len() + 1);
        let provider_name = tool.provider.as_str();
        if providers.contains_contract(provider_name) {
            let reason = format!(
                "external provider contract `{provider_name}` cannot execute tool {tool_name}"
            );
            state.activate_failure("ToolDenied");
            state.trace_ledger.record(
                EventType::ExternalProviderExecutionBlocked,
                "blocked",
                reason.clone(),
                EventFields::target(agent),
            );
            return Err(VmError::ProviderBoundary {
                provider: provider_name.into(),
                reason,
            });
        }
        state.trace_ledger.record(
            EventType::ProviderSelected,
            "selected",
            format!("provider {provider_name} selected for tool {tool_name}"),
            EventFields::target(agent),
        );
        let provider = providers.get(provider_name).map_err(|error| {
            state.activate_failure("ToolDenied");
            state.trace_ledger.record(
                EventType::ProviderBoundaryDenied,
                "denied",
                error.to_string(),
                EventFields::target(agent),
            );
            VmError::ProviderBoundary {
                provider: provider_name.to_owned(),
                reason: error.to_string(),
            }
        })?;
        state.trace_ledger.record(
            EventType::ProviderRequestCreated,
            "created",
            format!("provider request {call_id} created for tool {tool_name}"),
            EventFields::target(agent),
        );
        state.trace_ledger.record(
            EventType::ProviderDryRunEnforced,
            "enforced",
            format!("provider {provider_name} restricted to dry-run"),
            EventFields::target(agent),
        );
        let response = provider
            .invoke_tool(ToolProviderRequest {
                call_id: call_id.clone(),
                agent: agent.to_owned(),
                tool: tool_name.to_owned(),
                input_type: tool.input.clone(),
                output_type: tool.output.clone(),
                input_binding: binding.to_owned(),
                dry_run: true,
            })
            .map_err(|error| {
                state.activate_failure("ToolDenied");
                state.trace_ledger.record(
                    EventType::ProviderBoundaryDenied,
                    "denied",
                    error.to_string(),
                    EventFields::target(agent),
                );
                VmError::ProviderBoundary {
                    provider: provider_name.to_owned(),
                    reason: error.to_string(),
                }
            })?;
        state.trace_ledger.record(
            EventType::ProviderResponseReceived,
            "received",
            format!("provider response received for {call_id}"),
            EventFields::target(agent),
        );
        if response.call_id != call_id
            || response.output_type != tool.output
            || response.status != ProviderCallStatus::Allowed
            || !response.simulated
        {
            state.activate_failure("ToolDenied");
            state.trace_ledger.record(
                EventType::ProviderBoundaryDenied,
                "denied",
                format!("invalid provider response for {call_id}"),
                EventFields::target(agent),
            );
            return Err(VmError::ProviderBoundary {
                provider: provider_name.to_owned(),
                reason: "invalid tool response".into(),
            });
        }
        state.tool_calls.push(ToolCallEnvelope {
            id: call_id,
            agent: agent.to_owned(),
            tool: tool_name.to_owned(),
            capability: tool.capability.clone(),
            input_binding: binding.to_owned(),
            input_message_id: message.id.clone(),
            status: "allowed".into(),
        });
        state.provider_calls.push(crate::ProviderCallSummary {
            provider: provider_name.to_owned(),
            kind: "tool".into(),
            target: tool_name.to_owned(),
            status: "allowed".into(),
            simulated: response.simulated,
        });
        state.trace_ledger.record(
            EventType::ToolCallDryRunResult,
            "ok",
            format!("tool {tool_name} dry-run result generated"),
            EventFields::target(agent),
        );
        Ok(())
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

    #[allow(clippy::too_many_arguments)]
    fn call_model(
        &self,
        state: &mut RuntimeState,
        models: &HashMap<&str, &argorix_bytecode::BytecodeModel>,
        authorized: &HashSet<(&str, &str)>,
        agent_capabilities: &HashSet<(&str, &str)>,
        agent: &str,
        handler_binding: &str,
        model_name: &str,
        binding: &str,
        message: &MessageEnvelope,
        providers: &ProviderRegistry,
    ) -> Result<(), VmError> {
        if binding != handler_binding {
            return Err(VmError::ModelBindingMismatch {
                argument: binding.to_owned(),
                binding: handler_binding.to_owned(),
            });
        }
        let model = models
            .get(model_name)
            .ok_or_else(|| VmError::UnknownModel(model_name.to_owned()))?;
        state.trace_ledger.record(
            EventType::ModelCallRequested,
            "requested",
            format!("{agent} asked model {model_name}"),
            EventFields::target(agent),
        );
        if !authorized.contains(&(agent, model_name))
            || !agent_capabilities.contains(&(agent, model.capability.as_str()))
        {
            state.activate_failure("ModelDenied");
            state.trace_ledger.record(
                EventType::ModelCallDenied,
                "denied",
                format!("{agent} denied model {model_name}"),
                EventFields::target(agent),
            );
            return Err(VmError::ModelNotAuthorized {
                agent: agent.to_owned(),
                model: model_name.to_owned(),
            });
        }
        state.trace_ledger.record(
            EventType::ModelCallAllowed,
            "allowed",
            format!(
                "model {model_name} allowed by capability {}",
                model.capability
            ),
            EventFields::target(agent),
        );
        let call_id = format!("model_{:03}", state.model_calls.len() + 1);
        let provider_name = model.provider.as_str();
        if providers.contains_contract(provider_name) {
            let reason = format!(
                "external provider contract `{provider_name}` cannot execute model {model_name}"
            );
            state.activate_failure("ModelDenied");
            state.trace_ledger.record(
                EventType::ExternalProviderExecutionBlocked,
                "blocked",
                reason.clone(),
                EventFields::target(agent),
            );
            return Err(VmError::ProviderBoundary {
                provider: provider_name.into(),
                reason,
            });
        }
        state.trace_ledger.record(
            EventType::ProviderSelected,
            "selected",
            format!("provider {provider_name} selected for model {model_name}"),
            EventFields::target(agent),
        );
        let provider = providers.get(provider_name).map_err(|error| {
            state.activate_failure("ModelDenied");
            state.trace_ledger.record(
                EventType::ProviderBoundaryDenied,
                "denied",
                error.to_string(),
                EventFields::target(agent),
            );
            VmError::ProviderBoundary {
                provider: provider_name.to_owned(),
                reason: error.to_string(),
            }
        })?;
        state.trace_ledger.record(
            EventType::ProviderRequestCreated,
            "created",
            format!("provider request {call_id} created for model {model_name}"),
            EventFields::target(agent),
        );
        state.trace_ledger.record(
            EventType::ProviderDryRunEnforced,
            "enforced",
            format!("provider {provider_name} restricted to dry-run"),
            EventFields::target(agent),
        );
        let response = provider
            .invoke_model(ModelProviderRequest {
                call_id: call_id.clone(),
                agent: agent.to_owned(),
                model: model_name.to_owned(),
                input_type: model.input.clone(),
                output_type: model.output.clone(),
                input_binding: binding.to_owned(),
                dry_run: true,
            })
            .map_err(|error| {
                state.activate_failure("ModelDenied");
                state.trace_ledger.record(
                    EventType::ProviderBoundaryDenied,
                    "denied",
                    error.to_string(),
                    EventFields::target(agent),
                );
                VmError::ProviderBoundary {
                    provider: provider_name.to_owned(),
                    reason: error.to_string(),
                }
            })?;
        state.trace_ledger.record(
            EventType::ProviderResponseReceived,
            "received",
            format!("provider response received for {call_id}"),
            EventFields::target(agent),
        );
        if response.call_id != call_id
            || response.output_type != model.output
            || response.status != ProviderCallStatus::Allowed
            || !response.simulated
        {
            state.activate_failure("ModelDenied");
            state.trace_ledger.record(
                EventType::ProviderBoundaryDenied,
                "denied",
                format!("invalid provider response for {call_id}"),
                EventFields::target(agent),
            );
            return Err(VmError::ProviderBoundary {
                provider: provider_name.to_owned(),
                reason: "invalid model response".into(),
            });
        }
        state.model_calls.push(ModelCallEnvelope {
            id: call_id,
            agent: agent.to_owned(),
            model: model_name.to_owned(),
            provider: model.provider.clone(),
            capability: model.capability.clone(),
            input_binding: binding.to_owned(),
            input_message_id: message.id.clone(),
            status: "allowed".into(),
        });
        state.provider_calls.push(crate::ProviderCallSummary {
            provider: provider_name.to_owned(),
            kind: "model".into(),
            target: model_name.to_owned(),
            status: "allowed".into(),
            simulated: response.simulated,
        });
        state.trace_ledger.record(
            EventType::ModelCallDryRunResult,
            "ok",
            format!("model {model_name} dry-run result generated"),
            EventFields::target(agent),
        );
        Ok(())
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
    use crate::{EventType, InjectedMessage, RuntimeState, RuntimeStatus, VmError};
    use argorix_bytecode::{BytecodeProgram, Instruction};
    use argorix_provider::{AdapterContract, ProviderKind, ProviderRegistry};

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

    fn v07_fixture() -> BytecodeProgram {
        serde_json::from_str(include_str!("../../../examples/tool_call_v07.argbc.json")).unwrap()
    }

    fn v08_fixture() -> BytecodeProgram {
        serde_json::from_str(include_str!("../../../examples/model_call_v08.argbc.json")).unwrap()
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

    #[test]
    fn tool_call_records_requested_allowed_and_result_events() {
        let bytecode = v07_fixture();
        let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
        ReactiveScheduler::new()
            .run(
                &bytecode,
                &mut state,
                &InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        assert_eq!(state.tool_calls.len(), 1);
        for expected in [
            EventType::ToolCallRequested,
            EventType::ToolCallAllowed,
            EventType::ToolCallDryRunResult,
        ] {
            assert!(state
                .trace_ledger
                .events
                .iter()
                .any(|event| event.event_type == expected));
        }
    }

    #[test]
    fn model_call_records_requested_allowed_and_result_events() {
        let bytecode = v08_fixture();
        let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
        ReactiveScheduler::new()
            .run(
                &bytecode,
                &mut state,
                &InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        assert_eq!(state.model_calls.len(), 1);
        for expected in [
            EventType::ModelCallRequested,
            EventType::ModelCallAllowed,
            EventType::ModelCallDryRunResult,
        ] {
            assert!(state
                .trace_ledger
                .events
                .iter()
                .any(|event| event.event_type == expected));
        }
    }

    #[test]
    fn missing_provider_fails_closed_and_preserves_ledger() {
        let bytecode = v08_fixture();
        let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
        let result = ReactiveScheduler::new().run_with_registry(
            &bytecode,
            &mut state,
            &InjectedMessage {
                from: "User".into(),
                to: "ResearchAgent".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
            &ProviderRegistry::empty(),
        );

        assert!(matches!(result, Err(VmError::ProviderBoundary { .. })));
        assert_eq!(state.status, RuntimeStatus::Failed);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderBoundaryDenied));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::FailureModeActivated));
    }

    #[test]
    fn external_provider_execution_is_blocked_and_preserves_ledger() {
        let mut bytecode = v08_fixture();
        bytecode.models[0].provider = "OpenAI".into();
        let mut registry = ProviderRegistry::default();
        registry
            .register_contract(AdapterContract {
                name: "OpenAI".into(),
                kind: ProviderKind::External,
                enabled: false,
                dry_run_only: true,
                requires_feature_flag: true,
                requires_explicit_approval: true,
                allowed_targets: vec![],
                allowed_capabilities: vec![],
            })
            .unwrap();
        let mut state = RuntimeState::from_bytecode(&bytecode).unwrap();
        let result = ReactiveScheduler::new().run_with_registry(
            &bytecode,
            &mut state,
            &InjectedMessage {
                from: "User".into(),
                to: "ResearchAgent".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
            &registry,
        );

        assert!(matches!(result, Err(VmError::ProviderBoundary { .. })));
        assert_eq!(state.status, RuntimeStatus::Failed);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| { event.event_type == EventType::ExternalProviderExecutionBlocked }));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::FailureModeActivated));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }
}
