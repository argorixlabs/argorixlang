use crate::symbols::{Symbols, COMMUNICATIVE_ACTS};
use argorix_parser::{
    ast::{
        ATrustCredentialContractDecl, ATrustCredentialMode, ATrustEvidenceMapCoverage,
        ATrustEvidenceMapMappingMode, ATrustEvidenceRequirement, ATrustExecution,
        ATrustHandshakeMode, ATrustIdentityFormat, ATrustIdentityStatus, ATrustIdentityValidation,
        ATrustMaterialBoundary, ATrustResolutionMode, ATrustSecurityClaims, AdapterExecution,
        AdapterFilesystem, AdapterKind, AdapterMode, AdapterNetwork, AdapterProfileApiStyle,
        AdapterProfileAuth, AdapterProfileExecution, AdapterProfileFamily, AdapterProfileNetwork,
        AdapterProfileSecrets, AdapterSecrets, Approval, CapabilityLevel, CryptoKind, CryptoStatus,
        CryptoStrength, DidLedgerMode, DidMethodStatus, DidResolutionMode, FeatureDefault,
        FeatureStatus, GovernanceAssurance, GovernanceControlCategory, GovernanceControlStatus,
        GovernanceDomain, GovernanceLevel, GovernanceReviewStatus, GovernanceRiskLevel,
        GovernanceScope, HandlerInstruction, HarnessFilesystem, HarnessMode, HarnessNetwork,
        HarnessSecrets, PolicyRule, PolicyRuleDecl, PolicyViolationAction, Program,
        RegulatoryAssessment, RegulatoryCoverage, RegulatoryObligationStatus, SecretAccess,
        SecretScope, SecretSource,
    },
    diagnostics::Diagnostic,
    span::Spanned,
};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Default)]
pub struct CheckOptions {
    pub allow_legacy_capabilities: bool,
}

pub fn check_program(program: &Program) -> Result<(), Vec<Diagnostic>> {
    check_program_with_options(program, CheckOptions::default())
}

