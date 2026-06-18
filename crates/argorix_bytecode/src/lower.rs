use crate::{BytecodeAgent, BytecodeCapability, BytecodeProgram, Instruction};
use argorix_ir::{ir::IrHandlerInstruction, IrProgram};
use std::collections::HashMap;

pub fn lower_ir(ir: &IrProgram) -> BytecodeProgram {
    let mut instructions = Vec::new();
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
    instructions.push(Instruction::End);

    BytecodeProgram {
        bytecode_version: "0.6".to_owned(),
        language: ir.language.clone(),
        module: ir.module.clone(),
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
        instructions,
    }
}

#[cfg(test)]
mod tests {
    use super::lower_ir;
    use crate::Instruction;
    use argorix_ir::{
        ir::{IrAgent, IrCapability, IrHandler, IrHandlerInstruction, IrProtocol, IrProtocolStep},
        IrProgram,
    };

    #[test]
    fn lowers_ir_to_versioned_message_bytecode_ending_in_end() {
        let ir = IrProgram {
            ir_version: "0.2".into(),
            language: "Argorix Lang".into(),
            module: "Example".into(),
            capabilities: vec![IrCapability {
                name: "trace.write".into(),
                level: "safe".into(),
                requires_approval: false,
            }],
            enums: vec![],
            types: vec![],
            agents: vec![IrAgent {
                name: "Worker".into(),
                approval: "denied".into(),
                receives: vec![],
                sends: vec![],
                capabilities: vec!["trace.write".into()],
                handlers: vec![IrHandler {
                    message_type: "Ping".into(),
                    binding: "ping".into(),
                    instructions: vec![
                        IrHandlerInstruction::Intrinsic {
                            name: "facu".into(),
                            argument: "ping".into(),
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
        };

        let bytecode = lower_ir(&ir);
        assert_eq!(bytecode.bytecode_version, "0.6");
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
        assert!(matches!(
            bytecode.instructions.last(),
            Some(Instruction::End)
        ));
    }
}
