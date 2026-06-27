use argorix_ir::IrProgram;
use argorix_parser::{
    ast::{Approval, CapabilityLevel},
    parse_source,
};
use argorix_semantics::{check_program, check_program_with_options, CheckOptions};

const VALID_V02: &str = include_str!("../examples/prompt_defense_v02.argx");
const LEGACY_V01: &str = include_str!("../examples/prompt_defense.argx");

fn check(source: &str) -> Result<(), Vec<argorix_parser::Diagnostic>> {
    let ast = parse_source(source).expect("test source should parse");
    check_program(&ast)
}

#[test]
fn parses_capability_declarations() {
    let ast = parse_source(
        "module Example\ncapability shell.execute { level dangerous requires approval }\n",
    )
    .unwrap();

    assert_eq!(ast.capabilities.len(), 1);
    assert_eq!(ast.capabilities[0].name.value, "shell.execute");
    assert_eq!(ast.capabilities[0].level.value, CapabilityLevel::Dangerous);
    assert!(ast.capabilities[0].requires_approval);
}

#[test]
fn parses_security_blocks() {
    let ast = parse_source(
        "module Example\ntype Job { value: string }\nagent Runner { security { approval granted } receives Job }\n",
    )
    .unwrap();

    assert_eq!(
        ast.agents[0].approval.as_ref().map(|value| value.value),
        Some(Approval::Granted)
    );
}

#[test]
fn allows_safe_capability_without_approval() {
    let source = r#"
        module Example
        capability regex.match { level safe }
        type Prompt { content: string }
        agent Scanner { receives Prompt capabilities { regex.match } }
    "#;
    check(source).expect("safe capability should not require approval");
}

#[test]
fn rejects_restricted_capability_without_approval() {
    let source = include_str!("../examples/restricted_without_approval.argx");
    let diagnostics = check(source).unwrap_err();

    assert!(diagnostics[0]
        .message
        .contains("uses restricted capability runtime.halt without approval"));
    assert_eq!(diagnostics[0].span.line, 16);
}

#[test]
fn rejects_dangerous_capability_without_approval() {
    let source = r#"
        module Example
        capability shell.execute { level dangerous requires approval }
        type Job { command: string }
        agent Runner { receives Job capabilities { shell.execute } }
    "#;
    let diagnostics = check(source).unwrap_err();

    assert!(diagnostics[0]
        .message
        .contains("uses dangerous capability shell.execute without approval"));
}

#[test]
fn rejects_unknown_capability() {
    let source = include_str!("../examples/unknown_capability.argx");
    let diagnostics = check(source).unwrap_err();

    assert!(diagnostics[0]
        .message
        .contains("Unknown capability regex.match used by agent PromptScanner"));
}

#[test]
fn validates_protocol_backed_by_sends_and_receives() {
    let ast = parse_source(VALID_V02).expect("v0.2 example should parse");
    check_program(&ast).expect("protocol contracts should match");
}

#[test]
fn rejects_protocol_without_corresponding_send() {
    let source = r#"
        module Example
        type Ping { value: string }
        agent Sender {}
        agent Receiver { receives Ping from Sender }
        protocol Flow { Sender -> Receiver: tell Ping }
    "#;
    let diagnostics = check(source).unwrap_err();

    assert!(diagnostics
        .iter()
        .any(|item| item.message.contains("to declare `sends Ping to Receiver`")));
}

#[test]
fn rejects_protocol_without_corresponding_receive() {
    let source = r#"
        module Example
        type Ping { value: string }
        agent Sender { sends Ping to Receiver }
        agent Receiver {}
        protocol Flow { Sender -> Receiver: tell Ping }
    "#;
    let diagnostics = check(source).unwrap_err();

    assert!(diagnostics.iter().any(|item| item
        .message
        .contains("to declare `receives Ping from Sender`")));
}

#[test]
fn emits_versioned_v02_ir_with_capabilities() {
    let ast = parse_source(VALID_V02).unwrap();
    check_program(&ast).unwrap();
    let json = serde_json::to_value(IrProgram::from(&ast)).unwrap();

    assert_eq!(json["ir_version"], "0.36");
    assert_eq!(json["language"], "Argorix Lang");
    assert_eq!(json["capabilities"][3]["name"], "runtime.halt");
    assert_eq!(json["capabilities"][3]["requires_approval"], true);
    assert_eq!(json["agents"][2]["approval"], "granted");
}

#[test]
fn parses_handler_instructions() {
    use argorix_parser::ast::HandlerInstruction;
    let source = r#"
        module Handlers
        type Input { value: string }
        type Output { value: string }
        agent Worker {
            receives Input
            sends Output to Sink
            capabilities { runtime.halt }
            on Input as input {
                emit Output to Sink
                trace input
                halt
            }
        }
        agent Sink { receives Output from Worker }
    "#;
    let ast = parse_source(source).unwrap();
    let instructions = &ast.agents[0].handlers[0].instructions;
    assert!(matches!(instructions[0], HandlerInstruction::Emit { .. }));
    assert!(matches!(instructions[1], HandlerInstruction::Trace { .. }));
    assert!(matches!(instructions[2], HandlerInstruction::Halt { .. }));
}

#[test]
fn accepts_valid_handlers_and_emits_ir_handlers() {
    let source = include_str!("../examples/prompt_defense_v05.argx");
    let ast = parse_source(source).unwrap();
    check_program(&ast).unwrap();
    let json = serde_json::to_value(IrProgram::from(&ast)).unwrap();
    assert_eq!(
        json["agents"][0]["handlers"][0]["message_type"],
        "UserPrompt"
    );
    assert_eq!(
        json["agents"][0]["handlers"][0]["instructions"][0]["op"],
        "emit"
    );
}

#[test]
fn rejects_handler_without_receives() {
    let source = include_str!("../examples/handler_without_receives.argx");
    let diagnostics = check(source).unwrap_err();
    assert!(diagnostics
        .iter()
        .any(|item| item.message.contains("requires `receives UserPrompt`")));
}

#[test]
fn rejects_emit_without_sends() {
    let source = include_str!("../examples/emit_without_sends.argx");
    let diagnostics = check(source).unwrap_err();
    assert!(diagnostics.iter().any(|item| item
        .message
        .contains("requires `sends Finding to PolicyJudge`")));
}

#[test]
fn rejects_halt_without_runtime_halt_capability() {
    let source = include_str!("../examples/halt_without_capability.argx");
    let diagnostics = check(source).unwrap_err();
    assert!(diagnostics
        .iter()
        .any(|item| item.message.contains("without capability `runtime.halt`")));
}

#[test]
fn parses_facu_and_marron_intrinsics() {
    use argorix_parser::ast::HandlerInstruction;
    let source = include_str!("../examples/prompt_defense_v06.argx");
    let ast = parse_source(source).unwrap();
    let prompt = &ast.agents[0].handlers[0].instructions[0];
    let guard = &ast.agents[1].handlers[0].instructions[0];
    assert!(matches!(
        prompt,
        HandlerInstruction::IntrinsicCall { name, argument }
            if name.value == "facu" && argument.value == "prompt"
    ));
    assert!(matches!(
        guard,
        HandlerInstruction::IntrinsicCall { name, argument }
            if name.value == "marron" && argument.value == "finding"
    ));
}

#[test]
fn validates_intrinsic_capabilities_and_bindings() {
    check(include_str!("../examples/prompt_defense_v06.argx")).unwrap();
    for (source, expected) in [
        (
            include_str!("../examples/facu_without_state_write.argx"),
            "without capability `state.write`",
        ),
        (
            include_str!("../examples/marron_without_runtime_guard.argx"),
            "without capability `runtime.guard`",
        ),
        (
            include_str!("../examples/intrinsic_wrong_binding.argx"),
            "does not match handler binding",
        ),
        (
            include_str!("../examples/unknown_intrinsic.argx"),
            "unknown runtime intrinsic `blade`",
        ),
    ] {
        let diagnostics = check(source).unwrap_err();
        assert!(diagnostics
            .iter()
            .any(|item| item.message.contains(expected)));
    }
}

#[test]
fn ir_includes_intrinsic_instructions() {
    let ast = parse_source(include_str!("../examples/prompt_defense_v06.argx")).unwrap();
    check_program(&ast).unwrap();
    let json = serde_json::to_value(IrProgram::from(&ast)).unwrap();
    assert_eq!(json["ir_version"], "0.36");
    assert_eq!(
        json["agents"][0]["handlers"][0]["instructions"][0]["op"],
        "intrinsic"
    );
    assert_eq!(
        json["agents"][0]["handlers"][0]["instructions"][0]["name"],
        "facu"
    );
}

#[test]
fn parses_tools_permissions_and_calls() {
    use argorix_parser::ast::HandlerInstruction;
    let ast = parse_source(include_str!("../examples/tool_call_v07.argx")).unwrap();
    assert_eq!(ast.tools[0].name.value, "WebSearch");
    assert_eq!(ast.agents[0].tools[0].value, "WebSearch");
    assert!(matches!(
        ast.agents[0].handlers[0].instructions[2],
        HandlerInstruction::CallTool { ref tool, ref binding }
            if tool.value == "WebSearch" && binding.value == "prompt"
    ));
}

#[test]
fn parser_preserves_optional_tool_provider() {
    let explicit = parse_source(
        "module P\ncapability web.search { level safe }\ntype I { value: string }\ntype O { value: string }\ntool T { provider simulated capability web.search input I output O }\n",
    )
    .unwrap();
    assert_eq!(
        explicit.tools[0]
            .provider
            .as_ref()
            .map(|provider| provider.value.as_str()),
        Some("simulated")
    );

    let omitted = parse_source(
        "module P\ncapability web.search { level safe }\ntype I { value: string }\ntype O { value: string }\ntool T { capability web.search input I output O }\n",
    )
    .unwrap();
    assert!(omitted.tools[0].provider.is_none());
}

#[test]
fn semantic_checker_resolves_and_validates_tool_providers() {
    let omitted = "module P\ncapability web.search { level safe }\ntype I { value: string }\ntype O { value: string }\ntool T { capability web.search input I output O }\n";
    let explicit = "module P\ncapability web.search { level safe }\ntype I { value: string }\ntype O { value: string }\ntool T { provider simulated capability web.search input I output O }\n";
    check(omitted).unwrap();
    check(explicit).unwrap();

    let diagnostics = check(include_str!("../examples/tool_invalid_provider.argx")).unwrap_err();
    assert!(diagnostics
        .iter()
        .any(|item| item.message.contains("unsupported provider `real_web`")));
}

#[test]
fn semantic_checker_rejects_v010_invalid_model_provider() {
    let diagnostics =
        check(include_str!("../examples/model_invalid_provider_v010.argx")).unwrap_err();
    assert!(diagnostics
        .iter()
        .any(|item| item.message.contains("unsupported provider `external_llm`")));
}

#[test]
fn parser_recognizes_external_provider_contract_fields() {
    let ast = parse_source(
        r#"
        module Contracts
        provider OpenAI {
            kind external
            enabled false
            dry_run_only true
            requires feature_flag
            requires approval
        }
        "#,
    )
    .unwrap();

    let provider = &ast.providers[0];
    assert_eq!(provider.name.value, "OpenAI");
    assert_eq!(provider.kind.value.as_str(), "external");
    assert!(!provider.enabled.value);
    assert!(provider.dry_run_only.value);
    assert!(provider.requires_feature_flag);
    assert!(provider.requires_explicit_approval);
}

#[test]
fn parser_recognizes_provider_allowlists_in_either_order() {
    for blocks in [
        "allowed_targets { GuardModel WebSearch }\nallowed_capabilities { model.invoke web.search }",
        "allowed_capabilities { model.invoke web.search }\nallowed_targets { GuardModel WebSearch }",
    ] {
        let source = format!(
            "module Contracts\nprovider OpenAI {{ kind external enabled false dry_run_only true requires feature_flag requires approval {blocks} }}\n"
        );
        let ast = parse_source(&source).unwrap();
        let provider = &ast.providers[0];
        assert_eq!(provider.allowed_targets.len(), 2);
        assert_eq!(provider.allowed_capabilities.len(), 2);
    }
}

#[test]
fn parser_accepts_v011_contract_without_allowlists() {
    let ast = parse_source(
        "module Contracts\nprovider OpenAI { kind external enabled false dry_run_only true requires feature_flag requires approval }\n",
    )
    .unwrap();
    assert!(ast.providers[0].allowed_targets.is_empty());
    assert!(ast.providers[0].allowed_capabilities.is_empty());
}

#[test]
fn parser_rejects_duplicate_allowlist_blocks() {
    for block in ["allowed_targets", "allowed_capabilities"] {
        let source = format!(
            "module Contracts\nprovider OpenAI {{ kind external enabled false dry_run_only true requires feature_flag requires approval {block} {{ One }} {block} {{ Two }} }}\n"
        );
        let diagnostics = parse_source(&source).unwrap_err();
        assert!(diagnostics[0]
            .message
            .contains(&format!("duplicate `{block}` block")));
    }
}

#[test]
fn parser_rejects_reversed_provider_requirements() {
    let diagnostics = parse_source(
        "module Contracts\nprovider OpenAI { kind external enabled false dry_run_only true requires approval requires feature_flag }\n",
    )
    .unwrap_err();
    assert!(diagnostics[0]
        .message
        .contains("must appear before `requires approval`"));
}
#[test]
fn semantic_checker_accepts_disabled_external_provider_contract() {
    check(
        r#"
        module Contracts
        provider OpenAI {
            kind external
            enabled false
            dry_run_only true
            requires feature_flag
            requires approval
        }
        "#,
    )
    .unwrap();
}