pub fn check_program_with_options(
    program: &Program,
    options: CheckOptions,
) -> Result<(), Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let symbols = collect_symbols(program, &mut diagnostics);
    check_provider_contracts(program, &symbols, &mut diagnostics);
    check_provider_harnesses(program, &symbols, &mut diagnostics);
    check_features(program, &symbols, &mut diagnostics);
    check_secrets(program, &symbols, &mut diagnostics);
    check_adapters(program, &symbols, &mut diagnostics);
    check_adapter_profiles(program, &symbols, &mut diagnostics);
    check_cryptos(program, &symbols, &mut diagnostics);
    check_did_methods(program, &symbols, &mut diagnostics);
    check_atrust_boundaries(program, &symbols, &mut diagnostics);
    check_atrust_identities(program, &symbols, &mut diagnostics);
    check_atrust_credential_contracts(program, &symbols, &mut diagnostics);
    check_atrust_handshakes(program, &symbols, &mut diagnostics);
    check_trust_ledgers(program, &symbols, &mut diagnostics);
    check_mcp_bridge_contracts(program, &symbols, &mut diagnostics);
    check_a2a_bridge_contracts(program, &symbols, &mut diagnostics);
    check_atrust_evidence_maps(program, &symbols, &mut diagnostics);
    check_governance_profiles(program, &mut diagnostics);
    check_regulatory_mappings(program, &mut diagnostics);
    check_third_party_verifiers(program, &mut diagnostics);
    check_public_conformance_reports(program, &mut diagnostics);
    check_runtime_hardening_profiles(program, &mut diagnostics);
    check_threat_models(program, &mut diagnostics);
    check_spec_freezes(program, &mut diagnostics);
    check_release_candidates(program, &mut diagnostics);
    check_runtime_execution_profiles(program, &mut diagnostics);
    check_sandboxed_provider_adapters(program, &mut diagnostics);
    check_policies(program, &mut diagnostics);
    check_passports(program, &symbols, &mut diagnostics);
    check_tool_declarations(program, &symbols, &mut diagnostics);
    check_model_declarations(program, &symbols, &mut diagnostics);

    for type_decl in &program.types {
        let mut field_names = HashSet::new();
        for field in &type_decl.fields {
            report_duplicate(&mut field_names, &field.name, "field", &mut diagnostics);
            let field_type = field.field_type.value.source_name();
            if !field.field_type.value.is_primitive() && !symbols.is_field_type(field_type) {
                diagnostics.push(Diagnostic::new(
                    format!("unknown message field type `{field_type}`"),
                    field.field_type.span,
                ));
            }
        }
    }

    for agent in &program.agents {
        for receive in &agent.receives {
            require_message_type(&symbols, &receive.message_type, &mut diagnostics);
            if let Some(from) = &receive.from {
                require_participant(&symbols, from, "receive source", &mut diagnostics);
            }
        }
        for send in &agent.sends {
            require_message_type(&symbols, &send.message_type, &mut diagnostics);
            if !symbols.is_participant(&send.to.value) {
                diagnostics.push(Diagnostic::new(
                    format!("unknown send destination `{}`", send.to.value),
                    send.to.span,
                ));
            }
        }
        check_agent_capabilities(program, &symbols, agent, options, &mut diagnostics);
        check_handlers(program, &symbols, agent, &mut diagnostics);
    }

    for protocol in &program.protocols {
        for step in &protocol.steps {
            require_participant(
                &symbols,
                &step.from,
                "protocol participant",
                &mut diagnostics,
            );
            require_participant(&symbols, &step.to, "protocol participant", &mut diagnostics);
            require_message_type(&symbols, &step.message_type, &mut diagnostics);
            if !COMMUNICATIVE_ACTS.contains(&step.act.value.as_str()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "unsupported communicative act `{}`; allowed acts: {}",
                        step.act.value,
                        COMMUNICATIVE_ACTS.join(", ")
                    ),
                    step.act.span,
                ));
            }
            verify_protocol_step(program, &symbols, step, &mut diagnostics);
        }
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn check_provider_harnesses(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = HashSet::new();
    for harness in &program.harnesses {
        report_duplicate(&mut names, &harness.name, "harness", diagnostics);
        let harness_name = &harness.name.value;

        if harness.provider.value.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` is missing required field `provider`"),
                harness.provider.span,
            ));
        } else if !symbols.providers.contains(&harness.provider.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "harness `{harness_name}` references unknown provider `{}`",
                    harness.provider.value
                ),
                harness.provider.span,
            ));
        }

        match &harness.mode.value {
            HarnessMode::Unknown(value) if value.is_empty() => diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` is missing required field `mode`"),
                harness.mode.span,
            )),
            HarnessMode::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` has invalid mode `{value}`"),
                harness.mode.span,
            )),
            HarnessMode::DryRun | HarnessMode::Simulated => {}
        }
        match &harness.network.value {
            HarnessNetwork::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("harness `{harness_name}` is missing required field `network`"),
                    harness.network.span,
                ))
            }
            HarnessNetwork::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` has invalid network `{value}`"),
                harness.network.span,
            )),
            HarnessNetwork::Denied => {}
        }
        match &harness.secrets.value {
            HarnessSecrets::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("harness `{harness_name}` is missing required field `secrets`"),
                    harness.secrets.span,
                ))
            }
            HarnessSecrets::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` has invalid secrets `{value}`"),
                harness.secrets.span,
            )),
            HarnessSecrets::Denied => {}
        }
        match &harness.filesystem.value {
            HarnessFilesystem::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("harness `{harness_name}` is missing required field `filesystem`"),
                    harness.filesystem.span,
                ))
            }
            HarnessFilesystem::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("harness `{harness_name}` has invalid filesystem `{value}`"),
                harness.filesystem.span,
            )),
            HarnessFilesystem::None | HarnessFilesystem::ReadOnly => {}
        }

        for (label, limit) in [
            ("max_steps", harness.max_steps.as_ref()),
            ("timeout_ms", harness.timeout_ms.as_ref()),
        ] {
            if let Some(limit) = limit {
                if limit.value == 0 {
                    diagnostics.push(Diagnostic::new(
                        format!("harness `{harness_name}` {label} must be positive"),
                        limit.span,
                    ));
                }
            }
        }

        for (label, contract) in [
            ("input_contract", harness.input_contract.as_ref()),
            ("output_contract", harness.output_contract.as_ref()),
        ] {
            if let Some(contract) = contract {
                if !symbols.types.contains(&contract.value) {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "harness `{harness_name}` references unknown {label} `{}`",
                            contract.value
                        ),
                        contract.span,
                    ));
                }
            }
        }

        for attestation in &harness.attestations {
            if attestation.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("harness `{harness_name}` attestation entries must not be empty"),
                    attestation.span,
                ));
            }
        }

        // v0.21 boundary links: feature/secret references and provider coherence.
        let feature_decl = harness.feature.as_ref().and_then(|reference| {
            if !symbols.features.contains(&reference.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "harness `{harness_name}` references unknown feature `{}`",
                        reference.value
                    ),
                    reference.span,
                ));
                None
            } else {
                program
                    .features
                    .iter()
                    .find(|feature| feature.name.value == reference.value)
            }
        });
        let secret_decl = harness.secret.as_ref().and_then(|reference| {
            if !symbols.secrets.contains(&reference.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "harness `{harness_name}` references unknown secret `{}`",
                        reference.value
                    ),
                    reference.span,
                ));
                None
            } else {
                program
                    .secrets
                    .iter()
                    .find(|secret| secret.name.value == reference.value)
            }
        });

        let harness_provider =
            (!harness.provider.value.trim().is_empty()).then_some(harness.provider.value.as_str());
        if let (Some(harness_provider), Some(feature)) = (harness_provider, feature_decl) {
            if let Some(feature_provider) = &feature.provider {
                if feature_provider.value != harness_provider {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "harness `{harness_name}` provider `{harness_provider}` does not match feature `{}` provider `{}`",
                            feature.name.value, feature_provider.value
                        ),
                        harness.provider.span,
                    ));
                }
            }
        }
        if let (Some(harness_provider), Some(secret)) = (harness_provider, secret_decl) {
            if let Some(secret_provider) = &secret.provider {
                if secret_provider.value != harness_provider {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "harness `{harness_name}` provider `{harness_provider}` does not match secret `{}` provider `{}`",
                            secret.name.value, secret_provider.value
                        ),
                        harness.provider.span,
                    ));
                }
            }
        }
        if let (Some(feature_ref), Some(secret)) = (harness.feature.as_ref(), secret_decl) {
            if let Some(required_by) = &secret.required_by {
                if required_by.value != feature_ref.value {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "harness `{harness_name}` feature `{}` does not match secret `{}` required_by `{}`",
                            feature_ref.value, secret.name.value, required_by.value
                        ),
                        feature_ref.span,
                    ));
                }
            }
        }
    }
}

/// Validate v0.21 feature flag declarations.
///
/// Feature flags are governance metadata. Validation is structural and offline:
/// it never enables a provider, reads a secret, or performs network access.
fn check_features(program: &Program, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) {
    for feature in &program.features {
        let feature_name = &feature.name.value;

        if let Some(provider) = &feature.provider {
            if !symbols.providers.contains(&provider.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "feature `{feature_name}` references unknown provider `{}`",
                        provider.value
                    ),
                    provider.span,
                ));
            }
        }

        match &feature.status.value {
            FeatureStatus::Unknown(value) if value.is_empty() => diagnostics.push(Diagnostic::new(
                format!("feature `{feature_name}` is missing required field `status`"),
                feature.status.span,
            )),
            FeatureStatus::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("feature `{feature_name}` has invalid status `{value}`"),
                feature.status.span,
            )),
            _ => {}
        }

        match &feature.default.value {
            FeatureDefault::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("feature `{feature_name}` is missing required field `default`"),
                    feature.default.span,
                ))
            }
            FeatureDefault::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("feature `{feature_name}` has invalid default `{value}`"),
                feature.default.span,
            )),
            _ => {}
        }

        if let Some(purpose) = &feature.purpose {
            if purpose.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("feature `{feature_name}` purpose must not be empty"),
                    purpose.span,
                ));
            }
        }

        if feature.status.value.is_gated() && !feature.requires_approval {
            diagnostics.push(Diagnostic::new(
                format!(
                    "feature `{feature_name}` is `{}` and requires `requires approval`",
                    feature.status.value.source_name()
                ),
                feature.status.span,
            ));
        }

        // A feature linked to an external provider must default to disabled.
        if let Some(provider) = &feature.provider {
            let is_external = program.providers.iter().any(|declared| {
                declared.name.value == provider.value && declared.kind.value.as_str() == "external"
            });
            if is_external && feature.default.value != FeatureDefault::Disabled {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "feature `{feature_name}` is linked to external provider `{}` and must default to disabled",
                        provider.value
                    ),
                    feature.default.span,
                ));
            }
        }
    }
}

/// Validate v0.21 secret boundary declarations.
///
/// Secret declarations record boundary metadata only. They never contain secret
/// material; the parser already rejects forbidden value fields. This pass enforces
/// required fields and the denied/none invariants.
fn check_secrets(program: &Program, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) {
    for secret in &program.secrets {
        let secret_name = &secret.name.value;

        if secret.handle.value.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("secret `{secret_name}` is missing required field `handle`"),
                secret.handle.span,
            ));
        }

        if let Some(provider) = &secret.provider {
            if !symbols.providers.contains(&provider.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "secret `{secret_name}` references unknown provider `{}`",
                        provider.value
                    ),
                    provider.span,
                ));
            }
        }

        if let Some(required_by) = &secret.required_by {
            if !symbols.features.contains(&required_by.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "secret `{secret_name}` required_by references unknown feature `{}`",
                        required_by.value
                    ),
                    required_by.span,
                ));
            }
        }

        match &secret.scope.value {
            SecretScope::Unknown(value) if value.is_empty() => diagnostics.push(Diagnostic::new(
                format!("secret `{secret_name}` is missing required field `scope`"),
                secret.scope.span,
            )),
            SecretScope::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("secret `{secret_name}` has invalid scope `{value}`"),
                secret.scope.span,
            )),
            _ => {}
        }

        match &secret.access.value {
            SecretAccess::Denied => {}
            SecretAccess::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("secret `{secret_name}` is missing required field `access`"),
                    secret.access.span,
                ))
            }
            SecretAccess::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!(
                    "secret `{secret_name}` has invalid access `{value}`; only `denied` is allowed in v0.21"
                ),
                secret.access.span,
            )),
        }

        match &secret.source.value {
            SecretSource::None => {}
            SecretSource::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("secret `{secret_name}` is missing required field `source`"),
                    secret.source.span,
                ))
            }
            SecretSource::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!(
                    "secret `{secret_name}` has invalid source `{value}`; only `none` is allowed in v0.21"
                ),
                secret.source.span,
            )),
        }

        // Link coherence: if both providers exist and required_by points at a
        // feature with a provider, the secret/feature providers must match.
        if let (Some(secret_provider), Some(required_by)) =
            (secret.provider.as_ref(), secret.required_by.as_ref())
        {
            if let Some(feature) = program
                .features
                .iter()
                .find(|feature| feature.name.value == required_by.value)
            {
                if let Some(feature_provider) = &feature.provider {
                    if feature_provider.value != secret_provider.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "secret `{secret_name}` provider `{}` does not match feature `{}` provider `{}`",
                                secret_provider.value, feature.name.value, feature_provider.value
                            ),
                            secret_provider.span,
                        ));
                    }
                }
            }
        }
    }
}

fn check_adapters(program: &Program, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) {
    let mut names = HashSet::new();
    for adapter in &program.adapters {
        report_duplicate(&mut names, &adapter.name, "adapter", diagnostics);
        let adapter_name = &adapter.name.value;

        // provider (required)
        if adapter.provider.value.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("adapter `{adapter_name}` is missing required field `provider`"),
                adapter.provider.span,
            ));
        } else if !symbols.providers.contains(&adapter.provider.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "adapter `{adapter_name}` references unknown provider `{}`",
                    adapter.provider.value
                ),
                adapter.provider.span,
            ));
        }

        // feature reference (if present)
        if let Some(feature) = &adapter.feature {
            if !symbols.features.contains(&feature.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "adapter `{adapter_name}` references unknown feature `{}`",
                        feature.value
                    ),
                    feature.span,
                ));
            }
        }

        // secret reference (if present)
        if let Some(secret) = &adapter.secret {
            if !symbols.secrets.contains(&secret.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "adapter `{adapter_name}` references unknown secret `{}`",
                        secret.value
                    ),
                    secret.span,
                ));
            }
        }

        // harness reference (if present)
        if let Some(harness) = &adapter.harness {
            // harness names are not in symbols yet, check against program.harnesses directly
            if !program
                .harnesses
                .iter()
                .any(|h| h.name.value == harness.value)
            {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "adapter `{adapter_name}` references unknown harness `{}`",
                        harness.value
                    ),
                    harness.span,
                ));
            }
        }

        // kind (optional, validate if present)
        if let Some(kind) = &adapter.kind {
            if let AdapterKind::Unknown(value) = &kind.value {
                if !value.is_empty() {
                    // unknown but non-empty kind: still allow for future extensibility per spec
                }
            }
        }

        // vendor (optional) - non-empty check only when present
        if let Some(vendor) = &adapter.vendor {
            if vendor.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("adapter `{adapter_name}` has empty `vendor`"),
                    vendor.span,
                ));
            }
        }

        // mode (required)
        match &adapter.mode.value {
            AdapterMode::Unknown(value) if value.is_empty() => diagnostics.push(Diagnostic::new(
                format!("adapter `{adapter_name}` is missing required field `mode`"),
                adapter.mode.span,
            )),
            AdapterMode::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter `{adapter_name}` has invalid mode `{value}`"),
                adapter.mode.span,
            )),
            _ => {}
        }

        // execution (required, must be disabled in v0.22)
        match &adapter.execution.value {
            AdapterExecution::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("adapter `{adapter_name}` is missing required field `execution`"),
                    adapter.execution.span,
                ))
            }
            AdapterExecution::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter `{adapter_name}` has invalid execution `{value}`"),
                adapter.execution.span,
            )),
            AdapterExecution::Disabled => {}
        }

        // network (required, must be denied)
        match &adapter.network.value {
            AdapterNetwork::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("adapter `{adapter_name}` is missing required field `network`"),
                    adapter.network.span,
                ))
            }
            AdapterNetwork::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter `{adapter_name}` has invalid network `{value}`"),
                adapter.network.span,
            )),
            AdapterNetwork::Denied => {}
        }

        // secrets (required, must be denied)
        match &adapter.secrets.value {
            AdapterSecrets::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("adapter `{adapter_name}` is missing required field `secrets`"),
                    adapter.secrets.span,
                ))
            }
            AdapterSecrets::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter `{adapter_name}` has invalid secrets `{value}`"),
                adapter.secrets.span,
            )),
            AdapterSecrets::Denied => {}
        }

        // filesystem (required, none or read_only)
        match &adapter.filesystem.value {
            AdapterFilesystem::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("adapter `{adapter_name}` is missing required field `filesystem`"),
                    adapter.filesystem.span,
                ))
            }
            AdapterFilesystem::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter `{adapter_name}` has invalid filesystem `{value}`"),
                adapter.filesystem.span,
            )),
            AdapterFilesystem::None | AdapterFilesystem::ReadOnly => {}
        }

        // conformance items must be non-empty strings
        for item in &adapter.conformance {
            if item.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("adapter `{adapter_name}` has empty conformance item"),
                    item.span,
                ));
            }
        }

        // input/output contracts validated for existence later against types if present
    }

    // cross reference validations (provider/feature etc match)
    for adapter in &program.adapters {
        let adapter_name = &adapter.name.value;

        // provider matches on referenced feature/secret/harness when present
        let adapter_provider = &adapter.provider.value;

        if let Some(feat) = &adapter.feature {
            if let Some(feature_decl) = program.features.iter().find(|f| f.name.value == feat.value)
            {
                if let Some(fp) = &feature_decl.provider {
                    if fp.value != *adapter_provider {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "adapter `{}` provider `{}` does not match feature `{}` provider `{}`",
                                adapter_name, adapter_provider, feat.value, fp.value
                            ),
                            feat.span,
                        ));
                    }
                }
            }
        }

        if let Some(sec) = &adapter.secret {
            if let Some(secret_decl) = program.secrets.iter().find(|s| s.name.value == sec.value) {
                if let Some(sp) = &secret_decl.provider {
                    if sp.value != *adapter_provider {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "adapter `{}` provider `{}` does not match secret `{}` provider `{}`",
                                adapter_name, adapter_provider, sec.value, sp.value
                            ),
                            sec.span,
                        ));
                    }
                }
                if let Some(required_by) = &secret_decl.required_by {
                    if let Some(feat) = &adapter.feature {
                        if required_by.value != feat.value {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "adapter `{}` feature `{}` does not match secret `{}` required_by `{}`",
                                    adapter_name, feat.value, sec.value, required_by.value
                                ),
                                sec.span,
                            ));
                        }
                    }
                }
            }
        }

        if let Some(harn) = &adapter.harness {
            if let Some(harness_decl) = program
                .harnesses
                .iter()
                .find(|h| h.name.value == harn.value)
            {
                if harness_decl.provider.value != *adapter_provider {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "adapter `{}` provider `{}` does not match harness `{}` provider `{}`",
                            adapter_name, adapter_provider, harn.value, harness_decl.provider.value
                        ),
                        harn.span,
                    ));
                }
                if let Some(hf) = &harness_decl.feature {
                    if let Some(af) = &adapter.feature {
                        if hf.value != af.value {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "adapter `{}` feature `{}` does not match harness `{}` feature `{}`",
                                    adapter_name, af.value, harn.value, hf.value
                                ),
                                harn.span,
                            ));
                        }
                    }
                }
                if let Some(hs) = &harness_decl.secret {
                    if let Some(as_) = &adapter.secret {
                        if hs.value != as_.value {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "adapter `{}` secret `{}` does not match harness `{}` secret `{}`",
                                    adapter_name, as_.value, harn.value, hs.value
                                ),
                                harn.span,
                            ));
                        }
                    }
                }
            }
        }
    }

    // input/output contract type existence (if declared)
    for adapter in &program.adapters {
        let adapter_name = &adapter.name.value;
        if let Some(ic) = &adapter.input_contract {
            if !symbols.is_field_type(&ic.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "adapter `{adapter_name}` input_contract references unknown type `{}`",
                        ic.value
                    ),
                    ic.span,
                ));
            }
        }
        if let Some(oc) = &adapter.output_contract {
            if !symbols.is_field_type(&oc.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "adapter `{adapter_name}` output_contract references unknown type `{}`",
                        oc.value
                    ),
                    oc.span,
                ));
            }
        }
    }

    // external provider + adapter boundary rules (when provider is external)
    for adapter in &program.adapters {
        let prov = program
            .providers
            .iter()
            .find(|p| p.name.value == adapter.provider.value);
        if let Some(p) = prov {
            if p.kind.value.as_str() == "external" {
                let adapter_name = &adapter.name.value;
                // must have feature, secret, harness
                if adapter.feature.is_none() {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "adapter `{adapter_name}` references external provider but is missing `feature`"
                        ),
                        adapter.provider.span,
                    ));
                }
                if adapter.secret.is_none() {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "adapter `{adapter_name}` references external provider but is missing `secret`"
                        ),
                        adapter.provider.span,
                    ));
                }
                if adapter.harness.is_none() {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "adapter `{adapter_name}` references external provider but is missing `harness`"
                        ),
                        adapter.provider.span,
                    ));
                }
                // feature must be disabled + approval if present
                if let Some(feat_ref) = &adapter.feature {
                    if let Some(feat) = program
                        .features
                        .iter()
                        .find(|f| f.name.value == feat_ref.value)
                    {
                        if let FeatureDefault::Enabled = &feat.default.value {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "adapter `{adapter_name}` for external provider requires feature `{}` to default to disabled",
                                    feat_ref.value
                                ),
                                feat.default.span,
                            ));
                        }
                        if !feat.requires_approval {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "adapter `{adapter_name}` for external provider requires feature `{}` to require approval",
                                    feat_ref.value
                                ),
                                feat.name.span,
                            ));
                        }
                    }
                }
                // secret must deny + none
                if let Some(sec_ref) = &adapter.secret {
                    if let Some(sec) = program
                        .secrets
                        .iter()
                        .find(|s| s.name.value == sec_ref.value)
                    {
                        if let SecretAccess::Denied = &sec.access.value {
                        } else if matches!(&sec.access.value, SecretAccess::Unknown(v) if v.is_empty())
                        {
                        } else {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "adapter `{adapter_name}` secret boundary for external provider must have access denied"
                                ),
                                sec.access.span,
                            ));
                        }
                        if let SecretSource::None = &sec.source.value {
                        } else if matches!(&sec.source.value, SecretSource::Unknown(v) if v.is_empty())
                        {
                        } else {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "adapter `{adapter_name}` secret boundary for external provider must have source none"
                                ),
                                sec.source.span,
                            ));
                        }
                    }
                }
                // harness network/secrets denied
                if let Some(harn_ref) = &adapter.harness {
                    if let Some(harn) = program
                        .harnesses
                        .iter()
                        .find(|h| h.name.value == harn_ref.value)
                    {
                        if !matches!(&harn.network.value, HarnessNetwork::Denied) {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "adapter `{adapter_name}` harness for external provider must have network denied"
                                ),
                                harn.network.span,
                            ));
                        }
                        if !matches!(&harn.secrets.value, HarnessSecrets::Denied) {
                            diagnostics.push(Diagnostic::new(
                                format!(
                                    "adapter `{adapter_name}` harness for external provider must have secrets denied"
                                ),
                                harn.secrets.span,
                            ));
                        }
                    }
                }
            }
        }
    }
}

fn check_adapter_profiles(program: &Program, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) {
    let mut names = HashSet::new();
    for profile in &program.adapter_profiles {
        report_duplicate(&mut names, &profile.name, "adapter_profile", diagnostics);
        let profile_name = &profile.name.value;

        // required fields
        if profile.adapter.value.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("adapter_profile `{profile_name}` is missing required field `adapter`"),
                profile.adapter.span,
            ));
        } else if !symbols.adapters.contains(&profile.adapter.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "adapter_profile `{profile_name}` references unknown adapter `{}`",
                    profile.adapter.value
                ),
                profile.adapter.span,
            ));
        }

        if profile.provider.value.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("adapter_profile `{profile_name}` is missing required field `provider`"),
                profile.provider.span,
            ));
        } else if !symbols.providers.contains(&profile.provider.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "adapter_profile `{profile_name}` references unknown provider `{}`",
                    profile.provider.value
                ),
                profile.provider.span,
            ));
        }

        if profile.vendor.value.trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("adapter_profile `{profile_name}` is missing required field `vendor`"),
                profile.vendor.span,
            ));
        }

        // family / api_style / auth (required, allowed values)
        match &profile.family.value {
            AdapterProfileFamily::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("adapter_profile `{profile_name}` is missing required field `family`"),
                    profile.family.span,
                ))
            }
            AdapterProfileFamily::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter_profile `{profile_name}` has invalid family `{value}`"),
                profile.family.span,
            )),
            _ => {}
        }

        match &profile.api_style.value {
            AdapterProfileApiStyle::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "adapter_profile `{profile_name}` is missing required field `api_style`"
                    ),
                    profile.api_style.span,
                ))
            }
            AdapterProfileApiStyle::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter_profile `{profile_name}` has invalid api_style `{value}`"),
                profile.api_style.span,
            )),
            _ => {}
        }

        match &profile.auth.value {
            AdapterProfileAuth::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("adapter_profile `{profile_name}` is missing required field `auth`"),
                    profile.auth.span,
                ))
            }
            AdapterProfileAuth::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter_profile `{profile_name}` has invalid auth `{value}`"),
                profile.auth.span,
            )),
            _ => {}
        }

        // execution / network / secrets (required + strict values)
        match &profile.execution.value {
            AdapterProfileExecution::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "adapter_profile `{profile_name}` is missing required field `execution`"
                    ),
                    profile.execution.span,
                ))
            }
            AdapterProfileExecution::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter_profile `{profile_name}` has invalid execution `{value}`"),
                profile.execution.span,
            )),
            AdapterProfileExecution::Disabled => {}
        }

        match &profile.network.value {
            AdapterProfileNetwork::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("adapter_profile `{profile_name}` is missing required field `network`"),
                    profile.network.span,
                ))
            }
            AdapterProfileNetwork::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter_profile `{profile_name}` has invalid network `{value}`"),
                profile.network.span,
            )),
            AdapterProfileNetwork::Denied => {}
        }

        match &profile.secrets.value {
            AdapterProfileSecrets::Unknown(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("adapter_profile `{profile_name}` is missing required field `secrets`"),
                    profile.secrets.span,
                ))
            }
            AdapterProfileSecrets::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("adapter_profile `{profile_name}` has invalid secrets `{value}`"),
                profile.secrets.span,
            )),
            AdapterProfileSecrets::Denied => {}
        }

        // optional contract references
        if let Some(rc) = &profile.request_contract {
            if !symbols.is_field_type(&rc.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "adapter_profile `{profile_name}` request_contract references unknown type `{}`",
                        rc.value
                    ),
                    rc.span,
                ));
            }
        }
        if let Some(rc) = &profile.response_contract {
            if !symbols.is_field_type(&rc.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "adapter_profile `{profile_name}` response_contract references unknown type `{}`",
                        rc.value
                    ),
                    rc.span,
                ));
            }
        }

        // empty items
        for c in &profile.capabilities {
            if c.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("adapter_profile `{profile_name}` has empty capability"),
                    c.span,
                ));
            }
        }
        for c in &profile.required_conformance {
            if c.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("adapter_profile `{profile_name}` has empty required_conformance item"),
                    c.span,
                ));
            }
        }
    }

    // link + coherence validations
    for profile in &program.adapter_profiles {
        let profile_name = &profile.name.value;
        let p_adapter = &profile.adapter.value;
        let p_provider = &profile.provider.value;

        if let Some(adapter_decl) = program.adapters.iter().find(|a| a.name.value == *p_adapter) {
            if &adapter_decl.provider.value != p_provider {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "adapter_profile `{}` provider `{}` does not match adapter `{}` provider `{}`",
                        profile_name, p_provider, p_adapter, adapter_decl.provider.value
                    ),
                    profile.adapter.span,
                ));
            }
        }

        // when auth == secret_boundary and adapter has secret reference
        if matches!(&profile.auth.value, AdapterProfileAuth::SecretBoundary) {
            if let Some(adapter_decl) = program.adapters.iter().find(|a| a.name.value == *p_adapter)
            {
                if adapter_decl.secret.is_none() {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "adapter_profile `{profile_name}` uses auth secret_boundary but linked adapter has no secret boundary"
                        ),
                        profile.auth.span,
                    ));
                }
            }
        }
    }

    // external provider + profile/adapter boundary rules
    for profile in &program.adapter_profiles {
        if let Some(prov) = program
            .providers
            .iter()
            .find(|p| p.name.value == profile.provider.value)
        {
            if prov.kind.value.as_str() == "external" {
                let pname = &profile.name.value;
                let adapter_opt = program
                    .adapters
                    .iter()
                    .find(|a| a.name.value == profile.adapter.value);

                if let Some(ad) = adapter_opt {
                    if ad.feature.is_none() {
                        diagnostics.push(Diagnostic::new(
                            format!("adapter_profile `{pname}` for external provider requires linked adapter to declare feature"),
                            profile.adapter.span,
                        ));
                    }
                    if ad.secret.is_none() {
                        diagnostics.push(Diagnostic::new(
                            format!("adapter_profile `{pname}` for external provider requires linked adapter to declare secret"),
                            profile.adapter.span,
                        ));
                    }
                    if ad.harness.is_none() {
                        diagnostics.push(Diagnostic::new(
                            format!("adapter_profile `{pname}` for external provider requires linked adapter to declare harness"),
                            profile.adapter.span,
                        ));
                    }
                } else {
                    // unknown adapter already reported
                }

                // feature / secret / harness details on the referenced adapter
                if let Some(ad) = adapter_opt {
                    if let Some(fref) = &ad.feature {
                        if let Some(feat) =
                            program.features.iter().find(|f| f.name.value == fref.value)
                        {
                            if let FeatureDefault::Enabled = &feat.default.value {
                                diagnostics.push(Diagnostic::new(
                                    format!("adapter_profile `{pname}` requires feature `{}` default disabled for external provider", fref.value),
                                    feat.default.span,
                                ));
                            }
                            if !feat.requires_approval {
                                diagnostics.push(Diagnostic::new(
                                    format!("adapter_profile `{pname}` requires feature `{}` to require approval", fref.value),
                                    feat.name.span,
                                ));
                            }
                        }
                    }
                    if let Some(sref) = &ad.secret {
                        if let Some(sec) =
                            program.secrets.iter().find(|s| s.name.value == sref.value)
                        {
                            if !matches!(&sec.access.value, SecretAccess::Denied) {
                                diagnostics.push(Diagnostic::new(
                                    format!(
                                        "adapter_profile `{pname}` requires secret access denied"
                                    ),
                                    sec.access.span,
                                ));
                            }
                            if !matches!(&sec.source.value, SecretSource::None) {
                                diagnostics.push(Diagnostic::new(
                                    format!(
                                        "adapter_profile `{pname}` requires secret source none"
                                    ),
                                    sec.source.span,
                                ));
                            }
                        }
                    }
                    if let Some(href) = &ad.harness {
                        if let Some(h) = program
                            .harnesses
                            .iter()
                            .find(|hh| hh.name.value == href.value)
                        {
                            if !matches!(&h.network.value, HarnessNetwork::Denied) {
                                diagnostics.push(Diagnostic::new(
                                    format!(
                                        "adapter_profile `{pname}` requires harness network denied"
                                    ),
                                    h.network.span,
                                ));
                            }
                            if !matches!(&h.secrets.value, HarnessSecrets::Denied) {
                                diagnostics.push(Diagnostic::new(
                                    format!(
                                        "adapter_profile `{pname}` requires harness secrets denied"
                                    ),
                                    h.secrets.span,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // vendor-specific loose structural hints (OpenAI / Anthropic)
    for profile in &program.adapter_profiles {
        let pname = &profile.name.value;
        if profile.vendor.value == "openai"
            && !matches!(
                &profile.family.value,
                AdapterProfileFamily::Llm | AdapterProfileFamily::Custom
            )
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "adapter_profile `{pname}` for vendor openai should use family llm (or custom)"
                ),
                profile.family.span,
            ));
        }
        if profile.vendor.value == "anthropic"
            && !matches!(
                &profile.family.value,
                AdapterProfileFamily::Llm | AdapterProfileFamily::Custom
            )
        {
            diagnostics.push(Diagnostic::new(
                format!("adapter_profile `{pname}` for vendor anthropic should use family llm (or custom)"),
                profile.family.span,
            ));
        }
    }
}

fn check_cryptos(program: &Program, _symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) {
    let mut names = HashSet::new();
    for crypto in &program.cryptos {
        report_duplicate(&mut names, &crypto.name, "crypto", diagnostics);
        let crypto_name = &crypto.name.value;

        // required fields
        match &crypto.kind.value {
            CryptoKind::Unknown(value) if value.is_empty() => diagnostics.push(Diagnostic::new(
                format!("crypto `{crypto_name}` is missing required field `kind`"),
                crypto.kind.span,
            )),
            CryptoKind::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("crypto `{crypto_name}` has invalid kind `{value}`"),
                crypto.kind.span,
            )),
            _ => {}
        }

        match &crypto.status.value {
            CryptoStatus::Unknown(value) if value.is_empty() => diagnostics.push(Diagnostic::new(
                format!("crypto `{crypto_name}` is missing required field `status`"),
                crypto.status.span,
            )),
            CryptoStatus::Unknown(value) => diagnostics.push(Diagnostic::new(
                format!("crypto `{crypto_name}` has invalid status `{value}`"),
                crypto.status.span,
            )),
            _ => {}
        }

        match &crypto.strength.value {
            CryptoStrength::UnknownValue(value) if value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!("crypto `{crypto_name}` is missing required field `strength`"),
                    crypto.strength.span,
                ))
            }
            CryptoStrength::UnknownValue(value) => diagnostics.push(Diagnostic::new(
                format!("crypto `{crypto_name}` has invalid strength `{value}`"),
                crypto.strength.span,
            )),
            _ => {}
        }

        if crypto.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("crypto `{crypto_name}` is missing required field `purpose`"),
                crypto.name.span,
            ));
        } else {
            for p in &crypto.purpose {
                if p.value.trim().is_empty() {
                    diagnostics.push(Diagnostic::new(
                        format!("crypto `{crypto_name}` has empty purpose item"),
                        p.span,
                    ));
                }
            }
        }

        // optional numeric fields
        if let Some(ob) = &crypto.output_bits {
            if ob.value == 0 {
                diagnostics.push(Diagnostic::new(
                    format!("crypto `{crypto_name}` has invalid output_bits 0"),
                    ob.span,
                ));
            }
        }
        if let Some(mk) = &crypto.min_key_bits {
            if mk.value == 0 {
                diagnostics.push(Diagnostic::new(
                    format!("crypto `{crypto_name}` has invalid min_key_bits 0"),
                    mk.span,
                ));
            }
        }

        if let Some(n) = &crypto.notes {
            if n.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("crypto `{crypto_name}` has empty notes"),
                    n.span,
                ));
            }
        }
    }
}

fn check_provider_contracts(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = HashSet::new();
    for provider in &program.providers {
        report_duplicate(&mut names, &provider.name, "provider", diagnostics);
        if provider.name.value == "simulated" {
            diagnostics.push(Diagnostic::new(
                "`simulated` is the reserved executable provider and must not be declared as a provider contract",
                provider.name.span,
            ));
        }
        if provider.kind.value.as_str() != "external" {
            diagnostics.push(Diagnostic::new(
                format!(
                    "provider contract `{}` must use kind `external` in v0.15",
                    provider.name.value
                ),
                provider.kind.span,
            ));
        }
        if provider.enabled.value {
            diagnostics.push(Diagnostic::new(
                format!(
                    "external provider contract `{}` must be disabled",
                    provider.name.value
                ),
                provider.enabled.span,
            ));
        }
        if !provider.dry_run_only.value {
            diagnostics.push(Diagnostic::new(
                format!(
                    "external provider contract `{}` requires `dry_run_only true`",
                    provider.name.value
                ),
                provider.dry_run_only.span,
            ));
        }
        if !provider.requires_feature_flag {
            diagnostics.push(Diagnostic::new(
                format!(
                    "external provider contract `{}` requires `requires feature_flag`",
                    provider.name.value
                ),
                provider.name.span,
            ));
        }
        if !provider.requires_explicit_approval {
            diagnostics.push(Diagnostic::new(
                format!(
                    "external provider contract `{}` requires `requires approval`",
                    provider.name.value
                ),
                provider.name.span,
            ));
        }

        let mut targets = HashSet::new();
        for target in &provider.allowed_targets {
            if !targets.insert(target.value.clone()) {
                diagnostics.push(Diagnostic::new(
                    format!("duplicate allowed target `{}`", target.value),
                    target.span,
                ));
                continue;
            }
            let tool = program
                .tools
                .iter()
                .find(|item| item.name.value == target.value);
            let model = program
                .models
                .iter()
                .find(|item| item.name.value == target.value);
            let capability = match (tool, model) {
                (None, None) => {
                    diagnostics.push(Diagnostic::new(
                        format!("unknown allowlist target `{}`", target.value),
                        target.span,
                    ));
                    None
                }
                (Some(_), Some(_)) => {
                    diagnostics.push(Diagnostic::new(
                        format!("ambiguous allowlist target `{}`", target.value),
                        target.span,
                    ));
                    None
                }
                (Some(tool), None) => Some(tool.capability.value.as_str()),
                (None, Some(model)) => Some(model.capability.value.as_str()),
            };
            if let Some(capability) = capability {
                if !provider.allowed_capabilities.is_empty()
                    && !provider
                        .allowed_capabilities
                        .iter()
                        .any(|allowed| allowed.value == capability)
                {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "allowlist target `{}` requires capability `{capability}`",
                            target.value
                        ),
                        target.span,
                    ));
                }
            }
        }

        let mut capabilities = HashSet::new();
        for capability in &provider.allowed_capabilities {
            if !capabilities.insert(capability.value.clone()) {
                diagnostics.push(Diagnostic::new(
                    format!("duplicate allowed capability `{}`", capability.value),
                    capability.span,
                ));
            }
            if !symbols.capabilities.contains_key(&capability.value) {
                diagnostics.push(Diagnostic::new(
                    format!("unknown allowlist capability `{}`", capability.value),
                    capability.span,
                ));
            }
        }
    }
}
fn check_policies(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    const ASSERTIONS: [&str; 6] = [
        "no_unhandled_messages",
        "all_tool_calls_traced",
        "all_model_calls_traced",
        "all_intrinsics_traced",
        "halt_requires_trace",
        "runtime_status",
    ];
    for assertion in &program.assertions {
        if !ASSERTIONS.contains(&assertion.name.value.as_str()) {
            diagnostics.push(Diagnostic::new(
                format!("unknown policy assertion `{}`", assertion.name.value),
                assertion.name.span,
            ));
        }
        if assertion.name.value == "runtime_status"
            && assertion
                .argument
                .as_ref()
                .map(|value| value.value.as_str())
                != Some("completed")
        {
            diagnostics.push(Diagnostic::new(
                "`runtime_status` assertion requires argument `completed`",
                assertion.name.span,
            ));
        }
    }
    let mut policy_names = HashSet::new();
    for policy in &program.policies {
        if !policy_names.insert(policy.name.value.clone()) {
            diagnostics.push(Diagnostic::new(
                format!("duplicate policy `{}`", policy.name.value),
                policy.name.span,
            ));
        }
        let mut required = HashSet::new();
        let mut denied = HashSet::new();
        for declaration in &policy.rules {
            let rule = declaration.rule();
            if let PolicyRule::Unknown(value) = &rule.value {
                diagnostics.push(Diagnostic::new(
                    format!("unknown policy rule `{value}`"),
                    rule.span,
                ));
                continue;
            }
            let name = rule.value.source_name();
            let (current, opposite) = match declaration {
                PolicyRuleDecl::Require { .. } => (&mut required, &denied),
                PolicyRuleDecl::Deny { .. } => (&mut denied, &required),
            };
            if !current.insert(name.clone()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "duplicate {} rule `{name}` in policy `{}`",
                        declaration.effect(),
                        policy.name.value
                    ),
                    rule.span,
                ));
            }
            if opposite.contains(&name) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "contradictory policy rule `{name}` in policy `{}`",
                        policy.name.value
                    ),
                    rule.span,
                ));
            }
        }
        if let Some(violation) = &policy.violation {
            if let PolicyViolationAction::Unknown(value) = &violation.action.value {
                diagnostics.push(Diagnostic::new(
                    format!("invalid policy violation action `{value}`"),
                    violation.action.span,
                ));
            }
        }
    }
    let mut failures = HashSet::new();
    for failure in &program.failures {
        if !failures.insert(failure.name.value.clone()) {
            diagnostics.push(Diagnostic::new(
                format!("duplicate failure `{}`", failure.name.value),
                failure.name.span,
            ));
        }
        if !matches!(failure.action.value.as_str(), "block" | "review" | "halt") {
            diagnostics.push(Diagnostic::new(
                format!("invalid failure action `{}`", failure.action.value),
                failure.action.span,
            ));
        }
        if !failure.trace_required {
            diagnostics.push(Diagnostic::new(
                format!("failure `{}` requires `trace required`", failure.name.value),
                failure.name.span,
            ));
        }
    }
}

/// Validate v0.19 Agent Passport blocks.
///
/// Passports are sovereign-identity metadata only. Validation is structural and
/// offline: no DID resolution, ASN lookup, registry queries, or real country
/// verification beyond a basic ISO-like format check.
fn check_passports(program: &Program, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) {
    const RISK_LEVELS: [&str; 4] = ["low", "medium", "high", "critical"];
    const ASN_REGISTRIES: [&str; 6] = ["LACNIC", "ARIN", "RIPE", "APNIC", "AFRINIC", "UNKNOWN"];

    let mut passport_names = HashSet::new();
    let mut agents_with_passport: HashSet<String> = HashSet::new();
    for passport in &program.passports {
        let name = passport.name.value.clone();
        if !passport_names.insert(name.clone()) {
            diagnostics.push(Diagnostic::new(
                format!("duplicate passport `{name}`"),
                passport.name.span,
            ));
        }

        // Required, non-empty scalar fields.
        for (label, field) in [
            ("agent", &passport.agent),
            ("agent_name", &passport.agent_name),
            ("global_id", &passport.global_id),
            ("identity", &passport.identity),
            ("provider", &passport.provider),
            ("version", &passport.version),
            ("country", &passport.country),
            ("jurisdiction", &passport.jurisdiction),
            ("intent", &passport.intent),
            ("risk_level", &passport.risk_level),
        ] {
            if field.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("passport `{name}` is missing required field `{label}`"),
                    field.span,
                ));
            }
        }

        if passport.data_residency.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("passport `{name}` requires non-empty `data_residency`"),
                passport.name.span,
            ));
        }

        // Agent reference must resolve to a declared agent, at most one per agent.
        if !passport.agent.value.trim().is_empty() {
            if !symbols.agents.contains(&passport.agent.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "passport `{name}` references unknown agent `{}`",
                        passport.agent.value
                    ),
                    passport.agent.span,
                ));
            } else if !agents_with_passport.insert(passport.agent.value.clone()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "agent `{}` already has a passport; only one passport per agent is allowed",
                        passport.agent.value
                    ),
                    passport.agent.span,
                ));
            }
        }

        if !passport.country.value.trim().is_empty() && !is_iso_country(&passport.country.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "passport `{name}` country `{}` must use a 2-letter ISO-like code",
                    passport.country.value
                ),
                passport.country.span,
            ));
        }

        if !passport.risk_level.value.trim().is_empty()
            && !RISK_LEVELS.contains(&passport.risk_level.value.as_str())
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "passport `{name}` has invalid risk_level `{}`; allowed: {}",
                    passport.risk_level.value,
                    RISK_LEVELS.join(", ")
                ),
                passport.risk_level.span,
            ));
        }

        if let Some(asn) = &passport.asn {
            if !ASN_REGISTRIES.contains(&asn.registry.value.as_str()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "passport `{name}` has invalid asn registry `{}`; allowed: {}",
                        asn.registry.value,
                        ASN_REGISTRIES.join(", ")
                    ),
                    asn.registry.span,
                ));
            }
            if !is_valid_asn_number(&asn.number.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "passport `{name}` asn number `{}` must be an `AS`-prefixed value or explicit placeholder",
                        asn.number.value
                    ),
                    asn.number.span,
                ));
            }
            if !asn.country.value.trim().is_empty() && !is_iso_country(&asn.country.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "passport `{name}` asn country `{}` must use a 2-letter ISO-like code",
                        asn.country.value
                    ),
                    asn.country.span,
                ));
            }
        }
    }
}

fn is_iso_country(value: &str) -> bool {
    value.len() == 2
        && value
            .chars()
            .all(|character| character.is_ascii_uppercase())
}

fn is_valid_asn_number(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && (value.starts_with("AS") || value == "PLACEHOLDER")
}

fn check_model_declarations(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for model in &program.models {
        if model.provider.value != "simulated" {
            diagnostics.push(Diagnostic::new(
                format!(
                    "model `{}` uses unsupported provider `{}`; only `simulated` is allowed",
                    model.name.value, model.provider.value
                ),
                model.provider.span,
            ));
        }
        if !symbols.capabilities.contains_key(&model.capability.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "model `{}` references unknown capability `{}`",
                    model.name.value, model.capability.value
                ),
                model.capability.span,
            ));
        }
        require_message_type(symbols, &model.input, diagnostics);
        require_message_type(symbols, &model.output, diagnostics);
    }
}

fn check_tool_declarations(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for tool in &program.tools {
        if let Some(provider) = &tool.provider {
            if provider.value != "simulated" {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "tool `{}` uses unsupported provider `{}`; only `simulated` is allowed",
                        tool.name.value, provider.value
                    ),
                    provider.span,
                ));
            }
        }
        if !symbols.capabilities.contains_key(&tool.capability.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "tool `{}` references unknown capability `{}`",
                    tool.name.value, tool.capability.value
                ),
                tool.capability.span,
            ));
        }
        require_message_type(symbols, &tool.input, diagnostics);
        require_message_type(symbols, &tool.output, diagnostics);
    }
}

fn check_handlers(
    program: &Program,
    symbols: &Symbols,
    agent: &argorix_parser::ast::AgentDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for tool in &agent.tools {
        if !symbols.tools.contains(&tool.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "agent `{}` references unknown tool `{}`",
                    agent.name.value, tool.value
                ),
                tool.span,
            ));
        }
    }
    for model in &agent.models {
        if !symbols.models.contains(&model.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "agent `{}` references unknown model `{}`",
                    agent.name.value, model.value
                ),
                model.span,
            ));
        }
    }
    let mut handled_types = HashSet::new();
    for handler in &agent.handlers {
        require_message_type(symbols, &handler.message_type, diagnostics);
        if !handled_types.insert(handler.message_type.value.clone()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "agent `{}` has duplicate handlers for `{}`",
                    agent.name.value, handler.message_type.value
                ),
                handler.message_type.span,
            ));
        }
        if !agent
            .receives
            .iter()
            .any(|receive| receive.message_type.value == handler.message_type.value)
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "handler `on {}` in agent `{}` requires `receives {}`",
                    handler.message_type.value, agent.name.value, handler.message_type.value
                ),
                handler.message_type.span,
            ));
        }

        for instruction in &handler.instructions {
            match instruction {
                HandlerInstruction::Emit { message_type, to } => {
                    require_message_type(symbols, message_type, diagnostics);
                    if !symbols.is_participant(&to.value) {
                        diagnostics.push(Diagnostic::new(
                            format!("unknown emit destination `{}`", to.value),
                            to.span,
                        ));
                    }
                    if !agent.sends.iter().any(|send| {
                        send.message_type.value == message_type.value && send.to.value == to.value
                    }) {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "handler emit in agent `{}` requires `sends {} to {}`",
                                agent.name.value, message_type.value, to.value
                            ),
                            message_type.span,
                        ));
                    }
                }
                HandlerInstruction::Trace { binding } => {
                    if binding.value != handler.binding.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "trace binding `{}` does not match handler binding `{}`",
                                binding.value, handler.binding.value
                            ),
                            binding.span,
                        ));
                    }
                }
                HandlerInstruction::Halt { span } => {
                    if !agent
                        .capabilities
                        .iter()
                        .any(|capability| capability.value == "runtime.halt")
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` uses `halt` without capability `runtime.halt`",
                                agent.name.value
                            ),
                            *span,
                        ));
                    }
                }
                HandlerInstruction::IntrinsicCall { name, argument } => {
                    if !matches!(name.value.as_str(), "facu" | "marron") {
                        diagnostics.push(Diagnostic::new(
                            format!("unknown runtime intrinsic `{}`", name.value),
                            name.span,
                        ));
                        continue;
                    }
                    if argument.value != handler.binding.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "intrinsic `{}` argument `{}` does not match handler binding `{}`",
                                name.value, argument.value, handler.binding.value
                            ),
                            argument.span,
                        ));
                    }
                    let required = if name.value == "facu" {
                        "state.write"
                    } else {
                        "runtime.guard"
                    };
                    if !agent
                        .capabilities
                        .iter()
                        .any(|capability| capability.value == required)
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` uses `{}` without capability `{required}`",
                                agent.name.value, name.value
                            ),
                            name.span,
                        ));
                    }
                }
                HandlerInstruction::CallTool { tool, binding } => {
                    let Some(declaration) = program
                        .tools
                        .iter()
                        .find(|item| item.name.value == tool.value)
                    else {
                        diagnostics.push(Diagnostic::new(
                            format!("unknown tool `{}`", tool.value),
                            tool.span,
                        ));
                        continue;
                    };
                    if !agent
                        .tools
                        .iter()
                        .any(|allowed| allowed.value == tool.value)
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` calls tool `{}` without declaring it in `tools`",
                                agent.name.value, tool.value
                            ),
                            tool.span,
                        ));
                    }
                    if binding.value != handler.binding.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "tool call binding `{}` does not match handler binding `{}`",
                                binding.value, handler.binding.value
                            ),
                            binding.span,
                        ));
                    }
                    if declaration.input.value != handler.message_type.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "tool `{}` expects `{}` but handler receives `{}`",
                                tool.value, declaration.input.value, handler.message_type.value
                            ),
                            tool.span,
                        ));
                    }
                    if !agent
                        .capabilities
                        .iter()
                        .any(|capability| capability.value == declaration.capability.value)
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` calls tool `{}` without capability `{}`",
                                agent.name.value, tool.value, declaration.capability.value
                            ),
                            tool.span,
                        ));
                    } else if symbols
                        .capabilities
                        .get(&declaration.capability.value)
                        .is_some_and(|level| {
                            matches!(
                                level,
                                CapabilityLevel::Restricted | CapabilityLevel::Dangerous
                            )
                        })
                        && agent.effective_approval() != Approval::Granted
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` calls restricted tool `{}` without approval",
                                agent.name.value, tool.value
                            ),
                            tool.span,
                        ));
                    }
                }
                HandlerInstruction::AskModel { model, binding } => {
                    let Some(declaration) = program
                        .models
                        .iter()
                        .find(|item| item.name.value == model.value)
                    else {
                        diagnostics.push(Diagnostic::new(
                            format!("unknown model `{}`", model.value),
                            model.span,
                        ));
                        continue;
                    };
                    if !agent
                        .models
                        .iter()
                        .any(|allowed| allowed.value == model.value)
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` asks model `{}` without declaring it in `models`",
                                agent.name.value, model.value
                            ),
                            model.span,
                        ));
                    }
                    if binding.value != handler.binding.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "model call binding `{}` does not match handler binding `{}`",
                                binding.value, handler.binding.value
                            ),
                            binding.span,
                        ));
                    }
                    if declaration.input.value != handler.message_type.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "model `{}` expects `{}` but handler receives `{}`",
                                model.value, declaration.input.value, handler.message_type.value
                            ),
                            model.span,
                        ));
                    }
                    if !agent
                        .capabilities
                        .iter()
                        .any(|capability| capability.value == declaration.capability.value)
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` asks model `{}` without capability `{}`",
                                agent.name.value, model.value, declaration.capability.value
                            ),
                            model.span,
                        ));
                    } else if symbols
                        .capabilities
                        .get(&declaration.capability.value)
                        .is_some_and(|level| {
                            matches!(
                                level,
                                CapabilityLevel::Restricted | CapabilityLevel::Dangerous
                            )
                        })
                        && agent.effective_approval() != Approval::Granted
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "agent `{}` asks restricted model `{}` without approval",
                                agent.name.value, model.value
                            ),
                            model.span,
                        ));
                    }
                }
            }
        }
    }
}

