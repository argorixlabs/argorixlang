use crate::{
    ast::{
        ATrustBoundaryDecl, ATrustCredentialContractDecl, ATrustCredentialMode,
        ATrustCredentialPresentation, ATrustCredentialStatus, ATrustCredentialVerification,
        ATrustEvidenceRequirement, ATrustExecution, ATrustHandshakeChallenge, ATrustHandshakeDecl,
        ATrustHandshakeDirection, ATrustHandshakeDryRunMode, ATrustHandshakeMode,
        ATrustHandshakeResponse, ATrustHandshakeTranscript, ATrustHandshakeVerification,
        ATrustIdentityDecl, ATrustIdentityFormat, ATrustIdentityStatus, ATrustIdentityValidation,
        ATrustMaterialBoundary, ATrustNetworkBoundary, ATrustPostQuantumRequirement,
        ATrustResolutionMode, ATrustSecurityClaims, AdapterDecl, AdapterExecution,
        AdapterFilesystem, AdapterKind, AdapterMode, AdapterNetwork, AdapterProfileApiStyle,
        AdapterProfileAuth, AdapterProfileDecl, AdapterProfileExecution, AdapterProfileFamily,
        AdapterProfileNetwork, AdapterProfileSecrets, AdapterSecrets, AgentDecl, Approval,
        AssertionDecl, CapabilityDecl, CapabilityLevel, CryptoBoundaryDecl, CryptoDecl, CryptoKind,
        CryptoStatus, CryptoStrength, DidLedgerMode, DidMethodDecl, DidMethodStatus,
        DidResolutionMode, EnumDecl, FailureDecl, FeatureDecl, FeatureDefault, FeatureStatus,
        FieldDecl, HandlerDecl, HandlerInstruction, HarnessFilesystem, HarnessMode, HarnessNetwork,
        HarnessSecrets, ImportDecl, MessageFieldType, ModelDecl, PassportAsnDecl, PassportDecl,
        PolicyDecl, PolicyRule, PolicyRuleDecl, PolicyViolationAction, PolicyViolationDecl,
        Program, ProtocolDecl, ProtocolStep, ProviderDecl, ProviderHarnessDecl, ProviderKindDecl,
        ReceiveDecl, SecretAccess, SecretDecl, SecretScope, SecretSource, SendDecl, ToolDecl,
        TrustLedgerChainPolicy, TrustLedgerDecl, TrustLedgerEntryDecl, TrustLedgerEntryKind,
        TrustLedgerMode, TrustLedgerScope, TypeDecl,
    },
    diagnostics::Diagnostic,
    lexer::{lex, Token, TokenKind},
    span::Spanned,
};

/// Validate a dotted module path such as `agents.research`.
///
/// Each segment must match `[a-zA-Z_][a-zA-Z0-9_]*`. Relative or aliased imports
/// are not supported in v0.16.
pub fn is_valid_module_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.split('.').all(|segment| {
        let mut chars = segment.chars();
        match chars.next() {
            Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
            _ => return false,
        }
        chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
    })
}

