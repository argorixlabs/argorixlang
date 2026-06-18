use crate::symbols::{Symbols, COMMUNICATIVE_ACTS};
use argorix_parser::{
    ast::{Approval, CapabilityLevel, HandlerInstruction, Program},
    diagnostics::Diagnostic,
    span::Spanned,
};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Default)]
pub struct CheckOptions {
    pub allow_legacy_capabilities: bool,
}

pub fn check_program(program: &Program) -> Result<(), Vec<Diagnostic>> {
    check_program_with_options(program, CheckOptions::default())
}

pub fn check_program_with_options(
    program: &Program,
    options: CheckOptions,
) -> Result<(), Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let symbols = collect_symbols(program, &mut diagnostics);
    check_provider_contracts(program, &symbols, &mut diagnostics);
    check_policies(program, &mut diagnostics);
    check_tool_declarations(program, &symbols, &mut diagnostics);
    check_model_declarations(program, &symbols, &mut diagnostics);

    for type_decl in &program.types {
        let mut field_names = HashSet::new();
        for field in &type_decl.fields {
            report_duplicate(&mut field_names, &field.name, "field", &mut diagnostics);
            if !symbols.is_field_type(&field.field_type.value) {
                diagnostics.push(Diagnostic::new(
                    format!("unknown field type `{}`", field.field_type.value),
                    field.field_type.span,
                ));
            }
        }
    }

    for agent in &program.agents {
        for receive in &agent.receives {
            require_message_type(&symbols, &receive.message_type, &mut diagnostics);
            if let Some(from) = &receive.from {
                require_participant(&symbols, from, "receive source", &mut diagnostics);
            }
        }
        for send in &agent.sends {
            require_message_type(&symbols, &send.message_type, &mut diagnostics);
            if !symbols.is_participant(&send.to.value) {
                diagnostics.push(Diagnostic::new(
                    format!("unknown send destination `{}`", send.to.value),
                    send.to.span,
                ));
            }
        }
        check_agent_capabilities(program, &symbols, agent, options, &mut diagnostics);
        check_handlers(program, &symbols, agent, &mut diagnostics);
    }

    for protocol in &program.protocols {
        for step in &protocol.steps {
            require_participant(
                &symbols,
                &step.from,
                "protocol participant",
                &mut diagnostics,
            );
            require_participant(&symbols, &step.to, "protocol participant", &mut diagnostics);
            require_message_type(&symbols, &step.message_type, &mut diagnostics);
            if !COMMUNICATIVE_ACTS.contains(&step.act.value.as_str()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "unsupported communicative act `{}`; allowed acts: {}",
                        step.act.value,
                        COMMUNICATIVE_ACTS.join(", ")
                    ),
                    step.act.span,
                ));
            }
            verify_protocol_step(program, &symbols, step, &mut diagnostics);
        }
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn check_provider_contracts(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = HashSet::new();
    for provider in &program.providers {
        report_duplicate(&mut names, &provider.name, "provider", diagnostics);
        if provider.name.value == "simulated" {
            diagnostics.push(Diagnostic::new(
                "`simulated` is the reserved executable provider and must not be declared as a provider contract",
                provider.name.span,
            ));
        }
        if provider.kind.value.as_str() != "external" {
            diagnostics.push(Diagnostic::new(
                format!(
                    "provider contract `{}` must use kind `external` in v0.12",
                    provider.name.value
                ),
                provider.kind.span,
            ));
        }
        if provider.enabled.value {
            diagnostics.push(Diagnostic::new(
                format!(
                    "external provider contract `{}` must be disabled",
                    provider.name.value
                ),
                provider.enabled.span,
            ));
        }
        if !provider.dry_run_only.value {
            diagnostics.push(Diagnostic::new(
                format!(
                    "external provider contract `{}` requires `dry_run_only true`",
                    provider.name.value
                ),
                provider.dry_run_only.span,
            ));
        }
        if !provider.requires_feature_flag {
            diagnostics.push(Diagnostic::new(
                format!(
                    "external provider contract `{}` requires `requires feature_flag`",
                    provider.name.value
                ),
                provider.name.span,
            ));
        }
        if !provider.requires_explicit_approval {
            diagnostics.push(Diagnostic::new(
                format!(
                    "external provider contract `{}` requires `requires approval`",
                    provider.name.value
                ),
                provider.name.span,
            ));
        }

        let mut targets = HashSet::new();
        for target in &provider.allowed_targets {
            if !targets.insert(target.value.clone()) {
                diagnostics.push(Diagnostic::new(
                    format!("duplicate allowed target `{}`", target.value),
                    target.span,
                ));
                continue;
            }
            let tool = program
                .tools
                .iter()
                .find(|item| item.name.value == target.value);
            let model = program
                .models
                .iter()
                .find(|item| item.name.value == target.value);
            let capability = match (tool, model) {
                (None, None) => {
                    diagnostics.push(Diagnostic::new(
                        format!("unknown allowlist target `{}`", target.value),
                        target.span,
                    ));
                    None
                }
                (Some(_), Some(_)) => {
                    diagnostics.push(Diagnostic::new(
                        format!("ambiguous allowlist target `{}`", target.value),
                        target.span,
                    ));
                    None
                }
                (Some(tool), None) => Some(tool.capability.value.as_str()),
                (None, Some(model)) => Some(model.capability.value.as_str()),
            };
            if let Some(capability) = capability {
                if !provider.allowed_capabilities.is_empty()
                    && !provider
                        .allowed_capabilities
                        .iter()
                        .any(|allowed| allowed.value == capability)
                {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "allowlist target `{}` requires capability `{capability}`",
                            target.value
                        ),
                        target.span,
                    ));
                }
            }
        }

        let mut capabilities = HashSet::new();
        for capability in &provider.allowed_capabilities {
            if !capabilities.insert(capability.value.clone()) {
                diagnostics.push(Diagnostic::new(
                    format!("duplicate allowed capability `{}`", capability.value),
                    capability.span,
                ));
            }
            if !symbols.capabilities.contains_key(&capability.value) {
                diagnostics.push(Diagnostic::new(
                    format!("unknown allowlist capability `{}`", capability.value),
                    capability.span,
                ));
            }
        }
    }
}
fn check_policies(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    const ASSERTIONS: [&str; 6] = [
        "no_unhandled_messages",
        "all_tool_calls_traced",
        "all_model_calls_traced",
        "all_intrinsics_traced",
        "halt_requires_trace",
        "runtime_status",
    ];
    for assertion in &program.assertions {
        if !ASSERTIONS.contains(&assertion.name.value.as_str()) {
            diagnostics.push(Diagnostic::new(
                format!("unknown policy assertion `{}`", assertion.name.value),
                assertion.name.span,
            ));
        }
        if assertion.name.value == "runtime_status"
            && assertion
                .argument
                .as_ref()
                .map(|value| value.value.as_str())
                != Some("completed")
        {
            diagnostics.push(Diagnostic::new(
                "`runtime_status` assertion requires argument `completed`",
                assertion.name.span,
            ));
        }
    }
    let mut failures = HashSet::new();
    for failure in &program.failures {
        if !failures.insert(failure.name.value.clone()) {
            diagnostics.push(Diagnostic::new(
                format!("duplicate failure `{}`", failure.name.value),
                failure.name.span,
            ));
        }
        if !matches!(failure.action.value.as_str(), "block" | "review" | "halt") {
            diagnostics.push(Diagnostic::new(
                format!("invalid failure action `{}`", failure.action.value),
                failure.action.span,
            ));
        }
        if !failure.trace_required {
            diagnostics.push(Diagnostic::new(
                format!("failure `{}` requires `trace required`", failure.name.value),
                failure.name.span,
            ));
        }
    }
}

