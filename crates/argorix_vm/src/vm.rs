use crate::policy::{evaluate_rule, PolicyEvidenceContext};
use crate::{
    AgentStateSummary, AssertionResult, EventFields, EventType, ExecutionTrace, FailureActivation,
    InjectedMessage, IntrinsicExecution, MailboxSummary, PolicyActionResult, PolicyBlockResult,
    PolicyReport, PolicyRuleResult, PolicyViolation, ReactiveExecutionTrace, ReactiveScheduler,
    RuntimeState, RuntimeStatus, Scheduler, VmError,
};
use argorix_bytecode::{verify_bytecode, BytecodeError, BytecodeProgram};
use argorix_provider::{AdapterContract, ProviderKind, ProviderRegistry};

pub struct Vm {
    providers: ProviderRegistry,
}

#[derive(Debug)]
pub struct ExecutionOutcome {
    pub state: RuntimeState,
    pub result: Result<ReactiveExecutionTrace, VmError>,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Self {
            providers: ProviderRegistry::default(),
        }
    }

    pub fn run_dry(&self, bytecode: &BytecodeProgram) -> Result<ExecutionTrace, VmError> {
        let mut state = RuntimeState::from_bytecode(bytecode)?;
        let steps = Scheduler::new().run(bytecode, &mut state)?;
        let mailboxes = state
            .agents
            .iter()
            .filter_map(|agent| {
                state.mailboxes.get(agent).map(|mailbox| MailboxSummary {
                    agent: agent.clone(),
                    delivered: mailbox.delivered(),
                    processed: mailbox.processed(),
                })
            })
            .collect::<Vec<_>>();

        Ok(ExecutionTrace {
            vm_version: "0.5".to_owned(),
            status: "completed".to_owned(),
            mode: "dry-run".to_owned(),
            scheduler: "deterministic".to_owned(),
            steps,
            mailboxes,
            events: state.trace_ledger.events,
            security_checks: "passed".to_owned(),
        })
    }

    pub fn initialize(&self, bytecode: &BytecodeProgram) -> Result<RuntimeState, VmError> {
        RuntimeState::from_bytecode(bytecode)
    }

    pub fn load_provider_contracts(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
    ) -> Result<ProviderRegistry, VmError> {
        let mut registry = self.providers.execution_registry();
        for provider in &bytecode.providers {
            state.trace_ledger.record(
                EventType::ProviderContractDeclared,
                "declared",
                format!("provider contract {} declared", provider.name),
                EventFields::default(),
            );
            let kind = match provider.kind.as_str() {
                "simulated" => ProviderKind::Simulated,
                "external" => ProviderKind::External,
                other => {
                    let reason = format!("unsupported provider kind `{other}`");
                    self.reject_provider_contract(state, &provider.name, &reason);
                    return Err(VmError::ProviderBoundary {
                        provider: provider.name.clone(),
                        reason,
                    });
                }
            };
            let contract = AdapterContract {
                name: provider.name.clone(),
                kind,
                enabled: provider.enabled,
                dry_run_only: provider.dry_run_only,
                requires_feature_flag: provider.requires_feature_flag,
                requires_explicit_approval: provider.requires_explicit_approval,
                allowed_targets: provider.allowed_targets.clone(),
                allowed_capabilities: provider.allowed_capabilities.clone(),
            };
            if let Err(error) = registry.register_contract(contract) {
                let reason = error.to_string();
                self.reject_provider_contract(state, &provider.name, &reason);
                return Err(VmError::ProviderBoundary {
                    provider: provider.name.clone(),
                    reason,
                });
            }
            if let Err(error) = registry.validate_contract(&provider.name) {
                let reason = error.to_string();
                self.reject_provider_contract(state, &provider.name, &reason);
                return Err(VmError::ProviderBoundary {
                    provider: provider.name.clone(),
                    reason,
                });
            }
            state.trace_ledger.record(
                EventType::ProviderContractValidated,
                "validated",
                format!("provider contract {} validated", provider.name),
                EventFields::default(),
            );
        }
        Ok(registry)
    }

    fn reject_provider_contract(&self, state: &mut RuntimeState, name: &str, reason: &str) {
        state.activate_failure("ProviderContractRejected");
        state.trace_ledger.record(
            EventType::ProviderContractRejected,
            "rejected",
            format!("provider contract {name} rejected: {reason}"),
            EventFields::default(),
        );
        state.fail(format!("provider contract {name} rejected: {reason}"));
    }

    pub fn run_reactive(
        &self,
        bytecode: &BytecodeProgram,
        injected: InjectedMessage,
    ) -> Result<ReactiveExecutionTrace, VmError> {
        let outcome = self.run_reactive_outcome(bytecode, injected);
        match outcome.result {
            Ok(trace) => {
                if let Some(action) = trace
                    .policy_report
                    .actions
                    .iter()
                    .find(|action| action.action == "block")
                {
                    Err(VmError::PolicyViolation {
                        policy: action.policy.clone(),
                    })
                } else {
                    Ok(trace)
                }
            }
            Err(error) => Err(error),
        }
    }

    pub fn run_reactive_outcome(
        &self,
        bytecode: &BytecodeProgram,
        injected: InjectedMessage,
    ) -> ExecutionOutcome {
        let mut state = RuntimeState::from_bytecode(bytecode)
            .expect("runtime state initialization is infallible for decoded bytecode");
        if let Err(errors) = verify_bytecode(bytecode) {
            if let Some(provider) = blocked_external_provider(bytecode, &errors) {
                state.trace_ledger.record(
                    EventType::ExternalProviderExecutionBlocked,
                    "blocked",
                    format!("external provider execution through {provider} blocked"),
                    EventFields::default(),
                );
                state.trace_ledger.record(
                    EventType::ProviderBoundaryDenied,
                    "denied",
                    format!("provider boundary denied external provider {provider}"),
                    EventFields::default(),
                );
                record_external_execution_policy_violations(bytecode, &mut state);
            }
            if errors.iter().any(|error| {
                matches!(
                    error,
                    BytecodeError::HarnessesRequireV020
                        | BytecodeError::DuplicateProviderHarness(_)
                        | BytecodeError::InvalidProviderHarness { .. }
                )
            }) {
                state.trace_ledger.record(
                    EventType::ProviderHarnessRejected,
                    "rejected",
                    "provider harness metadata rejected by bytecode verification",
                    EventFields::default(),
                );
            }
            let error = VmError::from_verification(errors);
            state.fail(error.to_string());
            return ExecutionOutcome {
                state,
                result: Err(error),
            };
        }
        for harness in &bytecode.provider_harnesses {
            state.trace_ledger.record(
                EventType::ProviderHarnessValidated,
                "validated",
                format!("provider harness {} validated", harness.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::ProviderHarnessSandboxed,
                "sandboxed",
                format!(
                    "provider harness {} containment validated without execution",
                    harness.name
                ),
                EventFields::default(),
            );
        }
        for feature in &bytecode.features {
            state.trace_ledger.record(
                EventType::FeatureDeclared,
                "declared",
                format!("feature {} declared", feature.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::FeatureValidated,
                "validated",
                format!(
                    "feature {} validated as governance metadata without execution",
                    feature.name
                ),
                EventFields::default(),
            );
        }
        for secret in &bytecode.secrets {
            state.trace_ledger.record(
                EventType::SecretBoundaryDeclared,
                "declared",
                format!("secret boundary {} declared", secret.name),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::SecretBoundaryValidated,
                "validated",
                format!(
                    "secret boundary {} validated; handle is metadata, no secret value present",
                    secret.name
                ),
                EventFields::default(),
            );
            state.trace_ledger.record(
                EventType::SecretAccessDenied,
                "denied",
                format!(
                    "secret boundary {} access denied; no env, vault, or network lookup performed",
                    secret.name
                ),
                EventFields::default(),
            );
        }
        let execution_providers = match self.load_provider_contracts(bytecode, &mut state) {
            Ok(providers) => providers,
            Err(error) => {
                return ExecutionOutcome {
                    state,
                    result: Err(error),
                };
            }
        };
        for (name, kind) in execution_providers.entries() {
            state.trace_ledger.record(
                EventType::ProviderRegistered,
                "registered",
                format!("provider {name} registered as {kind:?}"),
                EventFields::default(),
            );
        }
        state.executable_providers = execution_providers
            .entries()
            .into_iter()
            .map(|(name, kind)| crate::ProviderSummary {
                name: name.to_owned(),
                kind: match kind {
                    ProviderKind::Simulated => "simulated",
                    ProviderKind::External => "external",
                }
                .into(),
                enabled: true,
            })
            .collect();
        state.provider_contracts = execution_providers
            .contract_entries()
            .into_iter()
            .map(|contract| crate::ProviderContractSummary {
                name: contract.name.clone(),
                kind: match contract.kind {
                    ProviderKind::Simulated => "simulated",
                    ProviderKind::External => "external",
                }
                .into(),
                enabled: contract.enabled,
                dry_run_only: contract.dry_run_only,
                requires_feature_flag: contract.requires_feature_flag,
                requires_explicit_approval: contract.requires_explicit_approval,
                allowed_targets: contract.allowed_targets.clone(),
                allowed_capabilities: contract.allowed_capabilities.clone(),
            })
            .collect();
        let steps = match ReactiveScheduler::new().run_with_registry(
            bytecode,
            &mut state,
            &injected,
            &execution_providers,
        ) {
            Ok(steps) => steps,
            Err(error) => {
                return ExecutionOutcome {
                    state,
                    result: Err(error),
                };
            }
        };
        let policy_report = self.evaluate_policy(bytecode, &mut state, &steps);
        let mailboxes = state
            .agents
            .iter()
            .filter_map(|agent| {
                state.mailboxes.get(agent).map(|mailbox| MailboxSummary {
                    agent: agent.clone(),
                    delivered: mailbox.delivered(),
                    processed: mailbox.processed(),
                })
            })
            .collect();
        let agent_state = state
            .agents
            .iter()
            .filter_map(|agent| state.agent_state.get(agent))
            .map(|agent_state| AgentStateSummary {
                agent: agent_state.agent.clone(),
                handled_count: agent_state.handled_count,
                checkpoints: agent_state.checkpoints.len(),
                last_message_type: agent_state.last_message_type.clone(),
            })
            .collect();
        let intrinsics = steps
            .iter()
            .flat_map(|step| {
                step.intrinsics.iter().map(|intrinsic| IntrinsicExecution {
                    agent: step.agent.clone(),
                    name: intrinsic.name.clone(),
                    argument: intrinsic.argument.clone(),
                    status: "ok".into(),
                })
            })
            .collect();
        let tool_calls = state
            .tool_calls
            .iter()
            .map(|call| crate::ToolCallSummary {
                agent: call.agent.clone(),
                tool: call.tool.clone(),
                capability: call.capability.clone(),
                status: call.status.clone(),
                mode: "dry-run".into(),
            })
            .collect();
        let model_calls = state
            .model_calls
            .iter()
            .map(|call| crate::ModelCallSummary {
                agent: call.agent.clone(),
                model: call.model.clone(),
                provider: call.provider.clone(),
                capability: call.capability.clone(),
                status: call.status.clone(),
                mode: "dry-run".into(),
            })
            .collect();
        let providers = state.executable_providers.clone();
        let provider_contracts = state.provider_contracts.clone();
        let provider_calls = state.provider_calls.clone();
        let trace = ReactiveExecutionTrace {
            vm_version: "0.21".into(),
            status: match state.status {
                RuntimeStatus::Completed => "completed",
                RuntimeStatus::Failed => "failed",
                RuntimeStatus::Initialized => "initialized",
                RuntimeStatus::Running => "running",
            }
            .into(),
            mode: "reactive-dry-run".into(),
            scheduler: "deterministic".into(),
            modules: bytecode.modules.clone(),
            imports: bytecode.imports.clone(),
            message_contracts: bytecode.types.clone(),
            passports: bytecode.passports.clone(),
            provider_harnesses: bytecode.provider_harnesses.clone(),
            features: bytecode.features.clone(),
            secrets: bytecode.secrets.clone(),
            injected,
            steps,
            mailboxes,
            agent_state,
            intrinsics,
            tool_calls,
            model_calls,
            providers,
            provider_contracts,
            provider_calls,
            policy_report,
            events: state.trace_ledger.events.clone(),
            security_checks: "passed".into(),
        };
        ExecutionOutcome {
            state,
            result: Ok(trace),
        }
    }

    fn evaluate_policy(
        &self,
        bytecode: &BytecodeProgram,
        state: &mut RuntimeState,
        steps: &[crate::ReactiveStep],
    ) -> PolicyReport {
        let evidence_context = policy_evidence_context(bytecode);
        let mut results = Vec::new();
        for assertion in &bytecode.assertions {
            let name = if assertion.name == "runtime_status" {
                "runtime_status completed"
            } else {
                assertion.name.as_str()
            };
            let mut evaluation = evaluate_rule(name, state, steps, evidence_context);
            if assertion.name == "runtime_status"
                && assertion.argument.as_deref() != Some("completed")
            {
                evaluation.passed = false;
                evaluation.reason = "runtime status assertion requires completed";
            }
            let passed = evaluation.passed;
            state.trace_ledger.record(
                if passed {
                    EventType::AssertionVerified
                } else {
                    EventType::AssertionFailed
                },
                if passed { "passed" } else { "failed" },
                format!("assertion {} evaluated", assertion.name),
                EventFields::default(),
            );
            results.push(AssertionResult {
                name: assertion.name.clone(),
                argument: assertion.argument.clone(),
                status: if passed { "passed" } else { "failed" }.into(),
                reason: (!passed).then(|| evaluation.reason.to_owned()),
            });
        }
        let legacy_failed = results.iter().any(|result| result.status == "failed");
        let mut failures = Vec::new();
        if legacy_failed {
            state.status = RuntimeStatus::Failed;
            let selected = bytecode
                .failures
                .iter()
                .find(|failure| failure.name == "PolicyViolation")
                .cloned()
                .unwrap_or(argorix_bytecode::BytecodeFailure {
                    name: "InternalPolicyViolation".into(),
                    action: "block".into(),
                    trace: "required".into(),
                });
            state.trace_ledger.record(
                EventType::FailureModeActivated,
                "active",
                format!("failure mode {} activated", selected.name),
                EventFields::default(),
            );
            failures.push(FailureActivation {
                name: selected.name,
                action: selected.action,
                trace: selected.trace,
            });
        }
        let mut policy_blocks = Vec::new();
        let mut actions = Vec::new();
        for policy in &bytecode.policies {
            let mut require_rules = Vec::new();
            let mut deny_rules = Vec::new();
            let mut violations = Vec::new();
            for declaration in &policy.rules {
                let evaluation = evaluate_rule(&declaration.rule, state, steps, evidence_context);
                let passed = if declaration.effect == "deny" {
                    !evaluation.passed
                } else {
                    evaluation.passed
                };
                let reason = (!passed).then(|| {
                    if declaration.effect == "deny" && declaration.rule == "external_execution" {
                        "external provider execution was attempted".to_owned()
                    } else if declaration.effect == "deny" {
                        format!("denied condition `{}` occurred", declaration.rule)
                    } else {
                        evaluation.reason.to_owned()
                    }
                });
                state.trace_ledger.record(
                    EventType::PolicyEvaluated,
                    if passed { "passed" } else { "failed" },
                    format!(
                        "policy {} {} {} evaluated",
                        policy.name, declaration.effect, declaration.rule
                    ),
                    EventFields::default(),
                );
                let result = PolicyRuleResult {
                    rule: declaration.rule.clone(),
                    effect: declaration.effect.clone(),
                    passed,
                    reason: reason.clone(),
                };
                if declaration.effect == "require" {
                    require_rules.push(result);
                } else {
                    deny_rules.push(result);
                }
                if let Some(reason) = reason {
                    state.trace_ledger.record(
                        EventType::PolicyViolation,
                        "violated",
                        format!(
                            "policy {} {} {} violated: {}",
                            policy.name, declaration.effect, declaration.rule, reason
                        ),
                        EventFields::default(),
                    );
                    violations.push(PolicyViolation {
                        rule: declaration.rule.clone(),
                        effect: declaration.effect.clone(),
                        reason,
                    });
                }
            }
            let passed = violations.is_empty();
            let action = (!passed)
                .then_some(policy.on_violation.as_ref())
                .flatten()
                .map(|violation| violation.action.clone());
            let trace_required = policy
                .on_violation
                .as_ref()
                .is_some_and(|violation| violation.trace_required);
            let status = if passed {
                "passed"
            } else {
                match action.as_deref() {
                    Some("block") => "failed",
                    Some("review") => "review_required",
                    Some("warn") => "warning",
                    _ => "violated",
                }
            };
            if let Some(action) = &action {
                state.trace_ledger.record(
                    EventType::PolicyActionActivated,
                    "active",
                    format!(
                        "policy {} action {} activated trace_required={trace_required}",
                        policy.name, action
                    ),
                    EventFields::default(),
                );
                actions.push(PolicyActionResult {
                    policy: policy.name.clone(),
                    action: action.clone(),
                    trace_required,
                });
            }
            policy_blocks.push(PolicyBlockResult {
                name: policy.name.clone(),
                passed,
                status: status.into(),
                require_rules,
                deny_rules,
                violations,
                action,
                trace_required,
            });
        }
        let has_block = actions.iter().any(|action| action.action == "block");
        let has_review = actions.iter().any(|action| action.action == "review");
        let has_warn = actions.iter().any(|action| action.action == "warn");
        let has_unhandled_violation = policy_blocks
            .iter()
            .any(|policy| !policy.passed && policy.action.is_none());
        let status = if legacy_failed || has_block {
            "failed"
        } else if has_review {
            "review_required"
        } else if has_warn {
            "warning"
        } else if has_unhandled_violation {
            "violated"
        } else {
            "passed"
        };
        state.trace_ledger.record(
            EventType::PolicyReportGenerated,
            status,
            "policy report generated",
            EventFields::default(),
        );
        if has_block {
            state.fail("policy block action activated");
        }
        PolicyReport {
            status: status.into(),
            assertions: results,
            policy_blocks,
            actions,
            failures,
        }
    }
}

fn record_external_execution_policy_violations(
    bytecode: &BytecodeProgram,
    state: &mut RuntimeState,
) {
    for policy in &bytecode.policies {
        if !policy
            .rules
            .iter()
            .any(|rule| rule.effect == "deny" && rule.rule == "external_execution")
        {
            continue;
        }
        state.trace_ledger.record(
            EventType::PolicyEvaluated,
            "failed",
            format!("policy {} deny external_execution evaluated", policy.name),
            EventFields::default(),
        );
        state.trace_ledger.record(
            EventType::PolicyViolation,
            "violated",
            format!(
                "policy {} deny external_execution violated: external provider execution was attempted",
                policy.name
            ),
            EventFields::default(),
        );
        if let Some(violation) = &policy.on_violation {
            state.trace_ledger.record(
                EventType::PolicyActionActivated,
                "active",
                format!(
                    "policy {} action {} activated trace_required={}",
                    policy.name, violation.action, violation.trace_required
                ),
                EventFields::default(),
            );
        }
    }
}

fn blocked_external_provider<'a>(
    _bytecode: &'a BytecodeProgram,
    errors: &'a [BytecodeError],
) -> Option<&'a str> {
    errors.iter().find_map(|error| {
        let provider = match error {
            BytecodeError::UnknownToolProvider(provider)
            | BytecodeError::UnknownModelProvider(provider) => provider.as_str(),
            _ => return None,
        };
        (provider != "simulated").then_some(provider)
    })
}

