use crate::symbols::{Symbols, COMMUNICATIVE_ACTS};
use argorix_parser::{
    ast::{
        Approval, CapabilityLevel, HandlerInstruction, HarnessFilesystem, HarnessMode,
        HarnessNetwork, HarnessSecrets, PolicyRule, PolicyRuleDecl, PolicyViolationAction,
        Program,
    },
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
    check_provider_harnesses(program, &symbols, &mut diagnostics);
    check_policies(program, &mut diagnostics);
    check_passports(program, &symbols, &mut diagnostics);
    check_tool_declarations(program, &symbols, &mut diagnostics);
    check_model_declarations(program, &symbols, &mut diagnostics);

    for type_decl in &program.types {
        let mut field_names = HashSet::new();
        for field in &type_decl.fields {
            report_duplicate(&mut field_names, &field.name, "field", &mut diagnostics);
            let field_type = field.field_type.value.source_name();
            if !field.field_type.value.is_primitive() && !symbols.is_field_type(field_type) {
                diagnostics.push(Diagnostic::new(
                    format!("unknown message field type `{field_type}`"),
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

fn check_provider_harnesses(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = HashSet::new();
    for harness in &program.harnesses {
        report_duplicate(&mut names, &harness.name, "harness", diagnostics);
        let harness_name = &harness.name.value;

        if harness.provider.value.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` is missing required field `provider`"),
                harness.provider.span,
            ));
        } else if !symbols.providers.contains(&harness.provider.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "harness `{harness_name}` references unknown provider `{}`",
                    harness.provider.value
                ),
                harness.provider.span,
            ));
        }

        match &harness.mode.value {
            HarnessMode::Unknown(value) if value.is_empty() => diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` is missing required field `mode`"),
                harness.mode.span,
            )),
            HarnessMode::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` has invalid mode `{value}`"),
                harness.mode.span,
            )),
            HarnessMode::DryRun | HarnessMode::Simulated => {}
        }
        match &harness.network.value {
            HarnessNetwork::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("harness `{harness_name}` is missing required field `network`"),
                    harness.network.span,
                ))
            }
            HarnessNetwork::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` has invalid network `{value}`"),
                harness.network.span,
            )),
            HarnessNetwork::Denied => {}
        }
        match &harness.secrets.value {
            HarnessSecrets::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("harness `{harness_name}` is missing required field `secrets`"),
                    harness.secrets.span,
                ))
            }
            HarnessSecrets::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` has invalid secrets `{value}`"),
                harness.secrets.span,
            )),
            HarnessSecrets::Denied => {}
        }
        match &harness.filesystem.value {
            HarnessFilesystem::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("harness `{harness_name}` is missing required field `filesystem`"),
                    harness.filesystem.span,
                ))
            }
            HarnessFilesystem::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` has invalid filesystem `{value}`"),
                harness.filesystem.span,
            )),
            HarnessFilesystem::None | HarnessFilesystem::ReadOnly => {}
        }

        for (label, limit) in [
            ("max_steps", harness.max_steps.as_ref()),
            ("timeout_ms", harness.timeout_ms.as_ref()),
        ] {
            if let Some(limit) = limit {
                if limit.value == 0 {
                    diagnostics.push(Diagnostic::new(
                        format!("harness `{harness_name}` {label} must be positive"),
                        limit.span,
                    ));
                }
            }
        }

        for (label, contract) in [
            ("input_contract", harness.input_contract.as_ref()),
            ("output_contract", harness.output_contract.as_ref()),
        ] {
            if let Some(contract) = contract {
                if !symbols.types.contains(&contract.value) {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "harness `{harness_name}` references unknown {label} `{}`",
                            contract.value
                        ),
                        contract.span,
                    ));
                }
            }
        }

        for attestation in &harness.attestations {
            if attestation.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("harness `{harness_name}` attestation entries must not be empty"),
                    attestation.span,
                ));
            }
        }
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
                    "provider contract `{}` must use kind `external` in v0.15",
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
    let mut policy_names = HashSet::new();
    for policy in &program.policies {
        if !policy_names.insert(policy.name.value.clone()) {
            diagnostics.push(Diagnostic::new(
                format!("duplicate policy `{}`", policy.name.value),
                policy.name.span,
            ));
        }
        let mut required = HashSet::new();
        let mut denied = HashSet::new();
        for declaration in &policy.rules {
            let rule = declaration.rule();
            if let PolicyRule::Unknown(value) = &rule.value {
                diagnostics.push(Diagnostic::new(
                    format!("unknown policy rule `{value}`"),
                    rule.span,
                ));
                continue;
            }
            let name = rule.value.source_name();
            let (current, opposite) = match declaration {
                PolicyRuleDecl::Require { .. } => (&mut required, &denied),
                PolicyRuleDecl::Deny { .. } => (&mut denied, &required),
            };
            if !current.insert(name.clone()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "duplicate {} rule `{name}` in policy `{}`",
                        declaration.effect(),
                        policy.name.value
                    ),
                    rule.span,
                ));
            }
            if opposite.contains(&name) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "contradictory policy rule `{name}` in policy `{}`",
                        policy.name.value
                    ),
                    rule.span,
                ));
            }
        }
        if let Some(violation) = &policy.violation {
            if let PolicyViolationAction::Unknown(value) = &violation.action.value {
                diagnostics.push(Diagnostic::new(
                    format!("invalid policy violation action `{value}`"),
                    violation.action.span,
                ));
            }
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

/// Validate v0.19 Agent Passport blocks.
///
/// Passports are sovereign-identity metadata only. Validation is structural and
/// offline: no DID resolution, ASN lookup, registry queries, or real country
/// verification beyond a basic ISO-like format check.
fn check_passports(program: &Program, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) {
    const RISK_LEVELS: [&str; 4] = ["low", "medium", "high", "critical"];
    const ASN_REGISTRIES: [&str; 6] = ["LACNIC", "ARIN", "RIPE", "APNIC", "AFRINIC", "UNKNOWN"];

    let mut passport_names = HashSet::new();
    let mut agents_with_passport: HashSet<String> = HashSet::new();
    for passport in &program.passports {
        let name = passport.name.value.clone();
        if !passport_names.insert(name.clone()) {
            diagnostics.push(Diagnostic::new(
                format!("duplicate passport `{name}`"),
                passport.name.span,
            ));
        }

        // Required, non-empty scalar fields.
        for (label, field) in [
            ("agent", &passport.agent),
            ("agent_name", &passport.agent_name),
            ("global_id", &passport.global_id),
            ("identity", &passport.identity),
            ("provider", &passport.provider),
            ("version", &passport.version),
            ("country", &passport.country),
            ("jurisdiction", &passport.jurisdiction),
            ("intent", &passport.intent),
            ("risk_level", &passport.risk_level),
        ] {
            if field.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("passport `{name}` is missing required field `{label}`"),
                    field.span,
                ));
            }
        }

        if passport.data_residency.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("passport `{name}` requires non-empty `data_residency`"),
                passport.name.span,
            ));
        }

        // Agent reference must resolve to a declared agent, at most one per agent.
        if !passport.agent.value.trim().is_empty() {
            if !symbols.agents.contains(&passport.agent.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "passport `{name}` references unknown agent `{}`",
                        passport.agent.value
                    ),
                    passport.agent.span,
                ));
            } else if !agents_with_passport.insert(passport.agent.value.clone()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "agent `{}` already has a passport; only one passport per agent is allowed",
                        passport.agent.value
                    ),
                    passport.agent.span,
                ));
            }
        }

        if !passport.country.value.trim().is_empty() && !is_iso_country(&passport.country.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "passport `{name}` country `{}` must use a 2-letter ISO-like code",
                    passport.country.value
                ),
                passport.country.span,
            ));
        }

        if !passport.risk_level.value.trim().is_empty()
            && !RISK_LEVELS.contains(&passport.risk_level.value.as_str())
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "passport `{name}` has invalid risk_level `{}`; allowed: {}",
                    passport.risk_level.value,
                    RISK_LEVELS.join(", ")
                ),
                passport.risk_level.span,
            ));
        }

        if let Some(asn) = &passport.asn {
            if !ASN_REGISTRIES.contains(&asn.registry.value.as_str()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "passport `{name}` has invalid asn registry `{}`; allowed: {}",
                        asn.registry.value,
                        ASN_REGISTRIES.join(", ")
                    ),
                    asn.registry.span,
                ));
            }
            if !is_valid_asn_number(&asn.number.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "passport `{name}` asn number `{}` must be an `AS`-prefixed value or explicit placeholder",
                        asn.number.value
                    ),
                    asn.number.span,
                ));
            }
            if !asn.country.value.trim().is_empty() && !is_iso_country(&asn.country.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "passport `{name}` asn country `{}` must use a 2-letter ISO-like code",
                        asn.country.value
                    ),
                    asn.country.span,
                ));
            }
        }
    }
}

