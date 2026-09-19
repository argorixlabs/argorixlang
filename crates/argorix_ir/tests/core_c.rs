use argorix_ir::{lower_core_program, verify_core_ir, CoreCBackend, CoreIrBackend};
use argorix_parser::core::parse_core_source;
use argorix_semantics::{verify_core_program, CoreCheckOptions};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn emit(file: &str) -> String {
    let source = fs::read_to_string(root().join("tests/selfhost/runtime").join(file)).unwrap();
    let program = parse_core_source(&source).unwrap();
    let checked = verify_core_program(
        &program,
        &CoreCheckOptions {
            available_modules: BTreeSet::from([program.module.value.clone()]),
        },
    )
    .unwrap();
    let ir = lower_core_program(checked);
    let verified = verify_core_ir(&ir).unwrap();
    CoreCBackend.emit(verified).unwrap().source
}

#[test]
fn scalar_emission_is_deterministic_and_sequenced() {
    let first = emit("scalar_success.argx");
    let second = emit("scalar_success.argx");
    assert_eq!(first, second);
    assert!(first.contains("argorix_fn_fibonacci"));
    assert!(first.contains("argorix_u32_add"));
    assert!(first.contains("argorix_step(budget)"));
    assert!(!first.contains("cargo"));
    assert!(!first.contains("rustc"));
}

#[test]
fn fixed_arrays_use_value_wrappers_and_checked_indexes() {
    let source = emit("array_success.argx");
    assert!(source.contains("typedef struct { uint32_t data[3]; } argorix_array_u32_3;"));
    assert!(source.contains("argorix_bounds"));
    assert!(source.contains(".data["));
}

#[test]
fn structs_lower_to_typed_c_values() {
    let source = emit("struct_success.argx");
    assert!(source.contains("typedef struct argorix_type_Pair"));
    assert!(source.contains(".first"));
    assert!(source.contains(".second"));
}

#[test]
fn enums_lower_to_tagged_unions_and_exhaustive_match() {
    let source = emit("enum_match_success.argx");
    assert!(source.contains("typedef enum argorix_tag_Choice"));
    assert!(source.contains("union"));
    assert!(source.contains("argorix_tag_Choice_Value"));
    assert!(source.contains("NON_EXHAUSTIVE_MATCH"));
}

#[test]
fn byte_views_and_utf8_decode_use_runtime_checks() {
    let source = emit("utf8_success.argx");
    assert!(source.contains("argorix_bytes"));
    assert!(source.contains("argorix_decode_utf8"));
    assert!(source.contains(".length"));
}

#[test]
fn emitted_programs_compile_and_observe_results_when_cc_is_available() {
    let compiler = ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("C compiler unavailable; execution is covered by C-enabled CI/WSL");
        return;
    };
    let cases = [
        ("scalar_success.argx", 0, "ARGORIX_RESULT:42", ""),
        (
            "overflow_trap.argx",
            70,
            "",
            "ARGORIX_TRAP:INTEGER_OVERFLOW",
        ),
        (
            "division_trap.argx",
            70,
            "",
            "ARGORIX_TRAP:DIVISION_BY_ZERO",
        ),
        ("array_success.argx", 0, "ARGORIX_RESULT:42", ""),
        (
            "bounds_trap.argx",
            70,
            "",
            "ARGORIX_TRAP:INDEX_OUT_OF_BOUNDS",
        ),
        ("struct_success.argx", 0, "ARGORIX_RESULT:42", ""),
        ("enum_match_success.argx", 0, "ARGORIX_RESULT:42", ""),
        ("utf8_success.argx", 0, "ARGORIX_RESULT:42", ""),
        ("utf8_trap.argx", 70, "", "ARGORIX_TRAP:UTF8_INVALID"),
    ];
    let temporary = std::env::temp_dir().join(format!("argorix-core-c-{}", std::process::id()));
    fs::create_dir_all(&temporary).unwrap();
    for (index, (fixture, exit, stdout, stderr)) in cases.iter().enumerate() {
        let generated = temporary.join(format!("case-{index}.c"));
        let executable = temporary.join(format!("case-{index}"));
        fs::write(&generated, emit(fixture)).unwrap();
        let compile = Command::new(compiler)
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic"])
            .arg("-I")
            .arg(root().join("bootstrap/c"))
            .arg(root().join("bootstrap/c/argorix_core_runtime.c"))
            .arg(&generated)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "C compile failed for {fixture}: {}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(&executable).output().unwrap();
        assert_eq!(run.status.code(), Some(*exit), "wrong exit for {fixture}");
        assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), *stdout);
        assert_eq!(String::from_utf8_lossy(&run.stderr).trim(), *stderr);
    }
    fs::remove_dir_all(temporary).unwrap();
}
