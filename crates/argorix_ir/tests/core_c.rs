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
fn buffers_use_bounded_c1_storage_and_checked_indexes() {
    let source = emit("buffer_success.argx");
    assert!(source.contains("argorix_buffer_new"));
    assert!(source.contains("argorix_buffer_push"));
    assert!(source.contains("ARGORIX_BUFFER_LIMIT_BYTES"));
    assert!(source.contains("argorix_bounds"));
}

#[test]
fn arenas_allocate_canonical_handles_and_validate_lifetime() {
    let source = emit("arena_success.argx");
    assert!(source.contains("argorix_arena_new"));
    assert!(source.contains("argorix_arena_alloc"));
    assert!(source.contains("argorix_handle_get"));
    assert!(source.contains("argorix_arena_release"));
    assert!(source.contains("ARGORIX_ARENA_SLOT_LIMIT"));
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
        ("step_limit_trap.argx", 70, "", "ARGORIX_TRAP:STEP_LIMIT"),
        ("buffer_success.argx", 0, "ARGORIX_RESULT:42", ""),
        (
            "buffer_limit_trap.argx",
            70,
            "",
            "ARGORIX_TRAP:RESOURCE_LIMIT",
        ),
        ("arena_success.argx", 0, "ARGORIX_RESULT:42", ""),
        (
            "arena_released_trap.argx",
            70,
            "",
            "ARGORIX_TRAP:ARENA_RELEASED",
        ),
        (
            "arena_limit_trap.argx",
            70,
            "",
            "ARGORIX_TRAP:RESOURCE_LIMIT",
        ),
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

#[test]
fn c1_runtime_validates_handle_generation_and_arena_lifetime_when_cc_is_available() {
    let compiler = ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("C compiler unavailable; runtime validation is covered by C-enabled CI");
        return;
    };
    let temporary =
        std::env::temp_dir().join(format!("argorix-core-c-runtime-{}", std::process::id()));
    fs::create_dir_all(&temporary).unwrap();
    let executable = temporary.join("runtime-selftest");
    let compile = Command::new(compiler)
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(root().join("bootstrap/c"))
        .arg(root().join("bootstrap/c/argorix_core_runtime.c"))
        .arg(root().join("bootstrap/c/runtime_selftest.c"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "C1 runtime compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let cases = [
        ("handle-ok", 0, "ARGORIX_RESULT:HANDLE_OK", ""),
        ("handle-generation", 70, "", "ARGORIX_TRAP:USE_AFTER_FREE"),
        ("arena-released", 70, "", "ARGORIX_TRAP:ARENA_RELEASED"),
    ];
    for (case, exit, stdout, stderr) in cases {
        let run = Command::new(&executable).arg(case).output().unwrap();
        assert_eq!(run.status.code(), Some(exit), "wrong exit for {case}");
        assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), stdout);
        assert_eq!(String::from_utf8_lossy(&run.stderr).trim(), stderr);
    }
    fs::remove_dir_all(temporary).unwrap();
}

#[test]
fn buffers_are_released_before_every_return() {
    let source = emit("buffer_success.argx");
    let drop_call = "argorix_buffer_drop(&argorix_v_values);";
    assert!(
        source.contains(drop_call),
        "a Buffer local must be dropped, or its storage leaks:\n{source}"
    );
    // The drop has to precede the return that leaves the function, and the
    // result is already in a temporary by then.
    let dropped = source.find(drop_call).unwrap();
    let returned = source[dropped..]
        .find("return argorix_t_")
        .expect("the function returns after dropping");
    assert!(
        source[dropped..dropped + returned]
            .lines()
            .all(|line| !line.contains("argorix_buffer_push")),
        "nothing may use the buffer after it is dropped:\n{source}"
    );
}
