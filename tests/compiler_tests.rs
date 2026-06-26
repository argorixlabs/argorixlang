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

    assert_eq!(json["ir_version"], "0.32");
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
    assert_eq!(json["ir_version"], "0.32");
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
    assert_eq!(ir.ir_version, "0.32");
    assert_eq!(
        ir.providers[0].allowed_targets,
        vec!["GuardModel", "WebSearch"]
    );
    assert_eq!(
        ir.providers[0].allowed_capabilities,
        vec!["model.invoke", "web.search"]
    );
    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.32");
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
    assert_eq!(ir.ir_version, "0.32");
    assert_eq!(ir.providers[0].name, "OpenAI");
    assert_eq!(ir.providers[0].kind, "external");
    assert!(ir.providers[0].allowed_targets.is_empty());
    assert!(ir.providers[0].allowed_capabilities.is_empty());

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.32");
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
    assert_eq!(ir.ir_version, "0.32");
    assert_eq!(ir.tools[0].provider, "simulated");
    assert_eq!(ir.models.len(), 0);

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.32");
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
    assert_eq!(json["ir_version"], "0.32");
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
    assert_eq!(json["ir_version"], "0.32");
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
    assert_eq!(json["ir_version"], "0.32");
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

    assert_eq!(emitted.bytecode_version, "0.32");
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

    assert_eq!(emitted.bytecode_version, "0.32");
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
    assert_eq!(emitted.bytecode_version, "0.32");
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
    assert_eq!(emitted.bytecode_version, "0.32");
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
    assert_eq!(emitted.bytecode_version, "0.32");
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
    assert_eq!(trace.vm_version, "0.32");

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
    assert_eq!(report.report_version, "0.32");
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
    assert_eq!(ir.ir_version, "0.32");
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
    assert_eq!(bytecode.bytecode_version, "0.32");
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
    assert_eq!(v029.bytecode_version, "0.32");
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
    assert_eq!(report.report_version, "0.32");
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
    assert_eq!(ir.ir_version, "0.32");
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
    assert_eq!(bytecode.bytecode_version, "0.32");
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
    assert_eq!(v030.bytecode_version, "0.32");
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
    assert_eq!(report.report_version, "0.32");
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
    assert_eq!(ir.ir_version, "0.32");
    assert_eq!(ir.mcp_bridge_contracts.len(), 1);
    assert_eq!(ir.a2a_bridge_contracts.len(), 1);
    assert_eq!(ir.mcp_bridge_contracts[0].protocol, "mcp");
    assert_eq!(ir.a2a_bridge_contracts[0].protocol, "a2a");

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.32");
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
    assert_eq!(v031.bytecode_version, "0.32");
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
    assert_eq!(report.report_version, "0.32");
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
    assert_eq!(ir.ir_version, "0.32");
    assert_eq!(ir.atrust_evidence_maps.len(), 1);
    assert_eq!(ir.atrust_evidence_maps[0].mapping_mode, "declared_only");

    let bytecode = argorix_bytecode::lower_ir(&ir);
    assert_eq!(bytecode.bytecode_version, "0.32");
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
    assert_eq!(trace.vm_version, "0.32");
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
    assert_eq!(report.report_version, "0.32");
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
    assert_eq!(bundle.bundle_version, "0.32");

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
    assert_eq!(v032.bytecode_version, "0.32");
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
