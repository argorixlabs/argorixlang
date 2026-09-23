//! Transitional stage0 frontend for explicitly versioned Argorix Core 0.1.
//!
//! This module is intentionally isolated from the agent-language parser. It
//! produces an AST only; ESP-007 owns lowering and executable IR.

use crate::span::{Span, Spanned};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorePhase {
    Lexical,
    Syntax,
    Resolution,
    Semantic,
}

impl fmt::Display for CorePhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Lexical => "lexical",
            Self::Syntax => "syntax",
            Self::Resolution => "resolution",
            Self::Semantic => "semantic",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreDiagnostic {
    pub phase: CorePhase,
    pub code: String,
    pub message: String,
    pub span: Span,
}

impl CoreDiagnostic {
    pub fn new(
        phase: CorePhase,
        code: impl Into<String>,
        message: impl Into<String>,
        span: Span,
    ) -> Self {
        Self {
            phase,
            code: code.into(),
            message: message.into(),
            span,
        }
    }

    pub fn render(&self, file: &str, source: &str) -> String {
        let line_text = source
            .lines()
            .nth(self.span.line.saturating_sub(1))
            .unwrap_or("");
        let marker_width = self.span.end.saturating_sub(self.span.start).max(1).min(
            line_text
                .len()
                .saturating_sub(self.span.column.saturating_sub(1))
                .max(1),
        );
        format!(
            "{file}:{}:{}: {}[{}]: {}\n  |\n{:>3} | {line_text}\n  | {}{}",
            self.span.line,
            self.span.column,
            self.phase,
            self.code,
            self.message,
            self.span.line,
            " ".repeat(self.span.column.saturating_sub(1)),
            "^".repeat(marker_width),
        )
    }
}

impl fmt::Display for CoreDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: {}[{}]: {}",
            self.span.line, self.span.column, self.phase, self.code, self.message
        )
    }
}

