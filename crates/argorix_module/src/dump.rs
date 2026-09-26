//! The canonical dump of a package's module graph and whole-package check
//! (ESP-018.C, `spec/language/packages.md`): what
//! `compiler/agent_package.argx` must print for the same package.

use crate::{merge_package, resolve_package};
use argorix_semantics::check_program;
use std::fmt::Write;
use std::path::Path;

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
