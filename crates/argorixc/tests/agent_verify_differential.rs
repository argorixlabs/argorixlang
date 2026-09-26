//! ESP-018.D.3: the Argorix bytecode verifier (`compiler/agent_verify.argx`)
//! against stage0's `serde_json::from_str::<BytecodeProgram>` and
//! `verify_bytecode`.
//!
//! `tests/selfhost/agent/verify_files.argx` is compiled through the
//! transitional C backend and run over every `.argbc.json` file of the
//! repository, the adversarial samples of `tests/selfhost/agent/verify_samples/`,
//! and the bytecode stage0 emits for every agent-language program that
//! checks. For each it writes the canonical dump of `spec/language/verify.md`,
//! which must equal stage0's (`argorixc agent-verify`).
//!
//! The verifier is ported in steps. A file the Argorix side cannot decide
//! yet dumps `unsupported`; it is counted, not compared. It fails on any
//! mismatch, when the Argorix side rejects a file stage0 reads, and when
//! fewer files match than the port has reached (`MATCHED_AT_LEAST`).

// The differential needs a Unix C toolchain; elsewhere this file is empty.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use argorix_module::{agent_bytecode_dump, bytecode_verify_dump};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

/// How many files the Argorix verifier decides as stage0 does today.
const MATCHED_AT_LEAST: usize = 89;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn walk(directory: &Path, suffix: &str, found: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "target" && !name.starts_with('.') {
                walk(&path, suffix, found);
            }
        } else if name.ends_with(suffix) {
            found.push(path);
        }
    }
}

/// The samples: the adversarial ones, the repository's serialized bytecode,
/// then stage0's bytecode for every checked program.
fn samples() -> Vec<(String, Vec<u8>)> {
    let mut paths = Vec::new();
    walk(
        &root().join("tests/selfhost/agent/verify_samples"),
        ".json",
        &mut paths,
    );
    walk(&root(), ".argbc.json", &mut paths);
    paths.dedup();
    let mut samples: Vec<(String, Vec<u8>)> = paths
        .into_iter()
        .map(|path| {
            let name = path.strip_prefix(root()).unwrap().display().to_string();
            (name, fs::read(&path).unwrap())
        })
        .collect();
    let mut sources = Vec::new();
    walk(&root(), ".argx", &mut sources);
    for path in sources {
        let source = fs::read(&path).unwrap();
        if source.starts_with(b"core 0.1;") {
            continue;
        }
        let bytecode = agent_bytecode_dump(&source);
        if bytecode.starts_with('{') {
            let name = path.strip_prefix(root()).unwrap().display().to_string();
            samples.push((format!("{name} (bytecode)"), bytecode.into_bytes()));
        }
    }
    samples
}

#[cfg(unix)]
#[test]
fn the_argorix_verifier_decides_as_stage0_when_cc_is_available() {
    let compiler = ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("C compiler unavailable; the differential runs in C-enabled CI");
        return;
    };
    let samples = samples();
    assert!(
        samples.len() > 700,
        "the corpus shrank to {}",
        samples.len()
    );

    let work = env::temp_dir().join(format!("argorix-agent-verify-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let package = work.join("package");
    let build = work.join("build");
    fs::create_dir_all(package.join("samples")).unwrap();
    fs::create_dir_all(build.join("dumps")).unwrap();
    let mut list = String::new();
    let mut total = 0_u64;
    for (index, (_, bytes)) in samples.iter().enumerate() {
        let file = format!("samples/{index}.json");
        fs::write(package.join(&file), bytes).unwrap();
        list.push_str(&file);
        list.push('\n');
        total += bytes.len() as u64;
    }
    fs::write(package.join("files.txt"), &list).unwrap();

    let c_file = work.join("verify_files.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join("tests/selfhost/agent/verify_files.argx"))
        .arg("--output")
        .arg(&c_file)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "emission failed: {}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let executable = work.join("verify_files");
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
        .args(["--write-budget", "400000000"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        format!("ARGORIX_RESULT:{}", samples.len()),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let mut matched = 0;
    let mut unsupported = 0;
    let mut failed = Vec::new();
    for (index, (name, bytes)) in samples.iter().enumerate() {
        let expected = bytecode_verify_dump(bytes);
        let actual = fs::read(build.join(format!("dumps/{index}.verify"))).unwrap();
        if actual == b"unsupported\n" {
            unsupported += 1;
        } else if actual == expected.as_bytes() {
            matched += 1;
        } else {
            failed.push(format!(
                "{name}:\n  expected {expected:?}\n  got      {:?}",
                String::from_utf8_lossy(&actual)
            ));
        }
    }
    eprintln!(
        "agent verifier: {matched} match, {unsupported} not decided yet, {} differ, of {}",
        failed.len(),
        samples.len()
    );
    assert!(
        failed.is_empty(),
        "the Argorix verifier disagrees on {} files:\n{}",
        failed.len(),
        failed
            .iter()
            .take(30)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        matched >= MATCHED_AT_LEAST,
        "only {matched} files match; the port had reached {MATCHED_AT_LEAST}"
    );
    fs::remove_dir_all(work).unwrap();
}
