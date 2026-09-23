//! ESP-012.C: the Argorix linker and checker (`compiler/link.argx`) against
//! the stage0 driver's way of checking a package.
//!
//! `tests/selfhost/check/link_files.argx` is compiled through the transitional
//! C backend and run with a package root holding every source file. It reads
//! each package through the compiler-host boundary, links and checks it, and
//! writes its diagnostics dump (`spec/core/check.md`). Each dump must equal,
//! byte for byte, what the stage0 driver's `check_core_package` produces
//! (`argorixc core-check-package-dump`).
//!
//! The packages are the compiler, which links the Argorix checker and linker
//! with themselves, each module of the standard library, the programs that
//! import them, the multi-module conformance programs, the single files of the
//! other corpora and the cases in `tests/selfhost/check/packages/`.

// The differential needs a Unix C toolchain; elsewhere this file is empty.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use argorix_semantics::core_package_check_dump;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The stage0 dump, on a stack as large as `argorixc` gives itself: the
/// deep-nesting samples recurse further than a test thread's default stack.
fn on_large_stack(sources: Vec<Vec<u8>>) -> String {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || core_package_check_dump(&sources))
        .unwrap()
        .join()
        .unwrap()
}

/// The files of a directory with one of `extensions`, sorted, as paths
/// relative to the repository.
fn files(directory: &str, extensions: &[&str]) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(root().join(directory))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extensions.contains(&extension))
        })
        .map(|path| {
            format!(
                "{directory}/{}",
                path.file_name().unwrap().to_string_lossy()
            )
        })
        .collect();
    found.sort();
    found
}

/// Each package is its root, then its locked set without the root.
fn packages() -> Vec<(String, Vec<String>)> {
    let compiler = files("compiler", &["argx"]);
    let stdlib = files("stdlib", &["argx"]);
    let everything: Vec<String> = compiler.iter().chain(&stdlib).cloned().collect();
    let with = |root: &str, set: &[String]| -> (String, Vec<String>) {
        let mut package = vec![root.to_string()];
        package.extend(set.iter().filter(|file| *file != root).cloned());
        (root.to_string(), package)
    };
    let mut found = Vec::new();
    for root in &compiler {
        found.push(with(root, &everything));
    }
    for root in &stdlib {
        found.push(with(root, &stdlib));
    }
    for directory in [
        "tests/selfhost/lexer",
        "tests/selfhost/parser",
        "tests/selfhost/check",
    ] {
        for root in files(directory, &["argx"]) {
            found.push(with(&root, &everything));
        }
    }
    for root in files("tests/selfhost/stdlib", &["argx"]) {
        found.push(with(&root, &stdlib));
    }
    let regression = files("conformance/core_c/regression", &["argx"]);
    for root in &regression {
        found.push(with(root, &regression));
    }
    for directory in [
        "tests/selfhost/runtime",
        "tests/selfhost/spec/valid",
        "tests/selfhost/spec/invalid",
        "tests/selfhost/check/samples",
        "tests/selfhost/parser/samples",
        "tests/selfhost/lexer/samples",
    ] {
        for root in files(directory, &["argx", "src"]) {
            found.push(with(&root, &[]));
        }
    }
    let cases = root().join("tests/selfhost/check/packages");
    let mut names: Vec<String> = fs::read_dir(&cases)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    for name in names {
        let directory = format!("tests/selfhost/check/packages/{name}");
        let main = format!("{directory}/main.argx");
        found.push(with(&main, &files(&directory, &["argx"])));
    }
    found
}

#[cfg(unix)]
#[test]
fn the_argorix_linker_matches_the_stage0_driver_when_cc_is_available() {
    let compiler = ["cc", "clang", "gcc"]
        .into_iter()
        .find(|name| Command::new(name).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("C compiler unavailable; the differential runs in C-enabled CI");
        return;
    };
    let packages = packages();
    assert!(packages
        .iter()
        .any(|(root, _)| root == "compiler/link.argx"));

    let work = env::temp_dir().join(format!("argorix-link-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let package_root = work.join("package");
    let build = work.join("build");
    fs::create_dir_all(package_root.join("samples")).unwrap();
    fs::create_dir_all(build.join("dumps")).unwrap();
    // Each source is copied once under a flat name: the listed paths are what
    // the Argorix program reads, and they must be normal relative paths.
    let mut copies: BTreeMap<String, String> = BTreeMap::new();
    let mut list = String::new();
    let mut total = 0_u64;
    for (_, package) in &packages {
        let mut line = Vec::new();
        for file in package {
            let next = copies.len();
            let copy = copies
                .entry(file.clone())
                .or_insert_with(|| format!("samples/{next}.src"))
                .clone();
            let source = fs::read(root().join(file)).unwrap();
            if !package_root.join(&copy).exists() {
                fs::write(package_root.join(&copy), &source).unwrap();
            }
            total += source.len() as u64;
            line.push(copy);
        }
        list.push_str(&line.join(" "));
        list.push('\n');
    }
    fs::write(package_root.join("packages.txt"), &list).unwrap();

    let c_file = work.join("link_files.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join("tests/selfhost/check/link_files.argx"))
        .arg("--output")
        .arg(&c_file)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "emission failed: {}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let executable = work.join("link_files");
    // Linking every package takes more than the default test budget of steps.
    let compile = Command::new(compiler)
        .args(["-std=c11", "-O1", "-Wall", "-Wextra", "-Werror"])
        .arg("-DARGORIX_STEP_LIMIT=400000000000ULL")
        // The merged trees of the compiler pass the 1 MiB default per buffer.
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
        .arg(&package_root)
        .arg("--read-budget")
        .arg((total + list.len() as u64).to_string())
        .arg("--build-root")
        .arg(&build)
        .args(["--write-budget", "100000000"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        format!("ARGORIX_RESULT:{}", packages.len()),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let mut failed = Vec::new();
    for (index, (name, package)) in packages.iter().enumerate() {
        let sources: Vec<Vec<u8>> = package
            .iter()
            .map(|file| fs::read(root().join(file)).unwrap())
            .collect();
        let expected = on_large_stack(sources);
        let actual = fs::read(build.join(format!("dumps/{index}.link"))).unwrap();
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
        "the Argorix linker disagrees:\n{}",
        failed.join("\n")
    );
    if env::var_os("ARGORIX_KEEP_WORK").is_none() {
        fs::remove_dir_all(work).unwrap();
    }
}
