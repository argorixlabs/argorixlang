//! ESP-010: the Argorix lexer (`compiler/lexer.argx`) against the stage0 one.
//!
//! `tests/selfhost/lexer/lex_files.argx` is compiled through the transitional
//! C backend and run with a package root holding every sample. It reads each
//! sample through the compiler-host boundary, lexes it with the Argorix lexer
//! and writes its canonical token dump (`spec/core/tokens.md`). Each dump must
//! equal, byte for byte, what the stage0 lexer produces (`argorixc
//! core-tokens`). The samples are the adversarial cases in
//! `tests/selfhost/lexer/samples/`, the lexer's own sources, the standard
//! library and the Core spec corpus.

// The differential needs a Unix C toolchain; elsewhere only the hash test runs.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use argorix_parser::core::core_token_dump;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// FNV-1a 64, as `compiler.token_dump.fnv1a` computes it.
fn fnv1a(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn samples() -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    let directories = [
        "compiler",
        "stdlib",
        "tests/selfhost/lexer/samples",
        "tests/selfhost/spec/invalid",
        "tests/selfhost/spec/valid",
    ];
    for directory in directories {
        let mut entries: Vec<PathBuf> = fs::read_dir(root().join(directory))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_file())
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

#[test]
fn the_rust_side_uses_the_documented_hash() {
    // Known FNV-1a 64 vectors, so both sides agree on the function itself.
    assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
}

#[cfg(unix)]
#[test]
fn the_argorix_lexer_matches_the_stage0_lexer_when_cc_is_available() {
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

    let work = env::temp_dir().join(format!("argorix-lexer-{}", std::process::id()));
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

    let c_file = work.join("lex_files.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join("tests/selfhost/lexer/lex_files.argx"))
        .arg("--output")
        .arg(&c_file)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "emission failed: {}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let executable = work.join("lex_files");
    // Lexing every sample takes more than the default test budget of steps.
    let compile = Command::new(compiler)
        .args(["-std=c11", "-O1", "-Wall", "-Wextra", "-Werror"])
        .arg("-DARGORIX_STEP_LIMIT=4000000000ULL")
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
        let expected = core_token_dump(source);
        let actual = fs::read(build.join(format!("dumps/{index}.tokens"))).unwrap();
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
        "the Argorix lexer disagrees:\n{}",
        failed.join("\n")
    );
    fs::remove_dir_all(work).unwrap();
}
