use crate::{
    ast::{
        AgentDecl, Approval, AssertionDecl, CapabilityDecl, CapabilityLevel, EnumDecl, FailureDecl,
        FieldDecl, HandlerDecl, HandlerInstruction, ImportDecl, MessageFieldType, ModelDecl,
        PassportAsnDecl, PassportDecl, PolicyDecl, PolicyRule, PolicyRuleDecl,
        PolicyViolationAction, PolicyViolationDecl, Program, ProtocolDecl, ProtocolStep,
        ProviderDecl, ProviderKindDecl, ReceiveDecl, SendDecl, ToolDecl, TypeDecl,
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
                        "expected `import`, `provider`, `assert`, `policy`, `failure`, `capability`, `enum`, `type`, `tool`, `model`, `agent`, `protocol`, or `passport`",
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
    use crate::ast::{PolicyRule, PolicyRuleDecl, PolicyViolationAction};

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
}
