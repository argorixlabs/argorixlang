//! ESP-010: the Argorix lexer (`compiler/lexer.argx`) against the stage0 one.
//!
//! One Core program embeds every sample source and the FNV-1a of the stage0
//! token dump (`argorixc core-tokens`) for each. Compiled through the
//! transitional C backend, it lexes each sample with the Argorix lexer, dumps
//! the tokens the same way and returns a bitmask of the samples whose digest
//! differs, so a failure names its sample. The samples are the adversarial
//! cases in `tests/selfhost/lexer/samples/`, the lexer's own sources, the
//! standard library and the Core spec corpus.

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
    let mut directories = vec![
        root().join("tests/selfhost/lexer/samples"),
        root().join("compiler"),
        root().join("stdlib"),
        root().join("tests/selfhost/spec/valid"),
        root().join("tests/selfhost/spec/invalid"),
    ];
    directories.sort();
    for directory in directories {
        let mut entries: Vec<PathBuf> = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_file())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .strip_prefix(root())
                .unwrap_or(&path)
                .display()
                .to_string();
            found.push((name, fs::read(&path).unwrap()));
        }
    }
    found
}

#[test]
fn the_rust_dump_uses_the_documented_hash() {
    // Known FNV-1a 64 vectors, so the two sides agree on the function itself.
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
    assert!(samples.len() <= 64, "one bit per sample");
    assert!(samples.iter().any(|(name, _)| name.ends_with("lexer.argx")));

    let mut program = String::from(
        "core 0.1;\nmodule lexer_check.all;\n\nimport compiler.token_dump;\n\npub fn argorix_main() -> u64 {\n    let mut mismatches: u64 = 0u64;\n",
    );
    for (index, (_, source)) in samples.iter().enumerate() {
        let bytes = source
            .iter()
            .map(|byte| format!("{byte}u8"))
            .collect::<Vec<_>>()
            .join(", ");
        let expected = fnv1a(core_token_dump(source).as_bytes());
        program.push_str(&format!(
            "    let sample_{index}: Array<u8, {}> = [{bytes}];\n    if token_dump.digest(sample_{index}.as_slice()) != {expected}u64 {{\n        mismatches += 1u64 << {index}u64;\n    }}\n",
            source.len()
        ));
    }
    program.push_str("    mismatches\n}\n");

    let work = env::temp_dir().join(format!("argorix-lexer-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).unwrap();
    let root_file = work.join("lexer_check.argx");
    fs::write(&root_file, &program).unwrap();
    let c_file = work.join("lexer_check.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(&root_file)
        .arg("--output")
        .arg(&c_file)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "emission failed: {}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let executable = work.join("lexer_check");
    // The samples together take more than the default test budget of steps.
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
    let run = Command::new(&executable).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).trim().to_string();
    let mask: u64 = stdout
        .strip_prefix("ARGORIX_RESULT:")
        .unwrap_or_else(|| {
            panic!(
                "no result: {stdout} {}",
                String::from_utf8_lossy(&run.stderr)
            )
        })
        .parse()
        .unwrap();
    let failed: Vec<&str> = samples
        .iter()
        .enumerate()
        .filter(|(index, _)| mask & (1 << index) != 0)
        .map(|(_, (name, _))| name.as_str())
        .collect();
    assert!(
        failed.is_empty(),
        "the Argorix lexer disagrees on {failed:?}"
    );
    fs::remove_dir_all(work).unwrap();
}