/// Derive offline passport policy evidence from the bytecode.
///
/// These booleans feed the v0.19 Policy v2 passport rules. They never resolve
/// DIDs, ASN registrations, or registries; they only inspect declared metadata.
fn policy_evidence_context(bytecode: &BytecodeProgram) -> PolicyEvidenceContext {
    let passports = &bytecode.passports;
    let agent_passport_declared = !bytecode.agents.is_empty()
        && bytecode.agents.iter().all(|agent| {
            passports
                .iter()
                .any(|passport| passport.agent == agent.name)
        });
    let harnesses = &bytecode.provider_harnesses;
    let features = &bytecode.features;
    let secrets = &bytecode.secrets;
    PolicyEvidenceContext {
        agent_passport_declared,
        agent_passport_attested: !passports.is_empty()
            && passports
                .iter()
                .all(|passport| !passport.attestations.is_empty()),
        agent_data_residency_declared: !passports.is_empty()
            && passports
                .iter()
                .all(|passport| !passport.data_residency.is_empty()),
        agent_identity_declared: !passports.is_empty()
            && passports
                .iter()
                .all(|passport| !passport.identity.trim().is_empty()),
        provider_harness_declared: !harnesses.is_empty(),
        provider_harness_sandboxed: harnesses.iter().all(|harness| {
            matches!(harness.mode.as_str(), "dry_run" | "simulated")
                && harness.network == "denied"
                && harness.secrets == "denied"
                && matches!(harness.filesystem.as_str(), "none" | "read_only")
        }),
        provider_network_denied: harnesses.iter().all(|harness| harness.network == "denied"),
        provider_secrets_denied: harnesses.iter().all(|harness| harness.secrets == "denied"),
        provider_filesystem_restricted: harnesses
            .iter()
            .all(|harness| matches!(harness.filesystem.as_str(), "none" | "read_only")),
        external_provider_harnessed: bytecode
            .providers
            .iter()
            .filter(|provider| provider.kind == "external")
            .all(|provider| {
                harnesses
                    .iter()
                    .any(|harness| harness.provider == provider.name)
            }),
        feature_flags_declared: !features.is_empty(),
        features_default_disabled: !features.is_empty()
            && features.iter().all(|feature| feature.default == "disabled"),
        experimental_features_require_approval: features
            .iter()
            .filter(|feature| matches!(feature.status.as_str(), "experimental" | "preview"))
            .all(|feature| feature.requires_approval),
        secret_boundaries_declared: !secrets.is_empty(),
        secret_access_denied: !secrets.is_empty()
            && secrets.iter().all(|secret| secret.access == "denied"),
        // Secret declarations never carry secret material in v0.21; the bytecode
        // schema has no value field, so by construction values are always absent.
        secret_values_absent: true,
        external_provider_feature_gated: bytecode
            .providers
            .iter()
            .filter(|provider| provider.kind == "external")
            .all(|provider| {
                features.iter().any(|feature| {
                    feature.provider.as_deref() == Some(provider.name.as_str())
                        && feature.default == "disabled"
                        && feature.requires_approval
                })
            }),
        external_provider_secret_boundary_declared: bytecode
            .providers
            .iter()
            .filter(|provider| provider.kind == "external")
            .all(|provider| {
                secrets.iter().any(|secret| {
                    secret.provider.as_deref() == Some(provider.name.as_str())
                        && secret.access == "denied"
                        && secret.source == "none"
                })
            }),
        ..PolicyEvidenceContext::default()
    }
}

