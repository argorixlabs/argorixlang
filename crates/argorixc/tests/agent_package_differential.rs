//! ESP-018.C: the Argorix module graph of agent-language packages
//! (`compiler/agent_package.argx`) against the stage0 one (`argorix_module`).
//!
//! `tests/selfhost/agent/package_files.argx` is compiled through the
//! transitional C backend and run with a package root holding a copy of
//! every package: each directory of the repository with an `argorix.toml`,
//! and the adversarial packages of `tests/selfhost/agent/package_samples/`.
//! For each it writes the canonical dump of `spec/language/packages.md`: the
//! resolver's error, or the module graph and the merged program's checker
//! diagnostics. Each dump must equal, byte for byte, what stage0 produces
//! (`argorixc agent-package`).

// The differential needs a Unix C toolchain; elsewhere this file is empty.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use argorix_module::package_dump;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every directory below `directory` with an `argorix.toml`, sorted.
fn packages(directory: &Path, found: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    if entries
        .iter()
        .any(|path| path.file_name().is_some_and(|name| name == "argorix.toml"))
    {
        found.push(directory.to_path_buf());
    }
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_dir() && name != "target" && !name.starts_with('.') {
            packages(&path, found);
        }
    }
}

/// `from` copied to `to`, returning the bytes copied.
fn copy_tree(from: &Path, to: &Path) -> u64 {
    fs::create_dir_all(to).unwrap();
    let mut total = 0;
    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            total += copy_tree(&path, &target);
        } else {
            total += fs::copy(&path, &target).unwrap();
        }
    }
    total
}

#[cfg(unix)]
#[test]
fn the_argorix_module_graph_matches_the_stage0_resolver_when_cc_is_available() {
    let compiler = ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("C compiler unavailable; the differential runs in C-enabled CI");
        return;
    };
    let mut found = Vec::new();
    packages(
        &root().join("tests/selfhost/agent/package_samples"),
        &mut found,
    );
    let samples = found.len();
    packages(&root(), &mut found);
    found.retain(|path| !path.starts_with(root().join("target")));
    let mut unique: Vec<PathBuf> = Vec::new();
    for path in found {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }
    assert!(samples > 10, "the package samples shrank to {samples}");
    assert!(unique.len() > 60, "the packages shrank to {}", unique.len());

    let work = env::temp_dir().join(format!("argorix-agent-packages-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let package = work.join("package");
    let build = work.join("build");
    fs::create_dir_all(&package).unwrap();
    fs::create_dir_all(build.join("dumps")).unwrap();
    let mut list = String::new();
    let mut total = 0_u64;
    for (index, path) in unique.iter().enumerate() {
        let directory = format!("p{index}");
        total += copy_tree(path, &package.join(&directory));
        list.push_str(&directory);
        list.push('\n');
    }
    fs::write(package.join("packages.txt"), &list).unwrap();

    let c_file = work.join("package_files.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join("tests/selfhost/agent/package_files.argx"))
        .arg("--output")
        .arg(&c_file)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "emission failed: {}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let executable = work.join("package_files");
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
        .arg((2 * total + list.len() as u64).to_string())
        .arg("--build-root")
        .arg(&build)
        .args(["--write-budget", "400000000"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        format!("ARGORIX_RESULT:{}", unique.len()),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let mut failed = Vec::new();
    for (index, path) in unique.iter().enumerate() {
        let expected = package_dump(&package.join(format!("p{index}/argorix.toml")));
        let actual = fs::read(build.join(format!("dumps/{index}.package"))).unwrap();
        if actual != expected.as_bytes() {
            let name = path.strip_prefix(root()).unwrap_or(path).display();
            failed.push(format!(
                "{name}:\n  expected {expected:?}\n  got      {:?}",
                String::from_utf8_lossy(&actual)
            ));
        }
    }
    eprintln!(
        "agent packages: {} match, {} differ, of {}",
        unique.len() - failed.len(),
        failed.len(),
        unique.len()
    );
    assert!(
        failed.is_empty(),
        "the Argorix module graph disagrees on {} packages:\n{}",
        failed.len(),
        failed.join("\n")
    );
    fs::remove_dir_all(work).unwrap();
}