impl std::error::Error for CoreDiagnostic {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreTokenKind {
    Ident(String),
    Integer { value: u64, suffix: Option<String> },
    String(String),
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Comma,
    Colon,
    ColonColon,
    Semicolon,
    Dot,
    Arrow,
    FatArrow,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    EqualEqual,
    BangEqual,
    Amp,
    AmpAmp,
    Pipe,
    PipePipe,
    Caret,
    ShiftLeft,
    ShiftRight,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreToken {
    pub kind: CoreTokenKind,
    pub span: Span,
}

pub fn lex_core(source: &str) -> Result<Vec<CoreToken>, Vec<CoreDiagnostic>> {
    let mut lexer = CoreLexer::new(source);
    lexer.run();
    if lexer.diagnostics.is_empty() {
        Ok(lexer.tokens)
    } else {
        Err(lexer.diagnostics)
    }
}

struct CoreLexer<'a> {
    source: &'a str,
    offset: usize,
    line: usize,
    column: usize,
    tokens: Vec<CoreToken>,
    diagnostics: Vec<CoreDiagnostic>,
}

impl<'a> CoreLexer<'a> {
    fn new(source: &'a str) -> Self {
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
        while let Some(ch) = self.peek() {
            match ch {
                ' ' | '\t' | '\r' | '\n' => self.whitespace(),
                '/' if self.peek_n(1) == Some('/') => self.line_comment(),
                '/' if self.peek_n(1) == Some('*') => self.block_comment(),
                '0'..='9' => self.integer(),
                '"' => self.string(),
                ch if is_core_ident_start(ch) => self.identifier(),
                '{' => self.single(CoreTokenKind::LeftBrace),
                '}' => self.single(CoreTokenKind::RightBrace),
                '(' => self.single(CoreTokenKind::LeftParen),
                ')' => self.single(CoreTokenKind::RightParen),
                '[' => self.single(CoreTokenKind::LeftBracket),
                ']' => self.single(CoreTokenKind::RightBracket),
                ',' => self.single(CoreTokenKind::Comma),
                ';' => self.single(CoreTokenKind::Semicolon),
                '.' => self.single(CoreTokenKind::Dot),
                '^' => self.single(CoreTokenKind::Caret),
                ':' if self.peek_n(1) == Some(':') => self.double(CoreTokenKind::ColonColon),
                ':' => self.single(CoreTokenKind::Colon),
                '-' if self.peek_n(1) == Some('>') => self.double(CoreTokenKind::Arrow),
                '=' if self.peek_n(1) == Some('>') => self.double(CoreTokenKind::FatArrow),
                '=' if self.peek_n(1) == Some('=') => self.double(CoreTokenKind::EqualEqual),
                '!' if self.peek_n(1) == Some('=') => self.double(CoreTokenKind::BangEqual),
                '<' if self.peek_n(1) == Some('=') => self.double(CoreTokenKind::LessEqual),
                '>' if self.peek_n(1) == Some('=') => self.double(CoreTokenKind::GreaterEqual),
                '<' if self.peek_n(1) == Some('<') => self.double(CoreTokenKind::ShiftLeft),
                '>' if self.peek_n(1) == Some('>') => self.double(CoreTokenKind::ShiftRight),
                '&' if self.peek_n(1) == Some('&') => self.double(CoreTokenKind::AmpAmp),
                '|' if self.peek_n(1) == Some('|') => self.double(CoreTokenKind::PipePipe),
                '+' if self.peek_n(1) == Some('=') => self.double(CoreTokenKind::PlusAssign),
                '-' if self.peek_n(1) == Some('=') => self.double(CoreTokenKind::MinusAssign),
                '*' if self.peek_n(1) == Some('=') => self.double(CoreTokenKind::StarAssign),
                '/' if self.peek_n(1) == Some('=') => self.double(CoreTokenKind::SlashAssign),
                '%' if self.peek_n(1) == Some('=') => self.double(CoreTokenKind::PercentAssign),
                '=' => self.single(CoreTokenKind::Assign),
                '<' => self.single(CoreTokenKind::Less),
                '>' => self.single(CoreTokenKind::Greater),
                '+' => self.single(CoreTokenKind::Plus),
                '-' => self.single(CoreTokenKind::Minus),
                '*' => self.single(CoreTokenKind::Star),
                '/' => self.single(CoreTokenKind::Slash),
                '%' => self.single(CoreTokenKind::Percent),
                '!' => self.single(CoreTokenKind::Bang),
                '&' => self.single(CoreTokenKind::Amp),
                '|' => self.single(CoreTokenKind::Pipe),
                unexpected => {
                    let span = self.span_here(unexpected.len_utf8());
                    self.diagnostics.push(CoreDiagnostic::new(
                        CorePhase::Lexical,
                        "UnexpectedCharacter",
                        format!("unexpected character `{unexpected}`"),
                        span,
                    ));
                    self.advance();
                }
            }
        }
        self.tokens.push(CoreToken {
            kind: CoreTokenKind::Eof,
            span: Span::new(self.offset, self.offset, self.line, self.column),
        });
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn peek_n(&self, count: usize) -> Option<char> {
        self.source[self.offset..].chars().nth(count)
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.offset += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn span_here(&self, width: usize) -> Span {
        Span::new(self.offset, self.offset + width, self.line, self.column)
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
            self.advance();
        }
    }

    fn line_comment(&mut self) {
        while !matches!(self.peek(), None | Some('\n')) {
            self.advance();
        }
    }

    fn block_comment(&mut self) {
        let start = self.span_here(2);
        self.advance();
        self.advance();
        let mut depth = 1_u32;
        while depth > 0 {
            match (self.peek(), self.peek_n(1)) {
                (None, _) => {
                    self.diagnostics.push(CoreDiagnostic::new(
                        CorePhase::Lexical,
                        "UnterminatedComment",
                        "unterminated block comment",
                        start,
                    ));
                    return;
                }
                (Some('/'), Some('*')) => {
                    self.advance();
                    self.advance();
                    depth += 1;
                }
                (Some('*'), Some('/')) => {
                    self.advance();
                    self.advance();
                    depth -= 1;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn single(&mut self, kind: CoreTokenKind) {
        let start = self.offset;
        let line = self.line;
        let column = self.column;
        self.advance();
        self.tokens.push(CoreToken {
            kind,
            span: Span::new(start, self.offset, line, column),
        });
    }

    fn double(&mut self, kind: CoreTokenKind) {
        let start = self.offset;
        let line = self.line;
        let column = self.column;
        self.advance();
        self.advance();
        self.tokens.push(CoreToken {
            kind,
            span: Span::new(start, self.offset, line, column),
        });
    }

    fn identifier(&mut self) {
        let start = self.offset;
        let line = self.line;
        let column = self.column;
        self.advance();
        while matches!(self.peek(), Some(ch) if is_core_ident_continue(ch)) {
            self.advance();
        }
        self.tokens.push(CoreToken {
            kind: CoreTokenKind::Ident(self.source[start..self.offset].to_owned()),
            span: Span::new(start, self.offset, line, column),
        });
    }

    fn integer(&mut self) {
        let start = self.offset;
        let line = self.line;
        let column = self.column;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit() || ch == '_') {
            self.advance();
        }
        let digits_end = self.offset;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_alphanumeric()) {
            self.advance();
        }
        let raw = self.source[start..digits_end].replace('_', "");
        let suffix =
            (digits_end != self.offset).then(|| self.source[digits_end..self.offset].to_owned());
        match raw.parse::<u64>() {
            Ok(value) => self.tokens.push(CoreToken {
                kind: CoreTokenKind::Integer { value, suffix },
                span: Span::new(start, self.offset, line, column),
            }),
            Err(_) => self.diagnostics.push(CoreDiagnostic::new(
                CorePhase::Lexical,
                "IntegerOutOfRange",
                "integer literal exceeds u64",
                Span::new(start, self.offset, line, column),
            )),
        }
    }

    fn string(&mut self) {
        let start = self.offset;
        let line = self.line;
        let column = self.column;
        self.advance();
        let mut value = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => {
                    self.diagnostics.push(CoreDiagnostic::new(
                        CorePhase::Lexical,
                        "UnterminatedString",
                        "unterminated string literal",
                        Span::new(start, self.offset, line, column),
                    ));
                    return;
                }
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    let escaped = match self.advance() {
                        Some('n') => '\n',
                        Some('r') => '\r',
                        Some('t') => '\t',
                        Some('"') => '"',
                        Some('\\') => '\\',
                        Some(other) => {
                            self.diagnostics.push(CoreDiagnostic::new(
                                CorePhase::Lexical,
                                "InvalidEscape",
                                format!("invalid escape `\\{other}`"),
                                Span::new(start, self.offset, line, column),
                            ));
                            other
                        }
                        // A backslash that ends the file leaves the string
                        // open, which used to pass without any diagnostic.
                        None => {
                            self.diagnostics.push(CoreDiagnostic::new(
                                CorePhase::Lexical,
                                "UnterminatedString",
                                "unterminated string literal",
                                Span::new(start, self.offset, line, column),
                            ));
                            return;
                        }
                    };
                    value.push(escaped);
                }
                Some(ch) => {
                    self.advance();
                    value.push(ch);
                }
            }
        }
        self.tokens.push(CoreToken {
            kind: CoreTokenKind::String(value),
            span: Span::new(start, self.offset, line, column),
        });
    }
}

/// The canonical token dump of `spec/core/tokens.md`: one line per token, or
/// one line per lexical diagnostic when there is any. The Argorix lexer
/// (`compiler/lexer.argx`) must produce the same bytes for the same source.
pub fn core_token_dump(source: &[u8]) -> String {
    let text = match std::str::from_utf8(source) {
        Ok(text) => text,
        Err(error) => {
            let valid = &source[..error.valid_up_to()];
            // The prefix is valid UTF-8 by construction.
            let prefix = std::str::from_utf8(valid).unwrap_or_default();
            let line = 1 + prefix.matches('\n').count();
            let column = 1 + prefix.rsplit('\n').next().unwrap_or("").chars().count();
            return format!("{line}:{column}: lexical[InvalidUtf8]: source is not valid UTF-8\n");
        }
    };
    let mut out = String::new();
    match lex_core(text) {
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                out.push_str(&format!("{diagnostic}\n"));
            }
        }
        Ok(tokens) => {
            for token in tokens {
                let span = token.span;
                out.push_str(&format!(
                    "{}:{} {} {} ",
                    span.line, span.column, span.start, span.end
                ));
                match &token.kind {
                    CoreTokenKind::Ident(name) => out.push_str(&format!("Ident {name}")),
                    CoreTokenKind::Integer { value, suffix } => {
                        out.push_str(&format!("Integer {value}"));
                        if let Some(suffix) = suffix {
                            out.push_str(&format!(" {suffix}"));
                        }
                    }
                    CoreTokenKind::String(value) => {
                        out.push_str("String \"");
                        out.push_str(&escape_json(value));
                        out.push('"');
                    }
                    other => out.push_str(&format!("{other:?}")),
                }
                out.push('\n');
            }
        }
    }
    out
}

