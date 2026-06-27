use crate::{
    BytecodeA2ABridgeContract, BytecodeATrustBoundary, BytecodeATrustCredentialContract,
    BytecodeATrustEvidenceMap, BytecodeATrustHandshake, BytecodeATrustIdentity, BytecodeAdapter,
    BytecodeAdapterProfile, BytecodeAgent, BytecodeAssertion, BytecodeCapability,
    BytecodeCompatibilityMatrixEntry, BytecodeCrypto, BytecodeCryptoBoundary, BytecodeDidMethod,
    BytecodeFailure, BytecodeFeature, BytecodeGovernanceControl, BytecodeGovernanceProfile,
    BytecodeMcpBridgeContract, BytecodeModel, BytecodeModule, BytecodeModuleImport,
    BytecodePassport, BytecodePassportAsn, BytecodePolicy, BytecodePolicyRule,
    BytecodePolicyViolation, BytecodeProgram, BytecodeProviderContract, BytecodeProviderHarness,
    BytecodePublicConformanceClaim, BytecodePublicConformanceReport, BytecodeRegulatoryMapping,
    BytecodeRegulatoryObligation, BytecodeReleaseCandidate, BytecodeRuntimeExecutionProfile,
    BytecodeRuntimeHardeningProfile, BytecodeSandboxedProviderAdapter, BytecodeSecret,
    BytecodeSpecFreeze, BytecodeThirdPartyVerifier, BytecodeThreat, BytecodeThreatAsset,
    BytecodeThreatMitigation, BytecodeThreatModel, BytecodeTool, BytecodeTrustLedger,
    BytecodeTrustLedgerEntry, BytecodeType, BytecodeTypeField, Instruction,
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
        bytecode_version: "1.0".to_owned(),
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
                feature: harness.feature.clone(),
                secret: harness.secret.clone(),
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
        features: ir
            .features
            .iter()
            .map(|feature| BytecodeFeature {
                name: feature.name.clone(),
                provider: feature.provider.clone(),
                status: feature.status.clone(),
                default: feature.default.clone(),
                requires_approval: feature.requires_approval,
                purpose: feature.purpose.clone(),
            })
            .collect(),
        secrets: ir
            .secrets
            .iter()
            .map(|secret| BytecodeSecret {
                name: secret.name.clone(),
                handle: secret.handle.clone(),
                provider: secret.provider.clone(),
                required_by: secret.required_by.clone(),
                scope: secret.scope.clone(),
                access: secret.access.clone(),
                source: secret.source.clone(),
            })
            .collect(),
        adapters: ir
            .adapters
            .iter()
            .map(|adapter| BytecodeAdapter {
                name: adapter.name.clone(),
                provider: adapter.provider.clone(),
                feature: adapter.feature.clone(),
                secret: adapter.secret.clone(),
                harness: adapter.harness.clone(),
                kind: adapter.kind.clone(),
                vendor: adapter.vendor.clone(),
                mode: adapter.mode.clone(),
                execution: adapter.execution.clone(),
                network: adapter.network.clone(),
                secrets: adapter.secrets.clone(),
                filesystem: adapter.filesystem.clone(),
                input_contract: adapter.input_contract.clone(),
                output_contract: adapter.output_contract.clone(),
                conformance: adapter.conformance.clone(),
            })
            .collect(),
        adapter_profiles: ir
            .adapter_profiles
            .iter()
            .map(|p| BytecodeAdapterProfile {
                name: p.name.clone(),
                adapter: p.adapter.clone(),
                provider: p.provider.clone(),
                vendor: p.vendor.clone(),
                family: p.family.clone(),
                api_style: p.api_style.clone(),
                auth: p.auth.clone(),
                execution: p.execution.clone(),
                network: p.network.clone(),
                secrets: p.secrets.clone(),
                request_contract: p.request_contract.clone(),
                response_contract: p.response_contract.clone(),
                capabilities: p.capabilities.clone(),
                required_conformance: p.required_conformance.clone(),
            })
            .collect(),
        cryptos: ir
            .cryptos
            .iter()
            .map(|c| BytecodeCrypto {
                name: c.name.clone(),
                kind: c.kind.clone(),
                status: c.status.clone(),
                strength: c.strength.clone(),
                purpose: c.purpose.clone(),
                output_bits: c.output_bits,
                min_key_bits: c.min_key_bits,
                security_level: c.security_level.clone(),
                notes: c.notes.clone(),
            })
            .collect(),
        crypto_boundaries: ir
            .crypto_boundaries
            .iter()
            .map(|b| BytecodeCryptoBoundary {
                name: b.name.clone(),
                allowed_hashes: b.allowed_hashes.clone(),
                allowed_signatures: b.allowed_signatures.clone(),
                allowed_kems: b.allowed_kems.clone(),
                allowed_aeads: b.allowed_aeads.clone(),
                legacy_allowed: b.legacy_allowed.clone(),
                denied: b.denied.clone(),
                purpose: b.purpose.clone(),
                min_hash_bits: b.min_hash_bits,
                post_quantum_ready: b.post_quantum_ready,
                hybrid_allowed: b.hybrid_allowed,
                key_material: b.key_material.clone(),
                secret_material: b.secret_material.clone(),
                execution: b.execution.clone(),
            })
            .collect(),
        did_methods: ir
            .did_methods
            .iter()
            .map(|d| BytecodeDidMethod {
                name: d.name.clone(),
                status: d.status.clone(),
                resolution: d.resolution.clone(),
                ledger: d.ledger.clone(),
                crypto_boundary: d.crypto_boundary.clone(),
                governance: d.governance.clone(),
                purpose: d.purpose.clone(),
                notes: d.notes.clone(),
            })
            .collect(),
        atrust_boundaries: ir
            .atrust_boundaries
            .iter()
            .map(|a| BytecodeATrustBoundary {
                name: a.name.clone(),
                crypto_boundary: a.crypto_boundary.clone(),
                did_methods: a.did_methods.clone(),
                identity_format: a.identity_format.clone(),
                credential_mode: a.credential_mode.clone(),
                handshake: a.handshake.clone(),
                resolution: a.resolution.clone(),
                key_material: a.key_material.clone(),
                secret_material: a.secret_material.clone(),
                execution: a.execution.clone(),
                post_quantum_ready: a.post_quantum_ready.clone(),
                security_claims: a.security_claims.clone(),
                purpose: a.purpose.clone(),
                notes: a.notes.clone(),
            })
            .collect(),
        atrust_identities: ir
            .atrust_identities
            .iter()
            .map(|i| BytecodeATrustIdentity {
                name: i.name.clone(),
                subject: i.subject.clone(),
                did: i.did.clone(),
                method: i.method.clone(),
                boundary: i.boundary.clone(),
                status: i.status.clone(),
                validation: i.validation.clone(),
                resolution: i.resolution.clone(),
                key_material: i.key_material.clone(),
                secret_material: i.secret_material.clone(),
                execution: i.execution.clone(),
                evidence: i.evidence.clone(),
                security_claims: i.security_claims.clone(),
                purpose: i.purpose.clone(),
                notes: i.notes.clone(),
            })
            .collect(),
        atrust_credential_contracts: ir
            .atrust_credential_contracts
            .iter()
            .map(|c| BytecodeATrustCredentialContract {
                name: c.name.clone(),
                subject: c.subject.clone(),
                identity: c.identity.clone(),
                boundary: c.boundary.clone(),
                method: c.method.clone(),
                issuer_did: c.issuer_did.clone(),
                holder_did: c.holder_did.clone(),
                credential_type: c.credential_type.clone(),
                schema: c.schema.clone(),
                status: c.status.clone(),
                verification: c.verification.clone(),
                presentation: c.presentation.clone(),
                resolution: c.resolution.clone(),
                key_material: c.key_material.clone(),
                secret_material: c.secret_material.clone(),
                execution: c.execution.clone(),
                evidence: c.evidence.clone(),
                security_claims: c.security_claims.clone(),
                claims: c.claims.clone(),
                purpose: c.purpose.clone(),
                notes: c.notes.clone(),
            })
            .collect(),
        atrust_handshakes: ir
            .atrust_handshakes
            .iter()
            .map(|h| BytecodeATrustHandshake {
                name: h.name.clone(),
                initiator: h.initiator.clone(),
                responder: h.responder.clone(),
                initiator_identity: h.initiator_identity.clone(),
                responder_identity: h.responder_identity.clone(),
                credential_contracts: h.credential_contracts.clone(),
                boundary: h.boundary.clone(),
                method: h.method.clone(),
                mode: h.mode.clone(),
                direction: h.direction.clone(),
                challenge: h.challenge.clone(),
                response: h.response.clone(),
                transcript: h.transcript.clone(),
                verification: h.verification.clone(),
                resolution: h.resolution.clone(),
                network: h.network.clone(),
                key_material: h.key_material.clone(),
                secret_material: h.secret_material.clone(),
                execution: h.execution.clone(),
                evidence: h.evidence.clone(),
                security_claims: h.security_claims.clone(),
                purpose: h.purpose.clone(),
                notes: h.notes.clone(),
            })
            .collect(),
        trust_ledgers: ir
            .trust_ledgers
            .iter()
            .map(|l| BytecodeTrustLedger {
                name: l.name.clone(),
                scope: l.scope.clone(),
                mode: l.mode.clone(),
                hash_algorithm: l.hash_algorithm.clone(),
                chain_policy: l.chain_policy.clone(),
                entries: l
                    .entries
                    .iter()
                    .map(|e| BytecodeTrustLedgerEntry {
                        id: e.id.clone(),
                        kind: e.kind.clone(),
                        subject: e.subject.clone(),
                        previous_hash: e.previous_hash.clone(),
                        entry_hash: e.entry_hash.clone(),
                        evidence_ref: e.evidence_ref.clone(),
                    })
                    .collect(),
                chain_root: l.chain_root.clone(),
                network: l.network.clone(),
                key_material: l.key_material.clone(),
                secret_material: l.secret_material.clone(),
                execution: l.execution.clone(),
                evidence: l.evidence.clone(),
                security_claims: l.security_claims.clone(),
                purpose: l.purpose.clone(),
                notes: l.notes.clone(),
            })
            .collect(),
        mcp_bridge_contracts: ir
            .mcp_bridge_contracts
            .iter()
            .map(|c| BytecodeMcpBridgeContract {
                name: c.name.clone(),
                agent: c.agent.clone(),
                passport: c.passport.clone(),
                identity: c.identity.clone(),
                boundary: c.boundary.clone(),
                transport: c.transport.clone(),
                protocol: c.protocol.clone(),
                direction: c.direction.clone(),
                tools: c.tools.clone(),
                resources: c.resources.clone(),
                prompts: c.prompts.clone(),
                network: c.network.clone(),
                external_execution: c.external_execution.clone(),
                tool_execution: c.tool_execution.clone(),
                secret_material: c.secret_material.clone(),
                key_material: c.key_material.clone(),
                authentication: c.authentication.clone(),
                authorization: c.authorization.clone(),
                evidence: c.evidence.clone(),
                security_claims: c.security_claims.clone(),
                purpose: c.purpose.clone(),
                notes: c.notes.clone(),
            })
            .collect(),
        a2a_bridge_contracts: ir
            .a2a_bridge_contracts
            .iter()
            .map(|c| BytecodeA2ABridgeContract {
                name: c.name.clone(),
                initiator: c.initiator.clone(),
                responder: c.responder.clone(),
                initiator_passport: c.initiator_passport.clone(),
                responder_passport: c.responder_passport.clone(),
                initiator_identity: c.initiator_identity.clone(),
                responder_identity: c.responder_identity.clone(),
                handshake: c.handshake.clone(),
                trust_ledger: c.trust_ledger.clone(),
                boundary: c.boundary.clone(),
                protocol: c.protocol.clone(),
                transport: c.transport.clone(),
                direction: c.direction.clone(),
                message_contracts: c.message_contracts.clone(),
                capabilities: c.capabilities.clone(),
                network: c.network.clone(),
                external_execution: c.external_execution.clone(),
                agent_execution: c.agent_execution.clone(),
                secret_material: c.secret_material.clone(),
                key_material: c.key_material.clone(),
                authentication: c.authentication.clone(),
                authorization: c.authorization.clone(),
                evidence: c.evidence.clone(),
                security_claims: c.security_claims.clone(),
                purpose: c.purpose.clone(),
                notes: c.notes.clone(),
            })
            .collect(),
        atrust_evidence_maps: ir
            .atrust_evidence_maps
            .iter()
            .map(|m| BytecodeATrustEvidenceMap {
                name: m.name.clone(),
                agent: m.agent.clone(),
                passport: m.passport.clone(),
                identity: m.identity.clone(),
                credential_contract: m.credential_contract.clone(),
                handshake: m.handshake.clone(),
                trust_ledger: m.trust_ledger.clone(),
                mcp_bridges: m.mcp_bridges.clone(),
                a2a_bridges: m.a2a_bridges.clone(),
                policies: m.policies.clone(),
                coverage: m.coverage.clone(),
                mapping_mode: m.mapping_mode.clone(),
                verification: m.verification.clone(),
                resolution: m.resolution.clone(),
                evidence_bundle: m.evidence_bundle.clone(),
                security_report: m.security_report.clone(),
                trace: m.trace.clone(),
                network: m.network.clone(),
                external_execution: m.external_execution.clone(),
                secret_material: m.secret_material.clone(),
                key_material: m.key_material.clone(),
                execution: m.execution.clone(),
                security_claims: m.security_claims.clone(),
                purpose: m.purpose.clone(),
                notes: m.notes.clone(),
            })
            .collect(),
        governance_profiles: ir
            .governance_profiles
            .iter()
            .map(|p| BytecodeGovernanceProfile {
                name: p.name.clone(),
                scope: p.scope.clone(),
                level: p.level.clone(),
                domain: p.domain.clone(),
                owner: p.owner.clone(),
                jurisdiction: p.jurisdiction.clone(),
                framework: p.framework.clone(),
                evidence_map: p.evidence_map.clone(),
                trust_ledger: p.trust_ledger.clone(),
                policies: p.policies.clone(),
                controls: p
                    .controls
                    .iter()
                    .map(|c| BytecodeGovernanceControl {
                        id: c.id.clone(),
                        category: c.category.clone(),
                        requirement: c.requirement.clone(),
                        evidence_ref: c.evidence_ref.clone(),
                        status: c.status.clone(),
                    })
                    .collect(),
                risk_level: p.risk_level.clone(),
                review_status: p.review_status.clone(),
                assurance: p.assurance.clone(),
                network: p.network.clone(),
                external_execution: p.external_execution.clone(),
                secret_material: p.secret_material.clone(),
                key_material: p.key_material.clone(),
                execution: p.execution.clone(),
                security_claims: p.security_claims.clone(),
                purpose: p.purpose.clone(),
                notes: p.notes.clone(),
            })
            .collect(),
        regulatory_mappings: ir
            .regulatory_mappings
            .iter()
            .map(|m| BytecodeRegulatoryMapping {
                name: m.name.clone(),
                governance_profile: m.governance_profile.clone(),
                evidence_map: m.evidence_map.clone(),
                jurisdiction: m.jurisdiction.clone(),
                framework: m.framework.clone(),
                obligations: m
                    .obligations
                    .iter()
                    .map(|o| BytecodeRegulatoryObligation {
                        id: o.id.clone(),
                        source: o.source.clone(),
                        requirement: o.requirement.clone(),
                        control: o.control.clone(),
                        evidence_ref: o.evidence_ref.clone(),
                        status: o.status.clone(),
                    })
                    .collect(),
                coverage: m.coverage.clone(),
                assessment: m.assessment.clone(),
                legal_claims: m.legal_claims.clone(),
                certification: m.certification.clone(),
                network: m.network.clone(),
                external_execution: m.external_execution.clone(),
                secret_material: m.secret_material.clone(),
                key_material: m.key_material.clone(),
                execution: m.execution.clone(),
                security_claims: m.security_claims.clone(),
                purpose: m.purpose.clone(),
                notes: m.notes.clone(),
            })
            .collect(),
        third_party_verifiers: ir
            .third_party_verifiers
            .iter()
            .map(|v| BytecodeThirdPartyVerifier {
                name: v.name.clone(),
                verifier_type: v.verifier_type.clone(),
                independence: v.independence.clone(),
                identity_mode: v.identity_mode.clone(),
                verification_mode: v.verification_mode.clone(),
                display_name: v.display_name.clone(),
                organization: v.organization.clone(),
                jurisdiction: v.jurisdiction.clone(),
                allowed_scopes: v.allowed_scopes.clone(),
                disallowed_claims: v.disallowed_claims.clone(),
                network: v.network.clone(),
                external_execution: v.external_execution.clone(),
                secret_material: v.secret_material.clone(),
                key_material: v.key_material.clone(),
                execution: v.execution.clone(),
                legal_claims: v.legal_claims.clone(),
                certification: v.certification.clone(),
                security_claims: v.security_claims.clone(),
                purpose: v.purpose.clone(),
                notes: v.notes.clone(),
            })
            .collect(),
        public_conformance_reports: ir
            .public_conformance_reports
            .iter()
            .map(|r| BytecodePublicConformanceReport {
                name: r.name.clone(),
                verifier: r.verifier.clone(),
                suite: r.suite.clone(),
                suite_version: r.suite_version.clone(),
                source_artifact: r.source_artifact.clone(),
                bytecode_artifact: r.bytecode_artifact.clone(),
                evidence_map: r.evidence_map.clone(),
                governance_profile: r.governance_profile.clone(),
                regulatory_mapping: r.regulatory_mapping.clone(),
                trust_ledger: r.trust_ledger.clone(),
                security_report: r.security_report.clone(),
                evidence_bundle: r.evidence_bundle.clone(),
                trace: r.trace.clone(),
                result: r.result.clone(),
                reproducibility: r.reproducibility.clone(),
                review_status: r.review_status.clone(),
                claims: r
                    .claims
                    .iter()
                    .map(|c| BytecodePublicConformanceClaim {
                        id: c.id.clone(),
                        category: c.category.clone(),
                        statement: c.statement.clone(),
                        evidence_ref: c.evidence_ref.clone(),
                        status: c.status.clone(),
                    })
                    .collect(),
                network: r.network.clone(),
                external_execution: r.external_execution.clone(),
                secret_material: r.secret_material.clone(),
                key_material: r.key_material.clone(),
                execution: r.execution.clone(),
                legal_claims: r.legal_claims.clone(),
                certification: r.certification.clone(),
                security_claims: r.security_claims.clone(),
                purpose: r.purpose.clone(),
                notes: r.notes.clone(),
            })
            .collect(),
        runtime_hardening_profiles: ir
            .runtime_hardening_profiles
            .iter()
            .map(|p| BytecodeRuntimeHardeningProfile {
                name: p.name.clone(),
                scope: p.scope.clone(),
                mode: p.mode.clone(),
                enforcement: p.enforcement.clone(),
                sandbox: p.sandbox.clone(),
                provider_execution: p.provider_execution.clone(),
                external_providers: p.external_providers.clone(),
                network: p.network.clone(),
                tool_execution: p.tool_execution.clone(),
                agent_execution: p.agent_execution.clone(),
                filesystem_access: p.filesystem_access.clone(),
                env_access: p.env_access.clone(),
                secret_material: p.secret_material.clone(),
                key_material: p.key_material.clone(),
                allowlist: p.allowlist.clone(),
                deny_by_default: p.deny_by_default,
                approval: p.approval.clone(),
                audit_log: p.audit_log.clone(),
                evidence: p.evidence.clone(),
                incident_response: p.incident_response.clone(),
                evidence_map: p.evidence_map.clone(),
                governance_profile: p.governance_profile.clone(),
                public_conformance_report: p.public_conformance_report.clone(),
                protected_assets: p.protected_assets.clone(),
                runtime_boundaries: p.runtime_boundaries.clone(),
                residual_risk: p.residual_risk.clone(),
                review_status: p.review_status.clone(),
                assurance: p.assurance.clone(),
                security_claims: p.security_claims.clone(),
                purpose: p.purpose.clone(),
                notes: p.notes.clone(),
            })
            .collect(),
        threat_models: ir
            .threat_models
            .iter()
            .map(|m| BytecodeThreatModel {
                name: m.name.clone(),
                hardening_profile: m.hardening_profile.clone(),
                evidence_map: m.evidence_map.clone(),
                governance_profile: m.governance_profile.clone(),
                public_conformance_report: m.public_conformance_report.clone(),
                methodology: m.methodology.clone(),
                scope: m.scope.clone(),
                review_status: m.review_status.clone(),
                assets: m
                    .assets
                    .iter()
                    .map(|a| BytecodeThreatAsset {
                        id: a.id.clone(),
                        category: a.category.clone(),
                        description: a.description.clone(),
                        sensitivity: a.sensitivity.clone(),
                        evidence_ref: a.evidence_ref.clone(),
                    })
                    .collect(),
                threats: m
                    .threats
                    .iter()
                    .map(|t| BytecodeThreat {
                        id: t.id.clone(),
                        category: t.category.clone(),
                        target: t.target.clone(),
                        impact: t.impact.clone(),
                        mitigation: t.mitigation.clone(),
                        status: t.status.clone(),
                    })
                    .collect(),
                mitigations: m
                    .mitigations
                    .iter()
                    .map(|x| BytecodeThreatMitigation {
                        id: x.id.clone(),
                        category: x.category.clone(),
                        control_ref: x.control_ref.clone(),
                        evidence_ref: x.evidence_ref.clone(),
                        status: x.status.clone(),
                    })
                    .collect(),
                residual_risk: m.residual_risk.clone(),
                risk_acceptance: m.risk_acceptance.clone(),
                network: m.network.clone(),
                external_execution: m.external_execution.clone(),
                tool_execution: m.tool_execution.clone(),
                agent_execution: m.agent_execution.clone(),
                secret_material: m.secret_material.clone(),
                key_material: m.key_material.clone(),
                execution: m.execution.clone(),
                security_claims: m.security_claims.clone(),
                purpose: m.purpose.clone(),
                notes: m.notes.clone(),
            })
            .collect(),
        spec_freezes: ir
            .spec_freezes
            .iter()
            .map(|s| BytecodeSpecFreeze {
                name: s.name.clone(),
                version: s.version.clone(),
                target: s.target.clone(),
                freeze_scope: s.freeze_scope.clone(),
                compatibility: s.compatibility.clone(),
                stability: s.stability.clone(),
                frozen_features: s.frozen_features.clone(),
                compatible_versions: s.compatible_versions.clone(),
                required_suites: s.required_suites.clone(),
                evidence_bundle: s.evidence_bundle.clone(),
                security_report: s.security_report.clone(),
                conformance: s.conformance.clone(),
                backward_compatibility: s.backward_compatibility.clone(),
                runtime_status: s.runtime_status.clone(),
                network: s.network.clone(),
                external_execution: s.external_execution.clone(),
                provider_execution: s.provider_execution.clone(),
                secret_material: s.secret_material.clone(),
                key_material: s.key_material.clone(),
                env_access: s.env_access.clone(),
                filesystem_access: s.filesystem_access.clone(),
                tool_execution: s.tool_execution.clone(),
                agent_execution: s.agent_execution.clone(),
                security_claims: s.security_claims.clone(),
                legal_claims: s.legal_claims.clone(),
                certification: s.certification.clone(),
                purpose: s.purpose.clone(),
                notes: s.notes.clone(),
            })
            .collect(),
        release_candidates: ir
            .release_candidates
            .iter()
            .map(|r| BytecodeReleaseCandidate {
                name: r.name.clone(),
                version: r.version.clone(),
                base_version: r.base_version.clone(),
                spec_freeze: r.spec_freeze.clone(),
                readiness: r.readiness.clone(),
                required_artifacts: r.required_artifacts.clone(),
                required_checks: r.required_checks.clone(),
                compatibility_matrix: r
                    .compatibility_matrix
                    .iter()
                    .map(|entry| BytecodeCompatibilityMatrixEntry {
                        version: entry.version.clone(),
                        bytecode: entry.bytecode.clone(),
                        evidence: entry.evidence.clone(),
                        conformance: entry.conformance.clone(),
                    })
                    .collect(),
                known_limitations: r.known_limitations.clone(),
                runtime_status: r.runtime_status.clone(),
                network: r.network.clone(),
                external_execution: r.external_execution.clone(),
                provider_execution: r.provider_execution.clone(),
                secret_material: r.secret_material.clone(),
                key_material: r.key_material.clone(),
                env_access: r.env_access.clone(),
                filesystem_access: r.filesystem_access.clone(),
                tool_execution: r.tool_execution.clone(),
                agent_execution: r.agent_execution.clone(),
                security_claims: r.security_claims.clone(),
                legal_claims: r.legal_claims.clone(),
                certification: r.certification.clone(),
                purpose: r.purpose.clone(),
                notes: r.notes.clone(),
            })
            .collect(),
        runtime_execution_profiles: ir
            .runtime_execution_profiles
            .iter()
            .map(|profile| BytecodeRuntimeExecutionProfile {
                name: profile.name.clone(),
                mode: profile.mode.clone(),
                agents: profile.agents.clone(),
                provider: profile.provider.clone(),
                hardening: profile.hardening.clone(),
                threat_model: profile.threat_model.clone(),
                evidence_map: profile.evidence_map.clone(),
                governance_profile: profile.governance_profile.clone(),
                allowed_actions: profile.allowed_actions.clone(),
                denied_actions: profile.denied_actions.clone(),
                network: profile.network.clone(),
                external_execution: profile.external_execution.clone(),
                tool_execution: profile.tool_execution.clone(),
                agent_execution: profile.agent_execution.clone(),
                secrets: profile.secrets.clone(),
                key_material: profile.key_material.clone(),
                audit: profile.audit.clone(),
                evidence: profile.evidence.clone(),
                security_report: profile.security_report.clone(),
                fail_closed: profile.fail_closed,
                security_claims: profile.security_claims.clone(),
                purpose: profile.purpose.clone(),
                notes: profile.notes.clone(),
            })
            .collect(),
        sandboxed_provider_adapters: ir
            .sandboxed_provider_adapters
            .iter()
            .map(|adapter| BytecodeSandboxedProviderAdapter {
                name: adapter.name.clone(),
                provider: adapter.provider.clone(),
                runtime: adapter.runtime.clone(),
                adapter_kind: adapter.adapter_kind.clone(),
                protocol: adapter.protocol.clone(),
                endpoint_ref: adapter.endpoint_ref.clone(),
                endpoint_value: None,
                secret_ref: adapter.secret_ref.clone(),
                secret_value: None,
                redacted: true,
                allowed_operations: adapter.allowed_operations.clone(),
                denied_operations: adapter.denied_operations.clone(),
                request_policy: adapter.request_policy.clone(),
                response_policy: adapter.response_policy.clone(),
                network: adapter.network.clone(),
                external_execution: adapter.external_execution.clone(),
                tool_execution: adapter.tool_execution.clone(),
                secret_material: adapter.secret_material.clone(),
                key_material: adapter.key_material.clone(),
                audit: adapter.audit.clone(),
                evidence: adapter.evidence.clone(),
                security_report: adapter.security_report.clone(),
                fail_closed: adapter.fail_closed,
                security_claims: adapter.security_claims.clone(),
                purpose: adapter.purpose.clone(),
                notes: adapter.notes.clone(),
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
            features: vec![],
            secrets: vec![],
            adapters: vec![],
            adapter_profiles: vec![],
            cryptos: vec![],
            crypto_boundaries: vec![],
            did_methods: vec![],
            atrust_boundaries: vec![],
            atrust_identities: vec![],
            atrust_credential_contracts: vec![],
            atrust_handshakes: vec![],
            trust_ledgers: vec![],
            mcp_bridge_contracts: vec![],
            a2a_bridge_contracts: vec![],
            atrust_evidence_maps: vec![],
            governance_profiles: vec![],
            regulatory_mappings: vec![],
            third_party_verifiers: vec![],
            public_conformance_reports: vec![],
            runtime_hardening_profiles: vec![],
            threat_models: vec![],
            spec_freezes: vec![],
            release_candidates: vec![],
            runtime_execution_profiles: vec![],
            sandboxed_provider_adapters: vec![],
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
        assert_eq!(bytecode.bytecode_version, "1.0");
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
                feature: None,
                secret: None,
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
            features: vec![],
            secrets: vec![],
            adapters: vec![],
            adapter_profiles: vec![],
            cryptos: vec![],
            crypto_boundaries: vec![],
            did_methods: vec![],
            atrust_boundaries: vec![],
            atrust_identities: vec![],
            atrust_credential_contracts: vec![],
            atrust_handshakes: vec![],
            trust_ledgers: vec![],
            mcp_bridge_contracts: vec![],
            a2a_bridge_contracts: vec![],
            atrust_evidence_maps: vec![],
            governance_profiles: vec![],
            regulatory_mappings: vec![],
            third_party_verifiers: vec![],
            public_conformance_reports: vec![],
            runtime_hardening_profiles: vec![],
            threat_models: vec![],
            spec_freezes: vec![],
            release_candidates: vec![],
            runtime_execution_profiles: vec![],
            sandboxed_provider_adapters: vec![],
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
        assert_eq!(bytecode.bytecode_version, "1.0");
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
