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

    assert_eq!(json["ir_version"], "0.14");
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
    assert_eq!(json["ir_version"], "0.14");
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
    assert_eq!(ir.ir_version, "0.14");
    assert_eq!(
        ir.providers[0].allowed_targets,
        vec!["GuardModel", "WebSearch"]
    );
    assert_eq!(
        ir.providers[0].allowed_capabilities,
        vec!["model.invoke", "web.search"]
    );
    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.14");
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
    assert_eq!(ir.ir_version, "0.14");
    assert_eq!(ir.providers[0].name, "OpenAI");
    assert_eq!(ir.providers[0].kind, "external");
    assert!(ir.providers[0].allowed_targets.is_empty());
    assert!(ir.providers[0].allowed_capabilities.is_empty());

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.14");
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
    assert_eq!(ir.ir_version, "0.14");
    assert_eq!(ir.tools[0].provider, "simulated");
    assert_eq!(ir.models.len(), 0);

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.14");
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
    assert_eq!(json["ir_version"], "0.14");
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
    assert_eq!(json["ir_version"], "0.14");
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
    assert_eq!(json["ir_version"], "0.14");
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
