use argorix_parser::core::parse_core_source;
use argorix_semantics::{check_core_program, CoreCheckOptions};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn accepts_normative_core_corpus() {
    let files = [
        "lexer.argx",
        "parser.argx",
        "symbols.argx",
        "control_and_types.argx",
    ];
    let mut programs = Vec::new();
    for file in files {
        let path = root().join("tests/selfhost/spec/valid").join(file);
        let source = fs::read_to_string(&path).unwrap();
        let program = parse_core_source(&source)
            .unwrap_or_else(|errors| panic!("{}: {errors:#?}", path.display()));
        programs.push((path, program));
    }
    let available_modules = programs
        .iter()
        .map(|(_, program)| program.module.value.clone())
        .collect::<BTreeSet<_>>();
    let options = CoreCheckOptions { available_modules };
    for (path, program) in programs {
        check_core_program(&program, &options)
            .unwrap_or_else(|errors| panic!("{}: {errors:#?}", path.display()));
    }
}

#[test]
fn rejects_normative_negative_corpus_by_category() {
    let cases = [
        ("missing_core_header.argx", "VersionRequired"),
        ("unknown_version.argx", "VersionUnsupported"),
        ("type_and_return.argx", "TypeMismatch"),
        (
            "mutation_and_scope.argx",
            "ImmutableAssignmentOrUnknownName",
        ),
        ("non_exhaustive_match.argx", "NonExhaustiveMatch"),
        ("recursive_without_handle.argx", "InfiniteType"),
        ("numeric_errors.argx", "LiteralOutOfRangeOrConstantTrap"),
        ("control_outside_loop.argx", "ControlOutsideLoop"),
        ("unknown_import.argx", "ImportNotLocked"),
        ("forbidden_magic.argx", "ForbiddenHostEscape"),
        ("invalid_utf8_and_index.argx", "StringNotByteIndexable"),
        ("user_generics.argx", "FeatureDeferred"),
    ];
    let options = CoreCheckOptions::default();
    for (file, category) in cases {
        let path = root().join("tests/selfhost/spec/invalid").join(file);
        let source = fs::read_to_string(&path).unwrap();
        let codes: Vec<String> = match parse_core_source(&source) {
            Err(errors) => errors.into_iter().map(|error| error.code).collect(),
            Ok(program) => check_core_program(&program, &options)
                .expect_err("invalid case unexpectedly accepted")
                .into_iter()
                .map(|error| error.code)
                .collect(),
        };
        assert!(
            codes.iter().any(|code| code == category),
            "{} expected {category}, got {codes:?}",
            path.display()
        );
    }
}
