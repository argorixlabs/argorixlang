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

    assert_eq!(json["ir_version"], "0.6");
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
    assert_eq!(json["ir_version"], "0.6");
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