#[test]
fn semantic_checker_rejects_invalid_external_provider_contracts() {
    let cases = [
        (
            "provider OpenAI { kind external enabled true dry_run_only true requires feature_flag requires approval }",
            "must be disabled",
        ),
        (
            "provider OpenAI { kind external enabled false dry_run_only false requires feature_flag requires approval }",
            "dry_run_only true",
        ),
        (
            "provider OpenAI { kind external enabled false dry_run_only true requires approval }",
            "requires feature_flag",
        ),
        (
            "provider OpenAI { kind external enabled false dry_run_only true requires feature_flag }",
            "requires approval",
        ),
        (
            "provider simulated { kind external enabled false dry_run_only true requires feature_flag requires approval }",
            "reserved executable provider",
        ),
    ];

    for (declaration, expected) in cases {
        let diagnostics = check(&format!("module Contracts\n{declaration}\n")).unwrap_err();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(expected)),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn semantic_checker_rejects_duplicate_provider_contracts() {
    let declaration = "provider OpenAI { kind external enabled false dry_run_only true requires feature_flag requires approval }";
    let diagnostics =
        check(&format!("module Contracts\n{declaration}\n{declaration}\n")).unwrap_err();
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("duplicate provider `OpenAI`")));
}

#[test]
fn semantic_checker_rejects_external_contract_used_by_model_or_tool() {
    for (target, declaration) in [
        (
            "M",
            "model M { provider OpenAI capability model.invoke input I output O }",
        ),
        (
            "T",
            "tool T { provider OpenAI capability model.invoke input I output O }",
        ),
    ] {
        let provider = format!("provider OpenAI {{ kind external enabled false dry_run_only true requires feature_flag requires approval allowed_targets {{ {target} }} allowed_capabilities {{ model.invoke }} }}");
        let source = format!(
            "module Contracts\n{provider}\ncapability model.invoke {{ level safe }}\ntype I {{ value: string }}\ntype O {{ value: string }}\n{declaration}\n"
        );
        let diagnostics = check(&source).unwrap_err();
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("only `simulated` is allowed")));
    }
}

fn allowlist_program(targets: &str, capabilities: &str, declarations: &str) -> String {
    format!(
        r#"
        module Allowlists
        provider OpenAI {{
            kind external
            enabled false
            dry_run_only true
            requires feature_flag
            requires approval
            allowed_targets {{ {targets} }}
            allowed_capabilities {{ {capabilities} }}
        }}
        capability model.invoke {{ level safe }}
        capability web.search {{ level safe }}
        type Input {{ value: string }}
        type Output {{ value: string }}
        {declarations}
        "#
    )
}

#[test]
fn semantic_checker_accepts_valid_model_and_tool_allowlists() {
    check(&allowlist_program(
        "GuardModel WebSearch",
        "model.invoke web.search",
        "model GuardModel { provider simulated capability model.invoke input Input output Output }\n\
         tool WebSearch { provider simulated capability web.search input Input output Output }",
    ))
    .unwrap();
}

#[test]
fn semantic_checker_rejects_invalid_allowlist_entries() {
    let cases = [
        (allowlist_program("Missing", "model.invoke", ""), "unknown allowlist target `Missing`"),
        (allowlist_program("", "missing.capability", ""), "unknown allowlist capability `missing.capability`"),
        (allowlist_program("GuardModel GuardModel", "model.invoke", "model GuardModel { provider simulated capability model.invoke input Input output Output }"), "duplicate allowed target `GuardModel`"),
        (allowlist_program("", "model.invoke model.invoke", ""), "duplicate allowed capability `model.invoke`"),
        (allowlist_program("Shared", "model.invoke web.search", "model Shared { provider simulated capability model.invoke input Input output Output }\ntool Shared { provider simulated capability web.search input Input output Output }"), "ambiguous allowlist target `Shared`"),
    ];
    for (source, expected) in cases {
        let diagnostics = check(&source).unwrap_err();
        assert!(
            diagnostics
                .iter()
                .any(|item| item.message.contains(expected)),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn semantic_checker_rejects_incompatible_allowlist_capabilities() {
    for (target, allowed, declaration, expected) in [
        ("GuardModel", "web.search", "model GuardModel { provider simulated capability model.invoke input Input output Output }", "target `GuardModel` requires capability `model.invoke`"),
        ("WebSearch", "model.invoke", "tool WebSearch { provider simulated capability web.search input Input output Output }", "target `WebSearch` requires capability `web.search`"),
    ] {
        let diagnostics = check(&allowlist_program(target, allowed, declaration)).unwrap_err();
        assert!(diagnostics.iter().any(|item| item.message.contains(expected)), "{diagnostics:?}");
    }
}
#[test]
fn ir_preserves_populated_provider_allowlists() {
    let ast = parse_source(&allowlist_program(
        "GuardModel WebSearch",
        "model.invoke web.search",
        "model GuardModel { provider simulated capability model.invoke input Input output Output }\n\
         tool WebSearch { provider simulated capability web.search input Input output Output }\n\
         agent Worker { receives Input }\n\
         protocol Flow { User -> Worker: tell Input }",
    ))
    .unwrap();
    check_program(&ast).unwrap();
    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(
        ir.providers[0].allowed_targets,
        vec!["GuardModel", "WebSearch"]
    );
    assert_eq!(
        ir.providers[0].allowed_capabilities,
        vec!["model.invoke", "web.search"]
    );
    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(
        bytecode.providers[0].allowed_targets,
        vec!["GuardModel", "WebSearch"]
    );
    assert_eq!(
        bytecode.providers[0].allowed_capabilities,
        vec!["model.invoke", "web.search"]
    );
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();
}
#[test]
fn ir_and_bytecode_include_declarative_provider_contracts() {
    let source = r#"
        module Contracts
        provider OpenAI {
            kind external
            enabled false
            dry_run_only true
            requires feature_flag
            requires approval
        }
        type Ping { value: string }
        agent Worker { receives Ping }
        protocol Flow { User -> Worker: tell Ping }
    "#;
    let ast = parse_source(source).unwrap();
    check_program(&ast).unwrap();
    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(ir.providers[0].name, "OpenAI");
    assert_eq!(ir.providers[0].kind, "external");
    assert!(ir.providers[0].allowed_targets.is_empty());
    assert!(ir.providers[0].allowed_capabilities.is_empty());

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(bytecode.providers[0].name, "OpenAI");
    assert!(bytecode.providers[0].allowed_targets.is_empty());
    assert!(bytecode.providers[0].allowed_capabilities.is_empty());
    assert!(matches!(
        bytecode.instructions.first(),
        Some(argorix_bytecode::Instruction::DeclareProviderContract {
            name,
            kind,
            enabled: false,
            dry_run_only: true,
            requires_feature_flag: true,
            requires_explicit_approval: true,
            allowed_targets,
            allowed_capabilities,
        }) if name == "OpenAI"
            && kind == "external"
            && allowed_targets.is_empty()
            && allowed_capabilities.is_empty()
    ));
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();
}

#[test]
fn ir_and_bytecode_make_tool_provider_explicit() {
    let ast = parse_source(include_str!("../examples/tool_call_v07.argx")).unwrap();
    check_program(&ast).unwrap();
    assert!(ast.tools[0].provider.is_none());

    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(ir.tools[0].provider, "simulated");
    assert_eq!(ir.models.len(), 0);

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(bytecode.tools[0].provider, "simulated");
    assert!(bytecode.instructions.iter().any(|instruction| matches!(
        instruction,
        argorix_bytecode::Instruction::DeclareTool { provider, .. }
            if provider == "simulated"
    )));
}

#[test]
fn ir_and_bytecode_preserve_model_provider() {
    let ast = parse_source(include_str!("../examples/model_call_v08.argx")).unwrap();
    check_program(&ast).unwrap();
    let ir = IrProgram::from(&ast);
    assert_eq!(ir.models[0].provider, "simulated");
    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert!(bytecode.instructions.iter().any(|instruction| matches!(
        instruction,
        argorix_bytecode::Instruction::DeclareModel { provider, .. }
            if provider == "simulated"
    )));
}

#[test]
fn validates_tool_contracts_and_permissions() {
    check(include_str!("../examples/tool_call_v07.argx")).unwrap();
    for (source, expected) in [
        (
            include_str!("../examples/tool_unknown.argx"),
            "unknown tool `MissingTool`",
        ),
        (
            include_str!("../examples/tool_without_agent_permission.argx"),
            "without declaring it in `tools`",
        ),
        (
            include_str!("../examples/tool_missing_capability.argx"),
            "without capability `web.search`",
        ),
        (
            include_str!("../examples/tool_restricted_without_approval.argx"),
            "without approval",
        ),
        (
            include_str!("../examples/tool_wrong_binding.argx"),
            "does not match handler binding",
        ),
    ] {
        let diagnostics = check(source).unwrap_err();
        assert!(diagnostics
            .iter()
            .any(|item| item.message.contains(expected)));
    }
}

#[test]
fn rejects_invalid_global_tool_declarations() {
    let duplicate = r#"
        module Invalid
        capability web.search { level safe }
        type Input { value: string }
        type Output { value: string }
        tool Search { capability web.search input Input output Output }
        tool Search { capability web.search input Input output Output }
    "#;
    assert!(check(duplicate)
        .unwrap_err()
        .iter()
        .any(|item| item.message.contains("duplicate tool")));

    let bad_contract = r#"
        module Invalid
        tool Search { capability missing.cap input MissingInput output MissingOutput }
    "#;
    let diagnostics = check(bad_contract).unwrap_err();
    assert!(diagnostics
        .iter()
        .any(|item| item.message.contains("unknown capability")));
    assert!(
        diagnostics
            .iter()
            .filter(|item| item.message.contains("unknown message type"))
            .count()
            >= 2
    );

    let unknown_agent_tool = r#"
        module Invalid
        type Input { value: string }
        agent Worker {
            receives Input
            tools { MissingTool }
        }
    "#;
    assert!(check(unknown_agent_tool)
        .unwrap_err()
        .iter()
        .any(|item| item
            .message
            .contains("references unknown tool `MissingTool`")));
}

#[test]
fn ir_includes_tools_and_call_instruction() {
    let ast = parse_source(include_str!("../examples/tool_call_v07.argx")).unwrap();
    check_program(&ast).unwrap();
    let json = serde_json::to_value(IrProgram::from(&ast)).unwrap();
    assert_eq!(json["ir_version"], "0.36");
    assert_eq!(json["tools"][0]["name"], "WebSearch");
    assert_eq!(json["agents"][0]["tools"][0], "WebSearch");
    assert_eq!(
        json["agents"][0]["handlers"][0]["instructions"][2]["op"],
        "call"
    );
}

#[test]
fn parses_models_permissions_and_ask() {
    use argorix_parser::ast::HandlerInstruction;
    let ast = parse_source(include_str!("../examples/model_call_v08.argx")).unwrap();
    assert_eq!(ast.models[0].provider.value, "simulated");
    assert_eq!(ast.agents[1].models[0].value, "GuardModel");
    assert!(matches!(
        ast.agents[1].handlers[0].instructions[2],
        HandlerInstruction::AskModel { ref model, ref binding }
            if model.value == "GuardModel" && binding.value == "result"
    ));
}

#[test]
fn validates_model_contracts_and_permissions() {
    check(include_str!("../examples/model_call_v08.argx")).unwrap();
    for (source, expected) in [
        (
            include_str!("../examples/model_unknown.argx"),
            "unknown model",
        ),
        (
            include_str!("../examples/model_without_agent_permission.argx"),
            "without declaring it in `models`",
        ),
        (
            include_str!("../examples/model_missing_capability.argx"),
            "without capability `model.invoke`",
        ),
        (
            include_str!("../examples/model_restricted_without_approval.argx"),
            "without approval",
        ),
        (
            include_str!("../examples/model_wrong_binding.argx"),
            "does not match handler binding",
        ),
        (
            include_str!("../examples/model_invalid_provider.argx"),
            "unsupported provider",
        ),
    ] {
        assert!(check(source)
            .unwrap_err()
            .iter()
            .any(|item| item.message.contains(expected)));
    }
}

#[test]
fn rejects_duplicate_and_invalid_model_contracts() {
    let source = r#"
        module Invalid
        model M { provider simulated capability missing input Missing output Missing }
        model M { provider simulated capability missing input Missing output Missing }
    "#;
    let diagnostics = check(source).unwrap_err();
    assert!(diagnostics
        .iter()
        .any(|item| item.message.contains("duplicate model")));
    assert!(diagnostics
        .iter()
        .any(|item| item.message.contains("unknown capability")));
    assert!(diagnostics
        .iter()
        .any(|item| item.message.contains("unknown message type")));
}

#[test]
fn ir_includes_models_and_ask() {
    let ast = parse_source(include_str!("../examples/model_call_v08.argx")).unwrap();
    check_program(&ast).unwrap();
    let json = serde_json::to_value(IrProgram::from(&ast)).unwrap();
    assert_eq!(json["ir_version"], "0.36");
    assert_eq!(json["models"][0]["name"], "GuardModel");
    assert_eq!(json["agents"][1]["models"][0], "GuardModel");
    assert_eq!(
        json["agents"][1]["handlers"][0]["instructions"][2]["op"],
        "ask"
    );
}

#[test]
fn legacy_mode_accepts_registry_free_v01_capabilities() {
    let ast = parse_source(LEGACY_V01).unwrap();
    check_program_with_options(
        &ast,
        CheckOptions {
            allow_legacy_capabilities: true,
        },
    )
    .expect("explicit legacy mode should preserve v0.1 sources");
}

#[test]
fn reports_precise_semantic_locations() {
    let source = "module Broken\nagent A { sends Missing to Nobody }\n";
    let ast = parse_source(source).expect("syntax should be valid");
    let diagnostics = check_program(&ast).expect_err("semantics should fail");

    assert_eq!(diagnostics[0].span.line, 2);
    assert_eq!(diagnostics[0].span.column, 17);
    assert!(diagnostics[0].message.contains("unknown message type"));
}

#[test]
fn parses_and_emits_v09_policies() {
    let ast = parse_source(include_str!("../examples/policy_assertions_v09.argx")).unwrap();
    check_program(&ast).unwrap();

    assert_eq!(ast.assertions.len(), 6);
    assert_eq!(ast.failures.len(), 3);
    assert_eq!(
        ast.assertions[5]
            .argument
            .as_ref()
            .map(|value| value.value.as_str()),
        Some("completed")
    );
    assert_eq!(ast.failures[0].action.value, "block");
    assert!(ast.failures[0].trace_required);

    let json = serde_json::to_value(IrProgram::from(&ast)).unwrap();
    assert_eq!(json["ir_version"], "0.36");
    assert_eq!(json["assertions"][0]["name"], "no_unhandled_messages");
    assert_eq!(json["failures"][0]["trace"], "required");
}

#[test]
fn rejects_invalid_v09_policy_declarations() {
    for (source, expected) in [
        (
            include_str!("../examples/assert_unknown.argx"),
            "unknown policy assertion",
        ),
        (
            include_str!("../examples/failure_invalid_action.argx"),
            "invalid failure action",
        ),
        (
            include_str!("../examples/failure_missing_trace.argx"),
            "requires `trace required`",
        ),
    ] {
        assert!(check(source)
            .unwrap_err()
            .iter()
            .any(|item| item.message.contains(expected)));
    }
}

#[test]
fn accepts_comments_and_external_participants() {
    let source = r#"
        // External callers are valid protocol participants.
        module Comments
        type Ping { value: int }
        agent Worker { receives Ping from System }
        protocol Run { System -> Worker: delegate Ping }
    "#;
    check(source).expect("source should be semantically valid");
}

#[test]
fn v016_fixture_remains_verifiable_after_v017_default_emission() {
    let source = include_str!("../examples/provider_allowlists_v016.argx");
    let ast = parse_source(source).unwrap();
    check_program(&ast).unwrap();
    let emitted = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let fixture: argorix_bytecode::BytecodeProgram = serde_json::from_str(include_str!(
        "../examples/provider_allowlists_v016.argbc.json"
    ))
    .unwrap();

    assert_eq!(emitted.bytecode_version, "0.36");
    assert_eq!(fixture.bytecode_version, "0.16");
    argorix_bytecode::verify_bytecode(&fixture).unwrap();
}

#[test]
fn v017_policy_fixture_remains_verifiable_after_v018_default_emission() {
    let source = include_str!("../examples/policy_v017.argx");
    let ast = parse_source(source).unwrap();
    check_program(&ast).unwrap();
    let emitted = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let fixture: argorix_bytecode::BytecodeProgram =
        serde_json::from_str(include_str!("../examples/policy_v017.argbc.json")).unwrap();

    assert_eq!(emitted.bytecode_version, "0.36");
    assert_eq!(fixture.bytecode_version, "0.17");
    argorix_bytecode::verify_bytecode(&fixture).unwrap();
    assert_eq!(emitted.policies.len(), 2);
}

#[test]
fn v018_typed_message_fixture_remains_verifiable_after_v019_default_emission() {
    let ast = parse_source(include_str!("../examples/typed_messages_v018.argx")).unwrap();
    check_program(&ast).unwrap();
    let emitted = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let fixture: argorix_bytecode::BytecodeProgram =
        serde_json::from_str(include_str!("../examples/typed_messages_v018.argbc.json")).unwrap();
    assert_eq!(emitted.bytecode_version, "0.36");
    assert_eq!(fixture.bytecode_version, "0.18");
    argorix_bytecode::verify_bytecode(&fixture).unwrap();
    assert_eq!(emitted.types[0].fields[0].field_type, "string");
}

#[test]
fn v019_passport_fixture_remains_verifiable_after_v020_default_emission() {
    let ast = parse_source(include_str!("../examples/agent_passport_v019.argx")).unwrap();
    check_program(&ast).unwrap();
    let emitted = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let fixture: argorix_bytecode::BytecodeProgram =
        serde_json::from_str(include_str!("../examples/agent_passport_v019.argbc.json")).unwrap();
    assert_eq!(emitted.bytecode_version, "0.36");
    assert_eq!(fixture.bytecode_version, "0.19");
    argorix_bytecode::verify_bytecode(&fixture).unwrap();
    assert_eq!(emitted.passports[0].agent, "ResearchAgent");
    assert_eq!(emitted.passports[0].risk_level, "high");
}

#[test]
fn v021_feature_secret_fixture_matches_fresh_output() {
    let ast = parse_source(include_str!(
        "../examples/feature_secret_boundary_v021.argx"
    ))
    .unwrap();
    check_program(&ast).unwrap();
    let emitted = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let mut fixture: argorix_bytecode::BytecodeProgram = serde_json::from_str(include_str!(
        "../examples/feature_secret_boundary_v021.argbc.json"
    ))
    .unwrap();
    assert_eq!(fixture.bytecode_version, "0.31");
    fixture.bytecode_version = emitted.bytecode_version.clone();
    assert_eq!(emitted, fixture);
    assert_eq!(emitted.bytecode_version, "0.36");
    assert_eq!(emitted.provider_harnesses[0].name, "OpenAIHarness");
    assert_eq!(
        emitted.provider_harnesses[0].feature.as_deref(),
        Some("OpenAIAdapter")
    );
    assert_eq!(
        emitted.provider_harnesses[0].secret.as_deref(),
        Some("OpenAISecret")
    );
    assert_eq!(emitted.features[0].name, "OpenAIAdapter");
    assert_eq!(emitted.secrets[0].name, "OpenAISecret");
}

#[test]
fn v020_provider_harness_fixture_remains_verifiable() {
    let fixture: argorix_bytecode::BytecodeProgram =
        serde_json::from_str(include_str!("../examples/provider_harness_v020.argbc.json")).unwrap();
    assert_eq!(fixture.bytecode_version, "0.20");
    argorix_bytecode::verify_bytecode(&fixture).unwrap();
    assert_eq!(fixture.provider_harnesses[0].name, "OpenAIHarness");
}

#[test]
fn v021_vm_preserves_feature_and_secret_boundary_evidence() {
    use argorix_vm::{EventType, InjectedMessage, Vm};

    let ast = parse_source(include_str!(
        "../examples/feature_secret_boundary_v021.argx"
    ))
    .unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));

    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        InjectedMessage {
            from: "User".into(),
            to: "ResearchAgent".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );
    let trace = outcome
        .result
        .as_ref()
        .expect("v0.21 program runs to a trace");

    // Trace preserves feature and secret metadata.
    assert_eq!(trace.features[0].name, "OpenAIAdapter");
    assert_eq!(trace.secrets[0].name, "OpenAISecret");
    assert_eq!(trace.vm_version, "0.36");

    // Ledger records governance-only events, never real secret reads.
    let events = &outcome.state.trace_ledger.events;
    let has = |kind: EventType| events.iter().any(|event| event.event_type == kind);
    assert!(has(EventType::FeatureDeclared));
    assert!(has(EventType::FeatureValidated));
    assert!(has(EventType::SecretBoundaryDeclared));
    assert!(has(EventType::SecretBoundaryValidated));
    assert!(has(EventType::SecretAccessDenied));

    // SecurityReport summarizes the boundary without exposing secret material.
    let report = argorix_vm::SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.report_version, "0.36");
    assert_eq!(report.feature_flags.total, 1);
    assert_eq!(report.feature_flags.requires_approval, 1);
    assert_eq!(report.feature_flags.linked_providers, vec!["OpenAI"]);
    assert_eq!(report.secret_boundaries.total, 1);
    assert!(!report.secret_boundaries.values_present);
    assert_eq!(report.secret_boundaries.required_by, vec!["OpenAIAdapter"]);

    // The declared boundary never leaks the secret value anywhere in the report.
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("sk-"));
    assert!(serialized.contains("\"values_present\":false"));

    // External providers remain non-executable; only `simulated` ran.
    assert!(outcome
        .state
        .provider_calls
        .iter()
        .all(|call| call.provider == "simulated"));
}

