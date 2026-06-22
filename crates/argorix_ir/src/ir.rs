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
    pub provider_harnesses: Vec<IrProviderHarness>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<IrFeature>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<IrSecret>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapters: Vec<IrAdapter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapter_profiles: Vec<IrAdapterProfile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cryptos: Vec<IrCrypto>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passports: Vec<IrPassport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPassport {
    pub name: String,
    pub agent: String,
    pub agent_name: String,
    pub global_id: String,
    pub identity: String,
    pub provider: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ans_name: Option<String>,
    pub country: String,
    pub jurisdiction: String,
    pub data_residency: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asn: Option<IrPassportAsn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub risk_level: String,
    pub data_scope: Vec<String>,
    pub intent: String,
    pub intended_use: Vec<String>,
    pub prohibited_use: Vec<String>,
    pub attestations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrPassportAsn {
    pub registry: String,
    pub number: String,
    pub holder: String,
    pub country: String,
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
pub struct IrFeature {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub status: String,
    pub default: String,
    pub requires_approval: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrSecret {
    pub name: String,
    pub handle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_by: Option<String>,
    pub scope: String,
    pub access: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrAdapter {
    pub name: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    pub mode: String,
    pub execution: String,
    pub network: String,
    pub secrets: String,
    pub filesystem: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<String>,
    pub conformance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrAdapterProfile {
    pub name: String,
    pub adapter: String,
    pub provider: String,
    pub vendor: String,
    pub family: String,
    pub api_style: String,
    pub auth: String,
    pub execution: String,
    pub network: String,
    pub secrets: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_contract: Option<String>,
    pub capabilities: Vec<String>,
    pub required_conformance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrCrypto {
    pub name: String,
    pub kind: String,
    pub status: String,
    pub strength: String,
    pub purpose: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_bits: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_key_bits: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IrProviderHarness {
    pub name: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    pub mode: String,
    pub network: String,
    pub secrets: String,
    pub filesystem: String,
    pub max_steps: Option<u64>,
    pub timeout_ms: Option<u64>,
    pub input_contract: Option<String>,
    pub output_contract: Option<String>,
    pub attestations: Vec<String>,
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
            ir_version: "0.24".to_owned(),
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
            provider_harnesses: program
                .harnesses
                .iter()
                .map(|harness| IrProviderHarness {
                    name: harness.name.value.clone(),
                    provider: harness.provider.value.clone(),
                    feature: harness.feature.as_ref().map(|value| value.value.clone()),
                    secret: harness.secret.as_ref().map(|value| value.value.clone()),
                    mode: harness.mode.value.source_name().to_owned(),
                    network: harness.network.value.source_name().to_owned(),
                    secrets: harness.secrets.value.source_name().to_owned(),
                    filesystem: harness.filesystem.value.source_name().to_owned(),
                    max_steps: harness.max_steps.as_ref().map(|value| value.value),
                    timeout_ms: harness.timeout_ms.as_ref().map(|value| value.value),
                    input_contract: harness
                        .input_contract
                        .as_ref()
                        .map(|value| value.value.clone()),
                    output_contract: harness
                        .output_contract
                        .as_ref()
                        .map(|value| value.value.clone()),
                    attestations: spanned_values(&harness.attestations),
                })
                .collect(),
            features: program
                .features
                .iter()
                .map(|feature| IrFeature {
                    name: feature.name.value.clone(),
                    provider: feature.provider.as_ref().map(|value| value.value.clone()),
                    status: feature.status.value.source_name().to_owned(),
                    default: feature.default.value.source_name().to_owned(),
                    requires_approval: feature.requires_approval,
                    purpose: feature.purpose.as_ref().map(|value| value.value.clone()),
                })
                .collect(),
            secrets: program
                .secrets
                .iter()
                .map(|secret| IrSecret {
                    name: secret.name.value.clone(),
                    handle: secret.handle.value.clone(),
                    provider: secret.provider.as_ref().map(|value| value.value.clone()),
                    required_by: secret.required_by.as_ref().map(|value| value.value.clone()),
                    scope: secret.scope.value.source_name().to_owned(),
                    access: secret.access.value.source_name().to_owned(),
                    source: secret.source.value.source_name().to_owned(),
                })
                .collect(),
            adapters: program
                .adapters
                .iter()
                .map(|adapter| IrAdapter {
                    name: adapter.name.value.clone(),
                    provider: adapter.provider.value.clone(),
                    feature: adapter.feature.as_ref().map(|v| v.value.clone()),
                    secret: adapter.secret.as_ref().map(|v| v.value.clone()),
                    harness: adapter.harness.as_ref().map(|v| v.value.clone()),
                    kind: adapter
                        .kind
                        .as_ref()
                        .map(|v| v.value.source_name().to_owned()),
                    vendor: adapter.vendor.as_ref().map(|v| v.value.clone()),
                    mode: adapter.mode.value.source_name().to_owned(),
                    execution: adapter.execution.value.source_name().to_owned(),
                    network: adapter.network.value.source_name().to_owned(),
                    secrets: adapter.secrets.value.source_name().to_owned(),
                    filesystem: adapter.filesystem.value.source_name().to_owned(),
                    input_contract: adapter.input_contract.as_ref().map(|v| v.value.clone()),
                    output_contract: adapter.output_contract.as_ref().map(|v| v.value.clone()),
                    conformance: adapter
                        .conformance
                        .iter()
                        .map(|v| v.value.clone())
                        .collect(),
                })
                .collect(),
            adapter_profiles: program
                .adapter_profiles
                .iter()
                .map(|p| IrAdapterProfile {
                    name: p.name.value.clone(),
                    adapter: p.adapter.value.clone(),
                    provider: p.provider.value.clone(),
                    vendor: p.vendor.value.clone(),
                    family: p.family.value.source_name().to_owned(),
                    api_style: p.api_style.value.source_name().to_owned(),
                    auth: p.auth.value.source_name().to_owned(),
                    execution: p.execution.value.source_name().to_owned(),
                    network: p.network.value.source_name().to_owned(),
                    secrets: p.secrets.value.source_name().to_owned(),
                    request_contract: p.request_contract.as_ref().map(|v| v.value.clone()),
                    response_contract: p.response_contract.as_ref().map(|v| v.value.clone()),
                    capabilities: p.capabilities.iter().map(|v| v.value.clone()).collect(),
                    required_conformance: p
                        .required_conformance
                        .iter()
                        .map(|v| v.value.clone())
                        .collect(),
                })
                .collect(),
            cryptos: program
                .cryptos
                .iter()
                .map(|c| IrCrypto {
                    name: c.name.value.clone(),
                    kind: c.kind.value.source_name().to_owned(),
                    status: c.status.value.source_name().to_owned(),
                    strength: c.strength.value.source_name().to_owned(),
                    purpose: c.purpose.iter().map(|v| v.value.clone()).collect(),
                    output_bits: c.output_bits.as_ref().map(|v| v.value),
                    min_key_bits: c.min_key_bits.as_ref().map(|v| v.value),
                    security_level: c.security_level.as_ref().map(|v| v.value.clone()),
                    notes: c.notes.as_ref().map(|v| v.value.clone()),
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
                            field_type: field.field_type.value.source_name().to_owned(),
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
            passports: program
                .passports
                .iter()
                .map(|passport| IrPassport {
                    name: passport.name.value.clone(),
                    agent: passport.agent.value.clone(),
                    agent_name: passport.agent_name.value.clone(),
                    global_id: passport.global_id.value.clone(),
                    identity: passport.identity.value.clone(),
                    provider: passport.provider.value.clone(),
                    version: passport.version.value.clone(),
                    ans_name: passport.ans_name.as_ref().map(|value| value.value.clone()),
                    country: passport.country.value.clone(),
                    jurisdiction: passport.jurisdiction.value.clone(),
                    data_residency: spanned_values(&passport.data_residency),
                    asn: passport.asn.as_ref().map(|asn| IrPassportAsn {
                        registry: asn.registry.value.clone(),
                        number: asn.number.value.clone(),
                        holder: asn.holder.value.clone(),
                        country: asn.country.value.clone(),
                    }),
                    model: passport.model.as_ref().map(|value| value.value.clone()),
                    risk_level: passport.risk_level.value.clone(),
                    data_scope: spanned_values(&passport.data_scope),
                    intent: passport.intent.value.clone(),
                    intended_use: spanned_values(&passport.intended_use),
                    prohibited_use: spanned_values(&passport.prohibited_use),
                    attestations: spanned_values(&passport.attestations),
                })
                .collect(),
        }
    }
}

fn spanned_values(values: &[argorix_parser::span::Spanned<String>]) -> Vec<String> {
    values.iter().map(|value| value.value.clone()).collect()
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
        assert_eq!(ir.ir_version, "0.21");
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

    #[test]
    fn ir_019_preserves_passport_metadata() {
        let program = parse_source(
            r#"
            module main
            agent ResearchAgent {}
            passport RiskAnalyzerPassport {
                agent ResearchAgent
                agent_name "Risk Analyzer"
                global_id "argx:agent:1"
                identity "did:argorix:risk-v1"
                provider "Argorix"
                version "1.0.0"
                ans_name "argx://risk.v1.sovereign"
                country "CL"
                jurisdiction "CL"
                data_residency ["CL", "EU"]
                asn { registry "LACNIC" number "AS-PLACEHOLDER" holder "Argorix Labs" country "CL" }
                risk_level "high"
                intent "risk_analysis"
                attestations ["redteam"]
            }
            "#,
        )
        .unwrap();
        let ir = IrProgram::from(&program);
        assert_eq!(ir.ir_version, "0.21");
        assert_eq!(ir.passports.len(), 1);
        assert_eq!(ir.passports[0].agent, "ResearchAgent");
        assert_eq!(ir.passports[0].data_residency, vec!["CL", "EU"]);
        assert_eq!(ir.passports[0].asn.as_ref().unwrap().registry, "LACNIC");
        assert_eq!(
            ir.passports[0].ans_name.as_deref(),
            Some("argx://risk.v1.sovereign")
        );
    }

    #[test]
    fn ir_020_preserves_provider_harness_metadata() {
        let program = parse_source(
            r#"
            module main
            provider OpenAI { kind external enabled false dry_run_only true requires feature_flag requires approval }
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
                attestations ["dry-run"]
            }
            "#,
        )
        .unwrap();
        let ir = IrProgram::from(&program);
        assert_eq!(ir.ir_version, "0.21");
        assert_eq!(ir.provider_harnesses.len(), 1);
        let harness = &ir.provider_harnesses[0];
        assert_eq!(harness.name, "OpenAIHarness");
        assert_eq!(harness.mode, "dry_run");
        assert_eq!(harness.max_steps, Some(10));
        assert_eq!(harness.input_contract.as_deref(), Some("UserPrompt"));
        assert_eq!(harness.attestations, vec!["dry-run"]);
    }
}
