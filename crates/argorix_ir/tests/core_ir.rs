use argorix_ir::{lower_core_program, verify_core_ir};
use argorix_parser::core::parse_core_source;
use argorix_semantics::{verify_core_program, CoreCheckOptions};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/selfhost/spec")
}

#[test]
fn all_valid_core_programs_lower_verify_and_roundtrip() {
    let root = fixture_root();
    let files = [
        "valid/lexer.argx",
        "valid/parser.argx",
        "valid/symbols.argx",
        "valid/control_and_types.argx",
    ];
    let programs = files
        .iter()
        .map(|file| {
            let source = fs::read_to_string(root.join(file)).unwrap();
            parse_core_source(&source).unwrap()
        })
        .collect::<Vec<_>>();
    let available_modules = programs
        .iter()
        .map(|program| program.module.value.clone())
        .collect::<BTreeSet<_>>();

    for program in &programs {
        let checked = verify_core_program(
            program,
            &CoreCheckOptions {
                available_modules: available_modules.clone(),
            },
        )
        .unwrap();
        let ir = lower_core_program(checked);
        let before = verify_core_ir(&ir).unwrap().semantic_fingerprint();
        let serialized = serde_json::to_string_pretty(&ir).unwrap();
        let decoded = serde_json::from_str(&serialized).unwrap();
        let after = verify_core_ir(&decoded).unwrap().semantic_fingerprint();
        assert_eq!(before, after, "roundtrip changed {}", program.module.value);
    }
}