#[test]
fn v029_atrust_handshake_pipeline_and_fixture() {
    let ast = parse_source(include_str!("../examples/atrust_handshake_v029.argx")).unwrap();
    check_program(&ast).unwrap();

    // IR carries the dry-run handshake metadata at version 0.29.
    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(ir.atrust_handshakes.len(), 1);
    let hs = &ir.atrust_handshakes[0];
    assert_eq!(hs.name, "ResearchHandshake");
    assert_eq!(hs.initiator, "ResearchAgent");
    assert_eq!(hs.responder, "VerifierAgent");
    assert_eq!(hs.mode, "dry_run");
    assert_eq!(hs.network, "denied");
    assert_eq!(hs.execution, "disabled");
    assert_eq!(hs.security_claims, "none");
    assert_eq!(hs.credential_contracts, vec!["ResearchCredential"]);

    // Bytecode preserves the metadata and verifies.
    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(bytecode.atrust_handshakes.len(), 1);
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();

    // The committed fixture matches fresh emission exactly.
    let mut fixture: argorix_bytecode::BytecodeProgram =
        serde_json::from_str(include_str!("../examples/atrust_handshake_v029.argbc.json")).unwrap();
    assert_eq!(fixture.bytecode_version, "0.31");
    fixture.bytecode_version = bytecode.bytecode_version.clone();
    assert_eq!(bytecode, fixture);

    // No runtime handshake/challenge/network instructions are emitted: handshakes
    // are governance metadata, not executable trust exchange.
    let json = serde_json::to_value(&bytecode).unwrap();
    let instrs = serde_json::to_string(&json["instructions"]).unwrap();
    for forbidden in [
        "RunHandshake",
        "HandshakeInit",
        "HandshakeAck",
        "GenerateChallenge",
        "SignChallenge",
        "VerifyResponse",
        "ResolveDid",
        "NetworkRequest",
    ] {
        assert!(
            !instrs.contains(forbidden),
            "unexpected instruction {forbidden}"
        );
    }
}

#[test]
fn v029_verifier_accepts_v028_and_v029_bytecode() {
    // Fresh 0.29 emission verifies.
    let ast = parse_source(include_str!("../examples/atrust_handshake_v029.argx")).unwrap();
    check_program(&ast).unwrap();
    let v029 = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    assert_eq!(v029.bytecode_version, "0.36");
    argorix_bytecode::verify_bytecode(&v029).unwrap();

    // A 0.28 fixture remains verifiable (backward compatibility preserved).
    let v028: argorix_bytecode::BytecodeProgram = serde_json::from_str(include_str!(
        "../examples/feature_secret_boundary_v021.argbc.json"
    ))
    .unwrap();
    // The committed fixture currently tracks 0.29; force the legacy version to
    // assert the verifier still accepts 0.28 bytecode.
    let mut legacy = v028;
    legacy.bytecode_version = "0.28".into();
    argorix_bytecode::verify_bytecode(&legacy).unwrap();
}

#[test]
fn v029_security_report_covers_handshakes_without_claims() {
    use argorix_vm::{SecurityReport, Vm};

    let ast = parse_source(include_str!("../examples/atrust_handshake_v029.argx")).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));

    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        argorix_vm::InjectedMessage {
            from: "User".into(),
            to: "ResearchAgent".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );

    // SecurityReport summarizes the declared dry-run handshake and makes NO
    // security claim about it.
    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.report_version, "0.36");
    assert_eq!(report.atrust_handshakes, 1);
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("handshake_secure"));
    assert!(!serialized.contains("identity_verified"));
}

/// Every fixture under `examples/invalid_atrust_handshakes/` must fail to
/// compile (parse + semantic check). This test also pins the exact expected set
/// of fixtures so it fails if any are missing, the directory is empty, an extra
/// unexercised file appears, or a fixture unexpectedly passes.
#[test]
fn invalid_atrust_handshake_fixtures_all_fail() {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    const EXPECTED: [&str; 54] = [
        "missing_initiator.argx",
        "unknown_initiator.argx",
        "missing_responder.argx",
        "unknown_responder.argx",
        "same_initiator_responder.argx",
        "missing_initiator_identity.argx",
        "unknown_initiator_identity.argx",
        "initiator_identity_subject_mismatch.argx",
        "missing_responder_identity.argx",
        "unknown_responder_identity.argx",
        "responder_identity_subject_mismatch.argx",
        "missing_credential_contracts.argx",
        "unknown_credential_contract.argx",
        "credential_not_participant_bound.argx",
        "missing_boundary.argx",
        "unknown_boundary.argx",
        "boundary_identity_mismatch.argx",
        "boundary_credential_mismatch.argx",
        "missing_method.argx",
        "unknown_method.argx",
        "method_identity_mismatch.argx",
        "method_credential_mismatch.argx",
        "missing_mode.argx",
        "mode_real.argx",
        "missing_direction.argx",
        "missing_challenge.argx",
        "challenge_generated.argx",
        "missing_response.argx",
        "response_signed.argx",
        "missing_transcript.argx",
        "transcript_raw.argx",
        "missing_verification.argx",
        "verification_real.argx",
        "missing_resolution.argx",
        "resolution_remote.argx",
        "missing_network.argx",
        "network_allowed.argx",
        "missing_key_material.argx",
        "key_material_allowed.argx",
        "missing_secret_material.argx",
        "secret_material_allowed.argx",
        "missing_execution.argx",
        "execution_enabled.argx",
        "missing_evidence.argx",
        "evidence_optional.argx",
        "missing_security_claims.argx",
        "security_claim_handshake_secure.argx",
        "missing_purpose.argx",
        "empty_purpose.argx",
        "duplicate_atrust_handshake_name.argx",
        "policy_missing_handshake.argx",
        "policy_handshake_not_dry_run.argx",
        "policy_handshake_security_claims_present.argx",
        "policy_handshake_payload_present.argx",
    ];

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/invalid_atrust_handshakes");

    let actual: BTreeSet<String> = std::fs::read_dir(&dir)
        .expect("invalid_atrust_handshakes directory must exist")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".argx"))
        .collect();

    assert!(!actual.is_empty(), "invalid fixture directory is empty");

    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        actual,
        expected,
        "fixture set drift: missing or unexercised files in {}",
        dir.display()
    );

    for name in EXPECTED {
        let path = dir.join(name);
        let src = std::fs::read_to_string(&path).expect("fixture file must be readable");
        let compiles = parse_source(&src)
            .ok()
            .map(|ast| check_program(&ast).is_ok())
            .unwrap_or(false);
        assert!(
            !compiles,
            "invalid fixture `{name}` unexpectedly passed semantic checking"
        );
    }
}