#[cfg(test)]
mod tests {
    use super::Vm;
    use crate::{EventType, InjectedMessage, RuntimeStatus, Scheduler};
    use argorix_bytecode::{
        BytecodeAgent, BytecodePolicy, BytecodePolicyRule, BytecodePolicyViolation,
        BytecodeProgram, BytecodeProviderContract, BytecodeProviderHarness, Instruction,
    };
    use serde_json::json;

    fn add_external_contract(bytecode: &mut BytecodeProgram, enabled: bool) {
        bytecode.bytecode_version = "0.11".into();
        let contract = BytecodeProviderContract {
            name: "OpenAI".into(),
            kind: "external".into(),
            enabled,
            dry_run_only: true,
            requires_feature_flag: true,
            requires_explicit_approval: true,
            allowed_targets: vec![],
            allowed_capabilities: vec![],
        };
        bytecode.providers.push(contract.clone());
        bytecode.instructions.insert(
            0,
            Instruction::DeclareProviderContract {
                name: contract.name,
                kind: contract.kind,
                enabled: contract.enabled,
                dry_run_only: contract.dry_run_only,
                requires_feature_flag: contract.requires_feature_flag,
                requires_explicit_approval: contract.requires_explicit_approval,
                allowed_targets: contract.allowed_targets,
                allowed_capabilities: contract.allowed_capabilities,
            },
        );
    }