fn check_model_declarations(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for model in &program.models {
        if model.provider.value != "simulated" {
            diagnostics.push(Diagnostic::new(
                format!(
                    "model `{}` uses unsupported provider `{}`; only `simulated` is allowed",
                    model.name.value, model.provider.value
                ),
                model.provider.span,
            ));
        }
        if !symbols.capabilities.contains_key(&model.capability.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "model `{}` references unknown capability `{}`",
                    model.name.value, model.capability.value
                ),
                model.capability.span,
            ));
        }
        require_message_type(symbols, &model.input, diagnostics);
        require_message_type(symbols, &model.output, diagnostics);
    }
}

fn check_tool_declarations(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for tool in &program.tools {
        if let Some(provider) = &tool.provider {
            if provider.value != "simulated" {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "tool `{}` uses unsupported provider `{}`; only `simulated` is allowed",
                        tool.name.value, provider.value
                    ),
                    provider.span,
                ));
            }
        }
        if !symbols.capabilities.contains_key(&tool.capability.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "tool `{}` references unknown capability `{}`",
                    tool.name.value, tool.capability.value
                ),
                tool.capability.span,
            ));
        }
        require_message_type(symbols, &tool.input, diagnostics);
        require_message_type(symbols, &tool.output, diagnostics);
    }
}