/// Every fixture under `examples/invalid_trust_ledgers/` must fail to compile
/// (parse + semantic check). Pins the exact expected set so the test fails if
/// any are missing, the directory is empty, an extra unexercised file appears,
/// or a fixture unexpectedly passes.
#[test]
fn invalid_trust_ledger_fixtures_all_fail() {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    const EXPECTED: [&str; 34] = [
        "missing_scope.argx",
        "scope_remote.argx",
        "missing_mode.argx",
        "mode_real.argx",
        "missing_hash_algorithm.argx",
        "unknown_hash_algorithm.argx",
        "denied_hash_algorithm.argx",
        "missing_chain_policy.argx",
        "chain_policy_mutable.argx",
        "missing_entries.argx",
        "empty_entries.argx",
        "duplicate_entry_id.argx",
        "missing_entry_id.argx",
        "missing_entry_kind.argx",
        "unknown_entry_subject.argx",
        "missing_previous_hash.argx",
        "wrong_genesis.argx",
        "broken_previous_hash.argx",
        "missing_entry_hash.argx",
        "entry_hash_wrong_algorithm.argx",
        "missing_evidence_ref.argx",
        "missing_chain_root.argx",
        "wrong_chain_root.argx",
        "network_allowed.argx",
        "key_material_allowed.argx",
        "secret_material_allowed.argx",
        "execution_enabled.argx",
        "evidence_optional.argx",
        "security_claim_immutable.argx",
        "security_claim_blockchain_verified.argx",
        "duplicate_trust_ledger_name.argx",
        "policy_missing_trust_ledger.argx",
        "policy_chain_invalid.argx",
        "policy_security_claims_present.argx",
    ];

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/invalid_trust_ledgers");

    let actual: BTreeSet<String> = std::fs::read_dir(&dir)
        .expect("invalid_trust_ledgers directory must exist")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".argx"))
        .collect();

    assert!(!actual.is_empty(), "invalid fixture directory is empty");

    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        actual,
        expected,
        "fixture set drift: missing or unexercised files in {}",
        dir.display()
    );

    for name in EXPECTED {
        let path = dir.join(name);
        let src = std::fs::read_to_string(&path).expect("fixture file must be readable");
        let compiles = parse_source(&src)
            .ok()
            .map(|ast| check_program(&ast).is_ok())
            .unwrap_or(false);
        assert!(
            !compiles,
            "invalid trust ledger fixture `{name}` unexpectedly passed semantic checking"
        );
    }
}

#[test]
fn v030_trust_ledger_pipeline_and_fixture() {
    let ast = parse_source(include_str!("../examples/trust_ledger_v030.argx")).unwrap();
    check_program(&ast).unwrap();

    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(ir.trust_ledgers.len(), 1);
    let l = &ir.trust_ledgers[0];
    assert_eq!(l.name, "ATrustLedger");
    assert_eq!(l.hash_algorithm, "sha256");
    assert_eq!(l.chain_policy, "append_only");
    assert_eq!(l.entries.len(), 3);
    assert_eq!(l.chain_root, "sha256:declared-entry-003");
    assert_eq!(l.network, "denied");
    assert_eq!(l.execution, "disabled");
    assert_eq!(l.security_claims, "none");

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(bytecode.trust_ledgers.len(), 1);
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();

    let mut fixture: argorix_bytecode::BytecodeProgram =
        serde_json::from_str(include_str!("../examples/trust_ledger_v030.argbc.json")).unwrap();
    assert_eq!(fixture.bytecode_version, "0.31");
    fixture.bytecode_version = bytecode.bytecode_version.clone();
    assert_eq!(bytecode, fixture);

    // No blockchain/consensus/signing/network instructions: ledgers are
    // declarative evidence metadata, not executable trust runtime.
    let json = serde_json::to_value(&bytecode).unwrap();
    let instrs = serde_json::to_string(&json["instructions"]).unwrap();
    for forbidden in [
        "RunLedger",
        "MineBlock",
        "SubmitBlock",
        "SignLedgerEntry",
        "VerifyLedgerSignature",
        "NetworkBroadcast",
        "BlockchainVerified",
    ] {
        assert!(
            !instrs.contains(forbidden),
            "unexpected instruction {forbidden}"
        );
    }
}

#[test]
fn v030_verifier_accepts_v029_and_v030_bytecode() {
    let ast = parse_source(include_str!("../examples/trust_ledger_v030.argx")).unwrap();
    check_program(&ast).unwrap();
    let v030 = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    assert_eq!(v030.bytecode_version, "0.36");
    argorix_bytecode::verify_bytecode(&v030).unwrap();

    // A 0.29 bytecode remains verifiable (backward compatibility preserved).
    let mut legacy = v030.clone();
    legacy.bytecode_version = "0.29".into();
    legacy.trust_ledgers.clear();
    argorix_bytecode::verify_bytecode(&legacy).unwrap();
}

#[test]
fn v030_security_report_covers_trust_ledgers_without_claims() {
    use argorix_vm::{SecurityReport, Vm};

    let ast = parse_source(include_str!("../examples/trust_ledger_v030.argx")).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));

    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        argorix_vm::InjectedMessage {
            from: "User".into(),
            to: "ResearchAgent".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );

    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.report_version, "0.36");
    assert_eq!(report.trust_ledgers, 1);
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("blockchain_verified"));
    assert!(!serialized.contains("tamper_proof"));
}

// ---------------------------------------------------------------------------
// v0.31 — MCP / A2A Bridge Contracts
// ---------------------------------------------------------------------------

const BRIDGE_V031: &str = include_str!("../examples/bridge_contracts_v031.argx");
const EVIDENCE_MAPPING_V032: &str = include_str!("../examples/evidence_mapping_v032.argx");
const GOVERNANCE_MAPPING_V033_DECLS: &str = r#"
governance_profile ChatbotGovernance {
  scope system
  level audit
  domain ai_agent
  owner "Argorix"
  jurisdiction "CL"
  framework "internal-ai-governance"
  evidence_map ChatbotEvidenceMap
  trust_ledger ATrustLedger
  policies ["BridgePolicy", "EvidenceMappingPolicy", "GovernancePolicy"]
  controls [
    {
      id "control-identity-declared"
      category identity
      requirement "agent identity must be declared"
      evidence_ref "evidence-map:identity"
      status mapped
    },
    {
      id "control-network-denied"
      category runtime_boundary
      requirement "network must be denied unless explicitly authorized"
      evidence_ref "security-report:network"
      status mapped
    }
  ]
  risk_level moderate
  review_status draft
  assurance declared_only
  network denied
  external_execution disabled
  secret_material denied
  key_material denied
  execution disabled
  security_claims none
  purpose ["governance", "audit", "risk-controls", "dry-run"]
  notes "metadata only; no legal certification"
}

regulatory_mapping ChatbotRegulatoryMap {
  governance_profile ChatbotGovernance
  evidence_map ChatbotEvidenceMap
  jurisdiction "CL"
  framework "internal-ai-governance"
  obligations [
    {
      id "obligation-transparency"
      source "internal-ai-governance"
      requirement "agent must disclose governance status"
      control "control-identity-declared"
      evidence_ref "evidence-map:ChatbotEvidenceMap"
      status mapped
    }
  ]
  coverage mapped
  assessment declared_only
  legal_claims none
  certification none
  network denied
  external_execution disabled
  secret_material denied
  key_material denied
  execution disabled
  security_claims none
  purpose ["regulatory-mapping", "audit", "governance", "dry-run"]
  notes "mapping only; not legal advice or certification"
}

policy GovernancePolicy {
  require governance_profiles_declared
  require governance_profiles_evidence_bound
  require governance_profiles_controls_mapped
  require governance_profiles_runtime_disabled
  require governance_profiles_security_claims_absent
  require governance_profiles_no_legal_certification
  require regulatory_mappings_declared
  require regulatory_mappings_profiles_bound
  require regulatory_mappings_obligations_mapped
  require regulatory_mappings_controls_bound
  require regulatory_mappings_legal_claims_absent
  require regulatory_mappings_certification_absent
  require regulatory_mappings_runtime_disabled
  require regulatory_mappings_security_claims_absent
  on violation {
    action review
    trace required
  }
}
"#;

fn governance_mapping_v033_source() -> String {
    format!("{EVIDENCE_MAPPING_V032}\n{GOVERNANCE_MAPPING_V033_DECLS}")
}

#[test]
fn v031_parses_mcp_and_a2a_bridge_contracts() {
    let ast = parse_source(BRIDGE_V031).unwrap();
    assert_eq!(ast.mcp_bridge_contracts.len(), 1);
    assert_eq!(ast.a2a_bridge_contracts.len(), 1);
    assert_eq!(ast.mcp_bridge_contracts[0].name.value, "ResearchMcpBridge");
    assert_eq!(ast.a2a_bridge_contracts[0].name.value, "ResearchA2ABridge");
}

#[test]
fn v031_semantic_accepts_valid_bridge_contracts() {
    let ast = parse_source(BRIDGE_V031).unwrap();
    check_program(&ast).expect("valid bridge contracts pass semantic checking");
}

#[test]
fn v031_ir_and_bytecode_include_bridge_contracts() {
    let ast = parse_source(BRIDGE_V031).unwrap();
    check_program(&ast).unwrap();

    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(ir.mcp_bridge_contracts.len(), 1);
    assert_eq!(ir.a2a_bridge_contracts.len(), 1);
    assert_eq!(ir.mcp_bridge_contracts[0].protocol, "mcp");
    assert_eq!(ir.a2a_bridge_contracts[0].protocol, "a2a");

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(bytecode.mcp_bridge_contracts.len(), 1);
    assert_eq!(bytecode.a2a_bridge_contracts.len(), 1);
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();

    let mut fixture: argorix_bytecode::BytecodeProgram =
        serde_json::from_str(include_str!("../examples/bridge_contracts_v031.argbc.json")).unwrap();
    assert_eq!(fixture.bytecode_version, "0.31");
    fixture.bytecode_version = bytecode.bytecode_version.clone();
    assert_eq!(bytecode, fixture);
}

#[test]
fn v031_verifier_accepts_v030_and_v031_bytecode() {
    let ast = parse_source(BRIDGE_V031).unwrap();
    check_program(&ast).unwrap();
    let v031 = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    assert_eq!(v031.bytecode_version, "0.36");
    argorix_bytecode::verify_bytecode(&v031).unwrap();

    let mut legacy = v031.clone();
    legacy.bytecode_version = "0.30".into();
    legacy.mcp_bridge_contracts.clear();
    legacy.a2a_bridge_contracts.clear();
    argorix_bytecode::verify_bytecode(&legacy).unwrap();

    legacy.bytecode_version = "0.29".into();
    legacy.trust_ledgers.clear();
    argorix_bytecode::verify_bytecode(&legacy).unwrap();
}

#[test]
fn v031_security_report_and_trace_cover_bridges_without_claims() {
    use argorix_vm::{SecurityReport, Vm};

    let ast = parse_source(BRIDGE_V031).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));

    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        argorix_vm::InjectedMessage {
            from: "User".into(),
            to: "ResearchAgent".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );

    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.report_version, "0.36");
    assert_eq!(report.mcp_bridge_contracts.total, 1);
    assert_eq!(report.a2a_bridge_contracts.total, 1);
    assert_eq!(report.mcp_bridge_contracts.network.get("denied"), Some(&1));
    assert_eq!(
        report.mcp_bridge_contracts.tool_execution.get("disabled"),
        Some(&1)
    );
    assert_eq!(
        report.a2a_bridge_contracts.agent_execution.get("disabled"),
        Some(&1)
    );
    assert_eq!(
        report.mcp_bridge_contracts.security_claims.get("none"),
        Some(&1)
    );

    let serialized = serde_json::to_string(&report).unwrap();
    for forbidden in [
        "mcp_connected",
        "a2a_connected",
        "tool_verified",
        "agent_verified",
        "secure_bridge",
    ] {
        assert!(!serialized.contains(forbidden), "leaked claim {forbidden}");
    }

    let trace = outcome.result.expect("reactive run succeeds");
    let events = serde_json::to_string(&trace.events).unwrap();
    for declared in [
        "McpBridgeContractDeclared",
        "McpBridgeNetworkDenied",
        "A2ABridgeContractDeclared",
        "A2AAgentExecutionDisabled",
        "BridgeSecurityClaimsDenied",
    ] {
        assert!(events.contains(declared), "missing event {declared}");
    }
    for forbidden in [
        "McpConnected",
        "McpToolCalled",
        "McpServerStarted",
        "A2AMessageSent",
        "A2AAgentExecuted",
        "NetworkRequest",
        "ApiKeyLoaded",
        "OAuthCompleted",
    ] {
        assert!(!events.contains(forbidden), "unexpected event {forbidden}");
    }
}

#[test]
fn v031_bytecode_has_no_executable_bridge_instructions() {
    let ast = parse_source(BRIDGE_V031).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let json = serde_json::to_value(&bytecode).unwrap();
    let instrs = serde_json::to_string(&json["instructions"]).unwrap();
    for forbidden in [
        "OpenMcpConnection",
        "CallMcpTool",
        "SendA2AMessage",
        "ExecuteAgent",
        "OpenNetwork",
        "ReadApiKey",
        "ResolveDid",
        "VerifyCredential",
        "RunHandshake",
    ] {
        assert!(
            !instrs.contains(forbidden),
            "unexpected bridge instruction {forbidden}"
        );
    }
}

#[test]
fn v031_duplicate_bridge_name_is_rejected() {
    let mut doubled = String::from(BRIDGE_V031);
    let block = concat!(
        "mcp_bridge_contract ResearchMcpBridge {\n",
        "  agent ResearchAgent\n  passport ResearchPassport\n  identity ResearchIdentity\n",
        "  boundary AgentTrustBoundary\n  transport declared_only\n  protocol mcp\n",
        "  direction outbound\n  tools []\n  resources []\n  prompts []\n",
        "  network denied\n  external_execution disabled\n  tool_execution disabled\n",
        "  secret_material denied\n  key_material denied\n  authentication none\n",
        "  authorization policy_bound\n  evidence required\n  security_claims none\n",
        "  purpose [\"mcp\"]\n}\n"
    );
    doubled.push_str(block);
    let ast = parse_source(&doubled).unwrap();
    let err = check_program(&ast).expect_err("duplicate bridge name must be rejected");
    assert!(err
        .iter()
        .any(|d| d.message.contains("duplicate mcp_bridge_contract")));
}