fn collect_symbols(program: &Program, diagnostics: &mut Vec<Diagnostic>) -> Symbols {
    let mut symbols = Symbols::default();
    for provider in &program.providers {
        symbols.providers.insert(provider.name.value.clone());
    }
    for type_decl in &program.types {
        report_duplicate(&mut symbols.types, &type_decl.name, "type", diagnostics);
    }
    for enum_decl in &program.enums {
        report_duplicate(&mut symbols.enums, &enum_decl.name, "enum", diagnostics);
    }
    for agent in &program.agents {
        report_duplicate(&mut symbols.agents, &agent.name, "agent", diagnostics);
    }
    for tool in &program.tools {
        report_duplicate(&mut symbols.tools, &tool.name, "tool", diagnostics);
    }
    for model in &program.models {
        report_duplicate(&mut symbols.models, &model.name, "model", diagnostics);
    }
    for feature in &program.features {
        report_duplicate(&mut symbols.features, &feature.name, "feature", diagnostics);
    }
    for secret in &program.secrets {
        report_duplicate(&mut symbols.secrets, &secret.name, "secret", diagnostics);
    }
    for adapter in &program.adapters {
        report_duplicate(&mut symbols.adapters, &adapter.name, "adapter", diagnostics);
    }
    for profile in &program.adapter_profiles {
        report_duplicate(
            &mut symbols.adapter_profiles,
            &profile.name,
            "adapter_profile",
            diagnostics,
        );
    }
    for crypto in &program.cryptos {
        report_duplicate(&mut symbols.cryptos, &crypto.name, "crypto", diagnostics);
    }
    for d in &program.did_methods {
        report_duplicate(&mut symbols.did_methods, &d.name, "did_method", diagnostics);
    }
    for a in &program.atrust_boundaries {
        report_duplicate(
            &mut symbols.atrust_boundaries,
            &a.name,
            "atrust_boundary",
            diagnostics,
        );
    }
    for i in &program.atrust_identities {
        report_duplicate(
            &mut symbols.atrust_identities,
            &i.name,
            "atrust_identity",
            diagnostics,
        );
    }
    for capability in &program.capabilities {
        if symbols
            .capabilities
            .insert(capability.name.value.clone(), capability.level.value)
            .is_some()
        {
            diagnostics.push(Diagnostic::new(
                format!("duplicate capability `{}`", capability.name.value),
                capability.name.span,
            ));
        }
    }
    symbols
}

fn check_agent_capabilities(
    program: &Program,
    symbols: &Symbols,
    agent: &argorix_parser::ast::AgentDecl,
    options: CheckOptions,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if program.capabilities.is_empty() && options.allow_legacy_capabilities {
        return;
    }

    for capability in &agent.capabilities {
        let Some(level) = symbols.capabilities.get(&capability.value) else {
            diagnostics.push(Diagnostic::new(
                format!(
                    "Unknown capability {} used by agent {}.",
                    capability.value, agent.name.value
                ),
                capability.span,
            ));
            continue;
        };

        if matches!(
            level,
            CapabilityLevel::Restricted | CapabilityLevel::Dangerous
        ) && agent.effective_approval() != Approval::Granted
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "Agent {} uses {} capability {} without approval.",
                    agent.name.value,
                    level.as_str(),
                    capability.value
                ),
                capability.span,
            ));
        }
    }
}

fn verify_protocol_step(
    program: &Program,
    symbols: &Symbols,
    step: &argorix_parser::ast::ProtocolStep,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if symbols.agents.contains(&step.from.value) {
        let sender = program
            .agents
            .iter()
            .find(|agent| agent.name.value == step.from.value)
            .expect("agent symbol must resolve to an AST declaration");
        let has_send = sender.sends.iter().any(|send| {
            send.message_type.value == step.message_type.value && send.to.value == step.to.value
        });
        if !has_send {
            diagnostics.push(Diagnostic::new(
                format!(
                    "Protocol step requires agent {} to declare `sends {} to {}`.",
                    step.from.value, step.message_type.value, step.to.value
                ),
                step.from.span,
            ));
        }
    }

    if symbols.agents.contains(&step.to.value) {
        let receiver = program
            .agents
            .iter()
            .find(|agent| agent.name.value == step.to.value)
            .expect("agent symbol must resolve to an AST declaration");
        let has_receive = receiver.receives.iter().any(|receive| {
            receive.message_type.value == step.message_type.value
                && receive
                    .from
                    .as_ref()
                    .is_none_or(|from| from.value == step.from.value)
        });
        if !has_receive {
            diagnostics.push(Diagnostic::new(
                format!(
                    "Protocol step requires agent {} to declare `receives {} from {}`.",
                    step.to.value, step.message_type.value, step.from.value
                ),
                step.to.span,
            ));
        }
    }
}

fn report_duplicate(
    names: &mut HashSet<String>,
    name: &Spanned<String>,
    kind: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !names.insert(name.value.clone()) {
        diagnostics.push(Diagnostic::new(
            format!("duplicate {kind} `{}`", name.value),
            name.span,
        ));
    }
}

fn require_message_type(
    symbols: &Symbols,
    message_type: &Spanned<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !symbols.types.contains(&message_type.value) {
        diagnostics.push(Diagnostic::new(
            format!(
                "unknown message type `{}`; messages must reference a declared `type`",
                message_type.value
            ),
            message_type.span,
        ));
    }
}

fn require_participant(
    symbols: &Symbols,
    participant: &Spanned<String>,
    role: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !symbols.is_participant(&participant.value) {
        diagnostics.push(Diagnostic::new(
            format!("unknown {role} `{}`", participant.value),
            participant.span,
        ));
    }
}

fn check_did_methods(program: &Program, symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) {
    let mut names = HashSet::new();
    for d in &program.did_methods {
        report_duplicate(&mut names, &d.name, "did_method", diagnostics);

        // required
        if matches!(&d.status.value, DidMethodStatus::Unknown(v) if v.is_empty()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "did_method `{}` is missing required field `status`",
                    d.name.value
                ),
                d.status.span,
            ));
        }
        if matches!(&d.resolution.value, DidResolutionMode::Unknown(v) if v.is_empty()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "did_method `{}` is missing required field `resolution`",
                    d.name.value
                ),
                d.resolution.span,
            ));
        }
        if matches!(&d.ledger.value, DidLedgerMode::Unknown(v) if v.is_empty()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "did_method `{}` is missing required field `ledger`",
                    d.name.value
                ),
                d.ledger.span,
            ));
        }
        if d.crypto_boundary.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "did_method `{}` is missing required field `crypto_boundary`",
                    d.name.value
                ),
                d.crypto_boundary.span,
            ));
        } else if !symbols.cryptos.contains(&d.crypto_boundary.value)
            && !program
                .crypto_boundaries
                .iter()
                .any(|cb| cb.name.value == d.crypto_boundary.value)
        {
            // crypto_boundary is from crypto_boundaries, but we check later in atrust or use symbols if extended
        }
        if d.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "did_method `{}` is missing required field `purpose`",
                    d.name.value
                ),
                d.name.span,
            ));
        }
        for p in &d.purpose {
            if p.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new("purpose entries must be non-empty", p.span));
            }
        }
        // resolution restrictions
        if matches!(&d.resolution.value, DidResolutionMode::Unknown(v) if v == "remote" || v == "network" || v == "live" || v == "distributed")
        {
            diagnostics.push(Diagnostic::new(
                "resolution `remote`/`network`/`live`/`distributed` not permitted in v0.26",
                d.resolution.span,
            ));
        }
    }
}

fn check_atrust_boundaries(
    program: &Program,
    _symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = HashSet::new();
    for a in &program.atrust_boundaries {
        report_duplicate(&mut names, &a.name, "atrust_boundary", diagnostics);

        if a.crypto_boundary.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` is missing required field `crypto_boundary`",
                    a.name.value
                ),
                a.crypto_boundary.span,
            ));
        }
        if a.did_methods.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` is missing required field `did_methods`",
                    a.name.value
                ),
                a.name.span,
            ));
        }
        for dm in &a.did_methods {
            if dm.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    "did_methods entries must be non-empty",
                    dm.span,
                ));
            }
            // existence and not denied will be enforced; simple check
        }
        if matches!(&a.identity_format.value, ATrustIdentityFormat::Unknown(v) if v.is_empty()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` is missing required field `identity_format`",
                    a.name.value
                ),
                a.identity_format.span,
            ));
        }
        if matches!(&a.credential_mode.value, ATrustCredentialMode::Unknown(v) if v.is_empty()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` is missing required field `credential_mode`",
                    a.name.value
                ),
                a.credential_mode.span,
            ));
        }
        if matches!(&a.handshake.value, ATrustHandshakeMode::Unknown(v) if v.is_empty()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` is missing required field `handshake`",
                    a.name.value
                ),
                a.handshake.span,
            ));
        }
        if matches!(&a.resolution.value, ATrustResolutionMode::Unknown(v) if v.is_empty()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` is missing required field `resolution`",
                    a.name.value
                ),
                a.resolution.span,
            ));
        }
        if matches!(&a.key_material.value, ATrustMaterialBoundary::Unknown(v) if v.is_empty() || v != "denied")
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` requires key_material `denied`",
                    a.name.value
                ),
                a.key_material.span,
            ));
        }
        if matches!(&a.secret_material.value, ATrustMaterialBoundary::Unknown(v) if v.is_empty() || v != "denied")
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` requires secret_material `denied`",
                    a.name.value
                ),
                a.secret_material.span,
            ));
        }
        if matches!(&a.execution.value, ATrustExecution::Unknown(v) if v.is_empty() || v != "disabled")
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` requires execution `disabled`",
                    a.name.value
                ),
                a.execution.span,
            ));
        }
        if matches!(&a.security_claims.value, ATrustSecurityClaims::Unknown(v) if v.is_empty() || v != "none")
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` requires security_claims `none`",
                    a.name.value
                ),
                a.security_claims.span,
            ));
        }
        if a.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_boundary `{}` is missing required field `purpose`",
                    a.name.value
                ),
                a.name.span,
            ));
        }
        for p in &a.purpose {
            if p.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new("purpose entries must be non-empty", p.span));
            }
        }
        // forbid live values
        if matches!(&a.credential_mode.value, ATrustCredentialMode::Unknown(v) if v == "verified" || v == "live" || v == "remote")
        {
            diagnostics.push(Diagnostic::new(
                "credential_mode verified/live/remote not permitted",
                a.credential_mode.span,
            ));
        }
        if matches!(&a.handshake.value, ATrustHandshakeMode::Unknown(v) if v == "enabled" || v == "live" || v == "real" || v == "network")
        {
            diagnostics.push(Diagnostic::new(
                "handshake enabled/live/real/network not permitted",
                a.handshake.span,
            ));
        }
        if matches!(&a.resolution.value, ATrustResolutionMode::Unknown(v) if v == "remote" || v == "network" || v == "live" || v == "distributed")
        {
            diagnostics.push(Diagnostic::new(
                "resolution remote/network/live/distributed not permitted",
                a.resolution.span,
            ));
        }
    }
}

fn check_atrust_identities(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = HashSet::new();
    for i in &program.atrust_identities {
        report_duplicate(&mut names, &i.name, "atrust_identity", diagnostics);

        // subject must be agent
        if i.subject.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` is missing required field `subject`",
                    i.name.value
                ),
                i.subject.span,
            ));
        } else if !symbols.agents.contains(&i.subject.value)
            && !program
                .agents
                .iter()
                .any(|a| a.name.value == i.subject.value)
        {
            // basic check; full may be in other pass
        }

        // did
        if i.did.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` is missing required field `did`",
                    i.name.value
                ),
                i.did.span,
            ));
        } else if !i.did.value.starts_with("did:") {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` did must start with 'did:'",
                    i.name.value
                ),
                i.did.span,
            ));
        }

        // method and boundary required
        if i.method.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` is missing required field `method`",
                    i.name.value
                ),
                i.method.span,
            ));
        }
        if i.boundary.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` is missing required field `boundary`",
                    i.name.value
                ),
                i.boundary.span,
            ));
        }

        // status etc.
        if matches!(&i.status.value, ATrustIdentityStatus::Unknown(v) if v.is_empty()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` is missing required field `status`",
                    i.name.value
                ),
                i.status.span,
            ));
        }
        if matches!(&i.validation.value, ATrustIdentityValidation::Unknown(v) if v.is_empty()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` is missing required field `validation`",
                    i.name.value
                ),
                i.validation.span,
            ));
        }
        if matches!(&i.resolution.value, ATrustResolutionMode::Unknown(v) if v.is_empty()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` is missing required field `resolution`",
                    i.name.value
                ),
                i.resolution.span,
            ));
        }
        if matches!(&i.key_material.value, ATrustMaterialBoundary::Unknown(v) if v.is_empty() || v != "denied")
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` requires key_material `denied`",
                    i.name.value
                ),
                i.key_material.span,
            ));
        }
        if matches!(&i.secret_material.value, ATrustMaterialBoundary::Unknown(v) if v.is_empty() || v != "denied")
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` requires secret_material `denied`",
                    i.name.value
                ),
                i.secret_material.span,
            ));
        }
        if matches!(&i.execution.value, ATrustExecution::Unknown(v) if v.is_empty() || v != "disabled")
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` requires execution `disabled`",
                    i.name.value
                ),
                i.execution.span,
            ));
        }
        if matches!(&i.evidence.value, ATrustEvidenceRequirement::Unknown(v) if v.is_empty() || v != "required")
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` requires evidence `required`",
                    i.name.value
                ),
                i.evidence.span,
            ));
        }
        if matches!(&i.security_claims.value, ATrustSecurityClaims::Unknown(v) if v.is_empty() || v != "none")
        {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` requires security_claims `none`",
                    i.name.value
                ),
                i.security_claims.span,
            ));
        }
        if i.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_identity `{}` is missing required field `purpose`",
                    i.name.value
                ),
                i.name.span,
            ));
        }
        for p in &i.purpose {
            if p.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new("purpose entries must be non-empty", p.span));
            }
        }
        // forbid live
        if matches!(&i.validation.value, ATrustIdentityValidation::Unknown(v) if v == "real" || v == "verified" || v == "live")
        {
            diagnostics.push(Diagnostic::new(
                "validation must be dry_run in v0.27",
                i.validation.span,
            ));
        }
        if matches!(&i.resolution.value, ATrustResolutionMode::Unknown(v) if v == "remote" || v == "network" || v == "live" || v == "distributed")
        {
            diagnostics.push(Diagnostic::new(
                "resolution remote/network/live/distributed not permitted",
                i.resolution.span,
            ));
        }
    }
}

#[allow(unused_variables, dead_code)]
fn check_atrust_credential_contracts(
    program: &Program,
    _symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = HashSet::new();
    for c in &program.atrust_credential_contracts {
        report_duplicate(
            &mut names,
            &c.name,
            "atrust_credential_contract",
            diagnostics,
        );

        if c.subject.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` is missing required field `subject`",
                    c.name.value
                ),
                c.subject.span,
            ));
        }
        if c.identity.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` is missing required field `identity`",
                    c.name.value
                ),
                c.identity.span,
            ));
        }
        if c.boundary.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` is missing required field `boundary`",
                    c.name.value
                ),
                c.boundary.span,
            ));
        }
        if c.method.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` is missing required field `method`",
                    c.name.value
                ),
                c.method.span,
            ));
        }
        if c.issuer_did.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` is missing required field `issuer_did`",
                    c.name.value
                ),
                c.issuer_did.span,
            ));
        }
        if c.holder_did.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` is missing required field `holder_did`",
                    c.name.value
                ),
                c.holder_did.span,
            ));
        }
        if c.credential_type.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` is missing required field `credential_type`",
                    c.name.value
                ),
                c.credential_type.span,
            ));
        }
        if c.schema.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` is missing required field `schema`",
                    c.name.value
                ),
                c.schema.span,
            ));
        }
        if c.claims.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` is missing required field `claims`",
                    c.name.value
                ),
                c.name.span,
            ));
        }
        if c.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` is missing required field `purpose`",
                    c.name.value
                ),
                c.name.span,
            ));
        }
        // verification must be declared_only
        if c.verification.value.source_name() != "declared_only" {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` requires verification `declared_only`",
                    c.name.value
                ),
                c.verification.span,
            ));
        }
        // similar for other
        if c.key_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` requires key_material `denied`",
                    c.name.value
                ),
                c.key_material.span,
            ));
        }
        if c.secret_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` requires secret_material `denied`",
                    c.name.value
                ),
                c.secret_material.span,
            ));
        }
        if c.execution.value.source_name() != "disabled" {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` requires execution `disabled`",
                    c.name.value
                ),
                c.execution.span,
            ));
        }
        if c.evidence.value.source_name() != "required" {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` requires evidence `required`",
                    c.name.value
                ),
                c.evidence.span,
            ));
        }
        if c.security_claims.value.source_name() != "none" {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_credential_contract `{}` requires security_claims `none`",
                    c.name.value
                ),
                c.security_claims.span,
            ));
        }
    }
}