fn check_handlers(
    program: &Program,
    symbols: &Symbols,
    agent: &argorix_parser::ast::AgentDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for tool in &agent.tools {
        if !symbols.tools.contains(&tool.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "agent `{}` references unknown tool `{}`",
                    agent.name.value, tool.value
                ),
                tool.span,
            ));
        }
    }
    for model in &agent.models {
        if !symbols.models.contains(&model.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "agent `{}` references unknown model `{}`",
                    agent.name.value, model.value
                ),
                model.span,
            ));
        }
    }
    let mut handled_types = HashSet::new();
    for handler in &agent.handlers {
        require_message_type(symbols, &handler.message_type, diagnostics);
        if !handled_types.insert(handler.message_type.value.clone()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "agent `{}` has duplicate handlers for `{}`",
                    agent.name.value, handler.message_type.value
                ),
                handler.message_type.span,
            ));
        }
        if !agent
            .receives
            .iter()
            .any(|receive| receive.message_type.value == handler.message_type.value)
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "handler `on {}` in agent `{}` requires `receives {}`",
                    handler.message_type.value, agent.name.value, handler.message_type.value
                ),
                handler.message_type.span,
            ));
        }

        for instruction in &handler.instructions {
            match instruction {
                HandlerInstruction::Emit { message_type, to } => {
                    require_message_type(symbols, message_type, diagnostics);
                    if !symbols.is_participant(&to.value) {
                        diagnostics.push(Diagnostic::new(
                            format!("unknown emit destination `{}`", to.value),
                            to.span,
                        ));
                    }
                    if !agent.sends.iter().any(|send| {
                        send.message_type.value == message_type.value && send.to.value == to.value
                    }) {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "handler emit in agent `{}` requires `sends {} to {}`",
                                agent.name.value, message_type.value, to.value
                            ),
                            message_type.span,
                        ));
                    }
                }
                HandlerInstruction::Trace { binding } => {
                    if binding.value != handler.binding.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "trace binding `{}` does not match handler binding `{}`",
                                binding.value, handler.binding.value
                            ),
                            binding.span,
                        ));
                    }
                }
                HandlerInstruction::Halt { span } => {
                    if !agent
                        .capabilities
                        .iter()
                        .any(|capability| capability.value == "runtime.halt")
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` uses `halt` without capability `runtime.halt`",
                                agent.name.value
                            ),
                            *span,
                        ));
                    }
                }
                HandlerInstruction::IntrinsicCall { name, argument } => {
                    if !matches!(name.value.as_str(), "facu" | "marron") {
                        diagnostics.push(Diagnostic::new(
                            format!("unknown runtime intrinsic `{}`", name.value),
                            name.span,
                        ));
                        continue;
                    }
                    if argument.value != handler.binding.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "intrinsic `{}` argument `{}` does not match handler binding `{}`",
                                name.value, argument.value, handler.binding.value
                            ),
                            argument.span,
                        ));
                    }
                    let required = if name.value == "facu" {
                        "state.write"
                    } else {
                        "runtime.guard"
                    };
                    if !agent
                        .capabilities
                        .iter()
                        .any(|capability| capability.value == required)
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` uses `{}` without capability `{required}`",
                                agent.name.value, name.value
                            ),
                            name.span,
                        ));
                    }
                }
                HandlerInstruction::CallTool { tool, binding } => {
                    let Some(declaration) = program
                        .tools
                        .iter()
                        .find(|item| item.name.value == tool.value)
                    else {
                        diagnostics.push(Diagnostic::new(
                            format!("unknown tool `{}`", tool.value),
                            tool.span,
                        ));
                        continue;
                    };
                    if !agent
                        .tools
                        .iter()
                        .any(|allowed| allowed.value == tool.value)
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` calls tool `{}` without declaring it in `tools`",
                                agent.name.value, tool.value
                            ),
                            tool.span,
                        ));
                    }
                    if binding.value != handler.binding.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "tool call binding `{}` does not match handler binding `{}`",
                                binding.value, handler.binding.value
                            ),
                            binding.span,
                        ));
                    }
                    if declaration.input.value != handler.message_type.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "tool `{}` expects `{}` but handler receives `{}`",
                                tool.value, declaration.input.value, handler.message_type.value
                            ),
                            tool.span,
                        ));
                    }
                    if !agent
                        .capabilities
                        .iter()
                        .any(|capability| capability.value == declaration.capability.value)
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` calls tool `{}` without capability `{}`",
                                agent.name.value, tool.value, declaration.capability.value
                            ),
                            tool.span,
                        ));
                    } else if symbols
                        .capabilities
                        .get(&declaration.capability.value)
                        .is_some_and(|level| {
                            matches!(
                                level,
                                CapabilityLevel::Restricted | CapabilityLevel::Dangerous
                            )
                        })
                        && agent.effective_approval() != Approval::Granted
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` calls restricted tool `{}` without approval",
                                agent.name.value, tool.value
                            ),
                            tool.span,
                        ));
                    }
                }
                HandlerInstruction::AskModel { model, binding } => {
                    let Some(declaration) = program
                        .models
                        .iter()
                        .find(|item| item.name.value == model.value)
                    else {
                        diagnostics.push(Diagnostic::new(
                            format!("unknown model `{}`", model.value),
                            model.span,
                        ));
                        continue;
                    };
                    if !agent
                        .models
                        .iter()
                        .any(|allowed| allowed.value == model.value)
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` asks model `{}` without declaring it in `models`",
                                agent.name.value, model.value
                            ),
                            model.span,
                        ));
                    }
                    if binding.value != handler.binding.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "model call binding `{}` does not match handler binding `{}`",
                                binding.value, handler.binding.value
                            ),
                            binding.span,
                        ));
                    }
                    if declaration.input.value != handler.message_type.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "model `{}` expects `{}` but handler receives `{}`",
                                model.value, declaration.input.value, handler.message_type.value
                            ),
                            model.span,
                        ));
                    }
                    if !agent
                        .capabilities
                        .iter()
                        .any(|capability| capability.value == declaration.capability.value)
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` asks model `{}` without capability `{}`",
                                agent.name.value, model.value, declaration.capability.value
                            ),
                            model.span,
                        ));
                    } else if symbols
                        .capabilities
                        .get(&declaration.capability.value)
                        .is_some_and(|level| {
                            matches!(
                                level,
                                CapabilityLevel::Restricted | CapabilityLevel::Dangerous
                            )
                        })
                        && agent.effective_approval() != Approval::Granted
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` asks restricted model `{}` without approval",
                                agent.name.value, model.value
                            ),
                            model.span,
                        ));
                    }
                }
            }
        }
    }
}