    fn valid_bytecode() -> BytecodeProgram {
        BytecodeProgram {
            bytecode_version: "0.3".into(),
            language: "Argorix Lang".into(),
            module: "Example".into(),
            modules: vec![],
            imports: vec![],
            providers: vec![],
            provider_harnesses: vec![],
            features: vec![],
            secrets: vec![],
            assertions: vec![],
            policies: vec![],
            types: vec![],
            enums: vec![],
            failures: vec![],
            passports: vec![],
            agents: vec![BytecodeAgent {
                name: "Worker".into(),
                approval: "denied".into(),
            }],
            capabilities: vec![],
            tools: vec![],
            models: vec![],
            instructions: vec![
                Instruction::DeclareProtocol {
                    name: "Flow".into(),
                },
                Instruction::SendMessage {
                    from: "User".into(),
                    to: "Worker".into(),
                    act: "tell".into(),
                    message_type: "Ping".into(),
                },
                Instruction::End,
            ],
        }
    }

    #[test]
    fn initializes_runtime_state_from_bytecode() {
        let state = Vm::new().initialize(&valid_bytecode()).unwrap();
        assert_eq!(state.status, RuntimeStatus::Initialized);
        assert!(state.mailboxes.contains_key("Worker"));
    }

    #[test]
    fn dry_run_generates_protocol_trace_and_mailbox_summary() {
        let trace = Vm::new().run_dry(&valid_bytecode()).unwrap();
        assert_eq!(trace.status, "completed");
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.steps[0].message_type, "Ping");
        assert_eq!(trace.mailboxes[0].delivered, 1);
        assert_eq!(trace.mailboxes[0].processed, 1);
    }

    #[test]
    fn ledger_records_lifecycle_and_delivery_events() {
        let trace = Vm::new().run_dry(&valid_bytecode()).unwrap();
        for expected in [
            EventType::VmStarted,
            EventType::MessageDelivered,
            EventType::VmCompleted,
        ] {
            assert!(trace
                .events
                .iter()
                .any(|event| event.event_type == expected));
        }
    }

    #[test]
    fn failure_keeps_trace_ledger_in_runtime_state() {
        let mut invalid = valid_bytecode();
        invalid.instructions.pop();
        let mut state = Vm::new().initialize(&invalid).unwrap();
        let result = Scheduler::new().run(&invalid, &mut state);

        assert!(result.is_err());
        assert_eq!(state.status, RuntimeStatus::Failed);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }

    #[test]
    fn reactive_outcome_preserves_failed_runtime_state_and_ledger() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/prompt_defense_v05.argbc.json"
        ))
        .unwrap();
        bytecode.instructions.pop();

        let outcome = Vm::new().run_reactive_outcome(
            &bytecode,
            InjectedMessage {
                from: "User".into(),
                to: "PromptScanner".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
        );

        assert!(outcome.result.is_err());
        assert_eq!(outcome.state.status, RuntimeStatus::Failed);
        assert!(outcome
            .state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }

    #[test]
    fn json_trace_contains_mailboxes() {
        let trace = Vm::new().run_dry(&valid_bytecode()).unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "0.5");
        assert_eq!(json["scheduler"], "deterministic");
        assert_eq!(json["mailboxes"][0]["agent"], "Worker");
    }

    #[test]
    fn reactive_json_trace_has_expected_mode() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/prompt_defense_v05.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "PromptScanner".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["mode"], "reactive-dry-run");
        assert_eq!(json["steps"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn v06_json_contains_agent_state_and_intrinsics() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/prompt_defense_v06.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "PromptScanner".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "0.21");
        assert_eq!(json["agent_state"].as_array().unwrap().len(), 3);
        assert_eq!(json["intrinsics"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn v07_json_contains_tool_calls() {
        let bytecode: BytecodeProgram =
            serde_json::from_str(include_str!("../../../examples/tool_call_v07.argbc.json"))
                .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "0.21");
        assert_eq!(json["tool_calls"][0]["tool"], "WebSearch");
        assert_eq!(json["tool_calls"][0]["mode"], "dry-run");
    }

    #[test]
    fn v08_json_contains_model_calls() {
        let bytecode: BytecodeProgram =
            serde_json::from_str(include_str!("../../../examples/model_call_v08.argbc.json"))
                .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["vm_version"], "0.21");
        assert_eq!(json["model_calls"][0]["model"], "GuardModel");
        assert_eq!(json["model_calls"][0]["provider"], "simulated");
    }

    #[test]
    fn v09_policy_report_verifies_global_assertions() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(trace.status, "completed");
        assert_eq!(trace.policy_report.status, "passed");
        assert_eq!(trace.policy_report.assertions.len(), 6);
        assert!(trace
            .policy_report
            .assertions
            .iter()
            .all(|assertion| assertion.status == "passed"));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyReportGenerated));
    }

    #[test]
    fn provider_boundary_routes_tool_and_model_calls() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(trace.vm_version, "0.21");
        assert_eq!(trace.providers[0].name, "simulated");
        assert_eq!(trace.providers[0].kind, "simulated");
        assert_eq!(trace.provider_calls.len(), 2);
        assert_eq!(trace.provider_calls[0].kind, "tool");
        assert_eq!(trace.provider_calls[0].target, "WebSearch");
        assert_eq!(trace.provider_calls[1].kind, "model");
        assert_eq!(trace.provider_calls[1].target, "GuardModel");
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderSelected));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderRegistered));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderRequestCreated));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderResponseReceived));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderDryRunEnforced));

        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["providers"][0]["name"], "simulated");
        assert_eq!(json["provider_calls"][0]["kind"], "tool");
        assert_eq!(json["provider_calls"][1]["kind"], "model");
    }

    #[test]
    fn vm_loads_and_validates_provider_contracts_before_execution() {
        let mut bytecode = valid_bytecode();
        add_external_contract(&mut bytecode, false);
        let vm = Vm::new();
        let mut state = vm.initialize(&bytecode).unwrap();
        let registry = vm.load_provider_contracts(&bytecode, &mut state).unwrap();

        assert!(registry.contains("simulated"));
        assert!(registry.contains_contract("OpenAI"));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderContractDeclared));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderContractValidated));
    }

    #[test]
    fn vm_json_preserves_populated_provider_allowlists() {
        let bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_allowlists_v012.argbc.json"
        ))
        .unwrap();
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        assert_eq!(trace.vm_version, "0.21");
        assert_eq!(
            trace.provider_contracts[0].allowed_targets,
            vec!["GuardModel"]
        );
        assert_eq!(
            trace.provider_contracts[0].allowed_capabilities,
            vec!["model.invoke"]
        );
    }
    #[test]
    fn rejected_provider_contract_preserves_runtime_ledger() {
        let mut bytecode = valid_bytecode();
        add_external_contract(&mut bytecode, true);
        let vm = Vm::new();
        let mut state = vm.initialize(&bytecode).unwrap();
        let result = vm.load_provider_contracts(&bytecode, &mut state);

        assert!(result.is_err());
        assert_eq!(state.status, RuntimeStatus::Failed);
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ProviderContractRejected));
        assert!(state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::VmFailed));
    }

    #[test]
    fn reactive_json_lists_executable_providers_and_declarative_contracts_separately() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/provider_boundary_v010.argbc.json"
        ))
        .unwrap();
        add_external_contract(&mut bytecode, false);
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        let json = serde_json::to_value(trace).unwrap();

        assert_eq!(json["vm_version"], "0.21");
        assert_eq!(json["providers"][0]["name"], "simulated");
        assert_eq!(json["providers"][0]["enabled"], true);
        assert_eq!(json["provider_contracts"][0]["name"], "OpenAI");
        assert_eq!(json["provider_contracts"][0]["allowed_targets"], json!([]));
        assert_eq!(
            json["provider_contracts"][0]["allowed_capabilities"],
            json!([])
        );
    }

    #[test]
    fn v09_policy_failure_activates_declared_failure_mode() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        bytecode
            .assertions
            .iter_mut()
            .find(|assertion| assertion.name == "runtime_status")
            .unwrap()
            .argument = Some("failed".into());

        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(trace.status, "failed");
        assert_eq!(trace.policy_report.status, "failed");
        assert_eq!(trace.policy_report.failures[0].name, "PolicyViolation");
        assert_eq!(trace.policy_report.failures[0].action, "block");
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::AssertionFailed));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::FailureModeActivated));
    }

    #[test]
    fn policy_v2_separates_legacy_assertions_and_policy_blocks() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        bytecode.bytecode_version = "0.17".into();
        bytecode.policies = vec![BytecodePolicy {
            name: "ProviderSafety".into(),
            rules: vec![BytecodePolicyRule {
                effect: "deny".into(),
                rule: "external_execution".into(),
            }],
            on_violation: Some(BytecodePolicyViolation {
                action: "block".into(),
                trace_required: true,
            }),
        }];
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        assert_eq!(trace.policy_report.assertions.len(), 6);
        assert_eq!(trace.policy_report.policy_blocks.len(), 1);
        assert!(trace.policy_report.policy_blocks[0].passed);
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyDeclared));
        assert!(trace
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyEvaluated));
    }

    #[test]
    fn policy_v2_actions_apply_block_review_warn_and_no_action() {
        for (action, expected_status, expected_error) in [
            (Some("block"), "failed", true),
            (Some("review"), "review_required", false),
            (Some("warn"), "warning", false),
            (None, "violated", false),
        ] {
            let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
                "../../../examples/policy_assertions_v09.argbc.json"
            ))
            .unwrap();
            bytecode.bytecode_version = "0.17".into();
            bytecode.policies = vec![BytecodePolicy {
                name: format!("Policy{expected_status}"),
                rules: vec![BytecodePolicyRule {
                    effect: "require".into(),
                    rule: "evidence_bundle_verified".into(),
                }],
                on_violation: action.map(|action| BytecodePolicyViolation {
                    action: action.into(),
                    trace_required: true,
                }),
            }];
            let injection = InjectedMessage {
                from: "User".into(),
                to: "ResearchAgent".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            };
            let outcome = Vm::new().run_reactive_outcome(&bytecode, injection.clone());
            let trace = outcome.result.unwrap();
            assert_eq!(trace.policy_report.status, expected_status);
            assert_eq!(
                trace.status,
                if expected_error {
                    "failed"
                } else {
                    "completed"
                }
            );
            assert_eq!(
                Vm::new().run_reactive(&bytecode, injection).is_err(),
                expected_error
            );
            assert!(trace
                .events
                .iter()
                .any(|event| event.event_type == EventType::PolicyViolation));
        }
    }

    #[test]
    fn deny_external_execution_records_violation_for_blocked_attempt() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        bytecode.bytecode_version = "0.17".into();
        bytecode.models[0].provider = "OpenAI".into();
        bytecode.policies = vec![BytecodePolicy {
            name: "ProviderSafety".into(),
            rules: vec![BytecodePolicyRule {
                effect: "deny".into(),
                rule: "external_execution".into(),
            }],
            on_violation: Some(BytecodePolicyViolation {
                action: "block".into(),
                trace_required: true,
            }),
        }];
        let outcome = Vm::new().run_reactive_outcome(
            &bytecode,
            InjectedMessage {
                from: "User".into(),
                to: "ResearchAgent".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
        );
        assert!(outcome.result.is_err());
        assert!(outcome
            .state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ExternalProviderExecutionBlocked));
        assert!(outcome
            .state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyViolation));
        assert!(outcome
            .state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::PolicyActionActivated));
    }

    #[test]
    fn provider_harness_metadata_is_traced_and_policy_evaluated_offline() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        add_external_contract(&mut bytecode, false);
        bytecode.bytecode_version = "0.20".into();
        bytecode.provider_harnesses = vec![BytecodeProviderHarness {
            name: "OpenAIHarness".into(),
            provider: "OpenAI".into(),
            feature: None,
            secret: None,
            mode: "dry_run".into(),
            network: "denied".into(),
            secrets: "denied".into(),
            filesystem: "none".into(),
            max_steps: Some(10),
            timeout_ms: Some(1000),
            input_contract: None,
            output_contract: None,
            attestations: vec!["policy-check".into()],
        }];
        bytecode.policies = vec![BytecodePolicy {
            name: "HarnessPolicy".into(),
            rules: [
                "provider_harness_declared",
                "provider_harness_sandboxed",
                "provider_network_denied",
                "provider_secrets_denied",
                "provider_filesystem_restricted",
                "external_provider_harnessed",
            ]
            .into_iter()
            .map(|rule| BytecodePolicyRule {
                effect: "require".into(),
                rule: rule.into(),
            })
            .collect(),
            on_violation: Some(BytecodePolicyViolation {
                action: "review".into(),
                trace_required: true,
            }),
        }];

        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();

        assert_eq!(trace.vm_version, "0.21");
        assert_eq!(trace.provider_harnesses, bytecode.provider_harnesses);
        assert_eq!(trace.policy_report.status, "passed");
        for expected in [
            EventType::ProviderHarnessDeclared,
            EventType::ProviderHarnessValidated,
            EventType::ProviderHarnessSandboxed,
        ] {
            assert!(trace
                .events
                .iter()
                .any(|event| event.event_type == expected));
        }
    }

    #[test]
    fn provider_harness_policies_fail_when_declaration_or_coverage_is_missing() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        add_external_contract(&mut bytecode, false);
        bytecode.bytecode_version = "0.20".into();
        bytecode.policies = vec![BytecodePolicy {
            name: "HarnessPolicy".into(),
            rules: ["provider_harness_declared", "external_provider_harnessed"]
                .into_iter()
                .map(|rule| BytecodePolicyRule {
                    effect: "require".into(),
                    rule: rule.into(),
                })
                .collect(),
            on_violation: Some(BytecodePolicyViolation {
                action: "review".into(),
                trace_required: true,
            }),
        }];
        let trace = Vm::new()
            .run_reactive(
                &bytecode,
                InjectedMessage {
                    from: "User".into(),
                    to: "ResearchAgent".into(),
                    act: "tell".into(),
                    message_type: "UserPrompt".into(),
                },
            )
            .unwrap();
        assert_eq!(trace.policy_report.status, "review_required");
        let violations = &trace.policy_report.policy_blocks[0].violations;
        assert!(violations
            .iter()
            .any(|violation| violation.rule == "provider_harness_declared"));
        assert!(violations
            .iter()
            .any(|violation| violation.rule == "external_provider_harnessed"));
    }

    #[test]
    fn harness_does_not_make_external_provider_executable() {
        let mut bytecode: BytecodeProgram = serde_json::from_str(include_str!(
            "../../../examples/policy_assertions_v09.argbc.json"
        ))
        .unwrap();
        add_external_contract(&mut bytecode, false);
        bytecode.bytecode_version = "0.20".into();
        bytecode.provider_harnesses = vec![BytecodeProviderHarness {
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
        }];
        bytecode.models[0].provider = "OpenAI".into();
        let outcome = Vm::new().run_reactive_outcome(
            &bytecode,
            InjectedMessage {
                from: "User".into(),
                to: "ResearchAgent".into(),
                act: "tell".into(),
                message_type: "UserPrompt".into(),
            },
        );
        assert!(outcome.result.is_err());
        assert!(outcome
            .state
            .trace_ledger
            .events
            .iter()
            .any(|event| event.event_type == EventType::ExternalProviderExecutionBlocked));
    }
}
