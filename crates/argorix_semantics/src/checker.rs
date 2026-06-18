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

fn check_handlers(
    _program: &Program,
    symbols: &Symbols,
    agent: &argorix_parser::ast::AgentDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
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