fn collect_symbols(program: &Program, diagnostics: &mut Vec<Diagnostic>) -> Symbols {
    let mut symbols = Symbols::default();
    for type_decl in &program.types {
        report_duplicate(&mut symbols.types, &type_decl.name, "type", diagnostics);
    }
    for enum_decl in &program.enums {
        report_duplicate(&mut symbols.enums, &enum_decl.name, "enum", diagnostics);
    }
    for agent in &program.agents {
        report_duplicate(&mut symbols.agents, &agent.name, "agent", diagnostics);
    }
    for tool in &program.tools {
        report_duplicate(&mut symbols.tools, &tool.name, "tool", diagnostics);
    }
    for model in &program.models {
        report_duplicate(&mut symbols.models, &model.name, "model", diagnostics);
    }
    for capability in &program.capabilities {
        if symbols
            .capabilities
            .insert(capability.name.value.clone(), capability.level.value)
            .is_some()
        {
            diagnostics.push(Diagnostic::new(
                format!("duplicate capability `{}`", capability.name.value),
                capability.name.span,
            ));
        }
    }
    symbols
}

fn check_agent_capabilities(
    program: &Program,
    symbols: &Symbols,
    agent: &argorix_parser::ast::AgentDecl,
    options: CheckOptions,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if program.capabilities.is_empty() && options.allow_legacy_capabilities {
        return;
    }

    for capability in &agent.capabilities {
        let Some(level) = symbols.capabilities.get(&capability.value) else {
            diagnostics.push(Diagnostic::new(
                format!(
                    "Unknown capability {} used by agent {}.",
                    capability.value, agent.name.value
                ),
                capability.span,
            ));
            continue;
        };

        if matches!(
            level,
            CapabilityLevel::Restricted | CapabilityLevel::Dangerous
        ) && agent.effective_approval() != Approval::Granted
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "Agent {} uses {} capability {} without approval.",
                    agent.name.value,
                    level.as_str(),
                    capability.value
                ),
                capability.span,
            ));
        }
    }
}

