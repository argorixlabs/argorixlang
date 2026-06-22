use crate::{
    BytecodeAgent, BytecodeAssertion, BytecodeCapability, BytecodeFailure, BytecodeModel,
    BytecodeModule, BytecodeModuleImport, BytecodePassport, BytecodePassportAsn, BytecodePolicy,
    BytecodePolicyRule, BytecodePolicyViolation, BytecodeProgram, BytecodeProviderContract,
    BytecodeProviderHarness, BytecodeTool, BytecodeType, BytecodeTypeField, Instruction,
};
use argorix_ir::{ir::IrHandlerInstruction, IrProgram};
use std::collections::HashMap;

pub fn lower_ir(ir: &IrProgram) -> BytecodeProgram {
    let mut instructions = Vec::new();
    for provider in &ir.providers {
        instructions.push(Instruction::DeclareProviderContract {
            name: provider.name.clone(),
            kind: provider.kind.clone(),
            enabled: provider.enabled,
            dry_run_only: provider.dry_run_only,
            requires_feature_flag: provider.requires_feature_flag,
            requires_explicit_approval: provider.requires_explicit_approval,
            allowed_targets: provider.allowed_targets.clone(),
            allowed_capabilities: provider.allowed_capabilities.clone(),
        });
    }
    for assertion in &ir.assertions {
        instructions.push(Instruction::DeclareAssertion {
            name: assertion.name.clone(),
            argument: assertion.argument.clone(),
        });
    }
    for failure in &ir.failures {
        instructions.push(Instruction::DeclareFailure {
            name: failure.name.clone(),
            action: failure.action.clone(),
            trace: failure.trace.clone(),
        });
    }
    let capability_levels: HashMap<&str, (&str, bool)> = ir
        .capabilities
        .iter()
        .map(|capability| {
            (
                capability.name.as_str(),
                (capability.level.as_str(), capability.requires_approval),
            )
        })
        .collect();

    for capability in &ir.capabilities {
        instructions.push(Instruction::DeclareCapability {
            name: capability.name.clone(),
            level: capability.level.clone(),
            requires_approval: capability.requires_approval,
        });
    }
    for tool in &ir.tools {
        instructions.push(Instruction::DeclareTool {
            name: tool.name.clone(),
            provider: tool.provider.clone(),
            capability: tool.capability.clone(),
            input: tool.input.clone(),
            output: tool.output.clone(),
        });
    }
    for model in &ir.models {
        instructions.push(Instruction::DeclareModel {
            name: model.name.clone(),
            provider: model.provider.clone(),
            capability: model.capability.clone(),
            input: model.input.clone(),
            output: model.output.clone(),
        });
    }
    for agent in &ir.agents {
        instructions.push(Instruction::DeclareAgent {
            name: agent.name.clone(),
            approval: agent.approval.clone(),
        });
        for capability in &agent.capabilities {
            instructions.push(Instruction::RequireCapability {
                agent: agent.name.clone(),
                capability: capability.clone(),
            });
            if capability_levels.get(capability.as_str()).is_some_and(
                |(level, requires_approval)| {
                    *requires_approval || matches!(*level, "restricted" | "dangerous")
                },
            ) {
                instructions.push(Instruction::RequireApproval {
                    agent: agent.name.clone(),
                    capability: capability.clone(),
                });
            }
        }
        for tool in &agent.tools {
            instructions.push(Instruction::AuthorizeTool {
                agent: agent.name.clone(),
                tool: tool.clone(),
            });
        }
        for model in &agent.models {
            instructions.push(Instruction::AuthorizeModel {
                agent: agent.name.clone(),
                model: model.clone(),
            });
        }
        for handler in &agent.handlers {
            instructions.push(Instruction::DeclareHandler {
                agent: agent.name.clone(),
                message_type: handler.message_type.clone(),
                binding: handler.binding.clone(),
            });
            for instruction in &handler.instructions {
                instructions.push(match instruction {
                    IrHandlerInstruction::Emit { message_type, to } => Instruction::EmitMessage {
                        agent: agent.name.clone(),
                        message_type: message_type.clone(),
                        to: to.clone(),
                    },
                    IrHandlerInstruction::Trace { binding } => Instruction::TraceValue {
                        agent: agent.name.clone(),
                        binding: binding.clone(),
                    },
                    IrHandlerInstruction::Halt => Instruction::HandlerHalt {
                        agent: agent.name.clone(),
                    },
                    IrHandlerInstruction::Intrinsic { name, argument } => {
                        Instruction::InvokeIntrinsic {
                            agent: agent.name.clone(),
                            name: name.clone(),
                            argument: argument.clone(),
                        }
                    }
                    IrHandlerInstruction::Call { tool, binding } => Instruction::CallTool {
                        agent: agent.name.clone(),
                        tool: tool.clone(),
                        binding: binding.clone(),
                    },
                    IrHandlerInstruction::Ask { model, binding } => Instruction::AskModel {
                        agent: agent.name.clone(),
                        model: model.clone(),
                        binding: binding.clone(),
                    },
                });
            }
            instructions.push(Instruction::EndHandler);
        }
    }
    for protocol in &ir.protocols {
        instructions.push(Instruction::DeclareProtocol {
            name: protocol.name.clone(),
        });
        for step in &protocol.steps {
            instructions.push(Instruction::SendMessage {
                from: step.from.clone(),
                to: step.to.clone(),
                act: step.act.clone(),
                message_type: step.message_type.clone(),
            });
        }
        instructions.push(Instruction::Trace {
            message: format!("protocol {} completed", protocol.name),
        });
    }
    for assertion in &ir.assertions {
        instructions.push(Instruction::VerifyAssertion {
            name: assertion.name.clone(),
            argument: assertion.argument.clone(),
        });
    }
    instructions.push(Instruction::PolicyReport);
    instructions.push(Instruction::End);

    BytecodeProgram {
        bytecode_version: "0.20".to_owned(),
        language: ir.language.clone(),
        module: ir.module.clone(),
        modules: ir
            .modules
            .iter()
            .map(|module| BytecodeModule {
                name: module.name.clone(),
                path: module.path.clone(),
            })
            .collect(),
        provider_harnesses: ir
            .provider_harnesses
            .iter()
            .map(|harness| BytecodeProviderHarness {
                name: harness.name.clone(),
                provider: harness.provider.clone(),
                mode: harness.mode.clone(),
                network: harness.network.clone(),
                secrets: harness.secrets.clone(),
                filesystem: harness.filesystem.clone(),
                max_steps: harness.max_steps,
                timeout_ms: harness.timeout_ms,
                input_contract: harness.input_contract.clone(),
                output_contract: harness.output_contract.clone(),
                attestations: harness.attestations.clone(),
            })
            .collect(),
        imports: ir
            .imports
            .iter()
            .map(|import| BytecodeModuleImport {
                from: import.from.clone(),
                to: import.to.clone(),
            })
            .collect(),
        providers: ir
            .providers
            .iter()
            .map(|provider| BytecodeProviderContract {
                name: provider.name.clone(),
                kind: provider.kind.clone(),
                enabled: provider.enabled,
                dry_run_only: provider.dry_run_only,
                requires_feature_flag: provider.requires_feature_flag,
                requires_explicit_approval: provider.requires_explicit_approval,
                allowed_targets: provider.allowed_targets.clone(),
                allowed_capabilities: provider.allowed_capabilities.clone(),
            })
            .collect(),
        assertions: ir
            .assertions
            .iter()
            .map(|assertion| BytecodeAssertion {
                name: assertion.name.clone(),
                argument: assertion.argument.clone(),
            })
            .collect(),
        policies: ir
            .policies
            .iter()
            .map(|policy| BytecodePolicy {
                name: policy.name.clone(),
                rules: policy
                    .rules
                    .iter()
                    .map(|rule| BytecodePolicyRule {
                        effect: rule.effect.clone(),
                        rule: rule.rule.clone(),
                    })
                    .collect(),
                on_violation: policy.on_violation.as_ref().map(|violation| {
                    BytecodePolicyViolation {
                        action: violation.action.clone(),
                        trace_required: violation.trace_required,
                    }
                }),
            })
            .collect(),
        types: ir
            .types
            .iter()
            .map(|contract| BytecodeType {
                name: contract.name.clone(),
                fields: contract
                    .fields
                    .iter()
                    .map(|field| BytecodeTypeField {
                        name: field.name.clone(),
                        field_type: field.field_type.clone(),
                    })
                    .collect(),
            })
            .collect(),
        enums: ir.enums.iter().map(|item| item.name.clone()).collect(),
        failures: ir
            .failures
            .iter()
            .map(|failure| BytecodeFailure {
                name: failure.name.clone(),
                action: failure.action.clone(),
                trace: failure.trace.clone(),
            })
            .collect(),
        passports: ir
            .passports
            .iter()
            .map(|passport| BytecodePassport {
                name: passport.name.clone(),
                agent: passport.agent.clone(),
                agent_name: passport.agent_name.clone(),
                global_id: passport.global_id.clone(),
                identity: passport.identity.clone(),
                provider: passport.provider.clone(),
                version: passport.version.clone(),
                ans_name: passport.ans_name.clone(),
                country: passport.country.clone(),
                jurisdiction: passport.jurisdiction.clone(),
                data_residency: passport.data_residency.clone(),
                asn: passport.asn.as_ref().map(|asn| BytecodePassportAsn {
                    registry: asn.registry.clone(),
                    number: asn.number.clone(),
                    holder: asn.holder.clone(),
                    country: asn.country.clone(),
                }),
                model: passport.model.clone(),
                risk_level: passport.risk_level.clone(),
                data_scope: passport.data_scope.clone(),
                intent: passport.intent.clone(),
                intended_use: passport.intended_use.clone(),
                prohibited_use: passport.prohibited_use.clone(),
                attestations: passport.attestations.clone(),
            })
            .collect(),
        agents: ir
            .agents
            .iter()
            .map(|agent| BytecodeAgent {
                name: agent.name.clone(),
                approval: agent.approval.clone(),
            })
            .collect(),
        capabilities: ir
            .capabilities
            .iter()
            .map(|capability| BytecodeCapability {
                name: capability.name.clone(),
                level: capability.level.clone(),
                requires_approval: capability.requires_approval,
            })
            .collect(),
        tools: ir
            .tools
            .iter()
            .map(|tool| BytecodeTool {
                name: tool.name.clone(),
                provider: tool.provider.clone(),
                capability: tool.capability.clone(),
                input: tool.input.clone(),
                output: tool.output.clone(),
            })
            .collect(),
        models: ir
            .models
            .iter()
            .map(|model| BytecodeModel {
                name: model.name.clone(),
                provider: model.provider.clone(),
                capability: model.capability.clone(),
                input: model.input.clone(),
                output: model.output.clone(),
            })
            .collect(),
        instructions,
    }
}