fn check_atrust_handshakes(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use std::collections::HashMap;

    // Lookup maps for cross-reference validation against declared identities,
    // credential contracts, and boundaries. v0.29 handshakes are dry-run
    // governance metadata: we validate the declared trust flow, never execute it.
    let identities: HashMap<&str, &argorix_parser::ast::ATrustIdentityDecl> = program
        .atrust_identities
        .iter()
        .map(|i| (i.name.value.as_str(), i))
        .collect();
    let contracts: HashMap<&str, &ATrustCredentialContractDecl> = program
        .atrust_credential_contracts
        .iter()
        .map(|c| (c.name.value.as_str(), c))
        .collect();
    let boundaries: HashMap<&str, &argorix_parser::ast::ATrustBoundaryDecl> = program
        .atrust_boundaries
        .iter()
        .map(|b| (b.name.value.as_str(), b))
        .collect();

    let mut names = HashSet::new();
    for h in &program.atrust_handshakes {
        report_duplicate(&mut names, &h.name, "atrust_handshake", diagnostics);
        let name = h.name.value.clone();

        // --- required field presence ---
        if h.initiator.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` is missing required field `initiator`"),
                h.initiator.span,
            ));
        }
        if h.responder.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` is missing required field `responder`"),
                h.responder.span,
            ));
        }
        if h.initiator_identity.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` is missing required field `initiator_identity`"),
                h.initiator_identity.span,
            ));
        }
        if h.responder_identity.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` is missing required field `responder_identity`"),
                h.responder_identity.span,
            ));
        }
        if h.credential_contracts.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_handshake `{name}` is missing required field `credential_contracts`"
                ),
                h.name.span,
            ));
        }
        if h.boundary.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` is missing required field `boundary`"),
                h.boundary.span,
            ));
        }
        if h.method.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` is missing required field `method`"),
                h.method.span,
            ));
        }
        if h.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` is missing required field `purpose`"),
                h.name.span,
            ));
        }
        for p in &h.purpose {
            if p.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("atrust_handshake `{name}` purpose must not contain empty strings"),
                    p.span,
                ));
            }
        }
        if let Some(notes) = &h.notes {
            if notes.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("atrust_handshake `{name}` notes must not be empty"),
                    notes.span,
                ));
            }
        }

        // --- enumerated boundary values (dry-run only) ---
        if h.mode.value.source_name() != "dry_run" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` requires mode `dry_run`"),
                h.mode.span,
            ));
        }
        if !matches!(h.direction.value.source_name(), "one_way" | "mutual") {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` requires direction `one_way` or `mutual`"),
                h.direction.span,
            ));
        }
        if !matches!(
            h.challenge.value.source_name(),
            "declared_only" | "disabled"
        ) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_handshake `{name}` requires challenge `declared_only` or `disabled`"
                ),
                h.challenge.span,
            ));
        }
        if !matches!(h.response.value.source_name(), "declared_only" | "disabled") {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_handshake `{name}` requires response `declared_only` or `disabled`"
                ),
                h.response.span,
            ));
        }
        if !matches!(
            h.transcript.value.source_name(),
            "metadata_only" | "evidence_only"
        ) {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` requires transcript `metadata_only` or `evidence_only`"),
                h.transcript.span,
            ));
        }
        if !matches!(
            h.verification.value.source_name(),
            "declared_only" | "disabled"
        ) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_handshake `{name}` requires verification `declared_only` or `disabled`"
                ),
                h.verification.span,
            ));
        }
        if !matches!(
            h.resolution.value.source_name(),
            "disabled" | "embedded" | "local"
        ) {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` resolution must be `disabled`, `embedded`, or `local`"),
                h.resolution.span,
            ));
        }
        if h.network.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` requires network `denied`"),
                h.network.span,
            ));
        }
        if h.key_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` requires key_material `denied`"),
                h.key_material.span,
            ));
        }
        if h.secret_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` requires secret_material `denied`"),
                h.secret_material.span,
            ));
        }
        if h.execution.value.source_name() != "disabled" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` requires execution `disabled`"),
                h.execution.span,
            ));
        }
        if h.evidence.value.source_name() != "required" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` requires evidence `required`"),
                h.evidence.span,
            ));
        }
        if h.security_claims.value.source_name() != "none" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` requires security_claims `none`"),
                h.security_claims.span,
            ));
        }

        // --- cross-reference validation (only when references are present) ---
        if !h.initiator.value.is_empty() && !symbols.agents.contains(&h.initiator.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_handshake `{name}` references unknown initiator agent `{}`",
                    h.initiator.value
                ),
                h.initiator.span,
            ));
        }
        if !h.responder.value.is_empty() && !symbols.agents.contains(&h.responder.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_handshake `{name}` references unknown responder agent `{}`",
                    h.responder.value
                ),
                h.responder.span,
            ));
        }
        if !h.initiator.value.is_empty() && h.initiator.value == h.responder.value {
            diagnostics.push(Diagnostic::new(
                format!("atrust_handshake `{name}` initiator and responder must be distinct"),
                h.responder.span,
            ));
        }
        if !h.boundary.value.is_empty() && !symbols.atrust_boundaries.contains(&h.boundary.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_handshake `{name}` references unknown atrust_boundary `{}`",
                    h.boundary.value
                ),
                h.boundary.span,
            ));
        }
        if !h.method.value.is_empty() && !symbols.did_methods.contains(&h.method.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_handshake `{name}` references unknown did_method `{}`",
                    h.method.value
                ),
                h.method.span,
            ));
        }
        if let Some(b) = boundaries.get(h.boundary.value.as_str()) {
            if !h.method.value.is_empty()
                && !b.did_methods.iter().any(|m| m.value == h.method.value)
            {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_handshake `{name}` method `{}` is not allowed by boundary `{}`",
                        h.method.value, h.boundary.value
                    ),
                    h.method.span,
                ));
            }
        }

        // initiator identity
        if !h.initiator_identity.value.is_empty() {
            match identities.get(h.initiator_identity.value.as_str()) {
                None => diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_handshake `{name}` references unknown initiator_identity `{}`",
                        h.initiator_identity.value
                    ),
                    h.initiator_identity.span,
                )),
                Some(id) => {
                    if !h.initiator.value.is_empty() && id.subject.value != h.initiator.value {
                        diagnostics.push(Diagnostic::new(
                            format!("atrust_handshake `{name}` initiator_identity subject `{}` does not match initiator `{}`", id.subject.value, h.initiator.value),
                            h.initiator_identity.span,
                        ));
                    }
                    if !h.boundary.value.is_empty() && id.boundary.value != h.boundary.value {
                        diagnostics.push(Diagnostic::new(
                            format!("atrust_handshake `{name}` initiator_identity boundary `{}` does not match handshake boundary `{}`", id.boundary.value, h.boundary.value),
                            h.initiator_identity.span,
                        ));
                    }
                    if !h.method.value.is_empty() && id.method.value != h.method.value {
                        diagnostics.push(Diagnostic::new(
                            format!("atrust_handshake `{name}` initiator_identity method `{}` does not match handshake method `{}`", id.method.value, h.method.value),
                            h.initiator_identity.span,
                        ));
                    }
                }
            }
        }
        // responder identity
        if !h.responder_identity.value.is_empty() {
            match identities.get(h.responder_identity.value.as_str()) {
                None => diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_handshake `{name}` references unknown responder_identity `{}`",
                        h.responder_identity.value
                    ),
                    h.responder_identity.span,
                )),
                Some(id) => {
                    if !h.responder.value.is_empty() && id.subject.value != h.responder.value {
                        diagnostics.push(Diagnostic::new(
                            format!("atrust_handshake `{name}` responder_identity subject `{}` does not match responder `{}`", id.subject.value, h.responder.value),
                            h.responder_identity.span,
                        ));
                    }
                    if !h.boundary.value.is_empty() && id.boundary.value != h.boundary.value {
                        diagnostics.push(Diagnostic::new(
                            format!("atrust_handshake `{name}` responder_identity boundary `{}` does not match handshake boundary `{}`", id.boundary.value, h.boundary.value),
                            h.responder_identity.span,
                        ));
                    }
                    if !h.method.value.is_empty() && id.method.value != h.method.value {
                        diagnostics.push(Diagnostic::new(
                            format!("atrust_handshake `{name}` responder_identity method `{}` does not match handshake method `{}`", id.method.value, h.method.value),
                            h.responder_identity.span,
                        ));
                    }
                }
            }
        }
        // credential contracts must exist, bind to a participant identity, and
        // share the handshake boundary + method.
        for c in &h.credential_contracts {
            match contracts.get(c.value.as_str()) {
                None => diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_handshake `{name}` references unknown credential_contract `{}`",
                        c.value
                    ),
                    c.span,
                )),
                Some(contract) => {
                    let participant = contract.identity.value == h.initiator_identity.value
                        || contract.identity.value == h.responder_identity.value;
                    if !participant {
                        diagnostics.push(Diagnostic::new(
                            format!("atrust_handshake `{name}` credential_contract `{}` is not bound to a participant identity", c.value),
                            c.span,
                        ));
                    }
                    if !h.boundary.value.is_empty() && contract.boundary.value != h.boundary.value {
                        diagnostics.push(Diagnostic::new(
                            format!("atrust_handshake `{name}` credential_contract `{}` boundary `{}` does not match handshake boundary `{}`", c.value, contract.boundary.value, h.boundary.value),
                            c.span,
                        ));
                    }
                    if !h.method.value.is_empty() && contract.method.value != h.method.value {
                        diagnostics.push(Diagnostic::new(
                            format!("atrust_handshake `{name}` credential_contract `{}` method `{}` does not match handshake method `{}`", c.value, contract.method.value, h.method.value),
                            c.span,
                        ));
                    }
                }
            }
        }
    }
}

fn check_trust_ledgers(program: &Program, _symbols: &Symbols, diagnostics: &mut Vec<Diagnostic>) {
    use std::collections::HashSet;

    // Crypto primitives declared with kind `hash` and not denied — the only
    // algorithms a ledger may name. v0.30 ledgers are hash-chain metadata only.
    let hash_cryptos: HashSet<&str> = program
        .cryptos
        .iter()
        .filter(|c| {
            c.kind.value.source_name() == "hash" && c.status.value.source_name() != "denied"
        })
        .map(|c| c.name.value.as_str())
        .collect();
    let identities: HashSet<&str> = program
        .atrust_identities
        .iter()
        .map(|i| i.name.value.as_str())
        .collect();
    let credentials: HashSet<&str> = program
        .atrust_credential_contracts
        .iter()
        .map(|c| c.name.value.as_str())
        .collect();
    let handshakes: HashSet<&str> = program
        .atrust_handshakes
        .iter()
        .map(|h| h.name.value.as_str())
        .collect();
    let policies: HashSet<&str> = program
        .policies
        .iter()
        .map(|p| p.name.value.as_str())
        .collect();

    let mut names = HashSet::new();
    for l in &program.trust_ledgers {
        report_duplicate(&mut names, &l.name, "trust_ledger", diagnostics);
        let name = l.name.value.clone();

        if !matches!(l.scope.value.source_name(), "local" | "package" | "bundle") {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` requires scope `local`, `package`, or `bundle`"),
                l.scope.span,
            ));
        }
        if !matches!(l.mode.value.source_name(), "dry_run" | "declared_only") {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` requires mode `dry_run` or `declared_only`"),
                l.mode.span,
            ));
        }
        if l.hash_algorithm.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` is missing required field `hash_algorithm`"),
                l.hash_algorithm.span,
            ));
        } else if !hash_cryptos.contains(l.hash_algorithm.value.as_str()) {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` hash_algorithm `{}` must reference a declared, non-denied crypto of kind `hash`", l.hash_algorithm.value),
                l.hash_algorithm.span,
            ));
        }
        if !matches!(
            l.chain_policy.value.source_name(),
            "append_only" | "declared_only"
        ) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "trust_ledger `{name}` requires chain_policy `append_only` or `declared_only`"
                ),
                l.chain_policy.span,
            ));
        }
        if l.network.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` requires network `denied`"),
                l.network.span,
            ));
        }
        if l.key_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` requires key_material `denied`"),
                l.key_material.span,
            ));
        }
        if l.secret_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` requires secret_material `denied`"),
                l.secret_material.span,
            ));
        }
        if l.execution.value.source_name() != "disabled" {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` requires execution `disabled`"),
                l.execution.span,
            ));
        }
        if l.evidence.value.source_name() != "required" {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` requires evidence `required`"),
                l.evidence.span,
            ));
        }
        if l.security_claims.value.source_name() != "none" {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` requires security_claims `none`"),
                l.security_claims.span,
            ));
        }
        if l.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` is missing required field `purpose`"),
                l.name.span,
            ));
        }
        for p in &l.purpose {
            if p.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("trust_ledger `{name}` purpose must not contain empty strings"),
                    p.span,
                ));
            }
        }
        if let Some(notes) = &l.notes {
            if notes.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("trust_ledger `{name}` notes must not be empty"),
                    notes.span,
                ));
            }
        }

        if l.entries.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` is missing required field `entries`"),
                l.name.span,
            ));
        }

        let prefix = format!("{}:", l.hash_algorithm.value);
        let mut entry_ids = HashSet::new();
        let mut previous_entry_hash: Option<&str> = None;
        for (index, e) in l.entries.iter().enumerate() {
            if e.id.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("trust_ledger `{name}` entry is missing required field `id`"),
                    e.id.span,
                ));
            } else if !entry_ids.insert(e.id.value.as_str()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "trust_ledger `{name}` has duplicate entry id `{}`",
                        e.id.value
                    ),
                    e.id.span,
                ));
            }
            let kind = e.kind.value.source_name();
            if !matches!(
                kind,
                "identity" | "credential" | "handshake" | "evidence" | "policy" | "custom"
            ) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "trust_ledger `{name}` entry `{}` has invalid kind",
                        e.id.value
                    ),
                    e.kind.span,
                ));
            }
            if e.subject.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "trust_ledger `{name}` entry `{}` is missing required field `subject`",
                        e.id.value
                    ),
                    e.subject.span,
                ));
            } else {
                let known = match kind {
                    "identity" => identities.contains(e.subject.value.as_str()),
                    "credential" => credentials.contains(e.subject.value.as_str()),
                    "handshake" => handshakes.contains(e.subject.value.as_str()),
                    "policy" => policies.contains(e.subject.value.as_str()),
                    // evidence/custom (and unknown kinds) only require a non-empty subject.
                    _ => true,
                };
                if !known {
                    diagnostics.push(Diagnostic::new(
                        format!("trust_ledger `{name}` entry `{}` references unknown {kind} subject `{}`", e.id.value, e.subject.value),
                        e.subject.span,
                    ));
                }
            }
            if e.evidence_ref.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "trust_ledger `{name}` entry `{}` is missing required field `evidence_ref`",
                        e.id.value
                    ),
                    e.evidence_ref.span,
                ));
            }
            if e.entry_hash.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "trust_ledger `{name}` entry `{}` is missing required field `entry_hash`",
                        e.id.value
                    ),
                    e.entry_hash.span,
                ));
            } else if !l.hash_algorithm.value.is_empty() && !e.entry_hash.value.starts_with(&prefix)
            {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "trust_ledger `{name}` entry `{}` entry_hash must use the `{}` prefix",
                        e.id.value, prefix
                    ),
                    e.entry_hash.span,
                ));
            }
            if e.previous_hash.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("trust_ledger `{name}` entry `{}` is missing required field `previous_hash`", e.id.value),
                    e.previous_hash.span,
                ));
            } else if index == 0 {
                if e.previous_hash.value != "GENESIS" {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "trust_ledger `{name}` first entry previous_hash must be `GENESIS`"
                        ),
                        e.previous_hash.span,
                    ));
                }
            } else if let Some(prev) = previous_entry_hash {
                if e.previous_hash.value != prev {
                    diagnostics.push(Diagnostic::new(
                        format!("trust_ledger `{name}` entry `{}` previous_hash must match the prior entry_hash", e.id.value),
                        e.previous_hash.span,
                    ));
                }
            }
            previous_entry_hash = Some(e.entry_hash.value.as_str());
        }

        if l.chain_root.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("trust_ledger `{name}` is missing required field `chain_root`"),
                l.chain_root.span,
            ));
        } else if let Some(last) = l.entries.last() {
            if l.chain_root.value != last.entry_hash.value {
                diagnostics.push(Diagnostic::new(
                    format!("trust_ledger `{name}` chain_root must match the final entry_hash"),
                    l.chain_root.span,
                ));
            }
        }
    }
}

fn check_mcp_bridge_contracts(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use std::collections::HashMap;

    // Bridge contracts are declarative interoperability surfaces (v0.31). They
    // describe how an agent *could* talk to MCP tools/resources/prompts; they
    // never open network access, start a server, or execute tools.
    let passports: HashMap<&str, &argorix_parser::ast::PassportDecl> = program
        .passports
        .iter()
        .map(|p| (p.name.value.as_str(), p))
        .collect();
    let identities: HashMap<&str, &argorix_parser::ast::ATrustIdentityDecl> = program
        .atrust_identities
        .iter()
        .map(|i| (i.name.value.as_str(), i))
        .collect();

    let mut names = HashSet::new();
    for c in &program.mcp_bridge_contracts {
        report_duplicate(&mut names, &c.name, "mcp_bridge_contract", diagnostics);
        let name = c.name.value.clone();

        for (field, value) in [
            ("agent", &c.agent),
            ("passport", &c.passport),
            ("identity", &c.identity),
            ("boundary", &c.boundary),
        ] {
            if value.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("mcp_bridge_contract `{name}` is missing required field `{field}`"),
                    value.span,
                ));
            }
        }
        if c.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` is missing required field `purpose`"),
                c.name.span,
            ));
        }
        for p in &c.purpose {
            if p.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("mcp_bridge_contract `{name}` purpose must not contain empty strings"),
                    p.span,
                ));
            }
        }
        for (field, items) in [
            ("tools", &c.tools),
            ("resources", &c.resources),
            ("prompts", &c.prompts),
        ] {
            for item in items {
                if item.value.trim().is_empty() {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "mcp_bridge_contract `{name}` {field} must not contain empty strings"
                        ),
                        item.span,
                    ));
                }
            }
        }
        if let Some(notes) = &c.notes {
            if notes.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("mcp_bridge_contract `{name}` notes must not be empty"),
                    notes.span,
                ));
            }
        }

        // --- enumerated security boundaries ---
        if !matches!(
            c.transport.value.source_name(),
            "declared_only" | "disabled"
        ) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "mcp_bridge_contract `{name}` requires transport `declared_only` or `disabled`"
                ),
                c.transport.span,
            ));
        }
        if c.protocol.value.source_name() != "mcp" {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires protocol `mcp`"),
                c.protocol.span,
            ));
        }
        if !matches!(
            c.direction.value.source_name(),
            "inbound" | "outbound" | "bidirectional"
        ) {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires direction `inbound`, `outbound`, or `bidirectional`"),
                c.direction.span,
            ));
        }
        if c.network.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires network `denied`"),
                c.network.span,
            ));
        }
        if c.external_execution.value.source_name() != "disabled" {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires external_execution `disabled`"),
                c.external_execution.span,
            ));
        }
        if c.tool_execution.value.source_name() != "disabled" {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires tool_execution `disabled`"),
                c.tool_execution.span,
            ));
        }
        if c.secret_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires secret_material `denied`"),
                c.secret_material.span,
            ));
        }
        if c.key_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires key_material `denied`"),
                c.key_material.span,
            ));
        }
        if !matches!(
            c.authentication.value.source_name(),
            "none" | "declared_only"
        ) {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires authentication `none` or `declared_only`"),
                c.authentication.span,
            ));
        }
        if !matches!(
            c.authorization.value.source_name(),
            "policy_bound" | "declared_only"
        ) {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires authorization `policy_bound` or `declared_only`"),
                c.authorization.span,
            ));
        }
        if c.evidence.value.source_name() != "required" {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires evidence `required`"),
                c.evidence.span,
            ));
        }
        if c.security_claims.value.source_name() != "none" {
            diagnostics.push(Diagnostic::new(
                format!("mcp_bridge_contract `{name}` requires security_claims `none`"),
                c.security_claims.span,
            ));
        }

        // --- cross-reference validation ---
        if !c.agent.value.is_empty() && !symbols.agents.contains(&c.agent.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "mcp_bridge_contract `{name}` references unknown agent `{}`",
                    c.agent.value
                ),
                c.agent.span,
            ));
        }
        if !c.boundary.value.is_empty() && !symbols.atrust_boundaries.contains(&c.boundary.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "mcp_bridge_contract `{name}` references unknown atrust_boundary `{}`",
                    c.boundary.value
                ),
                c.boundary.span,
            ));
        }
        if !c.passport.value.is_empty() {
            match passports.get(c.passport.value.as_str()) {
                None => diagnostics.push(Diagnostic::new(
                    format!(
                        "mcp_bridge_contract `{name}` references unknown passport `{}`",
                        c.passport.value
                    ),
                    c.passport.span,
                )),
                Some(passport) => {
                    if !c.agent.value.is_empty() && passport.agent.value != c.agent.value {
                        diagnostics.push(Diagnostic::new(
                            format!("mcp_bridge_contract `{name}` passport `{}` is not bound to agent `{}`", c.passport.value, c.agent.value),
                            c.passport.span,
                        ));
                    }
                }
            }
        }
        if !c.identity.value.is_empty() {
            match identities.get(c.identity.value.as_str()) {
                None => diagnostics.push(Diagnostic::new(
                    format!(
                        "mcp_bridge_contract `{name}` references unknown identity `{}`",
                        c.identity.value
                    ),
                    c.identity.span,
                )),
                Some(identity) => {
                    if !c.agent.value.is_empty() && identity.subject.value != c.agent.value {
                        diagnostics.push(Diagnostic::new(
                            format!("mcp_bridge_contract `{name}` identity `{}` is not bound to agent `{}`", c.identity.value, c.agent.value),
                            c.identity.span,
                        ));
                    }
                    if !c.boundary.value.is_empty() && identity.boundary.value != c.boundary.value {
                        diagnostics.push(Diagnostic::new(
                            format!("mcp_bridge_contract `{name}` identity boundary `{}` does not match boundary `{}`", identity.boundary.value, c.boundary.value),
                            c.identity.span,
                        ));
                    }
                }
            }
        }
    }
}

