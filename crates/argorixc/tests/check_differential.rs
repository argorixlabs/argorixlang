//! ESP-012.A: the Argorix checker (`compiler/check.argx`) against the stage0 one.
//!
//! `tests/selfhost/check/check_files.argx` is compiled through the transitional
//! C backend and run with a package root holding every sample. It reads each
//! sample through the compiler-host boundary, checks it with the Argorix checker
//! and writes its diagnostics dump (`spec/core/check.md`). Each dump must
//! equal, byte for byte, what the stage0 checker produces (`argorixc
//! core-check-dump`) without the codes of ESP-012.B. The samples are the cases in
//! `tests/selfhost/check/samples/`, the parser and lexer samples, the compiler's own
//! sources, the standard library, the stdlib, runtime and regression fixtures
//! and the Core spec corpus.

// The differential needs a Unix C toolchain; elsewhere this file is empty.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use argorix_semantics::core_check_dump;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Codes of ESP-012.B (ownership and views), which the Argorix checker does not
/// report yet.
const LATER_CODES: [&str; 7] = [
    "UseAfterMove",
    "MoveInLoop",
    "MoveOutOfPlace",
    "ResourceTemporary",
    "ResourceInArena",
    "SliceEscapes",
    "SliceAliasesMove",
];

/// The dump without the lines of `LATER_CODES`; `ok` if nothing is left.
fn without_later_codes(dump: &str) -> String {
    let kept: String = dump
        .lines()
        .filter(|line| {
            !LATER_CODES
                .iter()
                .any(|code| line.ends_with(&format!("[{code}]")))
        })
        .map(|line| format!("{line}\n"))
        .collect();
    if kept.is_empty() {
        "ok\n".into()
    } else {
        kept
    }
}

/// The stage0 dump, on a stack as large as `argorixc` gives itself: the
/// deep-nesting samples recurse further than a test thread's default stack.
fn on_large_stack(source: &[u8]) -> String {
    let source = source.to_vec();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || without_later_codes(&core_check_dump(&source)))
        .unwrap()
        .join()
        .unwrap()
}

fn samples() -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    let directories = [
        "compiler",
        "stdlib",
        "tests/selfhost/lexer/samples",
        "tests/selfhost/check/samples",
        "tests/selfhost/parser/samples",
        "tests/selfhost/runtime",
        "tests/selfhost/stdlib",
        "conformance/core_c/regression",
        "tests/selfhost/spec/invalid",
        "tests/selfhost/spec/valid",
    ];
    for directory in directories {
        let mut entries: Vec<PathBuf> = fs::read_dir(root().join(directory))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.is_file()
                    && matches!(
                        path.extension().and_then(|extension| extension.to_str()),
                        Some("argx" | "src")
                    )
            })
            .collect();
        entries.sort();
        for path in entries {
            let name = format!(
                "{directory}/{}",
                path.file_name().unwrap().to_string_lossy()
            );
            found.push((name, fs::read(&path).unwrap()));
        }
    }
    found
}

#[cfg(unix)]
#[test]
fn the_argorix_checker_matches_the_stage0_checker_when_cc_is_available() {
    let compiler = ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("C compiler unavailable; the differential runs in C-enabled CI");
        return;
    };
    let samples = samples();
    assert!(samples
        .iter()
        .any(|(name, _)| name == "compiler/lexer.argx"));

    let work = env::temp_dir().join(format!("argorix-check-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let package = work.join("package");
    let build = work.join("build");
    fs::create_dir_all(package.join("samples")).unwrap();
    fs::create_dir_all(build.join("dumps")).unwrap();
    // The samples are copied under flat names: the listed paths are what the
    // Argorix program reads, and they must be normal relative paths.
    let mut list = String::new();
    let mut total = 0_u64;
    for (index, (_, source)) in samples.iter().enumerate() {
        let file = format!("samples/{index}.src");
        fs::write(package.join(&file), source).unwrap();
        list.push_str(&file);
        list.push('\n');
        total += source.len() as u64;
    }
    fs::write(package.join("files.txt"), &list).unwrap();

    let c_file = work.join("check_files.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join("tests/selfhost/check/check_files.argx"))
        .arg("--output")
        .arg(&c_file)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "emission failed: {}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let executable = work.join("check_files");
    // Checking every sample takes more than the default test budget of steps.
    let compile = Command::new(compiler)
        .args(["-std=c11", "-O1", "-Wall", "-Wextra", "-Werror"])
        .arg("-DARGORIX_STEP_LIMIT=4000000000ULL")
        // The trees of the largest sources pass the 1 MiB default per buffer.
        .arg("-DARGORIX_BUFFER_LIMIT_BYTES=268435456U")
        .arg("-I")
        .arg(root().join("bootstrap/c"))
        .arg(root().join("bootstrap/c/argorix_core_runtime.c"))
        .arg(&c_file)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(&executable)
        .arg("--package-root")
        .arg(&package)
        .arg("--read-budget")
        .arg((total + list.len() as u64).to_string())
        .arg("--build-root")
        .arg(&build)
        .args(["--write-budget", "100000000"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        format!("ARGORIX_RESULT:{}", samples.len()),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let mut failed = Vec::new();
    for (index, (name, source)) in samples.iter().enumerate() {
        let expected = on_large_stack(source);
        let actual = fs::read(build.join(format!("dumps/{index}.check"))).unwrap();
        if actual != expected.as_bytes() {
            let first = expected
                .lines()
                .zip(String::from_utf8_lossy(&actual).lines())
                .find(|(want, got)| want != got)
                .map(|(want, got)| format!("expected `{want}`, got `{got}`"))
                .unwrap_or_else(|| "the dumps differ in length".into());
            failed.push(format!("{name}: {first}"));
        }
    }
    assert!(
        failed.is_empty(),
        "the Argorix checker disagrees:\n{}",
        failed.join("\n")
    );
    fs::remove_dir_all(work).unwrap();
}
