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
        harnesses: Vec::new(),
        features: Vec::new(),
        secrets: Vec::new(),
        adapters: Vec::new(),
        adapter_profiles: Vec::new(),
        cryptos: Vec::new(),
        crypto_boundaries: Vec::new(),
        did_methods: Vec::new(),
        atrust_boundaries: Vec::new(),
        atrust_identities: Vec::new(),
        atrust_credential_contracts: Vec::new(),
        atrust_handshakes: Vec::new(),
        trust_ledgers: Vec::new(),
        assertions: Vec::new(),
        policies: Vec::new(),
        failures: Vec::new(),
        capabilities: Vec::new(),
        enums: Vec::new(),
        types: Vec::new(),
        tools: Vec::new(),
        models: Vec::new(),
        agents: Vec::new(),
        protocols: Vec::new(),
        passports: Vec::new(),
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
        merged.harnesses.extend(program.harnesses.iter().cloned());
        merged.features.extend(program.features.iter().cloned());
        merged.secrets.extend(program.secrets.iter().cloned());
        merged.adapters.extend(program.adapters.iter().cloned());
        merged
            .adapter_profiles
            .extend(program.adapter_profiles.iter().cloned());
        merged.cryptos.extend(program.cryptos.iter().cloned());
        merged
            .crypto_boundaries
            .extend(program.crypto_boundaries.iter().cloned());
        merged
            .did_methods
            .extend(program.did_methods.iter().cloned());
        merged
            .atrust_boundaries
            .extend(program.atrust_boundaries.iter().cloned());
        merged
            .atrust_identities
            .extend(program.atrust_identities.iter().cloned());
        merged
            .atrust_credential_contracts
            .extend(program.atrust_credential_contracts.iter().cloned());
        merged
            .atrust_handshakes
            .extend(program.atrust_handshakes.iter().cloned());
        merged
            .trust_ledgers
            .extend(program.trust_ledgers.iter().cloned());
        merged.assertions.extend(program.assertions.iter().cloned());
        merged.policies.extend(program.policies.iter().cloned());
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
        merged.passports.extend(program.passports.iter().cloned());
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

#[cfg(test)]
mod tests {
    use super::{check_package, merge_package};
    use crate::resolver::{ModuleGraph, ResolvedModule, ResolvedPackage};
    use argorix_parser::parse_source;
    use std::collections::BTreeMap;

    fn package(sources: &[(&str, &str)]) -> ResolvedPackage {
        let programs = sources
            .iter()
            .map(|(name, source)| ((*name).to_owned(), parse_source(source).unwrap()))
            .collect::<BTreeMap<_, _>>();
        ResolvedPackage {
            graph: ModuleGraph {
                entry: "main".into(),
                modules: sources
                    .iter()
                    .map(|(name, source)| ResolvedModule {
                        name: (*name).into(),
                        path: format!("src/{}.argx", name.replace('.', "/")),
                        source: (*source).into(),
                    })
                    .collect(),
                imports: vec![],
            },
            programs,
        }
    }

    #[test]
    fn merges_imported_policy_declarations() {
        let package = package(&[
            ("main", "module main\nimport policies.default"),
            (
                "policies.default",
                "module policies.default\npolicy RuntimeSafety { require no_unhandled_messages }",
            ),
        ]);
        let merged = merge_package(&package);
        assert_eq!(merged.policies.len(), 1);
        assert_eq!(merged.policies[0].name.value, "RuntimeSafety");
    }

    #[test]
    fn rejects_duplicate_policy_names_across_modules() {
        let package = package(&[
            (
                "main",
                "module main\npolicy Shared { deny external_execution }",
            ),
            (
                "policies.default",
                "module policies.default\npolicy Shared { require no_unhandled_messages }",
            ),
        ]);
        let messages = check_package(&package).unwrap_err();
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate policy `Shared`")));
    }

    #[test]
    fn merges_imported_harnesses_deterministically_and_rejects_duplicates() {
        let harness_package = package(&[
            (
                "main",
                "module main\nprovider OpenAI { kind external enabled false dry_run_only true requires feature_flag requires approval }\nharness MainHarness { provider OpenAI mode dry_run network denied secrets denied filesystem none }",
            ),
            (
                "harnesses.openai",
                "module harnesses.openai\nharness ImportedHarness { provider OpenAI mode simulated network denied secrets denied filesystem read_only }",
            ),
        ]);
        let merged = merge_package(&harness_package);
        assert_eq!(
            merged
                .harnesses
                .iter()
                .map(|harness| harness.name.value.as_str())
                .collect::<Vec<_>>(),
            vec!["MainHarness", "ImportedHarness"]
        );

        let duplicate = package(&[
            (
                "main",
                "module main\nprovider OpenAI { kind external enabled false dry_run_only true requires feature_flag requires approval }\nharness Shared { provider OpenAI mode dry_run network denied secrets denied filesystem none }",
            ),
            (
                "harnesses.openai",
                "module harnesses.openai\nharness Shared { provider OpenAI mode dry_run network denied secrets denied filesystem none }",
            ),
        ]);
        let messages = check_package(&duplicate).unwrap_err();
        assert!(messages
            .iter()
            .any(|message| message.contains("duplicate harness `Shared`")));
    }
}
