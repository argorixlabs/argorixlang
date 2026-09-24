//! The diagnostics dumps of the Core checker, which the Argorix checker
//! (`compiler/check.argx`) and linker (`compiler/link.argx`) must reproduce
//! (ESP-012, `spec/core/check.md`).
//!
//! One line per diagnostic, `line:column: phase[Code]`, in the order the
//! checker reports them. A program that does not parse, or that checks cleanly,
//! gives the one line `parse failed` or `ok`.

use crate::core::{check_core_program, CoreCheckOptions};
use crate::core_link::check_core_package;
use argorix_parser::core::{parse_core_source, CoreDiagnostic, CoreProgram};
use std::collections::{BTreeMap, BTreeSet};

/// One module checked on its own: its locked set holds only itself, so an
/// import is `ImportNotLocked`.
pub fn core_check_dump(source: &[u8]) -> String {
    let Ok(text) = std::str::from_utf8(source) else {
        return "parse failed\n".into();
    };
    let Ok(program) = parse_core_source(text) else {
        return "parse failed\n".into();
    };
    let options = CoreCheckOptions {
        available_modules: BTreeSet::from([program.module.value.clone()]),
    };
    match check_core_program(&program, &options) {
        Ok(()) => "ok\n".into(),
        Err(diagnostics) => lines(&diagnostics, None),
    }
}

/// A package as `argorixc core-check` sees it: `files[0]` is the root and the
/// rest its locked compilation set, in the order given. A file that does not
/// parse is left out of the set, a module declared twice cannot be imported,
/// and the root's module may not be declared twice.
pub struct CorePackage {
    pub root: CoreProgram,
    pub modules: BTreeMap<String, CoreProgram>,
    pub duplicates: BTreeSet<String>,
    pub options: CoreCheckOptions,
    /// The position in `files` of the file that declares each module.
    pub positions: BTreeMap<String, usize>,
}

/// The package in `files`, or the line its dump is when it cannot be read:
/// `parse failed` or `root module declared twice`.
pub fn core_package(files: &[Vec<u8>]) -> Result<CorePackage, &'static str> {
    let parse = |source: &[u8]| -> Option<CoreProgram> {
        parse_core_source(std::str::from_utf8(source).ok()?).ok()
    };
    let Some(root) = files.first().and_then(|source| parse(source)) else {
        return Err("parse failed");
    };
    let root_name = root.module.value.clone();
    let mut positions = BTreeMap::from([(root_name.clone(), 0_usize)]);
    let mut modules = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for (position, source) in files.iter().enumerate().skip(1) {
        let Some(program) = parse(source) else {
            continue;
        };
        let name = program.module.value.clone();
        if name == root_name || modules.contains_key(&name) {
            duplicates.insert(name);
            continue;
        }
        positions.insert(name.clone(), position);
        modules.insert(name, program);
    }
    if duplicates.contains(&root_name) {
        return Err("root module declared twice");
    }
    let mut available_modules: BTreeSet<String> = modules.keys().cloned().collect();
    available_modules.insert(root_name);
    Ok(CorePackage {
        root,
        modules,
        duplicates,
        options: CoreCheckOptions { available_modules },
        positions,
    })
}

/// A package (see `core_package`), checked as `argorixc core-check` checks
/// it. Each line is prefixed with the position in `files` of the file it
/// points into.
pub fn core_package_check_dump(files: &[Vec<u8>]) -> String {
    let package = match core_package(files) {
        Ok(package) => package,
        Err(line) => return format!("{line}\n"),
    };
    match check_core_package(
        &package.root,
        &package.modules,
        &package.duplicates,
        &package.options,
    ) {
        Ok(_) => "ok\n".into(),
        Err(error) => lines(&error.diagnostics, Some(package.positions[&error.module])),
    }
}

fn lines(diagnostics: &[CoreDiagnostic], file: Option<usize>) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let prefix = file.map(|file| format!("{file}:")).unwrap_or_default();
            format!(
                "{prefix}{}:{}: {}[{}]\n",
                diagnostic.span.line, diagnostic.span.column, diagnostic.phase, diagnostic.code
            )
        })
        .collect()
}