#[test]
fn invalid_bridge_contract_fixtures_all_fail() {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    const EXPECTED: [&str; 32] = [
        "missing_mcp_agent.argx",
        "unknown_mcp_agent.argx",
        "mcp_passport_agent_mismatch.argx",
        "mcp_identity_agent_mismatch.argx",
        "mcp_boundary_mismatch.argx",
        "mcp_transport_live.argx",
        "mcp_network_allowed.argx",
        "mcp_external_execution_enabled.argx",
        "mcp_tool_execution_enabled.argx",
        "mcp_secret_material_allowed.argx",
        "mcp_key_material_allowed.argx",
        "mcp_authentication_api_key.argx",
        "mcp_security_claim_secure_bridge.argx",
        "missing_a2a_initiator.argx",
        "same_a2a_agents.argx",
        "a2a_passport_agent_mismatch.argx",
        "a2a_identity_agent_mismatch.argx",
        "a2a_unknown_handshake.argx",
        "a2a_handshake_agent_mismatch.argx",
        "a2a_unknown_trust_ledger.argx",
        "a2a_ledger_missing_handshake.argx",
        "a2a_unknown_message_contract.argx",
        "a2a_transport_live.argx",
        "a2a_network_allowed.argx",
        "a2a_external_execution_enabled.argx",
        "a2a_agent_execution_enabled.argx",
        "a2a_secret_material_allowed.argx",
        "a2a_key_material_allowed.argx",
        "a2a_authentication_api_key.argx",
        "a2a_security_claim_secure_bridge.argx",
        "duplicate_mcp_bridge_name.argx",
        "duplicate_a2a_bridge_name.argx",
    ];

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/invalid_bridge_contracts");
    let actual: BTreeSet<String> = std::fs::read_dir(&dir)
        .expect("invalid_bridge_contracts directory must exist")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".argx"))
        .collect();
    assert!(!actual.is_empty(), "invalid fixture directory is empty");
    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(actual, expected, "fixture set drift in {}", dir.display());
    for name in EXPECTED {
        let path = dir.join(name);
        let src = std::fs::read_to_string(&path).expect("fixture file must be readable");
        let compiles = parse_source(&src)
            .ok()
            .map(|ast| check_program(&ast).is_ok())
            .unwrap_or(false);
        assert!(
            !compiles,
            "invalid bridge fixture `{name}` unexpectedly passed semantic checking"
        );
    }
}

#[test]
fn v031_declares_no_network_secret_or_runtime_surface() {
    let ast = parse_source(BRIDGE_V031).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    assert!(bytecode.secrets.is_empty(), "no secret boundaries declared");
    for c in &bytecode.mcp_bridge_contracts {
        assert_eq!(c.network, "denied");
        assert_eq!(c.external_execution, "disabled");
        assert_eq!(c.tool_execution, "disabled");
        assert_eq!(c.secret_material, "denied");
        assert_eq!(c.key_material, "denied");
        assert!(matches!(
            c.authentication.as_str(),
            "none" | "declared_only"
        ));
        assert_eq!(c.security_claims, "none");
    }
    for c in &bytecode.a2a_bridge_contracts {
        assert_eq!(c.network, "denied");
        assert_eq!(c.external_execution, "disabled");
        assert_eq!(c.agent_execution, "disabled");
        assert_eq!(c.secret_material, "denied");
        assert_eq!(c.key_material, "denied");
        assert!(matches!(
            c.authentication.as_str(),
            "none" | "declared_only"
        ));
        assert_eq!(c.security_claims, "none");
    }
}

// ---------------------------------------------------------------------------
// v0.32 — ATrust Evidence Mapping
// ---------------------------------------------------------------------------

#[test]
fn v032_parses_atrust_evidence_map() {
    let ast = parse_source(EVIDENCE_MAPPING_V032).unwrap();
    assert_eq!(ast.atrust_evidence_maps.len(), 1);
    let map = &ast.atrust_evidence_maps[0];
    assert_eq!(map.name.value, "ChatbotEvidenceMap");
    assert_eq!(map.agent.value, "ResearchAgent");
    assert_eq!(map.passport.value, "ResearchPassport");
    assert_eq!(map.identity.value, "ResearchIdentity");
    assert_eq!(map.credential_contract.value, "ResearchCredential");
    assert_eq!(map.handshake.value, "ResearchHandshake");
    assert_eq!(map.trust_ledger.value, "ATrustLedger");
    assert_eq!(map.mcp_bridges[0].value, "ResearchMcpBridge");
    assert_eq!(map.a2a_bridges[0].value, "ResearchA2ABridge");
}

#[test]
fn v032_semantic_accepts_valid_evidence_map() {
    let ast = parse_source(EVIDENCE_MAPPING_V032).unwrap();
    check_program(&ast).expect("valid ATrust evidence map passes semantic checking");
}

#[test]
fn v032_semantic_rejects_evidence_map_invalid_core_links_and_denied_modes() {
    for (name, expected) in [
        ("unknown_agent.argx", "unknown agent"),
        ("passport_agent_mismatch.argx", "passport"),
        ("identity_agent_mismatch.argx", "identity"),
        ("credential_identity_mismatch.argx", "credential_contract"),
        ("handshake_missing_agent.argx", "does not include agent"),
        ("ledger_missing_identity.argx", "does not contain identity"),
        (
            "ledger_missing_credential.argx",
            "does not contain credential",
        ),
        (
            "ledger_missing_handshake.argx",
            "does not contain handshake",
        ),
        ("unknown_mcp_bridge.argx", "unknown mcp_bridge"),
        ("mcp_bridge_agent_mismatch.argx", "mcp_bridge"),
        ("unknown_a2a_bridge.argx", "unknown a2a_bridge"),
        ("a2a_bridge_handshake_mismatch.argx", "handshake mismatch"),
        ("a2a_bridge_ledger_mismatch.argx", "ledger mismatch"),
        ("unknown_policy.argx", "unknown policy"),
        ("coverage_optional.argx", "coverage"),
        ("mapping_mode_live.argx", "mapping_mode"),
        ("verification_real.argx", "verification"),
        ("resolution_remote.argx", "resolution"),
        ("network_allowed.argx", "network"),
        ("external_execution_enabled.argx", "external_execution"),
        ("secret_material_allowed.argx", "secret_material"),
        ("key_material_allowed.argx", "key_material"),
        ("execution_enabled.argx", "execution"),
        ("security_claim_identity_verified.argx", "security_claims"),
    ] {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples/invalid_evidence_maps")
            .join(name);
        let source = std::fs::read_to_string(&path).unwrap();
        let diagnostics = check(&source).expect_err("invalid fixture must fail");
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains(expected)),
            "{name}: expected `{expected}` in {diagnostics:?}"
        );
    }
}

#[test]
fn v032_ir_bytecode_vm_report_and_bundle_cover_evidence_maps_without_real_trust_claims() {
    use argorix_vm::{EvidenceBundle, SecurityReport, Vm};

    let ast = parse_source(EVIDENCE_MAPPING_V032).unwrap();
    check_program(&ast).unwrap();

    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(ir.atrust_evidence_maps.len(), 1);
    assert_eq!(ir.atrust_evidence_maps[0].mapping_mode, "declared_only");

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(bytecode.atrust_evidence_maps.len(), 1);
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();

    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        argorix_vm::InjectedMessage {
            from: "User".into(),
            to: "ResearchAgent".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );
    let trace = outcome.result.as_ref().expect("reactive run succeeds");
    assert_eq!(trace.vm_version, "0.36");
    let events = serde_json::to_string(&trace.events).unwrap();
    for expected in [
        "ATrustEvidenceMapDeclared",
        "ATrustEvidenceMapCoverageRequired",
        "ATrustEvidenceMapLinksValidated",
        "ATrustEvidenceMapRuntimeDisabled",
        "ATrustEvidenceMapSecurityClaimsDenied",
    ] {
        assert!(events.contains(expected), "missing event {expected}");
    }

    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.report_version, "0.36");
    assert_eq!(report.atrust_evidence_maps.total, 1);
    assert_eq!(
        report.atrust_evidence_maps.names,
        vec!["ChatbotEvidenceMap"]
    );
    assert_eq!(report.atrust_evidence_maps.bridge_links_total, 2);

    let bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        std::path::Path::new("target/evidence-v032/bundle.json"),
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(bundle.bundle_version, "0.36");

    let serialized = serde_json::to_string(&(bytecode, trace, report, bundle)).unwrap();
    for forbidden in [
        "IdentityVerified",
        "CredentialVerified",
        "HandshakeExecuted",
        "BridgeConnected",
        "McpToolCalled",
        "A2AMessageSent",
        "NetworkRequest",
        "SignatureVerified",
        "blockchain_verified",
    ] {
        assert!(!serialized.contains(forbidden), "leaked claim {forbidden}");
    }
    assert!(
        !serialized.contains("\"post_quantum_secure\":true"),
        "leaked positive post-quantum security claim"
    );
}

#[test]
fn v032_verifier_accepts_v031_and_v032_bytecode() {
    let ast = parse_source(EVIDENCE_MAPPING_V032).unwrap();
    check_program(&ast).unwrap();
    let v032 = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    assert_eq!(v032.bytecode_version, "0.36");
    argorix_bytecode::verify_bytecode(&v032).unwrap();

    let mut legacy = v032.clone();
    legacy.bytecode_version = "0.31".into();
    legacy.atrust_evidence_maps.clear();
    argorix_bytecode::verify_bytecode(&legacy).unwrap();
}

#[test]
fn invalid_evidence_map_fixtures_all_fail() {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    const EXPECTED: [&str; 40] = [
        "missing_agent.argx",
        "unknown_agent.argx",
        "unknown_passport.argx",
        "passport_agent_mismatch.argx",
        "unknown_identity.argx",
        "identity_agent_mismatch.argx",
        "unknown_credential_contract.argx",
        "credential_identity_mismatch.argx",
        "unknown_handshake.argx",
        "handshake_missing_agent.argx",
        "unknown_trust_ledger.argx",
        "ledger_missing_identity.argx",
        "ledger_missing_credential.argx",
        "ledger_missing_handshake.argx",
        "unknown_mcp_bridge.argx",
        "mcp_bridge_agent_mismatch.argx",
        "unknown_a2a_bridge.argx",
        "a2a_bridge_handshake_mismatch.argx",
        "a2a_bridge_ledger_mismatch.argx",
        "unknown_policy.argx",
        "coverage_optional.argx",
        "mapping_mode_live.argx",
        "verification_real.argx",
        "resolution_remote.argx",
        "evidence_bundle_optional.argx",
        "security_report_optional.argx",
        "trace_optional.argx",
        "network_allowed.argx",
        "external_execution_enabled.argx",
        "secret_material_allowed.argx",
        "key_material_allowed.argx",
        "execution_enabled.argx",
        "security_claim_identity_verified.argx",
        "security_claim_credential_verified.argx",
        "security_claim_handshake_secure.argx",
        "security_claim_bridge_connected.argx",
        "security_claim_tamper_proof.argx",
        "security_claim_blockchain_verified.argx",
        "security_claim_post_quantum_secure.argx",
        "duplicate_evidence_map_name.argx",
    ];

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/invalid_evidence_maps");
    let actual: BTreeSet<String> = std::fs::read_dir(&dir)
        .expect("invalid_evidence_maps directory must exist")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".argx"))
        .collect();
    assert!(!actual.is_empty(), "invalid fixture directory is empty");
    let expected: BTreeSet<String> = EXPECTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(actual, expected, "fixture set drift in {}", dir.display());
    for name in EXPECTED {
        let path = dir.join(name);
        let src = std::fs::read_to_string(&path).expect("fixture file must be readable");
        let compiles = parse_source(&src)
            .ok()
            .map(|ast| check_program(&ast).is_ok())
            .unwrap_or(false);
        assert!(
            !compiles,
            "invalid evidence map fixture `{name}` unexpectedly passed semantic checking"
        );
    }
}

// ---------------------------------------------------------------------------
// v0.33 — Governance Profiles + Regulatory Mapping
// ---------------------------------------------------------------------------

#[test]
fn v033_parses_governance_profile_and_regulatory_mapping() {
    let source = governance_mapping_v033_source();
    let ast = parse_source(&source).unwrap();

    assert_eq!(ast.governance_profiles.len(), 1);
    let profile = &ast.governance_profiles[0];
    assert_eq!(profile.name.value, "ChatbotGovernance");
    assert_eq!(profile.scope.value.source_name(), "system");
    assert_eq!(profile.level.value.source_name(), "audit");
    assert_eq!(profile.domain.value.source_name(), "ai_agent");
    assert_eq!(profile.owner.value, "Argorix");
    assert_eq!(profile.evidence_map.value, "ChatbotEvidenceMap");
    assert_eq!(profile.controls.len(), 2);
    assert_eq!(profile.controls[0].id.value, "control-identity-declared");
    assert_eq!(
        profile.controls[1].category.value.source_name(),
        "runtime_boundary"
    );

    assert_eq!(ast.regulatory_mappings.len(), 1);
    let mapping = &ast.regulatory_mappings[0];
    assert_eq!(mapping.name.value, "ChatbotRegulatoryMap");
    assert_eq!(mapping.governance_profile.value, "ChatbotGovernance");
    assert_eq!(mapping.evidence_map.value, "ChatbotEvidenceMap");
    assert_eq!(mapping.obligations.len(), 1);
    assert_eq!(
        mapping.obligations[0].control.value,
        "control-identity-declared"
    );
    assert_eq!(mapping.coverage.value.source_name(), "mapped");
}

#[test]
fn v033_semantic_accepts_valid_governance_mapping() {
    let source = governance_mapping_v033_source();
    let ast = parse_source(&source).unwrap();
    check_program(&ast).expect("valid governance and regulatory mapping must pass");
}

