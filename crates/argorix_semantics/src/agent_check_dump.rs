//! The canonical dump of the agent-language checker (ESP-018.C,
//! `spec/language/check.md`): what `compiler/agent_check.argx` must print for
//! the same bytes.

use crate::checker::check_program;
use argorix_parser::parser::{ast_dump, parse_source};

/// - Source that does not lex or parse gives the syntax-tree dump's lines:
///   its first error.
/// - A program the checker accepts gives the single line `ok`.
/// - Otherwise each diagnostic is one line, `line:column: message`, in the
///   order the checker reports them.
pub fn agent_check_dump(source: &[u8]) -> String {
    let Ok(text) = std::str::from_utf8(source) else {
        return ast_dump(source);
    };
    let Ok(program) = parse_source(text) else {
        return ast_dump(source);
    };
    match check_program(&program) {
        Ok(()) => "ok\n".to_string(),
        Err(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| format!("{diagnostic}\n"))
            .collect(),
    }
}
