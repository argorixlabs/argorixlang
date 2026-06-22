use crate::symbols::{Symbols, COMMUNICATIVE_ACTS};
use argorix_parser::{
    ast::{
        ATrustCredentialMode, ATrustExecution, ATrustHandshakeMode, ATrustIdentityFormat,
        ATrustMaterialBoundary, ATrustResolutionMode,
        ATrustSecurityClaims, AdapterExecution, AdapterFilesystem, AdapterKind, AdapterMode,
        AdapterNetwork, AdapterProfileApiStyle, AdapterProfileAuth, AdapterProfileExecution,
        AdapterProfileFamily, AdapterProfileNetwork, AdapterProfileSecrets, AdapterSecrets,
        Approval, CapabilityLevel, CryptoKind, CryptoStatus, CryptoStrength, DidLedgerMode,
        DidMethodStatus, DidResolutionMode, FeatureDefault, FeatureStatus, HandlerInstruction,
        HarnessFilesystem, HarnessMode, HarnessNetwork, HarnessSecrets, PolicyRule, PolicyRuleDecl,
        PolicyViolationAction, Program, SecretAccess, SecretScope, SecretSource,
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
    symbols: &Symbols,
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
