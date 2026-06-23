use argorix_module::{check_package, package_ir, resolve_package, ModuleError};
use std::path::PathBuf;

fn examples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn manifest(relative: &str) -> PathBuf {
    examples().join(relative)
}

#[test]
fn resolves_module_project_graph_deterministically() {
    let package = resolve_package(&manifest("module_project/argorix.toml")).unwrap();
    assert_eq!(package.graph.entry, "app.main");

    let names: Vec<&str> = package
        .graph
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec![
            "agents.research",
            "agents.reviewer",
            "app.main",
            "policies.default",
            "providers.contracts",
            "tools.search",
        ]
    );

    let research = package
        .graph
        .modules
        .iter()
        .find(|module| module.name == "agents.research")
        .unwrap();
    assert_eq!(research.path, "src/agents/research.argx");

    let edges: Vec<(&str, &str)> = package
        .graph
        .imports
        .iter()
        .map(|edge| (edge.from.as_str(), edge.to.as_str()))
        .collect();
    assert!(edges.contains(&("app.main", "agents.research")));
    assert!(edges.contains(&("app.main", "tools.search")));
    assert_eq!(edges.len(), 5);
}

#[test]
fn checks_module_project_semantics() {
    let package = resolve_package(&manifest("module_project/argorix.toml")).unwrap();
    let merged = check_package(&package).expect("multi-file semantics must pass");
    // Agents and protocol drawn from imported modules become globally visible.
    assert_eq!(merged.agents.len(), 3);
    assert_eq!(merged.protocols.len(), 1);
}

#[test]
fn package_ir_carries_module_metadata() {
    let package = resolve_package(&manifest("module_project/argorix.toml")).unwrap();
    let merged = check_package(&package).unwrap();
    let ir = package_ir(&merged, &package.graph);
    assert_eq!(ir.ir_version, "0.29");
    assert_eq!(ir.module, "app.main");
    assert_eq!(ir.modules.len(), 6);
    assert_eq!(ir.imports.len(), 5);
}

#[test]
fn rejects_unknown_import() {
    let error =
        resolve_package(&manifest("invalid_modules/unknown_import/argorix.toml")).unwrap_err();
    assert!(matches!(error, ModuleError::UnknownImport { .. }));
}

#[test]
fn rejects_cyclic_import() {
    let error =
        resolve_package(&manifest("invalid_modules/cyclic_import/argorix.toml")).unwrap_err();
    assert!(matches!(error, ModuleError::CyclicImport(_)));
}

#[test]
fn rejects_duplicate_module() {
    let error =
        resolve_package(&manifest("invalid_modules/duplicate_module/argorix.toml")).unwrap_err();
    assert!(matches!(error, ModuleError::DuplicateModule(_)));
}

#[test]
fn rejects_module_path_mismatch() {
    let error = resolve_package(&manifest(
        "invalid_modules/module_path_mismatch/argorix.toml",
    ))
    .unwrap_err();
    assert!(matches!(error, ModuleError::ModulePathMismatch { .. }));
}

#[test]
fn rejects_import_outside_root() {
    let error = resolve_package(&manifest(
        "invalid_modules/import_outside_root/argorix.toml",
    ))
    .unwrap_err();
    assert!(matches!(error, ModuleError::ImportOutsideRoot(_)));
}

#[test]
fn rejects_missing_entry() {
    let error =
        resolve_package(&manifest("invalid_modules/missing_entry/argorix.toml")).unwrap_err();
    assert!(matches!(error, ModuleError::ManifestParse(_)));
}

#[test]
fn rejects_duplicate_global_symbol_across_modules() {
    let package = resolve_package(&manifest(
        "invalid_modules/duplicate_global_symbol/argorix.toml",
    ))
    .unwrap();
    let errors = check_package(&package).unwrap_err();
    assert!(errors
        .iter()
        .any(|message| message.contains("duplicate type")));
}

#[test]
fn diagnostics_contain_no_absolute_paths() {
    let error =
        resolve_package(&manifest("invalid_modules/unknown_import/argorix.toml")).unwrap_err();
    let message = error.to_string();
    assert!(!message.contains(':') || !message.contains('\\'));
}

#[test]
fn resolves_and_checks_atrust_handshake_package() {
    let package = resolve_package(&manifest("atrust_handshake_project/argorix.toml")).unwrap();
    assert_eq!(package.graph.entry, "app.main");

    let merged = check_package(&package).expect("handshake package checks");
    assert_eq!(merged.atrust_handshakes.len(), 1);
    assert_eq!(merged.atrust_handshakes[0].name.value, "ResearchHandshake");

    // The dry-run handshake metadata survives merge + IR construction at 0.29.
    let ir = package_ir(&merged, &package.graph);
    assert_eq!(ir.ir_version, "0.29");
    assert_eq!(ir.atrust_handshakes.len(), 1);
    let hs = &ir.atrust_handshakes[0];
    assert_eq!(hs.initiator, "ResearchAgent");
    assert_eq!(hs.responder, "VerifierAgent");
    assert_eq!(hs.mode, "dry_run");
    assert_eq!(hs.network, "denied");
    assert_eq!(hs.credential_contracts, vec!["ResearchCredential"]);
}