#[cfg(test)]
mod tests {
    use super::lower_ir;
    use crate::Instruction;
    use argorix_ir::{
        ir::{
            IrAgent, IrAssertion, IrCapability, IrFailure, IrHandler, IrHandlerInstruction,
            IrProtocol, IrProtocolStep, IrProviderContract, IrProviderHarness, IrTool,
        },
        IrProgram,
    };

    #[test]
    fn lowers_ir_to_versioned_message_bytecode_ending_in_end() {
        let ir = IrProgram {
            ir_version: "0.2".into(),
            language: "Argorix Lang".into(),
            module: "Example".into(),
            modules: vec![],
            imports: vec![],
            providers: vec![],
            provider_harnesses: vec![],
            assertions: vec![IrAssertion {
                name: "runtime_status".into(),
                argument: Some("completed".into()),
            }],
            policies: vec![],
            failures: vec![IrFailure {
                name: "PolicyViolation".into(),
                action: "block".into(),
                trace: "required".into(),
            }],
            capabilities: vec![IrCapability {
                name: "trace.write".into(),
                level: "safe".into(),
                requires_approval: false,
            }],
            enums: vec![],
            types: vec![],
            tools: vec![IrTool {
                name: "Echo".into(),
                provider: "simulated".into(),
                capability: "trace.write".into(),
                input: "Ping".into(),
                output: "Pong".into(),
            }],
            models: vec![],
            agents: vec![IrAgent {
                name: "Worker".into(),
                approval: "denied".into(),
                receives: vec![],
                sends: vec![],
                capabilities: vec!["trace.write".into()],
                tools: vec!["Echo".into()],
                models: vec![],
                handlers: vec![IrHandler {
                    message_type: "Ping".into(),
                    binding: "ping".into(),
                    instructions: vec![
                        IrHandlerInstruction::Intrinsic {
                            name: "facu".into(),
                            argument: "ping".into(),
                        },
                        IrHandlerInstruction::Call {
                            tool: "Echo".into(),
                            binding: "ping".into(),
                        },
                        IrHandlerInstruction::Emit {
                            message_type: "Pong".into(),
                            to: "Worker".into(),
                        },
                    ],
                }],
            }],
            protocols: vec![IrProtocol {
                name: "Flow".into(),
                steps: vec![IrProtocolStep {
                    from: "User".into(),
                    to: "Worker".into(),
                    act: "tell".into(),
                    message_type: "Ping".into(),
                }],
            }],
            passports: vec![],
        };

        let bytecode = lower_ir(&ir);
        assert_eq!(bytecode.bytecode_version, "0.20");
        assert!(bytecode
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::SendMessage { .. })));
        assert!(bytecode
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::DeclareHandler { .. })));
        assert!(bytecode
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::EmitMessage { .. })));
        assert!(bytecode
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::InvokeIntrinsic { .. })));
        assert!(bytecode
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::DeclareTool { .. })));
        assert!(bytecode
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::AuthorizeTool { .. })));
        assert!(bytecode
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::CallTool { .. })));
        assert!(bytecode.instructions.iter().any(|instruction| matches!(
            instruction,
            Instruction::DeclareAssertion { name, .. } if name == "runtime_status"
        )));
        assert!(bytecode.instructions.iter().any(|instruction| matches!(
            instruction,
            Instruction::DeclareFailure { name, .. } if name == "PolicyViolation"
        )));
        assert!(bytecode.instructions.iter().any(|instruction| matches!(
            instruction,
            Instruction::VerifyAssertion { name, .. } if name == "runtime_status"
        )));
        assert!(bytecode
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::PolicyReport)));
        assert!(matches!(
            bytecode.instructions.last(),
            Some(Instruction::End)
        ));
    }

    #[test]
    fn lowers_provider_harness_as_top_level_metadata_only() {
        let ir = IrProgram {
            ir_version: "0.20".into(),
            language: "Argorix Lang".into(),
            module: "main".into(),
            modules: vec![],
            imports: vec![],
            providers: vec![IrProviderContract {
                name: "OpenAI".into(),
                kind: "external".into(),
                enabled: false,
                dry_run_only: true,
                requires_feature_flag: true,
                requires_explicit_approval: true,
                allowed_targets: vec![],
                allowed_capabilities: vec![],
            }],
            provider_harnesses: vec![IrProviderHarness {
                name: "OpenAIHarness".into(),
                provider: "OpenAI".into(),
                mode: "dry_run".into(),
                network: "denied".into(),
                secrets: "denied".into(),
                filesystem: "none".into(),
                max_steps: None,
                timeout_ms: None,
                input_contract: None,
                output_contract: None,
                attestations: vec![],
            }],
            assertions: vec![],
            policies: vec![],
            failures: vec![],
            capabilities: vec![],
            enums: vec![],
            types: vec![],
            tools: vec![],
            models: vec![],
            agents: vec![],
            protocols: vec![],
            passports: vec![],
        };
        let bytecode = lower_ir(&ir);
        assert_eq!(bytecode.bytecode_version, "0.20");
        assert_eq!(bytecode.provider_harnesses.len(), 1);
        assert_eq!(bytecode.provider_harnesses[0].name, "OpenAIHarness");
        assert_eq!(
            bytecode
                .instructions
                .iter()
                .filter(|instruction| matches!(instruction, Instruction::Unknown))
                .count(),
            0
        );
    }
}