/// The escaping of `stdlib.text.append_json_escaped`.
fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 32 => out.push_str(&format!("\\u00{:02x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

fn is_core_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_core_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreProgram {
    pub version: Spanned<String>,
    pub module: Spanned<String>,
    pub imports: Vec<CoreImport>,
    pub items: Vec<CoreItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreImport {
    pub path: Spanned<String>,
    pub alias: Option<Spanned<String>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreItem {
    pub public: bool,
    pub kind: CoreItemKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreItemKind {
    Function(CoreFunction),
    Struct(CoreStruct),
    Enum(CoreEnum),
    Const(CoreConst),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreFunction {
    pub name: Spanned<String>,
    pub parameters: Vec<CoreParameter>,
    pub return_type: CoreType,
    pub body: CoreBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreParameter {
    pub name: Spanned<String>,
    pub ty: CoreType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreStruct {
    pub name: Spanned<String>,
    pub fields: Vec<CoreField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreField {
    pub name: Spanned<String>,
    pub ty: CoreType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreEnum {
    pub name: Spanned<String>,
    pub variants: Vec<CoreVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreVariant {
    pub name: Spanned<String>,
    pub fields: Vec<CoreField>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreConst {
    pub name: Spanned<String>,
    pub ty: CoreType,
    pub value: CoreExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreType {
    pub kind: CoreTypeKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreTypeKind {
    Named(String),
    Container {
        name: String,
        element: Box<CoreType>,
        array_length: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreBlock {
    pub statements: Vec<CoreStatement>,
    pub tail: Option<Box<CoreExpr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreStatement {
    pub kind: CoreStatementKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreStatementKind {
    Let {
        mutable: bool,
        name: Spanned<String>,
        annotation: Option<CoreType>,
        value: CoreExpr,
    },
    Assign {
        target: CoreExpr,
        operator: CoreAssignOp,
        value: CoreExpr,
    },
    While {
        condition: CoreExpr,
        body: CoreBlock,
    },
    Break(Option<CoreExpr>),
    Continue,
    Return(Option<CoreExpr>),
    Expr(CoreExpr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreAssignOp {
    Assign,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreExpr {
    pub kind: CoreExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreExprKind {
    Integer {
        value: u64,
        suffix: Option<String>,
    },
    String(String),
    Bool(bool),
    Unit,
    Path(Vec<String>),
    Aggregate {
        path: Vec<String>,
        fields: Vec<(Spanned<String>, CoreExpr)>,
    },
    Array(Vec<CoreExpr>),
    Block(CoreBlock),
    If {
        condition: Box<CoreExpr>,
        then_block: CoreBlock,
        else_expr: Option<Box<CoreExpr>>,
    },
    Match {
        value: Box<CoreExpr>,
        arms: Vec<CoreMatchArm>,
    },
    Loop(CoreBlock),
    Call {
        callee: Box<CoreExpr>,
        arguments: Vec<CoreExpr>,
    },
    Index {
        value: Box<CoreExpr>,
        index: Box<CoreExpr>,
    },
    Field {
        value: Box<CoreExpr>,
        name: Spanned<String>,
    },
    Unary {
        operator: CoreUnaryOp,
        value: Box<CoreExpr>,
    },
    Binary {
        left: Box<CoreExpr>,
        operator: CoreBinaryOp,
        right: Box<CoreExpr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreUnaryOp {
    Not,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreBinaryOp {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    BitOr,
    BitXor,
    BitAnd,
    ShiftLeft,
    ShiftRight,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreMatchArm {
    pub pattern: CorePattern,
    pub guard: Option<CoreExpr>,
    pub value: CoreExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorePattern {
    pub kind: CorePatternKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorePatternKind {
    Wildcard,
    Bool(bool),
    Integer(u64),
    Binding(String),
    Variant {
        path: Vec<String>,
        fields: Vec<(Spanned<String>, Option<Box<CorePattern>>)>,
    },
}

pub fn parse_core_source(source: &str) -> Result<CoreProgram, Vec<CoreDiagnostic>> {
    let tokens = lex_core(source)?;
    CoreParser::new(tokens).parse_program()
}

struct CoreParser {
    tokens: Vec<CoreToken>,
    current: usize,
    diagnostics: Vec<CoreDiagnostic>,
}

type ParseResult<T> = Result<T, CoreDiagnostic>;

impl CoreParser {
    fn new(tokens: Vec<CoreToken>) -> Self {
        Self {
            tokens,
            current: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_program(mut self) -> Result<CoreProgram, Vec<CoreDiagnostic>> {
        match self.parse_program_inner() {
            Ok(program) if self.diagnostics.is_empty() => Ok(program),
            Ok(_) => Err(self.diagnostics),
            Err(error) => {
                self.diagnostics.push(error);
                Err(self.diagnostics)
            }
        }
    }

    fn parse_program_inner(&mut self) -> ParseResult<CoreProgram> {
        let start = self.peek().span;
        if !self.at_ident("core") {
            return Err(self.error(
                "VersionRequired",
                "Core source must begin with `core 0.1;`",
                self.peek().span,
            ));
        }
        self.advance();
        let major = self.expect_integer("VersionRequired", "expected Core major version")?;
        self.expect_kind(
            &CoreTokenKind::Dot,
            "VersionRequired",
            "expected `.` in Core version",
        )?;
        let minor = self.expect_integer("VersionRequired", "expected Core minor version")?;
        let version_span = merge_span(major.span, minor.span);
        let version = format!("{}.{}", integer_value(&major), integer_value(&minor));
        if version != "0.1" {
            return Err(self.error(
                "VersionUnsupported",
                format!("unsupported Core version `{version}`"),
                version_span,
            ));
        }
        self.expect_kind(
            &CoreTokenKind::Semicolon,
            "ExpectedSemicolon",
            "expected `;` after Core version",
        )?;
        self.expect_ident_value("module", "ExpectedModule", "expected module declaration")?;
        let module = self.parse_dotted_path()?;
        self.expect_kind(
            &CoreTokenKind::Semicolon,
            "ExpectedSemicolon",
            "expected `;` after module",
        )?;
        let mut imports = Vec::new();
        while self.at_ident("import") {
            let import_start = self.advance().span;
            let path = self.parse_dotted_path()?;
            let alias = if self.at_ident("as") {
                self.advance();
                Some(self.expect_ident("ExpectedIdentifier", "expected import alias")?)
            } else {
                None
            };
            let end = self.expect_kind(
                &CoreTokenKind::Semicolon,
                "ExpectedSemicolon",
                "expected `;` after import",
            )?;
            imports.push(CoreImport {
                path,
                alias,
                span: merge_span(import_start, end.span),
            });
        }
        let mut items = Vec::new();
        while !self.at_kind(&CoreTokenKind::Eof) {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(error) => {
                    self.diagnostics.push(error);
                    self.synchronize_item();
                }
            }
        }
        Ok(CoreProgram {
            version: Spanned::new(version, version_span),
            module,
            imports,
            items,
            span: merge_span(start, self.peek().span),
        })
    }

    fn parse_item(&mut self) -> ParseResult<CoreItem> {
        let start = self.peek().span;
        let public = if self.at_ident("pub") {
            self.advance();
            true
        } else {
            false
        };
        let kind = if self.at_ident("fn") {
            CoreItemKind::Function(self.parse_function()?)
        } else if self.at_ident("struct") {
            CoreItemKind::Struct(self.parse_struct()?)
        } else if self.at_ident("enum") {
            CoreItemKind::Enum(self.parse_enum()?)
        } else if self.at_ident("const") {
            CoreItemKind::Const(self.parse_const()?)
        } else if self.at_ident("extern") {
            return Err(self.error(
                "ForbiddenHostEscape",
                "Core 0.1 has no `extern` or arbitrary FFI",
                self.peek().span,
            ));
        } else {
            return Err(self.error(
                "ExpectedItem",
                "expected `fn`, `struct`, `enum` or `const`",
                self.peek().span,
            ));
        };
        let end = item_end(&kind);
        Ok(CoreItem {
            public,
            kind,
            span: merge_span(start, end),
        })
    }

    fn parse_function(&mut self) -> ParseResult<CoreFunction> {
        self.advance();
        let name = self.expect_ident("ExpectedIdentifier", "expected function name")?;
        self.expect_kind(&CoreTokenKind::LeftParen, "ExpectedToken", "expected `(`")?;
        let mut parameters = Vec::new();
        if !self.at_kind(&CoreTokenKind::RightParen) {
            loop {
                let parameter_start = self.peek().span;
                let parameter_name =
                    self.expect_ident("ExpectedIdentifier", "expected parameter name")?;
                self.expect_kind(&CoreTokenKind::Colon, "ExpectedToken", "expected `:`")?;
                let ty = self.parse_type()?;
                parameters.push(CoreParameter {
                    name: parameter_name,
                    span: merge_span(parameter_start, ty.span),
                    ty,
                });
                if !self.consume_kind(&CoreTokenKind::Comma) {
                    break;
                }
                if self.at_kind(&CoreTokenKind::RightParen) {
                    break;
                }
            }
        }
        self.expect_kind(&CoreTokenKind::RightParen, "ExpectedToken", "expected `)`")?;
        self.expect_kind(&CoreTokenKind::Arrow, "ExpectedToken", "expected `->`")?;
        let return_type = self.parse_type()?;
        let body = self.parse_block()?;
        Ok(CoreFunction {
            name,
            parameters,
            return_type,
            body,
        })
    }

    fn parse_struct(&mut self) -> ParseResult<CoreStruct> {
        self.advance();
        let name = self.expect_ident("ExpectedIdentifier", "expected struct name")?;
        if self.at_kind(&CoreTokenKind::Less) {
            return Err(self.error(
                "FeatureDeferred",
                "user-defined generics are deferred in Core 0.1",
                self.peek().span,
            ));
        }
        let fields = self.parse_field_block()?;
        Ok(CoreStruct { name, fields })
    }

    fn parse_enum(&mut self) -> ParseResult<CoreEnum> {
        self.advance();
        let name = self.expect_ident("ExpectedIdentifier", "expected enum name")?;
        self.expect_kind(&CoreTokenKind::LeftBrace, "ExpectedToken", "expected `{`")?;
        let mut variants = Vec::new();
        while !self.at_kind(&CoreTokenKind::RightBrace) && !self.at_kind(&CoreTokenKind::Eof) {
            let start = self.peek().span;
            let variant_name = self.expect_ident("ExpectedIdentifier", "expected variant name")?;
            let fields = if self.at_kind(&CoreTokenKind::LeftBrace) {
                self.parse_field_block()?
            } else {
                Vec::new()
            };
            let end = self.expect_kind(
                &CoreTokenKind::Comma,
                "ExpectedComma",
                "expected `,` after enum variant",
            )?;
            variants.push(CoreVariant {
                name: variant_name,
                fields,
                span: merge_span(start, end.span),
            });
        }
        self.expect_kind(&CoreTokenKind::RightBrace, "ExpectedToken", "expected `}`")?;
        Ok(CoreEnum { name, variants })
    }

    fn parse_const(&mut self) -> ParseResult<CoreConst> {
        self.advance();
        let name = self.expect_ident("ExpectedIdentifier", "expected const name")?;
        self.expect_kind(&CoreTokenKind::Colon, "ExpectedToken", "expected `:`")?;
        let ty = self.parse_type()?;
        self.expect_kind(&CoreTokenKind::Assign, "ExpectedToken", "expected `=`")?;
        let value = self.parse_expression(true)?;
        self.expect_kind(
            &CoreTokenKind::Semicolon,
            "ExpectedSemicolon",
            "expected `;`",
        )?;
        Ok(CoreConst { name, ty, value })
    }

    fn parse_field_block(&mut self) -> ParseResult<Vec<CoreField>> {
        self.expect_kind(&CoreTokenKind::LeftBrace, "ExpectedToken", "expected `{`")?;
        let mut fields = Vec::new();
        while !self.at_kind(&CoreTokenKind::RightBrace) && !self.at_kind(&CoreTokenKind::Eof) {
            let start = self.peek().span;
            let name = self.expect_ident("ExpectedIdentifier", "expected field name")?;
            self.expect_kind(&CoreTokenKind::Colon, "ExpectedToken", "expected `:`")?;
            let ty = self.parse_type()?;
            let end = self.expect_kind(
                &CoreTokenKind::Comma,
                "ExpectedComma",
                "expected `,` after field",
            )?;
            fields.push(CoreField {
                name,
                ty,
                span: merge_span(start, end.span),
            });
        }
        self.expect_kind(&CoreTokenKind::RightBrace, "ExpectedToken", "expected `}`")?;
        Ok(fields)
    }

    fn parse_type(&mut self) -> ParseResult<CoreType> {
        let name = self.expect_ident("ExpectedType", "expected type")?;
        let start = name.span;
        if !self.consume_kind(&CoreTokenKind::Less) {
            return Ok(CoreType {
                kind: CoreTypeKind::Named(name.value),
                span: start,
            });
        }
        let intrinsic = matches!(
            name.value.as_str(),
            "Array" | "Slice" | "Buffer" | "Arena" | "Handle"
        );
        if !intrinsic {
            return Err(self.error(
                "FeatureDeferred",
                "only intrinsic containers accept type arguments in Core 0.1",
                start,
            ));
        }
        let element = self.parse_type()?;
        let array_length = if name.value == "Array" {
            self.expect_kind(
                &CoreTokenKind::Comma,
                "ExpectedComma",
                "expected array length",
            )?;
            let length = self.expect_integer("ExpectedInteger", "expected array length")?;
            Some(integer_value(&length))
        } else {
            None
        };
        let end = self.expect_type_close()?;
        Ok(CoreType {
            kind: CoreTypeKind::Container {
                name: name.value,
                element: Box::new(element),
                array_length,
            },
            span: merge_span(start, end.span),
        })
    }

    fn parse_block(&mut self) -> ParseResult<CoreBlock> {
        let start = self
            .expect_kind(&CoreTokenKind::LeftBrace, "ExpectedToken", "expected `{`")?
            .span;
        let mut statements = Vec::new();
        let mut tail = None;
        while !self.at_kind(&CoreTokenKind::RightBrace) && !self.at_kind(&CoreTokenKind::Eof) {
            if self.at_ident("let") {
                statements.push(self.parse_let()?);
            } else if self.at_ident("while") {
                statements.push(self.parse_while()?);
            } else if self.at_ident("break") {
                statements.push(self.parse_break()?);
            } else if self.at_ident("continue") {
                let token = self.advance().clone();
                let end = self.expect_kind(
                    &CoreTokenKind::Semicolon,
                    "ExpectedSemicolon",
                    "expected `;` after continue",
                )?;
                statements.push(CoreStatement {
                    kind: CoreStatementKind::Continue,
                    span: merge_span(token.span, end.span),
                });
            } else if self.at_ident("return") {
                statements.push(self.parse_return()?);
            } else {
                let expression = self.parse_expression(true)?;
                if let Some(operator) = self.consume_assign_operator() {
                    let value = self.parse_expression(true)?;
                    let end = self.expect_kind(
                        &CoreTokenKind::Semicolon,
                        "ExpectedSemicolon",
                        "expected `;` after assignment",
                    )?;
                    statements.push(CoreStatement {
                        span: merge_span(expression.span, end.span),
                        kind: CoreStatementKind::Assign {
                            target: expression,
                            operator,
                            value,
                        },
                    });
                } else if self.consume_kind(&CoreTokenKind::Semicolon) {
                    let span = expression.span;
                    statements.push(CoreStatement {
                        kind: CoreStatementKind::Expr(expression),
                        span,
                    });
                } else if self.at_kind(&CoreTokenKind::RightBrace) {
                    tail = Some(Box::new(expression));
                    break;
                } else if matches!(
                    expression.kind,
                    CoreExprKind::Block(_)
                        | CoreExprKind::If { .. }
                        | CoreExprKind::Match { .. }
                        | CoreExprKind::Loop(_)
                ) {
                    let span = expression.span;
                    statements.push(CoreStatement {
                        kind: CoreStatementKind::Expr(expression),
                        span,
                    });
                } else {
                    return Err(self.error(
                        "ExpectedSemicolon",
                        "expected `;` or end of block",
                        self.peek().span,
                    ));
                }
            }
        }
        let end = self.expect_kind(&CoreTokenKind::RightBrace, "ExpectedToken", "expected `}`")?;
        Ok(CoreBlock {
            statements,
            tail,
            span: merge_span(start, end.span),
        })
    }

    fn parse_let(&mut self) -> ParseResult<CoreStatement> {
        let start = self.advance().span;
        let mutable = if self.at_ident("mut") {
            self.advance();
            true
        } else {
            false
        };
        let name = self.expect_ident("ExpectedIdentifier", "expected binding name")?;
        let annotation = if self.consume_kind(&CoreTokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect_kind(&CoreTokenKind::Assign, "ExpectedToken", "expected `=`")?;
        let value = self.parse_expression(true)?;
        let end = self.expect_kind(
            &CoreTokenKind::Semicolon,
            "ExpectedSemicolon",
            "expected `;` after binding",
        )?;
        Ok(CoreStatement {
            kind: CoreStatementKind::Let {
                mutable,
                name,
                annotation,
                value,
            },
            span: merge_span(start, end.span),
        })
    }

    fn parse_while(&mut self) -> ParseResult<CoreStatement> {
        let start = self.advance().span;
        let condition = self.parse_expression(false)?;
        let body = self.parse_block()?;
        let span = merge_span(start, body.span);
        Ok(CoreStatement {
            kind: CoreStatementKind::While { condition, body },
            span,
        })
    }

    fn parse_break(&mut self) -> ParseResult<CoreStatement> {
        let start = self.advance().span;
        let value = if self.at_kind(&CoreTokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression(true)?)
        };
        let end = self.expect_kind(
            &CoreTokenKind::Semicolon,
            "ExpectedSemicolon",
            "expected `;` after break",
        )?;
        Ok(CoreStatement {
            kind: CoreStatementKind::Break(value),
            span: merge_span(start, end.span),
        })
    }

    fn parse_return(&mut self) -> ParseResult<CoreStatement> {
        let start = self.advance().span;
        let value = if self.at_kind(&CoreTokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression(true)?)
        };
        let end = self.expect_kind(
            &CoreTokenKind::Semicolon,
            "ExpectedSemicolon",
            "expected `;` after return",
        )?;
        Ok(CoreStatement {
            kind: CoreStatementKind::Return(value),
            span: merge_span(start, end.span),
        })
    }

    fn parse_expression(&mut self, allow_aggregate: bool) -> ParseResult<CoreExpr> {
        self.parse_binary(0, allow_aggregate)
    }

    fn parse_binary(&mut self, minimum: u8, allow_aggregate: bool) -> ParseResult<CoreExpr> {
        let mut left = self.parse_unary(allow_aggregate)?;
        while let Some((precedence, operator)) = self.binary_operator() {
            if precedence < minimum {
                break;
            }
            self.advance();
            let right = self.parse_binary(precedence + 1, allow_aggregate)?;
            let span = merge_span(left.span, right.span);
            left = CoreExpr {
                kind: CoreExprKind::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self, allow_aggregate: bool) -> ParseResult<CoreExpr> {
        if self.at_kind(&CoreTokenKind::Bang) || self.at_kind(&CoreTokenKind::Minus) {
            let token = self.advance().clone();
            let operator = if token.kind == CoreTokenKind::Bang {
                CoreUnaryOp::Not
            } else {
                CoreUnaryOp::Negate
            };
            let value = self.parse_unary(allow_aggregate)?;
            return Ok(CoreExpr {
                span: merge_span(token.span, value.span),
                kind: CoreExprKind::Unary {
                    operator,
                    value: Box::new(value),
                },
            });
        }
        self.parse_postfix(allow_aggregate)
    }

    fn parse_postfix(&mut self, allow_aggregate: bool) -> ParseResult<CoreExpr> {
        let mut expression = self.parse_primary(allow_aggregate)?;
        loop {
            if self.consume_kind(&CoreTokenKind::LeftParen) {
                let mut arguments = Vec::new();
                if !self.at_kind(&CoreTokenKind::RightParen) {
                    loop {
                        arguments.push(self.parse_expression(true)?);
                        if !self.consume_kind(&CoreTokenKind::Comma) {
                            break;
                        }
                        if self.at_kind(&CoreTokenKind::RightParen) {
                            break;
                        }
                    }
                }
                let end =
                    self.expect_kind(&CoreTokenKind::RightParen, "ExpectedToken", "expected `)`")?;
                let span = merge_span(expression.span, end.span);
                expression = CoreExpr {
                    kind: CoreExprKind::Call {
                        callee: Box::new(expression),
                        arguments,
                    },
                    span,
                };
            } else if self.consume_kind(&CoreTokenKind::LeftBracket) {
                let index = self.parse_expression(true)?;
                let end = self.expect_kind(
                    &CoreTokenKind::RightBracket,
                    "ExpectedToken",
                    "expected `]`",
                )?;
                let span = merge_span(expression.span, end.span);
                expression = CoreExpr {
                    kind: CoreExprKind::Index {
                        value: Box::new(expression),
                        index: Box::new(index),
                    },
                    span,
                };
            } else if self.consume_kind(&CoreTokenKind::Dot) {
                let name = self.expect_ident("ExpectedIdentifier", "expected field name")?;
                let span = merge_span(expression.span, name.span);
                expression = CoreExpr {
                    kind: CoreExprKind::Field {
                        value: Box::new(expression),
                        name,
                    },
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expression)
    }

    fn parse_primary(&mut self, allow_aggregate: bool) -> ParseResult<CoreExpr> {
        let token = self.advance().clone();
        let mut expression = match token.kind {
            CoreTokenKind::Integer { value, suffix } => CoreExpr {
                kind: CoreExprKind::Integer { value, suffix },
                span: token.span,
            },
            CoreTokenKind::String(value) => CoreExpr {
                kind: CoreExprKind::String(value),
                span: token.span,
            },
            CoreTokenKind::Ident(ref value) if value == "true" || value == "false" => CoreExpr {
                kind: CoreExprKind::Bool(value == "true"),
                span: token.span,
            },
            CoreTokenKind::Ident(ref value) if value == "unit" => CoreExpr {
                kind: CoreExprKind::Unit,
                span: token.span,
            },
            CoreTokenKind::Ident(ref value) if value == "if" => return self.parse_if(token.span),
            CoreTokenKind::Ident(ref value) if value == "match" => {
                return self.parse_match(token.span)
            }
            CoreTokenKind::Ident(ref value) if value == "loop" => {
                let block = self.parse_block()?;
                return Ok(CoreExpr {
                    span: merge_span(token.span, block.span),
                    kind: CoreExprKind::Loop(block),
                });
            }
            CoreTokenKind::Ident(value) => {
                let mut path = vec![value];
                let mut end = token.span;
                while self.consume_kind(&CoreTokenKind::ColonColon) {
                    let segment =
                        self.expect_ident("ExpectedIdentifier", "expected path segment")?;
                    end = segment.span;
                    path.push(segment.value);
                }
                CoreExpr {
                    kind: CoreExprKind::Path(path),
                    span: merge_span(token.span, end),
                }
            }
            CoreTokenKind::LeftParen => {
                let value = self.parse_expression(true)?;
                let end =
                    self.expect_kind(&CoreTokenKind::RightParen, "ExpectedToken", "expected `)`")?;
                CoreExpr {
                    span: merge_span(token.span, end.span),
                    ..value
                }
            }
            CoreTokenKind::LeftBracket => {
                let mut values = Vec::new();
                if !self.at_kind(&CoreTokenKind::RightBracket) {
                    loop {
                        values.push(self.parse_expression(true)?);
                        if !self.consume_kind(&CoreTokenKind::Comma) {
                            break;
                        }
                        if self.at_kind(&CoreTokenKind::RightBracket) {
                            break;
                        }
                    }
                }
                let end = self.expect_kind(
                    &CoreTokenKind::RightBracket,
                    "ExpectedToken",
                    "expected `]`",
                )?;
                CoreExpr {
                    kind: CoreExprKind::Array(values),
                    span: merge_span(token.span, end.span),
                }
            }
            CoreTokenKind::LeftBrace => {
                self.current -= 1;
                let block = self.parse_block()?;
                return Ok(CoreExpr {
                    span: block.span,
                    kind: CoreExprKind::Block(block),
                });
            }
            _ => return Err(self.error("ExpectedExpression", "expected expression", token.span)),
        };
        if allow_aggregate && self.at_kind(&CoreTokenKind::LeftBrace) {
            if let CoreExprKind::Path(path) = expression.kind {
                self.advance();
                let mut fields = Vec::new();
                while !self.at_kind(&CoreTokenKind::RightBrace) {
                    let name = self.expect_ident("ExpectedIdentifier", "expected field name")?;
                    self.expect_kind(&CoreTokenKind::Colon, "ExpectedToken", "expected `:`")?;
                    let value = self.parse_expression(true)?;
                    fields.push((name, value));
                    if !self.consume_kind(&CoreTokenKind::Comma) {
                        break;
                    }
                    if self.at_kind(&CoreTokenKind::RightBrace) {
                        break;
                    }
                }
                let end =
                    self.expect_kind(&CoreTokenKind::RightBrace, "ExpectedToken", "expected `}`")?;
                expression = CoreExpr {
                    kind: CoreExprKind::Aggregate { path, fields },
                    span: merge_span(expression.span, end.span),
                };
            }
        }
        Ok(expression)
    }

    fn parse_if(&mut self, start: Span) -> ParseResult<CoreExpr> {
        let condition = self.parse_expression(false)?;
        let then_block = self.parse_block()?;
        let else_expr = if self.at_ident("else") {
            self.advance();
            if self.at_ident("if") {
                let if_token = self.advance().span;
                Some(Box::new(self.parse_if(if_token)?))
            } else {
                let block = self.parse_block()?;
                Some(Box::new(CoreExpr {
                    span: block.span,
                    kind: CoreExprKind::Block(block),
                }))
            }
        } else {
            None
        };
        let end = else_expr
            .as_ref()
            .map(|value| value.span)
            .unwrap_or(then_block.span);
        Ok(CoreExpr {
            kind: CoreExprKind::If {
                condition: Box::new(condition),
                then_block,
                else_expr,
            },
            span: merge_span(start, end),
        })
    }

    fn parse_match(&mut self, start: Span) -> ParseResult<CoreExpr> {
        let value = self.parse_expression(false)?;
        self.expect_kind(&CoreTokenKind::LeftBrace, "ExpectedToken", "expected `{`")?;
        let mut arms = Vec::new();
        while !self.at_kind(&CoreTokenKind::RightBrace) && !self.at_kind(&CoreTokenKind::Eof) {
            let arm_start = self.peek().span;
            let pattern = self.parse_pattern()?;
            let guard = if self.at_ident("if") {
                self.advance();
                Some(self.parse_expression(true)?)
            } else {
                None
            };
            self.expect_kind(&CoreTokenKind::FatArrow, "ExpectedToken", "expected `=>`")?;
            let arm_value = if self.at_ident("return") {
                let return_start = self.advance().span;
                let value = if self.at_kind(&CoreTokenKind::Comma) {
                    None
                } else {
                    Some(self.parse_expression(true)?)
                };
                let return_end = value
                    .as_ref()
                    .map(|value| value.span)
                    .unwrap_or(return_start);
                let statement = CoreStatement {
                    kind: CoreStatementKind::Return(value),
                    span: merge_span(return_start, return_end),
                };
                CoreExpr {
                    span: statement.span,
                    kind: CoreExprKind::Block(CoreBlock {
                        statements: vec![statement],
                        tail: None,
                        span: arm_start,
                    }),
                }
            } else if self.at_kind(&CoreTokenKind::LeftBrace) {
                let block = self.parse_block()?;
                CoreExpr {
                    span: block.span,
                    kind: CoreExprKind::Block(block),
                }
            } else {
                self.parse_expression(true)?
            };
            let end = self.expect_kind(
                &CoreTokenKind::Comma,
                "ExpectedComma",
                "expected `,` after match arm",
            )?;
            arms.push(CoreMatchArm {
                pattern,
                guard,
                value: arm_value,
                span: merge_span(arm_start, end.span),
            });
        }
        let end = self.expect_kind(&CoreTokenKind::RightBrace, "ExpectedToken", "expected `}`")?;
        Ok(CoreExpr {
            kind: CoreExprKind::Match {
                value: Box::new(value),
                arms,
            },
            span: merge_span(start, end.span),
        })
    }

    fn parse_pattern(&mut self) -> ParseResult<CorePattern> {
        let token = self.advance().clone();
        let kind = match token.kind {
            CoreTokenKind::Ident(value) if value == "_" => CorePatternKind::Wildcard,
            CoreTokenKind::Ident(value) if value == "true" || value == "false" => {
                CorePatternKind::Bool(value == "true")
            }
            CoreTokenKind::Integer { value, .. } => CorePatternKind::Integer(value),
            CoreTokenKind::Ident(value) => {
                let mut path = vec![value];
                while self.consume_kind(&CoreTokenKind::ColonColon) {
                    path.push(
                        self.expect_ident("ExpectedIdentifier", "expected variant segment")?
                            .value,
                    );
                }
                if path.len() == 1 && !self.at_kind(&CoreTokenKind::LeftBrace) {
                    CorePatternKind::Binding(path.remove(0))
                } else {
                    let mut fields = Vec::new();
                    if self.consume_kind(&CoreTokenKind::LeftBrace) {
                        while !self.at_kind(&CoreTokenKind::RightBrace) {
                            let name =
                                self.expect_ident("ExpectedIdentifier", "expected pattern field")?;
                            let nested = if self.consume_kind(&CoreTokenKind::Colon) {
                                Some(Box::new(self.parse_pattern()?))
                            } else {
                                None
                            };
                            fields.push((name, nested));
                            if !self.consume_kind(&CoreTokenKind::Comma) {
                                break;
                            }
                            if self.at_kind(&CoreTokenKind::RightBrace) {
                                break;
                            }
                        }
                        self.expect_kind(
                            &CoreTokenKind::RightBrace,
                            "ExpectedToken",
                            "expected `}`",
                        )?;
                    }
                    CorePatternKind::Variant { path, fields }
                }
            }
            _ => return Err(self.error("ExpectedPattern", "expected match pattern", token.span)),
        };
        let end = self.previous().span;
        Ok(CorePattern {
            kind,
            span: merge_span(token.span, end),
        })
    }

    fn parse_dotted_path(&mut self) -> ParseResult<Spanned<String>> {
        let first = self.expect_ident("ExpectedIdentifier", "expected path")?;
        let start = first.span;
        let mut value = first.value;
        let mut end = start;
        while self.consume_kind(&CoreTokenKind::Dot) {
            let segment = self.expect_ident("ExpectedIdentifier", "expected path segment")?;
            value.push('.');
            value.push_str(&segment.value);
            end = segment.span;
        }
        Ok(Spanned::new(value, merge_span(start, end)))
    }

    fn binary_operator(&self) -> Option<(u8, CoreBinaryOp)> {
        Some(match self.peek().kind {
            CoreTokenKind::PipePipe => (1, CoreBinaryOp::Or),
            CoreTokenKind::AmpAmp => (2, CoreBinaryOp::And),
            CoreTokenKind::EqualEqual => (3, CoreBinaryOp::Equal),
            CoreTokenKind::BangEqual => (3, CoreBinaryOp::NotEqual),
            CoreTokenKind::Less => (4, CoreBinaryOp::Less),
            CoreTokenKind::LessEqual => (4, CoreBinaryOp::LessEqual),
            CoreTokenKind::Greater => (4, CoreBinaryOp::Greater),
            CoreTokenKind::GreaterEqual => (4, CoreBinaryOp::GreaterEqual),
            CoreTokenKind::Pipe => (5, CoreBinaryOp::BitOr),
            CoreTokenKind::Caret => (6, CoreBinaryOp::BitXor),
            CoreTokenKind::Amp => (7, CoreBinaryOp::BitAnd),
            CoreTokenKind::ShiftLeft => (8, CoreBinaryOp::ShiftLeft),
            CoreTokenKind::ShiftRight => (8, CoreBinaryOp::ShiftRight),
            CoreTokenKind::Plus => (9, CoreBinaryOp::Add),
            CoreTokenKind::Minus => (9, CoreBinaryOp::Subtract),
            CoreTokenKind::Star => (10, CoreBinaryOp::Multiply),
            CoreTokenKind::Slash => (10, CoreBinaryOp::Divide),
            CoreTokenKind::Percent => (10, CoreBinaryOp::Remainder),
            _ => return None,
        })
    }

    fn consume_assign_operator(&mut self) -> Option<CoreAssignOp> {
        let operator = match self.peek().kind {
            CoreTokenKind::Assign => CoreAssignOp::Assign,
            CoreTokenKind::PlusAssign => CoreAssignOp::Add,
            CoreTokenKind::MinusAssign => CoreAssignOp::Subtract,
            CoreTokenKind::StarAssign => CoreAssignOp::Multiply,
            CoreTokenKind::SlashAssign => CoreAssignOp::Divide,
            CoreTokenKind::PercentAssign => CoreAssignOp::Remainder,
            _ => return None,
        };
        self.advance();
        Some(operator)
    }

    fn synchronize_item(&mut self) {
        while !self.at_kind(&CoreTokenKind::Eof) {
            if matches!(
                self.peek().kind,
                CoreTokenKind::Ident(ref value)
                    if matches!(value.as_str(), "fn" | "struct" | "enum" | "const" | "pub")
            ) {
                return;
            }
            self.advance();
        }
    }

    fn expect_integer(
        &mut self,
        code: &'static str,
        message: &'static str,
    ) -> ParseResult<CoreToken> {
        if matches!(self.peek().kind, CoreTokenKind::Integer { .. }) {
            Ok(self.advance().clone())
        } else {
            Err(self.error(code, message, self.peek().span))
        }
    }

    fn expect_ident(
        &mut self,
        code: &'static str,
        message: &'static str,
    ) -> ParseResult<Spanned<String>> {
        let token = self.peek().clone();
        if let CoreTokenKind::Ident(value) = token.kind {
            self.advance();
            Ok(Spanned::new(value, token.span))
        } else {
            Err(self.error(code, message, token.span))
        }
    }

    fn expect_ident_value(
        &mut self,
        value: &'static str,
        code: &'static str,
        message: &'static str,
    ) -> ParseResult<CoreToken> {
        if self.at_ident(value) {
            Ok(self.advance().clone())
        } else {
            Err(self.error(code, message, self.peek().span))
        }
    }

    fn expect_kind(
        &mut self,
        kind: &CoreTokenKind,
        code: &'static str,
        message: &'static str,
    ) -> ParseResult<CoreToken> {
        if self.at_kind(kind) {
            Ok(self.advance().clone())
        } else {
            Err(self.error(code, message, self.peek().span))
        }
    }

    fn expect_type_close(&mut self) -> ParseResult<CoreToken> {
        if self.at_kind(&CoreTokenKind::Greater) {
            return Ok(self.advance().clone());
        }
        if self.at_kind(&CoreTokenKind::ShiftRight) {
            let span = self.peek().span;
            self.tokens[self.current].kind = CoreTokenKind::Greater;
            return Ok(CoreToken {
                kind: CoreTokenKind::Greater,
                span: Span::new(span.start, span.start + 1, span.line, span.column),
            });
        }
        Err(self.error("ExpectedToken", "expected `>`", self.peek().span))
    }

    fn consume_kind(&mut self, kind: &CoreTokenKind) -> bool {
        if self.at_kind(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at_kind(&self, kind: &CoreTokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn at_ident(&self, value: &str) -> bool {
        matches!(&self.peek().kind, CoreTokenKind::Ident(actual) if actual == value)
    }

    fn peek(&self) -> &CoreToken {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &CoreToken {
        &self.tokens[self.current.saturating_sub(1)]
    }

    fn advance(&mut self) -> &CoreToken {
        if !self.at_kind(&CoreTokenKind::Eof) {
            self.current += 1;
        }
        self.previous()
    }

    fn error(
        &self,
        code: impl Into<String>,
        message: impl Into<String>,
        span: Span,
    ) -> CoreDiagnostic {
        CoreDiagnostic::new(CorePhase::Syntax, code, message, span)
    }
}

fn integer_value(token: &CoreToken) -> u64 {
    match token.kind {
        CoreTokenKind::Integer { value, .. } => value,
        _ => unreachable!(),
    }
}

fn merge_span(start: Span, end: Span) -> Span {
    Span::new(start.start, end.end, start.line, start.column)
}

fn item_end(item: &CoreItemKind) -> Span {
    match item {
        CoreItemKind::Function(value) => value.body.span,
        CoreItemKind::Struct(value) => value
            .fields
            .last()
            .map(|field| field.span)
            .unwrap_or(value.name.span),
        CoreItemKind::Enum(value) => value
            .variants
            .last()
            .map(|variant| variant.span)
            .unwrap_or(value.name.span),
        CoreItemKind::Const(value) => value.value.span,
    }
}

#[cfg(test)]
mod tests {
    use super::{lex_core, parse_core_source, CoreItemKind, CoreTokenKind};

    #[test]
    fn lexer_preserves_suffixes_spans_and_nested_comments() {
        let tokens = lex_core("core 0.1; /* a /* b */ c */ const X: u8 = 7u8;").unwrap();
        assert!(tokens.iter().any(|token| matches!(
            token.kind,
            CoreTokenKind::Integer {
                value: 7,
                suffix: Some(ref suffix)
            } if suffix == "u8"
        )));
        assert!(tokens.iter().all(|token| token.span.line >= 1));
    }

    #[test]
    fn parses_versioned_function() {
        let program =
            parse_core_source("core 0.1; module sample; fn add(a: i64, b: i64) -> i64 { a + b }")
                .unwrap();
        assert_eq!(program.module.value, "sample");
        assert!(matches!(program.items[0].kind, CoreItemKind::Function(_)));
    }

    #[test]
    fn refuses_missing_or_unknown_version() {
        assert_eq!(
            parse_core_source("module sample; fn x() -> unit { unit }").unwrap_err()[0].code,
            "VersionRequired"
        );
        assert_eq!(
            parse_core_source("core 9.9; module sample; fn x() -> unit { unit }").unwrap_err()[0]
                .code,
            "VersionUnsupported"
        );
    }
}