fn check_a2a_bridge_contracts(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use std::collections::HashMap;

    let passports: HashMap<&str, &argorix_parser::ast::PassportDecl> = program
        .passports
        .iter()
        .map(|p| (p.name.value.as_str(), p))
        .collect();
    let identities: HashMap<&str, &argorix_parser::ast::ATrustIdentityDecl> = program
        .atrust_identities
        .iter()
        .map(|i| (i.name.value.as_str(), i))
        .collect();
    let handshakes: HashMap<&str, &argorix_parser::ast::ATrustHandshakeDecl> = program
        .atrust_handshakes
        .iter()
        .map(|h| (h.name.value.as_str(), h))
        .collect();
    let ledgers: HashMap<&str, &argorix_parser::ast::TrustLedgerDecl> = program
        .trust_ledgers
        .iter()
        .map(|l| (l.name.value.as_str(), l))
        .collect();

    let mut names = HashSet::new();
    for c in &program.a2a_bridge_contracts {
        report_duplicate(&mut names, &c.name, "a2a_bridge_contract", diagnostics);
        let name = c.name.value.clone();

        for (field, value) in [
            ("initiator", &c.initiator),
            ("responder", &c.responder),
            ("initiator_passport", &c.initiator_passport),
            ("responder_passport", &c.responder_passport),
            ("initiator_identity", &c.initiator_identity),
            ("responder_identity", &c.responder_identity),
            ("handshake", &c.handshake),
            ("trust_ledger", &c.trust_ledger),
            ("boundary", &c.boundary),
        ] {
            if value.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("a2a_bridge_contract `{name}` is missing required field `{field}`"),
                    value.span,
                ));
            }
        }
        if c.message_contracts.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "a2a_bridge_contract `{name}` is missing required field `message_contracts`"
                ),
                c.name.span,
            ));
        }
        if c.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` is missing required field `purpose`"),
                c.name.span,
            ));
        }
        for p in &c.purpose {
            if p.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("a2a_bridge_contract `{name}` purpose must not contain empty strings"),
                    p.span,
                ));
            }
        }
        for item in &c.capabilities {
            if item.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "a2a_bridge_contract `{name}` capabilities must not contain empty strings"
                    ),
                    item.span,
                ));
            }
        }
        if let Some(notes) = &c.notes {
            if notes.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("a2a_bridge_contract `{name}` notes must not be empty"),
                    notes.span,
                ));
            }
        }

        // --- enumerated security boundaries ---
        if c.protocol.value.source_name() != "a2a" {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires protocol `a2a`"),
                c.protocol.span,
            ));
        }
        if !matches!(
            c.transport.value.source_name(),
            "declared_only" | "disabled"
        ) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "a2a_bridge_contract `{name}` requires transport `declared_only` or `disabled`"
                ),
                c.transport.span,
            ));
        }
        if !matches!(
            c.direction.value.source_name(),
            "inbound" | "outbound" | "bidirectional"
        ) {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires direction `inbound`, `outbound`, or `bidirectional`"),
                c.direction.span,
            ));
        }
        if c.network.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires network `denied`"),
                c.network.span,
            ));
        }
        if c.external_execution.value.source_name() != "disabled" {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires external_execution `disabled`"),
                c.external_execution.span,
            ));
        }
        if c.agent_execution.value.source_name() != "disabled" {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires agent_execution `disabled`"),
                c.agent_execution.span,
            ));
        }
        if c.secret_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires secret_material `denied`"),
                c.secret_material.span,
            ));
        }
        if c.key_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires key_material `denied`"),
                c.key_material.span,
            ));
        }
        if !matches!(
            c.authentication.value.source_name(),
            "none" | "declared_only"
        ) {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires authentication `none` or `declared_only`"),
                c.authentication.span,
            ));
        }
        if !matches!(
            c.authorization.value.source_name(),
            "policy_bound" | "declared_only"
        ) {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires authorization `policy_bound` or `declared_only`"),
                c.authorization.span,
            ));
        }
        if c.evidence.value.source_name() != "required" {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires evidence `required`"),
                c.evidence.span,
            ));
        }
        if c.security_claims.value.source_name() != "none" {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` requires security_claims `none`"),
                c.security_claims.span,
            ));
        }

        // --- cross-reference validation ---
        if !c.initiator.value.is_empty() && !symbols.agents.contains(&c.initiator.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "a2a_bridge_contract `{name}` references unknown initiator agent `{}`",
                    c.initiator.value
                ),
                c.initiator.span,
            ));
        }
        if !c.responder.value.is_empty() && !symbols.agents.contains(&c.responder.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "a2a_bridge_contract `{name}` references unknown responder agent `{}`",
                    c.responder.value
                ),
                c.responder.span,
            ));
        }
        if !c.initiator.value.is_empty() && c.initiator.value == c.responder.value {
            diagnostics.push(Diagnostic::new(
                format!("a2a_bridge_contract `{name}` initiator and responder must be distinct"),
                c.responder.span,
            ));
        }
        if !c.boundary.value.is_empty() && !symbols.atrust_boundaries.contains(&c.boundary.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "a2a_bridge_contract `{name}` references unknown atrust_boundary `{}`",
                    c.boundary.value
                ),
                c.boundary.span,
            ));
        }
        check_a2a_passport_binding(
            &name,
            &c.initiator_passport,
            &c.initiator,
            &passports,
            diagnostics,
        );
        check_a2a_passport_binding(
            &name,
            &c.responder_passport,
            &c.responder,
            &passports,
            diagnostics,
        );
        check_a2a_identity_binding(
            &name,
            &c.initiator_identity,
            &c.initiator,
            &c.boundary,
            &identities,
            diagnostics,
        );
        check_a2a_identity_binding(
            &name,
            &c.responder_identity,
            &c.responder,
            &c.boundary,
            &identities,
            diagnostics,
        );

        // handshake must exist and bind initiator/responder
        if !c.handshake.value.is_empty() {
            match handshakes.get(c.handshake.value.as_str()) {
                None => diagnostics.push(Diagnostic::new(
                    format!(
                        "a2a_bridge_contract `{name}` references unknown handshake `{}`",
                        c.handshake.value
                    ),
                    c.handshake.span,
                )),
                Some(handshake) => {
                    let matches_pair = (handshake.initiator.value == c.initiator.value
                        && handshake.responder.value == c.responder.value)
                        || (handshake.initiator.value == c.responder.value
                            && handshake.responder.value == c.initiator.value);
                    if !c.initiator.value.is_empty()
                        && !c.responder.value.is_empty()
                        && !matches_pair
                    {
                        diagnostics.push(Diagnostic::new(
                            format!("a2a_bridge_contract `{name}` handshake `{}` does not bind initiator `{}` and responder `{}`", c.handshake.value, c.initiator.value, c.responder.value),
                            c.handshake.span,
                        ));
                    }
                }
            }
        }

        // trust_ledger must exist and include a handshake entry for this handshake
        if !c.trust_ledger.value.is_empty() {
            match ledgers.get(c.trust_ledger.value.as_str()) {
                None => diagnostics.push(Diagnostic::new(
                    format!(
                        "a2a_bridge_contract `{name}` references unknown trust_ledger `{}`",
                        c.trust_ledger.value
                    ),
                    c.trust_ledger.span,
                )),
                Some(ledger) => {
                    if !c.handshake.value.is_empty() {
                        let has_entry = ledger.entries.iter().any(|e| {
                            e.kind.value.source_name() == "handshake"
                                && e.subject.value == c.handshake.value
                        });
                        if !has_entry {
                            diagnostics.push(Diagnostic::new(
                                format!("a2a_bridge_contract `{name}` trust_ledger `{}` does not contain a handshake entry for `{}`", c.trust_ledger.value, c.handshake.value),
                                c.trust_ledger.span,
                            ));
                        }
                    }
                }
            }
        }

        // message_contracts must reference declared message types
        for m in &c.message_contracts {
            if m.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("a2a_bridge_contract `{name}` message_contracts must not contain empty strings"),
                    m.span,
                ));
            } else if !symbols.types.contains(&m.value) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "a2a_bridge_contract `{name}` references unknown message_contract `{}`",
                        m.value
                    ),
                    m.span,
                ));
            }
        }
    }
}

fn check_a2a_passport_binding(
    name: &str,
    passport_ref: &Spanned<String>,
    agent_ref: &Spanned<String>,
    passports: &std::collections::HashMap<&str, &argorix_parser::ast::PassportDecl>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if passport_ref.value.is_empty() {
        return;
    }
    match passports.get(passport_ref.value.as_str()) {
        None => diagnostics.push(Diagnostic::new(
            format!(
                "a2a_bridge_contract `{name}` references unknown passport `{}`",
                passport_ref.value
            ),
            passport_ref.span,
        )),
        Some(passport) => {
            if !agent_ref.value.is_empty() && passport.agent.value != agent_ref.value {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "a2a_bridge_contract `{name}` passport `{}` is not bound to agent `{}`",
                        passport_ref.value, agent_ref.value
                    ),
                    passport_ref.span,
                ));
            }
        }
    }
}

fn check_a2a_identity_binding(
    name: &str,
    identity_ref: &Spanned<String>,
    agent_ref: &Spanned<String>,
    boundary_ref: &Spanned<String>,
    identities: &std::collections::HashMap<&str, &argorix_parser::ast::ATrustIdentityDecl>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if identity_ref.value.is_empty() {
        return;
    }
    match identities.get(identity_ref.value.as_str()) {
        None => diagnostics.push(Diagnostic::new(
            format!(
                "a2a_bridge_contract `{name}` references unknown identity `{}`",
                identity_ref.value
            ),
            identity_ref.span,
        )),
        Some(identity) => {
            if !agent_ref.value.is_empty() && identity.subject.value != agent_ref.value {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "a2a_bridge_contract `{name}` identity `{}` is not bound to agent `{}`",
                        identity_ref.value, agent_ref.value
                    ),
                    identity_ref.span,
                ));
            }
            if !boundary_ref.value.is_empty() && identity.boundary.value != boundary_ref.value {
                diagnostics.push(Diagnostic::new(
                    format!("a2a_bridge_contract `{name}` identity boundary `{}` does not match boundary `{}`", identity.boundary.value, boundary_ref.value),
                    identity_ref.span,
                ));
            }
        }
    }
}

fn check_atrust_evidence_maps(
    program: &Program,
    symbols: &Symbols,
    diagnostics: &mut Vec<Diagnostic>,
) {
    use std::collections::HashMap;

    let passports: HashMap<&str, &argorix_parser::ast::PassportDecl> = program
        .passports
        .iter()
        .map(|p| (p.name.value.as_str(), p))
        .collect();
    let identities: HashMap<&str, &argorix_parser::ast::ATrustIdentityDecl> = program
        .atrust_identities
        .iter()
        .map(|i| (i.name.value.as_str(), i))
        .collect();
    let credentials: HashMap<&str, &ATrustCredentialContractDecl> = program
        .atrust_credential_contracts
        .iter()
        .map(|c| (c.name.value.as_str(), c))
        .collect();
    let handshakes: HashMap<&str, &argorix_parser::ast::ATrustHandshakeDecl> = program
        .atrust_handshakes
        .iter()
        .map(|h| (h.name.value.as_str(), h))
        .collect();
    let ledgers: HashMap<&str, &argorix_parser::ast::TrustLedgerDecl> = program
        .trust_ledgers
        .iter()
        .map(|l| (l.name.value.as_str(), l))
        .collect();
    let mcp_bridges: HashMap<&str, &argorix_parser::ast::McpBridgeContractDecl> = program
        .mcp_bridge_contracts
        .iter()
        .map(|b| (b.name.value.as_str(), b))
        .collect();
    let a2a_bridges: HashMap<&str, &argorix_parser::ast::A2ABridgeContractDecl> = program
        .a2a_bridge_contracts
        .iter()
        .map(|b| (b.name.value.as_str(), b))
        .collect();
    let policies = program
        .policies
        .iter()
        .map(|p| p.name.value.as_str())
        .collect::<HashSet<_>>();

    let mut names = HashSet::new();
    for map in &program.atrust_evidence_maps {
        report_duplicate(&mut names, &map.name, "atrust_evidence_map", diagnostics);
        let name = map.name.value.as_str();

        for (field, value) in [
            ("agent", &map.agent),
            ("passport", &map.passport),
            ("identity", &map.identity),
            ("credential_contract", &map.credential_contract),
            ("handshake", &map.handshake),
            ("trust_ledger", &map.trust_ledger),
        ] {
            if value.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("atrust_evidence_map `{name}` is missing required field `{field}`"),
                    value.span,
                ));
            }
        }
        for (field, values) in [
            ("mcp_bridges", &map.mcp_bridges),
            ("a2a_bridges", &map.a2a_bridges),
            ("policies", &map.policies),
            ("purpose", &map.purpose),
        ] {
            if values.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("atrust_evidence_map `{name}` is missing required field `{field}`"),
                    map.name.span,
                ));
            }
            for value in values {
                if value.value.trim().is_empty() {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "atrust_evidence_map `{name}` {field} must not contain empty strings"
                        ),
                        value.span,
                    ));
                }
            }
        }
        if let Some(notes) = &map.notes {
            if notes.value.trim().is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("atrust_evidence_map `{name}` notes must not be empty"),
                    notes.span,
                ));
            }
        }

        if !map.agent.value.is_empty() && !symbols.agents.contains(&map.agent.value) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_evidence_map `{name}` references unknown agent `{}`",
                    map.agent.value
                ),
                map.agent.span,
            ));
        }
        match passports.get(map.passport.value.as_str()) {
            None if !map.passport.value.is_empty() => diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_evidence_map `{name}` references unknown passport `{}`",
                    map.passport.value
                ),
                map.passport.span,
            )),
            Some(passport)
                if !map.agent.value.is_empty() && passport.agent.value != map.agent.value =>
            {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_evidence_map `{name}` passport `{}` is not bound to agent `{}`",
                        map.passport.value, map.agent.value
                    ),
                    map.passport.span,
                ));
            }
            _ => {}
        }
        match identities.get(map.identity.value.as_str()) {
            None if !map.identity.value.is_empty() => diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_evidence_map `{name}` references unknown identity `{}`",
                    map.identity.value
                ),
                map.identity.span,
            )),
            Some(identity)
                if !map.agent.value.is_empty() && identity.subject.value != map.agent.value =>
            {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_evidence_map `{name}` identity `{}` is not bound to agent `{}`",
                        map.identity.value, map.agent.value
                    ),
                    map.identity.span,
                ));
            }
            _ => {}
        }
        match credentials.get(map.credential_contract.value.as_str()) {
            None if !map.credential_contract.value.is_empty() => {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_evidence_map `{name}` references unknown credential_contract `{}`",
                        map.credential_contract.value
                    ),
                    map.credential_contract.span,
                ));
            }
            Some(credential) => {
                if !map.identity.value.is_empty() && credential.identity.value != map.identity.value
                {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "atrust_evidence_map `{name}` credential_contract `{}` is not bound to identity `{}`",
                            map.credential_contract.value, map.identity.value
                        ),
                        map.credential_contract.span,
                    ));
                }
                if !map.agent.value.is_empty() && credential.subject.value != map.agent.value {
                    diagnostics.push(Diagnostic::new(
                        format!(
                            "atrust_evidence_map `{name}` credential_contract `{}` is not bound to agent `{}`",
                            map.credential_contract.value, map.agent.value
                        ),
                        map.credential_contract.span,
                    ));
                }
            }
            _ => {}
        }
        match handshakes.get(map.handshake.value.as_str()) {
            None if !map.handshake.value.is_empty() => diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_evidence_map `{name}` references unknown handshake `{}`",
                    map.handshake.value
                ),
                map.handshake.span,
            )),
            Some(handshake)
                if !map.agent.value.is_empty()
                    && handshake.initiator.value != map.agent.value
                    && handshake.responder.value != map.agent.value =>
            {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_evidence_map `{name}` handshake `{}` does not include agent `{}`",
                        map.handshake.value, map.agent.value
                    ),
                    map.handshake.span,
                ));
            }
            _ => {}
        }

        match ledgers.get(map.trust_ledger.value.as_str()) {
            None if !map.trust_ledger.value.is_empty() => diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_evidence_map `{name}` references unknown trust_ledger `{}`",
                    map.trust_ledger.value
                ),
                map.trust_ledger.span,
            )),
            Some(ledger) => {
                require_ledger_entry(
                    name,
                    ledger,
                    "identity",
                    &map.identity.value,
                    &map.trust_ledger,
                    diagnostics,
                );
                require_ledger_entry(
                    name,
                    ledger,
                    "credential",
                    &map.credential_contract.value,
                    &map.trust_ledger,
                    diagnostics,
                );
                require_ledger_entry(
                    name,
                    ledger,
                    "handshake",
                    &map.handshake.value,
                    &map.trust_ledger,
                    diagnostics,
                );
            }
            _ => {}
        }

        for bridge_ref in &map.mcp_bridges {
            match mcp_bridges.get(bridge_ref.value.as_str()) {
                None => diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_evidence_map `{name}` references unknown mcp_bridge `{}`",
                        bridge_ref.value
                    ),
                    bridge_ref.span,
                )),
                Some(bridge) => {
                    if bridge.agent.value != map.agent.value
                        || bridge.passport.value != map.passport.value
                        || bridge.identity.value != map.identity.value
                    {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "atrust_evidence_map `{name}` mcp_bridge `{}` has incompatible agent/passport/identity binding",
                                bridge_ref.value
                            ),
                            bridge_ref.span,
                        ));
                    }
                }
            }
        }
        for bridge_ref in &map.a2a_bridges {
            match a2a_bridges.get(bridge_ref.value.as_str()) {
                None => diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_evidence_map `{name}` references unknown a2a_bridge `{}`",
                        bridge_ref.value
                    ),
                    bridge_ref.span,
                )),
                Some(bridge) => {
                    if bridge.handshake.value != map.handshake.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "atrust_evidence_map `{name}` a2a_bridge `{}` handshake mismatch",
                                bridge_ref.value
                            ),
                            bridge_ref.span,
                        ));
                    }
                    if bridge.trust_ledger.value != map.trust_ledger.value {
                        diagnostics.push(Diagnostic::new(
                            format!(
                                "atrust_evidence_map `{name}` a2a_bridge `{}` ledger mismatch",
                                bridge_ref.value
                            ),
                            bridge_ref.span,
                        ));
                    }
                }
            }
        }
        for policy in &map.policies {
            if !policies.contains(policy.value.as_str()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "atrust_evidence_map `{name}` references unknown policy `{}`",
                        policy.value
                    ),
                    policy.span,
                ));
            }
        }

        if !matches!(
            map.coverage.value,
            ATrustEvidenceMapCoverage::Required | ATrustEvidenceMapCoverage::Complete
        ) {
            diagnostics.push(Diagnostic::new(
                format!("atrust_evidence_map `{name}` requires coverage `required` or `complete`"),
                map.coverage.span,
            ));
        }
        if !matches!(
            map.mapping_mode.value,
            ATrustEvidenceMapMappingMode::DeclaredOnly | ATrustEvidenceMapMappingMode::EvidenceOnly
        ) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "atrust_evidence_map `{name}` requires mapping_mode `declared_only` or `evidence_only`"
                ),
                map.mapping_mode.span,
            ));
        }
        if !matches!(
            map.verification.value.source_name(),
            "declared_only" | "disabled"
        ) {
            diagnostics.push(Diagnostic::new(
                format!("atrust_evidence_map `{name}` requires verification `declared_only` or `disabled`"),
                map.verification.span,
            ));
        }
        if map.resolution.value.source_name() != "disabled" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_evidence_map `{name}` requires resolution `disabled`"),
                map.resolution.span,
            ));
        }
        for (field, value) in [
            ("evidence_bundle", &map.evidence_bundle),
            ("security_report", &map.security_report),
            ("trace", &map.trace),
        ] {
            if value.value.source_name() != "required" {
                diagnostics.push(Diagnostic::new(
                    format!("atrust_evidence_map `{name}` requires {field} `required`"),
                    value.span,
                ));
            }
        }
        if map.network.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_evidence_map `{name}` requires network `denied`"),
                map.network.span,
            ));
        }
        if map.external_execution.value.source_name() != "disabled" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_evidence_map `{name}` requires external_execution `disabled`"),
                map.external_execution.span,
            ));
        }
        if map.secret_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_evidence_map `{name}` requires secret_material `denied`"),
                map.secret_material.span,
            ));
        }
        if map.key_material.value.source_name() != "denied" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_evidence_map `{name}` requires key_material `denied`"),
                map.key_material.span,
            ));
        }
        if map.execution.value.source_name() != "disabled" {
            diagnostics.push(Diagnostic::new(
                format!("atrust_evidence_map `{name}` requires execution `disabled`"),
                map.execution.span,
            ));
        }
        if !matches!(map.security_claims.value, ATrustSecurityClaims::None) {
            diagnostics.push(Diagnostic::new(
                format!("atrust_evidence_map `{name}` requires security_claims `none`"),
                map.security_claims.span,
            ));
        }
    }
}

pub fn check_governance_profiles(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let evidence_maps: HashSet<&str> = program
        .atrust_evidence_maps
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let trust_ledgers: HashSet<&str> = program
        .trust_ledgers
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let policies: HashSet<&str> = program
        .policies
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let mut names = HashSet::new();

    for profile in &program.governance_profiles {
        let name = profile.name.value.as_str();
        report_duplicate(&mut names, &profile.name, "governance_profile", diagnostics);
        if matches!(profile.scope.value, GovernanceScope::Unknown(_)) {
            invalid_governance_value(
                name,
                "scope",
                profile.scope.value.source_name(),
                profile.scope.span,
                diagnostics,
            );
        }
        if matches!(profile.level.value, GovernanceLevel::Unknown(_)) {
            invalid_governance_value(
                name,
                "level",
                profile.level.value.source_name(),
                profile.level.span,
                diagnostics,
            );
        }
        if matches!(profile.domain.value, GovernanceDomain::Unknown(_)) {
            invalid_governance_value(
                name,
                "domain",
                profile.domain.value.source_name(),
                profile.domain.span,
                diagnostics,
            );
        }
        require_governance_text(name, "owner", &profile.owner, diagnostics);
        require_governance_text(name, "jurisdiction", &profile.jurisdiction, diagnostics);
        require_governance_text(name, "framework", &profile.framework, diagnostics);

        if !evidence_maps.contains(profile.evidence_map.value.as_str()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "governance_profile `{name}` references unknown evidence_map `{}`",
                    profile.evidence_map.value
                ),
                profile.evidence_map.span,
            ));
        }
        if !trust_ledgers.contains(profile.trust_ledger.value.as_str()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "governance_profile `{name}` references unknown trust_ledger `{}`",
                    profile.trust_ledger.value
                ),
                profile.trust_ledger.span,
            ));
        }
        if profile.policies.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("governance_profile `{name}` requires non-empty policies"),
                profile.name.span,
            ));
        }
        for policy in &profile.policies {
            if policy.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("governance_profile `{name}` policies must not contain empty strings"),
                    policy.span,
                ));
            } else if !policies.contains(policy.value.as_str()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "governance_profile `{name}` references unknown policy `{}`",
                        policy.value
                    ),
                    policy.span,
                ));
            }
        }
        if profile.controls.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("governance_profile `{name}` requires non-empty controls"),
                profile.name.span,
            ));
        }
        let mut control_ids = HashSet::new();
        for control in &profile.controls {
            if !control_ids.insert(control.id.value.as_str()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "governance_profile `{name}` has duplicate control id `{}`",
                        control.id.value
                    ),
                    control.id.span,
                ));
            }
            require_governance_text(name, "control id", &control.id, diagnostics);
            require_governance_text(
                name,
                "control requirement",
                &control.requirement,
                diagnostics,
            );
            require_governance_text(
                name,
                "control evidence_ref",
                &control.evidence_ref,
                diagnostics,
            );
            if matches!(
                control.category.value,
                GovernanceControlCategory::Unknown(_)
            ) {
                invalid_governance_value(
                    name,
                    "control category",
                    control.category.value.source_name(),
                    control.category.span,
                    diagnostics,
                );
            }
            if matches!(control.status.value, GovernanceControlStatus::Unknown(_)) {
                invalid_governance_value(
                    name,
                    "control status",
                    control.status.value.source_name(),
                    control.status.span,
                    diagnostics,
                );
            }
        }
        if matches!(profile.risk_level.value, GovernanceRiskLevel::Unknown(_)) {
            invalid_governance_value(
                name,
                "risk_level",
                profile.risk_level.value.source_name(),
                profile.risk_level.span,
                diagnostics,
            );
        }
        if matches!(
            profile.review_status.value,
            GovernanceReviewStatus::Unknown(_)
        ) {
            invalid_governance_value(
                name,
                "review_status",
                profile.review_status.value.source_name(),
                profile.review_status.span,
                diagnostics,
            );
        }
        if matches!(profile.assurance.value, GovernanceAssurance::Unknown(_)) {
            invalid_governance_value(
                name,
                "assurance",
                profile.assurance.value.source_name(),
                profile.assurance.span,
                diagnostics,
            );
        }
        require_governance_denied_boundaries(
            "governance_profile",
            name,
            profile.network.value.source_name(),
            profile.network.span,
            profile.external_execution.value.source_name(),
            profile.external_execution.span,
            profile.secret_material.value.source_name(),
            profile.secret_material.span,
            profile.key_material.value.source_name(),
            profile.key_material.span,
            profile.execution.value.source_name(),
            profile.execution.span,
            profile.security_claims.value.source_name(),
            profile.security_claims.span,
            diagnostics,
        );
        if profile.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("governance_profile `{name}` requires non-empty purpose"),
                profile.name.span,
            ));
        }
        for purpose in &profile.purpose {
            if purpose.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("governance_profile `{name}` purpose must not contain empty strings"),
                    purpose.span,
                ));
            }
        }
        if let Some(notes) = &profile.notes {
            require_governance_text(name, "notes", notes, diagnostics);
        }
    }
}

