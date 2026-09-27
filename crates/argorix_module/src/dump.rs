//! The canonical dump of a package's module graph and whole-package check
//! (ESP-018.C, `spec/language/packages.md`): what
//! `compiler/agent_package.argx` must print for the same package.

use crate::{merge_package, resolve_package};
use argorix_bytecode::{lower_ir, source_digest};
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

/// The canonical bytecode dump of one agent-language file (ESP-018.D.2,
/// `spec/language/bytecode.md`): what `compiler/agent_ir.argx` must print.
///
/// - Source that does not parse or check gives the checker dump.
/// - Otherwise the bytecode `emit-bytecode` lowers, with the source's
///   digest, as pretty JSON and a line feed. It is not verified here: the
///   verifier's decisions are a dump of their own.
pub fn agent_bytecode_dump(source: &[u8]) -> String {
    let Ok(text) = std::str::from_utf8(source) else {
        return ast_dump(source);
    };
    let Ok(program) = parse_source(text) else {
        return ast_dump(source);
    };
    if check_program(&program).is_err() {
        return agent_check_dump(source);
    }
    let mut bytecode = lower_ir(&IrProgram::from(&program));
    bytecode.source_digest = Some(source_digest(source));
    let mut out = serde_json::to_string_pretty(&bytecode).expect("the bytecode serializes");
    out.push('\n');
    out
}

/// The canonical dump of the bytecode verifier's decision on serialized
/// bytecode (ESP-018.D.3, `spec/language/verify.md`): what
/// `argorixc verify-bytecode` decides on a `.argbc.json` file.
///
/// - Bytes that are not UTF-8: `invalid bytecode: not UTF-8`.
/// - JSON serde cannot read as `BytecodeProgram`: `invalid bytecode JSON: `
///   and serde_json's message, with its line and column.
/// - Otherwise `ok`, or each error of `verify_bytecode`, one per line.
pub fn bytecode_verify_dump(bytes: &[u8]) -> String {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return "invalid bytecode: not UTF-8\n".to_string();
    };
    let program: argorix_bytecode::BytecodeProgram = match serde_json::from_str(text) {
        Ok(program) => program,
        Err(error) => return format!("invalid bytecode JSON: {error}\n"),
    };
    match argorix_bytecode::verify_bytecode(&program) {
        Ok(()) => "ok\n".to_string(),
        Err(errors) => errors.iter().map(|error| format!("{error}\n")).collect(),
    }
}

/// The canonical bytecode dump of a package (`spec/language/bytecode.md`):
/// what `package_ir_dump` gives when there is no IR, otherwise the bytecode
/// `emit-bytecode-package` lowers, unverified and with no source digest.
pub fn package_bytecode_dump(manifest_path: &Path) -> String {
    let ir = package_ir_dump(manifest_path);
    if !ir.starts_with('{') {
        return ir;
    }
    let package = resolve_package(manifest_path).expect("the package resolved");
    let merged = merge_package(&package);
    let bytecode = lower_ir(&crate::package_ir(&merged, &package.graph));
    let mut out = serde_json::to_string_pretty(&bytecode).expect("the bytecode serializes");
    out.push('\n');
    out
}

/// The canonical IR dump of a package (`spec/language/ir.md`): the
/// resolver's error as in `package_dump`, the merged program's diagnostics,
/// or `package_ir` as pretty JSON and a line feed.
pub fn package_ir_dump(manifest_path: &Path) -> String {
    let package = match resolve_package(manifest_path) {
        Ok(package) => package,
        Err(error) => return format!("error: {error}\n"),
    };
    let merged = merge_package(&package);
    if let Err(diagnostics) = check_program(&merged) {
        let mut out = String::new();
        for diagnostic in diagnostics {
            let _ = writeln!(out, "{diagnostic}");
        }
        return out;
    }
    let ir = crate::package_ir(&merged, &package.graph);
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