fn verify_protocol_step(
    program: &Program,
    symbols: &Symbols,
    step: &argorix_parser::ast::ProtocolStep,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if symbols.agents.contains(&step.from.value) {
        let sender = program
            .agents
            .iter()
            .find(|agent| agent.name.value == step.from.value)
            .expect("agent symbol must resolve to an AST declaration");
        let has_send = sender.sends.iter().any(|send| {
            send.message_type.value == step.message_type.value && send.to.value == step.to.value
        });
        if !has_send {
            diagnostics.push(Diagnostic::new(
                format!(
                    "Protocol step requires agent {} to declare `sends {} to {}`.",
                    step.from.value, step.message_type.value, step.to.value
                ),
                step.from.span,
            ));
        }
    }

    if symbols.agents.contains(&step.to.value) {
        let receiver = program
            .agents
            .iter()
            .find(|agent| agent.name.value == step.to.value)
            .expect("agent symbol must resolve to an AST declaration");
        let has_receive = receiver.receives.iter().any(|receive| {
            receive.message_type.value == step.message_type.value
                && receive
                    .from
                    .as_ref()
                    .is_none_or(|from| from.value == step.from.value)
        });
        if !has_receive {
            diagnostics.push(Diagnostic::new(
                format!(
                    "Protocol step requires agent {} to declare `receives {} from {}`.",
                    step.to.value, step.message_type.value, step.from.value
                ),
                step.to.span,
            ));
        }
    }
}

fn report_duplicate(
    names: &mut HashSet<String>,
    name: &Spanned<String>,
    kind: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !names.insert(name.value.clone()) {
        diagnostics.push(Diagnostic::new(
            format!("duplicate {kind} `{}`", name.value),
            name.span,
        ));
    }
}

fn require_message_type(
    symbols: &Symbols,
    message_type: &Spanned<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !symbols.types.contains(&message_type.value) {
        diagnostics.push(Diagnostic::new(
            format!(
                "unknown message type `{}`; messages must reference a declared `type`",
                message_type.value
            ),
            message_type.span,
        ));
    }
}

fn require_participant(
    symbols: &Symbols,
    participant: &Spanned<String>,
    role: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !symbols.is_participant(&participant.value) {
        diagnostics.push(Diagnostic::new(
            format!("unknown {role} `{}`", participant.value),
            participant.span,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::check_program;
    use argorix_parser::parse_source;

    #[test]
    fn rejects_unknown_messages_and_agents() {
        let source = r#"
            module Example
            agent Sender { sends Missing to Nobody }
        "#;
        let program = parse_source(source).unwrap();
        let diagnostics = check_program(&program).unwrap_err();
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].message.contains("unknown message type"));
        assert!(diagnostics[1].message.contains("unknown send destination"));
    }
}
