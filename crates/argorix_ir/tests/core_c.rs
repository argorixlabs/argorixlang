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
    // Fields carry the `argorix_f_` prefix so a name like `long` or `char`
    // cannot collide with C (ESP-009.B).
    assert!(source.contains(".argorix_f_first"));
    assert!(source.contains(".argorix_f_second"));
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
    // An owning local is released through its type's drop function, and
    // only while it still owns: a moved-out local has its flag cleared.
    let drop_call = "argorix_drop_buffer_u32(&argorix_v_values);";
    assert!(
        source.contains(drop_call),
        "a Buffer local must be dropped, or its storage leaks:\n{source}"
    );
    assert!(
        source.contains("if (argorix_live_values) {"),
        "the drop must be guarded by the local's live flag:\n{source}"
    );
    assert!(
        source.contains("static void argorix_drop_buffer_u32(argorix_buffer *value) {"),
        "the drop function must be defined:\n{source}"
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

/// The programs behind these tests are the regression corpus in
/// `conformance/core_c/regression/`, which runs them end to end; here we check
/// the shape of the C the backend emits for the constructs they cover.
fn emit_regression(file: &str) -> String {
    let source =
        fs::read_to_string(root().join("conformance/core_c/regression").join(file)).unwrap();
    let program = parse_core_source(&source).unwrap();
    let checked = verify_core_program(
        &program,
        &CoreCheckOptions {
            available_modules: BTreeSet::from([program.module.value.clone()]),
        },
    )
    .unwrap();
    let ir = lower_core_program(checked);
    CoreCBackend
        .emit(verify_core_ir(&ir).unwrap())
        .unwrap()
        .source
}

#[test]
fn an_if_statement_lowers_without_a_result_temporary() {
    let source = emit_regression("g12_if_statement.argx");
    assert!(source.contains("if ("), "no if emitted:\n{source}");
    // A statement `if` has no value, so nothing declares a temporary for it.
    let if_line = source
        .lines()
        .find(|line| line.trim_start().starts_with("if ("))
        .unwrap();
    assert!(!if_line.contains('='), "statement if assigns: {if_line}");
}

#[test]
fn a_block_expression_gets_its_own_c_scope() {
    let source = emit_regression("g04_block_scope.argx");
    // The same name is declared twice, which only compiles because the block
    // expression is wrapped in braces of its own.
    assert_eq!(
        source.matches("uint32_t argorix_v_t = ").count(),
        2,
        "both declarations should be present:\n{source}"
    );
    let body = source
        .split_once("argorix_fn_argorix_main(argorix_budget *budget) {")
        .and_then(|(_, rest)| rest.split_once("\n}"))
        .map(|(body, _)| body)
        .unwrap();
    let inner = body.find("uint32_t argorix_v_t = 40U;").unwrap();
    let opening = body[..inner].rfind('{').unwrap();
    let closing = body[inner..].find('}').unwrap() + inner;
    let outer = body.find("uint32_t argorix_v_t = 2U;").unwrap();
    assert!(
        opening < inner && inner < closing && closing < outer,
        "the first declaration is not enclosed in its own block:\n{body}"
    );
}

#[test]
fn struct_fields_are_prefixed_so_c_keywords_are_safe() {
    let source = emit_regression("g07_field_named_like_c_keyword.argx");
    assert!(source.contains("argorix_f_long"));
    assert!(source.contains("argorix_f_char"));
    assert!(
        !source.contains(" long;") && !source.contains(" char;"),
        "a field reached C unprefixed:\n{source}"
    );
}

#[test]
fn a_type_is_defined_before_the_type_that_holds_it() {
    let source = emit_regression("g02_struct_in_struct.argx");
    let point = source.find("struct argorix_type_Point {").unwrap();
    let line = source.find("struct argorix_type_Line {").unwrap();
    assert!(point < line, "Line is defined before Point:\n{source}");
}

#[test]
fn shifts_and_negation_go_through_checked_helpers() {
    let shift = emit_regression("g13_shift.argx");
    assert!(
        shift.contains("argorix_u32_shr("),
        "no checked shift:\n{shift}"
    );
    let negation = emit_regression("g14_signed_negation.argx");
    assert!(
        negation.contains("argorix_i32_neg("),
        "no checked negation:\n{negation}"
    );
}

#[test]
fn every_function_charges_and_returns_the_call_depth_budget() {
    let source = emit_regression("g15_deep_recursion.argx");
    assert!(source.contains("argorix_enter(budget);"));
    assert!(source.contains("argorix_leave(budget);"));
    assert!(source.contains("ARGORIX_DEPTH_LIMIT"));
    // Every exit path gives the budget back, so a loop of calls cannot drain it.
    let returns = source.matches("    return").count();
    let leaves = source.matches("argorix_leave(budget);").count();
    assert!(
        leaves >= returns - 1,
        "{leaves} releases for {returns} returns:\n{source}"
    );
}

#[test]
fn an_unused_parameter_is_marked_used_in_plain_c() {
    let source = emit_regression("g08_unused_parameter.argx");
    assert!(source.contains("(void)argorix_v_b;"));
    assert!(source.contains("(void)argorix_fn_pick;"));
    assert!(
        !source.contains("__attribute__"),
        "the fix should stay plain C11:\n{source}"
    );
}

#[test]
fn a_stored_slice_is_refused_by_this_backend() {
    // Core accepts it; the backend's views carry no generation, so a stored
    // view could outlive the storage it points into.
    let source = "core 0.1;
module views.stored;

struct Cursor { bytes: Slice<u8>, at: u64, }

fn start(bytes: Slice<u8>) -> u64 {
    let cursor: Cursor = Cursor { bytes: bytes, at: 0u64 };
    cursor.at
}

pub fn argorix_main() -> u64 {
    let raw: Array<u8, 1> = [1u8];
    start(raw.as_slice())
}
";
    let program = parse_core_source(source).unwrap();
    let checked = verify_core_program(
        &program,
        &CoreCheckOptions {
            available_modules: BTreeSet::from([program.module.value.clone()]),
        },
    )
    .unwrap();
    let ir = lower_core_program(checked);
    let verified = verify_core_ir(&ir).unwrap();
    let error = CoreCBackend.emit(verified).unwrap_err();
    assert_eq!(error.code, "CBackendUnsupported");
    assert!(
        error.message.contains("`Cursor` holds a slice"),
        "{error:?}"
    );
}