#[test]
fn v033_semantics_rejects_invalid_governance_and_regulatory_claims() {
    let valid = governance_mapping_v033_source();
    for (name, from, to, expected) in [
        (
            "unknown evidence map",
            "evidence_map ChatbotEvidenceMap",
            "evidence_map MissingEvidenceMap",
            "unknown evidence_map",
        ),
        (
            "unknown trust ledger",
            "trust_ledger ATrustLedger",
            "trust_ledger MissingLedger",
            "unknown trust_ledger",
        ),
        (
            "unknown policy",
            "policies [\"BridgePolicy\", \"EvidenceMappingPolicy\"]",
            "policies [\"MissingPolicy\"]",
            "unknown policy",
        ),
        (
            "eliminated risk",
            "risk_level moderate",
            "risk_level eliminated",
            "risk_level",
        ),
        (
            "regulator approval",
            "review_status draft",
            "review_status regulator_approved",
            "review_status",
        ),
        (
            "legal certification assurance",
            "assurance declared_only",
            "assurance legally_certified",
            "assurance",
        ),
        (
            "governance network",
            "network denied",
            "network allowed",
            "network",
        ),
        (
            "governance execution",
            "key_material denied\n  execution disabled\n  security_claims none\n  purpose [\"governance\"",
            "key_material denied\n  execution enabled\n  security_claims none\n  purpose [\"governance\"",
            "execution",
        ),
        (
            "certified coverage",
            "coverage mapped",
            "coverage complete_certified",
            "coverage",
        ),
        (
            "legal assessment",
            "assessment declared_only",
            "assessment legally_verified",
            "assessment",
        ),
        (
            "legal claims",
            "legal_claims none",
            "legal_claims compliant",
            "legal_claims",
        ),
        (
            "certification",
            "certification none",
            "certification regulator_approved",
            "certification",
        ),
    ] {
        let source = valid.replacen(from, to, 1);
        let diagnostics = check(&source).expect_err(name);
        assert!(
            diagnostics.iter().any(|d| d.message.contains(expected)),
            "{name}: expected `{expected}` in {diagnostics:?}"
        );
    }
}

#[test]
fn v033_ir_and_bytecode_preserve_governance_mappings_and_v032_compatibility() {
    let source = governance_mapping_v033_source();
    let ast = parse_source(&source).unwrap();
    check_program(&ast).unwrap();

    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(ir.governance_profiles.len(), 1);
    assert_eq!(ir.governance_profiles[0].controls.len(), 2);
    assert_eq!(ir.governance_profiles[0].assurance, "declared_only");
    assert_eq!(ir.regulatory_mappings.len(), 1);
    assert_eq!(ir.regulatory_mappings[0].obligations.len(), 1);
    assert_eq!(ir.regulatory_mappings[0].legal_claims, "none");

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(bytecode.governance_profiles.len(), 1);
    assert_eq!(bytecode.regulatory_mappings.len(), 1);
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();

    let mut legacy = bytecode.clone();
    legacy.bytecode_version = "0.32".into();
    legacy.governance_profiles.clear();
    legacy.regulatory_mappings.clear();
    argorix_bytecode::verify_bytecode(&legacy).unwrap();
}

#[test]
fn v033_vm_report_and_evidence_expose_governance_without_certification_claims() {
    use argorix_vm::{EvidenceBundle, SecurityReport, Vm};

    let source = governance_mapping_v033_source();
    let ast = parse_source(&source).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        argorix_vm::InjectedMessage {
            from: "User".into(),
            to: "ResearchAgent".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );
    let trace = outcome.result.as_ref().unwrap();
    assert_eq!(trace.vm_version, "0.36");
    assert_eq!(trace.governance_profiles.len(), 1);
    assert_eq!(trace.regulatory_mappings.len(), 1);
    let events = serde_json::to_string(&trace.events).unwrap();
    for expected in [
        "GovernanceProfileDeclared",
        "GovernanceControlsMapped",
        "RegulatoryMappingDeclared",
        "RegulatoryObligationsMapped",
        "GovernanceRuntimeDisabled",
        "GovernanceSecurityClaimsDenied",
        "LegalCertificationDenied",
    ] {
        assert!(events.contains(expected), "missing event {expected}");
    }

    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.report_version, "0.36");
    assert_eq!(report.governance_profiles.total, 1);
    assert_eq!(report.governance_profiles.controls_total, 2);
    assert_eq!(report.regulatory_mappings.total, 1);
    assert_eq!(report.regulatory_mappings.obligations_total, 1);
    assert_eq!(report.regulatory_mappings.legal_claims_none, 1);
    assert_eq!(report.regulatory_mappings.certification_none, 1);

    let bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        std::path::Path::new("target/evidence-v033/bundle.json"),
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(bundle.bundle_version, "0.36");

    let serialized = serde_json::to_string(&(trace, report, bundle)).unwrap();
    for forbidden in [
        "ComplianceCertified",
        "RegulatorApproved",
        "LegalVerified",
        "RiskEliminated",
        "ExternalAuditPassed",
    ] {
        assert!(!serialized.contains(forbidden), "leaked claim {forbidden}");
    }
}

#[test]
fn v033_single_file_fixture_compiles() {
    let source = include_str!("../examples/governance_mapping_v033.argx");
    let ast = parse_source(source).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();
}

#[test]
fn invalid_governance_mapping_fixtures_are_complete_and_all_fail() {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    const EXPECTED: [&str; 33] = [
        "unknown_governance_evidence_map.argx",
        "unknown_governance_trust_ledger.argx",
        "unknown_governance_policy.argx",
        "empty_controls.argx",
        "duplicate_control_id.argx",
        "control_empty_requirement.argx",
        "control_empty_evidence_ref.argx",
        "risk_level_eliminated.argx",
        "review_status_regulator_approved.argx",
        "assurance_legally_certified.argx",
        "governance_network_allowed.argx",
        "governance_external_execution_enabled.argx",
        "governance_secret_material_allowed.argx",
        "governance_key_material_allowed.argx",
        "governance_execution_enabled.argx",
        "governance_security_claim_compliant.argx",
        "unknown_regulatory_governance_profile.argx",
        "regulatory_evidence_map_mismatch.argx",
        "empty_obligations.argx",
        "duplicate_obligation_id.argx",
        "obligation_unknown_control.argx",
        "coverage_complete_certified.argx",
        "assessment_legally_verified.argx",
        "legal_claims_compliant.argx",
        "certification_regulator_approved.argx",
        "regulatory_network_allowed.argx",
        "regulatory_external_execution_enabled.argx",
        "regulatory_secret_material_allowed.argx",
        "regulatory_key_material_allowed.argx",
        "regulatory_execution_enabled.argx",
        "regulatory_security_claim_certified.argx",
        "duplicate_governance_profile_name.argx",
        "duplicate_regulatory_mapping_name.argx",
    ];
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/invalid_governance_mappings");
    let actual: BTreeSet<String> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let expected: BTreeSet<String> = EXPECTED.iter().map(|value| (*value).into()).collect();
    assert!(
        !actual.is_empty(),
        "invalid fixture directory must not be empty"
    );
    assert_eq!(actual, expected, "missing or unexercised invalid fixture");
    for filename in EXPECTED {
        let source = std::fs::read_to_string(directory.join(filename)).unwrap();
        let ast = parse_source(&source).unwrap();
        assert!(
            check_program(&ast).is_err(),
            "{filename} unexpectedly passed semantic checking"
        );
    }
}

// ---------------------------------------------------------------------------
// v0.34 — Third-Party Verification / Public Conformance
// ---------------------------------------------------------------------------

const PUBLIC_CONFORMANCE_V034_DECLS: &str = r#"
module main

third_party_verifier CommunityVerifier {
  verifier_type community
  independence declared
  identity_mode declared_only
  verification_mode reproducible_artifacts
  name "Community Reviewer"
  organization "Argorix Community"
  jurisdiction "CL"
  allowed_scopes ["conformance", "evidence", "security-report", "governance"]
  disallowed_claims ["legal-certification", "regulator-approval", "cryptographic-endorsement"]
  network denied
  external_execution disabled
  secret_material denied
  key_material denied
  execution disabled
  legal_claims none
  certification none
  security_claims none
  purpose ["third-party-verifier", "public-conformance", "audit"]
  notes "metadata only; no external verification runtime"
}

public_conformance_report PublicConformanceV034 {
  verifier CommunityVerifier
  suite "conformance/suite.v034.json"
  suite_version "0.34"
  source_artifact "conformance/sources/public_conformance_v034.argx"
  bytecode_artifact "examples/public_conformance_v034.argbc.json"
  evidence_map ChatbotEvidenceMap
  governance_profile ChatbotGovernance
  regulatory_mapping ChatbotRegulatoryMap
  trust_ledger ATrustLedger
  security_report required
  evidence_bundle required
  trace required
  result passed
  reproducibility declared
  review_status draft
  claims [
    {
      id "claim-conformance-replayable"
      category conformance
      statement "conformance suite can be replayed locally"
      evidence_ref "conformance:suite.v034"
      status mapped
    },
    {
      id "claim-security-report-present"
      category security_report
      statement "security report is produced as an artifact"
      evidence_ref "security-report:required"
      status mapped
    }
  ]
  network denied
  external_execution disabled
  secret_material denied
  key_material denied
  execution disabled
  legal_claims none
  certification none
  security_claims none
  purpose ["public-conformance", "third-party-review", "audit", "dry-run"]
  notes "public conformance metadata only; not certification"
}

policy PublicConformancePolicy {
  require third_party_verifiers_declared
  require third_party_verifiers_identity_declared
  require third_party_verifiers_scope_bounded
  require third_party_verifiers_runtime_disabled
  require third_party_verifiers_legal_claims_absent
  require third_party_verifiers_certification_absent
  require third_party_verifiers_security_claims_absent
  require public_conformance_reports_declared
  require public_conformance_reports_verifiers_bound
  require public_conformance_reports_artifacts_declared
  require public_conformance_reports_evidence_bound
  require public_conformance_reports_governance_bound
  require public_conformance_reports_regulatory_bound
  require public_conformance_reports_replayable
  require public_conformance_reports_runtime_disabled
  require public_conformance_reports_legal_claims_absent
  require public_conformance_reports_certification_absent
  require public_conformance_reports_security_claims_absent
  on violation {
    action review
    trace required
  }
}
"#;

#[test]
fn v034_parses_third_party_verifier_and_public_conformance_report() {
    let ast = parse_source(PUBLIC_CONFORMANCE_V034_DECLS).unwrap();
    assert_eq!(ast.third_party_verifiers.len(), 1);
    assert_eq!(ast.third_party_verifiers[0].name.value, "CommunityVerifier");
    assert_eq!(
        ast.third_party_verifiers[0]
            .verification_mode
            .value
            .source_name(),
        "reproducible_artifacts"
    );
    assert_eq!(ast.public_conformance_reports.len(), 1);
    let report = &ast.public_conformance_reports[0];
    assert_eq!(report.name.value, "PublicConformanceV034");
    assert_eq!(report.verifier.value, "CommunityVerifier");
    assert_eq!(report.claims.len(), 2);
    assert_eq!(report.claims[0].category.value.source_name(), "conformance");
}

fn public_conformance_v034_source() -> String {
    format!(
        "{}\n{}",
        governance_mapping_v033_source(),
        PUBLIC_CONFORMANCE_V034_DECLS.replacen("module main", "", 1)
    )
}

#[test]
fn v034_semantics_accept_valid_public_conformance() {
    let ast = parse_source(&public_conformance_v034_source()).unwrap();
    check_program(&ast).expect("valid public conformance declarations must pass");
}

#[test]
fn v034_semantics_reject_unknown_verifier() {
    let source = public_conformance_v034_source().replacen(
        "verifier CommunityVerifier",
        "verifier MissingVerifier",
        1,
    );
    let diagnostics = check(&source).expect_err("unknown verifier must fail");
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("unknown third_party_verifier")),
        "{diagnostics:?}"
    );
}

#[test]
fn v034_ir_and_bytecode_preserve_public_conformance_and_v033_compatibility() {
    let ast = parse_source(&public_conformance_v034_source()).unwrap();
    check_program(&ast).unwrap();
    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(ir.third_party_verifiers.len(), 1);
    assert_eq!(ir.public_conformance_reports.len(), 1);
    assert_eq!(ir.public_conformance_reports[0].claims.len(), 2);
    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(bytecode.third_party_verifiers.len(), 1);
    assert_eq!(bytecode.public_conformance_reports.len(), 1);
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();
    let mut legacy = bytecode;
    legacy.bytecode_version = "0.33".into();
    legacy.third_party_verifiers.clear();
    legacy.public_conformance_reports.clear();
    argorix_bytecode::verify_bytecode(&legacy).unwrap();
}