pub fn parse_source(source: &str) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lex(source)?;
    Parser::new(tokens).parse().map_err(|error| vec![error])
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    fn parse(mut self) -> Result<Program, Diagnostic> {
        self.expect_keyword("module")?;
        let module = self.expect_identifier("module name")?;
        if !is_valid_module_name(&module.value) {
            return Err(Diagnostic::new(
                format!(
                    "invalid module name `{}`; module names use dotted identifiers like `agents.research`",
                    module.value
                ),
                module.span,
            ));
        }

        let mut program = Program {
            module,
            imports: Vec::new(),
            providers: Vec::new(),
            harnesses: Vec::new(),
            features: Vec::new(),
            secrets: Vec::new(),
            adapters: Vec::new(),
            adapter_profiles: Vec::new(),
            cryptos: Vec::new(),
            crypto_boundaries: Vec::new(),
            did_methods: Vec::new(),
            atrust_boundaries: Vec::new(),
            atrust_identities: Vec::new(),
            atrust_credential_contracts: Vec::new(),
            atrust_handshakes: Vec::new(),
            trust_ledgers: Vec::new(),
            assertions: Vec::new(),
            policies: Vec::new(),
            failures: Vec::new(),
            capabilities: Vec::new(),
            enums: Vec::new(),
            types: Vec::new(),
            tools: Vec::new(),
            models: Vec::new(),
            agents: Vec::new(),
            protocols: Vec::new(),
            passports: Vec::new(),
        };

        while !self.is_eof() {
            match self.peek_identifier() {
                Some("import") => program.imports.push(self.parse_import()?),
                Some("provider") => program.providers.push(self.parse_provider()?),
                Some("harness") => program.harnesses.push(self.parse_harness()?),
                Some("feature") => program.features.push(self.parse_feature()?),
                Some("secret") => program.secrets.push(self.parse_secret()?),
                Some("adapter") => program.adapters.push(self.parse_adapter()?),
                Some("adapter_profile") => program.adapter_profiles.push(self.parse_adapter_profile()?),
                Some("crypto_boundary") => {
                    program.crypto_boundaries.push(self.parse_crypto_boundary()?)
                }
                Some("crypto") => program.cryptos.push(self.parse_crypto()?),
                Some("did_method") => program.did_methods.push(self.parse_did_method()?),
                Some("atrust_boundary") => program.atrust_boundaries.push(self.parse_atrust_boundary()?),
                Some("atrust_identity") => program.atrust_identities.push(self.parse_atrust_identity()?),
                Some("atrust_credential_contract") => program.atrust_credential_contracts.push(self.parse_atrust_credential_contract()?),
                Some("atrust_handshake") => program.atrust_handshakes.push(self.parse_atrust_handshake()?),
                Some("trust_ledger") => program.trust_ledgers.push(self.parse_trust_ledger()?),
                Some("capability") => program.capabilities.push(self.parse_capability()?),
                Some("assert") => program.assertions.push(self.parse_assertion()?),
                Some("policy") => program.policies.push(self.parse_policy()?),
                Some("failure") => program.failures.push(self.parse_failure()?),
                Some("enum") => program.enums.push(self.parse_enum()?),
                Some("type") => program.types.push(self.parse_type()?),
                Some("tool") => program.tools.push(self.parse_tool()?),
                Some("model") => program.models.push(self.parse_model()?),
                Some("agent") => program.agents.push(self.parse_agent()?),
                Some("protocol") => program.protocols.push(self.parse_protocol()?),
                Some("passport") => program.passports.push(self.parse_passport()?),
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected top-level declaration `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected `import`, `provider`, `harness`, `feature`, `secret`, `adapter`, `adapter_profile`, `crypto`, `did_method`, `atrust_boundary`, `atrust_identity`, `atrust_credential_contract`, `atrust_handshake`, `trust_ledger`, `assert`, `policy`, `failure`, `capability`, `enum`, `type`, `tool`, `model`, `agent`, `protocol`, or `passport`",
                        self.peek().span,
                    ))
                }
            }
        }

        Ok(program)
    }

    fn parse_import(&mut self) -> Result<ImportDecl, Diagnostic> {
        self.expect_keyword("import")?;
        let path = self.expect_identifier("imported module path")?;
        if !is_valid_module_name(&path.value) {
            return Err(Diagnostic::new(
                format!(
                    "invalid import path `{}`; relative imports are not supported in v0.16",
                    path.value
                ),
                path.span,
            ));
        }
        if self.peek_identifier() == Some("as") {
            return Err(Diagnostic::new(
                "import aliases are not supported in v0.16",
                self.peek().span,
            ));
        }
        Ok(ImportDecl { path })
    }

    fn parse_provider(&mut self) -> Result<ProviderDecl, Diagnostic> {
        self.expect_keyword("provider")?;
        let name = self.expect_identifier("provider name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        self.expect_keyword("kind")?;
        let kind_token = self.expect_identifier("provider kind")?;
        let kind = match kind_token.value.as_str() {
            "simulated" => ProviderKindDecl::Simulated,
            "external" => ProviderKindDecl::External,
            other => {
                return Err(Diagnostic::new(
                    format!("invalid provider kind `{other}`; expected `simulated` or `external`"),
                    kind_token.span,
                ))
            }
        };
        self.expect_keyword("enabled")?;
        let enabled = self.expect_bool("provider enabled value")?;
        self.expect_keyword("dry_run_only")?;
        let dry_run_only = self.expect_bool("provider dry_run_only value")?;
        let mut requires_feature_flag = false;
        let mut requires_explicit_approval = false;
        while self.match_keyword("requires") {
            let requirement = self.expect_identifier("provider requirement")?;
            match requirement.value.as_str() {
                "feature_flag" if !requires_feature_flag => {
                    if requires_explicit_approval {
                        return Err(Diagnostic::new(
                            "`requires feature_flag` must appear before `requires approval`",
                            requirement.span,
                        ));
                    }
                    requires_feature_flag = true;
                }
                "approval" if !requires_explicit_approval => {
                    requires_explicit_approval = true;
                }
                "feature_flag" | "approval" => {
                    return Err(Diagnostic::new(
                        format!("duplicate provider requirement `{}`", requirement.value),
                        requirement.span,
                    ))
                }
                other => {
                    return Err(Diagnostic::new(
                        format!(
                            "invalid provider requirement `{other}`; expected `feature_flag` or `approval`"
                        ),
                        requirement.span,
                    ))
                }
            }
        }
        let mut allowed_targets = None;
        let mut allowed_capabilities = None;
        while !self.check(&TokenKind::RightBrace) {
            match self.peek_identifier() {
                Some("allowed_targets") => {
                    if allowed_targets.is_some() {
                        return Err(Diagnostic::new(
                            "duplicate `allowed_targets` block",
                            self.peek().span,
                        ));
                    }
                    self.advance();
                    allowed_targets = Some(self.parse_identifier_block("allowed target")?);
                }
                Some("allowed_capabilities") => {
                    if allowed_capabilities.is_some() {
                        return Err(Diagnostic::new(
                            "duplicate `allowed_capabilities` block",
                            self.peek().span,
                        ));
                    }
                    self.advance();
                    allowed_capabilities = Some(self.parse_identifier_block("allowed capability")?);
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected provider item `{other}`"),
                        self.peek().span,
                    ));
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected provider allowlist block",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();
        Ok(ProviderDecl {
            name,
            kind: Spanned::new(kind, kind_token.span),
            enabled,
            dry_run_only,
            requires_feature_flag,
            requires_explicit_approval,
            allowed_targets: allowed_targets.unwrap_or_default(),
            allowed_capabilities: allowed_capabilities.unwrap_or_default(),
        })
    }

    fn parse_harness(&mut self) -> Result<ProviderHarnessDecl, Diagnostic> {
        self.expect_keyword("harness")?;
        let name = self.expect_identifier("harness name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut provider = None;
        let mut feature = None;
        let mut secret = None;
        let mut mode = None;
        let mut network = None;
        let mut secrets = None;
        let mut filesystem = None;
        let mut max_steps = None;
        let mut timeout_ms = None;
        let mut input_contract = None;
        let mut output_contract = None;
        let mut attestations = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated harness declaration")?;
            match self.peek_identifier() {
                Some("provider") => {
                    self.set_harness_field(&mut provider, "provider", |parser| {
                        parser.expect_identifier("harness provider reference")
                    })?
                }
                Some("feature") => self.set_harness_field(&mut feature, "feature", |parser| {
                    parser.expect_identifier("harness feature reference")
                })?,
                Some("secret") => self.set_harness_field(&mut secret, "secret", |parser| {
                    parser.expect_identifier("harness secret reference")
                })?,
                Some("mode") => self.set_harness_field(&mut mode, "mode", |parser| {
                    let token = parser.expect_identifier("harness mode")?;
                    let value = match token.value.as_str() {
                        "dry_run" => HarnessMode::DryRun,
                        "simulated" => HarnessMode::Simulated,
                        other => HarnessMode::Unknown(other.to_owned()),
                    };
                    Ok(Spanned::new(value, token.span))
                })?,
                Some("network") => self.set_harness_field(&mut network, "network", |parser| {
                    let token = parser.expect_identifier("harness network mode")?;
                    let value = match token.value.as_str() {
                        "denied" => HarnessNetwork::Denied,
                        other => HarnessNetwork::Unknown(other.to_owned()),
                    };
                    Ok(Spanned::new(value, token.span))
                })?,
                Some("secrets") => self.set_harness_field(&mut secrets, "secrets", |parser| {
                    let token = parser.expect_identifier("harness secrets mode")?;
                    let value = match token.value.as_str() {
                        "denied" => HarnessSecrets::Denied,
                        other => HarnessSecrets::Unknown(other.to_owned()),
                    };
                    Ok(Spanned::new(value, token.span))
                })?,
                Some("filesystem") => {
                    self.set_harness_field(&mut filesystem, "filesystem", |parser| {
                        let token = parser.expect_identifier("harness filesystem mode")?;
                        let value = match token.value.as_str() {
                            "none" => HarnessFilesystem::None,
                            "read_only" => HarnessFilesystem::ReadOnly,
                            other => HarnessFilesystem::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("max_steps") => {
                    self.set_harness_field(&mut max_steps, "max_steps", |parser| {
                        parser.expect_integer("harness max_steps integer")
                    })?
                }
                Some("timeout_ms") => {
                    self.set_harness_field(&mut timeout_ms, "timeout_ms", |parser| {
                        parser.expect_integer("harness timeout_ms integer")
                    })?
                }
                Some("input_contract") => {
                    self.set_harness_field(&mut input_contract, "input_contract", |parser| {
                        parser.expect_identifier("harness input contract")
                    })?
                }
                Some("output_contract") => {
                    self.set_harness_field(&mut output_contract, "output_contract", |parser| {
                        parser.expect_identifier("harness output contract")
                    })?
                }
                Some("attestations") => {
                    self.set_harness_field(&mut attestations, "attestations", |parser| {
                        parser.parse_string_array("harness attestation")
                    })?
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected harness item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected a harness field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        Ok(ProviderHarnessDecl {
            name,
            provider: provider.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            feature,
            secret,
            mode: mode.unwrap_or_else(|| {
                Spanned::new(HarnessMode::Unknown(String::new()), fallback_span)
            }),
            network: network.unwrap_or_else(|| {
                Spanned::new(HarnessNetwork::Unknown(String::new()), fallback_span)
            }),
            secrets: secrets.unwrap_or_else(|| {
                Spanned::new(HarnessSecrets::Unknown(String::new()), fallback_span)
            }),
            filesystem: filesystem.unwrap_or_else(|| {
                Spanned::new(HarnessFilesystem::Unknown(String::new()), fallback_span)
            }),
            max_steps,
            timeout_ms,
            input_contract,
            output_contract,
            attestations: attestations.unwrap_or_default(),
        })
    }

    fn set_harness_field<T>(
        &mut self,
        slot: &mut Option<T>,
        key: &str,
        parse_value: impl FnOnce(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<(), Diagnostic> {
        let span = self.peek().span;
        self.advance();
        if slot.is_some() {
            return Err(Diagnostic::new(
                format!("duplicate harness field `{key}`"),
                span,
            ));
        }
        *slot = Some(parse_value(self)?);
        Ok(())
    }

    fn parse_feature(&mut self) -> Result<FeatureDecl, Diagnostic> {
        self.expect_keyword("feature")?;
        let name = self.expect_identifier("feature name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut provider = None;
        let mut status = None;
        let mut default = None;
        let mut purpose = None;
        let mut requires_approval = false;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated feature declaration")?;
            match self.peek_identifier() {
                Some("provider") => {
                    self.set_block_field(&mut provider, "feature", "provider", |parser| {
                        parser.expect_identifier("feature provider reference")
                    })?
                }
                Some("status") => {
                    self.set_block_field(&mut status, "feature", "status", |parser| {
                        let token = parser.expect_identifier("feature status")?;
                        let value = match token.value.as_str() {
                            "experimental" => FeatureStatus::Experimental,
                            "preview" => FeatureStatus::Preview,
                            "stable" => FeatureStatus::Stable,
                            "deprecated" => FeatureStatus::Deprecated,
                            other => FeatureStatus::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("default") => {
                    self.set_block_field(&mut default, "feature", "default", |parser| {
                        let token = parser.expect_identifier("feature default")?;
                        let value = match token.value.as_str() {
                            "disabled" => FeatureDefault::Disabled,
                            "enabled" => FeatureDefault::Enabled,
                            other => FeatureDefault::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("requires") => {
                    let span = self.peek().span;
                    self.advance();
                    self.expect_keyword("approval")?;
                    if requires_approval {
                        return Err(Diagnostic::new(
                            "duplicate feature field `requires approval`",
                            span,
                        ));
                    }
                    requires_approval = true;
                }
                Some("purpose") => {
                    self.set_block_field(&mut purpose, "feature", "purpose", |parser| {
                        parser.expect_string("feature purpose value")
                    })?
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected feature item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected a feature field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        Ok(FeatureDecl {
            name,
            provider,
            status: status.unwrap_or_else(|| {
                Spanned::new(FeatureStatus::Unknown(String::new()), fallback_span)
            }),
            default: default.unwrap_or_else(|| {
                Spanned::new(FeatureDefault::Unknown(String::new()), fallback_span)
            }),
            requires_approval,
            purpose,
        })
    }

    fn parse_secret(&mut self) -> Result<SecretDecl, Diagnostic> {
        self.expect_keyword("secret")?;
        let name = self.expect_identifier("secret name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        const FORBIDDEN: [&str; 6] = [
            "value",
            "secret_value",
            "token",
            "api_key_value",
            "raw",
            "plaintext",
        ];

        let mut handle = None;
        let mut provider = None;
        let mut required_by = None;
        let mut scope = None;
        let mut access = None;
        let mut source = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated secret declaration")?;
            match self.peek_identifier() {
                Some("handle") => {
                    self.set_block_field(&mut handle, "secret", "handle", |parser| {
                        parser.expect_string("secret handle value")
                    })?
                }
                Some("provider") => {
                    self.set_block_field(&mut provider, "secret", "provider", |parser| {
                        parser.expect_identifier("secret provider reference")
                    })?
                }
                Some("required_by") => {
                    self.set_block_field(&mut required_by, "secret", "required_by", |parser| {
                        parser.expect_identifier("secret required_by reference")
                    })?
                }
                Some("scope") => {
                    self.set_block_field(&mut scope, "secret", "scope", |parser| {
                        let token = parser.expect_identifier("secret scope")?;
                        let value = match token.value.as_str() {
                            "provider" => SecretScope::Provider,
                            "adapter" => SecretScope::Adapter,
                            "model" => SecretScope::Model,
                            "tool" => SecretScope::Tool,
                            "runtime" => SecretScope::Runtime,
                            other => SecretScope::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("access") => {
                    self.set_block_field(&mut access, "secret", "access", |parser| {
                        let token = parser.expect_identifier("secret access")?;
                        let value = match token.value.as_str() {
                            "denied" => SecretAccess::Denied,
                            other => SecretAccess::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("source") => {
                    self.set_block_field(&mut source, "secret", "source", |parser| {
                        let token = parser.expect_identifier("secret source")?;
                        let value = match token.value.as_str() {
                            "none" => SecretSource::None,
                            other => SecretSource::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some(other) if FORBIDDEN.contains(&other) => {
                    return Err(Diagnostic::new(
                        format!(
                            "secret declarations must not contain secret material; field `{other}` is forbidden"
                        ),
                        self.peek().span,
                    ))
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected secret item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => return Err(Diagnostic::new("expected a secret field", self.peek().span)),
            }
        }
        self.advance();

        let fallback_span = name.span;
        Ok(SecretDecl {
            name,
            handle: handle.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            provider,
            required_by,
            scope: scope.unwrap_or_else(|| {
                Spanned::new(SecretScope::Unknown(String::new()), fallback_span)
            }),
            access: access.unwrap_or_else(|| {
                Spanned::new(SecretAccess::Unknown(String::new()), fallback_span)
            }),
            source: source.unwrap_or_else(|| {
                Spanned::new(SecretSource::Unknown(String::new()), fallback_span)
            }),
        })
    }

    fn parse_adapter(&mut self) -> Result<AdapterDecl, Diagnostic> {
        self.expect_keyword("adapter")?;
        let name = self.expect_identifier("adapter name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut provider = None;
        let mut feature = None;
        let mut secret = None;
        let mut harness = None;
        let mut kind = None;
        let mut vendor = None;
        let mut mode = None;
        let mut execution = None;
        let mut network = None;
        let mut secrets = None;
        let mut filesystem = None;
        let mut input_contract = None;
        let mut output_contract = None;
        let mut conformance: Option<Vec<Spanned<String>>> = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated adapter declaration")?;
            match self.peek_identifier() {
                Some("provider") => {
                    self.set_block_field(&mut provider, "adapter", "provider", |parser| {
                        parser.expect_identifier("adapter provider reference")
                    })?
                }
                Some("feature") => {
                    self.set_block_field(&mut feature, "adapter", "feature", |parser| {
                        parser.expect_identifier("adapter feature reference")
                    })?
                }
                Some("secret") => {
                    self.set_block_field(&mut secret, "adapter", "secret", |parser| {
                        parser.expect_identifier("adapter secret reference")
                    })?
                }
                Some("harness") => {
                    self.set_block_field(&mut harness, "adapter", "harness", |parser| {
                        parser.expect_identifier("adapter harness reference")
                    })?
                }
                Some("kind") => self.set_block_field(&mut kind, "adapter", "kind", |parser| {
                    let token = parser.expect_identifier("adapter kind")?;
                    let value = match token.value.as_str() {
                        "llm" => AdapterKind::Llm,
                        "tool" => AdapterKind::Tool,
                        "bridge" => AdapterKind::Bridge,
                        "registry" => AdapterKind::Registry,
                        "identity" => AdapterKind::Identity,
                        "payment" => AdapterKind::Payment,
                        "storage" => AdapterKind::Storage,
                        "custom" => AdapterKind::Custom,
                        other => AdapterKind::Unknown(other.to_owned()),
                    };
                    Ok(Spanned::new(value, token.span))
                })?,
                Some("vendor") => {
                    self.set_block_field(&mut vendor, "adapter", "vendor", |parser| {
                        parser.expect_string("adapter vendor")
                    })?
                }
                Some("mode") => self.set_block_field(&mut mode, "adapter", "mode", |parser| {
                    let token = parser.expect_identifier("adapter mode")?;
                    let value = match token.value.as_str() {
                        "experimental" => AdapterMode::Experimental,
                        "preview" => AdapterMode::Preview,
                        "stable" => AdapterMode::Stable,
                        "deprecated" => AdapterMode::Deprecated,
                        other => AdapterMode::Unknown(other.to_owned()),
                    };
                    Ok(Spanned::new(value, token.span))
                })?,
                Some("execution") => {
                    self.set_block_field(&mut execution, "adapter", "execution", |parser| {
                        let token = parser.expect_identifier("adapter execution")?;
                        let value = match token.value.as_str() {
                            "disabled" => AdapterExecution::Disabled,
                            other => AdapterExecution::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("network") => {
                    self.set_block_field(&mut network, "adapter", "network", |parser| {
                        let token = parser.expect_identifier("adapter network")?;
                        let value = match token.value.as_str() {
                            "denied" => AdapterNetwork::Denied,
                            other => AdapterNetwork::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("secrets") => {
                    self.set_block_field(&mut secrets, "adapter", "secrets", |parser| {
                        let token = parser.expect_identifier("adapter secrets")?;
                        let value = match token.value.as_str() {
                            "denied" => AdapterSecrets::Denied,
                            other => AdapterSecrets::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("filesystem") => {
                    self.set_block_field(&mut filesystem, "adapter", "filesystem", |parser| {
                        let token = parser.expect_identifier("adapter filesystem")?;
                        let value = match token.value.as_str() {
                            "none" => AdapterFilesystem::None,
                            "read_only" => AdapterFilesystem::ReadOnly,
                            other => AdapterFilesystem::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("input_contract") => self.set_block_field(
                    &mut input_contract,
                    "adapter",
                    "input_contract",
                    |parser| parser.expect_identifier("adapter input contract"),
                )?,
                Some("output_contract") => self.set_block_field(
                    &mut output_contract,
                    "adapter",
                    "output_contract",
                    |parser| parser.expect_identifier("adapter output contract"),
                )?,
                Some("conformance") => {
                    self.set_block_field(&mut conformance, "adapter", "conformance", |parser| {
                        parser.parse_string_array("adapter conformance item")
                    })?
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected adapter item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected an adapter field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        Ok(AdapterDecl {
            name,
            provider: provider.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            feature,
            secret,
            harness,
            kind,
            vendor,
            mode: mode.unwrap_or_else(|| {
                Spanned::new(AdapterMode::Unknown(String::new()), fallback_span)
            }),
            execution: execution.unwrap_or_else(|| {
                Spanned::new(AdapterExecution::Unknown(String::new()), fallback_span)
            }),
            network: network.unwrap_or_else(|| {
                Spanned::new(AdapterNetwork::Unknown(String::new()), fallback_span)
            }),
            secrets: secrets.unwrap_or_else(|| {
                Spanned::new(AdapterSecrets::Unknown(String::new()), fallback_span)
            }),
            filesystem: filesystem.unwrap_or_else(|| {
                Spanned::new(AdapterFilesystem::Unknown(String::new()), fallback_span)
            }),
            input_contract,
            output_contract,
            conformance: conformance.unwrap_or_default(),
        })
    }

    fn parse_adapter_profile(&mut self) -> Result<AdapterProfileDecl, Diagnostic> {
        self.expect_keyword("adapter_profile")?;
        let name = self.expect_identifier("adapter profile name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut adapter = None;
        let mut provider = None;
        let mut vendor = None;
        let mut family = None;
        let mut api_style = None;
        let mut auth = None;
        let mut execution = None;
        let mut network = None;
        let mut secrets = None;
        let mut request_contract = None;
        let mut response_contract = None;
        let mut capabilities: Option<Vec<Spanned<String>>> = None;
        let mut required_conformance: Option<Vec<Spanned<String>>> = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated adapter_profile declaration")?;
            match self.peek_identifier() {
                Some("adapter") => {
                    self.set_block_field(&mut adapter, "adapter_profile", "adapter", |parser| {
                        parser.expect_identifier("adapter reference")
                    })?
                }
                Some("provider") => {
                    self.set_block_field(&mut provider, "adapter_profile", "provider", |parser| {
                        parser.expect_identifier("provider reference")
                    })?
                }
                Some("vendor") => {
                    self.set_block_field(&mut vendor, "adapter_profile", "vendor", |parser| {
                        parser.expect_string("vendor name")
                    })?
                }
                Some("family") => {
                    self.set_block_field(&mut family, "adapter_profile", "family", |parser| {
                        let token = parser.expect_identifier("profile family")?;
                        let value = match token.value.as_str() {
                            "llm" => AdapterProfileFamily::Llm,
                            "tool" => AdapterProfileFamily::Tool,
                            "bridge" => AdapterProfileFamily::Bridge,
                            "registry" => AdapterProfileFamily::Registry,
                            "identity" => AdapterProfileFamily::Identity,
                            "payment" => AdapterProfileFamily::Payment,
                            "storage" => AdapterProfileFamily::Storage,
                            "custom" => AdapterProfileFamily::Custom,
                            other => AdapterProfileFamily::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("api_style") => self.set_block_field(
                    &mut api_style,
                    "adapter_profile",
                    "api_style",
                    |parser| {
                        let token = parser.expect_identifier("profile api_style")?;
                        let value = match token.value.as_str() {
                            "responses" => AdapterProfileApiStyle::Responses,
                            "messages" => AdapterProfileApiStyle::Messages,
                            "chat" => AdapterProfileApiStyle::Chat,
                            "completion" => AdapterProfileApiStyle::Completion,
                            "tool_call" => AdapterProfileApiStyle::ToolCall,
                            "mcp" => AdapterProfileApiStyle::Mcp,
                            "a2a" => AdapterProfileApiStyle::A2a,
                            "rest" => AdapterProfileApiStyle::Rest,
                            "custom" => AdapterProfileApiStyle::Custom,
                            other => AdapterProfileApiStyle::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("auth") => {
                    self.set_block_field(&mut auth, "adapter_profile", "auth", |parser| {
                        let token = parser.expect_identifier("profile auth")?;
                        let value = match token.value.as_str() {
                            "none" => AdapterProfileAuth::None,
                            "secret_boundary" => AdapterProfileAuth::SecretBoundary,
                            "did" => AdapterProfileAuth::Did,
                            "credential" => AdapterProfileAuth::Credential,
                            "custom" => AdapterProfileAuth::Custom,
                            other => AdapterProfileAuth::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("execution") => self.set_block_field(
                    &mut execution,
                    "adapter_profile",
                    "execution",
                    |parser| {
                        let token = parser.expect_identifier("profile execution")?;
                        let value = match token.value.as_str() {
                            "disabled" => AdapterProfileExecution::Disabled,
                            other => AdapterProfileExecution::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("network") => {
                    self.set_block_field(&mut network, "adapter_profile", "network", |parser| {
                        let token = parser.expect_identifier("profile network")?;
                        let value = match token.value.as_str() {
                            "denied" => AdapterProfileNetwork::Denied,
                            other => AdapterProfileNetwork::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("secrets") => {
                    self.set_block_field(&mut secrets, "adapter_profile", "secrets", |parser| {
                        let token = parser.expect_identifier("profile secrets")?;
                        let value = match token.value.as_str() {
                            "denied" => AdapterProfileSecrets::Denied,
                            other => AdapterProfileSecrets::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("request_contract") => self.set_block_field(
                    &mut request_contract,
                    "adapter_profile",
                    "request_contract",
                    |parser| parser.expect_identifier("request contract type"),
                )?,
                Some("response_contract") => self.set_block_field(
                    &mut response_contract,
                    "adapter_profile",
                    "response_contract",
                    |parser| parser.expect_identifier("response contract type"),
                )?,
                Some("capabilities") => self.set_block_field(
                    &mut capabilities,
                    "adapter_profile",
                    "capabilities",
                    |parser| parser.parse_string_array("profile capability"),
                )?,
                Some("required_conformance") => self.set_block_field(
                    &mut required_conformance,
                    "adapter_profile",
                    "required_conformance",
                    |parser| parser.parse_string_array("profile required conformance item"),
                )?,
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected adapter_profile item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected an adapter_profile field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        Ok(AdapterProfileDecl {
            name,
            adapter: adapter.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            provider: provider.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            vendor: vendor.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            family: family.unwrap_or_else(|| {
                Spanned::new(AdapterProfileFamily::Unknown(String::new()), fallback_span)
            }),
            api_style: api_style.unwrap_or_else(|| {
                Spanned::new(
                    AdapterProfileApiStyle::Unknown(String::new()),
                    fallback_span,
                )
            }),
            auth: auth.unwrap_or_else(|| {
                Spanned::new(AdapterProfileAuth::Unknown(String::new()), fallback_span)
            }),
            execution: execution.unwrap_or_else(|| {
                Spanned::new(
                    AdapterProfileExecution::Unknown(String::new()),
                    fallback_span,
                )
            }),
            network: network.unwrap_or_else(|| {
                Spanned::new(AdapterProfileNetwork::Unknown(String::new()), fallback_span)
            }),
            secrets: secrets.unwrap_or_else(|| {
                Spanned::new(AdapterProfileSecrets::Unknown(String::new()), fallback_span)
            }),
            request_contract,
            response_contract,
            capabilities: capabilities.unwrap_or_default(),
            required_conformance: required_conformance.unwrap_or_default(),
        })
    }

    fn parse_crypto(&mut self) -> Result<CryptoDecl, Diagnostic> {
        self.expect_keyword("crypto")?;
        let name = self.expect_identifier("crypto primitive name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut kind = None;
        let mut status = None;
        let mut strength = None;
        let mut purpose: Option<Vec<Spanned<String>>> = None;
        let mut output_bits = None;
        let mut min_key_bits = None;
        let mut security_level = None;
        let mut notes = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated crypto declaration")?;
            match self.peek_identifier() {
                Some("kind") => self.set_block_field(&mut kind, "crypto", "kind", |parser| {
                    let token = parser.expect_identifier("crypto kind")?;
                    let value = match token.value.as_str() {
                        "hash" => CryptoKind::Hash,
                        "signature" => CryptoKind::Signature,
                        "kem" => CryptoKind::Kem,
                        "aead" => CryptoKind::Aead,
                        "mac" => CryptoKind::Mac,
                        "kdf" => CryptoKind::Kdf,
                        "commitment" => CryptoKind::Commitment,
                        "randomness" => CryptoKind::Randomness,
                        "custom" => CryptoKind::Custom,
                        other => CryptoKind::Unknown(other.to_owned()),
                    };
                    Ok(Spanned::new(value, token.span))
                })?,
                Some("status") => {
                    self.set_block_field(&mut status, "crypto", "status", |parser| {
                        let token = parser.expect_identifier("crypto status")?;
                        let value = match token.value.as_str() {
                            "allowed" => CryptoStatus::Allowed,
                            "legacy" => CryptoStatus::Legacy,
                            "deprecated" => CryptoStatus::Deprecated,
                            "denied" => CryptoStatus::Denied,
                            "experimental" => CryptoStatus::Experimental,
                            "post_quantum_candidate" => CryptoStatus::PostQuantumCandidate,
                            other => CryptoStatus::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("strength") => {
                    self.set_block_field(&mut strength, "crypto", "strength", |parser| {
                        let token = parser.expect_identifier("crypto strength")?;
                        let value = match token.value.as_str() {
                            "classical" => CryptoStrength::Classical,
                            "post_quantum" => CryptoStrength::PostQuantum,
                            "hybrid" => CryptoStrength::Hybrid,
                            "unknown" => CryptoStrength::Unknown,
                            other => CryptoStrength::UnknownValue(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("purpose") => {
                    self.set_block_field(&mut purpose, "crypto", "purpose", |parser| {
                        parser.parse_string_array("crypto purpose")
                    })?
                }
                Some("output_bits") => {
                    self.set_block_field(&mut output_bits, "crypto", "output_bits", |parser| {
                        parser.expect_integer("crypto output_bits")
                    })?
                }
                Some("min_key_bits") => {
                    self.set_block_field(&mut min_key_bits, "crypto", "min_key_bits", |parser| {
                        parser.expect_integer("crypto min_key_bits")
                    })?
                }
                Some("security_level") => self.set_block_field(
                    &mut security_level,
                    "crypto",
                    "security_level",
                    |parser| parser.expect_string("crypto security_level"),
                )?,
                Some("notes") => self.set_block_field(&mut notes, "crypto", "notes", |parser| {
                    parser.expect_string("crypto notes")
                })?,
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected crypto item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => return Err(Diagnostic::new("expected a crypto field", self.peek().span)),
            }
        }
        self.advance();

        let fallback_span = name.span;
        Ok(CryptoDecl {
            name,
            kind: kind
                .unwrap_or_else(|| Spanned::new(CryptoKind::Unknown(String::new()), fallback_span)),
            status: status.unwrap_or_else(|| {
                Spanned::new(CryptoStatus::Unknown(String::new()), fallback_span)
            }),
            strength: strength.unwrap_or_else(|| {
                Spanned::new(CryptoStrength::UnknownValue(String::new()), fallback_span)
            }),
            purpose: purpose.unwrap_or_default(),
            output_bits,
            min_key_bits,
            security_level,
            notes,
        })
    }

    fn parse_crypto_boundary(&mut self) -> Result<CryptoBoundaryDecl, Diagnostic> {
        self.expect_keyword("crypto_boundary")?;
        let name = self.expect_identifier("crypto boundary name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut allowed_hashes: Option<Vec<Spanned<String>>> = None;
        let mut allowed_signatures: Option<Vec<Spanned<String>>> = None;
        let mut allowed_kems: Option<Vec<Spanned<String>>> = None;
        let mut allowed_aeads: Option<Vec<Spanned<String>>> = None;
        let mut legacy_allowed: Option<Vec<Spanned<String>>> = None;
        let mut denied: Option<Vec<Spanned<String>>> = None;
        let mut purpose: Option<Vec<Spanned<String>>> = None;
        let mut min_hash_bits = None;
        let mut post_quantum_ready = None;
        let mut hybrid_allowed = None;
        let mut key_material = None;
        let mut secret_material = None;
        let mut execution = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated crypto_boundary declaration")?;
            match self.peek_identifier() {
                Some("allowed_hashes") => self.set_block_field(
                    &mut allowed_hashes,
                    "crypto_boundary",
                    "allowed_hashes",
                    |parser| parser.parse_string_array("crypto_boundary allowed_hashes"),
                )?,
                Some("allowed_signatures") => self.set_block_field(
                    &mut allowed_signatures,
                    "crypto_boundary",
                    "allowed_signatures",
                    |parser| parser.parse_string_array("crypto_boundary allowed_signatures"),
                )?,
                Some("allowed_kems") => self.set_block_field(
                    &mut allowed_kems,
                    "crypto_boundary",
                    "allowed_kems",
                    |parser| parser.parse_string_array("crypto_boundary allowed_kems"),
                )?,
                Some("allowed_aeads") => self.set_block_field(
                    &mut allowed_aeads,
                    "crypto_boundary",
                    "allowed_aeads",
                    |parser| parser.parse_string_array("crypto_boundary allowed_aeads"),
                )?,
                Some("legacy_allowed") => self.set_block_field(
                    &mut legacy_allowed,
                    "crypto_boundary",
                    "legacy_allowed",
                    |parser| parser.parse_string_array("crypto_boundary legacy_allowed"),
                )?,
                Some("denied") => {
                    self.set_block_field(&mut denied, "crypto_boundary", "denied", |parser| {
                        parser.parse_string_array("crypto_boundary denied")
                    })?
                }
                Some("purpose") => {
                    self.set_block_field(&mut purpose, "crypto_boundary", "purpose", |parser| {
                        parser.parse_string_array("crypto_boundary purpose")
                    })?
                }
                Some("min_hash_bits") => self.set_block_field(
                    &mut min_hash_bits,
                    "crypto_boundary",
                    "min_hash_bits",
                    |parser| parser.expect_integer("crypto_boundary min_hash_bits"),
                )?,
                Some("post_quantum_ready") => self.set_block_field(
                    &mut post_quantum_ready,
                    "crypto_boundary",
                    "post_quantum_ready",
                    |parser| parser.expect_bool("crypto_boundary post_quantum_ready"),
                )?,
                Some("hybrid_allowed") => self.set_block_field(
                    &mut hybrid_allowed,
                    "crypto_boundary",
                    "hybrid_allowed",
                    |parser| parser.expect_bool("crypto_boundary hybrid_allowed"),
                )?,
                Some("key_material") => self.set_block_field(
                    &mut key_material,
                    "crypto_boundary",
                    "key_material",
                    |parser| parser.expect_identifier("crypto_boundary key_material"),
                )?,
                Some("secret_material") => self.set_block_field(
                    &mut secret_material,
                    "crypto_boundary",
                    "secret_material",
                    |parser| parser.expect_identifier("crypto_boundary secret_material"),
                )?,
                Some("execution") => self.set_block_field(
                    &mut execution,
                    "crypto_boundary",
                    "execution",
                    |parser| parser.expect_identifier("crypto_boundary execution"),
                )?,
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected crypto_boundary item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected a crypto_boundary field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        let default_disposition = |span| Spanned::new("denied".to_owned(), span);
        Ok(CryptoBoundaryDecl {
            name,
            allowed_hashes: allowed_hashes.unwrap_or_default(),
            allowed_signatures: allowed_signatures.unwrap_or_default(),
            allowed_kems: allowed_kems.unwrap_or_default(),
            allowed_aeads: allowed_aeads.unwrap_or_default(),
            legacy_allowed: legacy_allowed.unwrap_or_default(),
            denied: denied.unwrap_or_default(),
            purpose: purpose.unwrap_or_default(),
            min_hash_bits,
            post_quantum_ready,
            hybrid_allowed,
            key_material: key_material.unwrap_or_else(|| default_disposition(fallback_span)),
            secret_material: secret_material.unwrap_or_else(|| default_disposition(fallback_span)),
            execution: execution
                .unwrap_or_else(|| Spanned::new("disabled".to_owned(), fallback_span)),
        })
    }

    fn parse_did_method(&mut self) -> Result<DidMethodDecl, Diagnostic> {
        self.expect_keyword("did_method")?;
        let name = self.expect_identifier("did method name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut status = None;
        let mut resolution = None;
        let mut ledger = None;
        let mut crypto_boundary = None;
        let mut governance = None;
        let mut purpose: Option<Vec<Spanned<String>>> = None;
        let mut notes = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated did_method declaration")?;
            match self.peek_identifier() {
                Some("status") => {
                    self.set_block_field(&mut status, "did_method", "status", |parser| {
                        let token = parser.expect_identifier("did_method status")?;
                        let value = match token.value.as_str() {
                            "experimental" => DidMethodStatus::Experimental,
                            "preview" => DidMethodStatus::Preview,
                            "stable" => DidMethodStatus::Stable,
                            "deprecated" => DidMethodStatus::Deprecated,
                            "denied" => DidMethodStatus::Denied,
                            other => DidMethodStatus::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("resolution") => {
                    self.set_block_field(&mut resolution, "did_method", "resolution", |parser| {
                        let token = parser.expect_identifier("did_method resolution")?;
                        let value = match token.value.as_str() {
                            "disabled" => DidResolutionMode::Disabled,
                            "embedded" => DidResolutionMode::Embedded,
                            "local" => DidResolutionMode::Local,
                            other => DidResolutionMode::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("ledger") => {
                    self.set_block_field(&mut ledger, "did_method", "ledger", |parser| {
                        let token = parser.expect_identifier("did_method ledger")?;
                        let value = match token.value.as_str() {
                            "none" => DidLedgerMode::None,
                            "local" => DidLedgerMode::Local,
                            "embedded" => DidLedgerMode::Embedded,
                            "custom" => DidLedgerMode::Custom,
                            other => DidLedgerMode::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("crypto_boundary") => self.set_block_field(
                    &mut crypto_boundary,
                    "did_method",
                    "crypto_boundary",
                    |parser| parser.expect_identifier("did_method crypto_boundary reference"),
                )?,
                Some("governance") => {
                    self.set_block_field(&mut governance, "did_method", "governance", |parser| {
                        parser.expect_string("did_method governance")
                    })?
                }
                Some("purpose") => {
                    self.set_block_field(&mut purpose, "did_method", "purpose", |parser| {
                        parser.parse_string_array("did_method purpose")
                    })?
                }
                Some("notes") => {
                    self.set_block_field(&mut notes, "did_method", "notes", |parser| {
                        parser.expect_string("did_method notes")
                    })?
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected did_method item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected a did_method field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        let unknown_status =
            || Spanned::new(DidMethodStatus::Unknown(String::new()), fallback_span);
        let unknown_resolution =
            || Spanned::new(DidResolutionMode::Unknown(String::new()), fallback_span);
        let unknown_ledger = || Spanned::new(DidLedgerMode::Unknown(String::new()), fallback_span);
        Ok(DidMethodDecl {
            name,
            status: status.unwrap_or_else(unknown_status),
            resolution: resolution.unwrap_or_else(unknown_resolution),
            ledger: ledger.unwrap_or_else(unknown_ledger),
            crypto_boundary: crypto_boundary
                .unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            governance,
            purpose: purpose.unwrap_or_default(),
            notes,
        })
    }

    fn parse_atrust_boundary(&mut self) -> Result<ATrustBoundaryDecl, Diagnostic> {
        self.expect_keyword("atrust_boundary")?;
        let name = self.expect_identifier("atrust boundary name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut crypto_boundary = None;
        let mut did_methods: Option<Vec<Spanned<String>>> = None;
        let mut identity_format = None;
        let mut credential_mode = None;
        let mut handshake = None;
        let mut resolution = None;
        let mut key_material = None;
        let mut secret_material = None;
        let mut execution = None;
        let mut post_quantum_ready = None;
        let mut security_claims = None;
        let mut purpose: Option<Vec<Spanned<String>>> = None;
        let mut notes = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated atrust_boundary declaration")?;
            match self.peek_identifier() {
                Some("crypto_boundary") => self.set_block_field(
                    &mut crypto_boundary,
                    "atrust_boundary",
                    "crypto_boundary",
                    |parser| parser.expect_identifier("atrust_boundary crypto_boundary reference"),
                )?,
                Some("did_methods") => self.set_block_field(
                    &mut did_methods,
                    "atrust_boundary",
                    "did_methods",
                    |parser| parser.parse_string_array("atrust_boundary did_methods"),
                )?,
                Some("identity_format") => self.set_block_field(
                    &mut identity_format,
                    "atrust_boundary",
                    "identity_format",
                    |parser| {
                        let token = parser.expect_identifier("atrust_boundary identity_format")?;
                        let value = match token.value.as_str() {
                            "did" => ATrustIdentityFormat::Did,
                            "opaque" => ATrustIdentityFormat::Opaque,
                            "custom" => ATrustIdentityFormat::Custom,
                            other => ATrustIdentityFormat::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("credential_mode") => self.set_block_field(
                    &mut credential_mode,
                    "atrust_boundary",
                    "credential_mode",
                    |parser| {
                        let token = parser.expect_identifier("atrust_boundary credential_mode")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustCredentialMode::Disabled,
                            "declared_only" => ATrustCredentialMode::DeclaredOnly,
                            other => ATrustCredentialMode::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("handshake") => self.set_block_field(
                    &mut handshake,
                    "atrust_boundary",
                    "handshake",
                    |parser| {
                        let token = parser.expect_identifier("atrust_boundary handshake")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustHandshakeMode::Disabled,
                            "declared_only" => ATrustHandshakeMode::DeclaredOnly,
                            other => ATrustHandshakeMode::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("resolution") => self.set_block_field(
                    &mut resolution,
                    "atrust_boundary",
                    "resolution",
                    |parser| {
                        let token = parser.expect_identifier("atrust_boundary resolution")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustResolutionMode::Disabled,
                            "embedded" => ATrustResolutionMode::Embedded,
                            "local" => ATrustResolutionMode::Local,
                            other => ATrustResolutionMode::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("key_material") => self.set_block_field(
                    &mut key_material,
                    "atrust_boundary",
                    "key_material",
                    |parser| {
                        let token = parser.expect_identifier("atrust_boundary key_material")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustMaterialBoundary::Denied,
                            other => ATrustMaterialBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("secret_material") => self.set_block_field(
                    &mut secret_material,
                    "atrust_boundary",
                    "secret_material",
                    |parser| {
                        let token = parser.expect_identifier("atrust_boundary secret_material")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustMaterialBoundary::Denied,
                            other => ATrustMaterialBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("execution") => self.set_block_field(
                    &mut execution,
                    "atrust_boundary",
                    "execution",
                    |parser| {
                        let token = parser.expect_identifier("atrust_boundary execution")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustExecution::Disabled,
                            other => ATrustExecution::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("post_quantum_ready") => self.set_block_field(
                    &mut post_quantum_ready,
                    "atrust_boundary",
                    "post_quantum_ready",
                    |parser| {
                        let token =
                            parser.expect_identifier("atrust_boundary post_quantum_ready")?;
                        let value = match token.value.as_str() {
                            "required" => ATrustPostQuantumRequirement::Required,
                            "optional" => ATrustPostQuantumRequirement::Optional,
                            "not_required" => ATrustPostQuantumRequirement::NotRequired,
                            other => ATrustPostQuantumRequirement::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("security_claims") => self.set_block_field(
                    &mut security_claims,
                    "atrust_boundary",
                    "security_claims",
                    |parser| {
                        let token = parser.expect_identifier("atrust_boundary security_claims")?;
                        let value = match token.value.as_str() {
                            "none" => ATrustSecurityClaims::None,
                            other => ATrustSecurityClaims::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("purpose") => {
                    self.set_block_field(&mut purpose, "atrust_boundary", "purpose", |parser| {
                        parser.parse_string_array("atrust_boundary purpose")
                    })?
                }
                Some("notes") => {
                    self.set_block_field(&mut notes, "atrust_boundary", "notes", |parser| {
                        parser.expect_string("atrust_boundary notes")
                    })?
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected atrust_boundary item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected an atrust_boundary field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        let unknown_material = || {
            Spanned::new(
                ATrustMaterialBoundary::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_exec = || Spanned::new(ATrustExecution::Unknown(String::new()), fallback_span);
        Ok(ATrustBoundaryDecl {
            name,
            crypto_boundary: crypto_boundary
                .unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            did_methods: did_methods.unwrap_or_default(),
            identity_format: identity_format.unwrap_or_else(|| {
                Spanned::new(ATrustIdentityFormat::Unknown(String::new()), fallback_span)
            }),
            credential_mode: credential_mode.unwrap_or_else(|| {
                Spanned::new(ATrustCredentialMode::Unknown(String::new()), fallback_span)
            }),
            handshake: handshake.unwrap_or_else(|| {
                Spanned::new(ATrustHandshakeMode::Unknown(String::new()), fallback_span)
            }),
            resolution: resolution.unwrap_or_else(|| {
                Spanned::new(ATrustResolutionMode::Unknown(String::new()), fallback_span)
            }),
            key_material: key_material.unwrap_or_else(unknown_material),
            secret_material: secret_material.unwrap_or_else(unknown_material),
            execution: execution.unwrap_or_else(unknown_exec),
            post_quantum_ready,
            security_claims: security_claims.unwrap_or_else(|| {
                Spanned::new(ATrustSecurityClaims::Unknown(String::new()), fallback_span)
            }),
            purpose: purpose.unwrap_or_default(),
            notes,
        })
    }

    fn parse_atrust_identity(&mut self) -> Result<ATrustIdentityDecl, Diagnostic> {
        self.expect_keyword("atrust_identity")?;
        let name = self.expect_identifier("atrust identity name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut subject = None;
        let mut did = None;
        let mut method = None;
        let mut boundary = None;
        let mut status = None;
        let mut validation = None;
        let mut resolution = None;
        let mut key_material = None;
        let mut secret_material = None;
        let mut execution = None;
        let mut evidence = None;
        let mut security_claims = None;
        let mut purpose: Option<Vec<Spanned<String>>> = None;
        let mut notes = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated atrust_identity declaration")?;
            match self.peek_identifier() {
                Some("subject") => {
                    self.set_block_field(&mut subject, "atrust_identity", "subject", |parser| {
                        parser.expect_identifier("atrust_identity subject agent")
                    })?
                }
                Some("did") => {
                    self.set_block_field(&mut did, "atrust_identity", "did", |parser| {
                        parser.expect_string("atrust_identity did")
                    })?
                }
                Some("method") => {
                    self.set_block_field(&mut method, "atrust_identity", "method", |parser| {
                        parser.expect_identifier("atrust_identity did_method reference")
                    })?
                }
                Some("boundary") => {
                    self.set_block_field(&mut boundary, "atrust_identity", "boundary", |parser| {
                        parser.expect_identifier("atrust_identity atrust_boundary reference")
                    })?
                }
                Some("status") => {
                    self.set_block_field(&mut status, "atrust_identity", "status", |parser| {
                        let token = parser.expect_identifier("atrust_identity status")?;
                        let value = match token.value.as_str() {
                            "active" => ATrustIdentityStatus::Active,
                            "suspended" => ATrustIdentityStatus::Suspended,
                            "revoked" => ATrustIdentityStatus::Revoked,
                            other => ATrustIdentityStatus::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("validation") => self.set_block_field(
                    &mut validation,
                    "atrust_identity",
                    "validation",
                    |parser| {
                        let token = parser.expect_identifier("atrust_identity validation")?;
                        let value = match token.value.as_str() {
                            "dry_run" => ATrustIdentityValidation::DryRun,
                            other => ATrustIdentityValidation::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("resolution") => self.set_block_field(
                    &mut resolution,
                    "atrust_identity",
                    "resolution",
                    |parser| {
                        let token = parser.expect_identifier("atrust_identity resolution")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustResolutionMode::Disabled,
                            "embedded" => ATrustResolutionMode::Embedded,
                            "local" => ATrustResolutionMode::Local,
                            other => ATrustResolutionMode::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("key_material") => self.set_block_field(
                    &mut key_material,
                    "atrust_identity",
                    "key_material",
                    |parser| {
                        let token = parser.expect_identifier("atrust_identity key_material")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustMaterialBoundary::Denied,
                            other => ATrustMaterialBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("secret_material") => self.set_block_field(
                    &mut secret_material,
                    "atrust_identity",
                    "secret_material",
                    |parser| {
                        let token = parser.expect_identifier("atrust_identity secret_material")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustMaterialBoundary::Denied,
                            other => ATrustMaterialBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("execution") => self.set_block_field(
                    &mut execution,
                    "atrust_identity",
                    "execution",
                    |parser| {
                        let token = parser.expect_identifier("atrust_identity execution")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustExecution::Disabled,
                            other => ATrustExecution::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("evidence") => {
                    self.set_block_field(&mut evidence, "atrust_identity", "evidence", |parser| {
                        let token = parser.expect_identifier("atrust_identity evidence")?;
                        let value = match token.value.as_str() {
                            "required" => ATrustEvidenceRequirement::Required,
                            other => ATrustEvidenceRequirement::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("security_claims") => self.set_block_field(
                    &mut security_claims,
                    "atrust_identity",
                    "security_claims",
                    |parser| {
                        let token = parser.expect_identifier("atrust_identity security_claims")?;
                        let value = match token.value.as_str() {
                            "none" => ATrustSecurityClaims::None,
                            other => ATrustSecurityClaims::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("purpose") => {
                    self.set_block_field(&mut purpose, "atrust_identity", "purpose", |parser| {
                        parser.parse_string_array("atrust_identity purpose")
                    })?
                }
                Some("notes") => {
                    self.set_block_field(&mut notes, "atrust_identity", "notes", |parser| {
                        parser.expect_string("atrust_identity notes")
                    })?
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected atrust_identity item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected an atrust_identity field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        let unknown_status =
            || Spanned::new(ATrustIdentityStatus::Unknown(String::new()), fallback_span);
        let unknown_validation = || {
            Spanned::new(
                ATrustIdentityValidation::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_material = || {
            Spanned::new(
                ATrustMaterialBoundary::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_exec = || Spanned::new(ATrustExecution::Unknown(String::new()), fallback_span);
        let unknown_evidence = || {
            Spanned::new(
                ATrustEvidenceRequirement::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_claims =
            || Spanned::new(ATrustSecurityClaims::Unknown(String::new()), fallback_span);

        Ok(ATrustIdentityDecl {
            name,
            subject: subject.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            did: did.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            method: method.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            boundary: boundary.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            status: status.unwrap_or_else(unknown_status),
            validation: validation.unwrap_or_else(unknown_validation),
            resolution: resolution.unwrap_or_else(|| {
                Spanned::new(ATrustResolutionMode::Unknown(String::new()), fallback_span)
            }),
            key_material: key_material.unwrap_or_else(unknown_material),
            secret_material: secret_material.unwrap_or_else(unknown_material),
            execution: execution.unwrap_or_else(unknown_exec),
            evidence: evidence.unwrap_or_else(unknown_evidence),
            security_claims: security_claims.unwrap_or_else(unknown_claims),
            purpose: purpose.unwrap_or_default(),
            notes,
        })
    }

    fn parse_atrust_credential_contract(
        &mut self,
    ) -> Result<ATrustCredentialContractDecl, Diagnostic> {
        self.expect_keyword("atrust_credential_contract")?;
        let name = self.expect_identifier("atrust credential contract name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut subject = None;
        let mut identity = None;
        let mut boundary = None;
        let mut method = None;
        let mut issuer_did = None;
        let mut holder_did = None;
        let mut credential_type = None;
        let mut schema = None;
        let mut status = None;
        let mut verification = None;
        let mut presentation = None;
        let mut resolution = None;
        let mut key_material = None;
        let mut secret_material = None;
        let mut execution = None;
        let mut evidence = None;
        let mut security_claims = None;
        let mut claims: Option<Vec<Spanned<String>>> = None;
        let mut purpose: Option<Vec<Spanned<String>>> = None;
        let mut notes = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated atrust_credential_contract declaration")?;
            match self.peek_identifier() {
                Some("subject") => self.set_block_field(
                    &mut subject,
                    "atrust_credential_contract",
                    "subject",
                    |parser| parser.expect_identifier("atrust_credential_contract subject agent"),
                )?,
                Some("identity") => self.set_block_field(
                    &mut identity,
                    "atrust_credential_contract",
                    "identity",
                    |parser| {
                        parser.expect_identifier("atrust_credential_contract identity reference")
                    },
                )?,
                Some("boundary") => self.set_block_field(
                    &mut boundary,
                    "atrust_credential_contract",
                    "boundary",
                    |parser| {
                        parser.expect_identifier(
                            "atrust_credential_contract atrust_boundary reference",
                        )
                    },
                )?,
                Some("method") => self.set_block_field(
                    &mut method,
                    "atrust_credential_contract",
                    "method",
                    |parser| {
                        parser.expect_identifier("atrust_credential_contract did_method reference")
                    },
                )?,
                Some("issuer_did") => self.set_block_field(
                    &mut issuer_did,
                    "atrust_credential_contract",
                    "issuer_did",
                    |parser| parser.expect_string("atrust_credential_contract issuer_did"),
                )?,
                Some("holder_did") => self.set_block_field(
                    &mut holder_did,
                    "atrust_credential_contract",
                    "holder_did",
                    |parser| parser.expect_string("atrust_credential_contract holder_did"),
                )?,
                Some("credential_type") => self.set_block_field(
                    &mut credential_type,
                    "atrust_credential_contract",
                    "credential_type",
                    |parser| parser.expect_string("atrust_credential_contract credential_type"),
                )?,
                Some("schema") => self.set_block_field(
                    &mut schema,
                    "atrust_credential_contract",
                    "schema",
                    |parser| parser.expect_string("atrust_credential_contract schema"),
                )?,
                Some("status") => self.set_block_field(
                    &mut status,
                    "atrust_credential_contract",
                    "status",
                    |parser| {
                        let token =
                            parser.expect_identifier("atrust_credential_contract status")?;
                        let value = match token.value.as_str() {
                            "declared" => ATrustCredentialStatus::Declared,
                            "active" => ATrustCredentialStatus::Active,
                            "suspended" => ATrustCredentialStatus::Suspended,
                            "revoked" => ATrustCredentialStatus::Revoked,
                            other => ATrustCredentialStatus::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("verification") => self.set_block_field(
                    &mut verification,
                    "atrust_credential_contract",
                    "verification",
                    |parser| {
                        let token =
                            parser.expect_identifier("atrust_credential_contract verification")?;
                        let value = match token.value.as_str() {
                            "declared_only" => ATrustCredentialVerification::DeclaredOnly,
                            other => ATrustCredentialVerification::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("presentation") => self.set_block_field(
                    &mut presentation,
                    "atrust_credential_contract",
                    "presentation",
                    |parser| {
                        let token =
                            parser.expect_identifier("atrust_credential_contract presentation")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustCredentialPresentation::Disabled,
                            "declared_only" => ATrustCredentialPresentation::DeclaredOnly,
                            other => ATrustCredentialPresentation::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("resolution") => self.set_block_field(
                    &mut resolution,
                    "atrust_credential_contract",
                    "resolution",
                    |parser| {
                        let token =
                            parser.expect_identifier("atrust_credential_contract resolution")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustResolutionMode::Disabled,
                            "embedded" => ATrustResolutionMode::Embedded,
                            "local" => ATrustResolutionMode::Local,
                            other => ATrustResolutionMode::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("key_material") => self.set_block_field(
                    &mut key_material,
                    "atrust_credential_contract",
                    "key_material",
                    |parser| {
                        let token =
                            parser.expect_identifier("atrust_credential_contract key_material")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustMaterialBoundary::Denied,
                            other => ATrustMaterialBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("secret_material") => self.set_block_field(
                    &mut secret_material,
                    "atrust_credential_contract",
                    "secret_material",
                    |parser| {
                        let token = parser
                            .expect_identifier("atrust_credential_contract secret_material")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustMaterialBoundary::Denied,
                            other => ATrustMaterialBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("execution") => self.set_block_field(
                    &mut execution,
                    "atrust_credential_contract",
                    "execution",
                    |parser| {
                        let token =
                            parser.expect_identifier("atrust_credential_contract execution")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustExecution::Disabled,
                            other => ATrustExecution::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("evidence") => self.set_block_field(
                    &mut evidence,
                    "atrust_credential_contract",
                    "evidence",
                    |parser| {
                        let token =
                            parser.expect_identifier("atrust_credential_contract evidence")?;
                        let value = match token.value.as_str() {
                            "required" => ATrustEvidenceRequirement::Required,
                            other => ATrustEvidenceRequirement::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("security_claims") => self.set_block_field(
                    &mut security_claims,
                    "atrust_credential_contract",
                    "security_claims",
                    |parser| {
                        let token = parser
                            .expect_identifier("atrust_credential_contract security_claims")?;
                        let value = match token.value.as_str() {
                            "none" => ATrustSecurityClaims::None,
                            other => ATrustSecurityClaims::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("claims") => self.set_block_field(
                    &mut claims,
                    "atrust_credential_contract",
                    "claims",
                    |parser| parser.parse_string_array("atrust_credential_contract claims"),
                )?,
                Some("purpose") => self.set_block_field(
                    &mut purpose,
                    "atrust_credential_contract",
                    "purpose",
                    |parser| parser.parse_string_array("atrust_credential_contract purpose"),
                )?,
                Some("notes") => self.set_block_field(
                    &mut notes,
                    "atrust_credential_contract",
                    "notes",
                    |parser| parser.expect_string("atrust_credential_contract notes"),
                )?,
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected atrust_credential_contract item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected an atrust_credential_contract field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        let unknown_status = || {
            Spanned::new(
                ATrustCredentialStatus::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_verification = || {
            Spanned::new(
                ATrustCredentialVerification::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_presentation = || {
            Spanned::new(
                ATrustCredentialPresentation::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_material = || {
            Spanned::new(
                ATrustMaterialBoundary::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_exec = || Spanned::new(ATrustExecution::Unknown(String::new()), fallback_span);
        let unknown_evidence = || {
            Spanned::new(
                ATrustEvidenceRequirement::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_claims =
            || Spanned::new(ATrustSecurityClaims::Unknown(String::new()), fallback_span);

        Ok(ATrustCredentialContractDecl {
            name,
            subject: subject.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            identity: identity.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            boundary: boundary.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            method: method.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            issuer_did: issuer_did.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            holder_did: holder_did.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            credential_type: credential_type
                .unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            schema: schema.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            status: status.unwrap_or_else(unknown_status),
            verification: verification.unwrap_or_else(unknown_verification),
            presentation: presentation.unwrap_or_else(unknown_presentation),
            resolution: resolution.unwrap_or_else(|| {
                Spanned::new(ATrustResolutionMode::Unknown(String::new()), fallback_span)
            }),
            key_material: key_material.unwrap_or_else(unknown_material),
            secret_material: secret_material.unwrap_or_else(unknown_material),
            execution: execution.unwrap_or_else(unknown_exec),
            evidence: evidence.unwrap_or_else(unknown_evidence),
            security_claims: security_claims.unwrap_or_else(unknown_claims),
            claims: claims.unwrap_or_default(),
            purpose: purpose.unwrap_or_default(),
            notes,
        })
    }

    fn parse_atrust_handshake(&mut self) -> Result<ATrustHandshakeDecl, Diagnostic> {
        self.expect_keyword("atrust_handshake")?;
        let name = self.expect_identifier("atrust handshake name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut initiator = None;
        let mut responder = None;
        let mut initiator_identity = None;
        let mut responder_identity = None;
        let mut credential_contracts: Option<Vec<Spanned<String>>> = None;
        let mut boundary = None;
        let mut method = None;
        let mut mode = None;
        let mut direction = None;
        let mut challenge = None;
        let mut response = None;
        let mut transcript = None;
        let mut verification = None;
        let mut resolution = None;
        let mut network = None;
        let mut key_material = None;
        let mut secret_material = None;
        let mut execution = None;
        let mut evidence = None;
        let mut security_claims = None;
        let mut purpose: Option<Vec<Spanned<String>>> = None;
        let mut notes = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated atrust_handshake declaration")?;
            match self.peek_identifier() {
                Some("initiator") => self.set_block_field(
                    &mut initiator,
                    "atrust_handshake",
                    "initiator",
                    |parser| parser.expect_identifier("atrust_handshake initiator agent"),
                )?,
                Some("responder") => self.set_block_field(
                    &mut responder,
                    "atrust_handshake",
                    "responder",
                    |parser| parser.expect_identifier("atrust_handshake responder agent"),
                )?,
                Some("initiator_identity") => self.set_block_field(
                    &mut initiator_identity,
                    "atrust_handshake",
                    "initiator_identity",
                    |parser| {
                        parser.expect_identifier("atrust_handshake initiator_identity reference")
                    },
                )?,
                Some("responder_identity") => self.set_block_field(
                    &mut responder_identity,
                    "atrust_handshake",
                    "responder_identity",
                    |parser| {
                        parser.expect_identifier("atrust_handshake responder_identity reference")
                    },
                )?,
                Some("credential_contracts") => self.set_block_field(
                    &mut credential_contracts,
                    "atrust_handshake",
                    "credential_contracts",
                    |parser| parser.parse_string_array("atrust_handshake credential_contracts"),
                )?,
                Some("boundary") => {
                    self.set_block_field(&mut boundary, "atrust_handshake", "boundary", |parser| {
                        parser.expect_identifier("atrust_handshake atrust_boundary reference")
                    })?
                }
                Some("method") => {
                    self.set_block_field(&mut method, "atrust_handshake", "method", |parser| {
                        parser.expect_identifier("atrust_handshake did_method reference")
                    })?
                }
                Some("mode") => {
                    self.set_block_field(&mut mode, "atrust_handshake", "mode", |parser| {
                        let token = parser.expect_identifier("atrust_handshake mode")?;
                        let value = match token.value.as_str() {
                            "dry_run" => ATrustHandshakeDryRunMode::DryRun,
                            other => ATrustHandshakeDryRunMode::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("direction") => self.set_block_field(
                    &mut direction,
                    "atrust_handshake",
                    "direction",
                    |parser| {
                        let token = parser.expect_identifier("atrust_handshake direction")?;
                        let value = match token.value.as_str() {
                            "one_way" => ATrustHandshakeDirection::OneWay,
                            "mutual" => ATrustHandshakeDirection::Mutual,
                            other => ATrustHandshakeDirection::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("challenge") => self.set_block_field(
                    &mut challenge,
                    "atrust_handshake",
                    "challenge",
                    |parser| {
                        let token = parser.expect_identifier("atrust_handshake challenge")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustHandshakeChallenge::Disabled,
                            "declared_only" => ATrustHandshakeChallenge::DeclaredOnly,
                            other => ATrustHandshakeChallenge::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("response") => {
                    self.set_block_field(&mut response, "atrust_handshake", "response", |parser| {
                        let token = parser.expect_identifier("atrust_handshake response")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustHandshakeResponse::Disabled,
                            "declared_only" => ATrustHandshakeResponse::DeclaredOnly,
                            other => ATrustHandshakeResponse::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("transcript") => self.set_block_field(
                    &mut transcript,
                    "atrust_handshake",
                    "transcript",
                    |parser| {
                        let token = parser.expect_identifier("atrust_handshake transcript")?;
                        let value = match token.value.as_str() {
                            "metadata_only" => ATrustHandshakeTranscript::MetadataOnly,
                            "evidence_only" => ATrustHandshakeTranscript::EvidenceOnly,
                            other => ATrustHandshakeTranscript::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("verification") => self.set_block_field(
                    &mut verification,
                    "atrust_handshake",
                    "verification",
                    |parser| {
                        let token = parser.expect_identifier("atrust_handshake verification")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustHandshakeVerification::Disabled,
                            "declared_only" => ATrustHandshakeVerification::DeclaredOnly,
                            other => ATrustHandshakeVerification::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("resolution") => self.set_block_field(
                    &mut resolution,
                    "atrust_handshake",
                    "resolution",
                    |parser| {
                        let token = parser.expect_identifier("atrust_handshake resolution")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustResolutionMode::Disabled,
                            "embedded" => ATrustResolutionMode::Embedded,
                            "local" => ATrustResolutionMode::Local,
                            other => ATrustResolutionMode::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("network") => {
                    self.set_block_field(&mut network, "atrust_handshake", "network", |parser| {
                        let token = parser.expect_identifier("atrust_handshake network")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustNetworkBoundary::Denied,
                            other => ATrustNetworkBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("key_material") => self.set_block_field(
                    &mut key_material,
                    "atrust_handshake",
                    "key_material",
                    |parser| {
                        let token = parser.expect_identifier("atrust_handshake key_material")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustMaterialBoundary::Denied,
                            other => ATrustMaterialBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("secret_material") => self.set_block_field(
                    &mut secret_material,
                    "atrust_handshake",
                    "secret_material",
                    |parser| {
                        let token = parser.expect_identifier("atrust_handshake secret_material")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustMaterialBoundary::Denied,
                            other => ATrustMaterialBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("execution") => self.set_block_field(
                    &mut execution,
                    "atrust_handshake",
                    "execution",
                    |parser| {
                        let token = parser.expect_identifier("atrust_handshake execution")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustExecution::Disabled,
                            other => ATrustExecution::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("evidence") => {
                    self.set_block_field(&mut evidence, "atrust_handshake", "evidence", |parser| {
                        let token = parser.expect_identifier("atrust_handshake evidence")?;
                        let value = match token.value.as_str() {
                            "required" => ATrustEvidenceRequirement::Required,
                            other => ATrustEvidenceRequirement::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("security_claims") => self.set_block_field(
                    &mut security_claims,
                    "atrust_handshake",
                    "security_claims",
                    |parser| {
                        let token = parser.expect_identifier("atrust_handshake security_claims")?;
                        let value = match token.value.as_str() {
                            "none" => ATrustSecurityClaims::None,
                            other => ATrustSecurityClaims::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("purpose") => {
                    self.set_block_field(&mut purpose, "atrust_handshake", "purpose", |parser| {
                        parser.parse_string_array("atrust_handshake purpose")
                    })?
                }
                Some("notes") => {
                    self.set_block_field(&mut notes, "atrust_handshake", "notes", |parser| {
                        parser.expect_string("atrust_handshake notes")
                    })?
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected atrust_handshake item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected an atrust_handshake field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        let unknown_mode = || {
            Spanned::new(
                ATrustHandshakeDryRunMode::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_dir = || {
            Spanned::new(
                ATrustHandshakeDirection::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_chal = || {
            Spanned::new(
                ATrustHandshakeChallenge::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_resp = || {
            Spanned::new(
                ATrustHandshakeResponse::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_trans = || {
            Spanned::new(
                ATrustHandshakeTranscript::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_ver = || {
            Spanned::new(
                ATrustHandshakeVerification::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_res =
            || Spanned::new(ATrustResolutionMode::Unknown(String::new()), fallback_span);
        let unknown_net =
            || Spanned::new(ATrustNetworkBoundary::Unknown(String::new()), fallback_span);
        let unknown_mat = || {
            Spanned::new(
                ATrustMaterialBoundary::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_exec = || Spanned::new(ATrustExecution::Unknown(String::new()), fallback_span);
        let unknown_evid = || {
            Spanned::new(
                ATrustEvidenceRequirement::Unknown(String::new()),
                fallback_span,
            )
        };
        let unknown_claims =
            || Spanned::new(ATrustSecurityClaims::Unknown(String::new()), fallback_span);

        Ok(ATrustHandshakeDecl {
            name,
            initiator: initiator.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            responder: responder.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            initiator_identity: initiator_identity
                .unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            responder_identity: responder_identity
                .unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            credential_contracts: credential_contracts.unwrap_or_default(),
            boundary: boundary.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            method: method.unwrap_or_else(|| Spanned::new(String::new(), fallback_span)),
            mode: mode.unwrap_or_else(unknown_mode),
            direction: direction.unwrap_or_else(unknown_dir),
            challenge: challenge.unwrap_or_else(unknown_chal),
            response: response.unwrap_or_else(unknown_resp),
            transcript: transcript.unwrap_or_else(unknown_trans),
            verification: verification.unwrap_or_else(unknown_ver),
            resolution: resolution.unwrap_or_else(unknown_res),
            network: network.unwrap_or_else(unknown_net),
            key_material: key_material.unwrap_or_else(unknown_mat),
            secret_material: secret_material.unwrap_or_else(unknown_mat),
            execution: execution.unwrap_or_else(unknown_exec),
            evidence: evidence.unwrap_or_else(unknown_evid),
            security_claims: security_claims.unwrap_or_else(unknown_claims),
            purpose: purpose.unwrap_or_default(),
            notes,
        })
    }

    fn parse_trust_ledger(&mut self) -> Result<TrustLedgerDecl, Diagnostic> {
        self.expect_keyword("trust_ledger")?;
        let name = self.expect_identifier("trust ledger name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut scope = None;
        let mut mode = None;
        let mut hash_algorithm = None;
        let mut chain_policy = None;
        let mut entries: Option<Vec<TrustLedgerEntryDecl>> = None;
        let mut chain_root = None;
        let mut network = None;
        let mut key_material = None;
        let mut secret_material = None;
        let mut execution = None;
        let mut evidence = None;
        let mut security_claims = None;
        let mut purpose: Option<Vec<Spanned<String>>> = None;
        let mut notes = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated trust_ledger declaration")?;
            match self.peek_identifier() {
                Some("scope") => {
                    self.set_block_field(&mut scope, "trust_ledger", "scope", |parser| {
                        let token = parser.expect_identifier("trust_ledger scope")?;
                        let value = match token.value.as_str() {
                            "local" => TrustLedgerScope::Local,
                            "package" => TrustLedgerScope::Package,
                            "bundle" => TrustLedgerScope::Bundle,
                            other => TrustLedgerScope::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("mode") => {
                    self.set_block_field(&mut mode, "trust_ledger", "mode", |parser| {
                        let token = parser.expect_identifier("trust_ledger mode")?;
                        let value = match token.value.as_str() {
                            "dry_run" => TrustLedgerMode::DryRun,
                            "declared_only" => TrustLedgerMode::DeclaredOnly,
                            other => TrustLedgerMode::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("hash_algorithm") => self.set_block_field(
                    &mut hash_algorithm,
                    "trust_ledger",
                    "hash_algorithm",
                    |parser| parser.expect_identifier("trust_ledger hash_algorithm reference"),
                )?,
                Some("chain_policy") => self.set_block_field(
                    &mut chain_policy,
                    "trust_ledger",
                    "chain_policy",
                    |parser| {
                        let token = parser.expect_identifier("trust_ledger chain_policy")?;
                        let value = match token.value.as_str() {
                            "append_only" => TrustLedgerChainPolicy::AppendOnly,
                            "declared_only" => TrustLedgerChainPolicy::DeclaredOnly,
                            other => TrustLedgerChainPolicy::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("entries") => {
                    self.set_block_field(&mut entries, "trust_ledger", "entries", |parser| {
                        parser.parse_trust_ledger_entries()
                    })?
                }
                Some("chain_root") => {
                    self.set_block_field(&mut chain_root, "trust_ledger", "chain_root", |parser| {
                        parser.expect_string("trust_ledger chain_root")
                    })?
                }
                Some("network") => {
                    self.set_block_field(&mut network, "trust_ledger", "network", |parser| {
                        let token = parser.expect_identifier("trust_ledger network")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustNetworkBoundary::Denied,
                            other => ATrustNetworkBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("key_material") => self.set_block_field(
                    &mut key_material,
                    "trust_ledger",
                    "key_material",
                    |parser| {
                        let token = parser.expect_identifier("trust_ledger key_material")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustMaterialBoundary::Denied,
                            other => ATrustMaterialBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("secret_material") => self.set_block_field(
                    &mut secret_material,
                    "trust_ledger",
                    "secret_material",
                    |parser| {
                        let token = parser.expect_identifier("trust_ledger secret_material")?;
                        let value = match token.value.as_str() {
                            "denied" => ATrustMaterialBoundary::Denied,
                            other => ATrustMaterialBoundary::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("execution") => {
                    self.set_block_field(&mut execution, "trust_ledger", "execution", |parser| {
                        let token = parser.expect_identifier("trust_ledger execution")?;
                        let value = match token.value.as_str() {
                            "disabled" => ATrustExecution::Disabled,
                            other => ATrustExecution::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("evidence") => {
                    self.set_block_field(&mut evidence, "trust_ledger", "evidence", |parser| {
                        let token = parser.expect_identifier("trust_ledger evidence")?;
                        let value = match token.value.as_str() {
                            "required" => ATrustEvidenceRequirement::Required,
                            other => ATrustEvidenceRequirement::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("security_claims") => self.set_block_field(
                    &mut security_claims,
                    "trust_ledger",
                    "security_claims",
                    |parser| {
                        let token = parser.expect_identifier("trust_ledger security_claims")?;
                        let value = match token.value.as_str() {
                            "none" => ATrustSecurityClaims::None,
                            other => ATrustSecurityClaims::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    },
                )?,
                Some("purpose") => {
                    self.set_block_field(&mut purpose, "trust_ledger", "purpose", |parser| {
                        parser.parse_string_array("trust_ledger purpose")
                    })?
                }
                Some("notes") => {
                    self.set_block_field(&mut notes, "trust_ledger", "notes", |parser| {
                        parser.expect_string("trust_ledger notes")
                    })?
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected trust_ledger item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected a trust_ledger field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        let empty = || Spanned::new(String::new(), fallback_span);
        Ok(TrustLedgerDecl {
            name,
            scope: scope.unwrap_or_else(|| {
                Spanned::new(TrustLedgerScope::Unknown(String::new()), fallback_span)
            }),
            mode: mode.unwrap_or_else(|| {
                Spanned::new(TrustLedgerMode::Unknown(String::new()), fallback_span)
            }),
            hash_algorithm: hash_algorithm.unwrap_or_else(empty),
            chain_policy: chain_policy.unwrap_or_else(|| {
                Spanned::new(
                    TrustLedgerChainPolicy::Unknown(String::new()),
                    fallback_span,
                )
            }),
            entries: entries.unwrap_or_default(),
            chain_root: chain_root.unwrap_or_else(empty),
            network: network.unwrap_or_else(|| {
                Spanned::new(ATrustNetworkBoundary::Unknown(String::new()), fallback_span)
            }),
            key_material: key_material.unwrap_or_else(|| {
                Spanned::new(
                    ATrustMaterialBoundary::Unknown(String::new()),
                    fallback_span,
                )
            }),
            secret_material: secret_material.unwrap_or_else(|| {
                Spanned::new(
                    ATrustMaterialBoundary::Unknown(String::new()),
                    fallback_span,
                )
            }),
            execution: execution.unwrap_or_else(|| {
                Spanned::new(ATrustExecution::Unknown(String::new()), fallback_span)
            }),
            evidence: evidence.unwrap_or_else(|| {
                Spanned::new(
                    ATrustEvidenceRequirement::Unknown(String::new()),
                    fallback_span,
                )
            }),
            security_claims: security_claims.unwrap_or_else(|| {
                Spanned::new(ATrustSecurityClaims::Unknown(String::new()), fallback_span)
            }),
            purpose: purpose.unwrap_or_default(),
            notes,
        })
    }

    fn parse_trust_ledger_entries(&mut self) -> Result<Vec<TrustLedgerEntryDecl>, Diagnostic> {
        self.expect_symbol(TokenKind::LeftBracket, "`[`")?;
        let mut entries = Vec::new();
        while !self.check(&TokenKind::RightBracket) {
            self.ensure_not_eof("unterminated trust_ledger entries array")?;
            entries.push(self.parse_trust_ledger_entry()?);
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }
        self.advance();
        Ok(entries)
    }

    fn parse_trust_ledger_entry(&mut self) -> Result<TrustLedgerEntryDecl, Diagnostic> {
        let open = self.peek().span;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut id = None;
        let mut kind = None;
        let mut subject = None;
        let mut previous_hash = None;
        let mut entry_hash = None;
        let mut evidence_ref = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated trust_ledger entry")?;
            match self.peek_identifier() {
                Some("id") => {
                    self.set_block_field(&mut id, "trust_ledger entry", "id", |parser| {
                        parser.expect_string("trust_ledger entry id")
                    })?
                }
                Some("kind") => {
                    self.set_block_field(&mut kind, "trust_ledger entry", "kind", |parser| {
                        let token = parser.expect_identifier("trust_ledger entry kind")?;
                        let value = match token.value.as_str() {
                            "identity" => TrustLedgerEntryKind::Identity,
                            "credential" => TrustLedgerEntryKind::Credential,
                            "handshake" => TrustLedgerEntryKind::Handshake,
                            "evidence" => TrustLedgerEntryKind::Evidence,
                            "policy" => TrustLedgerEntryKind::Policy,
                            "custom" => TrustLedgerEntryKind::Custom,
                            other => TrustLedgerEntryKind::Unknown(other.to_owned()),
                        };
                        Ok(Spanned::new(value, token.span))
                    })?
                }
                Some("subject") => {
                    self.set_block_field(&mut subject, "trust_ledger entry", "subject", |parser| {
                        parser.expect_identifier("trust_ledger entry subject")
                    })?
                }
                Some("previous_hash") => self.set_block_field(
                    &mut previous_hash,
                    "trust_ledger entry",
                    "previous_hash",
                    |parser| parser.expect_string("trust_ledger entry previous_hash"),
                )?,
                Some("entry_hash") => self.set_block_field(
                    &mut entry_hash,
                    "trust_ledger entry",
                    "entry_hash",
                    |parser| parser.expect_string("trust_ledger entry entry_hash"),
                )?,
                Some("evidence_ref") => self.set_block_field(
                    &mut evidence_ref,
                    "trust_ledger entry",
                    "evidence_ref",
                    |parser| parser.expect_string("trust_ledger entry evidence_ref"),
                )?,
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected trust_ledger entry item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected a trust_ledger entry field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let empty = || Spanned::new(String::new(), open);
        Ok(TrustLedgerEntryDecl {
            id: id.unwrap_or_else(empty),
            kind: kind.unwrap_or_else(|| {
                Spanned::new(TrustLedgerEntryKind::Unknown(String::new()), open)
            }),
            subject: subject.unwrap_or_else(empty),
            previous_hash: previous_hash.unwrap_or_else(empty),
            entry_hash: entry_hash.unwrap_or_else(empty),
            evidence_ref: evidence_ref.unwrap_or_else(empty),
        })
    }

    /// Consume a block key keyword and parse its value, rejecting duplicates.
    fn set_block_field<T>(
        &mut self,
        slot: &mut Option<T>,
        block: &str,
        key: &str,
        parse_value: impl FnOnce(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<(), Diagnostic> {
        let span = self.peek().span;
        self.advance();
        if slot.is_some() {
            return Err(Diagnostic::new(
                format!("duplicate {block} field `{key}`"),
                span,
            ));
        }
        *slot = Some(parse_value(self)?);
        Ok(())
    }

    fn parse_identifier_block(
        &mut self,
        description: &str,
    ) -> Result<Vec<Spanned<String>>, Diagnostic> {
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        let mut values = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated provider allowlist block")?;
            values.push(self.expect_identifier(description)?);
        }
        self.advance();
        Ok(values)
    }
    fn parse_assertion(&mut self) -> Result<AssertionDecl, Diagnostic> {
        self.expect_keyword("assert")?;
        let name = self.expect_identifier("assertion name")?;
        let argument = if name.value == "runtime_status" {
            Some(self.expect_identifier("runtime status assertion argument")?)
        } else {
            None
        };
        Ok(AssertionDecl { name, argument })
    }

    fn parse_policy(&mut self) -> Result<PolicyDecl, Diagnostic> {
        self.expect_keyword("policy")?;
        let name = self.expect_identifier("policy name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        let mut rules = Vec::new();
        let mut violation = None;
        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated policy declaration")?;
            match self.peek_identifier() {
                Some("require") | Some("deny") => {
                    let require = self.match_keyword("require");
                    if !require {
                        self.expect_keyword("deny")?;
                    }
                    let rule = self.parse_policy_rule()?;
                    rules.push(if require {
                        PolicyRuleDecl::Require { rule }
                    } else {
                        PolicyRuleDecl::Deny { rule }
                    });
                }
                Some("on") => {
                    if violation.is_some() {
                        return Err(Diagnostic::new(
                            "duplicate `on violation` block",
                            self.peek().span,
                        ));
                    }
                    violation = Some(self.parse_policy_violation()?);
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected policy item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected `require`, `deny`, or `on violation`",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();
        Ok(PolicyDecl {
            name,
            rules,
            violation,
        })
    }

    fn parse_policy_rule(&mut self) -> Result<Spanned<PolicyRule>, Diagnostic> {
        let token = self.expect_identifier("policy rule")?;
        let value = match token.value.as_str() {
            "no_unhandled_messages" => PolicyRule::NoUnhandledMessages,
            "all_tool_calls_traced" => PolicyRule::AllToolCallsTraced,
            "all_model_calls_traced" => PolicyRule::AllModelCallsTraced,
            "all_intrinsics_traced" => PolicyRule::AllIntrinsicsTraced,
            "all_provider_calls_traced" => PolicyRule::AllProviderCallsTraced,
            "halt_requires_trace" => PolicyRule::HaltRequiresTrace,
            "provider_contracts_declared" => PolicyRule::ProviderContractsDeclared,
            "provider_allowlists_valid" => PolicyRule::ProviderAllowlistsValid,
            "external_execution" => PolicyRule::ExternalExecution,
            "evidence_bundle_verified" => PolicyRule::EvidenceBundleVerified,
            "security_report_generated" => PolicyRule::SecurityReportGenerated,
            "agent_passport_declared" => PolicyRule::AgentPassportDeclared,
            "agent_passport_attested" => PolicyRule::AgentPassportAttested,
            "agent_data_residency_declared" => PolicyRule::AgentDataResidencyDeclared,
            "agent_identity_declared" => PolicyRule::AgentIdentityDeclared,
            "provider_harness_declared" => PolicyRule::ProviderHarnessDeclared,
            "provider_harness_sandboxed" => PolicyRule::ProviderHarnessSandboxed,
            "provider_network_denied" => PolicyRule::ProviderNetworkDenied,
            "provider_secrets_denied" => PolicyRule::ProviderSecretsDenied,
            "provider_filesystem_restricted" => PolicyRule::ProviderFilesystemRestricted,
            "external_provider_harnessed" => PolicyRule::ExternalProviderHarnessed,
            "feature_flags_declared" => PolicyRule::FeatureFlagsDeclared,
            "features_default_disabled" => PolicyRule::FeaturesDefaultDisabled,
            "experimental_features_require_approval" => {
                PolicyRule::ExperimentalFeaturesRequireApproval
            }
            "secret_boundaries_declared" => PolicyRule::SecretBoundariesDeclared,
            "secret_access_denied" => PolicyRule::SecretAccessDenied,
            "secret_values_absent" => PolicyRule::SecretValuesAbsent,
            "external_provider_feature_gated" => PolicyRule::ExternalProviderFeatureGated,
            "external_provider_secret_boundary_declared" => {
                PolicyRule::ExternalProviderSecretBoundaryDeclared
            }
            "adapters_declared" => PolicyRule::AdaptersDeclared,
            "adapters_execution_disabled" => PolicyRule::AdaptersExecutionDisabled,
            "adapters_network_denied" => PolicyRule::AdaptersNetworkDenied,
            "adapters_secrets_denied" => PolicyRule::AdaptersSecretsDenied,
            "adapters_provider_harnessed" => PolicyRule::AdaptersProviderHarnessed,
            "adapters_feature_gated" => PolicyRule::AdaptersFeatureGated,
            "adapters_secret_boundaried" => PolicyRule::AdaptersSecretBoundaried,
            "adapters_conformance_declared" => PolicyRule::AdaptersConformanceDeclared,
            "adapters_evidence_required" => PolicyRule::AdaptersEvidenceRequired,
            "adapter_profiles_declared" => PolicyRule::AdapterProfilesDeclared,
            "adapter_profiles_execution_disabled" => PolicyRule::AdapterProfilesExecutionDisabled,
            "adapter_profiles_network_denied" => PolicyRule::AdapterProfilesNetworkDenied,
            "adapter_profiles_secrets_denied" => PolicyRule::AdapterProfilesSecretsDenied,
            "adapter_profiles_linked" => PolicyRule::AdapterProfilesLinked,
            "adapter_profiles_conformance_declared" => {
                PolicyRule::AdapterProfilesConformanceDeclared
            }
            "vendor_profiles_declared" => PolicyRule::VendorProfilesDeclared,
            "crypto_primitives_declared" => PolicyRule::CryptoPrimitivesDeclared,
            "crypto_primitives_allowed" => PolicyRule::CryptoPrimitivesAllowed,
            "crypto_denied_not_used" => PolicyRule::CryptoDeniedNotUsed,
            "crypto_post_quantum_candidates_declared" => {
                PolicyRule::CryptoPostQuantumCandidatesDeclared
            }
            "crypto_key_material_absent" => PolicyRule::CryptoKeyMaterialAbsent,
            "crypto_secret_material_absent" => PolicyRule::CryptoSecretMaterialAbsent,
            "crypto_execution_absent" => PolicyRule::CryptoExecutionAbsent,
            "crypto_boundaries_declared" => PolicyRule::CryptoBoundariesDeclared,
            "post_quantum_readiness_declared" => PolicyRule::PostQuantumReadinessDeclared,
            "atrust_boundaries_declared" => PolicyRule::ATrustBoundariesDeclared,
            "atrust_did_methods_declared" => PolicyRule::ATrustDidMethodsDeclared,
            "atrust_did_method_allowed" => PolicyRule::ATrustDidMethodAllowed,
            "atrust_identity_format_declared" => PolicyRule::ATrustIdentityFormatDeclared,
            "atrust_credential_mode_declared" => PolicyRule::ATrustCredentialModeDeclared,
            "atrust_handshake_disabled" => PolicyRule::ATrustHandshakeDisabled,
            "atrust_resolution_disabled" => PolicyRule::ATrustResolutionDisabled,
            "atrust_key_material_denied" => PolicyRule::ATrustKeyMaterialDenied,
            "atrust_secret_material_denied" => PolicyRule::ATrustSecretMaterialDenied,
            "atrust_execution_disabled" => PolicyRule::ATrustExecutionDisabled,
            "atrust_post_quantum_readiness_declared" => {
                PolicyRule::ATrustPostQuantumReadinessDeclared
            }
            "atrust_security_claims_none" => PolicyRule::ATrustSecurityClaimsNone,
            "atrust_identity_declared" => PolicyRule::ATrustIdentityDeclared,
            "atrust_identity_subject_valid" => PolicyRule::ATrustIdentitySubjectValid,
            "atrust_identity_did_method_valid" => PolicyRule::ATrustIdentityDidMethodValid,
            "atrust_identity_boundary_valid" => PolicyRule::ATrustIdentityBoundaryValid,
            "atrust_identity_status_active" => PolicyRule::ATrustIdentityStatusActive,
            "atrust_identity_validation_dry_run" => PolicyRule::ATrustIdentityValidationDryRun,
            "atrust_identity_resolution_disabled" => PolicyRule::ATrustIdentityResolutionDisabled,
            "atrust_identity_key_material_denied" => PolicyRule::ATrustIdentityKeyMaterialDenied,
            "atrust_identity_secret_material_denied" => {
                PolicyRule::ATrustIdentitySecretMaterialDenied
            }
            "atrust_identity_execution_disabled" => PolicyRule::ATrustIdentityExecutionDisabled,
            "atrust_identity_evidence_required" => PolicyRule::ATrustIdentityEvidenceRequired,
            "atrust_identity_security_claims_absent" => {
                PolicyRule::ATrustIdentitySecurityClaimsAbsent
            }
            "atrust_identity_passport_consistent" => PolicyRule::ATrustIdentityPassportConsistent,
            "atrust_credential_contract_declared" => PolicyRule::ATrustCredentialContractDeclared,
            "atrust_credential_issuer_did_declared" => {
                PolicyRule::ATrustCredentialIssuerDidDeclared
            }
            "atrust_credential_holder_did_declared" => {
                PolicyRule::ATrustCredentialHolderDidDeclared
            }
            "atrust_credential_type_declared" => PolicyRule::ATrustCredentialTypeDeclared,
            "atrust_credential_schema_declared" => PolicyRule::ATrustCredentialSchemaDeclared,
            "atrust_credential_claims_declared" => PolicyRule::ATrustCredentialClaimsDeclared,
            "atrust_credential_verification_declared_only" => {
                PolicyRule::ATrustCredentialVerificationDeclaredOnly
            }
            "atrust_credential_presentation_disabled" => {
                PolicyRule::ATrustCredentialPresentationDisabled
            }
            "atrust_credential_resolution_disabled" => {
                PolicyRule::ATrustCredentialResolutionDisabled
            }
            "atrust_credential_key_material_denied" => {
                PolicyRule::ATrustCredentialKeyMaterialDenied
            }
            "atrust_credential_secret_material_denied" => {
                PolicyRule::ATrustCredentialSecretMaterialDenied
            }
            "atrust_credential_execution_disabled" => PolicyRule::ATrustCredentialExecutionDisabled,
            "atrust_credential_evidence_required" => PolicyRule::ATrustCredentialEvidenceRequired,
            "atrust_credential_security_claims_absent" => {
                PolicyRule::ATrustCredentialSecurityClaimsAbsent
            }
            "atrust_handshake_declared" => PolicyRule::ATrustHandshakeDeclared,
            "atrust_handshake_initiator_responder_valid" => {
                PolicyRule::ATrustHandshakeInitiatorResponderValid
            }
            "atrust_handshake_identities_valid" => PolicyRule::ATrustHandshakeIdentitiesValid,
            "atrust_handshake_credential_contracts_valid" => {
                PolicyRule::ATrustHandshakeCredentialContractsValid
            }
            "atrust_handshake_boundary_method_valid" => {
                PolicyRule::ATrustHandshakeBoundaryMethodValid
            }
            "atrust_handshake_mode_dry_run" => PolicyRule::ATrustHandshakeModeDryRun,
            "atrust_handshake_direction_valid" => PolicyRule::ATrustHandshakeDirectionValid,
            "atrust_handshake_challenge_declared_only" => {
                PolicyRule::ATrustHandshakeChallengeDeclaredOnly
            }
            "atrust_handshake_response_declared_only" => {
                PolicyRule::ATrustHandshakeResponseDeclaredOnly
            }
            "atrust_handshake_transcript_evidence_only" => {
                PolicyRule::ATrustHandshakeTranscriptEvidenceOnly
            }
            "atrust_handshake_verification_declared_only" => {
                PolicyRule::ATrustHandshakeVerificationDeclaredOnly
            }
            "atrust_handshake_resolution_disabled" => PolicyRule::ATrustHandshakeResolutionDisabled,
            "atrust_handshake_network_denied" => PolicyRule::ATrustHandshakeNetworkDenied,
            "atrust_handshake_key_material_denied" => PolicyRule::ATrustHandshakeKeyMaterialDenied,
            "atrust_handshake_secret_material_denied" => {
                PolicyRule::ATrustHandshakeSecretMaterialDenied
            }
            "atrust_handshake_execution_disabled" => PolicyRule::ATrustHandshakeExecutionDisabled,
            "atrust_handshake_evidence_required" => PolicyRule::ATrustHandshakeEvidenceRequired,
            "atrust_handshake_security_claims_absent" => {
                PolicyRule::ATrustHandshakeSecurityClaimsAbsent
            }
            "trust_ledgers_declared" => PolicyRule::TrustLedgersDeclared,
            "trust_ledger_hash_algorithm_declared" => PolicyRule::TrustLedgerHashAlgorithmDeclared,
            "trust_ledger_chain_valid" => PolicyRule::TrustLedgerChainValid,
            "trust_ledger_entries_bound" => PolicyRule::TrustLedgerEntriesBound,
            "trust_ledger_append_only" => PolicyRule::TrustLedgerAppendOnly,
            "trust_ledger_network_denied" => PolicyRule::TrustLedgerNetworkDenied,
            "trust_ledger_key_material_denied" => PolicyRule::TrustLedgerKeyMaterialDenied,
            "trust_ledger_secret_material_denied" => PolicyRule::TrustLedgerSecretMaterialDenied,
            "trust_ledger_execution_disabled" => PolicyRule::TrustLedgerExecutionDisabled,
            "trust_ledger_evidence_required" => PolicyRule::TrustLedgerEvidenceRequired,
            "trust_ledger_security_claims_absent" => PolicyRule::TrustLedgerSecurityClaimsAbsent,
            "trust_ledger_blockchain_absent" => PolicyRule::TrustLedgerBlockchainAbsent,
            "trust_ledger_signature_absent" => PolicyRule::TrustLedgerSignatureAbsent,
            "runtime_status" => {
                let argument = self.expect_identifier("runtime status policy argument")?;
                if argument.value == "completed" {
                    PolicyRule::RuntimeStatusCompleted
                } else {
                    PolicyRule::Unknown(format!("runtime_status {}", argument.value))
                }
            }
            other => PolicyRule::Unknown(other.to_owned()),
        };
        Ok(Spanned::new(value, token.span))
    }

    fn parse_policy_violation(&mut self) -> Result<PolicyViolationDecl, Diagnostic> {
        self.expect_keyword("on")?;
        self.expect_keyword("violation")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        self.expect_keyword("action")?;
        let action_token = self.expect_identifier("policy violation action")?;
        let action = match action_token.value.as_str() {
            "block" => PolicyViolationAction::Block,
            "review" => PolicyViolationAction::Review,
            "warn" => PolicyViolationAction::Warn,
            other => PolicyViolationAction::Unknown(other.to_owned()),
        };
        let mut trace_required = false;
        if self.match_keyword("trace") {
            self.expect_keyword("required")?;
            trace_required = true;
        }
        self.expect_symbol(TokenKind::RightBrace, "`}`")?;
        Ok(PolicyViolationDecl {
            action: Spanned::new(action, action_token.span),
            trace_required,
        })
    }

    fn parse_failure(&mut self) -> Result<FailureDecl, Diagnostic> {
        self.expect_keyword("failure")?;
        let name = self.expect_identifier("failure name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        self.expect_keyword("action")?;
        let action = self.expect_identifier("failure action")?;
        let trace_required = if self.match_keyword("trace") {
            self.expect_keyword("required")?;
            true
        } else {
            false
        };
        self.expect_symbol(TokenKind::RightBrace, "`}`")?;
        Ok(FailureDecl {
            name,
            action,
            trace_required,
        })
    }

    fn parse_tool(&mut self) -> Result<ToolDecl, Diagnostic> {
        self.expect_keyword("tool")?;
        let name = self.expect_identifier("tool name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        let provider = if self.match_keyword("provider") {
            Some(self.expect_identifier("tool provider")?)
        } else {
            None
        };
        self.expect_keyword("capability")?;
        let capability = self.expect_identifier("tool capability")?;
        self.expect_keyword("input")?;
        let input = self.expect_identifier("tool input type")?;
        self.expect_keyword("output")?;
        let output = self.expect_identifier("tool output type")?;
        self.expect_symbol(TokenKind::RightBrace, "`}`")?;
        Ok(ToolDecl {
            name,
            provider,
            capability,
            input,
            output,
        })
    }

    fn parse_model(&mut self) -> Result<ModelDecl, Diagnostic> {
        self.expect_keyword("model")?;
        let name = self.expect_identifier("model name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        self.expect_keyword("provider")?;
        let provider = self.expect_identifier("model provider")?;
        self.expect_keyword("capability")?;
        let capability = self.expect_identifier("model capability")?;
        self.expect_keyword("input")?;
        let input = self.expect_identifier("model input type")?;
        self.expect_keyword("output")?;
        let output = self.expect_identifier("model output type")?;
        self.expect_symbol(TokenKind::RightBrace, "`}`")?;
        Ok(ModelDecl {
            name,
            provider,
            capability,
            input,
            output,
        })
    }

    fn parse_capability(&mut self) -> Result<CapabilityDecl, Diagnostic> {
        self.expect_keyword("capability")?;
        let name = self.expect_identifier("capability name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        self.expect_keyword("level")?;
        let level_token = self.expect_identifier("capability level")?;
        let level = match level_token.value.as_str() {
            "safe" => CapabilityLevel::Safe,
            "restricted" => CapabilityLevel::Restricted,
            "dangerous" => CapabilityLevel::Dangerous,
            other => {
                return Err(Diagnostic::new(
                    format!(
                        "invalid capability level `{other}`; expected `safe`, `restricted`, or `dangerous`"
                    ),
                    level_token.span,
                ))
            }
        };
        let requires_approval = if self.match_keyword("requires") {
            self.expect_keyword("approval")?;
            true
        } else {
            false
        };
        self.expect_symbol(TokenKind::RightBrace, "`}`")?;

        Ok(CapabilityDecl {
            name,
            level: Spanned::new(level, level_token.span),
            requires_approval,
        })
    }

    fn parse_enum(&mut self) -> Result<EnumDecl, Diagnostic> {
        self.expect_keyword("enum")?;
        let name = self.expect_identifier("enum name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated enum declaration")?;
            variants.push(self.expect_identifier("enum variant")?);
        }
        self.advance();
        Ok(EnumDecl { name, variants })
    }

    fn parse_type(&mut self) -> Result<TypeDecl, Diagnostic> {
        self.expect_keyword("type")?;
        let name = self.expect_identifier("type name")?;
        if !self.check(&TokenKind::LeftBrace) {
            return Ok(TypeDecl {
                name,
                fields: Vec::new(),
            });
        }
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated type declaration")?;
            let field_name = self.expect_identifier("field name")?;
            self.expect_symbol(TokenKind::Colon, "`:`")?;
            let token = self.expect_identifier("field type")?;
            let value = match token.value.as_str() {
                "string" => MessageFieldType::String,
                "bool" => MessageFieldType::Bool,
                "int" => MessageFieldType::Int,
                "float" => MessageFieldType::Float,
                other => MessageFieldType::Unknown(other.to_owned()),
            };
            let field_type = Spanned::new(value, token.span);
            fields.push(FieldDecl {
                name: field_name,
                field_type,
            });
        }
        self.advance();
        Ok(TypeDecl { name, fields })
    }

    fn parse_agent(&mut self) -> Result<AgentDecl, Diagnostic> {
        self.expect_keyword("agent")?;
        let name = self.expect_identifier("agent name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        let mut approval = None;
        let mut receives = Vec::new();
        let mut sends = Vec::new();
        let mut capabilities = Vec::new();
        let mut tools = Vec::new();
        let mut models = Vec::new();
        let mut handlers = Vec::new();

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated agent declaration")?;
            match self.peek_identifier() {
                Some("security") => {
                    if approval.is_some() {
                        return Err(Diagnostic::new(
                            "duplicate security block",
                            self.peek().span,
                        ));
                    }
                    self.advance();
                    self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
                    self.expect_keyword("approval")?;
                    let approval_token = self.expect_identifier("approval value")?;
                    let value = match approval_token.value.as_str() {
                        "granted" => Approval::Granted,
                        "denied" => Approval::Denied,
                        other => {
                            return Err(Diagnostic::new(
                                format!(
                                "invalid approval value `{other}`; expected `granted` or `denied`"
                            ),
                                approval_token.span,
                            ))
                        }
                    };
                    approval = Some(Spanned::new(value, approval_token.span));
                    self.expect_symbol(TokenKind::RightBrace, "`}`")?;
                }
                Some("receives") => {
                    self.advance();
                    let message_type = self.expect_identifier("received message type")?;
                    let from = if self.match_keyword("from") {
                        Some(self.expect_identifier("source agent")?)
                    } else {
                        None
                    };
                    receives.push(ReceiveDecl { message_type, from });
                }
                Some("sends") => {
                    self.advance();
                    let message_type = self.expect_identifier("sent message type")?;
                    self.expect_keyword("to")?;
                    let to = self.expect_identifier("destination agent")?;
                    sends.push(SendDecl { message_type, to });
                }
                Some("capabilities") => {
                    self.advance();
                    self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
                    while !self.check(&TokenKind::RightBrace) {
                        self.ensure_not_eof("unterminated capabilities block")?;
                        capabilities.push(self.expect_identifier("capability")?);
                    }
                    self.advance();
                }
                Some("tools") => {
                    self.advance();
                    self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
                    while !self.check(&TokenKind::RightBrace) {
                        self.ensure_not_eof("unterminated tools block")?;
                        tools.push(self.expect_identifier("tool name")?);
                    }
                    self.advance();
                }
                Some("models") => {
                    self.advance();
                    self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
                    while !self.check(&TokenKind::RightBrace) {
                        self.ensure_not_eof("unterminated models block")?;
                        models.push(self.expect_identifier("model name")?);
                    }
                    self.advance();
                }
                Some("on") => handlers.push(self.parse_handler()?),
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected agent item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => return Err(Diagnostic::new(
                        "expected `security`, `receives`, `sends`, `capabilities`, `tools`, `models`, or `on`",
                    self.peek().span,
                )),
            }
        }
        self.advance();

        Ok(AgentDecl {
            name,
            approval,
            receives,
            sends,
            capabilities,
            tools,
            models,
            handlers,
        })
    }

    fn parse_handler(&mut self) -> Result<HandlerDecl, Diagnostic> {
        self.expect_keyword("on")?;
        let message_type = self.expect_identifier("handler message type")?;
        self.expect_keyword("as")?;
        let binding = self.expect_identifier("handler binding")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        let mut instructions = Vec::new();

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated handler declaration")?;
            match self.peek_identifier() {
                Some("emit") => {
                    self.advance();
                    let emitted_type = self.expect_identifier("emitted message type")?;
                    self.expect_keyword("to")?;
                    let to = self.expect_identifier("emit destination")?;
                    instructions.push(HandlerInstruction::Emit {
                        message_type: emitted_type,
                        to,
                    });
                }
                Some("trace") => {
                    self.advance();
                    let traced_binding = self.expect_identifier("trace binding")?;
                    instructions.push(HandlerInstruction::Trace {
                        binding: traced_binding,
                    });
                }
                Some("halt") => {
                    let span = self.peek().span;
                    self.advance();
                    instructions.push(HandlerInstruction::Halt { span });
                }
                Some("call") => {
                    self.advance();
                    let tool = self.expect_identifier("tool name")?;
                    self.expect_keyword("with")?;
                    let binding = self.expect_identifier("tool call binding")?;
                    instructions.push(HandlerInstruction::CallTool { tool, binding });
                }
                Some("ask") => {
                    self.advance();
                    let model = self.expect_identifier("model name")?;
                    self.expect_keyword("with")?;
                    let binding = self.expect_identifier("model call binding")?;
                    instructions.push(HandlerInstruction::AskModel { model, binding });
                }
                Some(_) if self.peek_next_is(&TokenKind::LeftParen) => {
                    let name = self.expect_identifier("intrinsic name")?;
                    self.expect_symbol(TokenKind::LeftParen, "`(`")?;
                    let argument = self.expect_identifier("intrinsic argument")?;
                    self.expect_symbol(TokenKind::RightParen, "`)`")?;
                    instructions.push(HandlerInstruction::IntrinsicCall { name, argument });
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unsupported handler instruction `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected `emit`, `trace`, or `halt`",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();
        Ok(HandlerDecl {
            message_type,
            binding,
            instructions,
        })
    }

    fn parse_protocol(&mut self) -> Result<ProtocolDecl, Diagnostic> {
        self.expect_keyword("protocol")?;
        let name = self.expect_identifier("protocol name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        let mut steps = Vec::new();

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated protocol declaration")?;
            let from = self.expect_identifier("protocol source")?;
            self.expect_symbol(TokenKind::Arrow, "`->`")?;
            let to = self.expect_identifier("protocol destination")?;
            self.expect_symbol(TokenKind::Colon, "`:`")?;
            let act = self.expect_identifier("communicative act")?;
            let message_type = self.expect_identifier("protocol message type")?;
            steps.push(ProtocolStep {
                from,
                to,
                act,
                message_type,
            });
        }
        self.advance();

        Ok(ProtocolDecl { name, steps })
    }

    fn parse_passport(&mut self) -> Result<PassportDecl, Diagnostic> {
        self.expect_keyword("passport")?;
        let name = self.expect_identifier("passport name")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;

        let mut agent = None;
        let mut agent_name = None;
        let mut global_id = None;
        let mut identity = None;
        let mut provider = None;
        let mut version = None;
        let mut ans_name = None;
        let mut country = None;
        let mut jurisdiction = None;
        let mut data_residency = None;
        let mut asn = None;
        let mut model = None;
        let mut risk_level = None;
        let mut data_scope = None;
        let mut intent = None;
        let mut intended_use = None;
        let mut prohibited_use = None;
        let mut attestations = None;

        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated passport declaration")?;
            let key = self.peek_identifier().map(str::to_owned);
            match key.as_deref() {
                Some("agent") => self.set_passport_field(&mut agent, "agent", |p| {
                    p.expect_identifier("passport agent reference")
                })?,
                Some("agent_name") => {
                    self.set_passport_field(&mut agent_name, "agent_name", |p| {
                        p.expect_string("passport agent_name value")
                    })?
                }
                Some("global_id") => self.set_passport_field(&mut global_id, "global_id", |p| {
                    p.expect_string("passport global_id value")
                })?,
                Some("identity") => self.set_passport_field(&mut identity, "identity", |p| {
                    p.expect_string("passport identity value")
                })?,
                Some("provider") => self.set_passport_field(&mut provider, "provider", |p| {
                    p.expect_string("passport provider value")
                })?,
                Some("version") => self.set_passport_field(&mut version, "version", |p| {
                    p.expect_string("passport version value")
                })?,
                Some("ans_name") => self.set_passport_field(&mut ans_name, "ans_name", |p| {
                    p.expect_string("passport ans_name value")
                })?,
                Some("country") => self.set_passport_field(&mut country, "country", |p| {
                    p.expect_string("passport country value")
                })?,
                Some("jurisdiction") => {
                    self.set_passport_field(&mut jurisdiction, "jurisdiction", |p| {
                        p.expect_string("passport jurisdiction value")
                    })?
                }
                Some("data_residency") => {
                    self.set_passport_field(&mut data_residency, "data_residency", |p| {
                        p.parse_string_array("data residency entry")
                    })?
                }
                Some("model") => self.set_passport_field(&mut model, "model", |p| {
                    p.expect_string("passport model value")
                })?,
                Some("risk_level") => {
                    self.set_passport_field(&mut risk_level, "risk_level", |p| {
                        p.expect_string("passport risk_level value")
                    })?
                }
                Some("data_scope") => {
                    self.set_passport_field(&mut data_scope, "data_scope", |p| {
                        p.parse_string_array("data scope entry")
                    })?
                }
                Some("intent") => self.set_passport_field(&mut intent, "intent", |p| {
                    p.expect_string("passport intent value")
                })?,
                Some("intended_use") => {
                    self.set_passport_field(&mut intended_use, "intended_use", |p| {
                        p.parse_string_array("intended use entry")
                    })?
                }
                Some("prohibited_use") => {
                    self.set_passport_field(&mut prohibited_use, "prohibited_use", |p| {
                        p.parse_string_array("prohibited use entry")
                    })?
                }
                Some("attestations") => {
                    self.set_passport_field(&mut attestations, "attestations", |p| {
                        p.parse_string_array("attestation entry")
                    })?
                }
                Some("asn") => {
                    if asn.is_some() {
                        return Err(Diagnostic::new(
                            "duplicate `asn` block in passport",
                            self.peek().span,
                        ));
                    }
                    asn = Some(self.parse_passport_asn()?);
                }
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected passport item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected a passport field",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        let fallback_span = name.span;
        let empty = || Spanned::new(String::new(), fallback_span);
        Ok(PassportDecl {
            agent: agent.unwrap_or_else(empty),
            agent_name: agent_name.unwrap_or_else(empty),
            global_id: global_id.unwrap_or_else(empty),
            identity: identity.unwrap_or_else(empty),
            provider: provider.unwrap_or_else(empty),
            version: version.unwrap_or_else(empty),
            ans_name,
            country: country.unwrap_or_else(empty),
            jurisdiction: jurisdiction.unwrap_or_else(empty),
            data_residency: data_residency.unwrap_or_default(),
            asn,
            model,
            risk_level: risk_level.unwrap_or_else(empty),
            data_scope: data_scope.unwrap_or_default(),
            intent: intent.unwrap_or_else(empty),
            intended_use: intended_use.unwrap_or_default(),
            prohibited_use: prohibited_use.unwrap_or_default(),
            attestations: attestations.unwrap_or_default(),
            name,
        })
    }

    fn parse_passport_asn(&mut self) -> Result<PassportAsnDecl, Diagnostic> {
        self.expect_keyword("asn")?;
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        let mut registry = None;
        let mut number = None;
        let mut holder = None;
        let mut country = None;
        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated asn block")?;
            match self.peek_identifier() {
                Some("registry") => self.set_passport_field(&mut registry, "registry", |p| {
                    p.expect_string("asn registry value")
                })?,
                Some("number") => self.set_passport_field(&mut number, "number", |p| {
                    p.expect_string("asn number value")
                })?,
                Some("holder") => self.set_passport_field(&mut holder, "holder", |p| {
                    p.expect_string("asn holder value")
                })?,
                Some("country") => self.set_passport_field(&mut country, "country", |p| {
                    p.expect_string("asn country value")
                })?,
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected asn item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => return Err(Diagnostic::new("expected an asn field", self.peek().span)),
            }
        }
        let fallback_span = self.peek().span;
        self.advance();
        let empty = || Spanned::new(String::new(), fallback_span);
        Ok(PassportAsnDecl {
            registry: registry.unwrap_or_else(empty),
            number: number.unwrap_or_else(empty),
            holder: holder.unwrap_or_else(empty),
            country: country.unwrap_or_else(empty),
        })
    }

    /// Consume a passport key keyword and parse its value, rejecting duplicates.
    fn set_passport_field<T>(
        &mut self,
        slot: &mut Option<T>,
        key: &str,
        parse_value: impl FnOnce(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<(), Diagnostic> {
        let span = self.peek().span;
        self.advance();
        if slot.is_some() {
            return Err(Diagnostic::new(
                format!("duplicate passport field `{key}`"),
                span,
            ));
        }
        *slot = Some(parse_value(self)?);
        Ok(())
    }

    fn parse_string_array(
        &mut self,
        description: &str,
    ) -> Result<Vec<Spanned<String>>, Diagnostic> {
        self.expect_symbol(TokenKind::LeftBracket, "`[`")?;
        let mut values = Vec::new();
        while !self.check(&TokenKind::RightBracket) {
            self.ensure_not_eof("unterminated string array")?;
            values.push(self.expect_string(description)?);
            if !self.check(&TokenKind::RightBracket) {
                self.expect_symbol(TokenKind::Comma, "`,`")?;
            }
        }
        self.advance();
        Ok(values)
    }

    fn expect_string(&mut self, description: &str) -> Result<Spanned<String>, Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::StringLiteral(value) = token.kind {
            self.advance();
            Ok(Spanned::new(value, token.span))
        } else {
            Err(Diagnostic::new(
                format!("expected {description}"),
                token.span,
            ))
        }
    }

    fn expect_integer(&mut self, description: &str) -> Result<Spanned<u64>, Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::IntegerLiteral(value) = token.kind {
            self.advance();
            Ok(Spanned::new(value, token.span))
        } else {
            Err(Diagnostic::new(
                format!("expected {description}"),
                token.span,
            ))
        }
    }

    fn expect_keyword(&mut self, expected: &str) -> Result<(), Diagnostic> {
        if self.match_keyword(expected) {
            Ok(())
        } else {
            Err(Diagnostic::new(
                format!("expected `{expected}`"),
                self.peek().span,
            ))
        }
    }

    fn match_keyword(&mut self, expected: &str) -> bool {
        if self.peek_identifier() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_identifier(&mut self, description: &str) -> Result<Spanned<String>, Diagnostic> {
        let token = self.peek().clone();
        if let TokenKind::Ident(value) = token.kind {
            self.advance();
            Ok(Spanned::new(value, token.span))
        } else {
            Err(Diagnostic::new(
                format!("expected {description}"),
                token.span,
            ))
        }
    }

    fn expect_bool(&mut self, description: &str) -> Result<Spanned<bool>, Diagnostic> {
        let token = self.expect_identifier(description)?;
        let value = match token.value.as_str() {
            "true" => true,
            "false" => false,
            other => {
                return Err(Diagnostic::new(
                    format!("invalid boolean `{other}`; expected `true` or `false`"),
                    token.span,
                ))
            }
        };
        Ok(Spanned::new(value, token.span))
    }

    fn expect_symbol(&mut self, expected: TokenKind, display: &str) -> Result<(), Diagnostic> {
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(Diagnostic::new(
                format!("expected {display}"),
                self.peek().span,
            ))
        }
    }

    fn ensure_not_eof(&self, message: &str) -> Result<(), Diagnostic> {
        if self.is_eof() {
            Err(Diagnostic::new(message, self.peek().span))
        } else {
            Ok(())
        }
    }

    fn check(&self, expected: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(expected)
    }

    fn peek_identifier(&self) -> Option<&str> {
        match &self.peek().kind {
            TokenKind::Ident(value) => Some(value),
            _ => None,
        }
    }

    fn peek_next_is(&self, expected: &TokenKind) -> bool {
        self.tokens.get(self.current + 1).is_some_and(|token| {
            std::mem::discriminant(&token.kind) == std::mem::discriminant(expected)
        })
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) {
        if !self.is_eof() {
            self.current += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_source;
    use crate::ast::{
        FeatureDefault, FeatureStatus, HarnessFilesystem, HarnessMode, HarnessNetwork,
        HarnessSecrets, PolicyRule, PolicyRuleDecl, PolicyViolationAction, SecretAccess,
        SecretScope, SecretSource,
    };

    #[test]
    fn parses_minimal_program() {
        let source = r#"
            module Example
            type Ping { content: string }
            agent Receiver { receives Ping }
            protocol Flow { User -> Receiver: tell Ping }
        "#;
        let program = parse_source(source).unwrap();
        assert_eq!(program.module.value, "Example");
        assert_eq!(program.types.len(), 1);
        assert_eq!(program.agents.len(), 1);
        assert_eq!(program.protocols[0].steps.len(), 1);
    }

    #[test]
    fn parses_import_statements() {
        let source = r#"
            module app.main
            import agents.research
            import policies.default
            type Ping { content: string }
            agent Receiver { receives Ping }
            protocol Flow { User -> Receiver: tell Ping }
        "#;
        let program = parse_source(source).unwrap();
        assert_eq!(program.module.value, "app.main");
        assert_eq!(program.imports.len(), 2);
        assert_eq!(program.imports[0].path.value, "agents.research");
        assert_eq!(program.imports[1].path.value, "policies.default");
    }

    #[test]
    fn rejects_import_alias() {
        let source = "module main\nimport agents.research as research\n";
        let diagnostics = parse_source(source).unwrap_err();
        assert!(diagnostics[0].message.contains("aliases are not supported"));
    }

    #[test]
    fn rejects_relative_import_syntax() {
        let source = "module main\nimport ./agents/research\n";
        // The lexer rejects `.` and `/` before the parser ever sees the path.
        assert!(parse_source(source).is_err());
    }

    #[test]
    fn rejects_invalid_module_name() {
        let source = "module agents..research\n";
        let diagnostics = parse_source(source).unwrap_err();
        assert!(diagnostics[0].message.contains("invalid module name"));
    }

    #[test]
    fn parses_policy_v2_block() {
        let program = parse_source(
            r#"
            module main
            policy ProviderSafety {
                require runtime_status completed
                deny external_execution
                on violation {
                    action block
                    trace required
                }
            }
            "#,
        )
        .unwrap();
        let policy = &program.policies[0];
        assert_eq!(policy.name.value, "ProviderSafety");
        assert!(matches!(
            policy.rules[0],
            PolicyRuleDecl::Require {
                rule: crate::span::Spanned {
                    value: PolicyRule::RuntimeStatusCompleted,
                    ..
                }
            }
        ));
        assert!(matches!(
            policy.rules[1],
            PolicyRuleDecl::Deny {
                rule: crate::span::Spanned {
                    value: PolicyRule::ExternalExecution,
                    ..
                }
            }
        ));
        let violation = policy.violation.as_ref().unwrap();
        assert_eq!(violation.action.value, PolicyViolationAction::Block);
        assert!(violation.trace_required);
    }

    #[test]
    fn preserves_unknown_policy_rule_and_action() {
        let program = parse_source(
            r#"
            module main
            policy FuturePolicy {
                require future_rule
                on violation { action future_action }
            }
            "#,
        )
        .unwrap();
        assert!(matches!(
            &program.policies[0].rules[0],
            PolicyRuleDecl::Require { rule }
                if rule.value == PolicyRule::Unknown("future_rule".into())
        ));
        assert_eq!(
            program.policies[0].violation.as_ref().unwrap().action.value,
            PolicyViolationAction::Unknown("future_action".into())
        );
    }

    #[test]
    fn rejects_duplicate_policy_violation_block() {
        let diagnostics = parse_source(
            r#"
            module main
            policy Bad {
                deny external_execution
                on violation { action warn }
                on violation { action block }
            }
            "#,
        )
        .unwrap_err();
        assert!(diagnostics[0]
            .message
            .contains("duplicate `on violation` block"));
    }

    #[test]
    fn parses_passport_block_with_asn_and_string_arrays() {
        let program = parse_source(
            r#"
            module main
            agent ResearchAgent {}
            passport RiskAnalyzerPassport {
                agent ResearchAgent
                agent_name "Risk Analyzer"
                global_id "argx:agent:01HZX9"
                identity "did:argorix:risk-v1"
                provider "Argorix"
                version "1.0.0"
                ans_name "argx://riskAnalyzer.v1.sovereign"
                country "CL"
                jurisdiction "CL"
                data_residency ["CL", "EU"]
                asn {
                    registry "LACNIC"
                    number "AS-PLACEHOLDER"
                    holder "Argorix Labs"
                    country "CL"
                }
                model "frontier-compatible"
                risk_level "high"
                data_scope ["internal", "confidential"]
                intent "risk_analysis"
                intended_use ["policy-review"]
                prohibited_use ["external-execution"]
                attestations ["redteam", "policy-check"]
            }
            "#,
        )
        .unwrap();
        let passport = &program.passports[0];
        assert_eq!(passport.name.value, "RiskAnalyzerPassport");
        assert_eq!(passport.agent.value, "ResearchAgent");
        assert_eq!(passport.agent_name.value, "Risk Analyzer");
        assert_eq!(passport.data_residency.len(), 2);
        assert_eq!(passport.data_residency[1].value, "EU");
        let asn = passport.asn.as_ref().unwrap();
        assert_eq!(asn.registry.value, "LACNIC");
        assert_eq!(asn.number.value, "AS-PLACEHOLDER");
        assert_eq!(
            passport.ans_name.as_ref().unwrap().value,
            "argx://riskAnalyzer.v1.sovereign"
        );
        assert_eq!(passport.attestations.len(), 2);
        assert_eq!(passport.intent.value, "risk_analysis");
    }

    #[test]
    fn rejects_malformed_passport_syntax() {
        // Missing quotes around a string value is a structural parser error.
        let diagnostics =
            parse_source("module main\npassport P {\n  agent_name Risk\n}\n").unwrap_err();
        assert!(diagnostics[0]
            .message
            .contains("expected passport agent_name value"));

        // Duplicate keys are rejected structurally.
        let duplicate =
            parse_source("module main\npassport P {\n  intent \"a\"\n  intent \"b\"\n}\n")
                .unwrap_err();
        assert!(duplicate[0]
            .message
            .contains("duplicate passport field `intent`"));
    }

    #[test]
    fn parses_primitive_and_legacy_message_contracts() {
        let program = parse_source(
            "module main\ntype Empty\ntype Typed { content: string approved: bool score: int confidence: float }\ntype Legacy { risk: RiskLevel }\n",
        )
        .unwrap();
        assert!(program.types[0].fields.is_empty());
        assert_eq!(program.types[1].fields.len(), 4);
        assert_eq!(
            program.types[2].fields[0].field_type.value.source_name(),
            "RiskLevel"
        );
    }

    #[test]
    fn parses_provider_harness_with_optional_metadata() {
        let program = parse_source(
            r#"
            module main
            harness OpenAIHarness {
                provider OpenAI
                mode dry_run
                network denied
                secrets denied
                filesystem read_only
                max_steps 10
                timeout_ms 1000
                input_contract UserPrompt
                output_contract DraftAnswer
                attestations ["dry-run", "policy-check"]
            }
            "#,
        )
        .unwrap();
        let harness = &program.harnesses[0];
        assert_eq!(harness.name.value, "OpenAIHarness");
        assert_eq!(harness.provider.value, "OpenAI");
        assert_eq!(harness.mode.value, HarnessMode::DryRun);
        assert_eq!(harness.network.value, HarnessNetwork::Denied);
        assert_eq!(harness.secrets.value, HarnessSecrets::Denied);
        assert_eq!(harness.filesystem.value, HarnessFilesystem::ReadOnly);
        assert_eq!(harness.max_steps.as_ref().unwrap().value, 10);
        assert_eq!(harness.timeout_ms.as_ref().unwrap().value, 1000);
        assert_eq!(harness.input_contract.as_ref().unwrap().value, "UserPrompt");
        assert_eq!(
            harness.output_contract.as_ref().unwrap().value,
            "DraftAnswer"
        );
        assert_eq!(harness.attestations.len(), 2);
    }

    #[test]
    fn preserves_unknown_harness_values_and_missing_required_sentinels() {
        let program = parse_source(
            r#"
            module main
            harness FutureHarness {
                provider OpenAI
                mode live
                attestations []
            }
            "#,
        )
        .unwrap();
        let harness = &program.harnesses[0];
        assert_eq!(harness.mode.value, HarnessMode::Unknown("live".into()));
        assert_eq!(
            harness.network.value,
            HarnessNetwork::Unknown(String::new())
        );
        assert_eq!(
            harness.secrets.value,
            HarnessSecrets::Unknown(String::new())
        );
        assert_eq!(
            harness.filesystem.value,
            HarnessFilesystem::Unknown(String::new())
        );
        assert!(harness.attestations.is_empty());
    }

    #[test]
    fn rejects_malformed_harness_syntax() {
        let duplicate = parse_source(
            "module main\nharness H { provider OpenAI provider Anthropic mode dry_run network denied secrets denied filesystem none }\n",
        )
        .unwrap_err();
        assert!(duplicate[0]
            .message
            .contains("duplicate harness field `provider`"));

        let malformed = parse_source(
            "module main\nharness H { provider OpenAI mode dry_run network denied secrets denied filesystem none max_steps \"ten\" }\n",
        )
        .unwrap_err();
        assert!(malformed[0].message.contains("integer"));
    }

    #[test]
    fn parses_provider_harness_policy_rules() {
        let program = parse_source(
            r#"
            module main
            policy HarnessPolicy {
                require provider_harness_declared
                require provider_harness_sandboxed
                require provider_network_denied
                require provider_secrets_denied
                require provider_filesystem_restricted
                require external_provider_harnessed
            }
            "#,
        )
        .unwrap();
        assert_eq!(program.policies[0].rules.len(), 6);
        assert!(matches!(
            program.policies[0].rules[0].rule().value,
            PolicyRule::ProviderHarnessDeclared
        ));
        assert!(matches!(
            program.policies[0].rules[5].rule().value,
            PolicyRule::ExternalProviderHarnessed
        ));
    }

    #[test]
    fn parses_feature_and_secret_blocks() {
        let program = parse_source(
            r#"
            module main
            feature OpenAIAdapter {
                provider OpenAI
                status experimental
                default disabled
                requires approval
                purpose "future-openai-adapter"
            }
            secret OpenAISecret {
                handle "OPENAI_API_KEY"
                provider OpenAI
                required_by OpenAIAdapter
                scope adapter
                access denied
                source none
            }
            "#,
        )
        .unwrap();
        let feature = &program.features[0];
        assert_eq!(feature.name.value, "OpenAIAdapter");
        assert_eq!(feature.provider.as_ref().unwrap().value, "OpenAI");
        assert!(matches!(feature.status.value, FeatureStatus::Experimental));
        assert!(matches!(feature.default.value, FeatureDefault::Disabled));
        assert!(feature.requires_approval);
        assert_eq!(
            feature.purpose.as_ref().unwrap().value,
            "future-openai-adapter"
        );

        let secret = &program.secrets[0];
        assert_eq!(secret.name.value, "OpenAISecret");
        assert_eq!(secret.handle.value, "OPENAI_API_KEY");
        assert_eq!(secret.required_by.as_ref().unwrap().value, "OpenAIAdapter");
        assert!(matches!(secret.scope.value, SecretScope::Adapter));
        assert!(matches!(secret.access.value, SecretAccess::Denied));
        assert!(matches!(secret.source.value, SecretSource::None));
    }

    #[test]
    fn parses_harness_feature_and_secret_references() {
        let program = parse_source(
            r#"
            module main
            harness H {
                provider OpenAI
                feature OpenAIAdapter
                secret OpenAISecret
                mode dry_run
                network denied
                secrets denied
                filesystem none
            }
            "#,
        )
        .unwrap();
        let harness = &program.harnesses[0];
        assert_eq!(harness.feature.as_ref().unwrap().value, "OpenAIAdapter");
        assert_eq!(harness.secret.as_ref().unwrap().value, "OpenAISecret");
    }

    #[test]
    fn rejects_secret_material_fields() {
        let error = parse_source(
            r#"
            module main
            secret S { handle "H" scope adapter access denied source none value "sk-leak" }
            "#,
        )
        .unwrap_err();
        assert!(error[0].message.contains("secret material"));
    }

    #[test]
    fn parses_feature_and_secret_policy_rules() {
        let program = parse_source(
            r#"
            module main
            policy P {
                require feature_flags_declared
                require features_default_disabled
                require experimental_features_require_approval
                require secret_boundaries_declared
                require secret_access_denied
                require secret_values_absent
                require external_provider_feature_gated
                require external_provider_secret_boundary_declared
            }
            "#,
        )
        .unwrap();
        assert_eq!(program.policies[0].rules.len(), 8);
        assert!(matches!(
            program.policies[0].rules[0].rule().value,
            PolicyRule::FeatureFlagsDeclared
        ));
        assert!(matches!(
            program.policies[0].rules[7].rule().value,
            PolicyRule::ExternalProviderSecretBoundaryDeclared
        ));
    }
}
