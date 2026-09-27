//! ESP-018.D.2: the Argorix agent-language bytecode (`compiler/agent_ir.argx`)
//! against stage0's `lower_ir` (`crates/argorix_bytecode/src/lower.rs`).
//!
//! `tests/selfhost/agent/bytecode_files.argx` is compiled through the transitional
//! C backend and run with a package root holding every sample. For each it
//! writes the canonical dump of `spec/language/bytecode.md`: the bytecode as pretty
//! JSON, or the checker dump of a program that does not check. Each dump
//! must equal, byte for byte, what stage0 produces (`argorixc agent-bytecode`).
//! The test fails on any mismatch, and when fewer programs lower than
//! `LOWERED_AT_LEAST`.

// The differential needs a Unix C toolchain; elsewhere this file is empty.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use argorix_module::agent_bytecode_dump;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

/// How many samples check and so lower to bytecode. Raise it when the corpus
/// grows; it guards against the comparison quietly covering less.
const LOWERED_AT_LEAST: usize = 557;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn walk(directory: &Path, extension: &str, found: &mut Vec<String>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "target" && !name.starts_with('.') {
                walk(&path, extension, found);
            }
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            let relative = path.strip_prefix(root()).unwrap();
            found.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// The adversarial samples, then every agent-language `.argx` of the
/// repository (Core sources are left out: they are not this language).
fn samples() -> Vec<(String, Vec<u8>)> {
    let mut names = Vec::new();
    walk(
        &root().join("tests/selfhost/agent/parser_samples"),
        "src",
        &mut names,
    );
    walk(
        &root().join("tests/selfhost/agent/check_samples"),
        "src",
        &mut names,
    );
    walk(&root(), "argx", &mut names);
    names
        .into_iter()
        .map(|name| {
            let data = fs::read(root().join(&name)).unwrap();
            (name, data)
        })
        .filter(|(_, data)| !data.starts_with(b"core 0.1;"))
        .collect()
}

#[cfg(unix)]
#[test]
fn the_argorix_agent_bytecode_matches_the_stage0_bytecode_when_cc_is_available() {
    let compiler = ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("C compiler unavailable; the differential runs in C-enabled CI");
        return;
    };
    let samples = samples();
    assert!(
        samples.len() > 1000,
        "the corpus shrank to {}",
        samples.len()
    );

    let work = env::temp_dir().join(format!("argorix-agent-bytecode-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let package = work.join("package");
    let build = work.join("build");
    fs::create_dir_all(package.join("samples")).unwrap();
    fs::create_dir_all(build.join("dumps")).unwrap();
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

    let c_file = work.join("bytecode_files.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join("tests/selfhost/agent/bytecode_files.argx"))
        .arg("--output")
        .arg(&c_file)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "emission failed: {}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let executable = work.join("bytecode_files");
    let compile = Command::new(compiler)
        .args(["-std=c11", "-O1", "-Wall", "-Wextra", "-Werror"])
        .arg("-DARGORIX_STEP_LIMIT=400000000000ULL")
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
        .args(["--write-budget", "4000000000"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        format!("ARGORIX_RESULT:{}", samples.len()),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let mut matched = 0;
    let mut lowered = 0;
    let mut failed = Vec::new();
    for (index, (name, source)) in samples.iter().enumerate() {
        let expected = agent_bytecode_dump(source);
        if expected.starts_with('{') {
            lowered += 1;
        }
        let actual = fs::read(build.join(format!("dumps/{index}.bytecode"))).unwrap();
        if actual == expected.as_bytes() {
            matched += 1;
        } else {
            let got = String::from_utf8_lossy(&actual);
            let at = expected
                .bytes()
                .zip(got.bytes())
                .position(|(want, have)| want != have)
                .unwrap_or(expected.len().min(got.len()));
            let from = at.saturating_sub(60);
            failed.push(format!(
                "{name}: at byte {at}:\n  expected …{}\n  got      …{}",
                &expected[from..(at + 60).min(expected.len())],
                &got[from.min(got.len())..(at + 60).min(got.len())]
            ));
        }
    }
    eprintln!(
        "agent bytecode: {matched} match ({lowered} lowered), {} differ, of {}",
        failed.len(),
        samples.len()
    );
    assert!(
        failed.is_empty(),
        "the Argorix agent bytecode disagrees on {} samples:\n{}",
        failed.len(),
        failed
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        lowered >= LOWERED_AT_LEAST,
        "only {lowered} samples lower; the corpus had {LOWERED_AT_LEAST}"
    );
    fs::remove_dir_all(work).unwrap();
}
