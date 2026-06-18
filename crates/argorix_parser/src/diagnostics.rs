use crate::span::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
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
        let padding = " ".repeat(self.span.column.saturating_sub(1));
        let marker = "^".repeat(marker_width);

        format!(
            "{file}:{}:{}: error: {}\n  |\n{:>3} | {line_text}\n  | {padding}{marker}",
            self.span.line, self.span.column, self.message, self.span.line
        )
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}: {}",
            self.span.line, self.span.column, self.message
        )
    }
}

impl std::error::Error for Diagnostic {}