pub fn check_regulatory_mappings(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    use std::collections::HashMap;

    let profiles: HashMap<&str, &argorix_parser::ast::GovernanceProfileDecl> = program
        .governance_profiles
        .iter()
        .map(|value| (value.name.value.as_str(), value))
        .collect();
    let evidence_maps: HashSet<&str> = program
        .atrust_evidence_maps
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let mut names = HashSet::new();

    for mapping in &program.regulatory_mappings {
        let name = mapping.name.value.as_str();
        report_duplicate(&mut names, &mapping.name, "regulatory_mapping", diagnostics);
        let profile = profiles
            .get(mapping.governance_profile.value.as_str())
            .copied();
        if profile.is_none() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "regulatory_mapping `{name}` references unknown governance_profile `{}`",
                    mapping.governance_profile.value
                ),
                mapping.governance_profile.span,
            ));
        }
        if !evidence_maps.contains(mapping.evidence_map.value.as_str()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "regulatory_mapping `{name}` references unknown evidence_map `{}`",
                    mapping.evidence_map.value
                ),
                mapping.evidence_map.span,
            ));
        }
        if let Some(profile) = profile {
            if mapping.evidence_map.value != profile.evidence_map.value {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "regulatory_mapping `{name}` evidence_map `{}` does not match governance_profile `{}` evidence_map `{}`",
                        mapping.evidence_map.value,
                        profile.name.value,
                        profile.evidence_map.value
                    ),
                    mapping.evidence_map.span,
                ));
            }
        }
        require_regulatory_text(name, "jurisdiction", &mapping.jurisdiction, diagnostics);
        require_regulatory_text(name, "framework", &mapping.framework, diagnostics);
        if mapping.obligations.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("regulatory_mapping `{name}` requires non-empty obligations"),
                mapping.name.span,
            ));
        }
        let control_ids: HashSet<&str> = profile
            .map(|profile| {
                profile
                    .controls
                    .iter()
                    .map(|control| control.id.value.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let mut obligation_ids = HashSet::new();
        for obligation in &mapping.obligations {
            if !obligation_ids.insert(obligation.id.value.as_str()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "regulatory_mapping `{name}` has duplicate obligation id `{}`",
                        obligation.id.value
                    ),
                    obligation.id.span,
                ));
            }
            require_regulatory_text(name, "obligation id", &obligation.id, diagnostics);
            require_regulatory_text(name, "obligation source", &obligation.source, diagnostics);
            require_regulatory_text(
                name,
                "obligation requirement",
                &obligation.requirement,
                diagnostics,
            );
            require_regulatory_text(name, "obligation control", &obligation.control, diagnostics);
            require_regulatory_text(
                name,
                "obligation evidence_ref",
                &obligation.evidence_ref,
                diagnostics,
            );
            if !obligation.control.value.is_empty()
                && !control_ids.contains(obligation.control.value.as_str())
            {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "regulatory_mapping `{name}` obligation `{}` references unknown control `{}`",
                        obligation.id.value, obligation.control.value
                    ),
                    obligation.control.span,
                ));
            }
            if matches!(
                obligation.status.value,
                RegulatoryObligationStatus::Unknown(_)
            ) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "regulatory_mapping `{name}` has invalid obligation status `{}`",
                        obligation.status.value.source_name()
                    ),
                    obligation.status.span,
                ));
            }
        }
        if matches!(mapping.coverage.value, RegulatoryCoverage::Unknown(_)) {
            invalid_regulatory_value(
                name,
                "coverage",
                mapping.coverage.value.source_name(),
                mapping.coverage.span,
                diagnostics,
            );
        }
        if matches!(mapping.assessment.value, RegulatoryAssessment::Unknown(_)) {
            invalid_regulatory_value(
                name,
                "assessment",
                mapping.assessment.value.source_name(),
                mapping.assessment.span,
                diagnostics,
            );
        }
        if mapping.legal_claims.value != "none" {
            invalid_regulatory_value(
                name,
                "legal_claims",
                &mapping.legal_claims.value,
                mapping.legal_claims.span,
                diagnostics,
            );
        }
        if mapping.certification.value != "none" {
            invalid_regulatory_value(
                name,
                "certification",
                &mapping.certification.value,
                mapping.certification.span,
                diagnostics,
            );
        }
        require_governance_denied_boundaries(
            "regulatory_mapping",
            name,
            mapping.network.value.source_name(),
            mapping.network.span,
            mapping.external_execution.value.source_name(),
            mapping.external_execution.span,
            mapping.secret_material.value.source_name(),
            mapping.secret_material.span,
            mapping.key_material.value.source_name(),
            mapping.key_material.span,
            mapping.execution.value.source_name(),
            mapping.execution.span,
            mapping.security_claims.value.source_name(),
            mapping.security_claims.span,
            diagnostics,
        );
        if mapping.purpose.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("regulatory_mapping `{name}` requires non-empty purpose"),
                mapping.name.span,
            ));
        }
        for purpose in &mapping.purpose {
            if purpose.value.is_empty() {
                diagnostics.push(Diagnostic::new(
                    format!("regulatory_mapping `{name}` purpose must not contain empty strings"),
                    purpose.span,
                ));
            }
        }
        if let Some(notes) = &mapping.notes {
            require_regulatory_text(name, "notes", notes, diagnostics);
        }
    }
}

fn require_governance_text(
    name: &str,
    field: &str,
    value: &Spanned<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if value.value.is_empty() {
        diagnostics.push(Diagnostic::new(
            format!("governance_profile `{name}` {field} must not be empty"),
            value.span,
        ));
    }
}

pub fn check_third_party_verifiers(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let mut names = HashSet::new();
    for verifier in &program.third_party_verifiers {
        let name = verifier.name.value.as_str();
        report_duplicate(
            &mut names,
            &verifier.name,
            "third_party_verifier",
            diagnostics,
        );
        for (field, value, allowed, span) in [
            (
                "verifier_type",
                verifier.verifier_type.value.source_name(),
                &[
                    "internal",
                    "community",
                    "academic",
                    "vendor",
                    "independent_lab",
                    "custom",
                ][..],
                verifier.verifier_type.span,
            ),
            (
                "independence",
                verifier.independence.value.source_name(),
                &["declared", "self_attested", "independent_declared"][..],
                verifier.independence.span,
            ),
            (
                "identity_mode",
                verifier.identity_mode.value.source_name(),
                &["declared_only", "document_only"][..],
                verifier.identity_mode.span,
            ),
            (
                "verification_mode",
                verifier.verification_mode.value.source_name(),
                &[
                    "reproducible_artifacts",
                    "document_review",
                    "conformance_replay",
                ][..],
                verifier.verification_mode.span,
            ),
        ] {
            if !allowed.contains(&value) {
                diagnostics.push(Diagnostic::new(
                    format!("third_party_verifier `{name}` has invalid {field} `{value}`"),
                    span,
                ));
            }
        }
        for (field, value) in [
            ("name", &verifier.display_name),
            ("organization", &verifier.organization),
            ("jurisdiction", &verifier.jurisdiction),
        ] {
            require_public_conformance_text(
                "third_party_verifier",
                name,
                field,
                value,
                diagnostics,
            );
        }
        require_non_empty_string_list(
            "third_party_verifier",
            name,
            "allowed_scopes",
            &verifier.allowed_scopes,
            verifier.name.span,
            diagnostics,
        );
        require_non_empty_string_list(
            "third_party_verifier",
            name,
            "disallowed_claims",
            &verifier.disallowed_claims,
            verifier.name.span,
            diagnostics,
        );
        require_non_empty_string_list(
            "third_party_verifier",
            name,
            "purpose",
            &verifier.purpose,
            verifier.name.span,
            diagnostics,
        );
        require_public_denied_boundaries(
            "third_party_verifier",
            name,
            verifier.network.value.source_name(),
            verifier.network.span,
            verifier.external_execution.value.source_name(),
            verifier.external_execution.span,
            verifier.secret_material.value.source_name(),
            verifier.secret_material.span,
            verifier.key_material.value.source_name(),
            verifier.key_material.span,
            verifier.execution.value.source_name(),
            verifier.execution.span,
            &verifier.legal_claims,
            &verifier.certification,
            verifier.security_claims.value.source_name(),
            verifier.security_claims.span,
            diagnostics,
        );
        if let Some(notes) = &verifier.notes {
            require_public_conformance_text(
                "third_party_verifier",
                name,
                "notes",
                notes,
                diagnostics,
            );
        }
    }
}

pub fn check_public_conformance_reports(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let verifiers: HashSet<&str> = program
        .third_party_verifiers
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let evidence_maps: HashSet<&str> = program
        .atrust_evidence_maps
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let governance_profiles: std::collections::HashMap<&str, _> = program
        .governance_profiles
        .iter()
        .map(|value| (value.name.value.as_str(), value))
        .collect();
    let regulatory_mappings: std::collections::HashMap<&str, _> = program
        .regulatory_mappings
        .iter()
        .map(|value| (value.name.value.as_str(), value))
        .collect();
    let trust_ledgers: HashSet<&str> = program
        .trust_ledgers
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let mut names = HashSet::new();
    for report in &program.public_conformance_reports {
        let name = report.name.value.as_str();
        report_duplicate(
            &mut names,
            &report.name,
            "public_conformance_report",
            diagnostics,
        );
        for (kind, reference, known) in [
            (
                "third_party_verifier",
                &report.verifier,
                verifiers.contains(report.verifier.value.as_str()),
            ),
            (
                "evidence_map",
                &report.evidence_map,
                evidence_maps.contains(report.evidence_map.value.as_str()),
            ),
            (
                "governance_profile",
                &report.governance_profile,
                governance_profiles.contains_key(report.governance_profile.value.as_str()),
            ),
            (
                "regulatory_mapping",
                &report.regulatory_mapping,
                regulatory_mappings.contains_key(report.regulatory_mapping.value.as_str()),
            ),
            (
                "trust_ledger",
                &report.trust_ledger,
                trust_ledgers.contains(report.trust_ledger.value.as_str()),
            ),
        ] {
            if !known {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "public_conformance_report `{name}` references unknown {kind} `{}`",
                        reference.value
                    ),
                    reference.span,
                ));
            }
        }
        if let Some(profile) = governance_profiles.get(report.governance_profile.value.as_str()) {
            if profile.evidence_map.value != report.evidence_map.value {
                diagnostics.push(Diagnostic::new(
                    format!("public_conformance_report `{name}` governance/evidence_map mismatch"),
                    report.evidence_map.span,
                ));
            }
        }
        if let Some(mapping) = regulatory_mappings.get(report.regulatory_mapping.value.as_str()) {
            if mapping.governance_profile.value != report.governance_profile.value {
                diagnostics.push(Diagnostic::new(
                    format!("public_conformance_report `{name}` regulatory/governance mismatch"),
                    report.regulatory_mapping.span,
                ));
            }
            if mapping.evidence_map.value != report.evidence_map.value {
                diagnostics.push(Diagnostic::new(
                    format!("public_conformance_report `{name}` regulatory/evidence_map mismatch"),
                    report.regulatory_mapping.span,
                ));
            }
        }
        for (field, value) in [
            ("suite", &report.suite),
            ("suite_version", &report.suite_version),
            ("source_artifact", &report.source_artifact),
            ("bytecode_artifact", &report.bytecode_artifact),
        ] {
            require_public_conformance_text(
                "public_conformance_report",
                name,
                field,
                value,
                diagnostics,
            );
        }
        if report.suite_version.value != "0.34" {
            diagnostics.push(Diagnostic::new(
                format!("public_conformance_report `{name}` requires suite_version `0.34`"),
                report.suite_version.span,
            ));
        }
        for (field, value, allowed, span) in [
            (
                "result",
                report.result.value.source_name(),
                &["passed", "failed", "pending_review"][..],
                report.result.span,
            ),
            (
                "reproducibility",
                report.reproducibility.value.source_name(),
                &["declared", "replayable_locally", "document_only"][..],
                report.reproducibility.span,
            ),
            (
                "review_status",
                report.review_status.value.source_name(),
                &["draft", "reviewed", "published", "deprecated"][..],
                report.review_status.span,
            ),
        ] {
            if !allowed.contains(&value) {
                diagnostics.push(Diagnostic::new(
                    format!("public_conformance_report `{name}` has invalid {field} `{value}`"),
                    span,
                ));
            }
        }
        for (field, value) in [
            ("security_report", &report.security_report),
            ("evidence_bundle", &report.evidence_bundle),
            ("trace", &report.trace),
        ] {
            if value.value != "required" {
                diagnostics.push(Diagnostic::new(
                    format!("public_conformance_report `{name}` requires {field} `required`"),
                    value.span,
                ));
            }
        }
        if report.claims.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("public_conformance_report `{name}` requires non-empty claims"),
                report.name.span,
            ));
        }
        let mut claim_ids = HashSet::new();
        for claim in &report.claims {
            report_duplicate(
                &mut claim_ids,
                &claim.id,
                "public conformance claim id",
                diagnostics,
            );
            for (field, value) in [
                ("id", &claim.id),
                ("statement", &claim.statement),
                ("evidence_ref", &claim.evidence_ref),
            ] {
                require_public_conformance_text(
                    "public_conformance_report",
                    name,
                    field,
                    value,
                    diagnostics,
                );
            }
            if ![
                "conformance",
                "evidence",
                "security_report",
                "governance",
                "regulatory_mapping",
                "bytecode",
                "source",
                "policy",
                "runtime_boundary",
                "custom",
            ]
            .contains(&claim.category.value.source_name())
            {
                diagnostics.push(Diagnostic::new(
                    format!("public_conformance_report `{name}` has invalid claim category"),
                    claim.category.span,
                ));
            }
            if !["mapped", "declared", "pending_review", "not_applicable"]
                .contains(&claim.status.value.source_name())
            {
                diagnostics.push(Diagnostic::new(
                    format!("public_conformance_report `{name}` has invalid claim status"),
                    claim.status.span,
                ));
            }
        }
        require_non_empty_string_list(
            "public_conformance_report",
            name,
            "purpose",
            &report.purpose,
            report.name.span,
            diagnostics,
        );
        require_public_denied_boundaries(
            "public_conformance_report",
            name,
            report.network.value.source_name(),
            report.network.span,
            report.external_execution.value.source_name(),
            report.external_execution.span,
            report.secret_material.value.source_name(),
            report.secret_material.span,
            report.key_material.value.source_name(),
            report.key_material.span,
            report.execution.value.source_name(),
            report.execution.span,
            &report.legal_claims,
            &report.certification,
            report.security_claims.value.source_name(),
            report.security_claims.span,
            diagnostics,
        );
        if let Some(notes) = &report.notes {
            require_public_conformance_text(
                "public_conformance_report",
                name,
                "notes",
                notes,
                diagnostics,
            );
        }
    }
}

fn require_public_conformance_text(
    kind: &str,
    name: &str,
    field: &str,
    value: &Spanned<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if value.value.is_empty() {
        diagnostics.push(Diagnostic::new(
            format!("{kind} `{name}` {field} must not be empty"),
            value.span,
        ));
    }
}

pub fn check_runtime_hardening_profiles(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let evidence_maps: HashSet<&str> = program
        .atrust_evidence_maps
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let governance_profiles: HashSet<&str> = program
        .governance_profiles
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let public_reports: HashSet<&str> = program
        .public_conformance_reports
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let mut names = HashSet::new();
    for profile in &program.runtime_hardening_profiles {
        let name = profile.name.value.as_str();
        report_duplicate(
            &mut names,
            &profile.name,
            "runtime_hardening_profile",
            diagnostics,
        );
        for (field, value, allowed) in [
            (
                "scope",
                &profile.scope,
                &["agent", "system", "package", "organization"][..],
            ),
            (
                "mode",
                &profile.mode,
                &["declared_only", "evidence_only"][..],
            ),
            (
                "enforcement",
                &profile.enforcement,
                &["declared_only", "evidence_only"][..],
            ),
            ("sandbox", &profile.sandbox, &["required", "declared"][..]),
            (
                "allowlist",
                &profile.allowlist,
                &["required", "declared"][..],
            ),
            ("approval", &profile.approval, &["required", "declared"][..]),
            (
                "incident_response",
                &profile.incident_response,
                &["declared", "required"][..],
            ),
            (
                "residual_risk",
                &profile.residual_risk,
                &["low", "moderate", "high", "critical", "unknown"][..],
            ),
            (
                "review_status",
                &profile.review_status,
                &["draft", "reviewed", "approved_internal", "deprecated"][..],
            ),
            (
                "assurance",
                &profile.assurance,
                &["declared_only", "evidence_mapped", "manually_reviewed"][..],
            ),
        ] {
            require_allowed_value(
                "runtime_hardening_profile",
                name,
                field,
                value,
                allowed,
                diagnostics,
            );
        }
        for (field, value, required) in [
            (
                "provider_execution",
                &profile.provider_execution,
                "disabled",
            ),
            (
                "external_providers",
                &profile.external_providers,
                "disabled",
            ),
            ("network", &profile.network, "denied"),
            ("tool_execution", &profile.tool_execution, "disabled"),
            ("agent_execution", &profile.agent_execution, "disabled"),
            ("filesystem_access", &profile.filesystem_access, "denied"),
            ("env_access", &profile.env_access, "denied"),
            ("secret_material", &profile.secret_material, "denied"),
            ("key_material", &profile.key_material, "denied"),
            ("audit_log", &profile.audit_log, "required"),
            ("evidence", &profile.evidence, "required"),
            ("security_claims", &profile.security_claims, "none"),
        ] {
            require_exact_value(
                "runtime_hardening_profile",
                name,
                field,
                value,
                required,
                diagnostics,
            );
        }
        if !profile.deny_by_default.value {
            diagnostics.push(Diagnostic::new(
                format!("runtime_hardening_profile `{name}` requires deny_by_default `true`"),
                profile.deny_by_default.span,
            ));
        }
        for (kind, value, known) in [
            (
                "evidence_map",
                &profile.evidence_map,
                evidence_maps.contains(profile.evidence_map.value.as_str()),
            ),
            (
                "governance_profile",
                &profile.governance_profile,
                governance_profiles.contains(profile.governance_profile.value.as_str()),
            ),
            (
                "public_conformance_report",
                &profile.public_conformance_report,
                public_reports.contains(profile.public_conformance_report.value.as_str()),
            ),
        ] {
            if !known {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "runtime_hardening_profile `{name}` references unknown {kind} `{}`",
                        value.value
                    ),
                    value.span,
                ));
            }
        }
        require_non_empty_string_list(
            "runtime_hardening_profile",
            name,
            "protected_assets",
            &profile.protected_assets,
            profile.name.span,
            diagnostics,
        );
        require_non_empty_string_list(
            "runtime_hardening_profile",
            name,
            "runtime_boundaries",
            &profile.runtime_boundaries,
            profile.name.span,
            diagnostics,
        );
        require_non_empty_string_list(
            "runtime_hardening_profile",
            name,
            "purpose",
            &profile.purpose,
            profile.name.span,
            diagnostics,
        );
        if let Some(notes) = &profile.notes {
            require_public_conformance_text(
                "runtime_hardening_profile",
                name,
                "notes",
                notes,
                diagnostics,
            );
        }
    }
}

pub fn check_threat_models(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let hardening: std::collections::HashMap<&str, _> = program
        .runtime_hardening_profiles
        .iter()
        .map(|value| (value.name.value.as_str(), value))
        .collect();
    let evidence_maps: HashSet<&str> = program
        .atrust_evidence_maps
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let governance_profiles: HashSet<&str> = program
        .governance_profiles
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let public_reports: HashSet<&str> = program
        .public_conformance_reports
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let mut names = HashSet::new();
    for model in &program.threat_models {
        let name = model.name.value.as_str();
        report_duplicate(&mut names, &model.name, "threat_model", diagnostics);
        let profile = hardening
            .get(model.hardening_profile.value.as_str())
            .copied();
        if profile.is_none() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "threat_model `{name}` references unknown runtime_hardening_profile `{}`",
                    model.hardening_profile.value
                ),
                model.hardening_profile.span,
            ));
        }
        for (kind, value, known) in [
            (
                "evidence_map",
                &model.evidence_map,
                evidence_maps.contains(model.evidence_map.value.as_str()),
            ),
            (
                "governance_profile",
                &model.governance_profile,
                governance_profiles.contains(model.governance_profile.value.as_str()),
            ),
            (
                "public_conformance_report",
                &model.public_conformance_report,
                public_reports.contains(model.public_conformance_report.value.as_str()),
            ),
        ] {
            if !known {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "threat_model `{name}` references unknown {kind} `{}`",
                        value.value
                    ),
                    value.span,
                ));
            }
        }
        if let Some(profile) = profile {
            for (field, actual, expected, span) in [
                (
                    "evidence_map",
                    model.evidence_map.value.as_str(),
                    profile.evidence_map.value.as_str(),
                    model.evidence_map.span,
                ),
                (
                    "governance_profile",
                    model.governance_profile.value.as_str(),
                    profile.governance_profile.value.as_str(),
                    model.governance_profile.span,
                ),
                (
                    "public_conformance_report",
                    model.public_conformance_report.value.as_str(),
                    profile.public_conformance_report.value.as_str(),
                    model.public_conformance_report.span,
                ),
            ] {
                if actual != expected {
                    diagnostics.push(Diagnostic::new(
                        format!("threat_model `{name}` hardening/{field} mismatch"),
                        span,
                    ));
                }
            }
        }
        for (field, value, allowed) in [
            (
                "methodology",
                &model.methodology,
                &["declared", "structured", "internal_review"][..],
            ),
            (
                "scope",
                &model.scope,
                &["agent", "system", "package", "organization"][..],
            ),
            (
                "review_status",
                &model.review_status,
                &["draft", "reviewed", "approved_internal", "deprecated"][..],
            ),
            (
                "residual_risk",
                &model.residual_risk,
                &["low", "moderate", "high", "critical", "unknown"][..],
            ),
            (
                "risk_acceptance",
                &model.risk_acceptance,
                &["declared_only", "pending_review", "manually_reviewed"][..],
            ),
        ] {
            require_allowed_value("threat_model", name, field, value, allowed, diagnostics);
        }
        for (field, value, required) in [
            ("network", &model.network, "denied"),
            ("external_execution", &model.external_execution, "disabled"),
            ("tool_execution", &model.tool_execution, "disabled"),
            ("agent_execution", &model.agent_execution, "disabled"),
            ("secret_material", &model.secret_material, "denied"),
            ("key_material", &model.key_material, "denied"),
            ("execution", &model.execution, "disabled"),
            ("security_claims", &model.security_claims, "none"),
        ] {
            require_exact_value("threat_model", name, field, value, required, diagnostics);
        }
        validate_threat_assets(name, &model.assets, diagnostics);
        validate_threat_entries(name, &model.threats, diagnostics);
        validate_threat_mitigations(name, &model.mitigations, diagnostics);
        require_non_empty_string_list(
            "threat_model",
            name,
            "purpose",
            &model.purpose,
            model.name.span,
            diagnostics,
        );
        if let Some(notes) = &model.notes {
            require_public_conformance_text("threat_model", name, "notes", notes, diagnostics);
        }
    }
}

