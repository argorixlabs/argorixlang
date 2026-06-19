use argorix_parser::ast::{HandlerInstruction, Program};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrProgram {
    pub ir_version: String,
    pub language: String,
    pub module: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<IrModule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<IrModuleImport>,
    pub providers: Vec<IrProviderContract>,
    pub assertions: Vec<IrAssertion>,
    pub policies: Vec<IrPolicy>,
    pub failures: Vec<IrFailure>,
    pub capabilities: Vec<IrCapability>,
    pub enums: Vec<IrEnum>,
    pub types: Vec<IrType>,
    pub tools: Vec<IrTool>,
    pub models: Vec<IrModel>,
    pub agents: Vec<IrAgent>,
    pub protocols: Vec<IrProtocol>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrModule {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrModuleImport {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrProviderContract {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub dry_run_only: bool,
    pub requires_feature_flag: bool,
    pub requires_explicit_approval: bool,
    pub allowed_targets: Vec<String>,
    pub allowed_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrAssertion {
    pub name: String,
    pub argument: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPolicy {
    pub name: String,
    pub rules: Vec<IrPolicyRule>,
    pub on_violation: Option<IrPolicyViolation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPolicyRule {
    pub effect: String,
    pub rule: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPolicyViolation {
    pub action: String,
    pub trace_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrFailure {
    pub name: String,
    pub action: String,
    pub trace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrModel {
    pub name: String,
    pub provider: String,
    pub capability: String,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrTool {
    pub name: String,
    pub provider: String,
    pub capability: String,
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrCapability {
    pub name: String,
    pub level: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrEnum {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrType {
    pub name: String,
    pub fields: Vec<IrField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrAgent {
    pub name: String,
    pub approval: String,
    pub receives: Vec<IrReceive>,
    pub sends: Vec<IrSend>,
    pub capabilities: Vec<String>,
    pub tools: Vec<String>,
    pub models: Vec<String>,
    pub handlers: Vec<IrHandler>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrHandler {
    pub message_type: String,
    pub binding: String,
    pub instructions: Vec<IrHandlerInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum IrHandlerInstruction {
    Emit { message_type: String, to: String },
    Trace { binding: String },
    Halt,
    Intrinsic { name: String, argument: String },
    Call { tool: String, binding: String },
    Ask { model: String, binding: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrReceive {
    pub message_type: String,
    pub from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrSend {
    pub message_type: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrProtocol {
    pub name: String,
    pub steps: Vec<IrProtocolStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrProtocolStep {
    pub from: String,
    pub to: String,
    pub act: String,
    pub message_type: String,
}

impl From<&Program> for IrProgram {
    fn from(program: &Program) -> Self {
        Self {
            ir_version: "0.17".to_owned(),
            language: "Argorix Lang".to_owned(),
            module: program.module.value.clone(),
            modules: Vec::new(),
            imports: Vec::new(),
            providers: program
                .providers
                .iter()
                .map(|provider| IrProviderContract {
                    name: provider.name.value.clone(),
                    kind: provider.kind.value.as_str().into(),
                    enabled: provider.enabled.value,
                    dry_run_only: provider.dry_run_only.value,
                    requires_feature_flag: provider.requires_feature_flag,
                    requires_explicit_approval: provider.requires_explicit_approval,
                    allowed_targets: provider
                        .allowed_targets
                        .iter()
                        .map(|item| item.value.clone())
                        .collect(),
                    allowed_capabilities: provider
                        .allowed_capabilities
                        .iter()
                        .map(|item| item.value.clone())
                        .collect(),
                })
                .collect(),
            assertions: program
                .assertions
                .iter()
                .map(|assertion| IrAssertion {
                    name: assertion.name.value.clone(),
                    argument: assertion.argument.as_ref().map(|value| value.value.clone()),
                })
                .collect(),
            policies: program
                .policies
                .iter()
                .map(|policy| IrPolicy {
                    name: policy.name.value.clone(),
                    rules: policy
                        .rules
                        .iter()
                        .map(|declaration| IrPolicyRule {
                            effect: declaration.effect().into(),
                            rule: declaration.rule().value.source_name(),
                        })
                        .collect(),
                    on_violation: policy
                        .violation
                        .as_ref()
                        .map(|violation| IrPolicyViolation {
                            action: violation.action.value.source_name(),
                            trace_required: violation.trace_required,
                        }),
                })
                .collect(),
            failures: program
                .failures
                .iter()
                .map(|failure| IrFailure {
                    name: failure.name.value.clone(),
                    action: failure.action.value.clone(),
                    trace: "required".into(),
                })
                .collect(),
            capabilities: program
                .capabilities
                .iter()
                .map(|capability| IrCapability {
                    name: capability.name.value.clone(),
                    level: capability.level.value.as_str().to_owned(),
                    requires_approval: capability.requires_approval,
                })
                .collect(),
            enums: program
                .enums
                .iter()
                .map(|item| IrEnum {
                    name: item.name.value.clone(),
                    variants: item
                        .variants
                        .iter()
                        .map(|variant| variant.value.clone())
                        .collect(),
                })
                .collect(),
            types: program
                .types
                .iter()
                .map(|item| IrType {
                    name: item.name.value.clone(),
                    fields: item
                        .fields
                        .iter()
                        .map(|field| IrField {
                            name: field.name.value.clone(),
                            field_type: field.field_type.value.clone(),
                        })
                        .collect(),
                })
                .collect(),
            tools: program
                .tools
                .iter()
                .map(|tool| IrTool {
                    name: tool.name.value.clone(),
                    provider: resolved_provider(
                        tool.provider
                            .as_ref()
                            .map(|provider| provider.value.as_str()),
                    )
                    .to_owned(),
                    capability: tool.capability.value.clone(),
                    input: tool.input.value.clone(),
                    output: tool.output.value.clone(),
                })
                .collect(),
            models: program
                .models
                .iter()
                .map(|model| IrModel {
                    name: model.name.value.clone(),
                    provider: model.provider.value.clone(),
                    capability: model.capability.value.clone(),
                    input: model.input.value.clone(),
                    output: model.output.value.clone(),
                })
                .collect(),
            agents: program
                .agents
                .iter()
                .map(|agent| IrAgent {
                    name: agent.name.value.clone(),
                    approval: agent.effective_approval().as_str().to_owned(),
                    receives: agent
                        .receives
                        .iter()
                        .map(|receive| IrReceive {
                            message_type: receive.message_type.value.clone(),
                            from: receive.from.as_ref().map(|from| from.value.clone()),
                        })
                        .collect(),
                    sends: agent
                        .sends
                        .iter()
                        .map(|send| IrSend {
                            message_type: send.message_type.value.clone(),
                            to: send.to.value.clone(),
                        })
                        .collect(),
                    capabilities: agent
                        .capabilities
                        .iter()
                        .map(|capability| capability.value.clone())
                        .collect(),
                    tools: agent.tools.iter().map(|tool| tool.value.clone()).collect(),
                    models: agent
                        .models
                        .iter()
                        .map(|model| model.value.clone())
                        .collect(),
                    handlers: agent
                        .handlers
                        .iter()
                        .map(|handler| IrHandler {
                            message_type: handler.message_type.value.clone(),
                            binding: handler.binding.value.clone(),
                            instructions: handler
                                .instructions
                                .iter()
                                .map(|instruction| match instruction {
                                    HandlerInstruction::Emit { message_type, to } => {
                                        IrHandlerInstruction::Emit {
                                            message_type: message_type.value.clone(),
                                            to: to.value.clone(),
                                        }
                                    }
                                    HandlerInstruction::Trace { binding } => {
                                        IrHandlerInstruction::Trace {
                                            binding: binding.value.clone(),
                                        }
                                    }
                                    HandlerInstruction::Halt { .. } => IrHandlerInstruction::Halt,
                                    HandlerInstruction::IntrinsicCall { name, argument } => {
                                        IrHandlerInstruction::Intrinsic {
                                            name: name.value.clone(),
                                            argument: argument.value.clone(),
                                        }
                                    }
                                    HandlerInstruction::CallTool { tool, binding } => {
                                        IrHandlerInstruction::Call {
                                            tool: tool.value.clone(),
                                            binding: binding.value.clone(),
                                        }
                                    }
                                    HandlerInstruction::AskModel { model, binding } => {
                                        IrHandlerInstruction::Ask {
                                            model: model.value.clone(),
                                            binding: binding.value.clone(),
                                        }
                                    }
                                })
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
            protocols: program
                .protocols
                .iter()
                .map(|protocol| IrProtocol {
                    name: protocol.name.value.clone(),
                    steps: protocol
                        .steps
                        .iter()
                        .map(|step| IrProtocolStep {
                            from: step.from.value.clone(),
                            to: step.to.value.clone(),
                            act: step.act.value.clone(),
                            message_type: step.message_type.value.clone(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

pub fn resolved_provider(provider: Option<&str>) -> &str {
    provider.unwrap_or("simulated")
}

#[cfg(test)]
mod tests {
    use super::IrProgram;
    use argorix_parser::parse_source;

    #[test]
    fn lowers_policy_v2_metadata_to_ir_017() {
        let program = parse_source(
            r#"
            module main
            assert all_tool_calls_traced
            policy ProviderSafety {
                deny external_execution
                on violation { action block trace required }
            }
            "#,
        )
        .unwrap();
        let ir = IrProgram::from(&program);
        assert_eq!(ir.ir_version, "0.17");
        assert_eq!(ir.assertions.len(), 1);
        assert_eq!(ir.policies[0].name, "ProviderSafety");
        assert_eq!(ir.policies[0].rules[0].effect, "deny");
        assert_eq!(ir.policies[0].rules[0].rule, "external_execution");
        assert_eq!(
            ir.policies[0].on_violation.as_ref().unwrap().action,
            "block"
        );
        assert!(ir.policies[0].on_violation.as_ref().unwrap().trace_required);
    }
}
