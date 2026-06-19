use crate::resolver::{ModuleGraph, ResolvedPackage};
use argorix_ir::{IrModule, IrModuleImport, IrProgram};
use argorix_parser::ast::Program;
use argorix_semantics::check_program;

/// Merge every module of a package into a single program for whole-package
/// semantic checking.
///
/// Top-level declarations from all modules become globally visible. The merged
/// program keeps the entry module name. Duplicate global symbols across modules
/// are detected later by the semantic checker.
pub fn merge_package(package: &ResolvedPackage) -> Program {
    let mut merged = Program {
        module: dummy_module(&package.graph.entry),
        imports: Vec::new(),
        providers: Vec::new(),
        assertions: Vec::new(),
        failures: Vec::new(),
        capabilities: Vec::new(),
        enums: Vec::new(),
        types: Vec::new(),
        tools: Vec::new(),
        models: Vec::new(),
        agents: Vec::new(),
        protocols: Vec::new(),
    };

    // Iterate modules in deterministic (sorted) order, entry first.
    let mut names: Vec<&String> = package.programs.keys().collect();
    names.sort();
    if let Some(index) = names.iter().position(|name| **name == package.graph.entry) {
        let entry = names.remove(index);
        names.insert(0, entry);
    }

    for name in names {
        let Some(program) = package.programs.get(name) else {
            continue;
        };
        merged.providers.extend(program.providers.iter().cloned());
        merged.assertions.extend(program.assertions.iter().cloned());
        merged.failures.extend(program.failures.iter().cloned());
        merged
            .capabilities
            .extend(program.capabilities.iter().cloned());
        merged.enums.extend(program.enums.iter().cloned());
        merged.types.extend(program.types.iter().cloned());
        merged.tools.extend(program.tools.iter().cloned());
        merged.models.extend(program.models.iter().cloned());
        merged.agents.extend(program.agents.iter().cloned());
        merged.protocols.extend(program.protocols.iter().cloned());
    }

    merged
}

/// Run whole-package semantic verification on the merged program.
pub fn check_package(package: &ResolvedPackage) -> Result<Program, Vec<String>> {
    let merged = merge_package(package);
    match check_program(&merged) {
        Ok(()) => Ok(merged),
        Err(diagnostics) => Err(diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()),
    }
}

/// Build an IR program for a package, attaching deterministic module metadata.
pub fn package_ir(merged: &Program, graph: &ModuleGraph) -> IrProgram {
    let mut ir = IrProgram::from(merged);
    ir.modules = graph
        .modules
        .iter()
        .map(|module| IrModule {
            name: module.name.clone(),
            path: module.path.clone(),
        })
        .collect();
    ir.imports = graph
        .imports
        .iter()
        .map(|edge| IrModuleImport {
            from: edge.from.clone(),
            to: edge.to.clone(),
        })
        .collect();
    ir
}

fn dummy_module(name: &str) -> argorix_parser::span::Spanned<String> {
    argorix_parser::span::Spanned::new(name.to_owned(), argorix_parser::span::Span::new(0, 0, 1, 1))
}