pub fn check_spec_freezes(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let mut names = HashSet::new();
    for freeze in &program.spec_freezes {
        let name = freeze.name.value.as_str();
        report_duplicate(&mut names, &freeze.name, "spec_freeze", diagnostics);
        require_exact_value(
            "spec_freeze",
            name,
            "version",
            &freeze.version,
            "0.36",
            diagnostics,
        );
        require_public_conformance_text("spec_freeze", name, "target", &freeze.target, diagnostics);
        for (field, value, allowed) in [
            (
                "freeze_scope",
                &freeze.freeze_scope,
                &["language", "bytecode", "conformance", "evidence", "full"][..],
            ),
            (
                "compatibility",
                &freeze.compatibility,
                &["cumulative", "declared_only"][..],
            ),
            (
                "stability",
                &freeze.stability,
                &["rc_candidate", "frozen_draft"][..],
            ),
        ] {
            require_allowed_value("spec_freeze", name, field, value, allowed, diagnostics);
        }
        for (field, value, required) in [
            ("evidence_bundle", &freeze.evidence_bundle, "required"),
            ("security_report", &freeze.security_report, "required"),
            ("conformance", &freeze.conformance, "required"),
            (
                "backward_compatibility",
                &freeze.backward_compatibility,
                "required",
            ),
            ("runtime_status", &freeze.runtime_status, "disabled"),
            ("network", &freeze.network, "denied"),
            ("external_execution", &freeze.external_execution, "disabled"),
            ("provider_execution", &freeze.provider_execution, "disabled"),
            ("secret_material", &freeze.secret_material, "denied"),
            ("key_material", &freeze.key_material, "denied"),
            ("env_access", &freeze.env_access, "denied"),
            ("filesystem_access", &freeze.filesystem_access, "denied"),
            ("tool_execution", &freeze.tool_execution, "disabled"),
            ("agent_execution", &freeze.agent_execution, "disabled"),
            ("security_claims", &freeze.security_claims, "none"),
            ("legal_claims", &freeze.legal_claims, "none"),
            ("certification", &freeze.certification, "none"),
        ] {
            require_exact_value("spec_freeze", name, field, value, required, diagnostics);
        }
        for (field, values) in [
            ("frozen_features", &freeze.frozen_features),
            ("compatible_versions", &freeze.compatible_versions),
            ("required_suites", &freeze.required_suites),
            ("purpose", &freeze.purpose),
        ] {
            require_non_empty_string_list(
                "spec_freeze",
                name,
                field,
                values,
                freeze.name.span,
                diagnostics,
            );
        }
        for required in ["0.34", "0.35", "0.36"] {
            if !freeze
                .compatible_versions
                .iter()
                .any(|value| value.value == required)
            {
                diagnostics.push(Diagnostic::new(
                    format!("spec_freeze `{name}` compatible_versions requires `{required}`"),
                    freeze.name.span,
                ));
            }
        }
        if !freeze
            .required_suites
            .iter()
            .any(|value| value.value == "conformance/suite.v036.json")
        {
            diagnostics.push(Diagnostic::new(
                format!("spec_freeze `{name}` requires conformance/suite.v036.json"),
                freeze.name.span,
            ));
        }
        if let Some(notes) = &freeze.notes {
            require_public_conformance_text("spec_freeze", name, "notes", notes, diagnostics);
        }
    }
}

pub fn check_release_candidates(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let freeze_names: HashSet<&str> = program
        .spec_freezes
        .iter()
        .map(|freeze| freeze.name.value.as_str())
        .collect();
    let mut names = HashSet::new();
    for candidate in &program.release_candidates {
        let name = candidate.name.value.as_str();
        report_duplicate(
            &mut names,
            &candidate.name,
            "release_candidate",
            diagnostics,
        );
        if !candidate.version.value.starts_with("1.0.0-rc") {
            diagnostics.push(Diagnostic::new(
                format!("release_candidate `{name}` version must start with `1.0.0-rc`"),
                candidate.version.span,
            ));
        }
        require_exact_value(
            "release_candidate",
            name,
            "base_version",
            &candidate.base_version,
            "0.36",
            diagnostics,
        );
        if !freeze_names.contains(candidate.spec_freeze.value.as_str()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "release_candidate `{name}` references unknown spec_freeze `{}`",
                    candidate.spec_freeze.value
                ),
                candidate.spec_freeze.span,
            ));
        }
        require_allowed_value(
            "release_candidate",
            name,
            "readiness",
            &candidate.readiness,
            &["draft", "rc", "pending_review"],
            diagnostics,
        );
        for (field, value, required) in [
            ("runtime_status", &candidate.runtime_status, "disabled"),
            ("network", &candidate.network, "denied"),
            (
                "external_execution",
                &candidate.external_execution,
                "disabled",
            ),
            (
                "provider_execution",
                &candidate.provider_execution,
                "disabled",
            ),
            ("secret_material", &candidate.secret_material, "denied"),
            ("key_material", &candidate.key_material, "denied"),
            ("env_access", &candidate.env_access, "denied"),
            ("filesystem_access", &candidate.filesystem_access, "denied"),
            ("tool_execution", &candidate.tool_execution, "disabled"),
            ("agent_execution", &candidate.agent_execution, "disabled"),
            ("security_claims", &candidate.security_claims, "none"),
            ("legal_claims", &candidate.legal_claims, "none"),
            ("certification", &candidate.certification, "none"),
        ] {
            require_exact_value(
                "release_candidate",
                name,
                field,
                value,
                required,
                diagnostics,
            );
        }
        for (field, values) in [
            ("required_artifacts", &candidate.required_artifacts),
            ("required_checks", &candidate.required_checks),
            ("known_limitations", &candidate.known_limitations),
            ("purpose", &candidate.purpose),
        ] {
            require_non_empty_string_list(
                "release_candidate",
                name,
                field,
                values,
                candidate.name.span,
                diagnostics,
            );
        }
        if candidate.compatibility_matrix.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("release_candidate `{name}` requires non-empty compatibility_matrix"),
                candidate.name.span,
            ));
        }
        let mut matrix_versions = HashSet::new();
        for entry in &candidate.compatibility_matrix {
            report_duplicate(
                &mut matrix_versions,
                &entry.version,
                "compatibility matrix version",
                diagnostics,
            );
            require_public_conformance_text(
                "release_candidate",
                name,
                "compatibility matrix version",
                &entry.version,
                diagnostics,
            );
            for (field, value) in [
                ("bytecode compatibility", &entry.bytecode),
                ("evidence compatibility", &entry.evidence),
                ("conformance compatibility", &entry.conformance),
            ] {
                require_allowed_value(
                    "release_candidate",
                    name,
                    field,
                    value,
                    &["compatible", "native", "unsupported"],
                    diagnostics,
                );
            }
        }
        for required in ["0.34", "0.35", "0.36"] {
            if !candidate
                .compatibility_matrix
                .iter()
                .any(|entry| entry.version.value == required)
            {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "release_candidate `{name}` compatibility_matrix requires `{required}`"
                    ),
                    candidate.name.span,
                ));
            }
        }
        if let Some(notes) = &candidate.notes {
            require_public_conformance_text("release_candidate", name, "notes", notes, diagnostics);
        }
    }
}

pub fn check_runtime_execution_profiles(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let providers: HashSet<&str> = program
        .providers
        .iter()
        .map(|value| value.name.value.as_str())
        .chain(std::iter::once("simulated"))
        .collect();
    let agents: HashSet<&str> = program
        .agents
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let hardening: HashSet<&str> = program
        .runtime_hardening_profiles
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let threat_models: HashSet<&str> = program
        .threat_models
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let evidence_maps: HashSet<&str> = program
        .atrust_evidence_maps
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let governance_profiles: std::collections::HashMap<&str, _> = program
        .governance_profiles
        .iter()
        .map(|value| (value.name.value.as_str(), value))
        .collect();
    let mut names = HashSet::new();
    for profile in &program.runtime_execution_profiles {
        let name = profile.name.value.as_str();
        report_duplicate(
            &mut names,
            &profile.name,
            "runtime_execution_profile",
            diagnostics,
        );
        require_allowed_value(
            "runtime_execution_profile",
            name,
            "mode",
            &profile.mode,
            &["dry_run", "simulated", "sandboxed_external"],
            diagnostics,
        );
        for (field, value, allowed) in [
            (
                "network",
                &profile.network,
                &["denied", "declared_only"][..],
            ),
            (
                "external_execution",
                &profile.external_execution,
                &["disabled", "sandboxed"][..],
            ),
            (
                "secrets",
                &profile.secrets,
                &["denied", "env_reference_only"][..],
            ),
        ] {
            require_allowed_value(
                "runtime_execution_profile",
                name,
                field,
                value,
                allowed,
                diagnostics,
            );
        }
        for (field, value, required) in [
            ("tool_execution", &profile.tool_execution, "disabled"),
            ("agent_execution", &profile.agent_execution, "disabled"),
            ("key_material", &profile.key_material, "denied"),
            ("audit", &profile.audit, "required"),
            ("evidence", &profile.evidence, "required"),
            ("security_report", &profile.security_report, "required"),
            ("security_claims", &profile.security_claims, "none"),
        ] {
            require_exact_value(
                "runtime_execution_profile",
                name,
                field,
                value,
                required,
                diagnostics,
            );
        }
        if !profile.fail_closed.value {
            diagnostics.push(Diagnostic::new(
                format!("runtime_execution_profile `{name}` requires fail_closed `true`"),
                profile.fail_closed.span,
            ));
        }
        require_non_empty_string_list(
            "runtime_execution_profile",
            name,
            "agents",
            &profile.agents,
            profile.name.span,
            diagnostics,
        );
        for agent in &profile.agents {
            if !agents.contains(agent.value.as_str()) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "runtime_execution_profile `{name}` references unknown agent `{}`",
                        agent.value
                    ),
                    agent.span,
                ));
            }
        }
        for (kind, value, known) in [
            (
                "provider",
                &profile.provider,
                providers.contains(profile.provider.value.as_str()),
            ),
            (
                "runtime_hardening_profile",
                &profile.hardening,
                hardening.contains(profile.hardening.value.as_str()),
            ),
            (
                "threat_model",
                &profile.threat_model,
                threat_models.contains(profile.threat_model.value.as_str()),
            ),
            (
                "evidence_map",
                &profile.evidence_map,
                evidence_maps.contains(profile.evidence_map.value.as_str()),
            ),
            (
                "governance_profile",
                &profile.governance_profile,
                governance_profiles.contains_key(profile.governance_profile.value.as_str()),
            ),
        ] {
            if !known {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "runtime_execution_profile `{name}` references unknown {kind} `{}`",
                        value.value
                    ),
                    value.span,
                ));
            }
        }
        if let Some(governance) = governance_profiles.get(profile.governance_profile.value.as_str())
        {
            if governance.evidence_map.value != profile.evidence_map.value {
                diagnostics.push(Diagnostic::new(
                    format!("runtime_execution_profile `{name}` governance/evidence_map mismatch"),
                    profile.governance_profile.span,
                ));
            }
        }
        require_non_empty_string_list(
            "runtime_execution_profile",
            name,
            "allowed_actions",
            &profile.allowed_actions,
            profile.name.span,
            diagnostics,
        );
        require_non_empty_string_list(
            "runtime_execution_profile",
            name,
            "denied_actions",
            &profile.denied_actions,
            profile.name.span,
            diagnostics,
        );
        require_disjoint_string_lists(
            "runtime_execution_profile",
            name,
            "allowed_actions",
            &profile.allowed_actions,
            "denied_actions",
            &profile.denied_actions,
            diagnostics,
        );
        require_non_empty_string_list(
            "runtime_execution_profile",
            name,
            "purpose",
            &profile.purpose,
            profile.name.span,
            diagnostics,
        );
        if let Some(notes) = &profile.notes {
            require_public_conformance_text(
                "runtime_execution_profile",
                name,
                "notes",
                notes,
                diagnostics,
            );
        }
    }
}

pub fn check_sandboxed_provider_adapters(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let providers: HashSet<&str> = program
        .providers
        .iter()
        .map(|value| value.name.value.as_str())
        .chain(std::iter::once("simulated"))
        .collect();
    let runtimes: std::collections::HashMap<&str, _> = program
        .runtime_execution_profiles
        .iter()
        .map(|value| (value.name.value.as_str(), value))
        .collect();
    let secret_boundaries: HashSet<&str> = program
        .secrets
        .iter()
        .map(|value| value.name.value.as_str())
        .collect();
    let mut names = HashSet::new();
    for adapter in &program.sandboxed_provider_adapters {
        let name = adapter.name.value.as_str();
        report_duplicate(
            &mut names,
            &adapter.name,
            "sandboxed_provider_adapter",
            diagnostics,
        );
        if !providers.contains(adapter.provider.value.as_str()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "sandboxed_provider_adapter `{name}` references unknown provider `{}`",
                    adapter.provider.value
                ),
                adapter.provider.span,
            ));
        }
        let runtime = runtimes.get(adapter.runtime.value.as_str()).copied();
        if runtime.is_none() {
            diagnostics.push(Diagnostic::new(
                format!(
                    "sandboxed_provider_adapter `{name}` references unknown runtime_execution_profile `{}`",
                    adapter.runtime.value
                ),
                adapter.runtime.span,
            ));
        } else if runtime.is_some_and(|value| value.provider.value != adapter.provider.value) {
            diagnostics.push(Diagnostic::new(
                format!("sandboxed_provider_adapter `{name}` provider/runtime mismatch"),
                adapter.provider.span,
            ));
        }
        for (field, value, allowed) in [
            (
                "adapter_kind",
                &adapter.adapter_kind,
                &["llm", "embedding", "moderation", "custom"][..],
            ),
            (
                "protocol",
                &adapter.protocol,
                &["https_declared", "local_declared"][..],
            ),
            (
                "request_policy",
                &adapter.request_policy,
                &["declared_only", "redacted_logged"][..],
            ),
            (
                "response_policy",
                &adapter.response_policy,
                &["evidence_logged", "redacted_logged"][..],
            ),
            (
                "network",
                &adapter.network,
                &["declared_only", "denied"][..],
            ),
            (
                "external_execution",
                &adapter.external_execution,
                &["sandboxed", "disabled"][..],
            ),
            (
                "secret_material",
                &adapter.secret_material,
                &["env_reference_only", "denied"][..],
            ),
        ] {
            require_allowed_value(
                "sandboxed_provider_adapter",
                name,
                field,
                value,
                allowed,
                diagnostics,
            );
        }
        for (field, value, required) in [
            ("tool_execution", &adapter.tool_execution, "disabled"),
            ("key_material", &adapter.key_material, "denied"),
            ("audit", &adapter.audit, "required"),
            ("evidence", &adapter.evidence, "required"),
            ("security_report", &adapter.security_report, "required"),
            ("security_claims", &adapter.security_claims, "none"),
        ] {
            require_exact_value(
                "sandboxed_provider_adapter",
                name,
                field,
                value,
                required,
                diagnostics,
            );
        }
        if !adapter.fail_closed.value {
            diagnostics.push(Diagnostic::new(
                format!("sandboxed_provider_adapter `{name}` requires fail_closed `true`"),
                adapter.fail_closed.span,
            ));
        }
        if !is_declared_reference(&adapter.endpoint_ref.value, &["env", "config"]) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "sandboxed_provider_adapter `{name}` endpoint_ref must use `env:<NAME>` or `config:<NAME>`"
                ),
                adapter.endpoint_ref.span,
            ));
        }
        if !is_declared_reference(&adapter.secret_ref.value, &["env", "secret_boundary"]) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "sandboxed_provider_adapter `{name}` secret_ref must use `env:<NAME>` or `secret_boundary:<NAME>`"
                ),
                adapter.secret_ref.span,
            ));
        } else if let Some(boundary) = adapter.secret_ref.value.strip_prefix("secret_boundary:") {
            if !secret_boundaries.contains(boundary) {
                diagnostics.push(Diagnostic::new(
                    format!(
                        "sandboxed_provider_adapter `{name}` references unknown secret boundary `{boundary}`"
                    ),
                    adapter.secret_ref.span,
                ));
            }
        }
        require_non_empty_string_list(
            "sandboxed_provider_adapter",
            name,
            "allowed_operations",
            &adapter.allowed_operations,
            adapter.name.span,
            diagnostics,
        );
        require_non_empty_string_list(
            "sandboxed_provider_adapter",
            name,
            "denied_operations",
            &adapter.denied_operations,
            adapter.name.span,
            diagnostics,
        );
        require_disjoint_string_lists(
            "sandboxed_provider_adapter",
            name,
            "allowed_operations",
            &adapter.allowed_operations,
            "denied_operations",
            &adapter.denied_operations,
            diagnostics,
        );
        require_non_empty_string_list(
            "sandboxed_provider_adapter",
            name,
            "purpose",
            &adapter.purpose,
            adapter.name.span,
            diagnostics,
        );
        if let Some(notes) = &adapter.notes {
            require_public_conformance_text(
                "sandboxed_provider_adapter",
                name,
                "notes",
                notes,
                diagnostics,
            );
        }
    }
}

