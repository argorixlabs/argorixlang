pub mod ast;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
pub mod span;

pub use ast::Program;
pub use diagnostics::Diagnostic;
pub use parser::{is_valid_module_name, parse_source};
