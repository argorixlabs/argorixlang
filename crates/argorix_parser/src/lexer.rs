use crate::{diagnostics::Diagnostic, span::Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ident(String),
    StringLiteral(String),
    IntegerLiteral(u64),
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Comma,
    Colon,
    Arrow,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut lexer = Lexer::new(source);
    lexer.run();
    if lexer.diagnostics.is_empty() {
        Ok(lexer.tokens)
    } else {
        Err(lexer.diagnostics)
    }
}

struct Lexer<'source> {
    source: &'source str,
    offset: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'source> Lexer<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            offset: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn run(&mut self) {
        while let Some(character) = self.peek() {
            match character {
                ' ' | '\t' | '\r' | '\n' => self.consume_whitespace(),
                '/' if self.peek_next() == Some('/') => self.consume_comment(),
                '{' => self.single(TokenKind::LeftBrace),
                '}' => self.single(TokenKind::RightBrace),
                '(' => self.single(TokenKind::LeftParen),
                ')' => self.single(TokenKind::RightParen),
                '[' => self.single(TokenKind::LeftBracket),
                ']' => self.single(TokenKind::RightBracket),
                ',' => self.single(TokenKind::Comma),
                ':' => self.single(TokenKind::Colon),
                '"' => self.string(),
                '-' if self.peek_next() == Some('>') => self.arrow(),
                character if character.is_ascii_digit() => self.integer(),
                character if is_ident_start(character) => self.identifier(),
                unexpected => {
                    let span = Span::new(
                        self.offset,
                        self.offset + unexpected.len_utf8(),
                        self.line,
                        self.column,
                    );
                    self.diagnostics.push(Diagnostic::new(
                        format!("unexpected character `{unexpected}`"),
                        span,
                    ));
                    self.advance();
                }
            }
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.offset, self.offset, self.line, self.column),
        });
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut chars = self.source[self.offset..].chars();
        chars.next()?;
        chars.next()
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.offset += character.len_utf8();
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(character)
    }

    fn consume_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
            self.advance();
        }
    }

    fn consume_comment(&mut self) {
        while !matches!(self.peek(), None | Some('\n')) {
            self.advance();
        }
    }

    fn single(&mut self, kind: TokenKind) {
        let start = self.offset;
        let line = self.line;
        let column = self.column;
        self.advance();
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.offset, line, column),
        });
    }

    fn arrow(&mut self) {
        let start = self.offset;
        let line = self.line;
        let column = self.column;
        self.advance();
        self.advance();
        self.tokens.push(Token {
            kind: TokenKind::Arrow,
            span: Span::new(start, self.offset, line, column),
        });
    }

    fn string(&mut self) {
        let start = self.offset;
        let line = self.line;
        let column = self.column;
        self.advance(); // opening quote
        let value_start = self.offset;
        while !matches!(self.peek(), None | Some('"' | '\n')) {
            self.advance();
        }
        if self.peek() != Some('"') {
            let span = Span::new(start, self.offset, line, column);
            self.diagnostics
                .push(Diagnostic::new("unterminated string literal", span));
            return;
        }
        let value = self.source[value_start..self.offset].to_owned();
        self.advance(); // closing quote
        self.tokens.push(Token {
            kind: TokenKind::StringLiteral(value),
            span: Span::new(start, self.offset, line, column),
        });
    }

    fn identifier(&mut self) {
        let start = self.offset;
        let line = self.line;
        let column = self.column;
        self.advance();
        while matches!(self.peek(), Some(character) if is_ident_continue(character)) {
            self.advance();
        }
        let value = self.source[start..self.offset].to_owned();
        self.tokens.push(Token {
            kind: TokenKind::Ident(value),
            span: Span::new(start, self.offset, line, column),
        });
    }

    fn integer(&mut self) {
        let start = self.offset;
        let line = self.line;
        let column = self.column;
        while matches!(self.peek(), Some(character) if character.is_ascii_digit()) {
            self.advance();
        }
        let source = &self.source[start..self.offset];
        match source.parse::<u64>() {
            Ok(value) => self.tokens.push(Token {
                kind: TokenKind::IntegerLiteral(value),
                span: Span::new(start, self.offset, line, column),
            }),
            Err(_) => self.diagnostics.push(Diagnostic::new(
                "integer literal exceeds u64 range",
                Span::new(start, self.offset, line, column),
            )),
        }
    }
}

fn is_ident_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}

fn is_ident_continue(character: char) -> bool {
    character == '_' || character == '.' || character.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::{lex, TokenKind};

    #[test]
    fn lexes_comments_paths_and_arrows() {
        let tokens = lex("// comment\nUser -> Agent: tell Message\nregex.match").unwrap();
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Arrow));
        assert!(tokens
            .iter()
            .any(|token| token.kind == TokenKind::Ident("regex.match".into())));
    }

    #[test]
    fn lexes_unsigned_integer_literals() {
        let tokens = lex("max_steps 10 timeout_ms 1000").unwrap();
        assert!(tokens
            .iter()
            .any(|token| token.kind == TokenKind::IntegerLiteral(10)));
        assert!(tokens
            .iter()
            .any(|token| token.kind == TokenKind::IntegerLiteral(1000)));
    }
}