fn require_disjoint_string_lists(
    kind: &str,
    name: &str,
    left_name: &str,
    left: &[Spanned<String>],
    right_name: &str,
    right: &[Spanned<String>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let right_values: HashSet<&str> = right.iter().map(|value| value.value.as_str()).collect();
    for value in left {
        if right_values.contains(value.value.as_str()) {
            diagnostics.push(Diagnostic::new(
                format!(
                    "{kind} `{name}` has overlapping {left_name}/{right_name} value `{}`",
                    value.value
                ),
                value.span,
            ));
        }
    }
}

fn is_declared_reference(value: &str, allowed_prefixes: &[&str]) -> bool {
    let Some((prefix, name)) = value.split_once(':') else {
        return false;
    };
    allowed_prefixes.contains(&prefix)
        && !name.is_empty()
        && name
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn validate_threat_assets(
    name: &str,
    assets: &[argorix_parser::ast::ThreatAssetDecl],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if assets.is_empty() {
        diagnostics.push(Diagnostic::new(
            format!("threat_model `{name}` requires non-empty assets"),
            argorix_parser::span::Span::new(0, 0, 1, 1),
        ));
    }
    let mut ids = HashSet::new();
    for asset in assets {
        report_duplicate(&mut ids, &asset.id, "threat asset id", diagnostics);
        for (field, value) in [
            ("id", &asset.id),
            ("description", &asset.description),
            ("evidence_ref", &asset.evidence_ref),
        ] {
            require_public_conformance_text("threat_model", name, field, value, diagnostics);
        }
        require_allowed_value(
            "threat_model",
            name,
            "asset category",
            &asset.category,
            &[
                "secret",
                "key",
                "identity",
                "credential",
                "handshake",
                "ledger",
                "bridge",
                "evidence",
                "policy",
                "runtime",
                "provider",
                "user_data",
                "custom",
            ],
            diagnostics,
        );
        require_allowed_value(
            "threat_model",
            name,
            "asset sensitivity",
            &asset.sensitivity,
            &["low", "moderate", "high", "critical", "unknown"],
            diagnostics,
        );
    }
}

fn validate_threat_entries(
    name: &str,
    threats: &[argorix_parser::ast::ThreatDecl],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if threats.is_empty() {
        diagnostics.push(Diagnostic::new(
            format!("threat_model `{name}` requires non-empty threats"),
            argorix_parser::span::Span::new(0, 0, 1, 1),
        ));
    }
    let mut ids = HashSet::new();
    for threat in threats {
        report_duplicate(&mut ids, &threat.id, "threat id", diagnostics);
        for (field, value) in [
            ("id", &threat.id),
            ("target", &threat.target),
            ("mitigation", &threat.mitigation),
        ] {
            require_public_conformance_text("threat_model", name, field, value, diagnostics);
        }
        require_allowed_value(
            "threat_model",
            name,
            "threat category",
            &threat.category,
            &[
                "prompt_injection",
                "secret_leakage",
                "tool_abuse",
                "network_exfiltration",
                "identity_spoofing",
                "credential_misuse",
                "handshake_replay",
                "bridge_misuse",
                "evidence_tampering",
                "policy_bypass",
                "provider_misuse",
                "runtime_escape",
                "supply_chain",
                "custom",
            ],
            diagnostics,
        );
        require_allowed_value(
            "threat_model",
            name,
            "threat impact",
            &threat.impact,
            &["low", "moderate", "high", "critical", "unknown"],
            diagnostics,
        );
        require_allowed_value(
            "threat_model",
            name,
            "threat status",
            &threat.status,
            &[
                "declared",
                "mitigated_declared",
                "pending_review",
                "accepted_risk",
                "not_applicable",
            ],
            diagnostics,
        );
    }
}

fn validate_threat_mitigations(
    name: &str,
    mitigations: &[argorix_parser::ast::ThreatMitigationDecl],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if mitigations.is_empty() {
        diagnostics.push(Diagnostic::new(
            format!("threat_model `{name}` requires non-empty mitigations"),
            argorix_parser::span::Span::new(0, 0, 1, 1),
        ));
    }
    let mut ids = HashSet::new();
    for mitigation in mitigations {
        report_duplicate(
            &mut ids,
            &mitigation.id,
            "threat mitigation id",
            diagnostics,
        );
        for (field, value) in [
            ("id", &mitigation.id),
            ("control_ref", &mitigation.control_ref),
            ("evidence_ref", &mitigation.evidence_ref),
        ] {
            require_public_conformance_text("threat_model", name, field, value, diagnostics);
        }
        require_allowed_value(
            "threat_model",
            name,
            "mitigation category",
            &mitigation.category,
            &[
                "network_boundary",
                "secret_boundary",
                "key_boundary",
                "tool_boundary",
                "agent_boundary",
                "provider_boundary",
                "policy_enforcement",
                "audit_logging",
                "evidence_mapping",
                "sandboxing",
                "review_process",
                "custom",
            ],
            diagnostics,
        );
        require_allowed_value(
            "threat_model",
            name,
            "mitigation status",
            &mitigation.status,
            &["mapped", "declared", "pending_review", "not_applicable"],
            diagnostics,
        );
    }
}

fn require_allowed_value(
    kind: &str,
    name: &str,
    field: &str,
    value: &Spanned<String>,
    allowed: &[&str],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !allowed.contains(&value.value.as_str()) {
        diagnostics.push(Diagnostic::new(
            format!("{kind} `{name}` has invalid {field} `{}`", value.value),
            value.span,
        ));
    }
}

fn require_exact_value(
    kind: &str,
    name: &str,
    field: &str,
    value: &Spanned<String>,
    required: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if value.value != required {
        diagnostics.push(Diagnostic::new(
            format!("{kind} `{name}` requires {field} `{required}`"),
            value.span,
        ));
    }
}

fn require_non_empty_string_list(
    kind: &str,
    name: &str,
    field: &str,
    values: &[Spanned<String>],
    fallback: argorix_parser::span::Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if values.is_empty() {
        diagnostics.push(Diagnostic::new(
            format!("{kind} `{name}` requires non-empty {field}"),
            fallback,
        ));
    }
    for value in values {
        if value.value.is_empty() {
            diagnostics.push(Diagnostic::new(
                format!("{kind} `{name}` {field} must not contain empty strings"),
                value.span,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn require_public_denied_boundaries(
    kind: &str,
    name: &str,
    network: &str,
    network_span: argorix_parser::span::Span,
    external_execution: &str,
    external_execution_span: argorix_parser::span::Span,
    secret_material: &str,
    secret_material_span: argorix_parser::span::Span,
    key_material: &str,
    key_material_span: argorix_parser::span::Span,
    execution: &str,
    execution_span: argorix_parser::span::Span,
    legal_claims: &Spanned<String>,
    certification: &Spanned<String>,
    security_claims: &str,
    security_claims_span: argorix_parser::span::Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (field, actual, required, span) in [
        ("network", network, "denied", network_span),
        (
            "external_execution",
            external_execution,
            "disabled",
            external_execution_span,
        ),
        (
            "secret_material",
            secret_material,
            "denied",
            secret_material_span,
        ),
        ("key_material", key_material, "denied", key_material_span),
        ("execution", execution, "disabled", execution_span),
        (
            "legal_claims",
            legal_claims.value.as_str(),
            "none",
            legal_claims.span,
        ),
        (
            "certification",
            certification.value.as_str(),
            "none",
            certification.span,
        ),
        (
            "security_claims",
            security_claims,
            "none",
            security_claims_span,
        ),
    ] {
        if actual != required {
            diagnostics.push(Diagnostic::new(
                format!("{kind} `{name}` requires {field} `{required}`"),
                span,
            ));
        }
    }
}

fn require_regulatory_text(
    name: &str,
    field: &str,
    value: &Spanned<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if value.value.is_empty() {
        diagnostics.push(Diagnostic::new(
            format!("regulatory_mapping `{name}` {field} must not be empty"),
            value.span,
        ));
    }
}

fn invalid_governance_value(
    name: &str,
    field: &str,
    value: &str,
    span: argorix_parser::span::Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    diagnostics.push(Diagnostic::new(
        format!("governance_profile `{name}` has invalid {field} `{value}`"),
        span,
    ));
}

fn invalid_regulatory_value(
    name: &str,
    field: &str,
    value: &str,
    span: argorix_parser::span::Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    diagnostics.push(Diagnostic::new(
        format!("regulatory_mapping `{name}` has invalid {field} `{value}`"),
        span,
    ));
}

#[allow(clippy::too_many_arguments)]
fn require_governance_denied_boundaries(
    kind: &str,
    name: &str,
    network: &str,
    network_span: argorix_parser::span::Span,
    external_execution: &str,
    external_execution_span: argorix_parser::span::Span,
    secret_material: &str,
    secret_material_span: argorix_parser::span::Span,
    key_material: &str,
    key_material_span: argorix_parser::span::Span,
    execution: &str,
    execution_span: argorix_parser::span::Span,
    security_claims: &str,
    security_claims_span: argorix_parser::span::Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (field, actual, required, span) in [
        ("network", network, "denied", network_span),
        (
            "external_execution",
            external_execution,
            "disabled",
            external_execution_span,
        ),
        (
            "secret_material",
            secret_material,
            "denied",
            secret_material_span,
        ),
        ("key_material", key_material, "denied", key_material_span),
        ("execution", execution, "disabled", execution_span),
        (
            "security_claims",
            security_claims,
            "none",
            security_claims_span,
        ),
    ] {
        if actual != required {
            diagnostics.push(Diagnostic::new(
                format!("{kind} `{name}` requires {field} `{required}`"),
                span,
            ));
        }
    }
}

fn require_ledger_entry(
    map_name: &str,
    ledger: &argorix_parser::ast::TrustLedgerDecl,
    kind: &str,
    subject: &str,
    span_source: &Spanned<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if subject.is_empty() {
        return;
    }
    let found = ledger
        .entries
        .iter()
        .any(|entry| entry.kind.value.source_name() == kind && entry.subject.value == subject);
    if !found {
        diagnostics.push(Diagnostic::new(
            format!(
                "atrust_evidence_map `{map_name}` trust_ledger `{}` does not contain {kind} `{subject}`",
                ledger.name.value
            ),
            span_source.span,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::check_program;
    use argorix_parser::parse_source;

    #[test]
    fn rejects_unknown_messages_and_agents() {
        let source = r#"
            module Example
            agent Sender { sends Missing to Nobody }
        "#;
        let program = parse_source(source).unwrap();
        let diagnostics = check_program(&program).unwrap_err();
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].message.contains("unknown message type"));
        assert!(diagnostics[1].message.contains("unknown send destination"));
    }

    #[test]
    fn rejects_invalid_policy_v2_declarations() {
        let source = r#"
            module main
            policy Duplicate {
                require external_execution
                require external_execution
            }
            policy Contradictory {
                require external_execution
                deny external_execution
            }
            policy UnknownRule {
                require future_rule
                on violation { action future_action }
            }
            policy Duplicate { deny external_execution }
        "#;
        let program = parse_source(source).unwrap();
        let messages = check_program(&program)
            .unwrap_err()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>();
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate policy `Duplicate`")));
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate require rule `external_execution`")));
        assert!(messages
            .iter()
            .any(|message| { message.contains("contradictory policy rule `external_execution`") }));
        assert!(messages
            .iter()
            .any(|message| message.contains("unknown policy rule `future_rule`")));
        assert!(messages
            .iter()
            .any(|message| message.contains("invalid policy violation action `future_action`")));
    }

    #[test]
    fn accepts_policy_v2_and_legacy_assertions_together() {
        let source = r#"
            module main
            assert all_tool_calls_traced
            policy RuntimeSafety {
                require no_unhandled_messages
                deny external_execution
                on violation { action review trace required }
            }
        "#;
        let program = parse_source(source).unwrap();
        check_program(&program).unwrap();
    }

    fn passport_messages(source: &str) -> Vec<String> {
        check_program(&parse_source(source).unwrap())
            .unwrap_err()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()
    }

    #[test]
    fn accepts_valid_passport() {
        let source = r#"
            module main
            agent ResearchAgent {}
            passport RiskAnalyzerPassport {
                agent ResearchAgent
                agent_name "Risk Analyzer"
                global_id "argx:agent:01HZX9"
                identity "did:argorix:risk-v1"
                provider "Argorix"
                version "1.0.0"
                country "CL"
                jurisdiction "CL"
                data_residency ["CL", "EU"]
                asn { registry "LACNIC" number "AS-PLACEHOLDER" holder "Argorix Labs" country "CL" }
                risk_level "high"
                intent "risk_analysis"
                attestations ["redteam"]
            }
        "#;
        check_program(&parse_source(source).unwrap()).unwrap();
    }

    #[test]
    fn rejects_invalid_passports() {
        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent A agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" country \"CL\" jurisdiction \"CL\" data_residency [\"CL\"] risk_level \"high\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("missing required field `version`")));

        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent Missing agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" version \"1\" country \"CL\" jurisdiction \"CL\" data_residency [\"CL\"] risk_level \"high\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("references unknown agent `Missing`")));

        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent A agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" version \"1\" country \"CL\" jurisdiction \"CL\" data_residency [\"CL\"] risk_level \"extreme\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("invalid risk_level `extreme`")));

        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent A agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" version \"1\" country \"Chile\" jurisdiction \"CL\" data_residency [\"CL\"] risk_level \"high\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("2-letter ISO-like code")));

        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent A agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" version \"1\" country \"CL\" jurisdiction \"CL\" data_residency [] risk_level \"high\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("non-empty `data_residency`")));

        assert!(passport_messages(
            "module main\nagent A {}\npassport P { agent A agent_name \"n\" global_id \"g\" identity \"i\" provider \"p\" version \"1\" country \"CL\" jurisdiction \"CL\" data_residency [\"CL\"] asn { registry \"FOO\" number \"AS-1\" holder \"h\" country \"CL\" } risk_level \"high\" intent \"x\" }\n"
        )
        .iter()
        .any(|message| message.contains("invalid asn registry `FOO`")));
    }

    #[test]
    fn rejects_duplicate_passport_names_and_per_agent() {
        let source = r#"
            module main
            agent A {}
            passport First { agent A agent_name "n" global_id "g" identity "i" provider "p" version "1" country "CL" jurisdiction "CL" data_residency ["CL"] risk_level "high" intent "x" }
            passport First { agent A agent_name "n" global_id "g" identity "i" provider "p" version "1" country "CL" jurisdiction "CL" data_residency ["CL"] risk_level "high" intent "x" }
        "#;
        let messages = passport_messages(source);
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate passport `First`")));
        assert!(messages
            .iter()
            .any(|message| message.contains("already has a passport")));
    }

    #[test]
    fn validates_typed_and_nominal_message_fields() {
        let valid = parse_source(
            "module main\nenum RiskLevel { low high }\ntype Message { content: string risk: RiskLevel }\n",
        )
        .unwrap();
        check_program(&valid).unwrap();

        let invalid =
            parse_source("module main\ntype Message { value: future value: string }\n").unwrap();
        let messages = check_program(&invalid)
            .unwrap_err()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>();
        assert!(messages
            .iter()
            .any(|message| message.contains("unknown message field type `future`")));
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate field `value`")));
    }

    fn harness_messages(body: &str) -> Vec<String> {
        let source = format!(
            "module main\nprovider OpenAI {{ kind external enabled false dry_run_only true requires feature_flag requires approval }}\ntype UserPrompt {{ content: string }}\ntype DraftAnswer {{ content: string }}\n{body}\n"
        );
        check_program(&parse_source(&source).unwrap())
            .unwrap_err()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()
    }

    #[test]
    fn accepts_valid_provider_harness_and_empty_attestations() {
        let source = r#"
            module main
            provider OpenAI {
                kind external
                enabled false
                dry_run_only true
                requires feature_flag
                requires approval
            }
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
                attestations []
            }
        "#;
        check_program(&parse_source(source).unwrap()).unwrap();
    }

    #[test]
    fn rejects_missing_and_unknown_harness_fields() {
        let missing = harness_messages("harness H {}");
        for field in ["provider", "mode", "network", "secrets", "filesystem"] {
            assert!(missing
                .iter()
                .any(|message| message.contains(&format!("missing required field `{field}`"))));
        }

        let invalid = harness_messages(
            "harness H { provider Missing mode live network allowed secrets env filesystem write }",
        );
        for expected in [
            "unknown provider `Missing`",
            "invalid mode `live`",
            "invalid network `allowed`",
            "invalid secrets `env`",
            "invalid filesystem `write`",
        ] {
            assert!(invalid.iter().any(|message| message.contains(expected)));
        }
    }

    #[test]
    fn rejects_duplicate_harness_limits_contracts_and_empty_entries() {
        let messages = harness_messages(
            r#"
            harness H {
                provider OpenAI
                mode simulated
                network denied
                secrets denied
                filesystem read_only
                max_steps 0
                timeout_ms 0
                input_contract MissingInput
                output_contract MissingOutput
                attestations [""]
            }
            harness H {
                provider OpenAI
                mode dry_run
                network denied
                secrets denied
                filesystem none
            }
            "#,
        );
        for expected in [
            "duplicate harness `H`",
            "max_steps must be positive",
            "timeout_ms must be positive",
            "unknown input_contract `MissingInput`",
            "unknown output_contract `MissingOutput`",
            "attestation entries must not be empty",
        ] {
            assert!(messages.iter().any(|message| message.contains(expected)));
        }
    }

    fn boundary_messages(body: &str) -> Vec<String> {
        let source = format!(
            "module main\nprovider OpenAI {{ kind external enabled false dry_run_only true requires feature_flag requires approval }}\n{body}\n"
        );
        check_program(&parse_source(&source).unwrap())
            .unwrap_err()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()
    }

    fn boundary_ok(body: &str) {
        let source = format!(
            "module main\nprovider OpenAI {{ kind external enabled false dry_run_only true requires feature_flag requires approval }}\n{body}\n"
        );
        check_program(&parse_source(&source).unwrap()).unwrap();
    }

    #[test]
    fn accepts_valid_feature_and_secret() {
        boundary_ok(
            r#"
            feature OpenAIAdapter { provider OpenAI status experimental default disabled requires approval purpose "p" }
            secret OpenAISecret { handle "OPENAI_API_KEY" provider OpenAI required_by OpenAIAdapter scope adapter access denied source none }
            "#,
        );
    }

    #[test]
    fn rejects_invalid_features() {
        assert!(boundary_messages(
            "feature F { provider OpenAI default disabled requires approval }"
        )
        .iter()
        .any(|m| m.contains("missing required field `status`")));
        assert!(boundary_messages(
            "feature F { provider OpenAI status experimental requires approval }"
        )
        .iter()
        .any(|m| m.contains("missing required field `default`")));
        assert!(boundary_messages(
            "feature F { provider OpenAI status legendary default disabled requires approval }"
        )
        .iter()
        .any(|m| m.contains("invalid status")));
        assert!(boundary_messages(
            "feature F { provider OpenAI status experimental default disabled }"
        )
        .iter()
        .any(|m| m.contains("requires approval")));
        assert!(
            boundary_messages("feature F { provider OpenAI status stable default enabled }")
                .iter()
                .any(|m| m.contains("must default to disabled"))
        );
        assert!(boundary_messages(
            "feature F { provider Ghost status experimental default disabled requires approval }"
        )
        .iter()
        .any(|m| m.contains("unknown provider")));
    }

    #[test]
    fn rejects_invalid_secrets() {
        assert!(boundary_messages(
            "secret S { provider OpenAI scope adapter access denied source none }"
        )
        .iter()
        .any(|m| m.contains("missing required field `handle`")));
        assert!(boundary_messages(
            "secret S { handle \"H\" provider OpenAI scope galaxy access denied source none }"
        )
        .iter()
        .any(|m| m.contains("invalid scope")));
        assert!(boundary_messages(
            "secret S { handle \"H\" provider OpenAI scope adapter access allowed source none }"
        )
        .iter()
        .any(|m| m.contains("invalid access")));
        assert!(boundary_messages(
            "secret S { handle \"H\" provider OpenAI scope adapter access denied source env }"
        )
        .iter()
        .any(|m| m.contains("invalid source")));
        assert!(boundary_messages("secret S { handle \"H\" provider OpenAI required_by Ghost scope adapter access denied source none }")
            .iter()
            .any(|m| m.contains("unknown feature")));
    }

    #[test]
    fn rejects_harness_link_mismatches() {
        let messages = boundary_messages(
            r#"
            provider OtherProv { kind external enabled false dry_run_only true requires feature_flag requires approval }
            feature OpenAIAdapter { provider OtherProv status experimental default disabled requires approval }
            harness H { provider OpenAI feature OpenAIAdapter mode dry_run network denied secrets denied filesystem none }
            "#,
        );
        assert!(messages
            .iter()
            .any(|m| m.contains("does not match feature")));

        let unknown = boundary_messages(
            "harness H { provider OpenAI feature Ghost mode dry_run network denied secrets denied filesystem none }",
        );
        assert!(unknown.iter().any(|m| m.contains("unknown feature")));
    }
}

#[cfg(test)]
mod atrust_handshake_tests {
    use super::check_program;
    use argorix_parser::parse_source;

    /// Builds a full, otherwise-valid v0.29 program with the supplied
    /// `atrust_handshake` block spliced in.
    fn program_with(handshake: &str) -> String {
        format!(
            r#"module HandshakeTest

crypto_boundary TrustBoundary {{
  allowed_hashes ["sha256"]
  key_material denied
  secret_material denied
  execution disabled
  purpose ["handshake"]
}}

did_method argorix {{
  status experimental
  resolution embedded
  ledger local
  crypto_boundary TrustBoundary
  purpose ["identity"]
}}

atrust_boundary AgentTrustBoundary {{
  crypto_boundary TrustBoundary
  did_methods ["argorix"]
  identity_format did
  credential_mode declared_only
  handshake disabled
  resolution disabled
  key_material denied
  secret_material denied
  execution disabled
  security_claims none
  purpose ["identity"]
}}

atrust_identity ResearchIdentity {{
  subject ResearchAgent
  did "did:argorix:research-agent-v1"
  method argorix
  boundary AgentTrustBoundary
  status active
  validation dry_run
  resolution disabled
  key_material denied
  secret_material denied
  execution disabled
  evidence required
  security_claims none
  purpose ["identity"]
}}

atrust_identity VerifierIdentity {{
  subject VerifierAgent
  did "did:argorix:verifier-v1"
  method argorix
  boundary AgentTrustBoundary
  status active
  validation dry_run
  resolution disabled
  key_material denied
  secret_material denied
  execution disabled
  evidence required
  security_claims none
  purpose ["identity"]
}}

atrust_credential_contract ResearchCredential {{
  subject ResearchAgent
  identity ResearchIdentity
  boundary AgentTrustBoundary
  method argorix
  issuer_did "did:argorix:issuer-v1"
  holder_did "did:argorix:research-agent-v1"
  credential_type "ResearchAccessCredential"
  schema "argorix:credential:research-access:v1"
  status declared
  verification declared_only
  presentation disabled
  resolution disabled
  key_material denied
  secret_material denied
  execution disabled
  evidence required
  security_claims none
  claims ["role", "scope"]
  purpose ["credential"]
}}

{handshake}

type UserPrompt {{ content: string }}
agent ResearchAgent {{ receives UserPrompt }}
agent VerifierAgent {{ receives UserPrompt }}

protocol Demo {{
  User -> ResearchAgent: tell UserPrompt
}}
"#
        )
    }

    const VALID: &str = r#"atrust_handshake ResearchHandshake {
  initiator ResearchAgent
  responder VerifierAgent
  initiator_identity ResearchIdentity
  responder_identity VerifierIdentity
  credential_contracts ["ResearchCredential"]
  boundary AgentTrustBoundary
  method argorix
  mode dry_run
  direction mutual
  challenge declared_only
  response declared_only
  transcript evidence_only
  verification declared_only
  resolution disabled
  network denied
  key_material denied
  secret_material denied
  execution disabled
  evidence required
  security_claims none
  purpose ["handshake", "dry-run"]
}"#;

    /// Returns a copy of VALID with the line containing `remove_contains`
    /// dropped (modeling a missing field) or replaced by `replacement`.
    fn variant(remove_contains: &str, replacement: Option<&str>) -> String {
        VALID
            .lines()
            .filter_map(|l| {
                if l.contains(remove_contains) {
                    replacement.map(|r| format!("  {r}"))
                } else {
                    Some(l.to_owned())
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn accepts(handshake: &str) {
        let src = program_with(handshake);
        let ast = parse_source(&src).expect("parse");
        check_program(&ast).expect("semantic check should pass");
    }

    fn rejects(handshake: &str) {
        let src = program_with(handshake);
        let ast = parse_source(&src).expect("parse");
        assert!(check_program(&ast).is_err(), "expected semantic rejection");
    }

    #[test]
    fn accepts_valid_handshake() {
        accepts(VALID);
    }

    #[test]
    fn parses_handshake_block() {
        let ast = parse_source(&program_with(VALID)).expect("parse");
        assert_eq!(ast.atrust_handshakes.len(), 1);
        assert_eq!(ast.atrust_handshakes[0].name.value, "ResearchHandshake");
    }

    #[test]
    fn rejects_missing_initiator() {
        rejects(&variant("initiator ResearchAgent", None));
    }

    #[test]
    fn rejects_unknown_initiator() {
        rejects(&variant(
            "initiator ResearchAgent",
            Some("initiator GhostAgent"),
        ));
    }

    #[test]
    fn rejects_missing_responder() {
        rejects(&variant("responder VerifierAgent", None));
    }

    #[test]
    fn rejects_unknown_responder() {
        rejects(&variant(
            "responder VerifierAgent",
            Some("responder GhostAgent"),
        ));
    }

    #[test]
    fn rejects_same_initiator_responder() {
        rejects(&variant(
            "responder VerifierAgent",
            Some("responder ResearchAgent"),
        ));
    }

    #[test]
    fn rejects_missing_initiator_identity() {
        rejects(&variant("initiator_identity ResearchIdentity", None));
    }

    #[test]
    fn rejects_unknown_initiator_identity() {
        rejects(&variant(
            "initiator_identity ResearchIdentity",
            Some("initiator_identity GhostIdentity"),
        ));
    }

    #[test]
    fn rejects_initiator_identity_subject_mismatch() {
        // VerifierIdentity has subject VerifierAgent, not the initiator agent.
        rejects(&variant(
            "initiator_identity ResearchIdentity",
            Some("initiator_identity VerifierIdentity"),
        ));
    }

    #[test]
    fn rejects_missing_responder_identity() {
        rejects(&variant("responder_identity VerifierIdentity", None));
    }

    #[test]
    fn rejects_unknown_responder_identity() {
        rejects(&variant(
            "responder_identity VerifierIdentity",
            Some("responder_identity GhostIdentity"),
        ));
    }

    #[test]
    fn rejects_missing_credential_contracts() {
        rejects(&variant("credential_contracts", None));
    }

    #[test]
    fn rejects_unknown_credential_contract() {
        rejects(&variant(
            "credential_contracts",
            Some(r#"credential_contracts ["GhostCredential"]"#),
        ));
    }

    #[test]
    fn rejects_missing_boundary() {
        rejects(&variant("boundary AgentTrustBoundary", None));
    }

    #[test]
    fn rejects_unknown_boundary() {
        rejects(&variant(
            "boundary AgentTrustBoundary",
            Some("boundary GhostBoundary"),
        ));
    }

    #[test]
    fn rejects_missing_method() {
        rejects(&variant("method argorix", None));
    }

    #[test]
    fn rejects_unknown_method() {
        rejects(&variant("method argorix", Some("method ghostdid")));
    }

    #[test]
    fn rejects_missing_mode() {
        rejects(&variant("mode dry_run", None));
    }

    #[test]
    fn rejects_mode_real() {
        rejects(&variant("mode dry_run", Some("mode real")));
    }

    #[test]
    fn rejects_missing_direction() {
        rejects(&variant("direction mutual", None));
    }

    #[test]
    fn rejects_challenge_generated() {
        rejects(&variant(
            "challenge declared_only",
            Some("challenge generated"),
        ));
    }

    #[test]
    fn rejects_response_signed() {
        rejects(&variant("response declared_only", Some("response signed")));
    }

    #[test]
    fn rejects_transcript_raw() {
        rejects(&variant("transcript evidence_only", Some("transcript raw")));
    }

    #[test]
    fn rejects_verification_real() {
        rejects(&variant(
            "verification declared_only",
            Some("verification real"),
        ));
    }

    #[test]
    fn rejects_resolution_remote() {
        rejects(&variant("resolution disabled", Some("resolution remote")));
    }

    #[test]
    fn rejects_network_allowed() {
        rejects(&variant("network denied", Some("network allowed")));
    }

    #[test]
    fn rejects_key_material_allowed() {
        rejects(&variant(
            "key_material denied",
            Some("key_material allowed"),
        ));
    }

    #[test]
    fn rejects_secret_material_allowed() {
        rejects(&variant(
            "secret_material denied",
            Some("secret_material allowed"),
        ));
    }

    #[test]
    fn rejects_execution_enabled() {
        rejects(&variant("execution disabled", Some("execution enabled")));
    }

    #[test]
    fn rejects_evidence_optional() {
        rejects(&variant("evidence required", Some("evidence optional")));
    }

    #[test]
    fn rejects_security_claims_present() {
        rejects(&variant(
            "security_claims none",
            Some("security_claims handshake_secure"),
        ));
    }

    #[test]
    fn rejects_empty_purpose() {
        rejects(&variant("purpose", Some("purpose []")));
    }

    #[test]
    fn rejects_duplicate_handshake() {
        let two = format!("{VALID}\n\n{VALID}");
        rejects(&two);
    }
}

#[cfg(test)]
mod trust_ledger_tests {
    use super::check_program;
    use argorix_parser::parse_source;

    const VALID: &str = include_str!("../../../examples/trust_ledger_v030.argx");

    #[test]
    fn parses_and_accepts_valid_trust_ledger() {
        let ast = parse_source(VALID).expect("parse");
        assert_eq!(ast.trust_ledgers.len(), 1);
        assert_eq!(ast.trust_ledgers[0].name.value, "ATrustLedger");
        check_program(&ast).expect("valid trust_ledger passes semantic check");
    }

    #[test]
    fn rejects_duplicate_trust_ledger_name() {
        // Append a second ledger block with the same name.
        let ledger = r#"
trust_ledger ATrustLedger {
  scope local
  mode dry_run
  hash_algorithm sha256
  chain_policy append_only
  entries [
    {
      id "dup-001"
      kind identity
      subject ResearchIdentity
      previous_hash "GENESIS"
      entry_hash "sha256:dup-001"
      evidence_ref "bundle:identity"
    }
  ]
  chain_root "sha256:dup-001"
  network denied
  key_material denied
  secret_material denied
  execution disabled
  evidence required
  security_claims none
  purpose ["trust-ledger"]
}
"#;
        let src = format!("{VALID}\n{ledger}");
        let ast = parse_source(&src).expect("parse");
        assert!(check_program(&ast).is_err(), "duplicate ledger must fail");
    }
}