#[test]
fn v034_vm_trace_preserves_public_conformance_without_external_verification() {
    use argorix_vm::Vm;
    let ast = parse_source(&public_conformance_v034_source()).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let trace = Vm::new()
        .run_reactive(
            &bytecode,
            argorix_vm::InjectedMessage {
                from: "User".into(),
                to: "ResearchAgent".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
        )
        .unwrap();
    assert_eq!(trace.vm_version, "0.36");
    assert_eq!(trace.third_party_verifiers.len(), 1);
    assert_eq!(trace.public_conformance_reports.len(), 1);
    let events = serde_json::to_string(&trace.events).unwrap();
    for expected in [
        "ThirdPartyVerifierDeclared",
        "PublicConformanceReportDeclared",
        "PublicConformanceArtifactsMapped",
        "PublicConformanceReplayDeclared",
        "PublicConformanceRuntimeDisabled",
        "PublicConformanceSecurityClaimsDenied",
        "LegalCertificationDenied",
        "RemoteVerificationDenied",
    ] {
        assert!(events.contains(expected), "missing {expected}");
    }
    for forbidden in [
        "ExternalAuditPassed",
        "ComplianceCertified",
        "RegulatorApproved",
        "SignatureVerified",
        "RemoteAttestationVerified",
    ] {
        assert!(!events.contains(forbidden), "leaked {forbidden}");
    }
}

#[test]
fn invalid_public_conformance_fixtures_are_complete_and_all_fail() {
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    const EXPECTED: [&str; 37] = [
        "unknown_verifier.argx",
        "verifier_network_allowed.argx",
        "verifier_external_execution_enabled.argx",
        "verifier_secret_material_allowed.argx",
        "verifier_key_material_allowed.argx",
        "verifier_execution_enabled.argx",
        "verifier_legal_claims_compliant.argx",
        "verifier_certification_regulator_approved.argx",
        "verifier_security_claim_cryptographically_verified.argx",
        "empty_allowed_scopes.argx",
        "empty_disallowed_claims.argx",
        "unknown_report_verifier.argx",
        "unknown_report_evidence_map.argx",
        "unknown_report_governance_profile.argx",
        "unknown_report_regulatory_mapping.argx",
        "unknown_report_trust_ledger.argx",
        "report_governance_evidence_mismatch.argx",
        "report_regulatory_governance_mismatch.argx",
        "report_regulatory_evidence_mismatch.argx",
        "suite_version_wrong.argx",
        "empty_claims.argx",
        "duplicate_claim_id.argx",
        "claim_empty_statement.argx",
        "claim_empty_evidence_ref.argx",
        "result_certified.argx",
        "reproducibility_live_verified.argx",
        "review_status_regulator_approved.argx",
        "report_network_allowed.argx",
        "report_external_execution_enabled.argx",
        "report_secret_material_allowed.argx",
        "report_key_material_allowed.argx",
        "report_execution_enabled.argx",
        "report_legal_claims_compliant.argx",
        "report_certification_iso_certified.argx",
        "report_security_claim_tamper_proof.argx",
        "duplicate_third_party_verifier_name.argx",
        "duplicate_public_conformance_report_name.argx",
    ];
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/invalid_public_conformance");
    let actual: BTreeSet<String> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let expected: BTreeSet<String> = EXPECTED.iter().map(|value| (*value).into()).collect();
    assert!(
        !actual.is_empty(),
        "invalid fixture directory must not be empty"
    );
    assert_eq!(actual, expected, "invalid fixture inventory drift");
    for name in EXPECTED {
        let source = std::fs::read_to_string(directory.join(name)).unwrap();
        let passes = parse_source(&source)
            .ok()
            .is_some_and(|program| check_program(&program).is_ok());
        assert!(!passes, "invalid fixture `{name}` unexpectedly passed");
    }
}

#[test]
fn v034_security_report_and_evidence_cover_public_conformance() {
    use argorix_vm::{EvidenceBundle, SecurityReport, Vm};
    let ast = parse_source(&public_conformance_v034_source()).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        argorix_vm::InjectedMessage {
            from: "User".into(),
            to: "ResearchAgent".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );
    let trace = outcome.result.as_ref().unwrap();
    let policy = trace
        .policy_report
        .policy_blocks
        .iter()
        .find(|policy| policy.name == "PublicConformancePolicy")
        .expect("public conformance policy is evaluated");
    assert!(policy.passed, "{policy:?}");
    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.report_version, "0.36");
    assert_eq!(report.third_party_verifiers.total, 1);
    assert_eq!(report.third_party_verifiers.network_denied, 1);
    assert_eq!(report.third_party_verifiers.legal_claims_none, 1);
    assert_eq!(report.public_conformance_reports.total, 1);
    assert_eq!(report.public_conformance_reports.claims_total, 2);
    assert_eq!(
        report.public_conformance_reports.evidence_bundle_required,
        1
    );
    let bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        std::path::Path::new("target/evidence-v034/bundle.json"),
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(bundle.bundle_version, "0.36");
}

#[test]
fn v034_single_file_and_bytecode_fixture_match() {
    let source = include_str!("../examples/public_conformance_v034.argx");
    let ast = parse_source(source).unwrap();
    check_program(&ast).unwrap();
    let mut emitted = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    emitted.bytecode_version = "0.34".into();
    let fixture: argorix_bytecode::BytecodeProgram = serde_json::from_str(include_str!(
        "../examples/public_conformance_v034.argbc.json"
    ))
    .unwrap();
    assert_eq!(emitted, fixture);
    argorix_bytecode::verify_bytecode(&fixture).unwrap();
}

// ---------------------------------------------------------------------------
// v0.35 — Runtime Hardening + Threat Model
// ---------------------------------------------------------------------------

const RUNTIME_HARDENING_V035_DECLS: &str = r#"
runtime_hardening_profile HardenedRuntime {
  scope system
  mode declared_only
  enforcement evidence_only
  sandbox required
  provider_execution disabled
  external_providers disabled
  network denied
  tool_execution disabled
  agent_execution disabled
  filesystem_access denied
  env_access denied
  secret_material denied
  key_material denied
  allowlist required
  deny_by_default true
  approval required
  audit_log required
  evidence required
  incident_response declared
  evidence_map ChatbotEvidenceMap
  governance_profile ChatbotGovernance
  public_conformance_report PublicConformanceV034
  protected_assets ["api-keys", "user-data", "agent-identity", "evidence-bundle"]
  runtime_boundaries ["network", "secrets", "tools", "providers", "agents"]
  residual_risk moderate
  review_status draft
  assurance declared_only
  security_claims none
  purpose ["runtime-hardening", "threat-model", "pre-runtime", "dry-run"]
  notes "metadata only; no real runtime execution"
}

threat_model AgentThreatModel {
  hardening_profile HardenedRuntime
  evidence_map ChatbotEvidenceMap
  governance_profile ChatbotGovernance
  public_conformance_report PublicConformanceV034
  methodology declared
  scope system
  review_status draft
  assets [
    {
      id "asset-api-keys"
      category secret
      description "provider API keys must not be exposed"
      sensitivity high
      evidence_ref "hardening:secret-boundary"
    },
    {
      id "asset-evidence-bundle"
      category evidence
      description "evidence bundles must remain auditable"
      sensitivity moderate
      evidence_ref "evidence:bundle"
    }
  ]
  threats [
    {
      id "threat-prompt-injection"
      category prompt_injection
      target "agent-runtime"
      impact high
      mitigation "deny external execution and require policy review"
      status mitigated_declared
    },
    {
      id "threat-secret-leakage"
      category secret_leakage
      target "provider-boundary"
      impact critical
      mitigation "env_access denied and secret_material denied"
      status mitigated_declared
    }
  ]
  mitigations [
    {
      id "mitigation-deny-network"
      category network_boundary
      control_ref "network"
      evidence_ref "security-report:network-denied"
      status mapped
    },
    {
      id "mitigation-deny-secrets"
      category secret_boundary
      control_ref "secret_material"
      evidence_ref "security-report:secrets-denied"
      status mapped
    }
  ]
  residual_risk moderate
  risk_acceptance declared_only
  network denied
  external_execution disabled
  tool_execution disabled
  agent_execution disabled
  secret_material denied
  key_material denied
  execution disabled
  security_claims none
  purpose ["threat-model", "runtime-hardening", "audit", "dry-run"]
  notes "metadata only; no attack execution or security certification"
}

policy RuntimeHardeningPolicy {
  require runtime_hardening_profiles_declared
  require runtime_hardening_evidence_bound
  require runtime_hardening_deny_by_default
  require runtime_hardening_sandbox_required
  require runtime_hardening_network_denied
  require runtime_hardening_external_providers_disabled
  require runtime_hardening_tool_execution_disabled
  require runtime_hardening_agent_execution_disabled
  require runtime_hardening_filesystem_denied
  require runtime_hardening_env_denied
  require runtime_hardening_secret_material_denied
  require runtime_hardening_key_material_denied
  require runtime_hardening_audit_log_required
  require runtime_hardening_security_claims_absent
  require threat_models_declared
  require threat_models_hardening_bound
  require threat_models_assets_mapped
  require threat_models_threats_mapped
  require threat_models_mitigations_mapped
  require threat_models_runtime_disabled
  require threat_models_network_denied
  require threat_models_secret_material_denied
  require threat_models_key_material_denied
  require threat_models_execution_disabled
  require threat_models_security_claims_absent
  on violation {
    action block
    trace required
  }
}
"#;

fn runtime_hardening_v035_source() -> String {
    format!(
        "{}\n{}",
        public_conformance_v034_source(),
        RUNTIME_HARDENING_V035_DECLS
    )
}

#[test]
fn v035_parses_runtime_hardening_profile_and_threat_model() {
    let ast = parse_source(&runtime_hardening_v035_source()).unwrap();
    assert_eq!(ast.runtime_hardening_profiles.len(), 1);
    assert_eq!(
        ast.runtime_hardening_profiles[0].name.value,
        "HardenedRuntime"
    );
    assert_eq!(ast.runtime_hardening_profiles[0].protected_assets.len(), 4);
    assert_eq!(ast.threat_models.len(), 1);
    assert_eq!(ast.threat_models[0].assets.len(), 2);
    assert_eq!(ast.threat_models[0].threats.len(), 2);
    assert_eq!(ast.threat_models[0].mitigations.len(), 2);
}

#[test]
fn v035_semantics_accept_valid_runtime_hardening_and_threat_model() {
    let ast = parse_source(&runtime_hardening_v035_source()).unwrap();
    check_program(&ast).expect("valid runtime hardening metadata must pass");
}

#[test]
fn v035_semantics_reject_unknown_hardening_profile() {
    let source = runtime_hardening_v035_source().replacen(
        "hardening_profile HardenedRuntime",
        "hardening_profile MissingRuntime",
        1,
    );
    let diagnostics = check(&source).expect_err("unknown hardening profile must fail");
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("unknown runtime_hardening_profile")),
        "{diagnostics:?}"
    );
}

#[test]
fn v035_ir_and_bytecode_preserve_runtime_hardening_and_v034_compatibility() {
    let ast = parse_source(&runtime_hardening_v035_source()).unwrap();
    check_program(&ast).unwrap();
    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(ir.runtime_hardening_profiles.len(), 1);
    assert_eq!(ir.threat_models.len(), 1);
    assert_eq!(ir.threat_models[0].threats.len(), 2);
    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(bytecode.runtime_hardening_profiles.len(), 1);
    assert_eq!(bytecode.threat_models.len(), 1);
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();
    let mut legacy = bytecode;
    legacy.bytecode_version = "0.34".into();
    legacy.runtime_hardening_profiles.clear();
    legacy.threat_models.clear();
    argorix_bytecode::verify_bytecode(&legacy).unwrap();
}

#[test]
fn v035_vm_trace_preserves_runtime_hardening_without_enabling_runtime() {
    use argorix_vm::Vm;
    let ast = parse_source(&runtime_hardening_v035_source()).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let trace = Vm::new()
        .run_reactive(
            &bytecode,
            argorix_vm::InjectedMessage {
                from: "User".into(),
                to: "ResearchAgent".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
        )
        .unwrap();
    assert_eq!(trace.vm_version, "0.36");
    assert_eq!(trace.runtime_hardening_profiles.len(), 1);
    assert_eq!(trace.threat_models.len(), 1);
    let events = serde_json::to_string(&trace.events).unwrap();
    for expected in [
        "RuntimeHardeningProfileDeclared",
        "RuntimeDenyByDefaultDeclared",
        "RuntimeSandboxRequired",
        "RuntimeNetworkDenied",
        "RuntimeSecretsDenied",
        "RuntimeExecutionDisabled",
        "ThreatModelDeclared",
        "ThreatAssetsMapped",
        "ThreatsMapped",
        "MitigationsMapped",
        "ThreatModelRuntimeDisabled",
        "ThreatModelSecurityClaimsDenied",
    ] {
        assert!(events.contains(expected), "missing {expected}");
    }
    for forbidden in [
        "RuntimeEnabled",
        "ProviderCalled",
        "NetworkRequest",
        "ApiKeyLoaded",
        "EnvRead",
        "ToolExecuted",
        "AgentExecuted",
        "AttackSimulated",
        "ExploitExecuted",
        "SecurityCertified",
        "RiskEliminated",
    ] {
        assert!(!events.contains(forbidden), "leaked {forbidden}");
    }
}

#[test]
fn v035_security_report_evidence_and_policy_cover_runtime_hardening() {
    use argorix_vm::{EvidenceBundle, SecurityReport, Vm};
    let ast = parse_source(&runtime_hardening_v035_source()).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        argorix_vm::InjectedMessage {
            from: "User".into(),
            to: "ResearchAgent".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );
    let trace = outcome.result.as_ref().unwrap();
    let policy = trace
        .policy_report
        .policy_blocks
        .iter()
        .find(|policy| policy.name == "RuntimeHardeningPolicy")
        .expect("runtime hardening policy is evaluated");
    assert!(policy.passed, "{policy:?}");
    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.report_version, "0.36");
    assert_eq!(report.runtime_hardening_profiles.total, 1);
    assert_eq!(report.runtime_hardening_profiles.protected_assets_total, 4);
    assert_eq!(report.runtime_hardening_profiles.network["denied"], 1);
    assert_eq!(report.threat_models.total, 1);
    assert_eq!(report.threat_models.assets_total, 2);
    assert_eq!(report.threat_models.threats_total, 2);
    assert_eq!(report.threat_models.mitigations_total, 2);
    let bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        std::path::Path::new("target/evidence-v035/bundle.json"),
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(bundle.bundle_version, "0.36");
}

#[test]
fn invalid_runtime_hardening_fixtures_are_complete_and_all_fail() {
    use std::collections::BTreeSet;
    const EXPECTED: &[&str] = &[
        "unknown_hardening_evidence_map.argx",
        "unknown_hardening_governance_profile.argx",
        "unknown_hardening_public_conformance_report.argx",
        "provider_execution_enabled.argx",
        "external_providers_enabled.argx",
        "network_allowed.argx",
        "tool_execution_enabled.argx",
        "agent_execution_enabled.argx",
        "filesystem_access_allowed.argx",
        "env_access_allowed.argx",
        "secret_material_allowed.argx",
        "key_material_allowed.argx",
        "deny_by_default_false.argx",
        "audit_log_optional.argx",
        "evidence_optional.argx",
        "residual_risk_eliminated.argx",
        "review_status_regulator_approved.argx",
        "assurance_legally_certified.argx",
        "security_claim_secure.argx",
        "empty_protected_assets.argx",
        "empty_runtime_boundaries.argx",
        "unknown_threat_hardening_profile.argx",
        "threat_hardening_evidence_mismatch.argx",
        "empty_assets.argx",
        "duplicate_asset_id.argx",
        "empty_threats.argx",
        "duplicate_threat_id.argx",
        "empty_mitigations.argx",
        "duplicate_mitigation_id.argx",
        "threat_network_allowed.argx",
        "threat_external_execution_enabled.argx",
        "threat_tool_execution_enabled.argx",
        "threat_agent_execution_enabled.argx",
        "threat_secret_material_allowed.argx",
        "threat_key_material_allowed.argx",
        "threat_execution_enabled.argx",
        "risk_acceptance_legally_accepted.argx",
        "threat_security_claim_risk_free.argx",
        "duplicate_runtime_hardening_profile_name.argx",
        "duplicate_threat_model_name.argx",
    ];
    let directory = std::path::Path::new("examples/invalid_runtime_hardening");
    let actual: BTreeSet<String> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let expected: BTreeSet<String> = EXPECTED.iter().map(|name| (*name).into()).collect();
    assert!(
        !actual.is_empty(),
        "invalid fixture directory must not be empty"
    );
    assert_eq!(actual, expected, "invalid fixture inventory drift");
    for name in EXPECTED {
        let source = std::fs::read_to_string(directory.join(name)).unwrap();
        let passes = parse_source(&source)
            .ok()
            .is_some_and(|program| check_program(&program).is_ok());
        assert!(!passes, "invalid fixture `{name}` unexpectedly passed");
    }
}

