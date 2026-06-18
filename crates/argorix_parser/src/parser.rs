use crate::{
    ast::{
        AgentDecl, Approval, AssertionDecl, CapabilityDecl, CapabilityLevel, EnumDecl, FailureDecl,
        FieldDecl, HandlerDecl, HandlerInstruction, ModelDecl, Program, ProtocolDecl, ProtocolStep,
        ProviderDecl, ProviderKindDecl, ReceiveDecl, SendDecl, ToolDecl, TypeDecl,
    },
    diagnostics::Diagnostic,
    lexer::{lex, Token, TokenKind},
    span::Spanned,
};

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

        let mut program = Program {
            module,
            providers: Vec::new(),
            assertions: Vec::new(),
            failures: Vec::new(),
            capabilities: Vec::new(),
            enums: Vec::new(),
            types: Vec::new(),
            tools: Vec::new(),
            models: Vec::new(),
            agents: Vec::new(),
            protocols: Vec::new(),
        };

        while !self.is_eof() {
            match self.peek_identifier() {
                Some("provider") => program.providers.push(self.parse_provider()?),
                Some("capability") => program.capabilities.push(self.parse_capability()?),
                Some("assert") => program.assertions.push(self.parse_assertion()?),
                Some("failure") => program.failures.push(self.parse_failure()?),
                Some("enum") => program.enums.push(self.parse_enum()?),
                Some("type") => program.types.push(self.parse_type()?),
                Some("tool") => program.tools.push(self.parse_tool()?),
                Some("model") => program.models.push(self.parse_model()?),
                Some("agent") => program.agents.push(self.parse_agent()?),
                Some("protocol") => program.protocols.push(self.parse_protocol()?),
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected top-level declaration `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected `provider`, `assert`, `failure`, `capability`, `enum`, `type`, `tool`, `model`, `agent`, or `protocol`",
                        self.peek().span,
                    ))
                }
            }
        }

        Ok(program)
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
        self.expect_symbol(TokenKind::LeftBrace, "`{`")?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            self.ensure_not_eof("unterminated type declaration")?;
            let field_name = self.expect_identifier("field name")?;
            self.expect_symbol(TokenKind::Colon, "`:`")?;
            let field_type = self.expect_identifier("field type")?;
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
}
