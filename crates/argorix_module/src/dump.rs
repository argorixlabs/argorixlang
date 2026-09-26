//! The canonical dump of a package's module graph and whole-package check
//! (ESP-018.C, `spec/language/packages.md`): what
//! `compiler/agent_package.argx` must print for the same package.

use crate::{merge_package, resolve_package};
use argorix_ir::IrProgram;
use argorix_parser::parser::{ast_dump, parse_source};
use argorix_semantics::{agent_check_dump, check_program};
use std::fmt::Write;
use std::path::Path;

/// The canonical IR dump of one agent-language file (ESP-018.D,
/// `spec/language/ir.md`): what `compiler/agent_ir.argx` must print.
///
/// - Source that does not parse or check gives the checker dump
///   (`check.md`).
/// - Otherwise the IR of `emit-ir`: `IrProgram` as serde_json's pretty JSON,
///   and a line feed.
pub fn agent_ir_dump(source: &[u8]) -> String {
    let Ok(text) = std::str::from_utf8(source) else {
        return ast_dump(source);
    };
    let Ok(program) = parse_source(text) else {
        return ast_dump(source);
    };
    if check_program(&program).is_err() {
        return agent_check_dump(source);
    }
    let ir = IrProgram::from(&program);
    let mut out = serde_json::to_string_pretty(&ir).expect("the IR serializes");
    out.push('\n');
    out
}

/// - A package that does not resolve gives one line, `error: ` and the
///   resolver's message.
/// - Otherwise `entry <name>`, a `module <name> <path>` line per module and
///   an `import <from> <to>` line per edge, both in the resolver's order;
///   then `ok`, or each diagnostic of the merged program as
///   `line:column: message`.
pub fn package_dump(manifest_path: &Path) -> String {
    let package = match resolve_package(manifest_path) {
        Ok(package) => package,
        Err(error) => return format!("error: {error}\n"),
    };
    let mut out = String::new();
    let graph = &package.graph;
    let _ = writeln!(out, "entry {}", graph.entry);
    for module in &graph.modules {
        let _ = writeln!(out, "module {} {}", module.name, module.path);
    }
    for edge in &graph.imports {
        let _ = writeln!(out, "import {} {}", edge.from, edge.to);
    }
    match check_program(&merge_package(&package)) {
        Ok(()) => out.push_str("ok\n"),
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                let _ = writeln!(out, "{diagnostic}");
            }
        }
    }
    out
}