#[test]
fn v035_single_file_and_bytecode_fixture_match() {
    let source = include_str!("../examples/runtime_hardening_v035.argx");
    let ast = parse_source(source).unwrap();
    check_program(&ast).unwrap();
    let mut emitted = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    emitted.bytecode_version = "0.35".into();
    let fixture: argorix_bytecode::BytecodeProgram = serde_json::from_str(include_str!(
        "../examples/runtime_hardening_v035.argbc.json"
    ))
    .unwrap();
    assert_eq!(emitted, fixture);
    argorix_bytecode::verify_bytecode(&fixture).unwrap();
}

// ---------------------------------------------------------------------------
// v0.36 — Spec Freeze + v1.0 Release Candidate
// ---------------------------------------------------------------------------

const SPEC_FREEZE_V036_DECLS: &str = r#"
spec_freeze ArgorixSpecFreezeV036 {
  version "0.36"
  target "v1.0-rc"
  freeze_scope language
  compatibility cumulative
  stability rc_candidate
  frozen_features ["agent", "provider_contract", "module_system", "policy_v2", "message_contracts", "agent_passport", "sandboxed_provider_harness", "feature_flags", "secret_boundary", "adapter_framework", "adapter_profiles", "crypto_registry", "crypto_boundary", "did_methods", "atrust_boundary", "atrust_identity", "atrust_credential_contract", "atrust_handshake", "trust_ledger", "mcp_bridge_contract", "a2a_bridge_contract", "atrust_evidence_map", "governance_profile", "regulatory_mapping", "third_party_verifier", "public_conformance_report", "runtime_hardening_profile", "threat_model"]
  compatible_versions ["0.29", "0.30", "0.31", "0.32", "0.33", "0.34", "0.35", "0.36"]
  required_suites ["conformance/suite.v034.json", "conformance/suite.v035.json", "conformance/suite.v036.json"]
  evidence_bundle required
  security_report required
  conformance required
  backward_compatibility required
  runtime_status disabled
  network denied
  external_execution disabled
  provider_execution disabled
  secret_material denied
  key_material denied
  env_access denied
  filesystem_access denied
  tool_execution disabled
  agent_execution disabled
  security_claims none
  legal_claims none
  certification none
  purpose ["spec-freeze", "v1-rc", "compatibility", "audit"]
  notes "metadata only; spec freeze does not enable runtime"
}

release_candidate ArgorixV1RC {
  version "1.0.0-rc.1"
  base_version "0.36"
  spec_freeze ArgorixSpecFreezeV036
  readiness rc
  required_artifacts ["README.md", "conformance/suite.v036.json", "examples/spec_freeze_v036.argx", "examples/spec_freeze_v036.argbc.json"]
  required_checks ["cargo fmt --all --check", "cargo clippy --workspace --all-targets --all-features", "cargo test --workspace", "cargo test --workspace --no-run", "argorix-conformance suite.v036"]
  compatibility_matrix [
    { version "0.34" bytecode compatible evidence compatible conformance compatible },
    { version "0.35" bytecode compatible evidence compatible conformance compatible },
    { version "0.36" bytecode native evidence native conformance native }
  ]
  known_limitations ["runtime remains disabled", "network remains denied", "OpenAI API support is external sandbox only, not core", "MCP/A2A are contracts only", "DID/VC/credential/handshake verification remains declared-only"]
  runtime_status disabled
  network denied
  external_execution disabled
  provider_execution disabled
  secret_material denied
  key_material denied
  env_access denied
  filesystem_access denied
  tool_execution disabled
  agent_execution disabled
  security_claims none
  legal_claims none
  certification none
  purpose ["v1-release-candidate", "spec-freeze", "audit", "public-conformance"]
  notes "release candidate metadata only; not production certification"
}

policy ReleaseCandidatePolicy {
  require spec_freezes_declared
  require spec_freeze_versions_pinned
  require spec_freeze_features_declared
  require spec_freeze_compatibility_declared
  require spec_freeze_required_suites_declared
  require spec_freeze_runtime_disabled
  require spec_freeze_network_denied
  require spec_freeze_external_execution_disabled
  require spec_freeze_provider_execution_disabled
  require spec_freeze_secret_material_denied
  require spec_freeze_key_material_denied
  require spec_freeze_env_denied
  require spec_freeze_filesystem_denied
  require spec_freeze_security_claims_absent
  require spec_freeze_legal_claims_absent
  require spec_freeze_certification_absent
  require release_candidates_declared
  require release_candidates_spec_freeze_bound
  require release_candidates_artifacts_declared
  require release_candidates_checks_declared
  require release_candidates_compatibility_matrix_declared
  require release_candidates_limitations_declared
  require release_candidates_runtime_disabled
  require release_candidates_network_denied
  require release_candidates_external_execution_disabled
  require release_candidates_provider_execution_disabled
  require release_candidates_secret_material_denied
  require release_candidates_key_material_denied
  require release_candidates_env_denied
  require release_candidates_filesystem_denied
  require release_candidates_security_claims_absent
  require release_candidates_legal_claims_absent
  require release_candidates_certification_absent
  on violation {
    action review
    trace required
  }
}
"#;

fn spec_freeze_v036_source() -> String {
    format!(
        "{}\n{}",
        runtime_hardening_v035_source(),
        SPEC_FREEZE_V036_DECLS
    )
}

#[test]
fn v036_parses_spec_freeze_and_release_candidate() {
    let ast = parse_source(&spec_freeze_v036_source()).unwrap();
    assert_eq!(ast.spec_freezes.len(), 1);
    assert_eq!(ast.spec_freezes[0].frozen_features.len(), 28);
    assert_eq!(ast.release_candidates.len(), 1);
    assert_eq!(ast.release_candidates[0].compatibility_matrix.len(), 3);
}

#[test]
fn v036_semantics_accept_valid_spec_freeze_and_release_candidate() {
    let ast = parse_source(&spec_freeze_v036_source()).unwrap();
    check_program(&ast).unwrap();
}

#[test]
fn v036_semantics_reject_invalid_freeze_and_release_boundaries() {
    for (from, to) in [
        ("version \"0.36\"", "version \"0.37\""),
        ("stability rc_candidate", "stability stable_final"),
        ("runtime_status disabled", "runtime_status enabled"),
        ("readiness rc", "readiness production"),
        (
            "spec_freeze ArgorixSpecFreezeV036",
            "spec_freeze MissingFreeze",
        ),
    ] {
        let source = spec_freeze_v036_source().replacen(from, to, 1);
        let ast = parse_source(&source).unwrap();
        assert!(
            check_program(&ast).is_err(),
            "mutation unexpectedly passed: {to}"
        );
    }
}

#[test]
fn v036_ir_bytecode_and_v035_compatibility_preserve_release_metadata() {
    let ast = parse_source(&spec_freeze_v036_source()).unwrap();
    check_program(&ast).unwrap();
    let ir = IrProgram::from(&ast);
    assert_eq!(ir.ir_version, "0.36");
    assert_eq!(ir.spec_freezes.len(), 1);
    assert_eq!(ir.release_candidates.len(), 1);
    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.36");
    assert_eq!(bytecode.spec_freezes.len(), 1);
    assert_eq!(bytecode.release_candidates.len(), 1);
    argorix_bytecode::verify_bytecode(&bytecode).unwrap();
    let mut legacy = bytecode;
    legacy.bytecode_version = "0.35".into();
    legacy.spec_freezes.clear();
    legacy.release_candidates.clear();
    argorix_bytecode::verify_bytecode(&legacy).unwrap();
}

#[test]
fn v036_vm_report_evidence_and_policy_preserve_declarative_rc_boundaries() {
    use argorix_vm::{EvidenceBundle, SecurityReport, Vm};
    let ast = parse_source(&spec_freeze_v036_source()).unwrap();
    check_program(&ast).unwrap();
    let bytecode = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let outcome = Vm::new().run_reactive_outcome(
        &bytecode,
        argorix_vm::InjectedMessage {
            from: "User".into(),
            to: "ResearchAgent".into(),
            act: "tell".into(),
            message_type: "UserPrompt".into(),
        },
    );
    let trace = outcome.result.as_ref().unwrap();
    assert_eq!(trace.vm_version, "0.36");
    assert_eq!(trace.spec_freezes.len(), 1);
    assert_eq!(trace.release_candidates.len(), 1);
    let events = serde_json::to_string(&trace.events).unwrap();
    for expected in [
        "SpecFreezeDeclared",
        "SpecFreezeCompatibilityDeclared",
        "SpecFreezeRuntimeDisabled",
        "ReleaseCandidateDeclared",
        "ReleaseCandidateArtifactsMapped",
        "ReleaseCandidateCompatibilityMapped",
        "ReleaseCandidateRuntimeDisabled",
        "ReleaseCandidateSecurityClaimsDenied",
    ] {
        assert!(events.contains(expected), "missing {expected}");
    }
    for forbidden in [
        "RuntimeEnabled",
        "ProviderCalled",
        "NetworkRequest",
        "ApiKeyLoaded",
        "EnvRead",
        "ToolExecuted",
        "AgentExecuted",
        "ReleaseCertified",
        "ProductionReadyCertified",
        "ComplianceCertified",
    ] {
        assert!(!events.contains(forbidden), "leaked {forbidden}");
    }
    let policy = trace
        .policy_report
        .policy_blocks
        .iter()
        .find(|policy| policy.name == "ReleaseCandidatePolicy")
        .unwrap();
    assert!(policy.passed, "{policy:?}");
    let report = SecurityReport::from_outcome(&bytecode, &outcome);
    assert_eq!(report.report_version, "0.36");
    assert_eq!(report.spec_freezes.total, 1);
    assert_eq!(report.spec_freezes.frozen_features_total, 28);
    assert_eq!(report.release_candidates.total, 1);
    assert_eq!(
        report.release_candidates.compatibility_matrix_versions["0.36"],
        1
    );
    let bundle = EvidenceBundle::from_outcome(
        &bytecode,
        &outcome,
        &report,
        std::path::Path::new("target/evidence-v036/bundle.json"),
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!(bundle.bundle_version, "0.36");
}

#[test]
fn invalid_spec_freeze_fixtures_are_complete_and_all_fail() {
    use std::collections::BTreeSet;
    const EXPECTED: &[&str] = &[
        "spec_version_wrong.argx",
        "spec_stability_stable_final.argx",
        "spec_empty_frozen_features.argx",
        "spec_missing_compatible_034.argx",
        "spec_missing_compatible_035.argx",
        "spec_missing_compatible_036.argx",
        "spec_missing_suite_v036.argx",
        "spec_runtime_enabled.argx",
        "spec_network_allowed.argx",
        "spec_external_execution_enabled.argx",
        "spec_provider_execution_enabled.argx",
        "spec_secret_material_allowed.argx",
        "spec_key_material_allowed.argx",
        "spec_env_access_allowed.argx",
        "spec_filesystem_access_allowed.argx",
        "spec_tool_execution_enabled.argx",
        "spec_agent_execution_enabled.argx",
        "spec_security_claim_secure.argx",
        "spec_legal_claims_compliant.argx",
        "spec_certification_regulator_approved.argx",
        "rc_unknown_spec_freeze.argx",
        "rc_base_version_wrong.argx",
        "rc_readiness_production.argx",
        "rc_empty_required_artifacts.argx",
        "rc_empty_required_checks.argx",
        "rc_missing_matrix_034.argx",
        "rc_missing_matrix_035.argx",
        "rc_missing_matrix_036.argx",
        "rc_empty_known_limitations.argx",
        "rc_runtime_enabled.argx",
        "rc_network_allowed.argx",
        "rc_external_execution_enabled.argx",
        "rc_provider_execution_enabled.argx",
        "rc_secret_material_allowed.argx",
        "rc_key_material_allowed.argx",
        "rc_env_access_allowed.argx",
        "rc_filesystem_access_allowed.argx",
        "rc_tool_execution_enabled.argx",
        "rc_agent_execution_enabled.argx",
        "rc_security_claim_certified.argx",
        "rc_legal_claims_certified.argx",
        "rc_certification_iso_certified.argx",
        "duplicate_spec_freeze_name.argx",
        "duplicate_release_candidate_name.argx",
    ];
    let directory = std::path::Path::new("examples/invalid_spec_freeze");
    let actual: BTreeSet<String> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let expected: BTreeSet<String> = EXPECTED.iter().map(|name| (*name).into()).collect();
    assert!(
        !actual.is_empty(),
        "invalid fixture directory must not be empty"
    );
    assert_eq!(actual, expected, "invalid fixture inventory drift");
    for name in EXPECTED {
        let source = std::fs::read_to_string(directory.join(name)).unwrap();
        let passes = parse_source(&source)
            .ok()
            .is_some_and(|program| check_program(&program).is_ok());
        assert!(!passes, "invalid fixture `{name}` unexpectedly passed");
    }
}

#[test]
fn v036_single_file_and_bytecode_fixture_match() {
    let source = include_str!("../examples/spec_freeze_v036.argx");
    let ast = parse_source(source).unwrap();
    check_program(&ast).unwrap();
    let emitted = argorix_bytecode::lower_ir(&IrProgram::from(&ast));
    let fixture: argorix_bytecode::BytecodeProgram =
        serde_json::from_str(include_str!("../examples/spec_freeze_v036.argbc.json")).unwrap();
    assert_eq!(emitted, fixture);
    argorix_bytecode::verify_bytecode(&fixture).unwrap();
}