fn is_iso_country(value: &str) -> bool {
    value.len() == 2
        && value
            .chars()
            .all(|character| character.is_ascii_uppercase())
}

fn is_valid_asn_number(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && (value.starts_with("AS") || value == "PLACEHOLDER")
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
    for provider in &program.providers {
        symbols.providers.insert(provider.name.value.clone());
    }
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

    #[test]
    fn rejects_invalid_policy_v2_declarations() {
        let source = r#"
            module main
            policy Duplicate {
                require external_execution
                require external_execution
            }
            policy Contradictory {
                require external_execution
                deny external_execution
            }
            policy UnknownRule {
                require future_rule
                on violation { action future_action }
            }
            policy Duplicate { deny external_execution }
        "#;
        let program = parse_source(source).unwrap();
        let messages = check_program(&program)
            .unwrap_err()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>();
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate policy `Duplicate`")));
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate require rule `external_execution`")));
        assert!(messages
            .iter()
            .any(|message| { message.contains("contradictory policy rule `external_execution`") }));
        assert!(messages
            .iter()
            .any(|message| message.contains("unknown policy rule `future_rule`")));
        assert!(messages
            .iter()
            .any(|message| message.contains("invalid policy violation action `future_action`")));
    }

    #[test]
    fn accepts_policy_v2_and_legacy_assertions_together() {
        let source = r#"
            module main
            assert all_tool_calls_traced
            policy RuntimeSafety {
                require no_unhandled_messages
                deny external_execution
                on violation { action review trace required }
            }
        "#;
        let program = parse_source(source).unwrap();
        check_program(&program).unwrap();
    }

    fn passport_messages(source: &str) -> Vec<String> {
        check_program(&parse_source(source).unwrap())
            .unwrap_err()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()
    }

    #[test]
    fn accepts_valid_passport() {
        let source = r#"
            module main
            agent ResearchAgent {}
            passport RiskAnalyzerPassport {
                agent ResearchAgent
                agent_name "Risk Analyzer"
                global_id "argx:agent:01HZX9"
                identity "did:argorix:risk-v1"
                provider "Argorix"
                version "1.0.0"
                country "CL"
                jurisdiction "CL"
                data_residency ["CL", "EU"]
                asn { registry "LACNIC" number "AS-PLACEHOLDER" holder "Argorix Labs" country "CL" }
                risk_level "high"
                intent "risk_analysis"
                attestations ["redteam"]
            }
        "#;
        check_program(&parse_source(source).unwrap()).unwrap();
    }

    #[test]
    fn rejects_invalid_passports() {
        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent A agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" country \"CL\" jurisdiction \"CL\" data_residency [\"CL\"] risk_level \"high\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("missing required field `version`")));

        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent Missing agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" version \"1\" country \"CL\" jurisdiction \"CL\" data_residency [\"CL\"] risk_level \"high\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("references unknown agent `Missing`")));

        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent A agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" version \"1\" country \"CL\" jurisdiction \"CL\" data_residency [\"CL\"] risk_level \"extreme\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("invalid risk_level `extreme`")));

        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent A agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" version \"1\" country \"Chile\" jurisdiction \"CL\" data_residency [\"CL\"] risk_level \"high\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("2-letter ISO-like code")));

        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent A agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" version \"1\" country \"CL\" jurisdiction \"CL\" data_residency [] risk_level \"high\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("non-empty `data_residency`")));

        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent A agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" version \"1\" country \"CL\" jurisdiction \"CL\" data_residency [\"CL\"] asn { registry \"FOO\" number \"AS-1\" holder \"h\" country \"CL\" } risk_level \"high\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("invalid asn registry `FOO`")));
    }

    #[test]
    fn rejects_duplicate_passport_names_and_per_agent() {
        let source = r#"
            module main
            agent A {}
            passport First { agent A agent_name "n" global_id "g" identity "i" provider "p" version "1" country "CL" jurisdiction "CL" data_residency ["CL"] risk_level "high" intent "x" }
            passport First { agent A agent_name "n" global_id "g" identity "i" provider "p" version "1" country "CL" jurisdiction "CL" data_residency ["CL"] risk_level "high" intent "x" }
        "#;
        let messages = passport_messages(source);
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate passport `First`")));
        assert!(messages
            .iter()
            .any(|message| message.contains("already has a passport")));
    }

    #[test]
    fn validates_typed_and_nominal_message_fields() {
        let valid = parse_source(
            "module main\nenum RiskLevel { low high }\ntype Message { content: string risk: RiskLevel }\n",
        )
        .unwrap();
        check_program(&valid).unwrap();

        let invalid =
            parse_source("module main\ntype Message { value: future value: string }\n").unwrap();
        let messages = check_program(&invalid)
            .unwrap_err()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>();
        assert!(messages
            .iter()
            .any(|message| message.contains("unknown message field type `future`")));
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate field `value`")));
    }

    fn harness_messages(body: &str) -> Vec<String> {
        let source = format!(
            "module main\nprovider OpenAI {{ kind external enabled false dry_run_only true requires feature_flag requires approval }}\ntype UserPrompt {{ content: string }}\ntype DraftAnswer {{ content: string }}\n{body}\n"
        );
        check_program(&parse_source(&source).unwrap())
            .unwrap_err()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()
    }

    #[test]
    fn accepts_valid_provider_harness_and_empty_attestations() {
        let source = r#"
            module main
            provider OpenAI {
                kind external
                enabled false
                dry_run_only true
                requires feature_flag
                requires approval
            }
            type UserPrompt { content: string }
            type DraftAnswer { content: string }
            harness OpenAIHarness {
                provider OpenAI
                mode dry_run
                network denied
                secrets denied
                filesystem none
                max_steps 10
                timeout_ms 1000
                input_contract UserPrompt
                output_contract DraftAnswer
                attestations []
            }
        "#;
        check_program(&parse_source(source).unwrap()).unwrap();
    }

    #[test]
    fn rejects_missing_and_unknown_harness_fields() {
        let missing = harness_messages("harness H {}");
        for field in ["provider", "mode", "network", "secrets", "filesystem"] {
            assert!(missing
                .iter()
                .any(|message| message.contains(&format!("missing required field `{field}`"))));
        }

        let invalid = harness_messages(
            "harness H { provider Missing mode live network allowed secrets env filesystem write }",
        );
        for expected in [
            "unknown provider `Missing`",
            "invalid mode `live`",
            "invalid network `allowed`",
            "invalid secrets `env`",
            "invalid filesystem `write`",
        ] {
            assert!(invalid.iter().any(|message| message.contains(expected)));
        }
    }

    #[test]
    fn rejects_duplicate_harness_limits_contracts_and_empty_entries() {
        let messages = harness_messages(
            r#"
            harness H {
                provider OpenAI
                mode simulated
                network denied
                secrets denied
                filesystem read_only
                max_steps 0
                timeout_ms 0
                input_contract MissingInput
                output_contract MissingOutput
                attestations [""]
            }
            harness H {
                provider OpenAI
                mode dry_run
                network denied
                secrets denied
                filesystem none
            }
            "#,
        );
        for expected in [
            "duplicate harness `H`",
            "max_steps must be positive",
            "timeout_ms must be positive",
            "unknown input_contract `MissingInput`",
            "unknown output_contract `MissingOutput`",
            "attestation entries must not be empty",
        ] {
            assert!(messages.iter().any(|message| message.contains(expected)));
        }
    }
}
