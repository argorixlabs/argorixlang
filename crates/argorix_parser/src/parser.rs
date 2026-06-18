use crate::{
    ast::{
        AgentDecl, Approval, CapabilityDecl, CapabilityLevel, EnumDecl, FieldDecl, HandlerDecl,
        HandlerInstruction, Program, ProtocolDecl, ProtocolStep, ReceiveDecl, SendDecl, TypeDecl,
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
            capabilities: Vec::new(),
            enums: Vec::new(),
            types: Vec::new(),
            agents: Vec::new(),
            protocols: Vec::new(),
        };

        while !self.is_eof() {
            match self.peek_identifier() {
                Some("capability") => program.capabilities.push(self.parse_capability()?),
                Some("enum") => program.enums.push(self.parse_enum()?),
                Some("type") => program.types.push(self.parse_type()?),
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
                        "expected `capability`, `enum`, `type`, `agent`, or `protocol`",
                        self.peek().span,
                    ))
                }
            }
        }

        Ok(program)
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
                Some("on") => handlers.push(self.parse_handler()?),
                Some(other) => {
                    return Err(Diagnostic::new(
                        format!("unexpected agent item `{other}`"),
                        self.peek().span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        "expected `security`, `receives`, `sends`, `capabilities`, or `on`",
                        self.peek().span,
                    ))
                }
            }
        }
        self.advance();

        Ok(AgentDecl {
            name,
            approval,
            receives,
            sends,
            capabilities,
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
